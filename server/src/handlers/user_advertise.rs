// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! User Advertise message handling
//!
//! When a User connects to a Server, that Server announces the User's presence
//! to the entire Network.

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
pub struct UserAdvertisePayload {
    pub user_id: String,
    pub server_id: String,
    pub meta: UserMetadata,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct UserMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronouns: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

/// # User Advertise Handler
/// Handles USER_ADVERTISE messages to update user location mappings and forward to other servers.
pub async fn handle_user_advertise(
    Json(msg): Json<Message>,
    state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<UserAdvertiseResponse>, HandlerError> {
    if msg.payload_type != PayloadType::UserAdvertise {
        return Err(ClientError::InvalidPayloadType {
            expected: "UserAdvertise",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }

    let payload: UserAdvertisePayload = try_extract_payload(&msg)?;

    // Parse user_id as Id
    let user_id = secure_chat::id::Id::try_from(payload.user_id.clone())
        .map_err(|e| ClientError::PayloadExtraction(format!("Invalid user_id: {}", e)))?;

    // Verify signature using the sending server's public key
    // TODO: Implement signature verification with server public keys

    // Update local user location mapping
    state
        .user_locations
        .lock()
        .map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire user_locations lock".to_string())
        })?
        .insert(user_id, payload.server_id.clone());

    info!(
        "Updated user location: {} is now on server {}",
        payload.user_id, payload.server_id
    );

    // TODO: Forward message to other servers (gossip protocol)

    Ok(Json(UserAdvertiseResponse {
        status: "ok".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct UserAdvertiseResponse {
    pub status: String,
}

#[cfg(test)]
pub async fn handle_user_advertise_test(
    Json(msg): Json<Message>,
    _state: &mut crate::AppState,
    _link: crate::transport::ConnectionInfo,
) -> Json<UserAdvertisePayload> {
    let payload: UserAdvertisePayload = try_extract_payload(&msg).unwrap();
    Json(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Message, PayloadType};

    #[test]
    fn user_advertise_deserialize_works() {
        let json = r#"{
            "type": "USER_ADVERTISE",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "*",
            "ts": [5678, 0],
            "payload": {
                "user_id": "550e8400-e29b-41d4-a716-446655440001",
                "server_id": "550e8400-e29b-41d4-a716-446655440000",
                "meta": {
                    "display_name": "Alice",
                    "pronouns": "she/her",
                    "age": 37
                }
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::UserAdvertise);
        let payload = try_extract_payload::<UserAdvertisePayload>(&parsed).unwrap();
        assert_eq!(payload.user_id, "550e8400-e29b-41d4-a716-446655440001");
        assert_eq!(payload.server_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(payload.meta.display_name, Some("Alice".to_string()));
        assert_eq!(payload.meta.pronouns, Some("she/her".to_string()));
        assert_eq!(payload.meta.age, Some(37));
    }
}
