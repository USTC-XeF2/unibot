use serde::{Deserialize, Serialize};

use super::{GroupRequestType, MessageSource, RequestState};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum MessageSegment {
    Text { text: String },
    Image { file: String, url: String },
    At { target: u64 },
    Reply { message_id: i64 },
    Face { id: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoticeType {
    Mute,
    Kick,
    AdminChange,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InternalEvent {
    Message {
        sender: u64,
        group_id: Option<u64>,
        content: Vec<MessageSegment>,
        time: u64,
    },
    MessageRecalled {
        message_id: i64,
        source: MessageSource,
        recalled_by_user_id: u64,
        time: u64,
    },
    MessageReaction {
        reaction_id: i64,
        message_id: i64,
        source: MessageSource,
        operator_user_id: u64,
        face_id: String,
        is_add: bool,
        time: u64,
    },
    Poke {
        poke_id: i64,
        source: MessageSource,
        sender_user_id: u64,
        target_user_id: u64,
        time: u64,
    },
    FriendRequestCreated {
        request_id: i64,
        initiator_user_id: u64,
        target_user_id: u64,
        time: u64,
    },
    FriendRequestHandled {
        request_id: i64,
        initiator_user_id: u64,
        target_user_id: u64,
        operator_user_id: u64,
        state: RequestState,
        time: u64,
    },
    GroupRequestCreated {
        request_id: i64,
        group_id: u64,
        request_type: GroupRequestType,
        initiator_user_id: u64,
        target_user_id: Option<u64>,
        time: u64,
    },
    GroupRequestHandled {
        request_id: i64,
        group_id: u64,
        request_type: GroupRequestType,
        initiator_user_id: u64,
        target_user_id: Option<u64>,
        operator_user_id: u64,
        state: RequestState,
        time: u64,
    },
    GroupMemberMuted {
        group_id: u64,
        operator_user_id: u64,
        target_user_id: u64,
        mute_until: Option<u64>,
        time: u64,
    },
    GroupMemberJoined {
        group_id: u64,
        operator_user_id: u64,
        target_user_id: u64,
        time: u64,
    },
    GroupMemberTitleUpdated {
        group_id: u64,
        operator_user_id: u64,
        target_user_id: u64,
        time: u64,
    },
    GroupWholeMuteUpdated {
        group_id: u64,
        operator_user_id: u64,
        muted: bool,
        mute_until: Option<u64>,
        time: u64,
    },
    GroupAnnouncementUpserted {
        announcement_id: String,
        group_id: u64,
        sender_user_id: u64,
        time: u64,
    },
    GroupFolderUpserted {
        folder_id: String,
        group_id: u64,
        creator_user_id: u64,
        time: u64,
    },
    GroupFileUpserted {
        file_id: String,
        group_id: u64,
        uploader_user_id: u64,
        time: u64,
    },
    GroupEssenceUpdated {
        essence_id: i64,
        group_id: u64,
        message_id: i64,
        sender_user_id: u64,
        operator_user_id: u64,
        is_set: bool,
        time: u64,
    },
    Notice {
        group_id: u64,
        actor: u64,
        target: u64,
        notice_type: NoticeType,
        time: u64,
    },
}
