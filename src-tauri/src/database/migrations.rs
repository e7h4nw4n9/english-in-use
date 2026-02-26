pub struct Migration {
    pub version: &'static str,
    pub up: &'static str,
    pub down: &'static str,
}

// NOTE: MIGRATIONS must be sorted by version in ascending order.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: "0.1.0",
    up: concat!(
        include_str!("../../migrations/0.1.0/up/_app_meta.sql"),
        "\n",
        include_str!("../../migrations/0.1.0/up/books.sql"),
        "\n",
        include_str!("../../migrations/0.1.0/up/reading_progress.sql")
    ),
    down: concat!(include_str!("../../migrations/0.1.0/down/down.sql")),
}];
