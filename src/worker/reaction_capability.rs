use crate::capability::motivation::MotivationCapability;
use crate::capability::person::PersonCapability;
use crate::capability::reaction::ReactionCapability;
use crate::domain::logger::Level;
use crate::domain::memory::Memory;
use crate::domain::motivation::Motivation;
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::{Completion, CompletionError};
use crate::open_ai::role::Role;
use crate::open_ai::tool_call::ToolCall;
use crate::person_actions::{
    PersonAction, PersonActionError, PersonActionKind, PersonReaction, ReflectionDecision,
};
use crate::worker::Worker;

pub enum Error {
    CompletionError(CompletionError),
    FailedToGetMotivations(String),
    FailedToGetReactionDualLayer(String),
    NoPersonActionFound,
    MoreThanOnePersonActionFound(Vec<PersonReaction>),
}

impl ReactionCapability for Worker {
    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonReaction, String> {
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
    let reaction_dual_layer = worker
        .is_reaction_dual_layer(&person_uuid)
        .await
        .map_err(Error::FailedToGetReactionDualLayer)?;

    if reaction_dual_layer {
        get_reaction_dual_layer(
            worker,
            memories,
            person_uuid,
            person_identity,
            state_of_mind,
            situation,
        )
        .await
    } else {
        get_reaction_single_layer(
            worker,
            memories,
            person_uuid,
            person_identity,
            state_of_mind,
            situation,
        )
        .await
    }
}

async fn get_reaction_dual_layer(
    worker: &Worker,
    memories: Vec<Memory>,
    person_uuid: PersonUuid,
    person_identity: String,
    state_of_mind: String,
    situation: String,
) -> Result<PersonReaction, Error> {
    let mut completion = Completion::new(open_ai::model::Model::DEFAULT);

    let person_name = worker
        .get_persons_name(person_uuid.clone())
        .await
        .map_err(|err| {
            Error::FailedToGetReactionDualLayer(format!("Failed to get person's name: {}", err))
        })?;

    let thinking_system_prompt = "You are an expert in understanding and predicting real human behavior and psychology. Think through the person's internal state and immediate intent and predict what they will do and think next. The person only has awareness of the information explicitly provided in this prompt and nothing else. The person is only capable of actions that are represented by the available tool calls; do not assume any other capabilities. Respond with plain text only. Your response must describe: (1) what the person wants to do next, and (2) what they are thinking right now.";
    completion.add_message(Role::System, thinking_system_prompt);

    let memories_list_text = Memory::to_list_text(&memories);

    let motivations = worker
        .get_motivations_for_person(person_uuid.clone())
        .await
        .map_err(Error::FailedToGetMotivations)?;

    let thinking_user_prompt = format!(
        "Describe this person's immediate intention and current thinking in plain text.\n\nName: \n{}\n\nMemories:\n{}\n\nBackground drives:\n{}\n\nPerson identity:\n{}\n\nState of mind:\n{}\n\nSituation:\n{}",
        person_name.to_string(), memories_list_text, Motivation::to_list_text(&motivations), person_identity, state_of_mind, situation
		);
    worker.logger.log(
        Level::Info,
        format!(
            "Dual-layer first call prompts\nSystem:\n{}\n\nUser:\n{}",
            thinking_system_prompt, thinking_user_prompt
        )
        .as_str(),
    );

    worker.logger.log(
        Level::Info,
        format!(
            "Sending dual-layer completion request with user prompt:\n{}",
            thinking_user_prompt
        )
        .as_str(),
    );

    completion.add_message(Role::User, thinking_user_prompt.as_str());

    let response = completion
        .send_request(&worker.open_ai_key, reqwest::Client::new())
        .await
        .map_err(Error::CompletionError)?;

    let dual_layer_text = response
        .as_message()
        .map_err(|err| Error::CompletionError(err.into()))?;

    worker.logger.log(
        Level::Info,
        format!(
            "Dual-layer first-pass reaction text for person {}:\n{}",
            person_uuid.to_uuid(),
            dual_layer_text
        )
        .as_str(),
    );

    // Second LLM call

    let mut action_completion = Completion::new(open_ai::model::Model::DEFAULT);

    let action_system_prompt = format!(
		"You ARE {}.\n\nPerson identity:\n{}\n\nYou only have awareness of information explicitly provided in this prompt and nothing else. You are only capable of actions represented by the available tool calls. Execute the intention determined previously within those constraints. Use exactly one tool call and no extra text.",
		person_name.to_string(),
		person_identity,
	);

    let action_user_prompt = format!(
        "Memories:\n{}\n\nRecent events and recent messages:\n{}\n\nFirst-pass internal reaction text:\n{}\n\nNow choose exactly one action tool call. Do not output any plain text.",
        memories_list_text,
        situation,
        dual_layer_text
			);
    worker.logger.log(
        Level::Info,
        format!(
            "Dual-layer action call prompts\nSystem:\n{}\n\nUser:\n{}",
            action_system_prompt, action_user_prompt
        )
        .as_str(),
    );

    action_completion.add_message(Role::System, action_system_prompt.as_str());
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
                        "Dual-layer reaction for person {}: {} (reflection: {})",
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

async fn get_reaction_single_layer(
    worker: &Worker,
    memories: Vec<Memory>,
    person_uuid: PersonUuid,
    person_identity: String,
    state_of_mind: String,
    situation: String,
) -> Result<PersonReaction, Error> {
    let mut completion = Completion::new(open_ai::model::Model::DEFAULT);

    completion.add_message(Role::System, "You are simulating a real human. Your goal is to predict what this person would actually do. The information provided is complete; the person has no other knowledge or context beyond what is in the prompt, and their behavior must not assume anything else. Use the memories, motivations, identity, and state of mind to choose the most realistic action. Predict behavior as a realistic person within the capacities of the available tool calls. Prefer ordinary, plausible behavior over dramatic or clever behavior. If the person does speak, they will contribute something novel instead of repeating what has already been said. If nothing new has happened, predict what a human of the following description realistically would do if that period of time elapsed with nothing happening. The person understands they can only do the following actions, and these are the only possible actions: wait, hibernate, idle, say in scene, move to scene. For say in scene, they may also include destination_scene_name to leave immediately after speaking. Respond with exactly one tool call and no extra text.");

    let motivations = worker
        .get_motivations_for_person(person_uuid.clone())
        .await
        .map_err(Error::FailedToGetMotivations)?;

    let user_prompt = format!(
        "Predict the most realistic, human behavior for this person in the situation below, then choose exactly one action tool call that best matches that behavior. Do not explain. The background drives should influence behavior implicitly; avoid stating them directly in dialogue.\n\nMemories:\n{}\n\nBackground drives:\n{}\n\nPerson identity:\n{}\n\nState of mind:\n{}\n\nSituation:\n{}",
        Memory::to_list_text(&memories), Motivation::to_list_text(&motivations), person_identity, state_of_mind, situation
	);

    worker.logger.log(
        Level::Info,
        format!(
            "Sending completion request with user prompt:\n{}",
            user_prompt
        )
        .as_str(),
    );

    completion.add_message(Role::User, user_prompt.as_str());

    completion.add_tool_call(PersonActionKind::to_choice_tool());

    let response = completion
        .send_request(&worker.open_ai_key, reqwest::Client::new())
        .await
        .map_err(Error::CompletionError)?;

    let tool_calls_res: Result<Vec<ToolCall>, CompletionError> =
        response.as_tool_calls().map_err(Into::into);

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
                        "Reaction for person {}: {} (reflection: {})",
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

fn describe_action(action: &PersonAction) -> String {
    match action {
        PersonAction::Wait { duration } => format!("wait for {} ms", duration),
        PersonAction::Hibernate { duration } => format!("hibernate for {} ms", duration),
        PersonAction::Idle => "idle".to_string(),
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

fn tool_calls_into_reactions(
    tool_calls: Vec<ToolCall>,
) -> Result<Vec<PersonReaction>, CompletionError> {
    tool_calls
        .into_iter()
        .map(|tool_call| PersonReaction::from_open_ai_tool_call(tool_call))
        .collect::<Result<Vec<PersonReaction>, PersonActionError>>()
        .map_err(Into::into)
}
