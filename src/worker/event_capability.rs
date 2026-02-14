use crate::capability::event::{EventCapability, GetArgs};
use crate::capability::person::PersonCapability;
use crate::capability::scene::SceneCapability;
use crate::domain::event::{Event, EventType};
use crate::domain::message::MessageSender;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;

impl EventCapability for Worker {
    async fn get_events(&self, args: GetArgs) -> Result<Vec<Event>, String> {
        let mut events = vec![];

        match (args.person_uuid, args.scene_uuid) {
            // Case 1: Both person and scene specified
            (Some(person_uuid), Some(scene_uuid)) => {
                let scene_name = match self.get_scene_name(&scene_uuid).await {
                    Ok(Some(name)) => name,
                    Ok(None) => format!("Unknown scene {}", scene_uuid.to_uuid()),
                    Err(err) => {
                        return Err(format!("Error fetching scene name: {}", err));
                    }
                };

                // Get direct messages to this person
                let direct_messages = sqlx::query!(
                    r#"
                    SELECT sender_person_uuid, content, sent_at
                    FROM message
                    WHERE receiver_person_uuid = $1
                      AND scene_uuid IS NULL
                    ORDER BY sent_at
                    "#,
                    person_uuid.to_uuid()
                )
                .fetch_all(&self.sqlx)
                .await
                .map_err(|err| format!("Error fetching direct messages: {}", err))?;

                for msg in direct_messages {
                    events.push(Event::new(
                        msg.sent_at,
                        EventType::PersonDirectMessaged {
                            sender: MessageSender::AiPerson(
                                crate::domain::person_uuid::PersonUuid::from_uuid(
                                    msg.sender_person_uuid.ok_or("Missing sender_person_uuid")?,
                                ),
                            ),
                            comment: msg.content,
                        },
                    ));
                }

                // Find when the person last joined this scene
                let participant_record = sqlx::query!(
                    r#"
                    SELECT joined_at
                    FROM scene_participant
                    WHERE scene_uuid = $1
                      AND person_uuid = $2
                      AND left_at IS NULL
                    ORDER BY joined_at DESC
                    LIMIT 1
                    "#,
                    scene_uuid.to_uuid(),
                    person_uuid.to_uuid()
                )
                .fetch_optional(&self.sqlx)
                .await
                .map_err(|err| format!("Error fetching scene participation: {}", err))?;

                if let Some(record) = participant_record {
                    let joined_at = record.joined_at;

                    // Get scene messages since they joined
                    let scene_messages = sqlx::query!(
                        r#"
                        SELECT sender_person_uuid, content, sent_at
                        FROM message
                        WHERE scene_uuid = $1
                          AND sent_at >= $2
                        ORDER BY sent_at
                        "#,
                        scene_uuid.to_uuid(),
                        joined_at
                    )
                    .fetch_all(&self.sqlx)
                    .await
                    .map_err(|err| format!("Error fetching scene messages: {}", err))?;

                    for msg in scene_messages {
                        let speaker_name = match msg.sender_person_uuid {
                            Some(sender_uuid) => {
                                let sender_person_uuid = PersonUuid::from_uuid(sender_uuid);
                                let sender_name =
                                    self.get_persons_name(sender_person_uuid).await.map_err(
                                        |err| format!("Error fetching sender name: {}", err),
                                    )?;
                                sender_name.as_str().to_string()
                            }
                            None => "Chadtech".to_string(),
                        };

                        events.push(Event::new(
                            msg.sent_at,
                            EventType::PersonSaidInScene {
                                scene_name: scene_name.clone(),
                                speaker_name,
                                comment: msg.content,
                            },
                        ));
                    }

                    // Get scene participant joins/leaves since they joined
                    let participant_events = sqlx::query!(
                        r#"
                        SELECT person_uuid, joined_at, left_at
                        FROM scene_participant
                        WHERE scene_uuid = $1
                          AND joined_at >= $2
                        ORDER BY joined_at
                        "#,
                        scene_uuid.to_uuid(),
                        joined_at
                    )
                    .fetch_all(&self.sqlx)
                    .await
                    .map_err(|err| format!("Error fetching participant events: {}", err))?;

                    for participant in participant_events {
                        let participant_person_uuid =
                            PersonUuid::from_uuid(participant.person_uuid);

                        // Add join event
                        events.push(Event::new(
                            participant.joined_at,
                            EventType::PersonJoinedScene {
                                person_uuid: participant_person_uuid.clone(),
                                scene_uuid: scene_uuid.clone(),
                                scene_name: scene_name.clone(),
                            },
                        ));

                        // Add leave event if they left
                        if let Some(left_at) = participant.left_at {
                            events.push(Event::new(
                                left_at,
                                EventType::PersonLeftScene {
                                    person_uuid: participant_person_uuid,
                                    scene_uuid: scene_uuid.clone(),
                                    scene_name: scene_name.clone(),
                                },
                            ));
                        }
                    }
                }
            }

            // Case 2: Only person specified (direct messages only)
            (Some(person_uuid), None) => {
                let direct_messages = sqlx::query!(
                    r#"
                    SELECT sender_person_uuid, content, sent_at
                    FROM message
                    WHERE receiver_person_uuid = $1
                      AND scene_uuid IS NULL
                    ORDER BY sent_at
                    "#,
                    person_uuid.to_uuid()
                )
                .fetch_all(&self.sqlx)
                .await
                .map_err(|err| format!("Error fetching direct messages: {}", err))?;

                for msg in direct_messages {
                    events.push(Event::new(
                        msg.sent_at,
                        EventType::PersonDirectMessaged {
                            sender: MessageSender::AiPerson(
                                crate::domain::person_uuid::PersonUuid::from_uuid(
                                    msg.sender_person_uuid.ok_or("Missing sender_person_uuid")?,
                                ),
                            ),
                            comment: msg.content,
                        },
                    ));
                }
            }

            // Case 3: Only scene specified (all scene events)
            (None, Some(scene_uuid)) => {
                let scene_name = match self.get_scene_name(&scene_uuid).await {
                    Ok(Some(name)) => name,
                    Ok(None) => format!("Unknown scene {}", scene_uuid.to_uuid()),
                    Err(err) => {
                        return Err(format!("Error fetching scene name: {}", err));
                    }
                };

                // Get all scene messages
                let scene_messages = sqlx::query!(
                    r#"
                    SELECT sender_person_uuid, content, sent_at
                    FROM message
                    WHERE scene_uuid = $1
                    ORDER BY sent_at
                    "#,
                    scene_uuid.to_uuid()
                )
                .fetch_all(&self.sqlx)
                .await
                .map_err(|err| format!("Error fetching scene messages: {}", err))?;

                for msg in scene_messages {
                    let speaker_name = match msg.sender_person_uuid {
                        Some(sender_uuid) => {
                            let sender_person_uuid = PersonUuid::from_uuid(sender_uuid);
                            let sender_name = self
                                .get_persons_name(sender_person_uuid)
                                .await
                                .map_err(|err| format!("Error fetching sender name: {}", err))?;
                            sender_name.as_str().to_string()
                        }
                        None => "Chadtech".to_string(),
                    };

                    events.push(Event::new(
                        msg.sent_at,
                        EventType::PersonSaidInScene {
                            scene_name: scene_name.clone(),
                            speaker_name,
                            comment: msg.content,
                        },
                    ));
                }

                // Get all participant joins/leaves
                let participant_events = sqlx::query!(
                    r#"
                    SELECT person_uuid, joined_at, left_at
                    FROM scene_participant
                    WHERE scene_uuid = $1
                    ORDER BY joined_at
                    "#,
                    scene_uuid.to_uuid()
                )
                .fetch_all(&self.sqlx)
                .await
                .map_err(|err| format!("Error fetching participant events: {}", err))?;

                for participant in participant_events {
                    let participant_person_uuid = PersonUuid::from_uuid(participant.person_uuid);

                    // Add join event
                    events.push(Event::new(
                        participant.joined_at,
                        EventType::PersonJoinedScene {
                            person_uuid: participant_person_uuid.clone(),
                            scene_uuid: scene_uuid.clone(),
                            scene_name: scene_name.clone(),
                        },
                    ));

                    // Add leave event if they left
                    if let Some(left_at) = participant.left_at {
                        events.push(Event::new(
                            left_at,
                            EventType::PersonLeftScene {
                                person_uuid: participant_person_uuid,
                                scene_uuid: scene_uuid.clone(),
                                scene_name: scene_name.clone(),
                            },
                        ));
                    }
                }
            }

            // Case 4: Neither specified - return empty
            (None, None) => {}
        }

        // Sort all events by timestamp
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(events)
    }
}
