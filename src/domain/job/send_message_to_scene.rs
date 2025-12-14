use crate::capability::job::JobCapability;
use crate::capability::message::{MessageCapability, NewMessage};
use crate::domain::actor_uuid::ActorUuid;
use crate::domain::job::process_message::ProcessMessageJob;
use crate::domain::job::JobKind;
use crate::domain::message::MessageRecipient;
use crate::domain::message_uuid::MessageUuid;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomSeed(u64);

impl RandomSeed {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
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
        let mut participants = worker
            .get_scene_current_participants(&self.scene_uuid)
            .await
            .map_err(|err| Error::FailedToGetSceneParticipants {
                scene_uuid: self.scene_uuid.clone(),
                details: err,
            })?;

        // Shuffle participants using the random seed for non-deterministic ordering
        let mut rng = rand::rngs::SmallRng::seed_from_u64(self.random_seed.value());
        participants.shuffle(&mut rng);

        for participant in participants {
            let new_message = NewMessage {
                sender: self.sender.clone(),
                recipient: MessageRecipient::from(&participant.actor_uuid),
                content: self.content.clone(),
                scene_uuid: Some(self.scene_uuid.clone()),
            };

            let message_uuid = worker.send_message(new_message).await.map_err(|err| {
                Error::FailedToSendMessage {
                    participant: participant.actor_uuid,
                    details: err,
                }
            })?;

            let process_message_job = ProcessMessageJob {
                message_uuid: message_uuid.clone(),
            };

            worker
                .unshift_job(JobKind::ProcessMessage(process_message_job))
                .await
                .map_err(|err| Error::FailedToUnshiftJob {
                    message_uuid,
                    details: err,
                })?;
        }

        Ok(())
    }
}
