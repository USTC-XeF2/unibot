use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::UserProfile;
use crate::persistence::UserRepo;

#[derive(Clone)]
pub struct UserService {
    repo: UserRepo,
}

impl UserService {
    pub fn new(repo: UserRepo) -> Self {
        Self { repo }
    }

    pub async fn register_user(
        &self,
        core: &CoreContainer,
        user_id: String,
        nickname: String,
        avatar: String,
        signature: String,
    ) -> AppResult<UserProfile> {
        let nickname = nickname.trim();
        if nickname.is_empty() {
            return Err(AppError::validation("nickname cannot be empty"));
        }

        let profile = UserProfile {
            user_id: user_id.clone(),
            nickname: nickname.to_string(),
            avatar: avatar.trim().to_string(),
            signature: signature.trim().to_string(),
            account_status: Default::default(),
        };

        if core.user_context(&user_id).is_some() {
            return Err(AppError::conflict(format!(
                "user {} is already registered",
                user_id
            )));
        }

        self.repo.upsert_user(&profile).await?;
        core.register_user(profile.clone())?;

        Ok(profile)
    }

    pub async fn list_users(&self) -> AppResult<Vec<UserProfile>> {
        self.repo.list_users().await.map_err(Into::into)
    }

    pub async fn update_user_profile(
        &self,
        user_id: String,
        nickname: Option<String>,
        avatar: Option<String>,
        signature: Option<String>,
    ) -> AppResult<UserProfile> {
        let existing = self
            .repo
            .get_user_by_id(&user_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("user {} not found", user_id)))?;

        let UserProfile {
            user_id,
            nickname: existing_nickname,
            avatar: existing_avatar,
            signature: existing_signature,
            ..
        } = existing;

        let nickname = match nickname {
            Some(next) => {
                let trimmed = next.trim();
                if trimmed.is_empty() {
                    return Err(AppError::validation("nickname cannot be empty"));
                }
                trimmed.to_string()
            }
            None => existing_nickname,
        };

        let avatar = match avatar {
            Some(next) => next.trim().to_string(),
            None => existing_avatar,
        };

        let signature = match signature {
            Some(next) => next.trim().to_string(),
            None => existing_signature,
        };

        let profile = UserProfile {
            user_id,
            nickname,
            avatar,
            signature,
            account_status: Default::default(),
        };

        self.repo.upsert_user(&profile).await?;
        Ok(profile)
    }

    pub async fn delete_user(&self, core: &CoreContainer, user_id: String) -> AppResult<()> {
        let deleted = self.repo.delete_user(&user_id).await?;
        if !deleted {
            return Err(AppError::not_found(format!("user {} not found", user_id)));
        }

        core.unregister_user(&user_id);
        Ok(())
    }

    pub async fn list_friends(&self, user_id: String) -> AppResult<Vec<String>> {
        let rows = self.repo.list_friends(&user_id).await?;
        Ok(rows.into_iter().map(|row| row.friend_user_id).collect())
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> AppResult<Option<UserProfile>> {
        self.repo.get_user_by_id(user_id).await.map_err(Into::into)
    }
}
