// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Public Channel Add message handling

use axum::{Json, extract::State};
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::info;

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, try_extract_payload};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
pub struct PublicChannelAddPayload {
    pub channel_id: String,
    pub name: String,
    pub description: Option<String>,
    pub creator: String,
    pub created_at: TimeDelta,
}

/// # Public Channel Add Handler
/// Handles creation of a new public channel.
pub async fn handle_public_channel_add(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<Message>,
) -> Result<Json<PublicChannelAddPayload>, HandlerError> {
    if msg.payload_type != PayloadType::PublicChannelAdd {
        return Err(ClientError::InvalidPayloadType {
            expected: "PublicChannelAdd",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let payload: PublicChannelAddPayload = try_extract_payload(&msg)?;
    let mut state = state.lock().unwrap();
    // Register the new public channel (single channel for now)
    state.public_channel_id = Some(payload.channel_id.clone());
    state.public_channel_name = Some(payload.name.clone());
    state.public_channel_description = payload.description.clone();
    // Add all known users to the public channel (by default)
    let user_ids: Vec<_> = {
        let local_users = state.local_users.lock().unwrap();
        local_users.keys().cloned().collect()
    };
    for user_id in &user_ids {
        state.public_channel_members.insert(user_id.clone());
    }
    // Always add the creator
    if let Ok(creator_id) = secure_chat::id::Id::from_str(&payload.creator) {
        state.public_channel_members.insert(creator_id);
    }
    // Bump version
    state.public_channel_version += 1;
    info!(
        "PublicChannelAdd registered: channel {} by {} at {} (version {})",
        payload.channel_id, payload.creator, payload.created_at, state.public_channel_version
    );
    Ok(Json(payload))
}

#[cfg(test)]
pub async fn handle_public_channel_add_test(
    Json(msg): Json<Message>,
    _state: &mut crate::AppState,
    _link: crate::transport::ConnectionInfo,
) -> Json<PublicChannelAddPayload> {
    let payload: PublicChannelAddPayload = try_extract_payload(&msg).unwrap();
    Json(payload)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_channel_add_deserialize_works() {
        let json = r#"{
            "type": "PUBLIC_CHANNEL_ADD",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440002",
            "ts": [5678, 0],
            "payload": {
                "channel_id": "channel-uuid-3333",
                "name": "Public Chat",
                "description": "A place for everyone",
                "creator": "550e8400-e29b-41d4-a716-446655440000",
                "created_at": [5678, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::PublicChannelAdd);
        let payload = try_extract_payload::<PublicChannelAddPayload>(&parsed).unwrap();
        assert_eq!(payload.channel_id, "channel-uuid-3333");
        assert_eq!(payload.name, "Public Chat");
        assert_eq!(payload.creator, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn public_channel_add_payload_fields() {
        let payload = PublicChannelAddPayload {
            channel_id: "channel-uuid-3333".to_owned(),
            name: "Public Chat".to_owned(),
            description: Some("A place for everyone".to_owned()),
            creator: "user-uuid-creator-1111".to_owned(),
            created_at: TimeDelta::seconds(5678),
        };
        assert_eq!(payload.channel_id, "channel-uuid-3333");
        assert_eq!(payload.name, "Public Chat");
        assert_eq!(payload.description.as_deref(), Some("A place for everyone"));
        assert_eq!(payload.creator, "user-uuid-creator-1111");
    }
}
