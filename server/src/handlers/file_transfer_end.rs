// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! File Transfer End message handling

use axum::Json;
use axum::extract::ws::WebSocket;
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, try_extract_payload};
use axum::extract::State;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize, Serialize)]
pub struct FileTransferEndPayload {
    pub file_id: String,
    pub sender: String,
    pub receiver: String,
    pub ended_at: TimeDelta,
}

/// # File Transfer End Handler
pub async fn handle_file_transfer_end(
    Json(msg): Json<Message>,
    _state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<FileTransferEndPayload>, HandlerError> {
    if msg.payload_type != PayloadType::FileEnd {
        return Err(ClientError::InvalidPayloadType {
            expected: "FileEnd",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let payload: FileTransferEndPayload = try_extract_payload(&msg)?;
    info!(
        "FileTransferEnd received: file {} from {} to {} at {}",
        payload.file_id, payload.sender, payload.receiver, payload.ended_at
    );
    Ok(Json(payload))
}

/// HTTP handler for file transfer end
pub async fn file_transfer_end_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<crate::messages::Message>,
) -> Json<&'static str> {
    // Queue the file end event for the recipient
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn sample_payload() -> FileTransferEndPayload {
        FileTransferEndPayload {
            file_id: "file-uuid-1".to_owned(),
            sender: "user-uuid-sender".to_owned(),
            receiver: "user-uuid-receiver".to_owned(),
            ended_at: TimeDelta::seconds(102),
        }
    }

    #[test]
    fn deserialize_valid() {
        let json = r#"{
            "type": "FILE_END",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [102, 0],
            "payload": {
                "file_id": "file-uuid-1",
                "sender": "550e8400-e29b-41d4-a716-446655440000",
                "receiver": "550e8400-e29b-41d4-a716-446655440001",
                "ended_at": [102, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::FileEnd);
        let payload = try_extract_payload::<FileTransferEndPayload>(&parsed).unwrap();
        assert_eq!(payload.file_id, "file-uuid-1");
        assert_eq!(payload.sender, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn deserialize_missing_field() {
        let json = r#"{
            "type": "FILE_END",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [102, 0],
            "payload": {
                "file_id": "file-uuid-1",
                "receiver": "550e8400-e29b-41d4-a716-446655440001",
                "ended_at": [102, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to parse Message");
        let result = try_extract_payload::<FileTransferEndPayload>(&parsed);
        assert!(result.is_err()); // sender missing
    }

    #[test]
    fn payload_fields() {
        let payload = sample_payload();
        assert_eq!(payload.file_id, "file-uuid-1");
        assert_eq!(payload.sender, "user-uuid-sender");
        assert_eq!(payload.receiver, "user-uuid-receiver");
    }
}
