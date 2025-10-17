// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Test heartbeat functionality
use reqwest::Client;
use secure_chat::id::Id;
use server::AppState;
use server::transport::app_router;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

fn spawn_app() -> String {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("Failed to bind random port");
            let port = listener.local_addr().unwrap().port();
            let addr = format!("http://127.0.0.1:{port}");
            tx.send(addr).unwrap();
            let app_state = Arc::new(Mutex::new(AppState {
                config: server::config::ServerConfig::default(),
                bootstrapped: Arc::new(Mutex::new(false)),
                server_id: Id::new(),
                server_crypto: std::sync::Arc::new(
                    secure_chat::crypto::RsaUtil::new().expect("Failed to create crypto"),
                ),
                server_pubkey: "test_pubkey".to_owned(),
                servers: Arc::new(Mutex::new(HashMap::new())),
                server_addrs: Arc::new(Mutex::new(HashMap::new())),
                server_pubkeys: Arc::new(Mutex::new(HashMap::new())),
                local_users: Arc::new(Mutex::new(HashMap::new())),
                user_locations: Arc::new(Mutex::new(HashMap::new())),
                user_heartbeat: HashMap::new(),
                user_pubkeys: Arc::new(Mutex::new(HashMap::new())),
                pending_messages: HashMap::new(),
                public_channel_members: std::collections::HashSet::new(),
                public_channel_id: None,
                public_channel_name: None,
                public_channel_description: None,
                public_channel_key: None,
                public_channel_version: 0,
                public_channel_messages: std::collections::VecDeque::new(),
                public_channel_file_events: std::collections::VecDeque::new(),
            }));
            let app = app_router(app_state);
            axum::serve(listener, app).await.unwrap();
        });
    });
    rx.recv().unwrap()
}

#[tokio::test]
async fn heartbeat() {
    let addr = spawn_app();
    let client = Client::new();

    // Insert a user to test heartbeat
    let user_id = Id::new();
    let payload = serde_json::json!({
        "user_id": user_id.to_string()
    });

    // First, heartbeat for unknown user (should return "not found")
    let response = client
        .post(format!("{addr}/api/heartbeat"))
        .body(payload.to_string())
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("Failed to send request");
    assert!(response.status().is_success());
    let body = response.text().await.expect("Failed to read response body");
    assert!(body.contains("not found"));
}
