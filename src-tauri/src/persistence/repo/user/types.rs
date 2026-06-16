use crate::models::{FriendCategoryEntity, FriendRequestEntity, FriendshipEntity, UserProfile};

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
}

#[derive(sqlx::FromRow)]
pub(super) struct FriendCategoryRow {
    pub category_id: String,
    pub owner_user_id: String,
    pub name: String,
    pub sort_order: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(sqlx::FromRow)]
pub(super) struct FriendshipDetailRow {
    pub friend_user_id: String,
    pub friend_category_id: Option<String>,
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
            comment: row.comment,
            state: codecs::request_state_from_db(&row.state)?,
            created_at: row.created_at,
            handled_at: row.handled_at,
        })
    }
}

impl TryFrom<FriendCategoryRow> for FriendCategoryEntity {
    type Error = sqlx::Error;

    fn try_from(row: FriendCategoryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            category_id: row.category_id,
            owner_user_id: row.owner_user_id,
            name: row.name,
            sort_order: row.sort_order,
            created_at: row.created_at as u64,
            updated_at: row.updated_at as u64,
        })
    }
}

impl From<FriendshipDetailRow> for FriendshipEntity {
    fn from(row: FriendshipDetailRow) -> Self {
        Self {
            friend_user_id: row.friend_user_id,
            category_id: row.friend_category_id,
        }
    }
}
