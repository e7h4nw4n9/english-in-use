ALTER TABLE study_sessions ADD COLUMN local_date TEXT;
-- statement-breakpoint
ALTER TABLE study_sessions ADD COLUMN timezone_offset_minutes INTEGER NOT NULL DEFAULT 0;

-- statement-breakpoint
UPDATE study_sessions
SET local_date = date(start_at, 'localtime')
WHERE local_date IS NULL;

-- statement-breakpoint
CREATE INDEX IF NOT EXISTS idx_study_sessions_local_date
ON study_sessions(local_date);
