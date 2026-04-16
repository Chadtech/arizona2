use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
use crate::capability::logging::LogCapability;
use crate::capability::memory::{MemoryCapability, MessageTypeArgs};
use crate::capability::message::MessageCapability;
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::person_task::PersonTaskCapability;
use crate::capability::reaction::ReactionCapability;
use crate::capability::reaction_history::ReactionHistoryCapability;
use crate::capability::scene::SceneCapability;
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::job::person_action_handler::{self, ActionHandleError};
use crate::domain::memory::Memory;
use crate::domain::message::MessageSender;
use crate::domain::person_name::PersonName;
use crate::domain::person_task::PersonTaskOutcomeCheck;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::state_of_mind::StateOfMind;
use crate::nice_display::NiceDisplay;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonWaitingJob {
    #[serde(default)]
    person_uuid: Option<PersonUuid>,
    #[serde(default)]
    started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    duration_ms: i64,
    #[serde(default)]
    start_active_ms: i64,
}

pub enum Error {
    MissingPersonUuid,
    MissingStartedAt,
    FailedToGetHibernationState(String),
    FailedToGetEnabledState(String),
    FailedToGetEvents(String),
    FailedToGetReactionHistory(String),
    FailedToGetStateOfMind(String),
    NoStateOfMindFound {
        person_uuid: PersonUuid,
    },
    FailedToGetPersonsName(String),
    CouldNotCreateMemoriesPrompt(String),
    FailedToSearchMemories(String),
    GetPersonReaction(String),
    CouldNotGetPersonsScene {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToGetCurrentTask(String),
    TaskOutcomeClassification(String),
    TaskTransition(String),
    Action(ActionHandleError),
}

pub enum WaitDecision {
    FinishedWaiting,
    ContinueWaiting,
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::MissingPersonUuid => "Missing person uuid on wait job".to_string(),
            Error::MissingStartedAt => "Missing started_at on wait job".to_string(),
            Error::FailedToGetHibernationState(err) => {
                format!("Failed to get hibernation state: {}", err)
            }
            Error::FailedToGetEnabledState(err) => {
                format!("Failed to get enabled state: {}", err)
            }
            Error::FailedToGetEvents(err) => {
                format!("Failed to get events: {}", err)
            }
            Error::FailedToGetReactionHistory(err) => {
                format!("Failed to get reaction history: {}", err)
            }
            Error::FailedToGetStateOfMind(err) => {
                format!("Failed to get state of mind: {}", err)
            }
            Error::NoStateOfMindFound { person_uuid } => {
                format!(
                    "No state of mind found for person {}",
                    person_uuid.to_uuid()
                )
            }
            Error::FailedToGetPersonsName(err) => {
                format!("Failed to get person's name: {}", err)
            }
            Error::CouldNotCreateMemoriesPrompt(err) => {
                format!("Could not create memories prompt: {}", err)
            }
            Error::FailedToSearchMemories(err) => {
                format!("Failed to search memories: {}", err)
            }
            Error::GetPersonReaction(err) => {
                format!("Failed to get person reaction: {}", err)
            }
            Error::CouldNotGetPersonsScene {
                person_uuid,
                details,
            } => {
                format!(
                    "Could not get current scene for person {}: {}",
                    person_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToGetCurrentTask(err) => {
                format!("Failed to get current task: {}", err)
            }
            Error::TaskOutcomeClassification(err) => {
                format!("Task outcome classification failed: {}", err)
            }
            Error::TaskTransition(err) => {
                format!("Task transition failed: {}", err)
            }
            Error::Action(err) => err.to_nice_error().to_string(),
        }
    }
}

impl PersonWaitingJob {
    pub fn new(person_uuid: PersonUuid, duration_ms: i64, start_active_ms: i64) -> Self {
        Self {
            person_uuid: Some(person_uuid),
            started_at: Some(Utc::now()),
            duration_ms: duration_ms.max(0),
            start_active_ms: start_active_ms.max(0),
        }
    }

    pub fn run_at_active_ms(&self) -> i64 {
        self.start_active_ms.saturating_add(self.duration_ms.max(0))
    }

    pub async fn run<
        W: JobCapability
            + SceneCapability
            + ReactionCapability
            + MessageCapability
            + MemoryCapability
            + PersonCapability
            + EventCapability
            + StateOfMindCapability
            + PersonIdentityCapability
            + PersonTaskCapability
            + ReactionHistoryCapability
            + LogCapability
            + Sync,
    >(
        &self,
        worker: &W,
        random_seed: RandomSeed,
        current_active_ms: i64,
    ) -> Result<WaitDecision, Error> {
        let person_uuid = self.person_uuid.clone().ok_or(Error::MissingPersonUuid)?;
        let started_at = self.started_at.ok_or(Error::MissingStartedAt)?;
        let is_hibernating = worker
            .is_person_hibernating(&person_uuid)
            .await
            .map_err(Error::FailedToGetHibernationState)?;
        if is_hibernating {
            return Ok(WaitDecision::FinishedWaiting);
        }

        let is_enabled = worker
            .is_person_enabled(&person_uuid)
            .await
            .map_err(Error::FailedToGetEnabledState)?;
        if !is_enabled {
            return Ok(WaitDecision::FinishedWaiting);
        }

        let elapsed = current_active_ms.saturating_sub(self.start_active_ms);
        if elapsed >= self.duration_ms {
            let get_args =
                crate::capability::event::GetArgs::new().with_person_uuid(person_uuid.clone());
            let events = worker
                .get_events(get_args)
                .await
                .map_err(Error::FailedToGetEvents)?;
            let has_recent_events = events.iter().any(|event| event.timestamp >= started_at);

            if has_recent_events {
                return Ok(WaitDecision::FinishedWaiting);
            }

            let reacted_since_wait = worker
                .has_reacted_since(&person_uuid, started_at)
                .await
                .map_err(Error::FailedToGetReactionHistory)?;

            if reacted_since_wait {
                return Ok(WaitDecision::FinishedWaiting);
            }

            let persons_name: PersonName = worker
                .get_persons_name(person_uuid.clone())
                .await
                .map_err(Error::FailedToGetPersonsName)?;

            let scene_uuid = worker
                .get_persons_current_scene_uuid(&person_uuid)
                .await
                .map_err(|err| Error::CouldNotGetPersonsScene {
                    person_uuid: person_uuid.clone(),
                    details: err,
                })?;

            let message_type_args = match scene_uuid.clone() {
                Some(scene_uuid) => MessageTypeArgs::SceneByUuid { scene_uuid },
                None => MessageTypeArgs::Direct {
                    from: MessageSender::RealWorldUser,
                },
            };

            let maybe_state_of_mind: Option<StateOfMind> = worker
                .get_latest_state_of_mind(&person_uuid)
                .await
                .map_err(Error::FailedToGetStateOfMind)?;

            let state_of_mind: StateOfMind = match maybe_state_of_mind {
                Some(som) => som,
                None => Err(Error::NoStateOfMindFound {
                    person_uuid: person_uuid.clone(),
                })?,
            };

            let events_text = events
                .iter()
                .map(|event| event.to_text())
                .collect::<Vec<String>>();

            let minutes_waiting = (self.duration_ms / 60000).max(0);
            let situation = format!(
                "{} decided to wait {} minutes, and nothing happened. There were no new messages or events. It is okay to do nothing and keep waiting silently.",
                persons_name.as_str(),
                minutes_waiting
            );

            let memories_prompt = worker
                .create_memory_query_prompt(
                    &persons_name,
                    message_type_args,
                    events_text,
                    &state_of_mind.content,
                    &situation,
                )
                .await
                .map_err(Error::CouldNotCreateMemoriesPrompt)?;

            let memories: Vec<Memory> = crate::domain::memory::filter_memory_results(
                worker
                    .search_memories(person_uuid.clone(), memories_prompt.prompt, 5)
                    .await
                    .map_err(Error::FailedToSearchMemories)?,
            );

            let reaction = worker
                .get_reaction(
                    memories,
                    person_uuid.clone(),
                    state_of_mind.content,
                    situation.clone(),
                )
                .await
                .map_err(Error::GetPersonReaction)?;

            let action = reaction.action;

            person_action_handler::handle_person_action(
                worker,
                &action,
                &person_uuid,
                random_seed.clone(),
                current_active_ms,
            )
            .await
            .map_err(Error::Action)?;

            maybe_transition_current_task(
                worker,
                &person_uuid,
                situation,
                Some(action.summarize()),
            )
            .await?;

            Ok(WaitDecision::FinishedWaiting)
        } else {
            Ok(WaitDecision::ContinueWaiting)
        }
    }
}

async fn maybe_transition_current_task<W: PersonTaskCapability + ReactionCapability + Sync>(
    worker: &W,
    person_uuid: &PersonUuid,
    situation: String,
    action_summary: Option<String>,
) -> Result<(), Error> {
    let maybe_current_task = worker
        .get_persons_current_active_task(person_uuid)
        .await
        .map_err(Error::FailedToGetCurrentTask)?;

    let current_task = match maybe_current_task {
        Some(current_task) => current_task,
        None => return Ok(()),
    };

    let outcome = worker
        .classify_current_task_outcome(current_task.clone(), situation, action_summary)
        .await
        .map_err(Error::TaskOutcomeClassification)?;

    match outcome {
        PersonTaskOutcomeCheck::StillActive => Ok(()),
        PersonTaskOutcomeCheck::Terminal(outcome) => worker
            .transition_person_task(person_uuid, &current_task.uuid, outcome)
            .await
            .map_err(Error::TaskTransition),
    }
}
