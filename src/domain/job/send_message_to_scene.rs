use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageToSceneJob {
    // If its None the message is to Chad rn
    pub sender_person_uuid: Option<PersonUuid>,
    pub scene_uuid: SceneUuid,
    pub content: String,
}
