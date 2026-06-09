pub mod bot;
pub mod group;
pub mod interaction;
pub mod message;
pub mod request;
pub mod user;

pub use bot::{BotService, StatsResult};
pub use group::{GroupService, MuteGroupMemberResult};
pub use interaction::InteractionService;
pub use message::{MessageService, SendMessageResult};
pub use request::RequestService;
pub use user::UserService;

#[derive(Clone)]
pub struct ServiceHub {
    pub message: MessageService,
    pub interaction: InteractionService,
    pub group: GroupService,
    pub request: RequestService,
    pub user: UserService,
    pub bot: BotService,
}

impl ServiceHub {
    pub fn new(
        message: MessageService,
        interaction: InteractionService,
        group: GroupService,
        request: RequestService,
        user: UserService,
        bot: BotService,
    ) -> Self {
        Self {
            message,
            interaction,
            group,
            request,
            user,
            bot,
        }
    }
}
