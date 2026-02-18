use crate::domain::goal::Goal;
use crate::domain::goal_uuid::GoalUuid;
use crate::domain::person_uuid::PersonUuid;

pub struct NewGoal {
    pub person_uuid: PersonUuid,
    pub content: String,
    pub priority: i32,
}

pub trait GoalCapability {
    async fn create_goal(&self, new_goal: NewGoal) -> Result<GoalUuid, String>;
    async fn get_goals_for_person(
        &self,
        person_uuid: PersonUuid,
    ) -> Result<Vec<Goal>, String>;
}
