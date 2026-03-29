use arizona2::capability::job::JobCapability;
use arizona2::capability::message::MessageCapability;
use arizona2::capability::person::{NewPerson, PersonCapability};
use arizona2::capability::scene::{NewScene, SceneCapability};
use arizona2::db;
use arizona2::domain::job::{JobKind, JobStatus};
use arizona2::domain::logger::{Level, Logger};
use arizona2::domain::message::MessageSender;
use arizona2::domain::person_name::PersonName;
use arizona2::domain::person_uuid::PersonUuid;
use arizona2::job_runner::{run_one_job, RunNextJobResult};
use arizona2::nice_display::NiceDisplay;
use arizona2::open_ai_key::OpenAiKey;
use arizona2::worker::Worker;
use serial_test::serial;
use sqlx::Row;

struct TestContext {
    worker: Worker,
}

impl TestContext {
    async fn new() -> Self {
        dotenv::dotenv().ok();

        let logger = Logger::init(Level::Error);
        let config = db::Config::load().await.unwrap_or_else(|err| {
            panic!(
                "failed to load database config for integration tests: {}",
                err.message()
            )
        });
        let database_url = format!(
            "postgres://{}:{}@{}/arizona2_test",
            config.user, config.password, config.host
        );

        let worker = Worker::from_connection_string(
            logger,
            &database_url,
            OpenAiKey::from_string("test-key".to_string()),
        )
        .await
        .unwrap_or_else(|err| {
            panic!(
                "failed to connect to integration test database 'arizona2_test': {}",
                err.message()
            )
        });

        reset_database(&worker).await;

        Self { worker }
    }

    fn worker(&self) -> &Worker {
        &self.worker
    }
}
async fn reset_database(worker: &Worker) {
    let table_rows = sqlx::query(
        r#"
            SELECT tablename
            FROM pg_tables
            WHERE schemaname = 'public'
              AND tablename <> '_sqlx_migrations'
            ORDER BY tablename ASC
        "#,
    )
    .fetch_all(&worker.sqlx)
    .await
    .expect("failed to list public tables");

    let table_names: Vec<String> = table_rows
        .into_iter()
        .map(|row| {
            row.try_get::<String, _>("tablename")
                .expect("failed to read table name")
        })
        .collect();

    if !table_names.is_empty() {
        let statement = format!(
            "TRUNCATE TABLE {} RESTART IDENTITY CASCADE",
            table_names
                .iter()
                .map(|table_name| format!("\"{}\"", table_name))
                .collect::<Vec<String>>()
                .join(", ")
        );

        sqlx::query(&statement)
            .execute(&worker.sqlx)
            .await
            .expect("failed to truncate database tables");
    }

    sqlx::query(
        r#"
            INSERT INTO active_clock (id, active_ms)
            VALUES (TRUE, 0)
            ON CONFLICT (id)
            DO UPDATE SET active_ms = EXCLUDED.active_ms
        "#,
    )
    .execute(&worker.sqlx)
    .await
    .expect("failed to seed active_clock");
}

fn test_person(name: &str) -> NewPerson {
    NewPerson {
        person_uuid: PersonUuid::new(),
        person_name: PersonName::from_string(name.to_string()),
    }
}

#[tokio::test]
#[serial]
#[ignore = "requires a configured postgres integration database"]
async fn person_capability_round_trip_and_flags() {
    let ctx = TestContext::new().await;
    let worker = ctx.worker();
    let new_person = test_person("Alice");

    let created_person_uuid = worker
        .create_person(NewPerson {
            person_uuid: new_person.person_uuid.clone(),
            person_name: new_person.person_name.clone(),
        })
        .await
        .expect("failed to create person");

    let all_person_uuids = worker
        .get_all_person_uuids()
        .await
        .expect("failed to fetch all person uuids");
    assert_eq!(all_person_uuids.len(), 1);
    assert_eq!(all_person_uuids[0].to_uuid(), created_person_uuid.to_uuid());

    let person_name = worker
        .get_persons_name(created_person_uuid.clone())
        .await
        .expect("failed to fetch person name");
    assert_eq!(person_name.as_str(), "Alice");

    let fetched_person_uuid = worker
        .get_person_uuid_by_name(PersonName::from_string("Alice".to_string()))
        .await
        .expect("failed to fetch person by name");
    assert_eq!(fetched_person_uuid.to_uuid(), created_person_uuid.to_uuid());

    assert!(!worker
        .is_person_hibernating(&created_person_uuid)
        .await
        .expect("failed to read hibernation state"));
    assert!(worker
        .is_person_enabled(&created_person_uuid)
        .await
        .expect("failed to read enabled state"));

    worker
        .set_person_hibernating(&created_person_uuid, true)
        .await
        .expect("failed to set hibernation state");
    worker
        .set_person_enabled(&created_person_uuid, false)
        .await
        .expect("failed to set enabled state");

    assert!(worker
        .is_person_hibernating(&created_person_uuid)
        .await
        .expect("failed to read updated hibernation state"));
    assert!(!worker
        .is_person_enabled(&created_person_uuid)
        .await
        .expect("failed to read updated enabled state"));
}

#[tokio::test]
#[serial]
#[ignore = "requires a configured postgres integration database"]
async fn scene_participants_move_between_scenes_and_history_tracks_departure() {
    let ctx = TestContext::new().await;
    let worker = ctx.worker();
    let new_person = test_person("Bailey");

    worker
        .create_person(NewPerson {
            person_uuid: new_person.person_uuid.clone(),
            person_name: new_person.person_name.clone(),
        })
        .await
        .expect("failed to create person");

    let cafe_scene_uuid = worker
        .create_scene(NewScene {
            name: "Cafe".to_string(),
            description: "A warm corner cafe.".to_string(),
        })
        .await
        .expect("failed to create cafe scene");
    let library_scene_uuid = worker
        .create_scene(NewScene {
            name: "Library".to_string(),
            description: "Tall shelves and quiet tables.".to_string(),
        })
        .await
        .expect("failed to create library scene");

    worker
        .add_person_to_scene(cafe_scene_uuid.clone(), new_person.person_name.clone())
        .await
        .expect("failed to add person to cafe");

    let initial_scene = worker
        .get_persons_current_scene(new_person.person_name.clone())
        .await
        .expect("failed to fetch current scene")
        .expect("expected person to be in a scene");
    assert_eq!(
        initial_scene.scene_uuid.to_uuid(),
        cafe_scene_uuid.to_uuid()
    );

    worker
        .add_person_to_scene(library_scene_uuid.clone(), new_person.person_name.clone())
        .await
        .expect("failed to move person to library");

    let current_scene_uuid = worker
        .get_persons_current_scene_uuid(&new_person.person_uuid)
        .await
        .expect("failed to fetch current scene uuid")
        .expect("expected current scene uuid");
    assert_eq!(current_scene_uuid.to_uuid(), library_scene_uuid.to_uuid());

    let cafe_history = worker
        .get_scene_participation_history(&cafe_scene_uuid)
        .await
        .expect("failed to fetch cafe history");
    assert_eq!(cafe_history.len(), 1);
    assert_eq!(
        cafe_history[0].person_uuid.to_uuid(),
        new_person.person_uuid.to_uuid()
    );
    assert!(cafe_history[0].left_at.is_some());

    let library_participants = worker
        .get_scene_current_participants(&library_scene_uuid)
        .await
        .expect("failed to fetch library participants");
    assert_eq!(library_participants.len(), 1);
    assert_eq!(library_participants[0].person_name.as_str(), "Bailey");
}

#[tokio::test]
#[serial]
#[ignore = "requires a configured postgres integration database"]
async fn deleting_a_scene_removes_it_from_active_lists_and_clears_presence() {
    let ctx = TestContext::new().await;
    let worker = ctx.worker();
    let new_person = test_person("Casey");

    worker
        .create_person(NewPerson {
            person_uuid: new_person.person_uuid.clone(),
            person_name: new_person.person_name.clone(),
        })
        .await
        .expect("failed to create person");

    let scene_uuid = worker
        .create_scene(NewScene {
            name: "Studio".to_string(),
            description: "A quiet recording room.".to_string(),
        })
        .await
        .expect("failed to create studio scene");

    worker
        .add_person_to_scene(scene_uuid.clone(), new_person.person_name.clone())
        .await
        .expect("failed to add person to scene");
    worker
        .delete_scene(&scene_uuid)
        .await
        .expect("failed to delete scene");

    let scenes = worker.get_scenes().await.expect("failed to fetch scenes");
    assert!(scenes.is_empty());

    let current_scene = worker
        .get_persons_current_scene(new_person.person_name.clone())
        .await
        .expect("failed to fetch current scene after delete");
    assert!(current_scene.is_none());

    let history = worker
        .get_scene_participation_history(&scene_uuid)
        .await
        .expect("failed to fetch participation history");
    assert_eq!(history.len(), 1);
    assert!(history[0].left_at.is_some());
}

#[tokio::test]
#[serial]
#[ignore = "requires a configured postgres integration database"]
async fn real_world_user_presence_is_reflected_in_scene_participants() {
    let ctx = TestContext::new().await;
    let worker = ctx.worker();

    let scene_uuid = worker
        .create_scene(NewScene {
            name: "Lounge".to_string(),
            description: "Low lamps and soft chairs.".to_string(),
        })
        .await
        .expect("failed to create lounge scene");

    assert!(!worker
        .is_real_world_user_in_scene(&scene_uuid)
        .await
        .expect("failed to read initial real-world-user state"));

    worker
        .set_real_world_user_in_scene(&scene_uuid, true)
        .await
        .expect("failed to enable real-world-user presence");

    let participants = worker
        .get_scene_current_participants(&scene_uuid)
        .await
        .expect("failed to fetch scene participants");
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0].person_name.as_str(), "Chadtech");

    worker
        .set_real_world_user_in_scene(&scene_uuid, false)
        .await
        .expect("failed to disable real-world-user presence");

    let participants = worker
        .get_scene_current_participants(&scene_uuid)
        .await
        .expect("failed to fetch scene participants after removal");
    assert!(participants.is_empty());
}

#[tokio::test]
#[serial]
#[ignore = "requires a configured postgres integration database"]
async fn message_flow_supports_paging_and_recipient_handling() {
    let ctx = TestContext::new().await;
    let worker = ctx.worker();
    let sender = test_person("Devon");
    let recipient = test_person("Elliot");

    worker
        .create_person(NewPerson {
            person_uuid: sender.person_uuid.clone(),
            person_name: sender.person_name.clone(),
        })
        .await
        .expect("failed to create sender");
    worker
        .create_person(NewPerson {
            person_uuid: recipient.person_uuid.clone(),
            person_name: recipient.person_name.clone(),
        })
        .await
        .expect("failed to create recipient");

    let scene_uuid = worker
        .create_scene(NewScene {
            name: "Atrium".to_string(),
            description: "Open skylights and polished stone.".to_string(),
        })
        .await
        .expect("failed to create atrium scene");

    let older_message_uuid = worker
        .send_scene_message(
            MessageSender::AiPerson(sender.person_uuid.clone()),
            scene_uuid.clone(),
            "older message".to_string(),
        )
        .await
        .expect("failed to send older message");

    sqlx::query(
        r#"
            UPDATE message
            SET sent_at = NOW() - INTERVAL '1 second'
            WHERE uuid = $1::UUID
        "#,
    )
    .bind(older_message_uuid.to_uuid())
    .execute(&worker.sqlx)
    .await
    .expect("failed to backdate first message");

    let newer_message_uuid = worker
        .send_scene_message(
            MessageSender::RealWorldUser,
            scene_uuid.clone(),
            "newer message".to_string(),
        )
        .await
        .expect("failed to send newer message");

    worker
        .add_scene_message_recipients(&older_message_uuid, vec![recipient.person_uuid.clone()])
        .await
        .expect("failed to add recipient to older message");
    worker
        .add_scene_message_recipients(&newer_message_uuid, vec![recipient.person_uuid.clone()])
        .await
        .expect("failed to add recipient to newer message");

    let page = worker
        .get_messages_in_scene_page(&scene_uuid, 10, None)
        .await
        .expect("failed to fetch message page");
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].content, "newer message");
    assert_eq!(page[1].content, "older message");

    let newer_message = worker
        .get_message_by_uuid(&newer_message_uuid)
        .await
        .expect("failed to fetch newer message")
        .expect("expected newer message to exist");
    assert_eq!(newer_message.content, "newer message");

    let unhandled = worker
        .get_unhandled_scene_messages_for_person(&recipient.person_uuid, &scene_uuid)
        .await
        .expect("failed to fetch unhandled messages");
    assert_eq!(unhandled.len(), 2);
    assert_eq!(unhandled[0].content, "older message");
    assert_eq!(unhandled[1].content, "newer message");

    worker
        .mark_scene_messages_handled_for_person(
            &recipient.person_uuid,
            vec![older_message_uuid.clone()],
        )
        .await
        .expect("failed to mark message handled");

    let remaining_unhandled = worker
        .get_unhandled_scene_messages_for_person(&recipient.person_uuid, &scene_uuid)
        .await
        .expect("failed to fetch remaining unhandled messages");
    assert_eq!(remaining_unhandled.len(), 1);
    assert_eq!(
        remaining_unhandled[0].uuid.to_uuid(),
        newer_message_uuid.to_uuid()
    );
}

#[tokio::test]
#[serial]
#[ignore = "requires a configured postgres integration database"]
async fn ping_job_runs_end_to_end_and_marks_the_job_finished() {
    let ctx = TestContext::new().await;
    let worker = ctx.worker().clone();

    worker
        .unshift_job(JobKind::Ping)
        .await
        .expect("failed to create ping job");

    let recent_jobs = worker
        .recent_jobs(10)
        .await
        .expect("failed to fetch recent jobs");
    assert_eq!(recent_jobs.len(), 1);
    assert_eq!(recent_jobs[0].status(), JobStatus::NotStarted);

    let random_seed = worker.get_random_seed().expect("failed to get random seed");
    let result = match run_one_job(worker.clone(), random_seed).await {
        Ok(result) => result,
        Err(err) => panic!("failed to run one job: {}", err.message()),
    };

    let job_uuid = match result {
        RunNextJobResult::RanJob { job_uuid, job_kind } => {
            assert_eq!(job_kind, "ping");
            job_uuid
        }
        RunNextJobResult::NoJob => panic!("expected a ping job to run"),
        RunNextJobResult::Deferred { .. } => panic!("expected ping job to complete"),
    };

    let persisted_job = worker
        .get_job_by_uuid(&job_uuid)
        .await
        .expect("failed to fetch job by uuid")
        .expect("expected persisted job");
    assert_eq!(persisted_job.status(), JobStatus::Finished);
    assert!(persisted_job.finished_at().is_some());
    assert!(persisted_job.error().is_none());
}

#[tokio::test]
#[serial]
#[ignore = "requires a configured postgres integration database"]
async fn deleted_jobs_are_hidden_from_active_queries() {
    let ctx = TestContext::new().await;
    let worker = ctx.worker();

    worker
        .unshift_job(JobKind::Ping)
        .await
        .expect("failed to create ping job");

    let recent_jobs = worker
        .recent_jobs(10)
        .await
        .expect("failed to fetch recent jobs");
    assert_eq!(recent_jobs.len(), 1);

    let job_uuid = recent_jobs[0].uuid().clone();
    worker
        .delete_job(&job_uuid)
        .await
        .expect("failed to delete job");

    let deleted_job = worker
        .get_job_by_uuid(&job_uuid)
        .await
        .expect("failed to fetch deleted job");
    assert!(deleted_job.is_none());

    let recent_jobs = worker
        .recent_jobs(10)
        .await
        .expect("failed to fetch recent jobs after delete");
    assert!(recent_jobs.is_empty());
}
