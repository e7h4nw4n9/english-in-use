CREATE INDEX IF NOT EXISTS idx_study_sessions_book_local_date
ON study_sessions(book_id, local_date);

-- statement-breakpoint
PRAGMA optimize;
