-- job-runner-settings

BEGIN;
CREATE TABLE IF NOT EXISTS job_runner_setting (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE,
    poll_interval_secs BIGINT NOT NULL DEFAULT 45 CHECK (poll_interval_secs > 0)
);

INSERT INTO job_runner_setting (id, poll_interval_secs)
VALUES (TRUE, 45)
ON CONFLICT (id) DO NOTHING;
COMMIT;
