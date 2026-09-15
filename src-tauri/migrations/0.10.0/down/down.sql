-- 已使用新算法时禁止丢失间隔历史；空迁移允许正常回退。
CREATE TABLE assessment_interval_downgrade_guard (count INTEGER CHECK (count = 0));
-- statement-breakpoint
INSERT INTO assessment_interval_downgrade_guard SELECT COUNT(*) FROM study_tasks WHERE assessment_local_date IS NOT NULL OR next_interval_days IS NOT NULL;
-- statement-breakpoint
DROP TABLE assessment_interval_downgrade_guard;
-- statement-breakpoint
ALTER TABLE study_tasks DROP COLUMN next_interval_days;
-- statement-breakpoint
ALTER TABLE study_tasks DROP COLUMN assessment_local_date;
