use crate::capability::job::JobCapability;
use crate::capability::message::MessageCapability;
use crate::domain::actor_uuid::ActorUuid;
use crate::domain::job::process_message::ProcessMessageJob;
use crate::domain::job::JobKind;
use crate::domain::message_uuid::MessageUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::nice_display::NiceDisplay;
use crate::{capability::scene::SceneCapability, domain::message::MessageSender};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageToSceneJob {
    pub sender: MessageSender,
    pub scene_uuid: SceneUuid,
    pub content: String,
    pub random_seed: RandomSeed,
}

pub enum Error {
    FailedToGetSceneParticipants {
        scene_uuid: SceneUuid,
        details: String,
    },
    FailedToSendMessage {
        participant: ActorUuid,
        details: String,
    },
    FailedToUnshiftJob {
        message_uuid: MessageUuid,
        details: String,
    },
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::FailedToGetSceneParticipants {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get participants for scene {}: {}",
                    scene_uuid.to_uuid().to_string(),
                    details
                )
            }
            Error::FailedToSendMessage {
                participant,
                details,
            } => {
                format!(
                    "Failed to send message to participant {}: {}",
                    participant.to_label(),
                    details
                )
            }
            Error::FailedToUnshiftJob {
                message_uuid,
                details,
            } => {
                format!(
                    "Failed to unshift process message job for message {}: {}",
                    message_uuid.to_uuid().to_string(),
                    details
                )
            }
        }
    }
}

impl SendMessageToSceneJob {
    pub async fn run<W: SceneCapability + MessageCapability + JobCapability>(
        self,
        worker: &W,
    ) -> Result<(), Error> {
        send_scene_message_and_enqueue_recipients(
            worker,
            self.sender,
            self.scene_uuid,
            self.content,
            self.random_seed,
        )
        .await?;

        Ok(())
    }
}

pub async fn send_scene_message_and_enqueue_recipients<
    W: SceneCapability + MessageCapability + JobCapability,
>(
    worker: &W,
    sender: MessageSender,
    scene_uuid: SceneUuid,
    content: String,
    random_seed: RandomSeed,
) -> Result<MessageUuid, Error> {
    let mut participants = worker
        .get_scene_current_participants(&scene_uuid)
        .await
        .map_err(|err| Error::FailedToGetSceneParticipants {
            scene_uuid: scene_uuid.clone(),
            details: err,
        })?;

    // Shuffle participants using the random seed for non-deterministic ordering
    let mut rng = rand::rngs::SmallRng::seed_from_u64(random_seed.value());
    participants.shuffle(&mut rng);

    let message_uuid = worker
        .send_scene_message(sender.clone(), scene_uuid.clone(), content)
        .await
        .map_err(|err| Error::FailedToSendMessage {
            participant: ActorUuid::RealWorldUser,
            details: err,
        })?;

    let mut recipient_uuids = Vec::new();
    let mut recipient_participants = Vec::new();

    for participant in participants {
        let is_sender = match (&sender, &participant.actor_uuid) {
            (MessageSender::AiPerson(sender_uuid), ActorUuid::AiPerson(participant_uuid)) => {
                sender_uuid.to_uuid() == participant_uuid.to_uuid()
            }
            _ => false,
        };

        if is_sender {
            continue;
        }

        if let ActorUuid::AiPerson(person_uuid) = participant.actor_uuid.clone() {
            recipient_uuids.push(person_uuid);
        }

        recipient_participants.push(participant);
    }

    worker
        .add_scene_message_recipients(&message_uuid, recipient_uuids)
        .await
        .map_err(|err| Error::FailedToSendMessage {
            participant: ActorUuid::RealWorldUser,
            details: err,
        })?;

    for participant in recipient_participants {
        let message_uuid = message_uuid.clone();
        let process_message_job = ProcessMessageJob {
            message_uuid: message_uuid.clone(),
            recipient_person_uuid: match participant.actor_uuid {
                ActorUuid::AiPerson(person_uuid) => Some(person_uuid),
                ActorUuid::RealWorldUser => None,
            },
        };

        worker
            .unshift_job(JobKind::ProcessMessage(process_message_job))
            .await
            .map_err(|err| Error::FailedToUnshiftJob {
                message_uuid,
                details: err,
            })?;
    }

    Ok(message_uuid)
}
