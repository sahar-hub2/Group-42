// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! User Remove message handling
//!
//! When a User disconnects, the Server that they are on announces removal.

use axum::Json;
use axum::extract::ws::WebSocket;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::try_extract_payload;
use crate::messages::{Message, PayloadType};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserRemovePayload {
    pub user_id: String,
    pub server_id: String,
}

/// # User Remove Handler
/// Handles USER_REMOVE messages to remove user location mappings and forward to other servers.
pub async fn handle_user_remove(
    Json(msg): Json<Message>,
    state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<UserRemoveResponse>, HandlerError> {
    if msg.payload_type != PayloadType::UserRemove {
        return Err(ClientError::InvalidPayloadType {
            expected: "UserRemove",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }

    let payload: UserRemovePayload = try_extract_payload(&msg)?;

    // Parse user_id as Id
    let user_id = secure_chat::id::Id::try_from(payload.user_id.clone())
        .map_err(|e| ClientError::PayloadExtraction(format!("Invalid user_id: {}", e)))?;

    // Verify signature using the sending server's public key
    // TODO: Implement signature verification with server public keys

    // Only remove the User if the local mapping still points to that Server
    let mut user_locations = state.user_locations.lock().map_err(|_| {
        ClientError::PayloadExtraction("Failed to acquire user_locations lock".to_string())
    })?;

    if let Some(current_server_id) = user_locations.get(&user_id) {
        if current_server_id == &payload.server_id {
            user_locations.remove(&user_id);
            info!(
                "Removed user {} from server {} mapping",
                payload.user_id, payload.server_id
            );
        } else {
            info!(
                "Ignoring user removal for {}: currently mapped to {} not {}",
                payload.user_id, current_server_id, payload.server_id
            );
        }
    } else {
        info!(
            "User {} not found in location mapping, ignoring removal",
            payload.user_id
        );
    }

    // TODO: Forward message to other servers (gossip protocol)

    Ok(Json(UserRemoveResponse {
        status: "ok".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct UserRemoveResponse {
    pub status: String,
}

#[cfg(test)]
pub async fn handle_user_remove_test(
    Json(msg): Json<Message>,
    _state: &mut crate::AppState,
    _link: crate::transport::ConnectionInfo,
) -> Json<UserRemovePayload> {
    let payload: UserRemovePayload = try_extract_payload(&msg).unwrap();
    Json(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Message, PayloadType};

    #[test]
    fn user_remove_deserialize_works() {
        let json = r#"{
            "type": "USER_REMOVE",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "*",
            "ts": [5678, 0],
            "payload": {
                "user_id": "550e8400-e29b-41d4-a716-446655440001",
                "server_id": "550e8400-e29b-41d4-a716-446655440000"
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::UserRemove);
        let payload = try_extract_payload::<UserRemovePayload>(&parsed).unwrap();
        assert_eq!(payload.user_id, "550e8400-e29b-41d4-a716-446655440001");
        assert_eq!(payload.server_id, "550e8400-e29b-41d4-a716-446655440000");
    }
}
