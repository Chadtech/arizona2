use crate::capability::motivation::MotivationCapability;
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::person_task::PersonTaskCapability;
use crate::capability::reaction::{ReactionCapability, ReactionPromptPreview};
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::logger::Level;
use crate::domain::memory::Memory;
use crate::domain::motivation::Motivation;
use crate::domain::person_task::{PersonTask, PersonTaskOutcomeCheck, PersonTaskTerminalOutcome};
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::{Completion, CompletionError};
use crate::open_ai::role::Role;
use crate::open_ai::tool::{Tool, ToolFunction, ToolFunctionParameter};
use crate::open_ai::tool_call::ToolCall;
use crate::person_actions::{
    PersonAction, PersonActionError, PersonActionKind, PersonReaction, ReflectionDecision,
};
use crate::worker::Worker;
use serde::Deserialize;

const INTERNAL_REACTION_PLACEHOLDER: &str = "<internal reaction text placeholder>";
const REACTION_VALIDATION_RETRY_LIMIT: usize = 1;

pub enum Error {
    CompletionError(CompletionError),
    FailedToGetMotivations(String),
    FailedToGetReactionDualLayer(String),
    FailedToClassifyTaskOutcome(String),
    InvalidTaskOutcomeToolCall(String),
    NoPersonActionFound,
    MoreThanOnePersonActionFound(Vec<PersonReaction>),
}

impl ReactionCapability for Worker {
    async fn summarize_reaction_events(&self, events_text: String) -> Result<String, String> {
        let trimmed = events_text.trim();
        if trimmed.is_empty() {
            return Ok("None.".to_string());
        }

        let mut completion = Completion::new();
        completion.add_message(
            Role::System,
            "You summarize recent roleplay events for another model that needs short working context before choosing an immediate action. Compress aggressively. Return a brief bullet list only. Keep the most decision-relevant developments, preserve important concrete facts, merge repetition, and omit low-signal detail. Prefer 4-8 bullets unless there is almost nothing to say.",
        );
        completion.add_message(
            Role::User,
            format!(
                "Summarize these recent events into a much shorter list.\n\n{}",
                trimmed
            )
            .as_str(),
        );

        let response = completion
            .send_request(&self.open_ai_key, reqwest::Client::new())
            .await
            .map_err(|err| err.message())?;

        let summary = response.as_message().map_err(|err| err.message())?;

        self.logger.log(
            Level::Info,
            format!("Summarized reaction events:\n{}", summary).as_str(),
        );

        Ok(summary)
    }

    async fn preview_reaction_prompts(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        situation: String,
    ) -> Result<ReactionPromptPreview, String> {
        let person_name = self
            .get_persons_name(person_uuid.clone())
            .await
            .map_err(|err| format!("Failed to get person's name: {}", err))?;

        let motivations = self
            .get_motivations_for_person(&person_uuid)
            .await
            .map_err(|err| format!("Failed to get motivations: {}", err))?;

        let person_identity = get_person_identity_summary(self, &person_uuid).await?;

        let state_of_mind = if let Some(som) = self
            .get_latest_state_of_mind(&person_uuid)
            .await
            .map_err(|err| format!("Failed to get state of mind: {}", err))?
        {
            som
        } else {
            Err(format!(
                "No state of mind found for person_uuid: {}",
                person_uuid.to_uuid()
            ))?
        };

        let current_person_task_text = get_current_person_task_text(self, &person_uuid)
            .await
            .map_err(|err| format!("Failed to get current person task: {}", err))?;

        let prompts = build_prompts(
            person_name.as_str(),
            &memories,
            &motivations,
            person_identity.as_str(),
            state_of_mind.content.as_str(),
            situation.as_str(),
            current_person_task_text.as_str(),
            INTERNAL_REACTION_PLACEHOLDER,
        );
        Ok(prompts)
    }

    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonReaction, String> {
        let person_identity = get_person_identity_summary(self, &person_uuid).await?;
        get_reaction_helper(
            self,
            memories,
            person_uuid,
            person_identity,
            state_of_mind,
            situation,
        )
        .await
        .map_err(|err| match err {
            Error::CompletionError(completion_err) => completion_err.message(),
            Error::FailedToGetMotivations(message) => message,
            Error::FailedToGetReactionDualLayer(message) => message,
            Error::FailedToClassifyTaskOutcome(message) => message,
            Error::InvalidTaskOutcomeToolCall(message) => message,
            Error::NoPersonActionFound => "No person action found".to_string(),
            Error::MoreThanOnePersonActionFound(actions) => {
                let actions_str = actions
                    .into_iter()
                    .map(|action| format!("{:?}", action))
                    .collect::<Vec<String>>()
                    .join(",\n");
                format!("More than one person action found: \n{}", actions_str)
            }
        })
    }

    async fn classify_current_task_outcome(
        &self,
        task: PersonTask,
        situation: String,
        action_summary: Option<String>,
    ) -> Result<PersonTaskOutcomeCheck, String> {
        classify_task_outcome_helper(self, &task.person_uuid, &task, situation, action_summary)
            .await
            .map_err(|err| match err {
                Error::CompletionError(completion_err) => completion_err.message(),
                Error::FailedToGetMotivations(message) => message,
                Error::FailedToGetReactionDualLayer(message) => message,
                Error::FailedToClassifyTaskOutcome(message) => message,
                Error::InvalidTaskOutcomeToolCall(message) => message,
                Error::NoPersonActionFound => "No person action found".to_string(),
                Error::MoreThanOnePersonActionFound(actions) => {
                    let actions_str = actions
                        .into_iter()
                        .map(|action| format!("{:?}", action))
                        .collect::<Vec<String>>()
                        .join(",\n");
                    format!("More than one person action found: \n{}", actions_str)
                }
            })
    }
}

async fn get_reaction_helper(
    worker: &Worker,
    memories: Vec<Memory>,
    person_uuid: PersonUuid,
    person_identity: String,
    state_of_mind: String,
    situation: String,
) -> Result<PersonReaction, Error> {
    let person_name = worker
        .get_persons_name(person_uuid.clone())
        .await
        .map_err(|err| {
            Error::FailedToGetReactionDualLayer(format!("Failed to get person's name: {}", err))
        })?;

    let motivations = worker
        .get_motivations_for_person(&person_uuid)
        .await
        .map_err(Error::FailedToGetMotivations)?;

    let current_person_task_text = get_current_person_task_text(worker, &person_uuid)
        .await
        .map_err(|err| {
            Error::FailedToGetReactionDualLayer(format!(
                "Failed to get current person task: {}",
                err
            ))
        })?;

    let prompts = build_prompts(
        person_name.as_str(),
        &memories,
        &motivations,
        person_identity.as_str(),
        state_of_mind.as_str(),
        situation.as_str(),
        current_person_task_text.as_str(),
        INTERNAL_REACTION_PLACEHOLDER,
    );

    let first_pass_text = get_first_pass_reaction_text(worker, &prompts, &person_uuid).await?;
    let reformulated_action_prompt =
        reformulate_action_prompt(worker, &prompts, &first_pass_text, &person_uuid).await?;

    let mut candidate = choose_reaction(
        worker,
        &prompts,
        reformulated_action_prompt.as_str(),
        None,
        &person_uuid,
    )
    .await?;

    for retry_index in 0..=REACTION_VALIDATION_RETRY_LIMIT {
        let validation = match validate_reaction_candidate(
            worker,
            &prompts,
            reformulated_action_prompt.as_str(),
            &candidate,
            &person_uuid,
        )
        .await
        {
            Ok(validation) => validation,
            Err(err) => {
                worker.logger.log(
                    Level::Error,
                    format!(
                        "Reaction validator failed for person {}: {}. Returning unvalidated action.",
                        person_uuid.to_uuid(),
                        err
                    )
                    .as_str(),
                );
                return Ok(candidate);
            }
        };

        if validation.is_valid {
            worker.logger.log(
                Level::Info,
                format!(
                    "Validated reaction for person {}: {} (reflection: {})",
                    person_uuid.to_uuid(),
                    describe_action(&candidate.action),
                    describe_reflection(&candidate.reflection)
                )
                .as_str(),
            );
            return Ok(candidate);
        }

        worker.logger.log(
            Level::Info,
            format!(
                "Rejected reaction for person {} on validation attempt {}: {} (candidate: {}, reflection: {})",
                person_uuid.to_uuid(),
                retry_index + 1,
                validation.reason,
                describe_action(&candidate.action),
                describe_reflection(&candidate.reflection)
            )
            .as_str(),
        );

        if retry_index < REACTION_VALIDATION_RETRY_LIMIT {
            candidate = choose_reaction(
                worker,
                &prompts,
                reformulated_action_prompt.as_str(),
                Some(validation.reason.as_str()),
                &person_uuid,
            )
            .await?;
            continue;
        }

        let fallback = fallback_reaction();
        worker.logger.log(
            Level::Info,
            format!(
                "Falling back to safe reaction for person {} after validator rejection: {} (reflection: {})",
                person_uuid.to_uuid(),
                describe_action(&fallback.action),
                describe_reflection(&fallback.reflection)
            )
            .as_str(),
        );
        return Ok(fallback);
    }

    Ok(fallback_reaction())
}

async fn classify_task_outcome_helper(
    worker: &Worker,
    person_uuid: &PersonUuid,
    task: &PersonTask,
    situation: String,
    action_summary: Option<String>,
) -> Result<PersonTaskOutcomeCheck, Error> {
    let mut completion = Completion::new();
    completion.add_message(
        Role::System,
        "You classify whether a person's current task is still active, completed, failed, or abandoned.\n\nUse the provided tool call exactly once.\n\nRules:\n- Use only the evidence in the prompt.\n- If the evidence is ambiguous, return `still_active`.\n- `completed` means the task's success condition is clearly met.\n- `failed` means the task's failure condition is clearly met, or the task is no longer achievable because of explicit evidence.\n- `abandoned` means the person clearly stopped pursuing the task, deprioritized it, or switched away from it without completing it.\n- Do not infer completion from vague progress.",
    );
    completion.add_tool_call(classify_task_outcome_tool());

    let action_summary_text = match action_summary {
        Some(summary) => summary,
        None => "None.".to_string(),
    };

    completion.add_message(
        Role::User,
        format!(
            "Person UUID: {}\n\nCurrent task:\n{}\n\nSuccess condition:\n{}\n\nFailure condition:\n{}\n\nAbandon condition:\n{}\n\nLatest situation:\n{}\n\nLatest action taken:\n{}",
            person_uuid.to_uuid(),
            task.content,
            optional_condition_text(task.success_condition.as_deref()),
            optional_condition_text(task.failure_condition.as_deref()),
            optional_condition_text(task.abandon_condition.as_deref()),
            situation,
            action_summary_text
        )
        .as_str(),
    );

    let response = completion
        .send_request(&worker.open_ai_key, worker.reqwest_client.clone())
        .await
        .map_err(|err| {
            Error::FailedToClassifyTaskOutcome(format!(
                "Failed to request task outcome classification: {}",
                err.message()
            ))
        })?;

    let tool_calls = response.as_tool_calls().map_err(|err| {
        Error::FailedToClassifyTaskOutcome(format!(
            "Failed to decode task outcome classifier tool calls: {}",
            err.message()
        ))
    })?;
    let classification = tool_calls_into_task_outcomes(tool_calls)?;

    worker.logger.log(
        Level::Info,
        format!(
            "Task outcome classification for person {} task {}: {:?} ({})",
            person_uuid.to_uuid(),
            task.uuid.to_uuid(),
            classification.outcome,
            classification.reason
        )
        .as_str(),
    );

    Ok(classification.outcome)
}

struct TaskOutcomeClassification {
    outcome: PersonTaskOutcomeCheck,
    reason: String,
}

fn classify_task_outcome_tool() -> Tool {
    Tool::FunctionCall(ToolFunction::new(
        "classify_task_outcome".to_string(),
        "Classify whether the person's current task is still active, completed, failed, or abandoned.".to_string(),
        vec![
            ToolFunctionParameter::StringEnum {
                name: "outcome".to_string(),
                description: "The task outcome classification.".to_string(),
                required: true,
                values: task_outcome_check_names(),
            },
            ToolFunctionParameter::String {
                name: "reason".to_string(),
                description: "Short evidence-based explanation for the classification.".to_string(),
                required: true,
            },
        ],
    ))
}

fn tool_calls_into_task_outcomes(
    tool_calls: Vec<ToolCall>,
) -> Result<TaskOutcomeClassification, Error> {
    if tool_calls.is_empty() {
        return Err(Error::InvalidTaskOutcomeToolCall(
            "Task outcome classifier returned no tool calls".to_string(),
        ));
    }

    if tool_calls.len() > 1 {
        return Err(Error::InvalidTaskOutcomeToolCall(format!(
            "Task outcome classifier returned {} tool calls; expected exactly one",
            tool_calls.len()
        )));
    }

    let tool_call = tool_calls.into_iter().next().expect("checked len above");
    if tool_call.name.as_str() != "classify_task_outcome" {
        return Err(Error::InvalidTaskOutcomeToolCall(format!(
            "Unexpected tool name from task outcome classifier: {}",
            tool_call.name
        )));
    }

    let mut maybe_outcome: Option<PersonTaskOutcomeCheck> = None;
    let mut maybe_reason: Option<String> = None;

    for (key, value) in tool_call.arguments {
        match key.as_str() {
            "outcome" => {
                let value = value.as_str().ok_or_else(|| {
                    Error::InvalidTaskOutcomeToolCall(
                        "Task outcome `outcome` argument was not a string".to_string(),
                    )
                })?;
                maybe_outcome = Some(
                    task_outcome_check_from_tool_value(value)
                        .map_err(Error::InvalidTaskOutcomeToolCall)?,
                );
            }
            "reason" => {
                let value = value.as_str().ok_or_else(|| {
                    Error::InvalidTaskOutcomeToolCall(
                        "Task outcome `reason` argument was not a string".to_string(),
                    )
                })?;
                maybe_reason = Some(value.to_string());
            }
            unexpected => {
                return Err(Error::InvalidTaskOutcomeToolCall(format!(
                    "Unexpected task outcome classifier argument: {}",
                    unexpected
                )));
            }
        }
    }

    let outcome = maybe_outcome.ok_or_else(|| {
        Error::InvalidTaskOutcomeToolCall(
            "Task outcome classifier omitted required `outcome` argument".to_string(),
        )
    })?;
    let reason = maybe_reason.ok_or_else(|| {
        Error::InvalidTaskOutcomeToolCall(
            "Task outcome classifier omitted required `reason` argument".to_string(),
        )
    })?;

    Ok(TaskOutcomeClassification { outcome, reason })
}

fn task_outcome_check_names() -> Vec<String> {
    let mut names = vec!["still_active".to_string()];
    names.extend(PersonTaskTerminalOutcome::all_names());
    names
}

fn task_outcome_check_from_tool_value(value: &str) -> Result<PersonTaskOutcomeCheck, String> {
    match value {
        "still_active" => Ok(PersonTaskOutcomeCheck::StillActive),
        _ => {
            PersonTaskTerminalOutcome::from_tool_value(value).map(PersonTaskOutcomeCheck::Terminal)
        }
    }
}

async fn reformulate_action_prompt(
    worker: &Worker,
    prompts: &ReactionPromptPreview,
    first_pass_text: &str,
    person_uuid: &PersonUuid,
) -> Result<String, Error> {
    let base_action_user_prompt = build_base_action_user_prompt(prompts, first_pass_text);

    worker.logger.log(
        Level::Info,
        format!(
            "=== ACTION USER PROMPT REFORMULATION INPUT ===\n{}",
            base_action_user_prompt
        )
        .as_str(),
    );

    let mut completion = Completion::new();
    completion.add_message(
        Role::System,
        "You rewrite action user prompts for another model. Compress aggressively. Keep only information that is directly relevant to choosing the person's single next action right now. Center the rewrite on what the person is deciding, what immediate actions are plausible, what constraints matter, and what newest facts override older context. Remove low-signal background detail, repetition, and anything that does not affect the immediate next action. Preserve concrete facts that materially change the decision. Keep references to the internal reaction text only insofar as they clarify immediate intent. Do not rewrite or restate any system prompt. Return only the rewritten user prompt as plain text.",
    );
    completion.add_message(
        Role::User,
        format!(
            "Rewrite this action user prompt into a much shorter action-focused user prompt.\n\n{}",
            base_action_user_prompt
        )
        .as_str(),
    );

    let response = completion
        .send_request(&worker.open_ai_key, worker.reqwest_client.clone())
        .await
        .map_err(Error::CompletionError)?;

    let reformulated_prompt = response
        .as_message()
        .map_err(|err| Error::CompletionError(err.into()))?;

    worker.logger.log(
        Level::Info,
        format!(
            "Reformulated action prompt for person {}:\n{}",
            person_uuid.to_uuid(),
            reformulated_prompt
        )
        .as_str(),
    );

    Ok(reformulated_prompt)
}

async fn get_first_pass_reaction_text(
    worker: &Worker,
    prompts: &ReactionPromptPreview,
    person_uuid: &PersonUuid,
) -> Result<String, Error> {
    let mut completion = Completion::new();
    completion.add_message(Role::System, prompts.thinking_system_prompt.as_str());
    completion.add_message(Role::User, prompts.thinking_user_prompt.as_str());

    worker.logger.log(
        Level::Info,
        format!(
            "first call prompts\nSystem Prompt ========\n{}\n\nUser Prompt =======\n{}",
            prompts.thinking_system_prompt, prompts.thinking_user_prompt
        )
        .as_str(),
    );

    let response = completion
        .send_request(&worker.open_ai_key, reqwest::Client::new())
        .await
        .map_err(Error::CompletionError)?;

    let text = response
        .as_message()
        .map_err(|err| Error::CompletionError(err.into()))?;

    worker.logger.log(
        Level::Info,
        format!(
            "first-pass reaction text for person {}:\n{}",
            person_uuid.to_uuid(),
            text
        )
        .as_str(),
    );

    Ok(text)
}

async fn choose_reaction(
    worker: &Worker,
    prompts: &ReactionPromptPreview,
    reformulated_action_prompt: &str,
    validation_feedback: Option<&str>,
    person_uuid: &PersonUuid,
) -> Result<PersonReaction, Error> {
    let mut action_completion = Completion::new();
    let action_user_prompt =
        build_action_user_prompt(reformulated_action_prompt, validation_feedback);

    worker.logger.log(
        Level::Info,
        format!(
            "=== ACTION SYSTEM PROMPT ===\n{}\n\n=== ACTION USER PROMPT ===\n{}",
            prompts.action_system_prompt, action_user_prompt
        )
        .as_str(),
    );

    action_completion.add_message(Role::System, prompts.action_system_prompt.as_str());
    action_completion.add_message(Role::User, action_user_prompt.as_str());
    action_completion.add_tool_call(PersonActionKind::to_choice_tool());

    let action_response = action_completion
        .send_request(&worker.open_ai_key, reqwest::Client::new())
        .await
        .map_err(Error::CompletionError)?;
    worker.logger.log(
        Level::Info,
        format!(
            "Dual-layer action call raw JSON response:\n{}",
            action_response.as_pretty_json()
        )
        .as_str(),
    );

    let tool_calls_res: Result<Vec<ToolCall>, CompletionError> =
        action_response.as_tool_calls().map_err(Into::into);

    let tool_calls = tool_calls_res.map_err(Error::CompletionError)?;

    let person_actions_res: Result<Vec<PersonReaction>, CompletionError> =
        tool_calls_into_reactions(tool_calls);

    let person_actions = person_actions_res.map_err(Error::CompletionError)?;

    match person_actions.first() {
        None => Err(Error::NoPersonActionFound)?,
        Some(first) => {
            if person_actions.len() > 1 {
                Err(Error::MoreThanOnePersonActionFound(person_actions))?
            } else {
                worker.logger.log(
                    Level::Info,
                    format!(
                        "Candidate reaction for person {}: {} (reflection: {})",
                        person_uuid.to_uuid(),
                        describe_action(&first.action),
                        describe_reflection(&first.reflection)
                    )
                    .as_str(),
                );
                Ok(first.clone())
            }
        }
    }
}

async fn validate_reaction_candidate(
    worker: &Worker,
    prompts: &ReactionPromptPreview,
    reformulated_action_prompt: &str,
    candidate: &PersonReaction,
    person_uuid: &PersonUuid,
) -> Result<ReactionValidationResult, String> {
    let mut completion = Completion::new();
    completion.add_message(
        Role::System,
        "You validate whether a single already-selected action is actually possible in Arizona2's action model. Be strict. The only real effects available are speaking in scene, moving to another scene, waiting, hibernating, or idling. Reject any chosen action that implies doing something else in the world, such as writing or editing a document, inspecting files, changing memory/state directly, manipulating objects, performing physical tasks, running a procedure, or otherwise claiming off-screen effects that Arizona2 cannot perform. For 'say in scene', the comment must be plausible spoken dialogue only, not narration of extra actions or claims that those actions were performed. Do not judge style, usefulness, or strategy beyond whether the chosen action is actually representable. Do not propose a replacement action. Return JSON only with keys is_valid (boolean) and reason (string). Keep reason brief and concrete.",
    );

    let action_user_prompt = build_action_user_prompt(reformulated_action_prompt, None);
    let validator_user_prompt = format!(
        "Action system prompt:\n{}\n\nAction user prompt:\n{}\n\nChosen reaction JSON:\n{}\n\nReturn JSON only.",
        prompts.action_system_prompt,
        action_user_prompt,
        reaction_to_json(candidate)
    );
    completion.add_message(Role::User, validator_user_prompt.as_str());

    let response = completion
        .send_request(&worker.open_ai_key, reqwest::Client::new())
        .await
        .map_err(|err| err.message())?;

    worker.logger.log(
        Level::Info,
        format!(
            "Reaction validator raw JSON response for person {}:\n{}",
            person_uuid.to_uuid(),
            response.as_pretty_json()
        )
        .as_str(),
    );

    let message = response.as_message().map_err(|err| err.message())?;
    serde_json::from_str::<ReactionValidationResult>(message.as_str()).map_err(|err| {
        format!(
            "Failed to decode reaction validator response as JSON: {}. Response text: {}",
            err, message
        )
    })
}

fn build_base_action_user_prompt(prompts: &ReactionPromptPreview, first_pass_text: &str) -> String {
    prompts
        .action_user_prompt
        .replace(INTERNAL_REACTION_PLACEHOLDER, first_pass_text)
}

fn build_action_user_prompt(
    base_action_user_prompt: &str,
    validation_feedback: Option<&str>,
) -> String {
    let mut action_user_prompt = base_action_user_prompt.to_string();

    if let Some(feedback) = validation_feedback {
        action_user_prompt.push_str(
            format!(
                "\n\nValidator feedback on your previous rejected action:\n{}\n\nChoose a different action that fixes this problem. Remember that Arizona2 can only speak, move scenes, wait, hibernate, or idle. Do not imply that any other action was performed. Choose exactly one tool call and do not output any plain text.",
                feedback
            )
            .as_str(),
        );
    }

    action_user_prompt
}

fn reaction_to_json(reaction: &PersonReaction) -> String {
    serde_json::json!({
        "reflection": reaction.reflection.to_name(),
        "action": action_to_json(&reaction.action),
    })
    .to_string()
}

fn action_to_json(action: &PersonAction) -> serde_json::Value {
    match action {
        PersonAction::Wait { duration } => serde_json::json!({
            "type": "wait",
            "duration": duration,
        }),
        PersonAction::Hibernate { duration } => serde_json::json!({
            "type": "hibernate",
            "duration": duration,
        }),
        PersonAction::Idle => serde_json::json!({
            "type": "idle",
        }),
        PersonAction::GazeInScene => serde_json::json!({
            "type": "gaze in scene",
        }),
        PersonAction::SayInScene {
            comment,
            destination_scene_name,
        } => serde_json::json!({
            "type": "say in scene",
            "comment": comment,
            "destination_scene_name": destination_scene_name,
        }),
        PersonAction::MoveToScene { scene_name } => serde_json::json!({
            "type": "move to scene",
            "scene_name": scene_name,
        }),
    }
}

fn fallback_reaction() -> PersonReaction {
    PersonReaction {
        action: PersonAction::Idle,
        reflection: ReflectionDecision::NoReflection,
    }
}

#[derive(Deserialize)]
struct ReactionValidationResult {
    is_valid: bool,
    reason: String,
}

fn build_prompts(
    person_name: &str,
    memories: &[Memory],
    motivations: &[Motivation],
    person_identity: &str,
    state_of_mind: &str,
    situation: &str,
    current_person_task_text: &str,
    first_pass_text: &str,
) -> ReactionPromptPreview {
    let thinking_system_prompt = "You are simulating a real person’s immediate inner reasoning at a single moment in time.

Your job is to infer this person’s current attention, what they believe is happening, what they want to do next, and which single next action they are leaning toward right now.

Rules:
- Use only the information explicitly present in this prompt.
- Do not assume abilities beyond the available tool calls.
- Focus on the newest message events first; use older context only to interpret them.
- Treat the person's current task as the strongest default signal for what they intend to do, unless the latest situation clearly overrides it.
- Treat the person as having stable drives, but not as mechanically repeating themselves.
- Prefer concrete immediate intent over general personality description.
- When multiple goals conflict, resolve them by choosing the action that best fits the person’s highest-priority drives and current constraints.
- If the newest messages do not materially change the situation, note that the person is likely to continue the current task without redundant restatement.
- Do not write a plan for multiple actions. Infer the single next action the person is most likely preparing to take now.

Respond in plain text only, as natural prose. Do not use bullet points, headings, labels, or numbered lists.

Your response should make clear:
- what the person is paying attention to right now,
- what they believe matters most in this moment,
- what they want to do next,
- and what specific action they are leaning toward taking immediately.
".to_string();
    let memories_list_text = Memory::many_to_list_text(memories);
    let motivations_list_text = Motivation::many_to_list_text(motivations);

    let thinking_user_prompt = format!(
        "Describe this person's immediate intention and current thinking in plain text.\n\nName: \n{}\n\nMemories:\n{}\n\nBackground drives:\n{}\n\nPerson identity:\n{}\n\nState of mind:\n{}\n\nSituation:\n{}{}",
        person_name,
        memories_list_text,
        motivations_list_text,
        person_identity,
        state_of_mind,
        situation,
        current_person_task_text
    );

    let action_system_prompt = format!(
		"You ARE $name$.\n\nPerson identity:\n{}\n\n

You only know what is explicitly in this prompt. You can only act through the available tool calls.

Stay in character as a person, not a document. Prefer brief, natural behavior over ceremonial repetition.

Your job is to choose the single action $name$ would take right now, based on the latest messages, the first-pass internal reaction text, and the available tools.

Rules:
- Available actions are only: `say in scene`, `move to scene`, `gaze in scene`, `wait`, `hibernate`, and `idle`.
- Prioritize the newest message over older context.
- Use the first-pass internal reaction text as the main guide to intent, unless it conflicts with newer information in this prompt.
- Choose exactly one tool call.
- Do not output any plain text.
- Do not repeat a prior acknowledgement unless it adds new information, resolves uncertainty, or changes another person’s behavior.
- Prefer actions that advance $name$’s current task, reduce uncertainty, or enforce an important constraint.
- If multiple actions are plausible, choose the one that best fits $name$’s highest-priority drives.",
		person_identity,
	).replace("$name$", person_name);

    let action_user_prompt = format!(
        "Memories:\n{}\n\nRecent events and recent messages:\n{}\n\nInternal reaction text:\n{}\n\nNow choose exactly one action tool call. Do not output any plain text.",
        memories_list_text,
        situation,
        first_pass_text
    );

    ReactionPromptPreview {
        thinking_system_prompt,
        thinking_user_prompt,
        action_system_prompt,
        action_user_prompt,
    }
}

fn describe_action(action: &PersonAction) -> String {
    match action {
        PersonAction::Wait { duration } => format!("wait for {} ms", duration),
        PersonAction::Hibernate { duration } => format!("hibernate for {} ms", duration),
        PersonAction::Idle => "idle".to_string(),
        PersonAction::GazeInScene => "gaze in scene".to_string(),
        PersonAction::SayInScene {
            comment,
            destination_scene_name,
        } => match destination_scene_name {
            Some(scene_name) => {
                format!("say in scene then move to {}: {}", scene_name, comment)
            }
            None => format!("say in scene: {}", comment),
        },
        PersonAction::MoveToScene { scene_name } => {
            format!("move to scene: {}", scene_name)
        }
    }
}

fn describe_reflection(reflection: &ReflectionDecision) -> String {
    match reflection {
        ReflectionDecision::Reflection => "reflection".to_string(),
        ReflectionDecision::NoReflection => "no_reflection".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_prompts() -> ReactionPromptPreview {
        ReactionPromptPreview {
            thinking_system_prompt: "thinking-system".to_string(),
            thinking_user_prompt: "thinking-user".to_string(),
            action_system_prompt: "action-system".to_string(),
            action_user_prompt: format!(
                "Recent events:\nalpha\n\nInternal reaction text:\n{}\n\nChoose one action.",
                INTERNAL_REACTION_PLACEHOLDER
            ),
        }
    }

    #[test]
    fn test_build_base_action_user_prompt_inserts_first_pass_text() {
        let prompts = sample_prompts();

        let result = build_base_action_user_prompt(&prompts, "Focus on Bob's question.");

        assert!(result.contains("Focus on Bob's question."));
        assert!(!result.contains(INTERNAL_REACTION_PLACEHOLDER));
    }

    #[test]
    fn test_build_action_user_prompt_appends_validator_feedback() {
        let result = build_action_user_prompt("short action prompt", Some("too much narration"));

        assert!(result.contains("short action prompt"));
        assert!(result.contains("too much narration"));
        assert!(result.contains("Choose a different action"));
    }

    #[test]
    fn test_build_action_user_prompt_without_feedback_returns_base_prompt() {
        let result = build_action_user_prompt("short action prompt", None);

        assert_eq!(result, "short action prompt");
    }
}

fn tool_calls_into_reactions(
    tool_calls: Vec<ToolCall>,
) -> Result<Vec<PersonReaction>, CompletionError> {
    tool_calls
        .into_iter()
        .map(PersonReaction::from_open_ai_tool_call)
        .collect::<Result<Vec<PersonReaction>, PersonActionError>>()
        .map_err(Into::into)
}

async fn get_person_identity_summary(
    worker: &Worker,
    person_uuid: &PersonUuid,
) -> Result<String, String> {
    if let Some(pi) = worker
        .get_person_identity_summary(person_uuid)
        .await
        .map_err(|err| format!("Failed to get person identity: {}", err))?
    {
        Ok(pi)
    } else {
        Err(format!(
            "No person identity found for person_uuid: {}",
            person_uuid.to_uuid()
        ))
    }
}

async fn get_current_person_task_text(
    worker: &Worker,
    person_uuid: &PersonUuid,
) -> Result<String, String> {
    match worker.get_persons_current_active_task(person_uuid).await? {
        Some(person_task) => Ok(format!("\n\nCurrent Task:\n{}", person_task)),
        None => Ok(String::new()),
    }
}

fn optional_condition_text(value: Option<&str>) -> &str {
    match value {
        Some(text) if !text.trim().is_empty() => text,
        _ => "None.",
    }
}
