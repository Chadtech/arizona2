use crate::capability::reaction::ReactionCapability;
use crate::domain::logger::Level;
use crate::domain::memory::Memory;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::{Completion, CompletionError};
use crate::open_ai::role::Role;
use crate::open_ai::tool_call::ToolCall;
use crate::person_actions::{PersonAction, PersonActionError, PersonActionKind};
use crate::worker::Worker;

pub enum Error {
    CompletionError(CompletionError),
    NoPersonActionFound,
    MoreThanOnePersonActionFound(Vec<PersonAction>),
}

impl ReactionCapability for Worker {
    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonAction, String> {
        get_reaction_helper(self, memories, person_identity, state_of_mind, situation)
            .await
            .map_err(|err| match err {
                Error::CompletionError(completion_err) => completion_err.message(),
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
    person_identity: String,
    state_of_mind: String,
    situation: String,
) -> Result<PersonAction, Error> {
    let mut completion = Completion::new(open_ai::model::Model::Gpt4p1);

    completion.add_message(Role::System, "You are a person simulation framework. You have deep insights into the human mind and are very good at predicting people's reactions to given situations. When given a description of a person, their state of mind, and some of their recent memories, respond as the person would in this situation by choosing exactly one tool call.");

    let memories_list = memories
        .iter()
        .map(|memory| format!("- {}", memory.content))
        .collect::<Vec<String>>()
        .join("\n");

    let user_prompt = format!(
        "Memories:\n{}\n\nPerson identity: {}\n\nState of mind: {}\n\nSituation: {}",
        memories_list, person_identity, state_of_mind, situation
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

    let person_actions_res: Result<Vec<PersonAction>, CompletionError> = tool_calls
        .into_iter()
        .map(|tool_call| PersonAction::from_open_ai_tool_call(tool_call))
        .collect::<Result<Vec<PersonAction>, PersonActionError>>()
        .map_err(Into::into);

    let person_actions = person_actions_res.map_err(Error::CompletionError)?;

    match person_actions.first() {
        None => Err(Error::NoPersonActionFound)?,
        Some(first) => {
            if person_actions.len() > 1 {
                Err(Error::MoreThanOnePersonActionFound(person_actions))?
            } else {
                Ok(first.clone())
            }
        }
    }
}
