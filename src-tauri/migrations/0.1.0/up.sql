CREATE TABLE IF NOT EXISTS _app_meta (version TEXT);
INSERT INTO _app_meta (version) SELECT '0.0.0' WHERE NOT EXISTS (SELECT 1 FROM _app_meta);
