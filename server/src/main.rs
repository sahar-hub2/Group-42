// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use secure_chat::crypto::RsaUtil;
use secure_chat::id::Id;
use server::AppState;
use server::bootstrap::bootstrap_process;
use server::config::ServerConfig;
use server::constants::{SERVER_HOST, SERVER_PORT};
use server::transport::ws_handler;
use tracing::{error, info};

use crate::heartbeat::spawn_heartbeat_cleanup;
use crate::log::LogLevel;

mod heartbeat;
mod log;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = log::new(LogLevel::Info);

    // Load config from custom path if specified
    let config_path = env::var("CONFIG_FILE").ok();
    let config = if let Some(path) = config_path {
        ServerConfig::from_file(path)?
    } else {
        ServerConfig::load()?
    };

    // Generate crypto and ID - use private key file if specified
    let server_crypto = if let Ok(private_key_path) = env::var("PRIVATE_KEY_FILE") {
        Arc::new(RsaUtil::new_from_file(private_key_path)?)
    } else {
        Arc::new(RsaUtil::new()?)
    };
    let server_pubkey = server_crypto.pubkey_base64url()?;
    let server_id = Id::new();

    let app_state = Arc::new(Mutex::new(AppState {
        config,
        bootstrapped: Arc::new(Mutex::new(false)),
        server_id,
        server_crypto,
        server_pubkey,
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

    // Spawn heartbeat cleanup task
    spawn_heartbeat_cleanup(app_state.clone());

    let host = env::var("HOST").unwrap_or_else(|_| SERVER_HOST.to_owned());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(SERVER_PORT);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Invalid HOST or PORT; expected valid IP and u16 port");

    // Handle bootstrap
    if app_state.lock().unwrap().config.skip_bootstrap {
        *app_state.lock().unwrap().bootstrapped.lock().unwrap() = true;
        info!("Skipping bootstrap since skip_bootstrap=true");
        info!(
            "Starter server initialized with ID: {}",
            app_state.lock().unwrap().server_id
        );
    } else {
        let state_clone = app_state.lock().unwrap().clone();
        let host_clone = host.clone();
        tokio::spawn(async move {
            if let Err(e) = bootstrap_process(state_clone, host_clone, port).await {
                error!("Bootstrap failed: {e}");
            }
        });
    }

    info!(
        "Using public key: {}",
        app_state.lock().unwrap().server_pubkey
    );

    // Use app_router to get all API routes with CORS, then add the WebSocket route
    let mut app = server::transport::app_router(app_state.clone());
    app = app.route(
        "/",
        axum::routing::any(ws_handler).with_state(app_state.clone()),
    );

    info!("Server listening on {addr} (HTTP and WS)");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
