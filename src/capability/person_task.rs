use crate::domain::person_task::PersonTask;
use crate::domain::person_task::PersonTaskTerminalOutcome;
use crate::domain::person_task_uuid::PersonTaskUuid;
use crate::domain::person_uuid::PersonUuid;

pub struct NewPersonTask {
    pub person_uuid: PersonUuid,
    pub content: String,
    pub state: Option<String>,
    pub success_condition: Option<String>,
    pub abandon_condition: Option<String>,
    pub failure_condition: Option<String>,
    pub priority: i32,
}

pub trait PersonTaskCapability {
    async fn get_persons_current_active_task(
        &self,
        person_uuid: &PersonUuid,
    ) -> Result<Option<PersonTask>, String>;

    async fn set_persons_current_active_task(
        &self,
        new_person_task: NewPersonTask,
    ) -> Result<PersonTaskUuid, String>;

    async fn transition_person_task(
        &self,
        person_uuid: &PersonUuid,
        person_task_uuid: &PersonTaskUuid,
        outcome: PersonTaskTerminalOutcome,
    ) -> Result<(), String>;
}
