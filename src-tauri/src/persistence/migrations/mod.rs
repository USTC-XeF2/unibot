pub struct Migration {
    pub version: &'static str,
    pub sql: &'static str,
}

pub fn all_migrations() -> Vec<Migration> {
    vec![Migration {
        version: "0001",
        sql: include_str!("0001_initial_schema.sql"),
    }]
}
