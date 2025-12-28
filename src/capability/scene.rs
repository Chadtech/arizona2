use crate::domain::actor_uuid::ActorUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_participant_uuid::SceneParticipantUuid;
use crate::domain::{person_name::PersonName, scene_uuid::SceneUuid};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub struct NewScene {
    pub name: String,
    pub description: String,
}

pub struct NewSceneSnapshot {
    pub scene_uuid: SceneUuid,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub uuid: SceneUuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SceneParticipant {
    pub person_name: PersonName,
    pub actor_uuid: ActorUuid,
}

pub struct SceneParticipation {
    pub person_uuid: PersonUuid,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

pub struct CurrentScene {
    pub scene_uuid: SceneUuid,
    pub scene_participant_uuid: SceneParticipantUuid,
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
    async fn get_persons_current_scene(
        &self,
        person_name: PersonName,
    ) -> Result<Option<CurrentScene>, String>;
    async fn create_scene_snapshot(
        &self,
        new_scene_snapshot: NewSceneSnapshot,
    ) -> Result<(), String>;
    async fn get_scene_from_name(&self, scene_name: String) -> Result<Option<Scene>, String>;
    async fn get_scene_current_participants(
        &self,
        scene_uuid: &SceneUuid,
    ) -> Result<Vec<SceneParticipant>, String>;
    async fn get_scene_participation_history(
        &self,
        scene_uuid: &SceneUuid,
    ) -> Result<Vec<SceneParticipation>, String>;
    async fn get_scene_name(&self, scene_uuid: &SceneUuid) -> Result<Option<String>, String>;
    async fn get_scene_description(&self, scene_uuid: &SceneUuid)
        -> Result<Option<String>, String>;
}
