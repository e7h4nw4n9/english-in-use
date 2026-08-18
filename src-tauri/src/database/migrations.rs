pub struct Migration {
    pub version: &'static str,
    pub up: &'static str,
    pub down: &'static str,
}

// 迁移必须按版本号升序排列。
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: "0.1.0",
        up: concat!(
            include_str!("../../migrations/0.1.0/up/_app_meta.sql"),
            "\n",
            include_str!("../../migrations/0.1.0/up/books.sql"),
            "\n",
            include_str!("../../migrations/0.1.0/up/reading_progress.sql")
        ),
        down: concat!(include_str!("../../migrations/0.1.0/down/down.sql")),
    },
    Migration {
        version: "0.2.0",
        up: concat!(include_str!("../../migrations/0.2.0/up/study_plan.sql")),
        down: concat!(include_str!("../../migrations/0.2.0/down/down.sql")),
    },
    Migration {
        version: "0.3.0",
        up: concat!(include_str!("../../migrations/0.3.0/up/study_sessions.sql")),
        down: concat!(include_str!("../../migrations/0.3.0/down/down.sql")),
    },
    Migration {
        version: "0.4.0",
        up: concat!(include_str!(
            "../../migrations/0.4.0/up/study_session_local_date.sql"
        )),
        down: concat!(include_str!("../../migrations/0.4.0/down/down.sql")),
    },
    Migration {
        version: "0.5.0",
        up: concat!(include_str!(
            "../../migrations/0.5.0/up/study_session_book_local_date.sql"
        )),
        down: concat!(include_str!("../../migrations/0.5.0/down/down.sql")),
    },
    Migration {
        version: "0.6.0",
        up: concat!(include_str!(
            "../../migrations/0.6.0/up/reading_progress_primary_key.sql"
        )),
        down: concat!(include_str!("../../migrations/0.6.0/down/down.sql")),
    },
    Migration {
        version: "0.7.0",
        up: concat!(include_str!(
            "../../migrations/0.7.0/up/book_short_title.sql"
        )),
        down: concat!(include_str!("../../migrations/0.7.0/down/down.sql")),
    },
];
