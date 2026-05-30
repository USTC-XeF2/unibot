use crate::models::{AccountStatus, GroupRequestType, GroupRole, GroupStatus, RequestState};

pub fn account_status_from_db(value: &str) -> Result<AccountStatus, sqlx::Error> {
    match value {
        "active" => Ok(AccountStatus::Active),
        "disabled" => Ok(AccountStatus::Disabled),
        "unavailable" => Ok(AccountStatus::Unavailable),
        "deleted" => Ok(AccountStatus::Deleted),
        _ => Err(sqlx::Error::Protocol(format!(
            "unknown account status: {value}"
        ))),
    }
}

pub fn group_status_from_db(value: &str) -> Result<GroupStatus, sqlx::Error> {
    match value {
        "active" => Ok(GroupStatus::Active),
        "dissolved" => Ok(GroupStatus::Dissolved),
        "unavailable" => Ok(GroupStatus::Unavailable),
        _ => Err(sqlx::Error::Protocol(format!(
            "unknown group status: {value}"
        ))),
    }
}

pub fn group_role_to_db(role: GroupRole) -> &'static str {
    match role {
        GroupRole::Owner => "owner",
        GroupRole::Admin => "admin",
        GroupRole::Member => "member",
    }
}

pub fn group_role_from_db(value: &str) -> Result<GroupRole, sqlx::Error> {
    match value {
        "owner" => Ok(GroupRole::Owner),
        "admin" => Ok(GroupRole::Admin),
        "member" => Ok(GroupRole::Member),
        _ => Err(sqlx::Error::Protocol(format!(
            "unknown group role: {value}"
        ))),
    }
}

pub fn request_state_to_db(state: RequestState) -> &'static str {
    match state {
        RequestState::Pending => "pending",
        RequestState::Accepted => "accepted",
        RequestState::Rejected => "rejected",
        RequestState::Ignored => "ignored",
    }
}

pub fn request_state_from_db(value: &str) -> Result<RequestState, sqlx::Error> {
    match value {
        "pending" => Ok(RequestState::Pending),
        "accepted" => Ok(RequestState::Accepted),
        "rejected" => Ok(RequestState::Rejected),
        "ignored" => Ok(RequestState::Ignored),
        _ => Err(sqlx::Error::Protocol(format!(
            "unknown request state: {value}"
        ))),
    }
}

pub fn group_request_type_to_db(value: GroupRequestType) -> &'static str {
    match value {
        GroupRequestType::Join => "join",
        GroupRequestType::Invite => "invite",
    }
}

pub fn group_request_type_from_db(value: &str) -> Result<GroupRequestType, sqlx::Error> {
    match value {
        "join" => Ok(GroupRequestType::Join),
        "invite" => Ok(GroupRequestType::Invite),
        _ => Err(sqlx::Error::Protocol(format!(
            "unknown group request type: {value}"
        ))),
    }
}
