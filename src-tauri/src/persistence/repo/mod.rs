pub mod bot;
pub(crate) mod codecs;
pub mod conversation;
pub mod group;
pub mod interaction;
pub mod message;
pub mod packet;
pub mod settings;
pub mod user;

#[cfg(test)]
mod tests;

pub use bot::BotRepo;
pub use conversation::ConversationRepo;
pub use group::{GroupEventRecord, GroupRepo, NewGroupEventRecord, NewGroupRequestRecord};
pub use interaction::{InteractionRepo, NewMessageReactionRecord, NewPokeRecord};
pub use message::{MessageRecord, MessageRepo, NewMessageRecord};
pub use packet::{PacketRepo, ProtocolPacketRecord};
pub use settings::{SettingRecord, SettingsRepo};
pub use user::{NewFriendRequestRecord, UserRepo};
