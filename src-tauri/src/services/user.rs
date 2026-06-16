use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{FriendCategoryEntity, FriendshipEntity, UserProfile};
use crate::persistence::UserRepo;

#[derive(Clone)]
pub struct UserService {
    repo: UserRepo,
}

fn normalize_category_name(name: String) -> AppResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("category name cannot be empty"));
    }
    Ok(trimmed.to_string())
}

fn friend_category_name_exists(
    categories: &[FriendCategoryEntity],
    name: &str,
    except_category_id: Option<&str>,
) -> bool {
    categories.iter().any(|category| {
        category.name == name && Some(category.category_id.as_str()) != except_category_id
    })
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

        tracing::info!(
            target: "user_service",
            user_id = %profile.user_id,
            nickname = %profile.nickname,
            "user registered"
        );
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
            account_status: existing_account_status,
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
            account_status: existing_account_status,
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
        tracing::info!(target: "user_service", user_id = %user_id, "user deleted");
        Ok(())
    }

    pub async fn list_friends(&self, user_id: String) -> AppResult<Vec<String>> {
        let rows = self.repo.list_friends(&user_id).await?;
        Ok(rows.into_iter().map(|row| row.friend_user_id).collect())
    }

    pub async fn list_friendships(&self, user_id: String) -> AppResult<Vec<FriendshipEntity>> {
        self.repo
            .list_friendships(&user_id)
            .await
            .map_err(Into::into)
    }

    pub async fn list_friend_categories(
        &self,
        user_id: String,
    ) -> AppResult<Vec<FriendCategoryEntity>> {
        self.repo
            .list_friend_categories(&user_id)
            .await
            .map_err(Into::into)
    }

    pub async fn create_friend_category(
        &self,
        user_id: String,
        name: String,
    ) -> AppResult<FriendCategoryEntity> {
        let name = normalize_category_name(name)?;

        let categories = self.repo.list_friend_categories(&user_id).await?;
        if friend_category_name_exists(&categories, &name, None) {
            return Err(AppError::conflict("category name already exists"));
        }

        self.repo
            .create_friend_category(&user_id, &name)
            .await
            .map_err(Into::into)
    }

    pub async fn rename_friend_category(
        &self,
        user_id: String,
        category_id: String,
        name: String,
    ) -> AppResult<FriendCategoryEntity> {
        let name = normalize_category_name(name)?;

        let category = self
            .repo
            .get_friend_category_by_id(&category_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("category {} not found", category_id)))?;

        if category.owner_user_id != user_id {
            return Err(AppError::validation(
                "cannot rename another user's category",
            ));
        }

        let categories = self.repo.list_friend_categories(&user_id).await?;
        if friend_category_name_exists(&categories, &name, Some(&category_id)) {
            return Err(AppError::conflict("category name already exists"));
        }

        self.repo
            .rename_friend_category(&user_id, &category_id, &name)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_friend_category(
        &self,
        user_id: String,
        category_id: String,
    ) -> AppResult<()> {
        let category = self
            .repo
            .get_friend_category_by_id(&category_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("category {} not found", category_id)))?;

        if category.owner_user_id != user_id {
            return Err(AppError::validation(
                "cannot delete another user's category",
            ));
        }

        self.repo
            .delete_friend_category(&category_id)
            .await
            .map_err(Into::into)
    }

    pub async fn set_friend_category(
        &self,
        user_id: String,
        friend_user_id: String,
        category_id: String,
    ) -> AppResult<()> {
        let category = self
            .repo
            .get_friend_category_by_id(&category_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("category {} not found", category_id)))?;

        if category.owner_user_id != user_id {
            return Err(AppError::validation(
                "cannot assign friend to another user's category",
            ));
        }

        let updated = self
            .repo
            .set_friend_category(&user_id, &friend_user_id, &category_id)
            .await?;
        if !updated {
            return Err(AppError::not_found(format!(
                "friendship {} -> {} not found",
                user_id, friend_user_id
            )));
        }

        Ok(())
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> AppResult<Option<UserProfile>> {
        self.repo.get_user_by_id(user_id).await.map_err(Into::into)
    }
}
