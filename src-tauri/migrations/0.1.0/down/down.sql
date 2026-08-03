-- 降级脚本，删除_app_meta, books, reading_progress 表
delete from _app_meta;
-- statement-breakpoint
INSERT INTO _app_meta (version) VALUES ('0.0.0');
-- statement-breakpoint
drop table if exists reading_progress;
-- statement-breakpoint
drop table if exists books;
