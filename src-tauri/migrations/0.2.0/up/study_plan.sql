-- Table Name: study_plan_units
-- Comment: 学习计划单元（按书籍 + 资源唯一）

CREATE TABLE IF NOT EXISTS study_plan_units (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL,
    resource_id VARCHAR(50) NOT NULL,
    unit_name NVARCHAR(100) NOT NULL,
    plan_status INTEGER NOT NULL DEFAULT 0, -- 0=in_progress, 1=mastered, 2=abandoned
    last_review_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_study_plan_units_book_resource UNIQUE (book_id, resource_id),
    CONSTRAINT chk_study_plan_units_status CHECK (plan_status IN (0, 1, 2)),
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);

-- statement-breakpoint
CREATE INDEX IF NOT EXISTS idx_study_plan_units_book_status
    ON study_plan_units (book_id, plan_status);

-- Table Name: study_tasks
-- Comment: 学习任务明细（每个计划固定7阶段）

-- statement-breakpoint
CREATE TABLE IF NOT EXISTS study_tasks (
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
CREATE INDEX IF NOT EXISTS idx_study_tasks_date_status
    ON study_tasks (scheduled_date, task_status);

-- statement-breakpoint
CREATE INDEX IF NOT EXISTS idx_study_tasks_plan_stage
    ON study_tasks (plan_unit_id, review_stage);
