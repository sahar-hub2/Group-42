// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Public Channel Key Share message handling

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
pub struct PublicChannelKeySharePayload {
    pub channel_id: String,
    pub key: String,
    pub shared_by: String,
    pub shared_at: TimeDelta,
}

/// # Public Channel Key Share Handler
pub async fn handle_public_channel_key_share(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<Message>,
) -> Result<Json<PublicChannelKeySharePayload>, HandlerError> {
    if msg.payload_type != PayloadType::PublicChannelKeyShare {
        return Err(ClientError::InvalidPayloadType {
            expected: "PublicChannelKeyShare",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let payload: PublicChannelKeySharePayload = try_extract_payload(&msg)?;
    let mut state = state.lock().unwrap();
    // Store the key for the public channel
    state.public_channel_key = Some(payload.key.clone());
    info!(
        "PublicChannelKeyShare stored: channel {} key shared by {} at {}",
        payload.channel_id, payload.shared_by, payload.shared_at
    );
    Ok(Json(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_valid() {
        let json = r#"{
            "type": "PUBLIC_CHANNEL_KEY_SHARE",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440002",
            "ts": [201, 0],
            "payload": {
                "channel_id": "channel-uuid-1",
                "key": "supersecretkey",
                "shared_by": "550e8400-e29b-41d4-a716-446655440000",
                "shared_at": [201, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::PublicChannelKeyShare);
        let payload = try_extract_payload::<PublicChannelKeySharePayload>(&parsed).unwrap();
        assert_eq!(payload.key, "supersecretkey");
    }

    #[test]
    fn deserialize_missing_field() {
        let json = r#"{
            "type": "PUBLIC_CHANNEL_KEY_SHARE",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440002",
            "ts": [201, 0],
            "payload": {
                "channel_id": "channel-uuid-1",
                "shared_by": "550e8400-e29b-41d4-a716-446655440000",
                "shared_at": [201, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to parse Message");
        let result = try_extract_payload::<PublicChannelKeySharePayload>(&parsed);
        assert!(result.is_err()); // key missing
    }
}
