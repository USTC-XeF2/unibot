pub mod chat;
pub mod main;

use crate::error::AppResult;

pub(crate) trait IntoCommandResult<T> {
    fn into_command_result(self) -> Result<T, String>;
}

impl<T> IntoCommandResult<T> for AppResult<T> {
    fn into_command_result(self) -> Result<T, String> {
        self.map_err(|err| err.to_string())
    }
}
