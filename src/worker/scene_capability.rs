use crate::capability::scene::{
    CurrentScene, NewScene, NewSceneSnapshot, Scene, SceneCapability, SceneParticipant,
    SceneParticipation,
};
use crate::domain::actor_uuid::ActorUuid;
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_participant_uuid::SceneParticipantUuid;
use crate::domain::scene_uuid::SceneUuid;
use crate::worker::Worker;
use async_trait::async_trait;

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
                RETURNING uuid;
            "#,
            SceneParticipantUuid::new().to_uuid(),
            scene_uuid.to_uuid(),
            person_name.to_string(),
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error adding person to scene: {}", err))?;

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
            person_name.to_string(),
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
                    scene.uuid AS scene_uuid,
                    scene_participant.uuid AS scene_participant_uuid
                FROM scene_participant
                JOIN scene ON scene_participant.scene_uuid = scene.uuid
                JOIN person ON scene_participant.person_uuid = person.uuid
                WHERE person.name = $1::TEXT AND scene_participant.left_at IS NULL
                ORDER BY scene_participant.joined_at DESC
                LIMIT 1;
            "#,
            person_name.to_string(),
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching person's current scene: {}", err))?;

        Ok(maybe_ret)
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
}
