-- Table Name: _app_meta
-- Comment: 系统元数据

CREATE TABLE IF NOT EXISTS _app_meta (
    version VARCHAR(64) NOT NULL
);

-- Initial Data
INSERT INTO _app_meta (version) VALUES ('0.1.0');
