use crate::capability::goal::GoalCapability;
use crate::capability::reaction::ReactionCapability;
use crate::domain::logger::Level;
use crate::domain::memory::Memory;
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::{Completion, CompletionError};
use crate::open_ai::role::Role;
use crate::open_ai::tool_call::ToolCall;
use crate::person_actions::{PersonAction, PersonActionError, PersonActionKind};
use crate::worker::Worker;

pub enum Error {
    CompletionError(CompletionError),
    FailedToGetGoals(String),
    NoPersonActionFound,
    MoreThanOnePersonActionFound(Vec<PersonAction>),
}

impl ReactionCapability for Worker {
    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonAction, String> {
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
            Error::FailedToGetGoals(message) => message,
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
) -> Result<PersonAction, Error> {
    let mut completion = Completion::new(open_ai::model::Model::DEFAULT);

    completion.add_message(Role::System, "You are a person simulation framework. You have deep insights into the human mind and are very good at predicting people's reactions to given situations. When given a description of a person, their state of mind, and some of their recent memories, respond as the person would in this situation by choosing exactly one tool call.");

    let memories_list = memories
        .iter()
        .map(|memory| format!("- {}", memory.content))
        .collect::<Vec<String>>()
        .join("\n");

    let goals = worker
        .get_goals_for_person(person_uuid)
        .await
        .map_err(Error::FailedToGetGoals)?;

    let goals_list = if goals.is_empty() {
        "None.".to_string()
    } else {
        goals
            .iter()
            .map(|goal| format!("- (priority {}) {}", goal.priority, goal.content))
            .collect::<Vec<String>>()
            .join("\n")
    };

    let user_prompt = format!(
        "Predict how this person would realistically and accurately behave in the situation, then choose exactly one action tool call that best matches that behavior.\n\nMemories:\n{}\n\nGoals:\n{}\n\nPerson identity: {}\n\nState of mind: {}\n\nSituation:\n{}",
        memories_list, goals_list, person_identity, state_of_mind, situation
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
                worker.logger.log(
                    Level::Info,
                    format!(
                        "Reaction for person {}: {}",
                        person_uuid.to_uuid(),
                        describe_action(first)
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
        PersonAction::Idle => "idle".to_string(),
        PersonAction::SayInScene { comment } => {
            format!("say in scene: {}", comment)
        }
    }
}
