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
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::domain::situation;
use crate::domain::situation::Situation;
use crate::domain::state_of_mind::StateOfMind;
use crate::domain::state_of_mind_uuid::StateOfMindUuid;
use crate::person_actions::ReflectionDecision;
use crate::text_utils::normalize_message_content;
use crate::{capability::message::MessageCapability, capability::scene::SceneCapability};
use crate::nice_display::NiceDisplay;
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
        SceneReactionTrigger::SceneDescriptionGaze => {
            "Scene gaze event (primary reaction target):"
        }
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
            let scene_name = match worker
                .get_scene_name(scene_uuid)
                .await
                .map_err(|err| Error::FailedToGetSceneName {
                    scene_uuid: scene_uuid.clone(),
                    details: err,
                })?
            {
                Some(scene_name) => scene_name,
                None => return Err(Error::SceneNameNotFound {
                    scene_uuid: scene_uuid.clone(),
                }),
            };
            let scene_description = match worker
                .get_scene_description(scene_uuid)
                .await
                .map_err(|err| Error::FailedToGetSceneDescription {
                    scene_uuid: scene_uuid.clone(),
                    details: err,
                })?
            {
                Some(scene_description) => scene_description,
                None => return Err(Error::SceneDescriptionNotFound {
                    scene_uuid: scene_uuid.clone(),
                }),
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
