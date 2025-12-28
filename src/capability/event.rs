use crate::capability::scene::SceneCapability;
use crate::domain::{event::Event, person_uuid::PersonUuid, scene_uuid::SceneUuid};

pub struct GetArgs {
    pub person_uuid: Option<PersonUuid>,
    pub scene_uuid: Option<SceneUuid>,
}

impl GetArgs {
    pub fn new() -> Self {
        Self {
            person_uuid: None,
            scene_uuid: None,
        }
    }

    pub fn with_person_uuid(mut self, person_uuid: PersonUuid) -> Self {
        self.person_uuid = Some(person_uuid);
        self
    }

    pub fn with_scene_uuid(mut self, scene_uuid: SceneUuid) -> Self {
        self.scene_uuid = Some(scene_uuid);
        self
    }
}

pub trait EventCapability {
    async fn get_events(&self, args: GetArgs) -> Result<Vec<Event>, String>;
}
