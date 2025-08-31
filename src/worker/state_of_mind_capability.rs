use crate::capability::state_of_mind::{NewStateOfMind, StateOfMindCapability};
use crate::domain::state_of_mind_uuid::StateOfMindUuid;
use crate::worker::Worker;
use async_trait::async_trait;

#[async_trait]
impl StateOfMindCapability for Worker {
    async fn create_state_of_mind(
        &self,
        new_state_of_mind: NewStateOfMind,
    ) -> Result<StateOfMindUuid, String> {
        let ret = sqlx::query!(
            r#"
                INSERT INTO state_of_mind (uuid, person_uuid, content)
                SELECT $1::UUID, person.uuid, $2::TEXT
                FROM person
                WHERE name = $3::TEXT
                RETURNING uuid;
            "#,
            new_state_of_mind.uuid.to_uuid(),
            new_state_of_mind.state_of_mind,
            new_state_of_mind.person_name.to_string()
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new state of mind: {}", err))?;

        Ok(StateOfMindUuid::from_uuid(ret.uuid))
    }
}
