-- Table Name: reading_progress
-- Comment: 用户阅读进度记录

CREATE TABLE IF NOT EXISTS reading_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL UNIQUE,
    resource_id VARCHAR(50), -- 对应 definition.json 中的 RE_XXXX
    page_label VARCHAR(20),  -- 对应 book.json 中的 pagelabel
    scale FLOAT DEFAULT 1.0, -- 缩放比例
    offset_x INTEGER DEFAULT 0, -- 滚动偏移 X
    offset_y INTEGER DEFAULT 0, -- 滚动偏移 Y
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);
