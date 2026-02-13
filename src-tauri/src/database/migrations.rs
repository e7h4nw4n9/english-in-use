pub struct Migration {
    pub version: &'static str,
    pub sql: &'static str,
}

// NOTE: MIGRATIONS must be sorted by version in ascending order.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: "0.1.0",
    sql: concat!(
        include_str!("../../migrations/0.1.0/_app_meta.sql"),
        "\n",
        include_str!("../../migrations/0.1.0/books.sql")
    ),
}];
