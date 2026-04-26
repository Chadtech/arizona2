use crate::capability::person_task::{NewPersonTask, PersonTaskCapability};
use crate::domain::logger::Level;
use crate::domain::person_task::{PersonTask, PersonTaskTerminalOutcome};
use crate::domain::person_task_uuid::PersonTaskUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;

impl PersonTaskCapability for Worker {
    async fn get_persons_current_active_task(
        &self,
        person_uuid: &PersonUuid,
    ) -> Result<Option<PersonTask>, String> {
        let current_task_rec = sqlx::query!(
            r#"
                SELECT current_person_task_uuid
                FROM person
                WHERE uuid = $1::UUID;
            "#,
            person_uuid.to_uuid()
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching current person task UUID: {}", err))?;

        let current_task_rec = match current_task_rec {
            Some(rec) => rec,
            None => return Err(format!("Person {} not found", person_uuid.to_uuid())),
        };

        let person_task_uuid = match current_task_rec.current_person_task_uuid {
            Some(person_task_uuid) => PersonTaskUuid::from_uuid(person_task_uuid),
            None => return Ok(None),
        };

        let maybe_task_row = sqlx::query!(
            r#"
                SELECT uuid,
                       person_uuid,
                       content,
                       state,
                       success_condition,
                       abandon_condition,
                       failure_condition,
                       priority,
                       created_at,
                       completed_at,
                       abandoned_at,
                       failed_at
                FROM person_task
                WHERE uuid = $1::UUID
                  AND person_uuid = $2::UUID
                  AND completed_at IS NULL
                  AND abandoned_at IS NULL
                  AND failed_at IS NULL;
            "#,
            person_task_uuid.to_uuid(),
            person_uuid.to_uuid()
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching current active person task: {}", err))?;

        match maybe_task_row {
            Some(row) => Ok(Some(PersonTask {
                uuid: PersonTaskUuid::from_uuid(row.uuid),
                person_uuid: PersonUuid::from_uuid(row.person_uuid),
                content: row.content,
                state: row.state,
                success_condition: row.success_condition,
                abandon_condition: row.abandon_condition,
                failure_condition: row.failure_condition,
                priority: row.priority,
                created_at: row.created_at,
                completed_at: row.completed_at,
                abandoned_at: row.abandoned_at,
                failed_at: row.failed_at,
            })),
            None => {
                let stale_task_state = sqlx::query!(
                    r#"
                        SELECT completed_at,
                               abandoned_at,
                               failed_at
                        FROM person_task
                        WHERE uuid = $1::UUID
                          AND person_uuid = $2::UUID;
                    "#,
                    person_task_uuid.to_uuid(),
                    person_uuid.to_uuid()
                )
                .fetch_optional(&self.sqlx)
                .await
                .map_err(|err| format!("Error checking stale current person task: {}", err))?;

                sqlx::query!(
                    r#"
                        UPDATE person
                        SET current_person_task_uuid = NULL,
                            updated_at = NOW()
                        WHERE uuid = $1::UUID
                          AND current_person_task_uuid = $2::UUID;
                    "#,
                    person_uuid.to_uuid(),
                    person_task_uuid.to_uuid()
                )
                .execute(&self.sqlx)
                .await
                .map_err(|err| {
                    format!("Error clearing stale current person task pointer: {}", err)
                })?;

                let stale_reason = match stale_task_state {
                    Some(task_state) if task_state.completed_at.is_some() => {
                        "task was already completed"
                    }
                    Some(task_state) if task_state.abandoned_at.is_some() => {
                        "task was already abandoned"
                    }
                    Some(task_state) if task_state.failed_at.is_some() => "task was already failed",
                    Some(_) => "task record existed but was not returned as active",
                    None => "task record was missing",
                };

                self.logger.log(
                    Level::Warning,
                    format!(
                        "Cleared stale current task pointer for person {} to task {} because {}.",
                        person_uuid.to_uuid(),
                        person_task_uuid.to_uuid(),
                        stale_reason
                    )
                    .as_str(),
                );

                Ok(None)
            }
        }
    }

    async fn set_persons_current_active_task(
        &self,
        new_person_task: NewPersonTask,
    ) -> Result<PersonTaskUuid, String> {
        let person_task_uuid = PersonTaskUuid::new();
        let mut transaction = self
            .sqlx
            .begin()
            .await
            .map_err(|err| format!("Error starting person task transaction: {}", err))?;

        let result = sqlx::query!(
            r#"
                INSERT INTO person_task (
                    uuid,
                    person_uuid,
                    content,
                    state,
                    success_condition,
                    abandon_condition,
                    failure_condition,
                    priority
                )
                VALUES (
                    $1::UUID,
                    $2::UUID,
                    $3::TEXT,
                    $4::TEXT,
                    $5::TEXT,
                    $6::TEXT,
                    $7::TEXT,
                    $8::INTEGER
                );
            "#,
            person_task_uuid.to_uuid(),
            new_person_task.person_uuid.to_uuid(),
            new_person_task.content,
            new_person_task.state,
            new_person_task.success_condition,
            new_person_task.abandon_condition,
            new_person_task.failure_condition,
            new_person_task.priority
        )
        .execute(&mut *transaction)
        .await
        .map_err(|err| format!("Error creating current person task: {}", err))?;

        if result.rows_affected() != 1 {
            return Err("Expected one person task to be inserted".to_string());
        }

        let result = sqlx::query!(
            r#"
                UPDATE person
                SET current_person_task_uuid = $2::UUID,
                    updated_at = NOW()
                WHERE uuid = $1::UUID;
            "#,
            new_person_task.person_uuid.to_uuid(),
            person_task_uuid.to_uuid()
        )
        .execute(&mut *transaction)
        .await
        .map_err(|err| format!("Error setting current person task: {}", err))?;

        if result.rows_affected() == 0 {
            return Err(format!(
                "Person {} not found",
                new_person_task.person_uuid.to_uuid()
            ));
        }

        transaction
            .commit()
            .await
            .map_err(|err| format!("Error committing person task transaction: {}", err))?;

        Ok(person_task_uuid)
    }

    async fn transition_person_task(
        &self,
        person_uuid: &PersonUuid,
        person_task_uuid: &PersonTaskUuid,
        outcome: PersonTaskTerminalOutcome,
    ) -> Result<(), String> {
        let mut transaction =
            self.sqlx.begin().await.map_err(|err| {
                format!("Error starting person task transition transaction: {}", err)
            })?;

        let task_state = sqlx::query!(
            r#"
                SELECT completed_at,
                       abandoned_at,
                       failed_at
                FROM person_task
                WHERE uuid = $1::UUID
                  AND person_uuid = $2::UUID;
            "#,
            person_task_uuid.to_uuid(),
            person_uuid.to_uuid()
        )
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|err| format!("Error fetching person task for transition: {}", err))?;

        let task_state = match task_state {
            Some(task_state) => task_state,
            None => {
                return Err(format!(
                    "Task {} for person {} not found",
                    person_task_uuid.to_uuid(),
                    person_uuid.to_uuid()
                ));
            }
        };

        if task_state.completed_at.is_some()
            || task_state.abandoned_at.is_some()
            || task_state.failed_at.is_some()
        {
            return Err(format!(
                "Task {} is already terminal",
                person_task_uuid.to_uuid()
            ));
        }

        let transition_result = match outcome {
            PersonTaskTerminalOutcome::Completed => sqlx::query!(
                r#"
                    UPDATE person_task
                    SET completed_at = NOW()
                        WHERE uuid = $1::UUID
                          AND person_uuid = $2::UUID
                          AND completed_at IS NULL
                          AND abandoned_at IS NULL
                          AND failed_at IS NULL;
                    "#,
                person_task_uuid.to_uuid(),
                person_uuid.to_uuid()
            )
            .execute(&mut *transaction)
            .await
            .map_err(|err| format!("Error completing person task: {}", err))?,
            PersonTaskTerminalOutcome::Failed => sqlx::query!(
                r#"
                    UPDATE person_task
                    SET failed_at = NOW()
                    WHERE uuid = $1::UUID
                      AND person_uuid = $2::UUID
                      AND completed_at IS NULL
                      AND abandoned_at IS NULL
                      AND failed_at IS NULL;
                "#,
                person_task_uuid.to_uuid(),
                person_uuid.to_uuid()
            )
            .execute(&mut *transaction)
            .await
            .map_err(|err| format!("Error failing person task: {}", err))?,
            PersonTaskTerminalOutcome::Abandoned => sqlx::query!(
                r#"
                    UPDATE person_task
                    SET abandoned_at = NOW()
                    WHERE uuid = $1::UUID
                      AND person_uuid = $2::UUID
                      AND completed_at IS NULL
                      AND abandoned_at IS NULL
                      AND failed_at IS NULL;
                "#,
                person_task_uuid.to_uuid(),
                person_uuid.to_uuid()
            )
            .execute(&mut *transaction)
            .await
            .map_err(|err| format!("Error abandoning person task: {}", err))?,
        };

        if transition_result.rows_affected() != 1 {
            return Err(format!(
                "Expected one task row to transition for {}",
                person_task_uuid.to_uuid()
            ));
        }

        sqlx::query!(
            r#"
                UPDATE person
                SET current_person_task_uuid = NULL,
                    updated_at = NOW()
                WHERE uuid = $1::UUID
                  AND current_person_task_uuid = $2::UUID;
            "#,
            person_uuid.to_uuid(),
            person_task_uuid.to_uuid()
        )
        .execute(&mut *transaction)
        .await
        .map_err(|err| format!("Error clearing current person task pointer: {}", err))?;

        transaction.commit().await.map_err(|err| {
            format!(
                "Error committing person task transition transaction: {}",
                err
            )
        })
    }

    async fn update_person_task_state(
        &self,
        person_uuid: &PersonUuid,
        person_task_uuid: &PersonTaskUuid,
        state: String,
    ) -> Result<(), String> {
        let normalized_state = state.trim().to_string();
        if normalized_state.is_empty() {
            return Err("Person task state cannot be empty".to_string());
        }

        let result = sqlx::query!(
            r#"
                UPDATE person_task
                SET state = $3::TEXT
                WHERE uuid = $1::UUID
                  AND person_uuid = $2::UUID
                  AND completed_at IS NULL
                  AND abandoned_at IS NULL
                  AND failed_at IS NULL;
            "#,
            person_task_uuid.to_uuid(),
            person_uuid.to_uuid(),
            normalized_state
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error updating person task state: {}", err))?;

        if result.rows_affected() != 1 {
            return Err(format!(
                "Expected one active task row to update state for {}",
                person_task_uuid.to_uuid()
            ));
        }

        Ok(())
    }
}
