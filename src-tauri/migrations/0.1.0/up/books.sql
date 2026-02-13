-- Table Name: books
-- Comment: 书籍信息

CREATE TABLE IF NOT EXISTS books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_group INTEGER NOT NULL, -- 分组，1-vocabulary;2-grammar
    product_code VARCHAR(60) NOT NULL UNIQUE, -- 书籍编码
    title NVARCHAR(100) NOT NULL, -- 书籍名称
    author VARCHAR(60), -- 作者
    product_type VARCHAR(50) NOT NULL, -- 书籍类型，可选项：imgbook
    cover TEXT, -- 书籍封面图片的base64字符串
    sort_num INTEGER NOT NULL -- 排序序号
);

-- Initial Data
INSERT OR IGNORE INTO books (book_group, product_code, title, author, product_type, cover, sort_num) VALUES
(1, 'eviupreebk', 'English Vocabulary in Use Pre-intermediate and Intermediate Fouth Edition', NULL, 'imgbook', 'cover.jpg', 2),
(1, 'eviuadvebk', 'English Vocabulary in Use Advanced Third Edition', NULL, 'imgbook', 'cover.jpg', 4),
(2, 'essgiuebk', 'Essential Grammar in Use Fourth Edition', NULL, 'imgbook', 'cover.jpg', 1);
