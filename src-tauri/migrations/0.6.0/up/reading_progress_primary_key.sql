CREATE TABLE reading_progress_new (
    book_id INTEGER PRIMARY KEY,
    resource_id VARCHAR(50),
    page_label VARCHAR(20),
    scale FLOAT DEFAULT 1.0,
    offset_x INTEGER DEFAULT 0,
    offset_y INTEGER DEFAULT 0,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);

-- statement-breakpoint
INSERT INTO reading_progress_new (
    book_id,
    resource_id,
    page_label,
    scale,
    offset_x,
    offset_y,
    updated_at
)
SELECT
    book_id,
    resource_id,
    page_label,
    scale,
    offset_x,
    offset_y,
    updated_at
FROM reading_progress;

-- statement-breakpoint
DROP TABLE reading_progress;

-- statement-breakpoint
ALTER TABLE reading_progress_new RENAME TO reading_progress;
