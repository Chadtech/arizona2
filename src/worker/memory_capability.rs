use super::Worker;
use crate::capability::memory::{
    MemoryCapability, MemoryQueryPrompt, MemorySearchResult, NewMemory,
};
use crate::domain::memory_uuid::MemoryUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::Completion;
use crate::open_ai::embedding::EmbeddingRequest;
use crate::open_ai::role::Role;

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

    async fn create_memory_query_prompt(
        &self,
        person_recalling: String,
        people: Vec<String>,
        scene_name: String,
        scene_description: String,
        recent_events: Vec<String>,
        state_of_mind: String,
    ) -> Result<MemoryQueryPrompt, String> {
        let mut prompt = format!(
            "{} is in a scene called '{}'. The scene is described as: {}.\n\n",
            person_recalling, scene_name, scene_description
        );

        prompt.push_str("\n\n");

        prompt.push_str(
            format!("{}'s state of mind is {}", person_recalling, state_of_mind).as_str(),
        );

        prompt.push_str("\n\nOther people present:\n");

        for person in people {
            prompt.push_str(format!("- {}\n", person).as_str());
        }

        prompt.push_str("\n\nRecent events in the scene:\n");
        for event in recent_events {
            prompt.push_str(format!("- {}\n", event).as_str());
        }

        let mut completion = Completion::new(open_ai::model::Model::Gpt4p1);

        completion.add_message(Role::System, "You are a memory retrieval assistant. Given context, generate a prompt that can be used in a vector database of memories to retrieve relevant memories for that person in that situation.");
        completion.add_message(Role::User, prompt.as_str());

        let response = completion
            .send_request(&self.open_ai_key, self.reqwest_client.clone())
            .await
            .map_err(|err| {
                format!(
                    "Error generating memory query prompt:\n{}",
                    err.to_nice_error().to_string()
                )
            })?;

        let memory_prompt = response.as_message().map_err(|err| {
            format!(
                "Error extracting memory prompt from completion response: {}",
                err.to_nice_error().to_string()
            )
        })?;

        Ok(MemoryQueryPrompt {
            prompt: memory_prompt,
        })
    }

    async fn search_memories(
        &self,
        query: String,
        limit: i64,
    ) -> Result<Vec<MemorySearchResult>, String> {
        // Generate embedding for the query
        let query_embedding = EmbeddingRequest::new(query)
            .create(self.open_ai_key.clone(), self.reqwest_client.clone())
            .await
            .map_err(|err| err.message())?;

        // Search for similar memories using vector similarity
        let records = sqlx::query!(
            r#"
                SELECT
                    uuid,
                    content,
                    (embedding <=> $1::vector)::FLOAT AS distance
                FROM memory
                ORDER BY embedding <=> $1::vector
                LIMIT $2
            "#,
            &query_embedding[..] as &[f32],
            limit
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error searching memories: {}", err))?;

        Ok(records
            .into_iter()
            .map(|rec| MemorySearchResult {
                memory_uuid: MemoryUuid::from_uuid(rec.uuid),
                content: rec.content,
                distance: rec.distance.unwrap_or(f64::MAX),
            })
            .collect())
    }
}
