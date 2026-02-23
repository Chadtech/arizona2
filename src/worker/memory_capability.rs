use super::Worker;
use crate::capability::log_event::LogEventCapability;
use crate::capability::memory::{
    MemoryCapability, MemoryQueryPrompt, MemorySearchResult, MessageTypeArgs, NewMemory,
};
use crate::capability::person::PersonCapability;
use crate::capability::scene::SceneCapability;
use crate::domain::logger::Level;
use crate::domain::memory_uuid::MemoryUuid;
use crate::domain::message::MessageSender;
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::Completion;
use crate::open_ai::embedding::EmbeddingRequest;
use crate::open_ai::role::Role;
use crate::open_ai::tool::{ToolFunction, ToolFunctionParameter};
use crate::open_ai::tool_call::ToolCall;
use sqlx::Row;

impl MemoryCapability for Worker {
    async fn create_memory(&self, new_memory: NewMemory) -> Result<MemoryUuid, String> {
        let embedding = EmbeddingRequest::new(new_memory.content.clone())
            .create(self.open_ai_key.clone(), self.reqwest_client.clone())
            .await
            .map_err(|err| err.message())?;

        let rec = sqlx::query!(
            r#"
                INSERT INTO memory (uuid, person_uuid, content, embedding)
                VALUES ($1::UUID, $2::UUID, $3::TEXT, $4)
                RETURNING uuid;
            "#,
            new_memory.memory_uuid.to_uuid(),
            new_memory.person_uuid.to_uuid(),
            new_memory.content,
            &embedding[..] as &[f32]
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new memory: {}", err))?;

        Ok(MemoryUuid::from_uuid(rec.uuid))
    }

    async fn maybe_create_memories_from_description(
        &self,
        person_uuid: PersonUuid,
        description: String,
    ) -> Result<Vec<MemoryUuid>, String> {
        let person_name = self
            .get_persons_name(person_uuid.clone())
            .await
            .map_err(|err| format!("Failed to get person name: {}", err))?;

        let mut completion = Completion::new(open_ai::model::Model::DEFAULT);
        completion.add_message(
            Role::System,
            "You decide whether a person should store a memory of a recent event. Be conservative: only store memories that are useful, relevant to motivations, emotionally significant, or important to relationships. If the event is not meaningful or lasting, do not call any tool. When you do create a memory, write it in standardized, first-person language (e.g., \"I ...\"), and include a memorable_score from 0-100.",
        );

        let user_prompt = format!(
            "Person: {}\n\nEvent description:\n{}",
            person_name.as_str(),
            description
        );
        completion.add_message(Role::User, user_prompt.as_str());

        let tool = ToolFunction::new(
            "create_memory".to_string(),
            "Store a memory if the event is worth remembering.".to_string(),
            vec![
                ToolFunctionParameter::StringParam {
                    name: "content".to_string(),
                    description:
                        "The memory to store, written in standardized first-person language."
                            .to_string(),
                    required: true,
                },
                ToolFunctionParameter::IntegerParam {
                    name: "memorable_score".to_string(),
                    description: "A 0-100 score for how memorable this is.".to_string(),
                    required: true,
                },
            ],
        );
        completion.add_tool_call(tool.into());
        let response = completion
            .send_request(&self.open_ai_key, reqwest::Client::new())
            .await
            .map_err(|err| err.message())?;

        let tool_calls = response.maybe_tool_calls().map_err(|err| err.message())?;

        let new_memories: Vec<MemoryCandidate> = match tool_calls {
            None => {
                log_memory_decision(self, None, None, Some("no_tool_call")).await;
                vec![]
            }
            Some(tool_calls) => extract_memory_content(tool_calls)?,
        };

        let mut ret = vec![];
        for candidate in new_memories.into_iter() {
            if candidate.memorable_score < MIN_MEMORABLE_SCORE {
                log_memory_decision(
                    self,
                    None,
                    Some(candidate.memorable_score),
                    Some("score_below_threshold"),
                )
                .await;
                continue;
            }

            let is_distinct = is_memory_distinct(self, &person_uuid, &candidate.content).await?;
            if !is_distinct {
                log_memory_decision(
                    self,
                    None,
                    Some(candidate.memorable_score),
                    Some("not_distinct"),
                )
                .await;
                continue;
            }

            let new_memory = NewMemory {
                memory_uuid: MemoryUuid::new(),
                content: candidate.content,
                person_uuid: person_uuid.clone(),
            };

            let memory_uuid = self.create_memory(new_memory).await?;

            log_memory_decision(
                self,
                Some(&memory_uuid),
                Some(candidate.memorable_score),
                None,
            )
            .await;

            ret.push(memory_uuid);
        }

        Ok(ret)
    }

    async fn create_memory_query_prompt(
        &self,
        person_recalling: PersonName,
        message_type_args: MessageTypeArgs,
        recent_events: Vec<String>,
        state_of_mind: &String,
        situation: &String,
    ) -> Result<MemoryQueryPrompt, String> {
        let mut prompt = String::new();

        fn add_scene_to_prompt(
            prompt: &mut String,
            person_recalling: &PersonName,
            scene_name: &String,
            scene_description: &String,
        ) {
            prompt.push_str(
                format!(
                    "{} is in a scene called '{}'. The scene is described as: {}.\n\n",
                    person_recalling.as_str(),
                    scene_name,
                    scene_description
                )
                .as_str(),
            );
        }

        match &message_type_args {
            MessageTypeArgs::Scene {
                scene_name,
                scene_description,
                ..
            } => {
                add_scene_to_prompt(
                    &mut prompt,
                    &person_recalling,
                    scene_name,
                    scene_description,
                );
            }
            MessageTypeArgs::SceneByUuid { scene_uuid } => {
                let maybe_scene_name = self
                    .get_scene_name(scene_uuid)
                    .await
                    .map_err(|err| format!("Error fetching scene name: {}", err))?;

                let scene_name = match maybe_scene_name {
                    Some(name) => name,
                    None => Err(format!("Scene with UUID {} not found", scene_uuid.clone()))?,
                };

                let maybe_scene_description = self
                    .get_scene_description(scene_uuid)
                    .await
                    .map_err(|err| format!("Error fetching scene description: {}", err))?;

                let scene_description = match maybe_scene_description {
                    Some(description) => description,
                    None => Err(format!(
                        "Scene description for UUID {} not found",
                        scene_uuid.to_string()
                    ))?,
                };

                add_scene_to_prompt(
                    &mut prompt,
                    &person_recalling,
                    &scene_name,
                    &scene_description,
                );
            }
            MessageTypeArgs::Direct { from } => {
                let sender_name = {
                    match from {
                        MessageSender::AiPerson(person_uuid) => {
                            let rec = sqlx::query!(
                                r#"
                                SELECT name
                                FROM person
                                WHERE uuid = $1::UUID;
                            "#,
                                person_uuid.to_uuid()
                            )
                            .fetch_one(&self.sqlx)
                            .await
                            .map_err(|err| format!("Error fetching person's name: {}", err))?;

                            rec.name
                        }
                        MessageSender::RealWorldUser => "Chadtech".to_string(),
                    }
                };

                prompt.push_str(
                    format!(
                        "{} has received a direct message from {}.\n\n",
                        person_recalling.as_str(),
                        sender_name
                    )
                    .as_str(),
                );
            }
        }

        prompt.push_str("\n\n");

        prompt.push_str(
            format!(
                "{}'s state of mind is {}",
                person_recalling.as_str(),
                state_of_mind
            )
            .as_str(),
        );

        fn add_people_to_prompt(prompt: &mut String, people: &Vec<String>) {
            prompt.push_str("\n\nOther people present:\n");

            for person in people {
                prompt.push_str(format!("- {}\n", person).as_str());
            }
        }

        prompt.push_str(
            format!(
                "\n\nThe current situation is described as:\n{}\n",
                situation
            )
            .as_str(),
        );

        match message_type_args {
            MessageTypeArgs::Scene { people, .. } => {
                add_people_to_prompt(&mut prompt, &people);
            }
            MessageTypeArgs::Direct { .. } => {
                prompt
                    .push_str("\n\nThis is a direct message, so no one else is around to see it\n");
            }
            MessageTypeArgs::SceneByUuid { scene_uuid } => {
                let scene_participants = self
                    .get_scene_current_participants(&scene_uuid)
                    .await
                    .map_err(|err| format!("Error fetching scene participants: {}", err))?;

                let participant_names: Vec<String> = scene_participants
                    .iter()
                    .map(|participant| participant.person_name.as_str().to_owned())
                    .collect();

                add_people_to_prompt(&mut prompt, &participant_names);
            }
        }

        prompt.push_str("\n\nRecent events include:\n");
        for event in recent_events {
            prompt.push_str(format!("- {}\n", event).as_str());
        }

        let mut completion = Completion::new(open_ai::model::Model::DEFAULT);

        completion.add_message(Role::System, "You are a memory retrieval assistant. Given context, generate a prompt that can be used in a vector database of memories to retrieve relevant memories for that person in that situation.");
        completion.add_message(Role::User, prompt.as_str());

        let response = completion
            .send_request(&self.open_ai_key, self.reqwest_client.clone())
            .await
            .map_err(|err| {
                format!(
                    "Error generating memory query prompt:\n{}",
                    err.to_nice_error().to_string()
                )
            })?;

        let memory_prompt = response.as_message().map_err(|err| {
            format!(
                "Error extracting memory prompt from completion response: {}",
                err.to_nice_error().to_string()
            )
        })?;

        Ok(MemoryQueryPrompt {
            prompt: memory_prompt,
        })
    }

    async fn search_memories(
        &self,
        person_uuid: PersonUuid,
        query: String,
        limit: i64,
    ) -> Result<Vec<MemorySearchResult>, String> {
        // Generate embedding for the query
        let query_embedding = EmbeddingRequest::new(query)
            .create(self.open_ai_key.clone(), self.reqwest_client.clone())
            .await
            .map_err(|err| err.message())?;

        // Search for similar memories using vector similarity
        let records = sqlx::query(
            r#"
                SELECT
                    content,
                    (embedding <=> $1::vector)::FLOAT AS distance
                FROM memory
                WHERE person_uuid = $2::UUID
                ORDER BY embedding <=> $1::vector
                LIMIT $3
            "#,
        )
        .bind(&query_embedding[..] as &[f32])
        .bind(person_uuid.to_uuid())
        .bind(limit)
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error searching memories: {}", err))?;

        let mut results = Vec::with_capacity(records.len());
        for rec in records {
            let content = rec
                .try_get::<String, _>("content")
                .map_err(|err| format!("Error reading memory content: {}", err))?;
            let distance = rec
                .try_get::<Option<f64>, _>("distance")
                .map_err(|err| format!("Error reading memory distance: {}", err))?
                .unwrap_or(f64::MAX);

            results.push(MemorySearchResult { content, distance });
        }

        Ok(results)
    }
}

const MIN_MEMORY_DISTANCE: f64 = 0.15;
const MIN_MEMORABLE_SCORE: i64 = 60;

async fn log_memory_decision(
    worker: &Worker,
    memory_uuid: Option<&MemoryUuid>,
    memorable_score: Option<i64>,
    reason: Option<&str>,
) {
    let reason = reason.map(|value| value.to_string());
    let data = match memory_uuid {
        Some(uuid) => serde_json::json!({
            "created": true,
            "memory_uuid": uuid.to_uuid().to_string(),
            "memorable_score": memorable_score,
            "reason": reason,
        }),
        None => serde_json::json!({
            "created": false,
            "memorable_score": memorable_score,
            "reason": reason,
        }),
    };

    if let Err(err) = worker
        .log_event("memory_decision".to_string(), Some(data))
        .await
    {
        worker.logger.log(
            Level::Warning,
            &format!("Failed to log memory decision: {}", err),
        );
    }
}

async fn is_memory_distinct(
    worker: &Worker,
    person_uuid: &PersonUuid,
    content: &str,
) -> Result<bool, String> {
    let matches = worker
        .search_memories(person_uuid.clone(), content.to_string(), 1)
        .await?;

    if matches.is_empty() {
        return Ok(true);
    }

    Ok(matches[0].distance > MIN_MEMORY_DISTANCE)
}
struct MemoryCandidate {
    content: String,
    memorable_score: i64,
}

fn extract_memory_content(tool_calls: Vec<ToolCall>) -> Result<Vec<MemoryCandidate>, String> {
    let mut ret: Vec<MemoryCandidate> = vec![];

    for call in tool_calls
        .into_iter()
        .filter(|call| call.name == "create_memory")
    {
        let content = call
            .arguments
            .iter()
            .find(|(name, _)| name == "content")
            .and_then(|(_, value)| value.as_str())
            .ok_or_else(|| "Missing 'content' argument in 'create_memory' tool call".to_string())?;

        let memorable_score = call
            .arguments
            .iter()
            .find(|(name, _)| name == "memorable_score")
            .and_then(|(_, value)| value.as_i64())
            .ok_or_else(|| {
                "Missing 'memorable_score' argument in 'create_memory' tool call".to_string()
            })?;

        if !(0..=100).contains(&memorable_score) {
            Err("Invalid 'memorable_score' (expected 0-100)".to_string())?;
        }

        ret.push(MemoryCandidate {
            content: content.to_string(),
            memorable_score,
        });
    }

    Ok(ret)
}
