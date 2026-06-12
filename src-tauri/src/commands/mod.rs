pub mod bot;
pub mod chat;
pub mod dev_tools;
pub mod log;
pub mod main;
pub mod packet;

use crate::error::AppResult;

pub(crate) trait IntoCommandResult<T> {
    fn into_command_result(self) -> Result<T, String>;
}

impl<T> IntoCommandResult<T> for AppResult<T> {
    fn into_command_result(self) -> Result<T, String> {
        self.map_err(|err| err.to_string())
    }
}
