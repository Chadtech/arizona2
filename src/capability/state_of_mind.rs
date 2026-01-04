use crate::domain::state_of_mind::StateOfMind;
use crate::domain::{
    person_name::PersonName,
    person_uuid::PersonUuid,
    state_of_mind_uuid::StateOfMindUuid,
};
use async_trait::async_trait;

pub struct NewStateOfMind {
    pub uuid: StateOfMindUuid,
    pub person_name: PersonName,
    pub state_of_mind: String,
}

#[async_trait]
pub trait StateOfMindCapability {
    async fn create_state_of_mind(
        &self,
        new_state_of_mind: NewStateOfMind,
    ) -> Result<StateOfMindUuid, String>;
    async fn get_latest_state_of_mind(
        &self,
        person_uuid: &PersonUuid,
    ) -> Result<Option<StateOfMind>, String>;
}
