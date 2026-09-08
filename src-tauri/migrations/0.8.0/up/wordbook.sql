CREATE TABLE wordbook_entries (
    word_id NVARCHAR(255) PRIMARY KEY,
    word NVARCHAR(255) NOT NULL,
    definition_eng TEXT NOT NULL DEFAULT '',
    definition_zh TEXT NOT NULL DEFAULT '',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- statement-breakpoint
CREATE INDEX idx_wordbook_entries_created_at ON wordbook_entries(created_at DESC);
