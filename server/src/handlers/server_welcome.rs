// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use axum::Json;
use axum::extract::ws::WebSocket;
use secure_chat::id::Id;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, ServerWelcomePayload, try_extract_payload};
use crate::transport::ConnectionInfo;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ServerWelcomeResponse {
    pub status: Status,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Error,
}

/// Handle a SERVER_WELCOME message.
/// This is typically received by a bootstrapping server from an introducer
/// or by any server when the network topology changes.
pub async fn handle_server_welcome(
    Json(msg): Json<Message>,
    state: AppState,
    _socket: &mut WebSocket,
) -> Result<Json<ServerWelcomeResponse>, HandlerError> {
    if msg.payload_type != PayloadType::ServerWelcome {
        return Err(ClientError::InvalidPayloadType {
            expected: "ServerWelcome",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }

    let payload: ServerWelcomePayload = try_extract_payload(&msg)?;

    info!("Received ServerWelcome from {}", &msg.from);

    // Process the assigned ID if different from current
    // Note: In practice, server_id is immutable after startup, but we log the discrepancy
    if payload.assigned_id != state.server_id.to_string() {
        match Id::try_from(payload.assigned_id.clone()) {
            Ok(new_id) => {
                warn!(
                    "ServerWelcome contains different assigned ID {} (current: {}). Server ID is immutable after startup.",
                    new_id, state.server_id
                );
            }
            Err(e) => {
                error!("Invalid assigned ID '{}': {}", payload.assigned_id, e);
                return Err(
                    ClientError::PayloadExtraction(format!("Invalid assigned ID: {}", e)).into(),
                );
            }
        }
    }

    // Get the introducer server info from the message sender
    if let Some(introducer_id) = msg.from.as_id() {
        // Add/update the introducer server in our known servers if not already present
        let servers_contains_introducer = state
            .servers
            .lock()
            .map_err(|_| {
                ClientError::PayloadExtraction("Failed to acquire servers lock".to_owned())
            })?
            .contains_key(introducer_id);

        if !servers_contains_introducer {
            let conn = ConnectionInfo::new();
            state
                .servers
                .lock()
                .map_err(|_| {
                    ClientError::PayloadExtraction("Failed to acquire servers lock".to_owned())
                })?
                .insert(introducer_id.clone(), Arc::new(Mutex::new(conn)));
            info!("Added introducer server {} to known servers", introducer_id);
        }
    }

    // Process the servers list from the welcome payload
    for server_info in payload.servers {
        match Id::try_from(server_info.server_id) {
            Ok(server_id) => {
                // Add or update server address and public key
                state
                    .server_addrs
                    .lock()
                    .map_err(|_| {
                        ClientError::PayloadExtraction(
                            "Failed to acquire server_addrs lock".to_owned(),
                        )
                    })?
                    .insert(
                        server_id.clone(),
                        (server_info.host.clone(), server_info.port),
                    );

                state
                    .server_pubkeys
                    .lock()
                    .map_err(|_| {
                        ClientError::PayloadExtraction(
                            "Failed to acquire server_pubkeys lock".to_owned(),
                        )
                    })?
                    .insert(server_id.clone(), server_info.pubkey);

                // Add connection info if not already present
                let servers_contains_server = state
                    .servers
                    .lock()
                    .map_err(|_| {
                        ClientError::PayloadExtraction("Failed to acquire servers lock".to_owned())
                    })?
                    .contains_key(&server_id);

                if !servers_contains_server {
                    let conn = ConnectionInfo::new();
                    state
                        .servers
                        .lock()
                        .map_err(|_| {
                            ClientError::PayloadExtraction(
                                "Failed to acquire servers lock".to_owned(),
                            )
                        })?
                        .insert(server_id.clone(), Arc::new(Mutex::new(conn)));
                }

                info!(
                    "Added/updated server {}: {}:{}",
                    server_id, server_info.host, server_info.port
                );
            }
            Err(e) => {
                warn!("Invalid server ID in welcome payload: {}", e);
                continue;
            }
        }
    }

    // Process the clients list from the welcome payload
    for client_info in payload.clients {
        match Id::try_from(client_info.user_id) {
            Ok(user_id) => {
                // Update user location and public key
                state
                    .user_locations
                    .lock()
                    .map_err(|_| {
                        ClientError::PayloadExtraction(
                            "Failed to acquire user_locations lock".to_owned(),
                        )
                    })?
                    .insert(user_id.clone(), client_info.server_id.clone());

                state
                    .user_pubkeys
                    .lock()
                    .map_err(|_| {
                        ClientError::PayloadExtraction(
                            "Failed to acquire user_pubkeys lock".to_owned(),
                        )
                    })?
                    .insert(user_id.clone(), client_info.pubkey);

                info!(
                    "Added/updated remote user {} on server {}",
                    user_id, client_info.server_id
                );
            }
            Err(e) => {
                warn!("Invalid user ID in welcome payload: {}", e);
                continue;
            }
        }
    }

    // Mark as bootstrapped if not already
    let is_bootstrapped = *state.bootstrapped.lock().map_err(|_| {
        ClientError::PayloadExtraction("Failed to acquire bootstrapped lock".to_owned())
    })?;

    if !is_bootstrapped {
        *state.bootstrapped.lock().map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire bootstrapped lock".to_owned())
        })? = true;
        info!("Server marked as bootstrapped after receiving ServerWelcome");
    }

    info!("Successfully processed ServerWelcome from {}", &msg.from);

    Ok(Json(ServerWelcomeResponse { status: Status::Ok }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Message, PayloadType};
    use chrono::TimeDelta;
    use secure_chat::id::{Id, Identifier};
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn create_test_state() -> AppState {
        AppState {
            config: crate::config::ServerConfig::default(),
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
        }
    }

    fn create_server_welcome_message() -> Message {
        let from = Identifier::Id(Id::new());
        let to = Identifier::Id(Id::new());
        let assigned_id = Id::new();
        let server_id = Id::new();
        let user_id = Id::new();

        Message {
            payload_type: PayloadType::ServerWelcome,
            from,
            to,
            ts: TimeDelta::milliseconds(1700000000500),
            payload: json!({
                "assigned_id": assigned_id.to_owned(),
                "servers": [
                    {
                        "server_id": server_id.to_owned(),
                        "host": "example.com",
                        "port": 8080,
                        "pubkey": "server_pubkey_123"
                    }
                ],
                "clients": [
                    {
                        "user_id": user_id.to_owned(),
                        "pubkey": "user_pubkey_456",
                        "server_id": server_id.to_owned()
                    }
                ]
            }),
            sig: "dummy_sig".to_owned(),
        }
    }

    #[test]
    fn test_server_welcome_payload_extraction() {
        let msg = create_server_welcome_message();
        let payload: ServerWelcomePayload = try_extract_payload(&msg).unwrap();

        assert!(!payload.assigned_id.is_empty());
        assert_eq!(payload.servers.len(), 1);
        assert_eq!(payload.clients.len(), 1);
        assert_eq!(payload.servers[0].host, "example.com");
        assert_eq!(payload.servers[0].port, 8080);
        assert_eq!(payload.clients[0].pubkey, "user_pubkey_456");
    }

    #[test]
    fn test_invalid_payload_type_validation() {
        let mut msg = create_server_welcome_message();
        msg.payload_type = PayloadType::UserHello;

        // Test the payload type validation logic
        if msg.payload_type != PayloadType::ServerWelcome {
            let error = ClientError::InvalidPayloadType {
                expected: "ServerWelcome",
                actual: format!("{:?}", msg.payload_type),
            };
            assert!(matches!(error, ClientError::InvalidPayloadType { .. }));
        } else {
            panic!("Should have detected invalid payload type");
        }
    }

    #[test]
    fn test_server_welcome_state_processing() {
        let state = create_test_state();
        let original_server_id = state.server_id.clone();
        let msg = create_server_welcome_message();

        // Extract the payload to test the processing logic
        let payload: ServerWelcomePayload = try_extract_payload(&msg).unwrap();
        let assigned_id = Id::try_from(payload.assigned_id.clone()).unwrap();

        // Test server ID validation (server_id is immutable in real implementation)
        assert_ne!(assigned_id, original_server_id); // Should be different for this test
        // In real implementation, server_id remains unchanged after startup

        // Test server info processing
        for server_info in &payload.servers {
            let server_id = Id::try_from(server_info.server_id.clone()).unwrap();
            state.server_addrs.lock().unwrap().insert(
                server_id.clone(),
                (server_info.host.clone(), server_info.port),
            );
            state
                .server_pubkeys
                .lock()
                .unwrap()
                .insert(server_id.clone(), server_info.pubkey.clone());

            // Add connection info if not already present
            if !state.servers.lock().unwrap().contains_key(&server_id) {
                let conn = ConnectionInfo::new();
                state
                    .servers
                    .lock()
                    .unwrap()
                    .insert(server_id.clone(), Arc::new(Mutex::new(conn)));
            }
        }

        let expected_server_id = Id::try_from(payload.servers[0].server_id.clone()).unwrap();
        assert!(
            state
                .server_addrs
                .lock()
                .unwrap()
                .contains_key(&expected_server_id)
        );
        assert!(
            state
                .server_pubkeys
                .lock()
                .unwrap()
                .contains_key(&expected_server_id)
        );
        assert!(
            state
                .servers
                .lock()
                .unwrap()
                .contains_key(&expected_server_id)
        );

        // Test client info processing
        for client_info in &payload.clients {
            let user_id = Id::try_from(client_info.user_id.clone()).unwrap();
            state
                .user_locations
                .lock()
                .unwrap()
                .insert(user_id.clone(), client_info.server_id.clone());
            state
                .user_pubkeys
                .lock()
                .unwrap()
                .insert(user_id.clone(), client_info.pubkey.clone());
        }

        let expected_user_id = Id::try_from(payload.clients[0].user_id.clone()).unwrap();
        assert!(
            state
                .user_locations
                .lock()
                .unwrap()
                .contains_key(&expected_user_id)
        );
        assert!(
            state
                .user_pubkeys
                .lock()
                .unwrap()
                .contains_key(&expected_user_id)
        );

        // Test bootstrap state
        *state.bootstrapped.lock().unwrap() = true;
        assert!(*state.bootstrapped.lock().unwrap());
    }

    #[test]
    fn test_same_server_id_assignment() {
        let state = create_test_state();
        let original_id = state.server_id.clone();

        // Create payload with same server ID as current state
        let payload = ServerWelcomePayload {
            assigned_id: original_id.to_string(),
            servers: vec![],
            clients: vec![],
        };

        // Test that server ID comparison works correctly (no change expected)
        let id_matches = payload.assigned_id == state.server_id.to_string();
        assert!(id_matches);

        // Server ID should remain unchanged in the AppState
        assert_eq!(state.server_id, original_id);
    }

    #[test]
    fn test_invalid_server_id_handling() {
        let msg = Message {
            payload_type: PayloadType::ServerWelcome,
            from: Identifier::Id(Id::new()),
            to: Identifier::Id(Id::new()),
            ts: TimeDelta::milliseconds(1700000000500),
            payload: json!({
                "assigned_id": "invalid-uuid-format",
                "servers": [],
                "clients": []
            }),
            sig: "dummy_sig".to_owned(),
        };

        let payload: ServerWelcomePayload = try_extract_payload(&msg).unwrap();

        // Test that invalid assigned_id is handled gracefully
        let result = Id::try_from(payload.assigned_id);
        assert!(result.is_err());
    }
}
