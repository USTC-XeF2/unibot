pub mod group;
pub mod interaction;
pub mod message;
pub mod request;
pub mod user;

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
}

impl ServiceHub {
    pub fn new(
        message: MessageService,
        interaction: InteractionService,
        group: GroupService,
        request: RequestService,
        user: UserService,
    ) -> Self {
        Self {
            message,
            interaction,
            group,
            request,
            user,
        }
    }
}
