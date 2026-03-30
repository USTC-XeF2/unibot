use crate::models::{FriendRequestEntity, RequestState, UserProfile};

#[derive(sqlx::FromRow)]
pub(super) struct UserRow {
    pub user_id: u64,
    pub nickname: String,
    pub avatar: String,
    pub signature: String,
}

#[derive(sqlx::FromRow)]
pub(super) struct FriendRequestRow {
    pub id: i64,
    pub initiator_user_id: u64,
    pub target_user_id: u64,
    pub comment: String,
    pub state: RequestState,
    pub created_at: u64,
    pub handled_at: Option<u64>,
    pub operator_user_id: Option<u64>,
}

impl From<FriendRequestRow> for FriendRequestEntity {
    fn from(row: FriendRequestRow) -> Self {
        Self {
            request_id: row.id,
            initiator_user_id: row.initiator_user_id,
            target_user_id: row.target_user_id,
            comment: row.comment,
            state: row.state,
            created_at: row.created_at,
            handled_at: row.handled_at,
            operator_user_id: row.operator_user_id,
        }
    }
}

impl From<UserRow> for UserProfile {
    fn from(row: UserRow) -> Self {
        Self {
            user_id: row.user_id,
            nickname: row.nickname,
            avatar: row.avatar,
            signature: row.signature,
        }
    }
}
