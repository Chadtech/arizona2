-- job-runner-enabled

ALTER TABLE job_runner_setting
ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT TRUE;
