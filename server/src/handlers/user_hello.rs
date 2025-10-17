// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! User Hello message handling

use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::Json;
use axum::extract::State;
use axum::extract::ws::WebSocket;
use secure_chat::id::Id;
use secure_chat::id::Identifier;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::handlers::user_advertise::UserMetadata;
use crate::messages::try_extract_payload;
use crate::messages::{Message, PayloadType};
use crate::network_utils::advertise_user_to_network;
use crate::transport::ConnectionInfo;

#[derive(Debug, Deserialize)]
pub struct UserHelloHttpPayload {
    pub user_id: String,
    pub client: String,
    pub pubkey: String,
    pub enc_pubkey: String,
    pub meta: Option<MetaField>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatPayload {
    pub user_id: String,
}

/// HTTP handler for client heartbeat
pub async fn heartbeat_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<HeartbeatPayload>,
) -> Json<&'static str> {
    let id = match Id::from_str(&payload.user_id) {
        Ok(id) => id,
        Err(_) => return Json("invalid id"),
    };
    let user_exists = {
        let state = state.lock().unwrap();
        let local_users = state.local_users.lock().unwrap();
        local_users.contains_key(&id)
    };
    if user_exists {
        let mut state = state.lock().unwrap();
        state.user_heartbeat.insert(id.clone(), Instant::now());
        info!(user_id = %payload.user_id, "Received heartbeat");
        Json("ok")
    } else {
        info!(user_id = %payload.user_id, "Heartbeat for unknown user");
        Json("not found")
    }
}

#[derive(Debug, Deserialize)]
pub struct MetaField {
    pub display_name: Option<String>,
}

/// HTTP handler for user hello (presence announcement)
pub async fn user_hello_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<UserHelloHttpPayload>,
) -> Json<&'static str> {
    info!(?payload, "Received UserHello HTTP message");
    let id = match Id::from_str(&payload.user_id) {
        Ok(id) => id,
        Err(_) => return Json("invalid id"),
    };
    let mut state = state.lock().unwrap();
    let mut conn = ConnectionInfo::default();
    if !payload.pubkey.is_empty() {
        conn.pubkey = Some(payload.pubkey.clone());
    }
    state
        .local_users
        .lock()
        .unwrap()
        .insert(id.clone(), Arc::new(Mutex::new(conn)));
    state.user_heartbeat.insert(id.clone(), Instant::now());
    let display_name = payload.meta.as_ref().and_then(|m| m.display_name.clone());
    if let Some(name) = display_name {
        state.user_locations.lock().unwrap().insert(id, name);
    } else {
        state
            .user_locations
            .lock()
            .unwrap()
            .insert(id, payload.user_id.clone());
    }
    info!("User {} added to local_users", payload.user_id);
    Json("ok")
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserHelloPayload {
    pub client: String,
    pub pubkey: String,
    pub enc_pubkey: String,
}

/// # User Hello
/// Announces a user's presence to the local server.
pub async fn handle_user_hello(
    Json(msg): Json<Message>,
    state: &mut AppState,
    link: WebSocket,
) -> Result<Json<UserHelloPayload>, HandlerError> {
    if msg.payload_type != PayloadType::UserHello {
        return Err(ClientError::InvalidPayloadType {
            expected: "UserHello",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }

    let payload: UserHelloPayload = try_extract_payload(&msg)?;

    // Extract Id from Identifier::Id
    let user_id = match &msg.from {
        Identifier::Id(id) => id.clone(),
        _ => {
            return Err(ClientError::PayloadExtraction(
                "Expected Id identifier in UserHello message".to_owned(),
            )
            .into());
        }
    };

    // Add user to local_users table
    let conn = ConnectionInfo::with_socket(link);
    state
        .local_users
        .lock()
        .map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire local_users lock".to_owned())
        })?
        .insert(user_id.clone(), Arc::new(Mutex::new(conn)));

    // Update user location to mark as local
    state
        .user_locations
        .lock()
        .map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire user_locations lock".to_owned())
        })?
        .insert(user_id.clone(), "local".to_string());

    info!("UserHello received and user added: {:?}", msg.from);

    // Extract user metadata from payload if available
    let user_meta = if let Some(meta_value) = msg.payload.get("meta") {
        // Try to parse metadata from the payload
        match serde_json::from_value::<UserMetadata>(meta_value.clone()) {
            Ok(meta) => {
                info!("Extracted user metadata for {}: {:?}", user_id, meta);
                meta
            }
            Err(e) => {
                warn!("Failed to parse user metadata for {}: {}", user_id, e);
                UserMetadata::default()
            }
        }
    } else {
        // No metadata provided, use defaults
        info!("No metadata provided for user {}, using defaults", user_id);
        UserMetadata::default()
    };

    // Advertise user presence to other servers
    if let Err(e) = advertise_user_to_network(state, &user_id, user_meta).await {
        warn!("Failed to advertise user to network: {}", e);
    }

    Ok(Json(payload))
}

#[cfg(test)]
pub async fn handle_user_hello_test(
    Json(msg): Json<Message>,
    state: &mut crate::AppState,
    link: crate::transport::ConnectionInfo,
) -> Json<UserHelloPayload> {
    // Extract Id from Identifier::Id
    let user_id = match &msg.from {
        Identifier::Id(id) => id.clone(),
        _ => panic!("Expected Id identifier in UserHello message"),
    };

    state
        .local_users
        .lock()
        .unwrap()
        .insert(user_id, Arc::new(Mutex::new(link)));
    let payload: UserHelloPayload = serde_json::from_value(msg.payload).unwrap();
    Json(payload)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::messages::{Message, PayloadType};
    use axum::Json;
    use chrono::TimeDelta;
    use secure_chat::id::{Id, Identifier};
    use serde_json::json;

    use crate::AppState;
    use crate::transport::ConnectionInfo;

    /// Create a sample UserHello message
    fn sample_user_hello() -> Message {
        let from = Identifier::Id(Id::new());
        let to = Identifier::Id(Id::new());
        Message {
            payload_type: PayloadType::UserHello,
            from,
            to,
            ts: TimeDelta::seconds(3600),
            payload: json!({
                "client": "cli-v1",
                "pubkey": "test_pubkey",
                "enc_pubkey": "test_enc_pubkey"
            }),
            sig: String::new(),
        }
    }

    #[tokio::test]
    async fn user_is_added_to_local_users_on_hello() {
        let msg = sample_user_hello();
        let user_id = match &msg.from {
            Identifier::Id(id) => id.clone(),
            _ => panic!("Expected Id identifier"),
        };

        let mut state = AppState {
            config: crate::config::ServerConfig::default(),
            bootstrapped: std::sync::Arc::new(std::sync::Mutex::new(false)),
            server_id: secure_chat::id::Id::new(),
            server_crypto: std::sync::Arc::new(
                secure_chat::crypto::RsaUtil::new().expect("Failed to create crypto"),
            ),
            server_pubkey: "test_pubkey".to_owned(),
            servers: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            server_addrs: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            server_pubkeys: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            local_users: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            user_locations: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            user_heartbeat: HashMap::new(),
            user_pubkeys: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            pending_messages: HashMap::new(),
            public_channel_members: std::collections::HashSet::new(),
            public_channel_id: None,
            public_channel_name: None,
            public_channel_description: None,
            public_channel_key: None,
            public_channel_version: 0,
            public_channel_messages: std::collections::VecDeque::new(),
            public_channel_file_events: std::collections::VecDeque::new(),
        };

        let dummy_link = ConnectionInfo::default();
        let response = super::handle_user_hello_test(Json(msg), &mut state, dummy_link).await;
        let body_str = serde_json::to_string(&response.0).expect("Failed to serialize response");
        assert!(body_str.contains("\"client\":"));
        assert!(state.local_users.lock().unwrap().contains_key(&user_id));
    }

    #[test]
    fn user_hello_missing_field() {
        let json = r#"{
            "type": "USER_HELLO",
            "from": "00000000-0000-0000-0000-000000000001",
            "to": "00000000-0000-0000-0000-000000000002",
            "ts": [3600, 0],
            "payload": {
                "client": "cli-v1",
                "pubkey": "test_pubkey"
            },
            "sig": ""
        }"#;
        let parsed = serde_json::from_str::<Message>(json).expect("Failed to parse Message");
        let p: Result<UserHelloPayload, _> = serde_json::from_value(parsed.payload);
        assert!(p.is_err());
    }

    #[test]
    fn user_hello_edge_empty_client() {
        let payload = UserHelloPayload {
            client: "".to_owned(),
            pubkey: "pubkey123".to_owned(),
            enc_pubkey: "enc123".to_owned(),
        };
        assert_eq!(payload.client, "");
    }

    #[test]
    fn user_hello_edge_empty_pubkey() {
        let payload = UserHelloPayload {
            client: "cli-v1".to_owned(),
            pubkey: "".to_owned(),
            enc_pubkey: "enc123".to_owned(),
        };
        assert_eq!(payload.pubkey, "");
    }

    #[test]
    fn user_hello_edge_empty_enc_pubkey() {
        let payload = UserHelloPayload {
            client: "cli-v1".to_owned(),
            pubkey: "pubkey123".to_owned(),
            enc_pubkey: "".to_owned(),
        };
        assert_eq!(payload.enc_pubkey, "");
    }

    #[test]
    fn user_hello_deserialize_works() {
        let json = r#"{
            "type": "USER_HELLO",
            "from": "00000000-0000-0000-0000-000000000001",
            "to": "00000000-0000-0000-0000-000000000002",
            "ts": [3600, 0],
            "payload": {
                "client": "cli-v1",
                "pubkey": "test_pubkey",
                "enc_pubkey": "test_enc_pubkey"
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::UserHello);
        assert_eq!(parsed.from.as_str(), "00000000-0000-0000-0000-000000000001");
        let payload: UserHelloPayload =
            serde_json::from_value(parsed.payload).expect("payload parse");
        assert_eq!(payload.client, "cli-v1");
    }

    #[test]
    fn user_hello_payload_fields() {
        let payload = UserHelloPayload {
            client: "cli-v1".to_owned(),
            pubkey: "pubkey123".to_owned(),
            enc_pubkey: "enc123".to_owned(),
        };
        assert_eq!(payload.client, "cli-v1");
        assert_eq!(payload.pubkey, "pubkey123");
        assert_eq!(payload.enc_pubkey, "enc123");
    }
}
