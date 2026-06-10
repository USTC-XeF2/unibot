use unibot_lib::core::CoreContainer;
use unibot_lib::models::UserProfile;
use unibot_lib::persistence::{
    BotRepo, GroupRepo, InteractionRepo, MessageRepo, SettingsRepo, UserRepo, migrator,
};
use unibot_lib::protocol::ProtocolRuntimeManager;
use unibot_lib::protocol::types::BotConfig;
use unibot_lib::services::{
    BotService, GroupService, InteractionService, MessageService, RequestService, ServiceHub,
    SettingsService, UserService,
};
use unibot_lib::utils::new_db_id;

async fn setup_test_env(
    pool: sqlx::SqlitePool,
) -> (ProtocolRuntimeManager, String, String, String) {
    migrator::run_migrations(&pool).await.unwrap();

    let user_repo = UserRepo::new(pool.clone());
    let bot_repo = BotRepo::new(pool.clone());
    let message_repo = MessageRepo::new(pool.clone());
    let group_repo = GroupRepo::new(pool.clone());
    let interaction_repo = InteractionRepo::new(pool.clone());

    let core = CoreContainer::new();

    // 注册测试用户
    let user = UserProfile {
        user_id: "10001".to_string(),
        nickname: "Alice".to_string(),
        avatar: "".to_string(),
        signature: "".to_string(),
        account_status: Default::default(),
    };
    user_repo.upsert_user(&user).await.unwrap();
    core.register_user(user).unwrap();

    let service_hub = ServiceHub::new(
        MessageService::new(message_repo.clone(), group_repo.clone()),
        InteractionService::new(interaction_repo, message_repo.clone(), group_repo.clone()),
        GroupService::new(group_repo, message_repo.clone()),
        RequestService::new(user_repo.clone()),
        UserService::new(user_repo.clone()),
        BotService::new(bot_repo.clone()),
        SettingsService::new(SettingsRepo::new(pool.clone())),
    );

    let temp_dir = std::env::temp_dir().join(format!("unibot-test-{}", new_db_id()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();
    let bots_dir = temp_dir.join("bots");
    tokio::fs::create_dir_all(&bots_dir).await.unwrap();

    // 创建 bot 记录
    let bot_id = new_db_id();
    let config_path = bots_dir.join(format!("{bot_id}.json"));
    let access_token = uuid::Uuid::new_v4().to_string();
    let config = BotConfig {
        version: 1,
        protocol: "milky".to_string(),
        http: unibot_lib::protocol::types::HttpConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // 让 OS 分配端口
        },
        access_token: access_token.clone(),
        event_transport: "sse".to_string(),
    };
    let config_json = serde_json::to_string_pretty(&config).unwrap();
    tokio::fs::write(&config_path, config_json).await.unwrap();

    bot_repo
        .insert_bot(&bot_id, "10001", "Test Bot", config_path.to_str().unwrap())
        .await
        .unwrap();

    let runtime = ProtocolRuntimeManager::new(
        bot_repo.clone(),
        service_hub,
        core.clone(),
        temp_dir.clone(),
        pool.clone(),
    );

    (
        runtime,
        bot_id,
        access_token,
        temp_dir.to_str().unwrap().to_string(),
    )
}

#[sqlx::test]
async fn start_bot_exposes_api_endpoints(pool: sqlx::SqlitePool) {
    let (runtime, bot_id, token, _dir) = setup_test_env(pool).await;

    let addr = runtime.start_bot(&bot_id).await.unwrap();
    let client = reqwest::Client::new();

    // 测试 get_login_info
    let resp = client
        .post(format!(
            "http://{addr}/api/get_login_info?access_token={token}"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["data"]["user_id"], 10001);

    runtime.stop_bot(&bot_id).await.unwrap();
}

#[sqlx::test]
async fn wrong_token_returns_401(pool: sqlx::SqlitePool) {
    let (runtime, bot_id, _token, _dir) = setup_test_env(pool).await;
    let addr = runtime.start_bot(&bot_id).await.unwrap();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://{addr}/api/get_login_info?access_token=wrong"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);

    runtime.stop_bot(&bot_id).await.unwrap();
}

#[sqlx::test]
async fn unknown_api_returns_404(pool: sqlx::SqlitePool) {
    let (runtime, bot_id, token, _dir) = setup_test_env(pool).await;
    let addr = runtime.start_bot(&bot_id).await.unwrap();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://{addr}/api/unknown_api?access_token={token}"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200); // axum returns 200 with failed status
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "failed");
    assert_eq!(body["retcode"], -404);

    runtime.stop_bot(&bot_id).await.unwrap();
}

#[sqlx::test]
async fn send_private_message_and_get_friend_list(pool: sqlx::SqlitePool) {
    let (runtime, bot_id, token, _dir) = setup_test_env(pool.clone()).await;
    let addr = runtime.start_bot(&bot_id).await.unwrap();
    let client = reqwest::Client::new();

    // 先创建第二个用户作为好友
    let user_repo = UserRepo::new(pool.clone());
    let user2 = UserProfile {
        user_id: "10002".to_string(),
        nickname: "Bob".to_string(),
        avatar: "".to_string(),
        signature: "".to_string(),
        account_status: Default::default(),
    };
    user_repo.upsert_user(&user2).await.unwrap();

    // 发送私聊消息
    let resp = client
        .post(format!(
            "http://{addr}/api/send_private_message?access_token={token}"
        ))
        .json(&serde_json::json!({
            "user_id": 10001,
            "message": [{"type":"text","data":{"text":"hello"}}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["data"]["message_id"].is_string());

    // get_friend_list（Alice 没有好友，所以是空列表）
    let resp = client
        .post(format!(
            "http://{addr}/api/get_friend_list?access_token={token}"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["data"]["data"].is_array());

    runtime.stop_bot(&bot_id).await.unwrap();
}

#[sqlx::test]
async fn event_endpoint_returns_sse(pool: sqlx::SqlitePool) {
    let (runtime, bot_id, token, _dir) = setup_test_env(pool).await;
    let addr = runtime.start_bot(&bot_id).await.unwrap();

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{addr}/event?access_token={token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/event-stream"));

    // 不要消费 body，直接关闭连接
    drop(resp);

    runtime.stop_bot(&bot_id).await.unwrap();
}
