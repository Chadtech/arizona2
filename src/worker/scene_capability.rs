use crate::capability::scene::{NewScene, NewSceneSnapshot, SceneCapability};
use crate::domain::person_name::PersonName;
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
        todo!()
    }

    async fn remove_person_from_scene(
        &self,
        scene_uuid: SceneUuid,
        person_name: PersonName,
    ) -> Result<SceneParticipantUuid, String> {
        todo!()
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

    async fn get_scene_description(&self, scene_uuid: SceneUuid) -> Result<String, String> {
        todo!()
    }
}
