DROP INDEX IF EXISTS idx_study_sessions_local_date;
-- statement-breakpoint
ALTER TABLE study_sessions DROP COLUMN timezone_offset_minutes;
-- statement-breakpoint
ALTER TABLE study_sessions DROP COLUMN local_date;
