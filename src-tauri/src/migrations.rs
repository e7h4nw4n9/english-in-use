pub struct Migration {
    pub version: &'static str,
    pub sql: &'static str,
}

// NOTE: MIGRATIONS must be sorted by version in ascending order.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: "0.1.0",
        sql: include_str!("../migrations/0.1.0/up.sql"),
    },
];
