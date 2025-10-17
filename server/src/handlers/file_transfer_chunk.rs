// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! File Transfer Chunk message handling

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
pub struct FileTransferChunkPayload {
    pub file_id: String,
    pub chunk_index: u64,
    pub chunk_data: String,
    pub sender: String,
    pub receiver: String,
    pub sent_at: TimeDelta,
}

/// # File Transfer Chunk Handler
pub async fn handle_file_transfer_chunk(
    Json(msg): Json<Message>,
    _state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<FileTransferChunkPayload>, HandlerError> {
    if msg.payload_type != PayloadType::FileChunk {
        return Err(ClientError::InvalidPayloadType {
            expected: "FileChunk",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let payload: FileTransferChunkPayload = try_extract_payload(&msg)?;
    info!(
        "FileTransferChunk received: file {} chunk {} from {} to {}, data len {}",
        payload.file_id,
        payload.chunk_index,
        payload.sender,
        payload.receiver,
        payload.chunk_data.len()
    );
    Ok(Json(payload))
}

/// HTTP handler for file transfer chunk
pub async fn file_transfer_chunk_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<crate::messages::Message>,
) -> Json<&'static str> {
    // Queue the file chunk event for the recipient
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

    fn sample_payload() -> FileTransferChunkPayload {
        FileTransferChunkPayload {
            file_id: "file-uuid-1".to_owned(),
            chunk_index: 0,
            chunk_data: "deadbeef".to_owned(),
            sender: "user-uuid-sender".to_owned(),
            receiver: "user-uuid-receiver".to_owned(),
            sent_at: TimeDelta::seconds(101),
        }
    }

    #[test]
    fn deserialize_valid() {
        let json = r#"{
            "type": "FILE_CHUNK",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [101, 0],
            "payload": {
                "file_id": "file-uuid-1",
                "chunk_index": 0,
                "chunk_data": "deadbeef",
                "sender": "550e8400-e29b-41d4-a716-446655440000",
                "receiver": "550e8400-e29b-41d4-a716-446655440001",
                "sent_at": [101, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::FileChunk);
        let payload = try_extract_payload::<FileTransferChunkPayload>(&parsed).unwrap();
        assert_eq!(payload.chunk_data, "deadbeef");
        assert_eq!(payload.chunk_index, 0);
    }

    #[test]
    fn deserialize_missing_field() {
        let json = r#"{
            "type": "FILE_CHUNK",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [101, 0],
            "payload": {
                "file_id": "file-uuid-1",
                "chunk_data": "deadbeef",
                "sender": "550e8400-e29b-41d4-a716-446655440000",
                "receiver": "550e8400-e29b-41d4-a716-446655440001",
                "sent_at": [101, 0]
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to parse Message");
        let result = try_extract_payload::<FileTransferChunkPayload>(&parsed);
        assert!(result.is_err()); // chunk_index missing
    }

    #[test]
    fn payload_fields() {
        let payload = sample_payload();
        assert_eq!(payload.file_id, "file-uuid-1");
        assert_eq!(payload.chunk_index, 0);
        assert_eq!(payload.chunk_data, "deadbeef");
    }
}
