use crate::domain::motivation::Motivation;
use crate::domain::motivation_uuid::MotivationUuid;
use crate::domain::person_uuid::PersonUuid;

pub struct NewMotivation {
    pub person_uuid: PersonUuid,
    pub content: String,
    pub priority: i32,
}

pub trait MotivationCapability {
    async fn create_motivation(
        &self,
        new_motivation: NewMotivation,
    ) -> Result<MotivationUuid, String>;
    async fn get_motivations_for_person(
        &self,
        person_uuid: &PersonUuid,
    ) -> Result<Vec<Motivation>, String>;
    async fn delete_motivation(&self, motivation_uuid: MotivationUuid) -> Result<(), String>;
}
