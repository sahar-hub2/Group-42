// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Public Channel Updated message handling

use axum::{Json, extract::State};
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::info;

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, try_extract_payload};

#[derive(Debug, Deserialize, Serialize)]
pub struct PublicChannelUpdatedPayload {
    pub channel_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub updated_by: String,
    pub updated_at: TimeDelta,
}

/// # Public Channel Updated Handler
pub async fn handle_public_channel_updated(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<Message>,
) -> Result<Json<PublicChannelUpdatedPayload>, HandlerError> {
    if msg.payload_type != PayloadType::PublicChannelUpdated {
        return Err(ClientError::InvalidPayloadType {
            expected: "PublicChannelUpdated",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let payload: PublicChannelUpdatedPayload = try_extract_payload(&msg)?;
    let mut state = state.lock().unwrap();
    // Update channel info and bump version
    state.public_channel_id = Some(payload.channel_id.clone());
    if let Some(name) = &payload.name {
        state.public_channel_name = Some(name.clone());
    }
    if let Some(desc) = &payload.description {
        state.public_channel_description = Some(desc.clone());
    }
    state.public_channel_version += 1;
    info!(
        "PublicChannelUpdated: channel {} updated by {} at {} (version {})",
        payload.channel_id, payload.updated_by, payload.updated_at, state.public_channel_version
    );
    Ok(Json(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_valid() {
        let json = r#"{
            "type": "PUBLIC_CHANNEL_UPDATED",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440002",
            "ts": [200, 0],
            "payload": {
                "channel_id": "channel-uuid-1",
                "name": "General",
                "description": "General chat channel",
                "updated_by": "550e8400-e29b-41d4-a716-446655440000",
                "updated_at": [200, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::PublicChannelUpdated);
        let payload = try_extract_payload::<PublicChannelUpdatedPayload>(&parsed).unwrap();
        assert_eq!(payload.channel_id, "channel-uuid-1");
        assert_eq!(payload.name.as_deref(), Some("General"));
    }

    #[test]
    fn deserialize_missing_field() {
        let json = r#"{
            "type": "PUBLIC_CHANNEL_UPDATED",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440002",
            "ts": [200, 0],
            "payload": {
                "description": "General chat channel",
                "updated_by": "550e8400-e29b-41d4-a716-446655440000",
                "updated_at": [200, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to parse Message");
        let result = try_extract_payload::<PublicChannelUpdatedPayload>(&parsed);
        assert!(result.is_err()); // channel_id missing
    }
}
