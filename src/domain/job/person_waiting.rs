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
use crate::domain::event::Event;
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
    FailedToSummarizeRecentEvents(String),
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
            Error::FailedToSummarizeRecentEvents(err) => {
                format!("Failed to summarize recent events: {}", err)
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

    pub fn person_uuid(&self) -> Option<&PersonUuid> {
        self.person_uuid.as_ref()
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
            let recent_events_text = Event::many_to_prompt_list(events);
            let recent_events_summary = worker
                .summarize_reaction_events(recent_events_text)
                .await
                .map_err(Error::FailedToSummarizeRecentEvents)?;
            let reaction_situation = format!(
                "Recent events (older context):\n{}\n\n{}",
                recent_events_summary, situation
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
                    reaction_situation.clone(),
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
                reaction_situation,
                Some(action.summarize()),
            )
            .await?;

            Ok(WaitDecision::FinishedWaiting)
        } else {
            Ok(WaitDecision::ContinueWaiting)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::event::GetArgs;
    use crate::capability::job::JobCapability;
    use crate::capability::memory::{MemoryQueryPrompt, MemorySearchResult, NewMemory};
    use crate::capability::person::NewPerson;
    use crate::capability::person_identity::NewPersonIdentity;
    use crate::capability::person_task::NewPersonTask;
    use crate::capability::reaction::ReactionPromptPreview;
    use crate::capability::scene::{
        CurrentScene, NewScene, NewSceneSnapshot, Scene, SceneParticipant, SceneParticipation,
    };
    use crate::capability::state_of_mind::NewStateOfMind;
    use crate::domain::event::{Event, EventType};
    use crate::domain::job::{Job, JobKind, PoppedJob};
    use crate::domain::job_uuid::JobUuid;
    use crate::domain::memory_uuid::MemoryUuid;
    use crate::domain::message::Message;
    use crate::domain::message_uuid::MessageUuid;
    use crate::domain::person_identity_uuid::PersonIdentityUuid;
    use crate::domain::person_task::{PersonTask, PersonTaskTerminalOutcome};
    use crate::domain::person_task_uuid::PersonTaskUuid;
    use crate::domain::scene_participant_uuid::SceneParticipantUuid;
    use crate::domain::scene_uuid::SceneUuid;
    use crate::domain::state_of_mind_uuid::StateOfMindUuid;
    use crate::person_actions::{PersonAction, PersonReaction, ReflectionDecision};
    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct MockWorker {
        state: Arc<Mutex<MockState>>,
    }

    struct MockState {
        person_uuid: PersonUuid,
        scene_uuid: SceneUuid,
        events: Vec<Event>,
        latest_state_of_mind: Option<StateOfMind>,
        summarize_inputs: Vec<String>,
        reaction_situations: Vec<String>,
        jobs: Vec<JobKind>,
    }

    impl MockWorker {
        fn new() -> Self {
            let person_uuid = PersonUuid::new();
            let scene_uuid = SceneUuid::new();
            let events = vec![Event::new(
                Utc::now() - Duration::minutes(10),
                EventType::Said {
                    scene_name: "Cafe".to_string(),
                    speaker_name: "Counter".to_string(),
                    comment: "one".to_string(),
                    message_uuid: MessageUuid::new(),
                },
            )];

            Self {
                state: Arc::new(Mutex::new(MockState {
                    person_uuid,
                    scene_uuid,
                    events,
                    latest_state_of_mind: Some(StateOfMind {
                        content: "Focused on counting.".to_string(),
                    }),
                    summarize_inputs: vec![],
                    reaction_situations: vec![],
                    jobs: vec![],
                })),
            }
        }
    }

    impl ReactionCapability for MockWorker {
        async fn summarize_reaction_events(&self, events_text: String) -> Result<String, String> {
            let mut state = self.state.lock().await;
            state.summarize_inputs.push(events_text);
            Ok("OLDER SUMMARY".to_string())
        }

        async fn preview_reaction_prompts(
            &self,
            _memories: Vec<Memory>,
            _person_uuid: PersonUuid,
            _situation: String,
        ) -> Result<ReactionPromptPreview, String> {
            Ok(ReactionPromptPreview {
                thinking_system_prompt: "sys".to_string(),
                thinking_user_prompt: "user".to_string(),
                action_system_prompt: "action-sys".to_string(),
                action_user_prompt: "action-user".to_string(),
            })
        }

        async fn get_reaction(
            &self,
            _memories: Vec<Memory>,
            _person_uuid: PersonUuid,
            _state_of_mind: String,
            situation: String,
        ) -> Result<PersonReaction, String> {
            let mut state = self.state.lock().await;
            state.reaction_situations.push(situation);
            Ok(PersonReaction {
                action: PersonAction::Idle,
                reflection: ReflectionDecision::NoReflection,
            })
        }

        async fn classify_current_task_outcome(
            &self,
            _task: PersonTask,
            _situation: String,
            _action_summary: Option<String>,
        ) -> Result<PersonTaskOutcomeCheck, String> {
            Ok(PersonTaskOutcomeCheck::StillActive)
        }
    }

    impl JobCapability for MockWorker {
        async fn unshift_job(&self, job: JobKind) -> Result<(), String> {
            let mut state = self.state.lock().await;
            state.jobs.push(job);
            Ok(())
        }

        async fn pop_next_job(&self, _current_active_ms: i64) -> Result<Option<PoppedJob>, String> {
            Ok(None)
        }

        async fn recent_jobs(&self, _limit: i64) -> Result<Vec<Job>, String> {
            Ok(vec![])
        }

        async fn get_job_by_uuid(&self, _job_uuid: &JobUuid) -> Result<Option<Job>, String> {
            Ok(None)
        }

        async fn mark_job_finished(&self, _job_uuid: &JobUuid) -> Result<(), String> {
            Ok(())
        }

        async fn mark_job_failed(&self, _job_uuid: &JobUuid, _details: &str) -> Result<(), String> {
            Ok(())
        }

        async fn reset_job(&self, _job_uuid: &JobUuid) -> Result<(), String> {
            Ok(())
        }

        async fn reset_all_failed_jobs(&self) -> Result<(), String> {
            Ok(())
        }

        async fn delete_job(&self, _job_uuid: &JobUuid) -> Result<(), String> {
            Ok(())
        }
    }

    #[async_trait]
    impl SceneCapability for MockWorker {
        async fn create_scene(&self, _new_scene: NewScene) -> Result<SceneUuid, String> {
            Ok(SceneUuid::new())
        }

        async fn delete_scene(&self, _scene_uuid: &SceneUuid) -> Result<(), String> {
            Ok(())
        }

        async fn get_scenes(&self) -> Result<Vec<Scene>, String> {
            Ok(vec![])
        }

        async fn add_person_to_scene(
            &self,
            _scene_uuid: SceneUuid,
            _person_name: PersonName,
        ) -> Result<SceneParticipantUuid, String> {
            Ok(SceneParticipantUuid::new())
        }

        async fn remove_person_from_scene(
            &self,
            _scene_uuid: SceneUuid,
            _person_name: PersonName,
        ) -> Result<SceneParticipantUuid, String> {
            Ok(SceneParticipantUuid::new())
        }

        async fn get_persons_current_scene(
            &self,
            _person_name: PersonName,
        ) -> Result<Option<CurrentScene>, String> {
            Ok(None)
        }

        async fn get_persons_current_scene_uuid(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Option<SceneUuid>, String> {
            let state = self.state.lock().await;
            Ok(Some(state.scene_uuid.clone()))
        }

        async fn create_scene_snapshot(
            &self,
            _new_scene_snapshot: NewSceneSnapshot,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn get_scene_from_name(&self, _scene_name: String) -> Result<Option<Scene>, String> {
            Ok(None)
        }

        async fn get_scene_current_participants(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<Vec<SceneParticipant>, String> {
            Ok(vec![])
        }

        async fn get_scene_participation_history(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<Vec<SceneParticipation>, String> {
            Ok(vec![])
        }

        async fn set_real_world_user_in_scene(
            &self,
            _scene_uuid: &SceneUuid,
            _is_in_scene: bool,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn is_real_world_user_in_scene(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<bool, String> {
            Ok(false)
        }

        async fn get_scene_name(&self, _scene_uuid: &SceneUuid) -> Result<Option<String>, String> {
            Ok(Some("Cafe".to_string()))
        }

        async fn get_scene_description(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<Option<String>, String> {
            Ok(Some("A quiet cafe.".to_string()))
        }
    }

    impl MessageCapability for MockWorker {
        async fn send_scene_message(
            &self,
            _sender: MessageSender,
            _scene_uuid: SceneUuid,
            _content: String,
        ) -> Result<MessageUuid, String> {
            Ok(MessageUuid::new())
        }

        async fn add_scene_message_recipients(
            &self,
            _message_uuid: &MessageUuid,
            _recipients: Vec<PersonUuid>,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn get_messages_in_scene_page(
            &self,
            _scene_uuid: &SceneUuid,
            _limit: i64,
            _before_sent_at: Option<DateTime<Utc>>,
        ) -> Result<Vec<Message>, String> {
            Ok(vec![])
        }

        async fn get_message_by_uuid(
            &self,
            _message_uuid: &MessageUuid,
        ) -> Result<Option<Message>, String> {
            Ok(None)
        }

        async fn get_unhandled_scene_messages_for_person(
            &self,
            _person_uuid: &PersonUuid,
            _scene_uuid: &SceneUuid,
        ) -> Result<Vec<Message>, String> {
            Ok(vec![])
        }

        async fn mark_scene_messages_handled_for_person(
            &self,
            _person_uuid: &PersonUuid,
            _message_uuids: Vec<MessageUuid>,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    impl MemoryCapability for MockWorker {
        async fn create_memory(&self, _new_memory: NewMemory) -> Result<MemoryUuid, String> {
            Ok(MemoryUuid::new())
        }

        async fn maybe_create_memories_from_description(
            &self,
            _person_uuid: PersonUuid,
            _description: String,
        ) -> Result<Vec<MemoryUuid>, String> {
            Ok(vec![])
        }

        async fn create_memory_query_prompt(
            &self,
            _person_recalling: &PersonName,
            _message_type_args: MessageTypeArgs,
            _recent_events: Vec<String>,
            _state_of_mind: &str,
            _situation: &str,
        ) -> Result<MemoryQueryPrompt, String> {
            Ok(MemoryQueryPrompt {
                prompt: "memory-query".to_string(),
            })
        }

        async fn search_memories(
            &self,
            _person_uuid: PersonUuid,
            _query: String,
            _limit: i64,
        ) -> Result<Vec<MemorySearchResult>, String> {
            Ok(vec![])
        }
    }

    impl PersonCapability for MockWorker {
        async fn create_person(&self, _new_person: NewPerson) -> Result<PersonUuid, String> {
            Ok(PersonUuid::new())
        }

        async fn get_all_person_uuids(&self) -> Result<Vec<PersonUuid>, String> {
            Ok(vec![])
        }

        async fn get_persons_name(&self, person_uuid: PersonUuid) -> Result<PersonName, String> {
            let state = self.state.lock().await;
            if person_uuid.to_uuid() == state.person_uuid.to_uuid() {
                Ok(PersonName::from_string("Counter".to_string()))
            } else {
                Err("unknown person".to_string())
            }
        }

        async fn get_person_uuid_by_name(
            &self,
            _person_name: PersonName,
        ) -> Result<PersonUuid, String> {
            Ok(PersonUuid::new())
        }

        async fn set_person_hibernating(
            &self,
            _person_uuid: &PersonUuid,
            _is_hibernating: bool,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn is_person_hibernating(&self, _person_uuid: &PersonUuid) -> Result<bool, String> {
            Ok(false)
        }

        async fn set_person_enabled(
            &self,
            _person_uuid: &PersonUuid,
            _is_enabled: bool,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn is_person_enabled(&self, _person_uuid: &PersonUuid) -> Result<bool, String> {
            Ok(true)
        }
    }

    impl EventCapability for MockWorker {
        async fn get_events(&self, _args: GetArgs) -> Result<Vec<Event>, String> {
            let state = self.state.lock().await;
            Ok(state.events.clone())
        }
    }

    #[async_trait]
    impl StateOfMindCapability for MockWorker {
        async fn create_state_of_mind(
            &self,
            _new_state_of_mind: NewStateOfMind,
        ) -> Result<StateOfMindUuid, String> {
            Ok(StateOfMindUuid::new())
        }

        async fn get_latest_state_of_mind(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Option<StateOfMind>, String> {
            let state = self.state.lock().await;
            Ok(state
                .latest_state_of_mind
                .as_ref()
                .map(|state_of_mind| StateOfMind {
                    content: state_of_mind.content.clone(),
                }))
        }
    }

    #[async_trait]
    impl PersonIdentityCapability for MockWorker {
        async fn summarize_person_identity(
            &self,
            _person_name: &str,
            _identity: &str,
        ) -> Result<String, String> {
            Ok("summary".to_string())
        }

        async fn create_person_identity(
            &self,
            _new_person_identity: NewPersonIdentity,
        ) -> Result<PersonIdentityUuid, String> {
            Ok(PersonIdentityUuid::new())
        }

        async fn get_person_identity(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Option<String>, String> {
            Ok(None)
        }

        async fn get_person_identity_summary(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Option<String>, String> {
            Ok(None)
        }
    }

    impl PersonTaskCapability for MockWorker {
        async fn get_persons_current_active_task(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Option<PersonTask>, String> {
            Ok(None)
        }

        async fn set_persons_current_active_task(
            &self,
            _new_person_task: NewPersonTask,
        ) -> Result<PersonTaskUuid, String> {
            Ok(PersonTaskUuid::new())
        }

        async fn transition_person_task(
            &self,
            _person_uuid: &PersonUuid,
            _person_task_uuid: &PersonTaskUuid,
            _outcome: PersonTaskTerminalOutcome,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    impl ReactionHistoryCapability for MockWorker {
        async fn record_reaction(
            &self,
            _person_uuid: &PersonUuid,
            _action_kind: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn has_reacted_since(
            &self,
            _person_uuid: &PersonUuid,
            _since: DateTime<Utc>,
        ) -> Result<bool, String> {
            Ok(false)
        }
    }

    impl LogCapability for MockWorker {
        fn log(&self, _level: crate::domain::logger::Level, _message: &str) {}
    }

    #[tokio::test]
    async fn wait_job_reaction_includes_recent_events_context() {
        let worker = MockWorker::new();
        let state = worker.state.lock().await;
        let person_uuid = state.person_uuid.clone();
        drop(state);

        let wait_job = PersonWaitingJob::new(person_uuid.clone(), 60_000, 0);

        let result = wait_job.run(&worker, RandomSeed::from_u64(1), 60_000).await;

        match result {
            Ok(WaitDecision::FinishedWaiting) => {}
            Ok(WaitDecision::ContinueWaiting) => panic!("wait job should have finished"),
            Err(err) => panic!("wait job should succeed: {}", err.message()),
        }

        let state = worker.state.lock().await;
        assert_eq!(state.summarize_inputs.len(), 1);
        assert!(state.summarize_inputs[0].contains("Counter said: \"one\""));

        assert_eq!(state.reaction_situations.len(), 1);
        let situation = &state.reaction_situations[0];
        assert!(situation.contains("Recent events (older context):"));
        assert!(situation.contains("OLDER SUMMARY"));
        assert!(situation.contains("nothing happened"));
        assert!(situation.contains("There were no new messages or events"));
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
