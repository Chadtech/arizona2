use super::Worker;
use crate::capability::memory::{MemoryCapability, NewMemory};
use crate::domain::memory_uuid::MemoryUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai::embedding::EmbeddingRequest;

impl MemoryCapability for Worker {
    async fn create_memory(&self, new_memory: NewMemory) -> Result<MemoryUuid, String> {
        let embedding = EmbeddingRequest::new(new_memory.content.clone())
            .create(self.open_ai_key.clone(), self.reqwest_client.clone())
            .await
            .map_err(|err| err.message())?;

        let rec = sqlx::query!(
            r#"
                INSERT INTO memory (uuid, person_uuid, content, embedding)
                VALUES ($1::UUID, (SELECT person.uuid FROM person WHERE name = $2::TEXT), $3::TEXT, $4)
                RETURNING uuid;
            "#,
            new_memory.memory_uuid.to_uuid(),
            new_memory.person_name.to_string(),
            new_memory.content,
            &embedding[..] as &[f32]
        )
            .fetch_one(&self.sqlx)
            .await
            .map_err(|err| {
                eprintln!("Error details: {:?}", err);
                format!("Error inserting new memory: {}", err)
            })?;

        Ok(MemoryUuid::from_uuid(rec.uuid))
    }
}
