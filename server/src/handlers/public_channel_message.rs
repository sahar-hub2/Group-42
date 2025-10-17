// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use crate::AppState;
use crate::errors::{ClientError, HandlerError};
use crate::messages::Message;
use crate::messages::{PayloadType, try_extract_payload};
use axum::{Json, extract::Query, extract::State};
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;

/// POST /api/public_channel/file_start
pub async fn public_channel_file_start(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<Message>,
) -> Json<&'static str> {
    let mut state = state.lock().unwrap();
    // Limit to last 100 file events
    if state.public_channel_file_events.len() >= 100 {
        state.public_channel_file_events.pop_front();
    }
    state.public_channel_file_events.push_back(msg);
    Json("ok")
}

/// POST /api/public_channel/file_chunk
pub async fn public_channel_file_chunk(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<Message>,
) -> Json<&'static str> {
    let mut state = state.lock().unwrap();
    if state.public_channel_file_events.len() >= 100 {
        state.public_channel_file_events.pop_front();
    }
    state.public_channel_file_events.push_back(msg);
    Json("ok")
}

/// POST /api/public_channel/file_end
pub async fn public_channel_file_end(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<Message>,
) -> Json<&'static str> {
    let mut state = state.lock().unwrap();
    if state.public_channel_file_events.len() >= 100 {
        state.public_channel_file_events.pop_front();
    }
    state.public_channel_file_events.push_back(msg);
    Json("ok")
}

/// GET /api/public_channel/file_events?since=timestamp
pub async fn poll_public_channel_file_events(
    State(state): State<Arc<Mutex<AppState>>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<Message>> {
    let since = params
        .get("since")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let state = state.lock().unwrap();
    let events: Vec<Message> = state
        .public_channel_file_events
        .iter()
        .filter(|msg| msg.ts.num_seconds() > since)
        .cloned()
        .collect();
    Json(events)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PublicChannelMessagePayload {
    pub channel_id: String,
    pub from: String,
    pub content: String,
    pub sent_at: TimeDelta,
}

pub async fn poll_public_channel_messages(
    State(state): State<Arc<Mutex<AppState>>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<PublicChannelMessagePayload>> {
    let since = params
        .get("since")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let exclude_from = params.get("exclude_from");
    let state = state.lock().unwrap();
    let msgs: Vec<PublicChannelMessagePayload> = state
        .public_channel_messages
        .iter()
        .filter(|msg| msg.sent_at.num_seconds() > since)
        .filter(|msg| match exclude_from {
            Some(user_id) => &msg.from != user_id,
            None => true,
        })
        .cloned()
        .collect();
    Json(msgs)
}

/// HTTP handler for posting a public channel message
#[derive(Debug, Deserialize)]
pub struct PublicChannelMessageHttpRequest {
    pub channel_id: String,
    pub from: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct PublicChannelMessageHttpResponse {
    pub status: String,
    pub delivered: bool,
}

pub async fn public_channel_message_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(req): Json<PublicChannelMessageHttpRequest>,
) -> Json<PublicChannelMessageHttpResponse> {
    // Assume every user is already in the public channel
    info!(
        "PublicChannelMessage delivered (HTTP): channel {} from {}: {}",
        req.channel_id, req.from, req.content
    );
    // Store message in AppState for polling
    let now = chrono::Utc::now();
    let payload = PublicChannelMessagePayload {
        channel_id: req.channel_id.clone(),
        from: req.from.clone(),
        content: req.content.clone(),
        sent_at: TimeDelta::zero() + chrono::Duration::seconds(now.timestamp()),
    };
    {
        let mut state = state.lock().unwrap();
        // Limit to last 100 messages
        if state.public_channel_messages.len() >= 100 {
            state.public_channel_messages.pop_front();
        }
        state.public_channel_messages.push_back(payload);
    }
    Json(PublicChannelMessageHttpResponse {
        status: "ok".to_string(),
        delivered: true,
    })
}

/// # Public Channel Message Handler
pub async fn handle_public_channel_message(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<Message>,
) -> Result<Json<PublicChannelMessagePayload>, HandlerError> {
    if msg.payload_type != PayloadType::MsgPublicChannel {
        return Err(ClientError::InvalidPayloadType {
            expected: "MsgPublicChannel",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let payload: PublicChannelMessagePayload = try_extract_payload(&msg)?;
    let state = state.lock().unwrap();
    // Only allow delivery if sender is a member
    use std::str::FromStr;
    let sender_id = secure_chat::id::Id::from_str(&payload.from).ok();
    let is_member = sender_id
        .as_ref()
        .map(|id| state.public_channel_members.contains(id))
        .unwrap_or(false);
    if is_member {
        info!(
            "PublicChannelMessage delivered: channel {} from {}: {}",
            payload.channel_id, payload.from, payload.content
        );
        // TODO: Fan-out to all members' hosting servers
    } else {
        info!(
            "PublicChannelMessage rejected: sender {} is not a member of channel {}",
            payload.from, payload.channel_id
        );
    }
    Ok(Json(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_valid() {
        let json = r#"{
            "type": "MSG_PUBLIC_CHANNEL",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440002",
            "ts": [202, 0],
            "payload": {
                "channel_id": "channel-uuid-1",
                "from": "550e8400-e29b-41d4-a716-446655440000",
                "content": "Hello everyone!",
                "sent_at": [202, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::MsgPublicChannel);
        let payload = try_extract_payload::<PublicChannelMessagePayload>(&parsed).unwrap();
        assert_eq!(payload.content, "Hello everyone!");
    }

    #[test]
    fn deserialize_missing_field() {
        let json = r#"{
            "type": "MSG_PUBLIC_CHANNEL",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440002",
            "ts": [202, 0],
            "payload": {
                "channel_id": "channel-uuid-1",
                "content": "Hello everyone!",
                "sent_at": [202, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to parse Message");
        let result = try_extract_payload::<PublicChannelMessagePayload>(&parsed);
        assert!(result.is_err()); // from missing
    }
}
