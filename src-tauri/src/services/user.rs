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
        user_id: u64,
        nickname: String,
        avatar: String,
        signature: String,
    ) -> AppResult<UserProfile> {
        let nickname = nickname.trim();
        if nickname.is_empty() {
            return Err(AppError::validation("nickname cannot be empty"));
        }

        let profile = UserProfile {
            user_id,
            nickname: nickname.to_string(),
            avatar: avatar.trim().to_string(),
            signature: signature.trim().to_string(),
        };

        self.repo.upsert_user(&profile).await?;
        if let Err(err) = core.register_user(profile.clone()) {
            let _ = self.repo.delete_user(profile.user_id).await;
            return Err(err);
        }

        Ok(profile)
    }

    pub async fn list_users(&self) -> AppResult<Vec<UserProfile>> {
        self.repo.list_users().await.map_err(Into::into)
    }

    pub async fn update_user_profile(
        &self,
        user_id: u64,
        nickname: Option<String>,
        avatar: Option<String>,
        signature: Option<String>,
    ) -> AppResult<UserProfile> {
        let existing = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("user {} not found", user_id)))?;

        let UserProfile {
            user_id,
            nickname: existing_nickname,
            avatar: existing_avatar,
            signature: existing_signature,
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
        };

        self.repo.upsert_user(&profile).await?;
        Ok(profile)
    }

    pub async fn delete_user(&self, core: &CoreContainer, user_id: u64) -> AppResult<()> {
        let deleted = self.repo.delete_user(user_id).await?;
        if !deleted {
            return Err(AppError::not_found(format!("user {} not found", user_id)));
        }

        core.unregister_user(user_id);
        Ok(())
    }

    pub async fn list_friends(&self, user_id: u64) -> AppResult<Vec<u64>> {
        let rows = self.repo.list_friends(user_id).await?;
        Ok(rows.into_iter().map(|row| row.friend_user_id).collect())
    }

    pub async fn get_user_by_id(&self, user_id: u64) -> AppResult<Option<UserProfile>> {
        self.repo.get_user_by_id(user_id).await.map_err(Into::into)
    }
}
