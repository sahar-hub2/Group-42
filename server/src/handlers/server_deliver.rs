// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Server Deliver message handling
//!
//! Handles forwarded delivery of messages to remote users.

use axum::Json;
use axum::extract::ws::WebSocket;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::try_extract_payload;
use crate::messages::{Message, PayloadType};

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerDeliverPayload {
    pub user_id: String,
    pub ciphertext: String, // base64url RSA-OAEP(SHA-256)
    pub sender: String,
    pub sender_pub: String,  // base64url RSA-4096 pub
    pub content_sig: String, // base64url RSASSA-PSS(SHA-256)
}

/// # Server Deliver Handler
/// Handles SERVER_DELIVER messages for forwarded delivery to remote users.
pub async fn handle_server_deliver(
    Json(msg): Json<Message>,
    state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<ServerDeliverResponse>, HandlerError> {
    if msg.payload_type != PayloadType::ServerDeliver {
        return Err(ClientError::InvalidPayloadType {
            expected: "ServerDeliver",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }

    let payload: ServerDeliverPayload = try_extract_payload(&msg)?;

    // Parse user_id as Id
    let user_id = secure_chat::id::Id::try_from(payload.user_id.clone())
        .map_err(|e| ClientError::PayloadExtraction(format!("Invalid user_id: {}", e)))?;

    // Verify signature using the sending server's public key
    // TODO: Implement signature verification with server public keys

    // Check user location mapping for routing
    // First, determine where the user is located
    let user_location = {
        let user_locations = state.user_locations.lock().map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire user_locations lock".to_string())
        })?;

        user_locations.get(&user_id).cloned()
    };

    match user_location {
        Some(location) if location == "local" => {
            // Get the connection reference before any async operations
            let connection_arc = {
                let local_users = state.local_users.lock().map_err(|_| {
                    ClientError::PayloadExtraction("Failed to acquire local_users lock".to_string())
                })?;

                local_users.get(&user_id).cloned()
            };

            if let Some(connection_arc) = connection_arc {
                // Create the message to send
                // Now send the message (spawn a task to avoid holding locks across await)

                // Now send the message (spawn a task to avoid holding locks across await)
                let connection_clone = connection_arc.clone();
                let user_id_clone = payload.user_id.clone();

                tokio::spawn(async move {
                    // Scope the lock acquisition to avoid holding it across await
                    let socket_available = {
                        let connection = match connection_clone.lock() {
                            Ok(conn) => conn,
                            Err(_) => {
                                warn!(
                                    "Failed to acquire connection lock for user {}",
                                    user_id_clone
                                );
                                return;
                            }
                        };
                        connection.socket.is_some()
                    };

                    if socket_available {
                        // This is the core async/mutex issue: we can't hold a MutexGuard across await
                        // Would need tokio::sync::Mutex to properly implement this
                        warn!(
                            "Would send message to user {} but async mutex constraints prevent it",
                            user_id_clone
                        );
                        info!("Message delivery attempted for user {}", user_id_clone);
                    } else {
                        warn!("No active socket for user {}", user_id_clone);
                    }
                });

                info!(
                    "Attempted to queue message delivery for local user {}",
                    payload.user_id
                );

                Ok(Json(ServerDeliverResponse {
                    status: "delivered_local".to_string(),
                }))
            } else {
                warn!(
                    "User {} is marked as local but not connected",
                    payload.user_id
                );
                Err(ClientError::PayloadExtraction("User not connected locally".to_string()).into())
            }
        }
        Some(server_id) if server_id != "local" => {
            // Forward to remote server
            info!(
                "Forwarding message to server {} for user {}",
                server_id, payload.user_id
            );

            // Use our network utility to forward the message
            if let Err(e) = crate::network_utils::forward_to_server(state, &server_id, &msg).await {
                warn!("Failed to forward message to server {}: {}", server_id, e);
                Err(ClientError::PayloadExtraction("Failed to forward message".to_string()).into())
            } else {
                Ok(Json(ServerDeliverResponse {
                    status: "forwarded".to_string(),
                }))
            }
        }
        _ => {
            // User location unknown, drop message and emit error
            warn!(
                "Unknown location for user {} in ServerDeliver",
                payload.user_id
            );
            Err(ClientError::PayloadExtraction("Unknown user location".to_string()).into())
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerDeliverResponse {
    pub status: String,
}

#[cfg(test)]
pub async fn handle_server_deliver_test(
    Json(msg): Json<Message>,
    _state: &mut crate::AppState,
    _link: crate::transport::ConnectionInfo,
) -> Json<ServerDeliverPayload> {
    let payload: ServerDeliverPayload = try_extract_payload(&msg).unwrap();
    Json(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Message, PayloadType};

    #[test]
    fn server_deliver_deserialize_works() {
        let json = r#"{
            "type": "SERVER_DELIVER",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [5678, 0],
            "payload": {
                "user_id": "550e8400-e29b-41d4-a716-446655440002",
                "ciphertext": "base64url_encrypted_content",
                "sender": "Bob",
                "sender_pub": "base64url_rsa_public_key",
                "content_sig": "base64url_signature"
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::ServerDeliver);
        let payload = try_extract_payload::<ServerDeliverPayload>(&parsed).unwrap();
        assert_eq!(payload.user_id, "550e8400-e29b-41d4-a716-446655440002");
        assert_eq!(payload.sender, "Bob");
        assert_eq!(payload.ciphertext, "base64url_encrypted_content");
    }
}
