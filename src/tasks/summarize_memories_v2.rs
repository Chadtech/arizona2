use crate::domain::logger::{Level, Logger};
use crate::domain::memory_uuid::MemoryUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::Completion;
use crate::open_ai::embedding::EmbeddingRequest;
use crate::open_ai::role::Role;
use crate::open_ai::tool::{ToolFunction, ToolFunctionParameter};
use crate::open_ai::tool_call::ToolCall;
use crate::worker;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

const TARGET_SECTION_COUNT: usize = 7;
const MAX_SECTION_SUMMARY_ATTEMPTS: usize = 4;

pub enum Error {
    WorkerInit(worker::InitError),
    FetchPeople(sqlx::Error),
    FetchPersonMemories(sqlx::Error),
    Completion(String),
    ToolCallDecode(String),
    MissingToolArgument(String),
    InvalidResponseShape { person_name: String, details: String },
    CreateEmbedding(String),
    InsertMemory(sqlx::Error),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::WorkerInit(err) => format!("Worker initialization failed: {}", err.message()),
            Error::FetchPeople(err) => format!("Failed to fetch people: {}", err),
            Error::FetchPersonMemories(err) => format!("Failed to fetch person memories: {}", err),
            Error::Completion(err) => format!("Failed to summarize memories with LLM: {}", err),
            Error::ToolCallDecode(err) => format!("Failed to decode LLM tool call: {}", err),
            Error::MissingToolArgument(err) => format!("Missing expected tool argument: {}", err),
            Error::InvalidResponseShape {
                person_name,
                details,
            } => format!(
                "LLM output shape invalid for person {}: {}",
                person_name, details
            ),
            Error::CreateEmbedding(err) => format!("Failed creating embedding: {}", err),
            Error::InsertMemory(err) => format!("Failed inserting summarized memory: {}", err),
        }
    }
}

#[derive(Deserialize)]
struct V2MemoryCandidate {
    third_person_memory: String,
    third_person_summary: String,
    first_person_memory: String,
    significance_comment: String,
    emotional_score: i32,
    people_names: Vec<String>,
    subject_tags: Vec<String>,
}

pub async fn run() -> Result<(), Error> {
    let logger = Logger::init(Level::Warning);
    let worker = crate::worker::Worker::new(logger)
        .await
        .map_err(Error::WorkerInit)?;

    let people = sqlx::query!(
        r#"
            SELECT uuid, name
            FROM person
            ORDER BY name ASC;
        "#
    )
    .fetch_all(&worker.sqlx)
    .await
    .map_err(Error::FetchPeople)?;

    let mut person_name_to_uuid: HashMap<String, Uuid> = HashMap::new();
    for person in &people {
        person_name_to_uuid.insert(person.name.to_lowercase(), person.uuid);
    }

    for person in people {
        let source_memories = sqlx::query!(
            r#"
                SELECT content
                FROM memory
                WHERE person_uuid = $1::UUID
                ORDER BY created_at ASC;
            "#,
            person.uuid
        )
        .fetch_all(&worker.sqlx)
        .await
        .map_err(Error::FetchPersonMemories)?;

        if source_memories.is_empty() {
            println!(
                "Skipping {} ({}) because no source memories exist",
                person.name, person.uuid
            );
            continue;
        }

        let source_texts = source_memories
            .iter()
            .map(|row| row.content.clone())
            .collect::<Vec<String>>();

        let summarized_memories =
            summarize_memories_for_person(&worker, &person.name, &source_texts).await?;

        let created_count = summarized_memories.len();
        for candidate in summarized_memories {
            let normalized_people_names = normalize_string_list(candidate.people_names);
            let normalized_subject_tags = normalize_string_list(candidate.subject_tags);
            let people_uuids =
                map_people_names_to_uuids(&normalized_people_names, &person_name_to_uuid);
            let third_person_memory = candidate.third_person_memory;
            let first_person_memory = candidate.first_person_memory;
            let significance_comment = candidate.significance_comment;
            let third_person_summary = clamp_to_max_words(&candidate.third_person_summary, 18);

            let retrieval_summary = format!(
                "{} Significance: {}",
                third_person_memory.trim(),
                significance_comment.trim()
            );

            let embedding = EmbeddingRequest::new(retrieval_summary.clone())
                .create(worker.open_ai_key.clone(), worker.reqwest_client.clone())
                .await
                .map_err(|err| Error::CreateEmbedding(err.message()))?;

            sqlx::query!(
                r#"
                    INSERT INTO memory (
                        uuid,
                        person_uuid,
                        content,
                        embedding,
                        summary,
                        emotional_score,
                        retrieval_summary,
                        summary_first_person,
                        people_names,
                        people_uuids,
                        subject_tags
                    )
                    VALUES (
                        $1::UUID,
                        $2::UUID,
                        $3::TEXT,
                        $4,
                        $5::TEXT,
                        $6::INT,
                        $7::TEXT,
                        $8::TEXT,
                        $9::TEXT[],
                        $10::UUID[],
                        $11::TEXT[]
                    );
                "#,
                MemoryUuid::new().to_uuid(),
                person.uuid,
                third_person_memory.clone(),
                &embedding[..] as &[f32],
                third_person_summary,
                candidate.emotional_score,
                retrieval_summary,
                first_person_memory,
                &normalized_people_names as &[String],
                &people_uuids as &[Uuid],
                &normalized_subject_tags as &[String],
            )
            .execute(&worker.sqlx)
            .await
            .map_err(Error::InsertMemory)?;
        }

        println!(
            "Created {} v2 summarized memories for {} ({}) from {} source memories",
            created_count,
            person.name,
            person.uuid,
            source_texts.len()
        );
    }

    Ok(())
}

async fn summarize_memories_for_person(
    worker: &worker::Worker,
    person_name: &str,
    memories: &[String],
) -> Result<Vec<V2MemoryCandidate>, Error> {
    let sections = split_memories_into_sections(memories, TARGET_SECTION_COUNT);
    let total_sections = sections.len();
    let mut ret = Vec::with_capacity(total_sections);
    for (section_index, section_memories) in sections.into_iter().enumerate() {
        let memory = summarize_memory_for_section(
            worker,
            person_name,
            &section_memories,
            section_index + 1,
            total_sections,
        )
        .await?;
        ret.push(memory);
    }

    Ok(ret)
}

fn split_memories_into_sections(memories: &[String], target_sections: usize) -> Vec<Vec<String>> {
    if memories.is_empty() {
        return vec![];
    }

    let section_count = memories.len().min(target_sections);
    let mut sections = Vec::with_capacity(section_count);
    for i in 0..section_count {
        let start = i * memories.len() / section_count;
        let end = (i + 1) * memories.len() / section_count;
        sections.push(memories[start..end].to_vec());
    }
    sections
}

async fn summarize_memory_for_section(
    worker: &worker::Worker,
    person_name: &str,
    section_memories: &[String],
    section_index: usize,
    total_sections: usize,
) -> Result<V2MemoryCandidate, Error> {
    let section_memory_list = section_memories
        .iter()
        .enumerate()
        .map(|(idx, memory)| {
            format!(
                "{}. {}",
                idx + 1,
                memory.trim().replace('\n', " ")
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    let mut last_count = 0usize;
    for _attempt in 1..=MAX_SECTION_SUMMARY_ATTEMPTS {
        let mut completion = Completion::new(open_ai::model::Model::DEFAULT);
        completion.add_tool_call(v2_memory_tool_definition().into());
        completion.add_message(
            Role::System,
            "You summarize raw memories into one distilled memory record. Use only the provided tool call and call it exactly once. Do not output plain text. The person's name is exactly the provided Person value. 'Arizona2' is a system/project name, not a person name. Never treat 'Arizona2' as the person unless the Person value is literally 'Arizona2'.",
        );
        completion.add_message(
            Role::User,
            format!(
                "Person: {}\n\nSection {}/{} source memories:\n{}\n\nCreate exactly one consolidated memory for this section. Provide: third_person_memory, third_person_summary, first_person_memory, significance_comment, emotional_score, people_names, and subject_tags. third_person_summary must be much shorter than third_person_memory (max 18 words, one sentence). Do not invent details not supported by source memories.",
                person_name,
                section_index,
                total_sections,
                section_memory_list
            )
            .as_str(),
        );

        let response = completion
            .send_request(&worker.open_ai_key, worker.reqwest_client.clone())
            .await
            .map_err(|err| Error::Completion(err.message()))?;

        let tool_calls = response
            .as_tool_calls()
            .map_err(|err| Error::ToolCallDecode(err.message()))?;
        let candidates = tool_calls
            .into_iter()
            .filter(|call| call.name == "add_v2_memory")
            .map(v2_memory_candidate_from_tool_call)
            .collect::<Result<Vec<V2MemoryCandidate>, Error>>()?;

        if candidates.len() == 1 {
            return Ok(candidates.into_iter().next().ok_or_else(|| Error::InvalidResponseShape {
                person_name: person_name.to_string(),
                details: "internal error extracting section summary".to_string(),
            })?);
        }
        last_count = candidates.len();
    }

    Err(Error::InvalidResponseShape {
        person_name: person_name.to_string(),
        details: format!(
            "section {} of {} expected 1 memory, got {} after {} attempts",
            section_index, total_sections, last_count, MAX_SECTION_SUMMARY_ATTEMPTS
        ),
    })
}

fn v2_memory_tool_definition() -> ToolFunction {
    ToolFunction::new(
        "add_v2_memory".to_string(),
        "Create one consolidated v2 memory.".to_string(),
        vec![
            ToolFunctionParameter::String {
                name: "third_person_memory".to_string(),
                description: "Neutral third-person version of the memory.".to_string(),
                required: true,
            },
            ToolFunctionParameter::String {
                name: "third_person_summary".to_string(),
                description:
                    "Very short one-sentence summary (max 18 words), much shorter than third_person_memory."
                        .to_string(),
                required: true,
            },
            ToolFunctionParameter::String {
                name: "first_person_memory".to_string(),
                description: "First-person version of the same memory.".to_string(),
                required: true,
            },
            ToolFunctionParameter::String {
                name: "significance_comment".to_string(),
                description: "Short explanation of why this memory is significant.".to_string(),
                required: true,
            },
            ToolFunctionParameter::Integer {
                name: "emotional_score".to_string(),
                description: "0-100 emotional score.".to_string(),
                required: true,
            },
            ToolFunctionParameter::StringArray {
                name: "people_names".to_string(),
                description: "People involved in the memory.".to_string(),
                required: true,
            },
            ToolFunctionParameter::StringArray {
                name: "subject_tags".to_string(),
                description: "Short lowercase tags for the memory subjects.".to_string(),
                required: true,
            },
        ],
    )
}

fn v2_memory_candidate_from_tool_call(call: ToolCall) -> Result<V2MemoryCandidate, Error> {
    Ok(V2MemoryCandidate {
        third_person_memory: find_required_string_argument(&call, "third_person_memory")?,
        third_person_summary: find_required_string_argument(&call, "third_person_summary")?,
        first_person_memory: find_required_string_argument(&call, "first_person_memory")?,
        significance_comment: find_required_string_argument(&call, "significance_comment")?,
        emotional_score: find_required_integer_argument(&call, "emotional_score")?,
        people_names: find_required_string_array_argument(&call, "people_names")?,
        subject_tags: find_required_string_array_argument(&call, "subject_tags")?,
    })
}

fn find_required_argument_value(call: &ToolCall, key: &str) -> Result<serde_json::Value, Error> {
    call.arguments
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| Error::MissingToolArgument(format!("Missing '{}' in tool arguments", key)))
}

fn find_required_string_argument(call: &ToolCall, key: &str) -> Result<String, Error> {
    find_required_argument_value(call, key)?
        .as_str()
        .map(|value| value.to_string())
        .ok_or_else(|| Error::MissingToolArgument(format!("'{}' must be a string", key)))
}

fn find_required_integer_argument(call: &ToolCall, key: &str) -> Result<i32, Error> {
    let value = find_required_argument_value(call, key)?
        .as_i64()
        .ok_or_else(|| Error::MissingToolArgument(format!("'{}' must be an integer", key)))?;
    if value < i32::MIN as i64 || value > i32::MAX as i64 {
        return Err(Error::MissingToolArgument(format!(
            "'{}' integer out of range: {}",
            key, value
        )));
    }
    Ok(value as i32)
}

fn find_required_string_array_argument(call: &ToolCall, key: &str) -> Result<Vec<String>, Error> {
    let value = find_required_argument_value(call, key)?;
    let array = value
        .as_array()
        .ok_or_else(|| Error::MissingToolArgument(format!("'{}' must be an array", key)))?;

    array
        .iter()
        .map(|item| {
            item.as_str().map(|text| text.to_string()).ok_or_else(|| {
                Error::MissingToolArgument(format!("'{}' must contain only strings", key))
            })
        })
        .collect::<Result<Vec<String>, Error>>()
}

fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    let mut ret: Vec<String> = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !ret.iter().any(|existing| existing.eq_ignore_ascii_case(trimmed)) {
            ret.push(trimmed.to_string());
        }
    }
    ret
}

fn map_people_names_to_uuids(
    people_names: &[String],
    name_to_uuid: &HashMap<String, Uuid>,
) -> Vec<Uuid> {
    let mut ret: Vec<Uuid> = Vec::new();
    for person_name in people_names {
        let key = person_name.to_lowercase();
        if let Some(uuid) = name_to_uuid.get(&key) {
            if !ret.contains(uuid) {
                ret.push(*uuid);
            }
        }
    }
    ret
}

fn clamp_to_max_words(input: &str, max_words: usize) -> String {
    let words = input.split_whitespace().collect::<Vec<&str>>();
    if words.len() <= max_words {
        return input.trim().to_string();
    }
    words[..max_words].join(" ")
}
