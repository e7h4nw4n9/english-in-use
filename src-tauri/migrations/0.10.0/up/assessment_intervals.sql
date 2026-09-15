-- 保存真实评估日期和算法安排的间隔，避免将逾期时间当作记忆能力。
ALTER TABLE study_tasks ADD COLUMN assessment_local_date DATE;
-- statement-breakpoint
ALTER TABLE study_tasks ADD COLUMN next_interval_days INTEGER CHECK (next_interval_days > 0);
