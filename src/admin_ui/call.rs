use crate::open_ai;
use crate::open_ai::completion::{Completion, CompletionError};
use crate::open_ai::role::Role;
use crate::open_ai_key::OpenAiKey;
use crate::person_actions::{PersonActionError, PersonActionKind, PersonReaction};

pub async fn submit_prompt(
    open_ai_key: OpenAiKey,
    client: reqwest::Client,
    prompt: String,
) -> Result<String, CompletionError> {
    let response = Completion::new(open_ai::model::Model::DEFAULT)
        .add_message(Role::User, prompt.as_str())
        .send_request(&open_ai_key, client)
        .await?;

    response.as_message().map_err(Into::into)
}

pub async fn submit_reaction(
    open_ai_key: OpenAiKey,
    memories: Vec<String>,
    person_identity: String,
    situation: String,
    state_of_mind: String,
) -> Result<Vec<PersonReaction>, CompletionError> {
    let mut completion = Completion::new(open_ai::model::Model::DEFAULT);

    completion.add_message(Role::System, "You are a person simulation framework. You have deep insights into the human mind and are very good at predicting people's reactions. When given a description of a person, their state of mind, and some of their recent memories, respond as the person would in the given situation.");

    let memories_list = memories
        .iter()
        .map(|memory| format!("- {}", memory))
        .collect::<Vec<String>>()
        .join("\n");

    completion.add_message(Role::User, format!("Memories:\n{}", memories_list).as_str());

    completion.add_message(
        Role::User,
        format!("Person identity: {}", person_identity).as_str(),
    );

    completion.add_message(
        Role::User,
        format!("State of Mind: {}", state_of_mind).as_str(),
    );

    completion.add_message(Role::User, format!("Situation: {}", situation).as_str());

    completion.add_tool_call(PersonActionKind::to_choice_tool());

    let response = completion
        .send_request(&open_ai_key, reqwest::Client::new())
        .await?;

    let tool_calls = response
        .as_tool_calls()
        .map_err(CompletionError::ToolCallDecode)?;

    let person_actions = tool_calls
        .into_iter()
        .map(PersonReaction::from_open_ai_tool_call)
        .collect::<Result<Vec<PersonReaction>, PersonActionError>>()
        .map_err(CompletionError::PersonAction)?;

    Ok(person_actions)
}

pub async fn submit_prompt_lab(
    open_ai_key: OpenAiKey,
    system_prompt: String,
    user_prompt: String,
) -> Result<String, CompletionError> {
    let mut completion = Completion::new(open_ai::model::Model::DEFAULT);

    if !system_prompt.trim().is_empty() {
        completion.add_message(Role::System, system_prompt.trim());
    }

    completion.add_message(Role::User, user_prompt.as_str());
    completion.add_tool_call(PersonActionKind::to_choice_tool());

    let response = completion
        .send_request(&open_ai_key, reqwest::Client::new())
        .await?;

    Ok(response.as_pretty_json())
}
