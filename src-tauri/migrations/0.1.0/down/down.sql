-- 降级脚本，删除_app_meta, books 表
delete from _app_meta;
INSERT INTO _app_meta (version) VALUES ('0.0.0');
drop table if exists books;
