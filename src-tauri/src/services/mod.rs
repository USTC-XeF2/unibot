pub mod bot;
pub mod conversation;
pub mod group;
pub mod interaction;
pub mod message;
pub mod packet;
pub mod request;
pub mod settings;
pub mod user;

pub use bot::{BotService, StatsResult};
pub use conversation::ConversationService;
pub use group::{GroupService, MuteGroupMemberResult, UploadGroupFileInput, UploadGroupPhotoInput};
pub use interaction::InteractionService;
pub use message::{MessageService, SendMessageResult};
pub use packet::PacketService;
pub use request::RequestService;
pub use settings::SettingsService;
pub use user::UserService;

#[derive(Clone)]
pub struct ServiceHub {
    pub message: MessageService,
    pub interaction: InteractionService,
    pub group: GroupService,
    pub request: RequestService,
    pub user: UserService,
    pub bot: BotService,
    pub settings: SettingsService,
    pub conversation: ConversationService,
    pub packet: PacketService,
}
