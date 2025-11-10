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
    pub fn value(&self) -> u64 {
        self.0
    }
}

pub enum Error {
    FailedToGetSceneParticipants {
        scene_uuid: SceneUuid,
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
        }
    }
}

impl SendMessageToSceneJob {
    pub async fn run<W: SceneCapability>(self, worker: W) -> Result<(), Error> {
        let mut participants = worker
            .get_scene_participants(&self.scene_uuid)
            .await
            .map_err(|err| Error::FailedToGetSceneParticipants {
                scene_uuid: self.scene_uuid.clone(),
                details: err,
            })?;

        // Shuffle participants using the random seed for non-deterministic ordering
        let mut rng = rand::rngs::SmallRng::seed_from_u64(self.random_seed.value());
        participants.shuffle(&mut rng);

        // TODO: Create message records and ProcessMessage jobs for each participant

        Ok(())
    }
}
