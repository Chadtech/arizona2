use crate::domain::scene_participant_uuid::SceneParticipantUuid;
use crate::domain::{person_name::PersonName, scene_uuid::SceneUuid};
use async_trait::async_trait;

pub struct NewScene {
    pub name: String,
    pub description: String,
}

pub struct NewSceneSnapshot {
    pub scene_uuid: SceneUuid,
    pub description: String,
}

#[async_trait]
pub trait SceneCapability {
    async fn create_scene(&self, new_scene: NewScene) -> Result<SceneUuid, String>;
    async fn add_person_to_scene(
        &self,
        scene_uuid: SceneUuid,
        person_name: PersonName,
    ) -> Result<SceneParticipantUuid, String>;
    async fn remove_person_from_scene(
        &self,
        scene_uuid: SceneUuid,
        person_name: PersonName,
    ) -> Result<SceneParticipantUuid, String>;
    async fn create_scene_snapshot(
        &self,
        new_scene_snapshot: NewSceneSnapshot,
    ) -> Result<(), String>;
    async fn get_scene_description(&self, scene_uuid: SceneUuid) -> Result<String, String>;
}
