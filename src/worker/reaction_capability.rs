use crate::capability::motivation::MotivationCapability;
use crate::capability::reaction::ReactionCapability;
use crate::domain::logger::Level;
use crate::domain::memory::Memory;
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
    let mut completion = Completion::new(open_ai::model::Model::DEFAULT);

    completion.add_message(Role::System, "You are a person simulation framework. You have deep insights into the human mind and are very good at predicting people's reactions to given situations. When given a description of a person, their state of mind, and some of their recent memories, respond as the person would in this situation by choosing exactly one tool call.");

    let memories_list = memories
        .iter()
        .map(|memory| format!("- {}", memory.content))
        .collect::<Vec<String>>()
        .join("\n");

    let motivations = worker
        .get_motivations_for_person(person_uuid.clone())
        .await
        .map_err(Error::FailedToGetMotivations)?;

    let motivations_list = if motivations.is_empty() {
        "None.".to_string()
    } else {
        motivations
            .iter()
            .map(|motivation| {
                format!(
                    "- (priority {}) {}",
                    motivation.priority, motivation.content
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    };

    let user_prompt = format!(
        "Predict how this person would realistically and accurately behave in the situation, then choose exactly one action tool call that best matches that behavior.\n\nMemories:\n{}\n\nMotivations:\n{}\n\nPerson identity: {}\n\nState of mind: {}\n\nSituation:\n{}",
        memories_list, motivations_list, person_identity, state_of_mind, situation
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

    let person_actions_res: Result<Vec<PersonReaction>, CompletionError> = tool_calls
        .into_iter()
        .map(|tool_call| PersonReaction::from_open_ai_tool_call(tool_call))
        .collect::<Result<Vec<PersonReaction>, PersonActionError>>()
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
        PersonAction::Idle => "idle".to_string(),
        PersonAction::SayInScene { comment } => {
            format!("say in scene: {}", comment)
        }
    }
}

fn describe_reflection(reflection: &ReflectionDecision) -> String {
    match reflection {
        ReflectionDecision::Reflection => "reflection".to_string(),
        ReflectionDecision::NoReflection => "no_reflection".to_string(),
    }
}
