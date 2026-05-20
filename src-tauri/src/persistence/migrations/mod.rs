#![allow(dead_code)]

pub struct Migration {
    pub version: &'static str,
    pub description: &'static str,
    pub sql: &'static str,
}

pub fn all_migrations() -> Vec<Migration> {
    vec![Migration {
        version: "0001",
        description: "initial_schema",
        sql: include_str!("0001_initial_schema.sql"),
    }]
}