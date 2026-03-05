use crate::capability::scene::{
    CurrentScene, NewScene, NewSceneSnapshot, Scene, SceneCapability, SceneParticipant,
    SceneParticipation,
};
use crate::domain::actor_uuid::ActorUuid;
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_participant_uuid::SceneParticipantUuid;
use crate::domain::scene_uuid::SceneUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::Completion;
use crate::open_ai::role::Role;
use crate::worker::Worker;
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
impl SceneCapability for Worker {
    async fn create_scene(&self, new_scene: NewScene) -> Result<SceneUuid, String> {
        let ret = sqlx::query!(
            r#"
				INSERT INTO scene (uuid, name)
				VALUES ($1::UUID, $2::TEXT)
				RETURNING uuid;
			"#,
            SceneUuid::new().to_uuid(),
            new_scene.name,
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new scene: {}", err))?;

        let scene_uuid = SceneUuid::from_uuid(ret.uuid);

        let new_snapshot = NewSceneSnapshot {
            scene_uuid,
            description: new_scene.description,
        };

        self.create_scene_snapshot(new_snapshot).await?;

        Ok(SceneUuid::from_uuid(ret.uuid))
    }

    async fn delete_scene(&self, scene_uuid: &SceneUuid) -> Result<(), String> {
        sqlx::query!(
            r#"
                UPDATE scene
                SET ended_at = NOW()
                WHERE uuid = $1::UUID
                  AND ended_at IS NULL;
            "#,
            scene_uuid.to_uuid(),
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error marking scene as ended: {}", err))?;

        sqlx::query!(
            r#"
                UPDATE scene_participant
                SET left_at = NOW()
                WHERE scene_uuid = $1::UUID
                  AND left_at IS NULL;
            "#,
            scene_uuid.to_uuid(),
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error removing active participants from scene: {}", err))?;

        Ok(())
    }

    async fn get_scenes(&self) -> Result<Vec<Scene>, String> {
        let rows = sqlx::query(
            r#"
                SELECT
                    scene.uuid,
                    scene.name,
                    latest_snapshot.description
                FROM scene
                LEFT JOIN LATERAL (
                    SELECT description
                    FROM scene_snapshot
                    WHERE scene_snapshot.scene_uuid = scene.uuid
                    ORDER BY created_at DESC
                    LIMIT 1
                ) AS latest_snapshot ON true
                WHERE scene.ended_at IS NULL
                ORDER BY scene.name ASC;
            "#,
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching scenes: {}", err))?;

        let mut scenes = Vec::with_capacity(rows.len());

        for row in rows {
            let uuid = row
                .try_get::<uuid::Uuid, _>("uuid")
                .map_err(|err| format!("Error reading scene uuid: {}", err))?;
            let name = row
                .try_get::<String, _>("name")
                .map_err(|err| format!("Error reading scene name: {}", err))?;
            let description = row
                .try_get::<Option<String>, _>("description")
                .map_err(|err| format!("Error reading scene description: {}", err))?;

            scenes.push(Scene {
                uuid: SceneUuid::from_uuid(uuid),
                name,
                description,
            });
        }

        Ok(scenes)
    }

    async fn add_person_to_scene(
        &self,
        scene_uuid: SceneUuid,
        person_name: PersonName,
    ) -> Result<SceneParticipantUuid, String> {
        let persons_current_scene = self.get_persons_current_scene(person_name.clone()).await?;

        if let Some(current_scene) = persons_current_scene {
            self.remove_person_from_scene(current_scene.scene_uuid, person_name.clone())
                .await?;
        }

        let rec = sqlx::query!(
            r#"
                INSERT INTO scene_participant (uuid, scene_uuid, person_uuid)
                SELECT $1::UUID, $2::UUID, person.uuid
                FROM person
                WHERE person.name = $3::TEXT
                RETURNING uuid, person_uuid;
            "#,
            SceneParticipantUuid::new().to_uuid(),
            scene_uuid.to_uuid(),
            person_name.as_str(),
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error adding person to scene: {}", err))?;

        sqlx::query!(
            r#"
                INSERT INTO person_scene_visit (
                    person_uuid,
                    scene_uuid,
                    first_visited_at,
                    last_visited_at,
                    visit_count,
                    created_at,
                    updated_at
                )
                VALUES ($1::UUID, $2::UUID, NOW(), NOW(), 1, NOW(), NOW())
                ON CONFLICT (person_uuid, scene_uuid)
                DO UPDATE
                SET last_visited_at = NOW(),
                    visit_count = person_scene_visit.visit_count + 1,
                    updated_at = NOW();
            "#,
            rec.person_uuid,
            scene_uuid.to_uuid(),
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error recording scene visit: {}", err))?;

        let ret = SceneParticipantUuid::from_uuid(rec.uuid);

        Ok(ret)
    }

    async fn remove_person_from_scene(
        &self,
        scene_uuid: SceneUuid,
        person_name: PersonName,
    ) -> Result<SceneParticipantUuid, String> {
        let rec = sqlx::query!(
            r#"
                UPDATE scene_participant
                SET left_at = NOW()
                WHERE scene_participant.scene_uuid = $1::UUID
                  AND scene_participant.person_uuid = (SELECT person.uuid FROM person WHERE person.name = $2::TEXT)
                  AND scene_participant.left_at IS NULL
                RETURNING uuid;
            "#,
            scene_uuid.to_uuid(),
            person_name.as_str(),
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error removing person from scene: {}", err))?;

        let ret = SceneParticipantUuid::from_uuid(rec.uuid);

        Ok(ret)
    }

    async fn get_persons_current_scene(
        &self,
        person_name: PersonName,
    ) -> Result<Option<CurrentScene>, String> {
        let maybe_ret = sqlx::query_as!(
            CurrentScene,
            r#"
                SELECT
                    scene.uuid AS scene_uuid
                FROM scene_participant
                JOIN scene ON scene_participant.scene_uuid = scene.uuid
                JOIN person ON scene_participant.person_uuid = person.uuid
                WHERE person.name = $1::TEXT AND scene_participant.left_at IS NULL
                ORDER BY scene_participant.joined_at DESC
                LIMIT 1;
            "#,
            person_name.as_str(),
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching person's current scene: {}", err))?;

        Ok(maybe_ret)
    }

    async fn get_persons_current_scene_uuid(
        &self,
        person_uuid: &PersonUuid,
    ) -> Result<Option<SceneUuid>, String> {
        let maybe_rec = sqlx::query!(
            r#"
                SELECT
                    scene.uuid AS scene_uuid
                FROM scene_participant
                JOIN scene ON scene_participant.scene_uuid = scene.uuid
                WHERE scene_participant.person_uuid = $1::UUID AND scene_participant.left_at IS NULL
                ORDER BY scene_participant.joined_at DESC
                LIMIT 1;
            "#,
            person_uuid.to_uuid(),
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| {
            format!(
                "Error fetching person {}'s current scene UUID: {}",
                person_uuid.clone().to_string(),
                err
            )
        })?;

        Ok(maybe_rec.map(|rec| SceneUuid::from_uuid(rec.scene_uuid)))
    }

    async fn create_scene_snapshot(
        &self,
        new_scene_snapshot: NewSceneSnapshot,
    ) -> Result<(), String> {
        sqlx::query!(
            r#"
                INSERT INTO scene_snapshot (scene_uuid, description)
                VALUES ($1::UUID, $2::TEXT);
            "#,
            new_scene_snapshot.scene_uuid.to_uuid(),
            new_scene_snapshot.description,
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new scene snapshot: {}", err))?;

        Ok(())
    }

    async fn get_scene_from_name(&self, scene_name: String) -> Result<Option<Scene>, String> {
        let maybe_ret = sqlx::query_as!(
            Scene,
            r#"
                SELECT
                    scene.uuid,
                    scene.name,
                    scene_snapshot.description
                FROM scene
                LEFT JOIN scene_snapshot ON scene.uuid = scene_snapshot.scene_uuid
                WHERE scene.name = $1::TEXT
                  AND scene.ended_at IS NULL
                ORDER BY scene.uuid, scene_snapshot.created_at DESC;
            "#,
            scene_name,
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching scene by name: {}", err))?;

        Ok(maybe_ret)
    }

    async fn get_scene_current_participants(
        &self,
        scene_uuid: &SceneUuid,
    ) -> Result<Vec<SceneParticipant>, String> {
        let participant_rows = sqlx::query!(
            r#"
                SELECT
                    person.name AS person_name,
                    person.uuid AS person_uuid
                FROM scene_participant
                JOIN person ON scene_participant.person_uuid = person.uuid
                WHERE scene_participant.scene_uuid = $1::UUID AND scene_participant.left_at IS NULL;
            "#,
            scene_uuid.to_uuid(),
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching scene participants: {}", err))?;

        let participants = participant_rows
            .into_iter()
            .map(|row| {
                let person_uuid = PersonUuid::from_uuid(row.person_uuid);
                SceneParticipant {
                    person_name: PersonName::from_string(row.person_name),
                    actor_uuid: ActorUuid::from_person_uuid(person_uuid),
                }
            })
            .collect();

        Ok(participants)
    }

    async fn get_scene_participation_history(
        &self,
        scene_uuid: &SceneUuid,
    ) -> Result<Vec<SceneParticipation>, String> {
        let participation_rows = sqlx::query!(
            r#"
                SELECT
                    scene_participant.person_uuid,
                    scene_participant.joined_at,
                    scene_participant.left_at
                FROM scene_participant
                WHERE scene_participant.scene_uuid = $1::UUID
                ORDER BY scene_participant.joined_at ASC;
            "#,
            scene_uuid.to_uuid(),
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching scene participation history: {}", err))?;

        let participation_history = participation_rows
            .into_iter()
            .map(|row| {
                let person_uuid = PersonUuid::from_uuid(row.person_uuid);
                SceneParticipation {
                    person_uuid,
                    joined_at: row.joined_at,
                    left_at: row.left_at,
                }
            })
            .collect();

        Ok(participation_history)
    }

    async fn get_scene_name(&self, scene_uuid: &SceneUuid) -> Result<Option<String>, String> {
        let maybe_rec = sqlx::query!(
            r#"
                SELECT name
                FROM scene
                WHERE uuid = $1::UUID;
            "#,
            scene_uuid.to_uuid(),
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching scene name: {}", err))?;

        Ok(maybe_rec.map(|rec| rec.name))
    }

    async fn get_scene_description(
        &self,
        scene_uuid: &SceneUuid,
    ) -> Result<Option<String>, String> {
        let maybe_rec = sqlx::query!(
            r#"
                SELECT description
                FROM scene_snapshot
                WHERE scene_uuid = $1::UUID
                ORDER BY created_at DESC
                LIMIT 1;
            "#,
            scene_uuid.to_uuid(),
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching scene description: {}", err))?;

        Ok(maybe_rec.map(|rec| rec.description))
    }

    async fn create_scene_from_travel(
        &self,
        scene_name: String,
        basis_scene_uuid: SceneUuid,
    ) -> Result<Scene, String> {
        let maybe_basis_scene_name = self.get_scene_name(&basis_scene_uuid).await?;

        let basis_scene_name = if let Some(name) = maybe_basis_scene_name {
            name
        } else {
            Err("Basis scene not found; cannot derive travel context".to_string())?
        };
        let basis_description_text = self.get_scene_description(&basis_scene_uuid).await?;
        let basis_description_text = match basis_description_text {
            Some(desc) if !desc.trim().is_empty() => desc,
            _ => Err("Basis scene has no description; cannot derive travel context".to_string())?,
        };

        let mut completion = Completion::new(open_ai::model::Model::DEFAULT);
        completion.add_message(
            Role::System,
            "You write concise, vivid scene descriptions for roleplay environments. Return exactly two paragraphs. No bullet points or titles. Write in neutral third-person environmental prose only. Do not address the reader (avoid 'you' and imperative phrasing). Do not include specific people, named actors, or what any individual is doing.",
        );
        completion.add_message(
            Role::User,
            format!(
                "A person is leaving their current scene and traveling to a new destination scene.\n\nCurrent scene name: {}\nCurrent scene description:\n{}\n\nNew destination scene name: {}\n\nWrite a description for the new destination scene that is plausibly connected to the current scene, but distinct. Output exactly two paragraphs.",
                basis_scene_name,
                basis_description_text,
                scene_name
            )
            .as_str(),
        );

        let response = completion
            .send_request(&self.open_ai_key, self.reqwest_client.clone())
            .await
            .map_err(|err| format!("Failed to generate scene description: {}", err.message()))?;

        let description = response.as_message().map_err(|err| {
            format!(
                "Failed to read generated scene description: {}",
                err.message()
            )
        })?;

        let scene_uuid = self
            .create_scene(NewScene {
                name: scene_name.clone(),
                description: description.clone(),
            })
            .await?;

        Ok(Scene {
            uuid: scene_uuid,
            name: scene_name,
            description: Some(description),
        })
    }
}
