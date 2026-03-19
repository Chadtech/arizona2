use crate::capability::event::{EventCapability, GetArgs};
use crate::capability::person::PersonCapability;
use crate::capability::scene::SceneCapability;
use crate::domain::event::{Event, EventType};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;
use crate::temporary_event_cutoff::event_history_cutoff;
use crate::worker::Worker;

impl EventCapability for Worker {
    async fn get_events(&self, args: GetArgs) -> Result<Vec<Event>, String> {
        let mut events = vec![];

        match (args.person_uuid, args.scene_uuid) {
            // Case 1: Both person and scene specified
            (Some(person_uuid), Some(scene_uuid)) => {
                let scene_name = get_scene_name(self, &scene_uuid).await?;

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
                        SELECT uuid, sender_person_uuid, content, sent_at
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
                            EventType::Said {
                                scene_name: scene_name.clone(),
                                speaker_name,
                                comment: msg.content,
                                message_uuid: MessageUuid::from_uuid(msg.uuid),
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
                        let participant_name = self
                            .get_persons_name(participant_person_uuid.clone())
                            .await
                            .map_err(|err| format!("Error fetching participant name: {}", err))?
                            .as_str()
                            .to_string();

                        // Add join event
                        events.push(Event::new(
                            participant.joined_at,
                            EventType::Entered {
                                person_name: participant_name.clone(),
                                scene_name: scene_name.clone(),
                            },
                        ));

                        // Add leave event if they left
                        if let Some(left_at) = participant.left_at {
                            events.push(Event::new(
                                left_at,
                                EventType::Left {
                                    person_name: participant_name.clone(),
                                    scene_name: scene_name.clone(),
                                },
                            ));
                        }
                    }
                }
            }

            // Case 2: Only person specified (person-scoped scene context)
            (Some(person_uuid), None) => {
                // Pull current and immediately previous scene participation windows so
                // context survives a fresh scene transition.
                let participations = sqlx::query!(
                    r#"
                    SELECT scene_uuid, joined_at, left_at
                    FROM scene_participant
                    WHERE person_uuid = $1::UUID
                    ORDER BY joined_at DESC
                    LIMIT 2
                    "#,
                    person_uuid.to_uuid()
                )
                .fetch_all(&self.sqlx)
                .await
                .map_err(|err| format!("Error fetching person participations: {}", err))?;

                for participation in participations {
                    let scene_uuid = match participation.scene_uuid {
                        Some(uuid) => SceneUuid::from_uuid(uuid),
                        None => continue,
                    };
                    let scene_name = get_scene_name(self, &scene_uuid).await?;
                    let joined_at = participation.joined_at;
                    let left_at = participation.left_at;

                    let scene_messages = sqlx::query!(
                        r#"
                        SELECT uuid, sender_person_uuid, content, sent_at
                        FROM message
                        WHERE scene_uuid = $1::UUID
                          AND sent_at >= $2
                          AND ($3::timestamptz IS NULL OR sent_at <= $3::timestamptz)
                        ORDER BY sent_at
                        "#,
                        scene_uuid.to_uuid(),
                        joined_at,
                        left_at
                    )
                    .fetch_all(&self.sqlx)
                    .await
                    .map_err(|err| format!("Error fetching person-scoped scene messages: {}", err))?;

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
                            EventType::Said {
                                scene_name: scene_name.clone(),
                                speaker_name,
                                comment: msg.content,
                                message_uuid: MessageUuid::from_uuid(msg.uuid),
                            },
                        ));
                    }

                    let participant_events = sqlx::query!(
                        r#"
                        SELECT person_uuid, joined_at, left_at
                        FROM scene_participant
                        WHERE scene_uuid = $1::UUID
                          AND joined_at >= $2
                          AND ($3::timestamptz IS NULL OR joined_at <= $3::timestamptz)
                        ORDER BY joined_at
                        "#,
                        scene_uuid.to_uuid(),
                        joined_at,
                        left_at
                    )
                    .fetch_all(&self.sqlx)
                    .await
                    .map_err(|err| {
                        format!("Error fetching person-scoped participant events: {}", err)
                    })?;

                    for participant in participant_events {
                        let participant_person_uuid =
                            PersonUuid::from_uuid(participant.person_uuid);
                        let participant_name = self
                            .get_persons_name(participant_person_uuid.clone())
                            .await
                            .map_err(|err| format!("Error fetching participant name: {}", err))?
                            .as_str()
                            .to_string();

                        events.push(Event::new(
                            participant.joined_at,
                            EventType::Entered {
                                person_name: participant_name.clone(),
                                scene_name: scene_name.clone(),
                            },
                        ));

                        if let Some(event_left_at) = participant.left_at {
                            match left_at {
                                Some(window_end) if event_left_at > window_end => {}
                                _ => events.push(Event::new(
                                    event_left_at,
                                    EventType::Left {
                                        person_name: participant_name.clone(),
                                        scene_name: scene_name.clone(),
                                    },
                                )),
                            }
                        }
                    }
                }
            }

            // Case 3: Only scene specified (all scene events)
            (None, Some(scene_uuid)) => {
                let scene_name = get_scene_name(self, &scene_uuid).await?;

                // Get all scene messages
                let scene_messages = sqlx::query!(
                    r#"
                    SELECT uuid, sender_person_uuid, content, sent_at
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
                        EventType::Said {
                            scene_name: scene_name.clone(),
                            speaker_name,
                            comment: msg.content,
                            message_uuid: MessageUuid::from_uuid(msg.uuid),
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
                    let participant_name = self
                        .get_persons_name(participant_person_uuid.clone())
                        .await
                        .map_err(|err| format!("Error fetching participant name: {}", err))?
                        .as_str()
                        .to_string();

                    // Add join event
                    events.push(Event::new(
                        participant.joined_at,
                        EventType::Entered {
                            person_name: participant_name.clone(),
                            scene_name: scene_name.clone(),
                        },
                    ));

                    // Add leave event if they left
                    if let Some(left_at) = participant.left_at {
                        events.push(Event::new(
                            left_at,
                            EventType::Left {
                                person_name: participant_name.clone(),
                                scene_name: scene_name.clone(),
                            },
                        ));
                    }
                }
            }

            // Case 4: Neither specified - return empty
            (None, None) => {}
        }

        let cutoff = event_history_cutoff();
        events.retain(|event| event.timestamp >= cutoff);

        // Sort all events by timestamp
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(events)
    }
}

async fn get_scene_name(worker: &Worker, scene_uuid: &SceneUuid) -> Result<String, String> {
    match worker.get_scene_name(scene_uuid).await {
        Ok(Some(name)) => Ok(name),
        Ok(None) => Ok(format!("Unknown scene {}", scene_uuid.to_uuid())),
        Err(err) => Err(format!("Error fetching scene name: {}", err)),
    }
}
