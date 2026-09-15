-- 动态任务和评价无法无损降级，有此类数据时通过约束中止整个迁移事务。
CREATE TABLE study_plan_downgrade_guard (value INTEGER CHECK (value = 0));
-- statement-breakpoint
INSERT INTO study_plan_downgrade_guard SELECT COUNT(*) FROM study_tasks WHERE review_stage > 7 OR mastery_rating IS NOT NULL;
-- statement-breakpoint
DROP TABLE study_plan_downgrade_guard;
-- statement-breakpoint
CREATE TABLE study_tasks_legacy (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_unit_id INTEGER NOT NULL,
    scheduled_date DATE NOT NULL,
    review_stage INTEGER NOT NULL,
    task_status INTEGER NOT NULL DEFAULT 0, -- 0=pending, 1=completed
    completed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_study_tasks_plan_stage UNIQUE (plan_unit_id, review_stage),
    CONSTRAINT chk_study_tasks_stage CHECK (review_stage BETWEEN 1 AND 7),
    CONSTRAINT chk_study_tasks_status CHECK (task_status IN (0, 1)),
    FOREIGN KEY (plan_unit_id) REFERENCES study_plan_units(id) ON DELETE CASCADE
);
-- statement-breakpoint
INSERT INTO study_tasks_legacy (id, plan_unit_id, scheduled_date, review_stage, task_status, completed_at, created_at, updated_at)
SELECT id, plan_unit_id, scheduled_date, review_stage, task_status, completed_at, created_at, updated_at FROM study_tasks;
-- statement-breakpoint
DROP TABLE study_tasks;
-- statement-breakpoint
ALTER TABLE study_tasks_legacy RENAME TO study_tasks;
-- statement-breakpoint
CREATE INDEX idx_study_tasks_date_status ON study_tasks (scheduled_date, task_status);
-- statement-breakpoint
CREATE INDEX idx_study_tasks_plan_stage ON study_tasks (plan_unit_id, review_stage);
