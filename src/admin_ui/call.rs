use crate::open_ai::completion::{Completion, CompletionError};
use crate::open_ai::role::Role;
use crate::open_ai_key::OpenAiKey;
use crate::person_actions::PersonAction;
use crate::{open_ai, person_actions};

pub async fn submit_prompt(
    open_ai_key: OpenAiKey,
    client: reqwest::Client,
    prompt: String,
) -> Result<String, CompletionError> {
    let response = Completion::new(open_ai::model::Model::Gpt4p1)
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
) -> Result<Vec<PersonAction>, CompletionError> {
    let mut completion = Completion::new(open_ai::model::Model::Gpt4p1);

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

    completion.add_tool_call(person_actions::PersonActionKind::Say.to_open_ai_tool());
    completion.add_tool_call(person_actions::PersonActionKind::Wait.to_open_ai_tool());

    let response = completion
        .send_request(&open_ai_key, reqwest::Client::new())
        .await?;

    let tool_calls = response.as_tool_calls().map_err(Into::into)?;

    let person_actions = tool_calls
        .into_iter()
        .map(|tool_call| PersonAction::from_open_ai_tool_call(tool_call))
        .collect::<Result<Vec<PersonAction>, person_actions::PersonActionError>>()
        .map_err(Into::into)?;

    Ok(person_actions)
}
