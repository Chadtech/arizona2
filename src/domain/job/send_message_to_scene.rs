use crate::domain::message::MessageSender;
use crate::domain::scene_uuid::SceneUuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageToSceneJob {
    pub sender: MessageSender,
    pub scene_uuid: SceneUuid,
    pub content: String,
}
