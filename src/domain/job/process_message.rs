use crate::domain::message::MessageRecipient;
use crate::{
    capability::{message::MessageCapability, scene::SceneCapability},
    domain::{message_uuid::MessageUuid, scene_uuid::SceneUuid},
    nice_display::NiceDisplay,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMessageJob {
    pub message_uuid: MessageUuid,
}

pub enum Error {
    FailedToGetMessage(String),
    MessageNotFound,
    FailedToGetSceneParticipants {
        scene_uuid: SceneUuid,
        details: String,
    },
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::FailedToGetMessage(details) => {
                format!("Failed to get message: {}", details)
            }
            Error::MessageNotFound => "Message not found".to_string(),
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
        }
    }
}

impl ProcessMessageJob {
    pub async fn run<W: MessageCapability + SceneCapability>(
        self,
        worker: &W,
    ) -> Result<(), Error> {
        let maybe_message = worker
            .get_message_by_uuid(&self.message_uuid)
            .await
            .map_err(Error::FailedToGetMessage)?;

        let message = match maybe_message {
            Some(msg) => msg,
            None => return Err(Error::MessageNotFound),
        };

        match message.recipient {
            MessageRecipient::Person(_person_uuid) => {
                todo!("Process message to AI person");
            }
            MessageRecipient::RealWorldUser => {
                // Process message to real-world user
            }
        }

        // Placeholder for additional processing logic on the message

        Ok(())
    }
}
