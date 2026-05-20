use crate::models::{FriendRequestEntity, UserProfile};

use crate::persistence::repo::codecs;

#[derive(sqlx::FromRow)]
pub(super) struct UserRow {
    pub user_id: String,
    pub nickname: String,
    pub avatar_url: String,
    pub signature: String,
    pub account_status: String,
}

#[derive(sqlx::FromRow)]
pub(super) struct FriendRequestRow {
    pub request_id: String,
    pub initiator_user_id: String,
    pub target_user_id: String,
    pub comment: Option<String>,
    pub state: String,
    pub created_at: u64,
    pub handled_at: Option<u64>,
    pub operator_user_id: Option<String>,
}

impl TryFrom<UserRow> for UserProfile {
    type Error = sqlx::Error;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            user_id: row.user_id,
            nickname: row.nickname,
            avatar: row.avatar_url,
            signature: row.signature,
            account_status: codecs::account_status_from_db(&row.account_status)?,
        })
    }
}

impl TryFrom<FriendRequestRow> for FriendRequestEntity {
    type Error = sqlx::Error;

    fn try_from(row: FriendRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: row.request_id,
            initiator_user_id: row.initiator_user_id,
            target_user_id: row.target_user_id,
            comment: row.comment.unwrap_or_default(),
            state: codecs::request_state_from_db(&row.state)?,
            created_at: row.created_at,
            handled_at: row.handled_at,
            operator_user_id: row.operator_user_id,
        })
    }
}
