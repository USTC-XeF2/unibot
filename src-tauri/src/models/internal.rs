use serde::{Deserialize, Serialize};

use super::{DbId, GroupRequestType, MessageSource, RequestState};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum MessageSegment {
    Text { text: String },
    Image { file: String, url: String },
    At { target: String },
    AtAll,
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
        message_id: DbId,
        message_seq: i64,
        sender_user_id: DbId,
        source: MessageSource,
        content: Vec<MessageSegment>,
        origin_bot_id: Option<DbId>,
        time: u64,
    },
    MessageRecalled {
        message_id: DbId,
        source: MessageSource,
        recalled_by_user_id: DbId,
        time: u64,
    },
    MessageReaction {
        reaction_id: DbId,
        message_id: DbId,
        source: MessageSource,
        operator_user_id: DbId,
        face_id: String,
        is_add: bool,
        time: u64,
    },
    Poke {
        poke_id: DbId,
        source: MessageSource,
        sender_user_id: DbId,
        target_user_id: DbId,
        time: u64,
    },
    FriendRequestCreated {
        request_id: DbId,
        initiator_user_id: DbId,
        target_user_id: DbId,
        time: u64,
    },
    FriendRequestHandled {
        request_id: DbId,
        initiator_user_id: DbId,
        target_user_id: DbId,
        operator_user_id: DbId,
        state: RequestState,
        time: u64,
    },
    GroupRequestCreated {
        request_id: DbId,
        group_id: DbId,
        request_type: GroupRequestType,
        initiator_user_id: DbId,
        target_user_id: Option<DbId>,
        time: u64,
    },
    GroupRequestHandled {
        request_id: DbId,
        group_id: DbId,
        request_type: GroupRequestType,
        initiator_user_id: DbId,
        target_user_id: Option<DbId>,
        operator_user_id: DbId,
        state: RequestState,
        time: u64,
    },
    GroupMemberMuted {
        group_id: DbId,
        operator_user_id: DbId,
        target_user_id: DbId,
        mute_until: Option<u64>,
        time: u64,
    },
    GroupMemberJoined {
        group_id: DbId,
        operator_user_id: DbId,
        target_user_id: DbId,
        time: u64,
    },
    GroupMemberLeft {
        group_id: DbId,
        operator_user_id: Option<DbId>,
        target_user_id: DbId,
        time: u64,
    },
    GroupMemberTitleUpdated {
        group_id: DbId,
        operator_user_id: DbId,
        target_user_id: DbId,
        time: u64,
    },
    GroupWholeMuteUpdated {
        group_id: DbId,
        operator_user_id: DbId,
        muted: bool,
        mute_until: Option<u64>,
        time: u64,
    },
    GroupAnnouncementUpserted {
        announcement_id: String,
        group_id: DbId,
        sender_user_id: DbId,
        time: u64,
    },
    GroupFolderUpserted {
        folder_id: String,
        group_id: DbId,
        creator_user_id: DbId,
        time: u64,
    },
    GroupFileUpserted {
        file_id: String,
        group_id: DbId,
        uploader_user_id: DbId,
        time: u64,
    },
    GroupEssenceUpdated {
        essence_id: DbId,
        group_id: DbId,
        message_id: DbId,
        sender_user_id: DbId,
        operator_user_id: DbId,
        is_set: bool,
        time: u64,
    },
    GroupAlbumCreated {
        album_id: DbId,
        group_id: DbId,
        name: String,
        time: u64,
    },
    GroupAlbumDeleted {
        album_id: DbId,
        group_id: DbId,
        time: u64,
    },
    GroupPhotoUploaded {
        photo_id: DbId,
        album_id: DbId,
        group_id: DbId,
        time: u64,
    },
    Notice {
        group_id: DbId,
        actor: DbId,
        target: DbId,
        notice_type: NoticeType,
        time: u64,
    },
}
