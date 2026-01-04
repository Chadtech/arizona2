use super::Worker;
use crate::capability::memory::{
    MemoryCapability, MemoryQueryPrompt, MemorySearchResult, MessageTypeArgs, NewMemory,
};
use crate::capability::scene::SceneCapability;
use crate::domain::memory_uuid::MemoryUuid;
use crate::domain::message::MessageSender;
use crate::domain::person_name::PersonName;
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
        person_recalling: PersonName,
        message_type_args: MessageTypeArgs,
        recent_events: Vec<String>,
        state_of_mind: &String,
        situation: &String,
    ) -> Result<MemoryQueryPrompt, String> {
        let mut prompt = String::new();

        fn add_scene_to_prompt(
            prompt: &mut String,
            person_recalling: &PersonName,
            scene_name: &String,
            scene_description: &String,
        ) {
            prompt.push_str(
                format!(
                    "{} is in a scene called '{}'. The scene is described as: {}.\n\n",
                    person_recalling.to_string(),
                    scene_name,
                    scene_description
                )
                .as_str(),
            );
        }

        match &message_type_args {
            MessageTypeArgs::Scene {
                scene_name,
                scene_description,
                ..
            } => {
                add_scene_to_prompt(
                    &mut prompt,
                    &person_recalling,
                    scene_name,
                    scene_description,
                );
            }
            MessageTypeArgs::SceneByUuid { scene_uuid } => {
                let maybe_scene_name = self
                    .get_scene_name(scene_uuid)
                    .await
                    .map_err(|err| format!("Error fetching scene name: {}", err))?;

                let scene_name = match maybe_scene_name {
                    Some(name) => name,
                    None => Err(format!("Scene with UUID {} not found", scene_uuid.clone()))?,
                };

                let maybe_scene_description = self
                    .get_scene_description(scene_uuid)
                    .await
                    .map_err(|err| format!("Error fetching scene description: {}", err))?;

                let scene_description = match maybe_scene_description {
                    Some(description) => description,
                    None => Err(format!(
                        "Scene description for UUID {} not found",
                        scene_uuid.to_string()
                    ))?,
                };

                add_scene_to_prompt(
                    &mut prompt,
                    &person_recalling,
                    &scene_name,
                    &scene_description,
                );
            }
            MessageTypeArgs::Direct { from } => {
                let sender_name = {
                    match from {
                        MessageSender::AiPerson(person_uuid) => {
                            let rec = sqlx::query!(
                                r#"
                                SELECT name
                                FROM person
                                WHERE uuid = $1::UUID;
                            "#,
                                person_uuid.to_uuid()
                            )
                            .fetch_one(&self.sqlx)
                            .await
                            .map_err(|err| format!("Error fetching person's name: {}", err))?;

                            rec.name
                        }
                        MessageSender::RealWorldUser => "Chadtech".to_string(),
                    }
                };

                prompt.push_str(
                    format!(
                        "{} has received a direct message from {}.\n\n",
                        person_recalling.to_string(),
                        sender_name
                    )
                    .as_str(),
                );
            }
        }

        prompt.push_str("\n\n");

        prompt.push_str(
            format!(
                "{}'s state of mind is {}",
                person_recalling.to_string(),
                state_of_mind
            )
            .as_str(),
        );

        fn add_people_to_prompt(prompt: &mut String, people: &Vec<String>) {
            prompt.push_str("\n\nOther people present:\n");

            for person in people {
                prompt.push_str(format!("- {}\n", person).as_str());
            }
        }

        prompt.push_str(
            format!(
                "\n\nThe current situation is described as:\n{}\n",
                situation
            )
            .as_str(),
        );

        match message_type_args {
            MessageTypeArgs::Scene { people, .. } => {
                add_people_to_prompt(&mut prompt, &people);
            }
            MessageTypeArgs::Direct { .. } => {
                prompt
                    .push_str("\n\nThis is a direct message, so no one else is around to see it\n");
            }
            MessageTypeArgs::SceneByUuid { scene_uuid } => {
                let scene_participants = self
                    .get_scene_current_participants(&scene_uuid)
                    .await
                    .map_err(|err| format!("Error fetching scene participants: {}", err))?;

                let participant_names: Vec<String> = scene_participants
                    .iter()
                    .map(|participant| participant.person_name.to_string().to_owned())
                    .collect();

                add_people_to_prompt(&mut prompt, &participant_names);
            }
        }

        prompt.push_str("\n\nRecent events include:\n");
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
