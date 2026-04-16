use crate::capability;
use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
use crate::capability::log_event::LogEventCapability;
use crate::capability::logging::LogCapability;
use crate::capability::memory::NewMemory;
use crate::capability::memory::{MemoryCapability, MessageTypeArgs};
use crate::capability::motivation::MotivationCapability;
use crate::capability::motivation::NewMotivation;
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::person_task::PersonTaskCapability;
use crate::capability::reaction::ReactionCapability;
use crate::capability::reaction::ReactionPromptPreview;
use crate::capability::reaction_history::ReactionHistoryCapability;
use crate::capability::reflection::ReflectionCapability;
use crate::capability::reflection::ReflectionChange;
use crate::capability::state_of_mind::NewStateOfMind;
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::event::{Event, EventType};
use crate::domain::job::person_action_handler;
use crate::domain::job::person_action_handler::ActionHandleError;
use crate::domain::memory::Memory;
use crate::domain::memory_uuid::MemoryUuid;
use crate::domain::message::{Message, MessageSender};
use crate::domain::person_name::PersonName;
use crate::domain::person_task::PersonTaskOutcomeCheck;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::domain::situation;
use crate::domain::situation::Situation;
use crate::domain::state_of_mind::StateOfMind;
use crate::domain::state_of_mind_uuid::StateOfMindUuid;
use crate::nice_display::NiceDisplay;
use crate::person_actions::ReflectionDecision;
use crate::text_utils::normalize_message_content;
use crate::{capability::message::MessageCapability, capability::scene::SceneCapability};
use std::collections::HashSet;

struct ReflectionInput {
    person_name: PersonName,
    memories: Vec<Memory>,
    person_identity: String,
    state_of_mind: String,
}

struct ReactionExecutionInput {
    situation: Situation,
    reflection_input: ReflectionInput,
    reaction_situation: String,
    description_prefix: Option<String>,
}

pub enum SceneReactionTrigger {
    NewMessages,
    PersonJoined { joined_person_uuid: PersonUuid },
    SceneDescriptionGaze,
}

pub enum Error {
    GetPersonReaction(String),
    FailedToGetEvents(String),
    FailedToGetStateOfMind(String),
    NoStateOfMindFound {
        person_uuid: PersonUuid,
    },
    CouldNotCreateMemoriesPrompt(String),
    FailedToSearchMemories(String),
    FailedToGetPersonIdentity(String),
    NoPersonIdentityFound {
        person_uuid: PersonUuid,
    },
    FailedToGetSendersName {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToGetPersonsName(String),
    FailedToGetSceneParticipants {
        scene_uuid: SceneUuid,
        details: String,
    },
    FailedToGetSceneName {
        scene_uuid: SceneUuid,
        details: String,
    },
    SceneNameNotFound {
        scene_uuid: SceneUuid,
    },
    FailedToGetSceneDescription {
        scene_uuid: SceneUuid,
        details: String,
    },
    SceneDescriptionNotFound {
        scene_uuid: SceneUuid,
    },
    FailedToGetUnhandledSceneMessages {
        scene_uuid: SceneUuid,
        details: String,
    },
    FailedToMarkSceneMessagesHandled {
        scene_uuid: SceneUuid,
        details: String,
    },
    FailedToGetHibernationState {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToGetEnabledState {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToCreateMemory(String),
    FailedToCreateReflectionStateOfMind(String),
    FailedToCreateReflectionMemory(String),
    FailedToCreateReflectionMotivation(String),
    FailedToDeleteReflectionMotivation(String),
    FailedToGetCurrentTask(String),
    TaskOutcomeClassification(String),
    TaskTransition(String),
    Action(ActionHandleError),
    Reflection(String),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::GetPersonReaction(err) => {
                format!("Failed to get person reaction: {}", err)
            }
            Error::FailedToGetEvents(err) => {
                format!("Failed to get events: {}", err)
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
            Error::CouldNotCreateMemoriesPrompt(err) => {
                format!("Could not create memories prompt: {}", err)
            }
            Error::FailedToSearchMemories(err) => {
                format!("Failed to search memories: {}", err)
            }
            Error::FailedToGetPersonIdentity(err) => {
                format!("Failed to get person identity: {}", err)
            }
            Error::NoPersonIdentityFound { person_uuid } => {
                format!(
                    "No person identity found for person {}",
                    person_uuid.to_uuid()
                )
            }
            Error::FailedToGetSendersName {
                person_uuid,
                details,
            } => {
                format!(
                    "Failed to get person's name for {}: {}",
                    person_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToGetPersonsName(err) => {
                format!("Failed to get person's name: {}", err)
            }
            Error::FailedToGetSceneParticipants {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get scene participants for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToGetSceneName {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get scene name for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::SceneNameNotFound { scene_uuid } => {
                format!("Scene name not found for {}", scene_uuid.to_uuid())
            }
            Error::FailedToGetSceneDescription {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get scene description for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::SceneDescriptionNotFound { scene_uuid } => {
                format!("Scene description not found for {}", scene_uuid.to_uuid())
            }
            Error::FailedToGetUnhandledSceneMessages {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get unhandled scene messages for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToMarkSceneMessagesHandled {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to mark scene messages handled for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToGetHibernationState {
                person_uuid,
                details,
            } => {
                format!(
                    "Failed to get hibernation state for {}: {}",
                    person_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToGetEnabledState {
                person_uuid,
                details,
            } => {
                format!(
                    "Failed to get enabled state for {}: {}",
                    person_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToCreateMemory(err) => {
                format!("Failed to create memory:\n{}", err)
            }
            Error::FailedToCreateReflectionStateOfMind(err) => {
                format!("Failed to create reflection state of mind:\n{}", err)
            }
            Error::FailedToCreateReflectionMemory(err) => {
                format!("Failed to create reflection memory:\n{}", err)
            }
            Error::FailedToCreateReflectionMotivation(err) => {
                format!("Failed to create reflection motivation:\n{}", err)
            }
            Error::FailedToDeleteReflectionMotivation(err) => {
                format!("Failed to delete reflection motivation:\n{}", err)
            }
            Error::FailedToGetCurrentTask(err) => {
                format!("Failed to get current task:\n{}", err)
            }
            Error::TaskOutcomeClassification(err) => {
                format!("Task outcome classification failed:\n{}", err)
            }
            Error::TaskTransition(err) => {
                format!("Task transition failed:\n{}", err)
            }
            Error::Action(err) => err.to_nice_error().to_string(),
            Error::Reflection(err) => {
                format!("Reflection error:\n{}", err)
            }
        }
    }
}

pub async fn run_scene_reaction<
    W: MessageCapability
        + SceneCapability
        + ReactionCapability
        + MemoryCapability
        + PersonCapability
        + EventCapability
        + StateOfMindCapability
        + PersonIdentityCapability
        + ReflectionCapability
        + LogCapability
        + LogEventCapability
        + MotivationCapability
        + ReactionHistoryCapability
        + PersonTaskCapability
        + JobCapability
        + Sync,
>(
    worker: &W,
    person_uuid: &PersonUuid,
    scene_uuid: &SceneUuid,
    trigger: SceneReactionTrigger,
    random_seed: RandomSeed,
    current_active_ms: i64,
) -> Result<(), Error> {
    let pending_messages = match trigger {
        SceneReactionTrigger::NewMessages => worker
            .get_unhandled_scene_messages_for_person(person_uuid, scene_uuid)
            .await
            .map_err(|err| Error::FailedToGetUnhandledSceneMessages {
                scene_uuid: scene_uuid.clone(),
                details: err,
            })?,
        SceneReactionTrigger::PersonJoined { .. } => vec![],
        SceneReactionTrigger::SceneDescriptionGaze => vec![],
    };

    let is_enabled = worker.is_person_enabled(person_uuid).await.map_err(|err| {
        Error::FailedToGetEnabledState {
            person_uuid: person_uuid.clone(),
            details: err,
        }
    })?;

    if !is_enabled {
        if !pending_messages.is_empty() {
            let handled_ids = pending_messages
                .iter()
                .map(|msg| msg.uuid.clone())
                .collect::<Vec<_>>();

            worker
                .mark_scene_messages_handled_for_person(person_uuid, handled_ids)
                .await
                .map_err(|err| Error::FailedToMarkSceneMessagesHandled {
                    scene_uuid: scene_uuid.clone(),
                    details: err,
                })?;
        }

        let skip_reason = match trigger {
            SceneReactionTrigger::NewMessages => "Skipping reaction",
            SceneReactionTrigger::PersonJoined { .. } => "Skipping join reaction",
            SceneReactionTrigger::SceneDescriptionGaze => "Skipping scene gaze reaction",
        };
        tracing::info!(
            "{} for person {} in scene {}: person is disabled",
            skip_reason,
            person_uuid.to_uuid(),
            scene_uuid.to_uuid()
        );
        return Ok(());
    }

    let is_hibernating = worker
        .is_person_hibernating(person_uuid)
        .await
        .map_err(|err| Error::FailedToGetHibernationState {
            person_uuid: person_uuid.clone(),
            details: err,
        })?;

    if is_hibernating {
        if !pending_messages.is_empty() {
            let handled_ids = pending_messages
                .iter()
                .map(|msg| msg.uuid.clone())
                .collect::<Vec<_>>();

            worker
                .mark_scene_messages_handled_for_person(person_uuid, handled_ids)
                .await
                .map_err(|err| Error::FailedToMarkSceneMessagesHandled {
                    scene_uuid: scene_uuid.clone(),
                    details: err,
                })?;
        }

        let skip_reason = match trigger {
            SceneReactionTrigger::NewMessages => "Skipping reaction",
            SceneReactionTrigger::PersonJoined { .. } => "Skipping join reaction",
            SceneReactionTrigger::SceneDescriptionGaze => "Skipping scene gaze reaction",
        };
        tracing::info!(
            "{} for person {} in scene {}: person is hibernating",
            skip_reason,
            person_uuid.to_uuid(),
            scene_uuid.to_uuid()
        );
        return Ok(());
    }

    let is_new_messages_trigger = match &trigger {
        SceneReactionTrigger::NewMessages => true,
        SceneReactionTrigger::PersonJoined { .. } => false,
        SceneReactionTrigger::SceneDescriptionGaze => false,
    };

    if is_new_messages_trigger && pending_messages.is_empty() {
        tracing::info!(
            "Skipping reaction for person {} in scene {}: no new messages",
            person_uuid.to_uuid(),
            scene_uuid.to_uuid()
        );
        return Ok(());
    }

    let reaction_input = build_reaction_execution_input(
        worker,
        person_uuid,
        scene_uuid,
        &trigger,
        &pending_messages,
    )
    .await?;

    let reaction = worker
        .get_reaction(
            reaction_input.reflection_input.memories.clone(),
            person_uuid.clone(),
            reaction_input.reflection_input.state_of_mind.clone(),
            reaction_input.reaction_situation,
        )
        .await
        .map_err(Error::GetPersonReaction)?;

    let action = reaction.action;

    person_action_handler::handle_person_action(
        worker,
        &action,
        person_uuid,
        random_seed,
        current_active_ms,
    )
    .await
    .map_err(Error::Action)?;

    maybe_transition_current_task(
        worker,
        person_uuid,
        reaction_input.situation.to_string(),
        Some(action.summarize()),
    )
    .await?;

    match reaction.reflection {
        ReflectionDecision::Reflection => {
            let reflection_recent_events = get_recent_events_text(
                worker,
                MessageTypeArgs::SceneByUuid {
                    scene_uuid: scene_uuid.clone(),
                },
                person_uuid,
            )
            .await?;

            let reflection_situation = format!(
                "{}\n\nRecent events:\n{}",
                reaction_input.situation,
                Event::many_to_prompt_list(reflection_recent_events)
            );
            let changes = worker
                .get_reflection_changes(
                    reaction_input.reflection_input.memories.clone(),
                    person_uuid.clone(),
                    reaction_input.reflection_input.person_identity.clone(),
                    reaction_input.reflection_input.state_of_mind.clone(),
                    reflection_situation,
                )
                .await
                .map_err(Error::Reflection)?;

            apply_reflection_changes(
                worker,
                person_uuid,
                &reaction_input.reflection_input,
                changes,
            )
            .await?;
        }
        ReflectionDecision::NoReflection => {}
    }

    let description = match reaction_input.description_prefix {
        Some(prefix) => format!(
            "{}\n\n{}\n\nResponse:\n{}",
            reaction_input.situation,
            prefix,
            action.summarize()
        ),
        None => format!(
            "{}\n\nResponse:\n{}",
            reaction_input.situation,
            action.summarize()
        ),
    };

    worker
        .maybe_create_memories_from_description(person_uuid.clone(), description)
        .await
        .map_err(Error::FailedToCreateMemory)?;

    if is_new_messages_trigger {
        let handled_ids = pending_messages
            .into_iter()
            .map(|msg| msg.uuid)
            .collect::<Vec<_>>();

        worker
            .mark_scene_messages_handled_for_person(person_uuid, handled_ids)
            .await
            .map_err(|err| Error::FailedToMarkSceneMessagesHandled {
                scene_uuid: scene_uuid.clone(),
                details: err,
            })?;
    }

    Ok(())
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

pub async fn preview_scene_reaction_prompts<
    W: MessageCapability
        + SceneCapability
        + ReactionCapability
        + MemoryCapability
        + PersonCapability
        + EventCapability
        + StateOfMindCapability
        + PersonIdentityCapability
        + MotivationCapability
        + Sync,
>(
    worker: &W,
    person_uuid: &PersonUuid,
    scene_uuid: &SceneUuid,
    trigger: SceneReactionTrigger,
) -> Result<ReactionPromptPreview, Error> {
    let pending_messages = match trigger {
        SceneReactionTrigger::NewMessages => worker
            .get_unhandled_scene_messages_for_person(person_uuid, scene_uuid)
            .await
            .map_err(|err| Error::FailedToGetUnhandledSceneMessages {
                scene_uuid: scene_uuid.clone(),
                details: err,
            })?,
        SceneReactionTrigger::PersonJoined { .. } => vec![],
        SceneReactionTrigger::SceneDescriptionGaze => vec![],
    };

    let reaction_input = build_reaction_execution_input(
        worker,
        person_uuid,
        scene_uuid,
        &trigger,
        &pending_messages,
    )
    .await?;

    worker
        .preview_reaction_prompts(
            reaction_input.reflection_input.memories,
            person_uuid.clone(),
            reaction_input.reaction_situation,
        )
        .await
        .map_err(Error::GetPersonReaction)
}

async fn build_reaction_execution_input<
    W: MessageCapability
        + SceneCapability
        + MemoryCapability
        + PersonCapability
        + EventCapability
        + ReactionCapability
        + StateOfMindCapability
        + PersonIdentityCapability
        + Sync,
>(
    worker: &W,
    person_uuid: &PersonUuid,
    scene_uuid: &SceneUuid,
    trigger: &SceneReactionTrigger,
    pending_messages: &[Message],
) -> Result<ReactionExecutionInput, Error> {
    let include_scene_context = match trigger {
        SceneReactionTrigger::SceneDescriptionGaze => true,
        SceneReactionTrigger::NewMessages => false,
        SceneReactionTrigger::PersonJoined { .. } => false,
    };
    let situation = build_scene_situation(
        worker,
        scene_uuid,
        pending_messages,
        person_uuid,
        include_scene_context,
    )
    .await?;
    let prompt_situation_messages = match trigger {
        SceneReactionTrigger::NewMessages => &[],
        SceneReactionTrigger::PersonJoined { .. } => pending_messages,
        SceneReactionTrigger::SceneDescriptionGaze => &[],
    };
    let prompt_situation = build_scene_situation(
        worker,
        scene_uuid,
        prompt_situation_messages,
        person_uuid,
        include_scene_context,
    )
    .await?;
    let prompt_situation_text = match trigger {
        SceneReactionTrigger::NewMessages => prompt_situation.to_people_present_text(),
        SceneReactionTrigger::PersonJoined { .. } => prompt_situation.to_string(),
        SceneReactionTrigger::SceneDescriptionGaze => prompt_situation.to_people_present_text(),
    };

    let reflection_input = build_reflection_input(
        worker,
        MessageTypeArgs::SceneByUuid {
            scene_uuid: scene_uuid.clone(),
        },
        &situation,
        person_uuid,
    )
    .await?;

    let reaction_recent_events = get_recent_events_text(
        worker,
        MessageTypeArgs::SceneByUuid {
            scene_uuid: scene_uuid.clone(),
        },
        person_uuid,
    )
    .await?;

    let reaction_events = filter_reaction_events(reaction_recent_events, pending_messages);
    let recent_events_text = Event::many_to_prompt_list(reaction_events);
    let recent_events_summary = worker
        .summarize_reaction_events(recent_events_text)
        .await
        .map_err(Error::GetPersonReaction)?;

    let priority_instruction = match trigger {
        SceneReactionTrigger::NewMessages => {
            "React to the newest activity first. Prioritize the NEW MESSAGE EVENT lines below when deciding what to do now."
        }
        SceneReactionTrigger::PersonJoined { .. } => {
            "React to the newest activity first. Prioritize the NEW JOIN EVENT lines below when deciding what to do now."
        }
        SceneReactionTrigger::SceneDescriptionGaze => {
            "React to the current scene description first. Prioritize the SCENE GAZE EVENT lines below when deciding what to do now."
        }
    };

    let new_event_section_label = match trigger {
        SceneReactionTrigger::NewMessages => {
            "New message events (newest; primary reaction target):"
        }
        SceneReactionTrigger::PersonJoined { .. } => {
            "New join events (newest; primary reaction target):"
        }
        SceneReactionTrigger::SceneDescriptionGaze => "Scene gaze event (primary reaction target):",
    };

    let new_event_section_text = match trigger {
        SceneReactionTrigger::NewMessages => {
            let new_message_event_lines =
                pending_messages_to_event_lines(worker, pending_messages, person_uuid).await?;
            if new_message_event_lines.is_empty() {
                "None.".to_string()
            } else {
                new_message_event_lines.join("\n")
            }
        }
        SceneReactionTrigger::PersonJoined { joined_person_uuid } => {
            let joined_person_name = worker
                .get_persons_name(joined_person_uuid.clone())
                .await
                .map_err(Error::FailedToGetPersonsName)?;
            format!(
                "In the current scene, {} joined the scene [NEW JOIN EVENT]",
                joined_person_name.as_str()
            )
        }
        SceneReactionTrigger::SceneDescriptionGaze => {
            let scene_name = match worker.get_scene_name(scene_uuid).await.map_err(|err| {
                Error::FailedToGetSceneName {
                    scene_uuid: scene_uuid.clone(),
                    details: err,
                }
            })? {
                Some(scene_name) => scene_name,
                None => {
                    return Err(Error::SceneNameNotFound {
                        scene_uuid: scene_uuid.clone(),
                    })
                }
            };
            let scene_description =
                match worker
                    .get_scene_description(scene_uuid)
                    .await
                    .map_err(|err| Error::FailedToGetSceneDescription {
                        scene_uuid: scene_uuid.clone(),
                        details: err,
                    })? {
                    Some(scene_description) => scene_description,
                    None => {
                        return Err(Error::SceneDescriptionNotFound {
                            scene_uuid: scene_uuid.clone(),
                        })
                    }
                };
            format!(
                "In the current scene \"{}\", the environment is described as:\n{}\n[SCENE GAZE EVENT]",
                scene_name, scene_description
            )
        }
    };

    let description_prefix = match trigger {
        SceneReactionTrigger::NewMessages => None,
        SceneReactionTrigger::PersonJoined { .. } => {
            Some(format!("Join event:\n{}", new_event_section_text))
        }
        SceneReactionTrigger::SceneDescriptionGaze => {
            Some(format!("Scene gaze event:\n{}", new_event_section_text))
        }
    };

    let reaction_situation = format!(
        "{}\n\nRecent events (older context):\n{}\n\n{}\n{}\n\n{}",
        priority_instruction,
        recent_events_summary,
        new_event_section_label,
        new_event_section_text,
        prompt_situation_text
    );

    Ok(ReactionExecutionInput {
        situation,
        reflection_input,
        reaction_situation,
        description_prefix,
    })
}

async fn build_scene_situation<W: SceneCapability + PersonCapability>(
    worker: &W,
    scene_uuid: &SceneUuid,
    messages: &[Message],
    person_uuid: &PersonUuid,
    include_scene_context: bool,
) -> Result<Situation, Error> {
    let person_name = worker
        .get_persons_name(person_uuid.clone())
        .await
        .map_err(Error::FailedToGetPersonsName)?;

    let participants = worker
        .get_scene_current_participants(scene_uuid)
        .await
        .map_err(|err| Error::FailedToGetSceneParticipants {
            scene_uuid: scene_uuid.clone(),
            details: err,
        })?;

    let participant_names = participants
        .iter()
        .map(|participant| participant.person_name.to_string())
        .collect::<Vec<String>>();

    let (scene_name, scene_description) = if include_scene_context {
        let scene_name = worker
            .get_scene_name(scene_uuid)
            .await
            .map_err(Error::GetPersonReaction)?;
        let scene_description = worker
            .get_scene_description(scene_uuid)
            .await
            .map_err(Error::GetPersonReaction)?;
        (scene_name, scene_description)
    } else {
        (None, None)
    };

    let mut lines = Vec::new();
    for message in messages {
        let sender_label = match &message.sender {
            MessageSender::AiPerson(sender_person_uuid) => {
                if sender_person_uuid.to_uuid() == person_uuid.to_uuid() {
                    continue;
                }
                worker
                    .get_persons_name(sender_person_uuid.clone())
                    .await
                    .map_err(|err| Error::FailedToGetSendersName {
                        person_uuid: sender_person_uuid.clone(),
                        details: err,
                    })?
                    .to_string()
            }
            MessageSender::RealWorldUser => "Chadtech".to_string(),
        };

        lines.push(format!(
            "{}: \"{}\"",
            sender_label,
            normalize_message_content(&message.content)
        ));
    }

    let situation = Situation::new(situation::Input {
        person_name: person_name.to_string(),
        scene_name,
        scene_description,
        particpants: participant_names,
        messages: lines,
    });

    Ok(situation)
}

fn filter_reaction_events(events: Vec<Event>, messages: &[Message]) -> Vec<Event> {
    let mut message_ids = HashSet::new();
    for message in messages {
        message_ids.insert(message.uuid.clone());
    }

    events
        .into_iter()
        .filter(|event| match &event.event_type {
            EventType::Said { message_uuid, .. } => !message_ids.contains(message_uuid),
            _ => true,
        })
        .collect()
}

async fn pending_messages_to_event_lines<W: PersonCapability + Sync>(
    worker: &W,
    pending_messages: &[Message],
    person_uuid: &PersonUuid,
) -> Result<Vec<String>, Error> {
    let mut lines = Vec::new();

    for message in pending_messages {
        let sender_label = match &message.sender {
            MessageSender::AiPerson(sender_person_uuid) => {
                if sender_person_uuid.to_uuid() == person_uuid.to_uuid() {
                    continue;
                }
                worker
                    .get_persons_name(sender_person_uuid.clone())
                    .await
                    .map_err(|err| Error::FailedToGetSendersName {
                        person_uuid: sender_person_uuid.clone(),
                        details: err,
                    })?
                    .to_string()
            }
            MessageSender::RealWorldUser => "Chadtech".to_string(),
        };

        lines.push(format!(
            "In the current scene, {} said: \"{}\" [NEW MESSAGE EVENT]",
            sender_label,
            normalize_message_content(&message.content)
        ));
    }

    Ok(lines)
}

async fn apply_reflection_changes<
    W: StateOfMindCapability + MemoryCapability + LogEventCapability + MotivationCapability,
>(
    worker: &W,
    person_uuid: &PersonUuid,
    reflection_input: &ReflectionInput,
    changes: Vec<ReflectionChange>,
) -> Result<(), Error> {
    for change in changes {
        match change {
            ReflectionChange::StateOfMind { content } => {
                let new_state_of_mind = NewStateOfMind {
                    uuid: StateOfMindUuid::new(),
                    person_name: reflection_input.person_name.clone(),
                    state_of_mind: content.clone(),
                };
                worker
                    .create_state_of_mind(new_state_of_mind)
                    .await
                    .map_err(Error::FailedToCreateReflectionStateOfMind)?;
                let data = serde_json::json!({
                    "person_uuid": person_uuid.to_uuid().to_string(),
                    "change_type": "state_of_mind",
                    "content": content,
                });
                let _ = worker
                    .log_event("reflection_change".to_string(), Some(data))
                    .await;
            }
            ReflectionChange::MemorySummary { summary } => {
                if summary.trim().is_empty() {
                    continue;
                }
                let new_memory = NewMemory {
                    memory_uuid: MemoryUuid::new(),
                    content: summary.clone(),
                    person_uuid: person_uuid.clone(),
                };
                worker
                    .create_memory(new_memory)
                    .await
                    .map_err(Error::FailedToCreateReflectionMemory)?;
                let data = serde_json::json!({
                    "person_uuid": person_uuid.to_uuid().to_string(),
                    "change_type": "memory_summary",
                    "summary": summary,
                });
                let _ = worker
                    .log_event("reflection_change".to_string(), Some(data))
                    .await;
            }
            ReflectionChange::NewMotivation { content, priority } => {
                let priority_i32 = i32::try_from(priority).map_err(|_| {
                    Error::FailedToCreateReflectionMotivation(
                        "Motivation priority must fit in i32".to_string(),
                    )
                })?;
                let new_motivation = NewMotivation {
                    person_uuid: person_uuid.clone(),
                    content: content.clone(),
                    priority: priority_i32,
                };
                let motivation_uuid = worker
                    .create_motivation(new_motivation)
                    .await
                    .map_err(Error::FailedToCreateReflectionMotivation)?;
                let data = serde_json::json!({
                    "person_uuid": person_uuid.to_uuid().to_string(),
                    "change_type": "add_motivation",
                    "content": content,
                    "priority": priority,
                    "motivation_uuid": motivation_uuid.to_uuid().to_string(),
                });
                let _ = worker
                    .log_event("reflection_change".to_string(), Some(data))
                    .await;
            }
            ReflectionChange::DeleteMotivation { motivation_uuid } => {
                worker
                    .delete_motivation(motivation_uuid.clone())
                    .await
                    .map_err(Error::FailedToDeleteReflectionMotivation)?;
                let data = serde_json::json!({
                    "person_uuid": person_uuid.to_uuid().to_string(),
                    "change_type": "remove_motivation",
                    "motivation_uuid": motivation_uuid.to_uuid().to_string(),
                });
                let _ = worker
                    .log_event("reflection_change".to_string(), Some(data))
                    .await;
            }
        }
    }

    Ok(())
}

async fn build_reflection_input<
    W: MessageCapability
        + SceneCapability
        + MemoryCapability
        + PersonCapability
        + EventCapability
        + StateOfMindCapability
        + PersonIdentityCapability,
>(
    worker: &W,
    message_type_args: MessageTypeArgs,
    situation: &Situation,
    person_uuid: &PersonUuid,
) -> Result<ReflectionInput, Error> {
    let persons_name: PersonName = worker
        .get_persons_name(person_uuid.clone())
        .await
        .map_err(Error::FailedToGetPersonsName)?;

    let get_args: capability::event::GetArgs = match &message_type_args {
        MessageTypeArgs::SceneByUuid { scene_uuid } => capability::event::GetArgs::new()
            .with_person_uuid(person_uuid.clone())
            .with_scene_uuid(scene_uuid.clone()),
        MessageTypeArgs::Scene { .. } => {
            capability::event::GetArgs::new().with_person_uuid(person_uuid.clone())
        }
        MessageTypeArgs::Direct { .. } => {
            capability::event::GetArgs::new().with_person_uuid(person_uuid.clone())
        }
    };

    let events = worker
        .get_events(get_args)
        .await
        .map_err(Error::FailedToGetEvents)?
        .iter()
        .map(|event| event.to_text())
        .collect::<Vec<String>>();

    let maybe_state_of_mind: Option<StateOfMind> = worker
        .get_latest_state_of_mind(person_uuid)
        .await
        .map_err(Error::FailedToGetStateOfMind)?;

    let state_of_mind: StateOfMind = match maybe_state_of_mind {
        Some(som) => som,
        None => Err(Error::NoStateOfMindFound {
            person_uuid: person_uuid.clone(),
        })?,
    };

    let memories_prompt = worker
        .create_memory_query_prompt(
            &persons_name,
            message_type_args,
            events,
            &state_of_mind.content,
            &situation.to_string(),
        )
        .await
        .map_err(Error::CouldNotCreateMemoriesPrompt)?;

    let memories: Vec<Memory> = crate::domain::memory::filter_memory_results(
        worker
            .search_memories(person_uuid.clone(), memories_prompt.prompt, 5)
            .await
            .map_err(Error::FailedToSearchMemories)?,
    );

    let maybe_person_identity: Option<String> = worker
        .get_person_identity_summary(person_uuid)
        .await
        .map_err(Error::FailedToGetPersonIdentity)?;

    let person_identity: String = match maybe_person_identity {
        Some(identity) => identity,
        None => Err(Error::NoPersonIdentityFound {
            person_uuid: person_uuid.clone(),
        })?,
    };

    Ok(ReflectionInput {
        person_name: persons_name,
        memories,
        person_identity,
        state_of_mind: state_of_mind.content,
    })
}

async fn get_recent_events_text<W: EventCapability>(
    worker: &W,
    message_type_args: MessageTypeArgs,
    person_uuid: &PersonUuid,
) -> Result<Vec<Event>, Error> {
    let get_args: capability::event::GetArgs = match &message_type_args {
        MessageTypeArgs::SceneByUuid { .. } => {
            capability::event::GetArgs::new().with_person_uuid(person_uuid.clone())
        }
        MessageTypeArgs::Scene { .. } => {
            capability::event::GetArgs::new().with_person_uuid(person_uuid.clone())
        }
        MessageTypeArgs::Direct { .. } => {
            capability::event::GetArgs::new().with_person_uuid(person_uuid.clone())
        }
    };

    worker
        .get_events(get_args)
        .await
        .map_err(Error::FailedToGetEvents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::event::GetArgs;
    use crate::capability::job::JobCapability;
    use crate::capability::log_event::LogEventCapability;
    use crate::capability::logging::LogCapability;
    use crate::capability::memory::{MemoryCapability, MemoryQueryPrompt, MemorySearchResult};
    use crate::capability::motivation::{MotivationCapability, NewMotivation};
    use crate::capability::person::{NewPerson, PersonCapability};
    use crate::capability::person_identity::{NewPersonIdentity, PersonIdentityCapability};
    use crate::capability::person_task::{NewPersonTask, PersonTaskCapability};
    use crate::capability::reaction::ReactionCapability;
    use crate::capability::reaction_history::ReactionHistoryCapability;
    use crate::capability::reflection::{ReflectionCapability, ReflectionChange};
    use crate::capability::scene::{
        CurrentScene, NewScene, NewSceneSnapshot, Scene, SceneCapability, SceneParticipant,
        SceneParticipation,
    };
    use crate::capability::state_of_mind::{NewStateOfMind, StateOfMindCapability};
    use crate::domain::actor_uuid::ActorUuid;
    use crate::domain::event::{Event, EventType};
    use crate::domain::job::process_message::ProcessMessageJob;
    use crate::domain::job::JobKind;
    use crate::domain::logger::Level;
    use crate::domain::memory_uuid::MemoryUuid;
    use crate::domain::message_uuid::MessageUuid;
    use crate::domain::motivation::Motivation;
    use crate::domain::motivation_uuid::MotivationUuid;
    use crate::domain::person_identity_uuid::PersonIdentityUuid;
    use crate::domain::person_task::{
        PersonTask, PersonTaskOutcomeCheck, PersonTaskTerminalOutcome,
    };
    use crate::domain::person_task_uuid::PersonTaskUuid;
    use crate::domain::scene_participant_uuid::SceneParticipantUuid;
    use crate::nice_display::NiceDisplay;
    use crate::person_actions::{PersonAction, PersonReaction, ReflectionDecision};
    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use serde_json::Value;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct MockWorker {
        state: Arc<Mutex<MockState>>,
    }

    struct MockState {
        alice_uuid: PersonUuid,
        bob_uuid: PersonUuid,
        charlie_uuid: PersonUuid,
        scene_uuid: SceneUuid,
        pending_messages: Vec<Message>,
        events: Vec<Event>,
        search_results: Vec<MemorySearchResult>,
        summarize_inputs: Vec<String>,
        preview_situations: Vec<String>,
        reaction_situations: Vec<String>,
        handled_message_ids: Vec<Vec<MessageUuid>>,
        sent_messages: Vec<(MessageSender, SceneUuid, String)>,
        recipient_batches: Vec<Vec<PersonUuid>>,
        jobs: Vec<JobKind>,
        reaction_kinds: Vec<String>,
        memory_descriptions: Vec<String>,
        is_enabled: bool,
        is_hibernating: bool,
        reaction_to_return: PersonReaction,
        latest_state_of_mind: Option<StateOfMind>,
        person_identity_summary: Option<String>,
    }

    impl MockWorker {
        fn new() -> Self {
            let alice_uuid = PersonUuid::new();
            let bob_uuid = PersonUuid::new();
            let charlie_uuid = PersonUuid::new();
            let scene_uuid = SceneUuid::new();
            let pending_message_uuid = MessageUuid::new();

            Self {
                state: Arc::new(Mutex::new(MockState {
                    alice_uuid: alice_uuid.clone(),
                    bob_uuid: bob_uuid.clone(),
                    charlie_uuid: charlie_uuid.clone(),
                    scene_uuid: scene_uuid.clone(),
                    pending_messages: vec![Message {
                        uuid: pending_message_uuid.clone(),
                        sender: MessageSender::AiPerson(bob_uuid.clone()),
                        scene_uuid: scene_uuid.clone(),
                        content: "Hey Alice, are you coming?".to_string(),
                        sent_at: Utc::now(),
                    }],
                    events: vec![
                        Event::new(
                            Utc::now() - Duration::minutes(4),
                            EventType::Entered {
                                person_name: "Charlie".to_string(),
                                scene_name: "Cafe".to_string(),
                            },
                        ),
                        Event::new(
                            Utc::now() - Duration::minutes(1),
                            EventType::Said {
                                scene_name: "Cafe".to_string(),
                                speaker_name: "Bob".to_string(),
                                comment: "Hey Alice, are you coming?".to_string(),
                                message_uuid: pending_message_uuid,
                            },
                        ),
                    ],
                    search_results: vec![MemorySearchResult {
                        content: "Alice trusts Bob when he sounds urgent.".to_string(),
                        distance: 0.2,
                    }],
                    summarize_inputs: vec![],
                    preview_situations: vec![],
                    reaction_situations: vec![],
                    handled_message_ids: vec![],
                    sent_messages: vec![],
                    recipient_batches: vec![],
                    jobs: vec![],
                    reaction_kinds: vec![],
                    memory_descriptions: vec![],
                    is_enabled: true,
                    is_hibernating: false,
                    reaction_to_return: PersonReaction {
                        action: PersonAction::SayInScene {
                            comment: "On my way.".to_string(),
                            destination_scene_name: None,
                        },
                        reflection: ReflectionDecision::NoReflection,
                    },
                    latest_state_of_mind: Some(StateOfMind {
                        content: "Alert and curious".to_string(),
                    }),
                    person_identity_summary: Some("Alice is thoughtful and reliable.".to_string()),
                })),
            }
        }
    }

    impl MessageCapability for MockWorker {
        async fn send_scene_message(
            &self,
            sender: MessageSender,
            scene_uuid: SceneUuid,
            content: String,
        ) -> Result<MessageUuid, String> {
            let mut state = self.state.lock().await;
            state.sent_messages.push((sender, scene_uuid, content));
            Ok(MessageUuid::new())
        }

        async fn add_scene_message_recipients(
            &self,
            _message_uuid: &MessageUuid,
            recipients: Vec<PersonUuid>,
        ) -> Result<(), String> {
            let mut state = self.state.lock().await;
            state.recipient_batches.push(recipients);
            Ok(())
        }

        async fn get_messages_in_scene_page(
            &self,
            _scene_uuid: &SceneUuid,
            _limit: i64,
            _before_sent_at: Option<chrono::DateTime<chrono::Utc>>,
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
            let state = self.state.lock().await;
            Ok(state.pending_messages.clone())
        }

        async fn mark_scene_messages_handled_for_person(
            &self,
            _person_uuid: &PersonUuid,
            message_uuids: Vec<MessageUuid>,
        ) -> Result<(), String> {
            let mut state = self.state.lock().await;
            state.handled_message_ids.push(message_uuids);
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
            let state = self.state.lock().await;
            Ok(Some(CurrentScene {
                scene_uuid: state.scene_uuid.clone(),
            }))
        }

        async fn get_persons_current_scene_uuid(
            &self,
            person_uuid: &PersonUuid,
        ) -> Result<Option<SceneUuid>, String> {
            let state = self.state.lock().await;
            if person_uuid.to_uuid() == state.alice_uuid.to_uuid() {
                Ok(Some(state.scene_uuid.clone()))
            } else {
                Ok(None)
            }
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
            let state = self.state.lock().await;
            Ok(vec![
                SceneParticipant {
                    person_name: PersonName::from_string("Alice".to_string()),
                    actor_uuid: ActorUuid::AiPerson(state.alice_uuid.clone()),
                },
                SceneParticipant {
                    person_name: PersonName::from_string("Bob".to_string()),
                    actor_uuid: ActorUuid::AiPerson(state.bob_uuid.clone()),
                },
                SceneParticipant {
                    person_name: PersonName::from_string("Charlie".to_string()),
                    actor_uuid: ActorUuid::AiPerson(state.charlie_uuid.clone()),
                },
            ])
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

        async fn get_scene_name(&self, scene_uuid: &SceneUuid) -> Result<Option<String>, String> {
            let state = self.state.lock().await;
            if scene_uuid.to_uuid() == state.scene_uuid.to_uuid() {
                Ok(Some("Cafe".to_string()))
            } else {
                Ok(None)
            }
        }

        async fn get_scene_description(
            &self,
            scene_uuid: &SceneUuid,
        ) -> Result<Option<String>, String> {
            let state = self.state.lock().await;
            if scene_uuid.to_uuid() == state.scene_uuid.to_uuid() {
                Ok(Some("A crowded lunch rush fills the room.".to_string()))
            } else {
                Ok(None)
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
            situation: String,
        ) -> Result<ReactionPromptPreview, String> {
            let mut state = self.state.lock().await;
            state.preview_situations.push(situation);
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
            Ok(state.reaction_to_return.clone())
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

    impl MemoryCapability for MockWorker {
        async fn create_memory(&self, _new_memory: NewMemory) -> Result<MemoryUuid, String> {
            Ok(MemoryUuid::new())
        }

        async fn maybe_create_memories_from_description(
            &self,
            _person_uuid: PersonUuid,
            description: String,
        ) -> Result<Vec<MemoryUuid>, String> {
            let mut state = self.state.lock().await;
            state.memory_descriptions.push(description);
            Ok(vec![MemoryUuid::new()])
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
            let state = self.state.lock().await;
            Ok(state.search_results.clone())
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
            if person_uuid.to_uuid() == state.alice_uuid.to_uuid() {
                Ok(PersonName::from_string("Alice".to_string()))
            } else if person_uuid.to_uuid() == state.bob_uuid.to_uuid() {
                Ok(PersonName::from_string("Bob".to_string()))
            } else if person_uuid.to_uuid() == state.charlie_uuid.to_uuid() {
                Ok(PersonName::from_string("Charlie".to_string()))
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
            let state = self.state.lock().await;
            Ok(state.is_hibernating)
        }

        async fn set_person_enabled(
            &self,
            _person_uuid: &PersonUuid,
            _is_enabled: bool,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn is_person_enabled(&self, _person_uuid: &PersonUuid) -> Result<bool, String> {
            let state = self.state.lock().await;
            Ok(state.is_enabled)
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
            Ok(state.latest_state_of_mind.as_ref().map(|som| StateOfMind {
                content: som.content.clone(),
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
            let state = self.state.lock().await;
            Ok(state.person_identity_summary.clone())
        }
    }

    impl ReflectionCapability for MockWorker {
        async fn get_reflection_changes(
            &self,
            _memories: Vec<Memory>,
            _person_uuid: PersonUuid,
            _person_identity: String,
            _state_of_mind: String,
            _situation: String,
        ) -> Result<Vec<ReflectionChange>, String> {
            Ok(vec![])
        }
    }

    impl LogCapability for MockWorker {
        fn log(&self, _level: Level, _message: &str) {}
    }

    impl LogEventCapability for MockWorker {
        async fn log_event(&self, _event_name: String, _data: Option<Value>) -> Result<(), String> {
            Ok(())
        }
    }

    impl MotivationCapability for MockWorker {
        async fn create_motivation(
            &self,
            _new_motivation: NewMotivation,
        ) -> Result<MotivationUuid, String> {
            Ok(MotivationUuid::new())
        }

        async fn get_motivations_for_person(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Vec<Motivation>, String> {
            Ok(vec![])
        }

        async fn delete_motivation(&self, _motivation_uuid: MotivationUuid) -> Result<(), String> {
            Ok(())
        }
    }

    impl ReactionHistoryCapability for MockWorker {
        async fn record_reaction(
            &self,
            _person_uuid: &PersonUuid,
            action_kind: &str,
        ) -> Result<(), String> {
            let mut state = self.state.lock().await;
            state.reaction_kinds.push(action_kind.to_string());
            Ok(())
        }

        async fn has_reacted_since(
            &self,
            _person_uuid: &PersonUuid,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<bool, String> {
            Ok(false)
        }
    }

    impl JobCapability for MockWorker {
        async fn unshift_job(&self, job: JobKind) -> Result<(), String> {
            let mut state = self.state.lock().await;
            state.jobs.push(job);
            Ok(())
        }

        async fn pop_next_job(
            &self,
            _current_active_ms: i64,
        ) -> Result<Option<crate::domain::job::PoppedJob>, String> {
            Ok(None)
        }

        async fn recent_jobs(&self, _limit: i64) -> Result<Vec<crate::domain::job::Job>, String> {
            Ok(vec![])
        }

        async fn get_job_by_uuid(
            &self,
            _job_uuid: &crate::domain::job_uuid::JobUuid,
        ) -> Result<Option<crate::domain::job::Job>, String> {
            Ok(None)
        }

        async fn mark_job_finished(
            &self,
            _job_uuid: &crate::domain::job_uuid::JobUuid,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn mark_job_failed(
            &self,
            _job_uuid: &crate::domain::job_uuid::JobUuid,
            _details: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn reset_job(
            &self,
            _job_uuid: &crate::domain::job_uuid::JobUuid,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn reset_all_failed_jobs(&self) -> Result<(), String> {
            Ok(())
        }

        async fn delete_job(
            &self,
            _job_uuid: &crate::domain::job_uuid::JobUuid,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn preview_scene_reaction_prompts_filters_pending_message_events_and_highlights_new_messages(
    ) {
        let worker = MockWorker::new();
        let state = worker.state.lock().await;
        let alice_uuid = state.alice_uuid.clone();
        let scene_uuid = state.scene_uuid.clone();
        drop(state);

        let _preview = match preview_scene_reaction_prompts(
            &worker,
            &alice_uuid,
            &scene_uuid,
            SceneReactionTrigger::NewMessages,
        )
        .await
        {
            Ok(preview) => preview,
            Err(err) => panic!("preview should succeed: {}", err.message()),
        };

        let state = worker.state.lock().await;
        assert_eq!(state.summarize_inputs.len(), 1);
        assert!(state.summarize_inputs[0].contains("Charlie entered scene Cafe"));
        assert!(!state.summarize_inputs[0].contains("Hey Alice, are you coming?"));

        assert_eq!(state.preview_situations.len(), 1);
        let situation = &state.preview_situations[0];
        assert!(situation.contains("OLDER SUMMARY"));
        assert!(situation.contains("New message events (newest; primary reaction target):"));
        assert!(situation.contains("[NEW MESSAGE EVENT]"));
        assert!(situation.contains("Bob said"));
    }

    #[tokio::test]
    async fn run_scene_reaction_skips_disabled_people_and_marks_pending_messages_handled() {
        let worker = MockWorker::new();
        let mut state = worker.state.lock().await;
        state.is_enabled = false;
        let expected_pending_uuid = state.pending_messages[0].uuid.clone();
        let alice_uuid = state.alice_uuid.clone();
        let scene_uuid = state.scene_uuid.clone();
        drop(state);

        match run_scene_reaction(
            &worker,
            &alice_uuid,
            &scene_uuid,
            SceneReactionTrigger::NewMessages,
            RandomSeed::from_u64(7),
            50,
        )
        .await
        {
            Ok(()) => {}
            Err(err) => panic!(
                "disabled reaction should be skipped cleanly: {}",
                err.message()
            ),
        }

        let state = worker.state.lock().await;
        assert!(state.reaction_situations.is_empty());
        assert_eq!(state.handled_message_ids.len(), 1);
        assert_eq!(state.handled_message_ids[0], vec![expected_pending_uuid]);
        assert!(state.sent_messages.is_empty());
    }

    #[tokio::test]
    async fn run_scene_reaction_say_in_scene_sends_reply_enqueues_follow_up_jobs_and_records_memory(
    ) {
        let worker = MockWorker::new();
        let state = worker.state.lock().await;
        let alice_uuid = state.alice_uuid.clone();
        let bob_uuid = state.bob_uuid.clone();
        let charlie_uuid = state.charlie_uuid.clone();
        let scene_uuid = state.scene_uuid.clone();
        let pending_uuid = state.pending_messages[0].uuid.clone();
        drop(state);

        match run_scene_reaction(
            &worker,
            &alice_uuid,
            &scene_uuid,
            SceneReactionTrigger::NewMessages,
            RandomSeed::from_u64(9),
            120_000,
        )
        .await
        {
            Ok(()) => {}
            Err(err) => panic!("scene reaction should complete: {}", err.message()),
        }

        let state = worker.state.lock().await;
        assert_eq!(state.sent_messages.len(), 1);
        let (_, sent_scene_uuid, sent_content) = &state.sent_messages[0];
        assert_eq!(sent_scene_uuid.to_uuid(), scene_uuid.to_uuid());
        assert_eq!(sent_content, "On my way.");

        assert_eq!(state.recipient_batches.len(), 1);
        let recipients = &state.recipient_batches[0];
        assert_eq!(recipients.len(), 2);
        assert!(recipients
            .iter()
            .any(|uuid| uuid.to_uuid() == bob_uuid.to_uuid()));
        assert!(recipients
            .iter()
            .any(|uuid| uuid.to_uuid() == charlie_uuid.to_uuid()));
        assert!(!recipients
            .iter()
            .any(|uuid| uuid.to_uuid() == alice_uuid.to_uuid()));

        let process_message_jobs = state
            .jobs
            .iter()
            .filter_map(|job| match job {
                JobKind::ProcessMessage(ProcessMessageJob {
                    recipient_person_uuid,
                    ..
                }) => Some(recipient_person_uuid.to_uuid()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(process_message_jobs.len(), 2);
        assert!(process_message_jobs.contains(&bob_uuid.to_uuid()));
        assert!(process_message_jobs.contains(&charlie_uuid.to_uuid()));

        let wait_jobs = state
            .jobs
            .iter()
            .filter_map(|job| match job {
                JobKind::PersonWaiting(wait_job) => Some(wait_job.run_at_active_ms()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(wait_jobs, vec![120_000]);

        assert_eq!(state.reaction_kinds, vec!["say_in_scene".to_string()]);
        assert_eq!(state.handled_message_ids, vec![vec![pending_uuid]]);
        assert_eq!(state.memory_descriptions.len(), 1);
        assert!(state.memory_descriptions[0].contains("Response:\nSpoke in scene: On my way."));
    }
}
