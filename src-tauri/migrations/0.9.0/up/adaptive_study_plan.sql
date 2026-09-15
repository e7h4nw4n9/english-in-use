-- 保留旧任务标识和记录，解除固定七阶段限制并保存每次评价。
CREATE TABLE study_tasks_adaptive (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_unit_id INTEGER NOT NULL,
    scheduled_date DATE NOT NULL,
    review_stage INTEGER NOT NULL CHECK (review_stage >= 1),
    task_status INTEGER NOT NULL DEFAULT 0 CHECK (task_status IN (0, 1)),
    completed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    mastery_rating TEXT CHECK (mastery_rating IN ('forgotten', 'hard', 'good', 'mastered')),
    mastered_streak INTEGER NOT NULL DEFAULT 0,
    UNIQUE (plan_unit_id, review_stage),
    FOREIGN KEY (plan_unit_id) REFERENCES study_plan_units(id) ON DELETE CASCADE
);
-- statement-breakpoint
INSERT INTO study_tasks_adaptive (id, plan_unit_id, scheduled_date, review_stage, task_status, completed_at, created_at, updated_at)
SELECT id, plan_unit_id, scheduled_date, review_stage, task_status, completed_at, created_at, updated_at FROM study_tasks;
-- statement-breakpoint
DROP TABLE study_tasks;
-- statement-breakpoint
ALTER TABLE study_tasks_adaptive RENAME TO study_tasks;
-- statement-breakpoint
CREATE INDEX idx_study_tasks_date_status ON study_tasks (scheduled_date, task_status);
-- statement-breakpoint
CREATE INDEX idx_study_tasks_plan_stage ON study_tasks (plan_unit_id, review_stage);
