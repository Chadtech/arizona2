use crate::capability::person_identity::PersonIdentityCapability;
use crate::domain::logger::Logger;
use crate::nice_display::NiceDisplay;
use crate::worker;

pub enum Error {
    WorkerInit(worker::InitError),
    FetchPeople(sqlx::Error),
    Completion(String),
    UpdateSummary(sqlx::Error),
    DeleteNullSummaries(sqlx::Error),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::WorkerInit(err) => format!("Worker initialization failed: {}", err.message()),
            Error::FetchPeople(err) => format!("Failed to fetch people/identities: {}", err),
            Error::Completion(err) => format!("Failed to summarize identity with LLM: {}", err),
            Error::UpdateSummary(err) => format!("Failed to update identity summary: {}", err),
            Error::DeleteNullSummaries(err) => {
                format!("Failed deleting identities with null summary: {}", err)
            }
        }
    }
}

pub async fn run() -> Result<(), Error> {
    let logger = Logger::init(crate::domain::logger::Level::Info);
    let worker = crate::worker::Worker::new(logger)
        .await
        .map_err(Error::WorkerInit)?;

    let rows = sqlx::query!(
        r#"
            SELECT
                p.uuid AS person_uuid,
                p.name AS person_name,
                latest_identity.uuid AS person_identity_uuid,
                latest_identity.identity
            FROM person p
            LEFT JOIN LATERAL (
                SELECT pi.uuid, pi.identity
                FROM person_identity pi
                WHERE pi.person_uuid = p.uuid
                ORDER BY pi.created_at DESC
                LIMIT 1
            ) AS latest_identity ON true
            ORDER BY p.name ASC;
        "#,
    )
    .fetch_all(&worker.sqlx)
    .await
    .map_err(Error::FetchPeople)?;

    for row in rows {
        let (person_identity_uuid, identity) = match (row.person_identity_uuid, row.identity) {
            (Some(identity_uuid), Some(identity_text)) => (identity_uuid, identity_text),
            _ => {
                println!(
                    "Skipping person {} ({}) because no identity exists",
                    row.person_name, row.person_uuid
                );
                continue;
            }
        };

        let summary = worker
            .summarize_person_identity(&row.person_name, &identity)
            .await
            .map_err(Error::Completion)?;

        sqlx::query!(
            r#"
                UPDATE person_identity
                SET summary = $2::TEXT
                WHERE uuid = $1::UUID;
            "#,
            person_identity_uuid,
            summary
        )
        .execute(&worker.sqlx)
        .await
        .map_err(Error::UpdateSummary)?;

        println!(
            "Updated identity summary for person {} ({}) on identity {}",
            row.person_name, row.person_uuid, person_identity_uuid
        );
    }

    let delete_result = sqlx::query!(
        r#"
            DELETE FROM person_identity
            WHERE summary IS NULL;
        "#
    )
    .execute(&worker.sqlx)
    .await
    .map_err(Error::DeleteNullSummaries)?;

    println!(
        "Deleted {} person_identity rows with null summary",
        delete_result.rows_affected()
    );

    Ok(())
}
