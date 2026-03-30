use sqlx::SqlitePool;

mod friends;
mod profile;
mod schema;
mod types;

#[derive(Debug, Clone)]
pub struct NewFriendRequestRecord {
    pub initiator_user_id: u64,
    pub target_user_id: u64,
    pub comment: String,
    pub created_at: u64,
}

#[derive(sqlx::FromRow)]
pub struct FriendshipRow {
    pub friend_user_id: u64,
}

#[derive(Clone)]
pub struct UserRepo {
    pool: SqlitePool,
}

impl UserRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}
