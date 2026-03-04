-- Table Name: study_sessions
-- Comment: 阅读器学习计时会话记录

CREATE TABLE IF NOT EXISTS study_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL,
    resource_id VARCHAR(50) NOT NULL,
    unit_name NVARCHAR(100) NOT NULL,
    entry_resource_id VARCHAR(50) NOT NULL,
    entry_unit_name NVARCHAR(100) NOT NULL,
    visited_units_json TEXT,
    start_at DATETIME NOT NULL,
    end_at DATETIME NOT NULL,
    duration INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_study_sessions_duration_non_negative CHECK (duration >= 0),
    CONSTRAINT chk_study_sessions_time_order CHECK (end_at >= start_at),
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_study_sessions_book_start
    ON study_sessions (book_id, start_at);

CREATE INDEX IF NOT EXISTS idx_study_sessions_resource_start
    ON study_sessions (resource_id, start_at);

CREATE INDEX IF NOT EXISTS idx_study_sessions_start
    ON study_sessions (start_at);
