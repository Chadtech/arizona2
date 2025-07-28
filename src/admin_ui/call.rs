use crate::open_ai;
use crate::open_ai::Role;
use crate::open_ai_key::OpenAiKey;

pub async fn submit_prompt(
    open_ai_key: OpenAiKey,
    client: reqwest::Client,
    prompt: String,
) -> Result<String, open_ai::CompletionError> {
    let response = open_ai::Completion::new(open_ai::Model::Gpt4p1)
        .add_message(open_ai::Role::User, prompt.as_str())
        .send_request(&open_ai_key, client)
        .await?;

    Ok(response)
}

pub async fn submit_reaction(
    open_ai_key: OpenAiKey,
    memories: Vec<String>,
    person_identity: String,
    situation: String,
) -> Result<String, open_ai::CompletionError> {
    let mut completion = open_ai::Completion::new(open_ai::Model::Gpt4p1);

    completion.add_message(Role::System, "You are a person simulation framework. You have deep insights into the human mind and are very good at predicting people's reactions. When given a description of a person and some of their recent memories, respond as the person would in the given situation.");

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

    completion.add_message(Role::User, format!("Situation: {}", situation).as_str());

    let response = completion
        .send_request(&open_ai_key, reqwest::Client::new())
        .await?;

    Ok(response)
}
