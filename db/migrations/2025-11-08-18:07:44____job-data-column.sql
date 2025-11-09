-- job-data-column

BEGIN;

-- Add JSONB column to store job-specific data (e.g., message UUIDs, person UUIDs)
ALTER TABLE job ADD COLUMN IF NOT EXISTS data JSONB;

COMMIT;