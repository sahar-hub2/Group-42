// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! File Transfer Start message handling

use axum::Json;
use axum::extract::ws::WebSocket;
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::info;

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, try_extract_payload};
use axum::extract::State;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize, Serialize)]
pub struct FileTransferStartPayload {
    pub file_id: String,
    pub filename: String,
    pub filesize: u64,
    pub sender: String,   // user UUID
    pub receiver: String, // user UUID
    pub started_at: TimeDelta,
}

/// # File Transfer Start Handler
pub async fn handle_file_transfer_start(
    Json(msg): Json<Message>,
    _state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<FileTransferStartPayload>, HandlerError> {
    if msg.payload_type != PayloadType::FileStart {
        return Err(ClientError::InvalidPayloadType {
            expected: "FileStart",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let payload: FileTransferStartPayload = try_extract_payload(&msg)?;
    info!(
        "FileTransferStart received: file {} from {} to {}, size {}",
        payload.filename, payload.sender, payload.receiver, payload.filesize
    );
    Ok(Json(payload))
}

/// HTTP handler for file transfer start
pub async fn file_transfer_start_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<crate::messages::Message>,
) -> Json<&'static str> {
    // Queue the file start event for the recipient
    let to_id = match &msg.to {
        crate::messages::Identifier::Id(id) => id.clone(),
        _ => return Json("invalid to id"),
    };
    let mut state = state.lock().unwrap();
    let queue = state
        .pending_messages
        .entry(to_id)
        .or_insert_with(std::collections::VecDeque::new);
    queue.push_back(msg);
    Json("ok")
}

/// HTTP handler for polling file events
pub async fn poll_file_events_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<Vec<crate::messages::Message>> {
    // Expect { "user_id": "..." }
    let user_id = match payload.get("user_id").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Json(vec![]),
    };
    let id = match secure_chat::id::Id::from_str(user_id) {
        Ok(id) => id,
        Err(_) => return Json(vec![]),
    };
    let mut state = state.lock().unwrap();
    let queue = state
        .pending_messages
        .entry(id)
        .or_insert_with(std::collections::VecDeque::new);
    let messages: Vec<crate::messages::Message> = queue.drain(..).collect();
    Json(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn sample_payload() -> FileTransferStartPayload {
        FileTransferStartPayload {
            file_id: "file-uuid-1".to_owned(),
            filename: "test.txt".to_owned(),
            filesize: 1024,
            sender: "user-uuid-sender".to_owned(),
            receiver: "user-uuid-receiver".to_owned(),
            started_at: TimeDelta::seconds(100),
        }
    }

    #[test]
    fn deserialize_valid() {
        let json = r#"{
            "type": "FILE_START",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [100, 0],
            "payload": {
                "file_id": "file-uuid-1",
                "filename": "test.txt",
                "filesize": 1024,
                "sender": "550e8400-e29b-41d4-a716-446655440000",
                "receiver": "550e8400-e29b-41d4-a716-446655440001",
                "started_at": [100, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::FileStart);
        let payload = try_extract_payload::<FileTransferStartPayload>(&parsed).unwrap();
        assert_eq!(payload.filename, "test.txt");
        assert_eq!(payload.filesize, 1024);
    }

    #[test]
    fn deserialize_missing_field() {
        let json = r#"{
            "type": "FILE_START",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [100, 0],
            "payload": {
                "file_id": "file-uuid-1",
                "filename": "test.txt",
                "sender": "550e8400-e29b-41d4-a716-446655440000",
                "receiver": "550e8400-e29b-41d4-a716-446655440001",
                "started_at": [100, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to parse Message");
        let result = try_extract_payload::<FileTransferStartPayload>(&parsed);
        assert!(result.is_err()); // filesize missing
    }

    #[test]
    fn payload_fields() {
        let payload = sample_payload();
        assert_eq!(payload.filename, "test.txt");
        assert_eq!(payload.filesize, 1024);
        assert_eq!(payload.sender, "user-uuid-sender");
        assert_eq!(payload.receiver, "user-uuid-receiver");
    }
}
