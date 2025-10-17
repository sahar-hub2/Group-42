// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Heartbeat message handling
//!
//! Server health check implementation. Servers can send heartbeats every 15s
//! and should be considered dead if no response received for 45s.

use axum::Json;
use axum::extract::ws::WebSocket;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::try_extract_payload;
use crate::messages::{Message, PayloadType};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct HeartbeatPayload {
    // Empty payload as per spec
}

/// Heartbeat Handler
///
/// Handles HEARTBEAT messages for server health monitoring.
pub async fn handle_heartbeat(
    Json(msg): Json<Message>,
    state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<HeartbeatResponse>, HandlerError> {
    if msg.payload_type != PayloadType::Heartbeat {
        return Err(ClientError::InvalidPayloadType {
            expected: "Heartbeat",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }

    let _payload: HeartbeatPayload = try_extract_payload(&msg)?;

    // Verify signature using the sending server's public key
    // TODO: Implement signature verification with server public keys

    // Log the heartbeat from the sending server
    info!("Received heartbeat from server: {}", msg.from);

    // Update last_seen timestamp for the sending server
    // We'll update this in a future implementation when we add server health tracking
    // For now, we log the heartbeat receipt as evidence of server health
    let server_id = match &msg.from {
        secure_chat::id::Identifier::Id(id) => id.clone(),
        _ => {
            warn!(
                "Received heartbeat from non-server identifier: {}",
                msg.from
            );
            return Err(ClientError::PayloadExtraction(
                "Heartbeat must come from a server ID".to_string(),
            )
            .into());
        }
    };

    // Check if we have this server in our connections
    let servers = state.servers.lock().map_err(|_| {
        ClientError::PayloadExtraction("Failed to acquire servers lock".to_string())
    })?;

    if servers.contains_key(&server_id) {
        info!("Heartbeat confirmed from known server: {}", server_id);
    } else {
        warn!("Received heartbeat from unknown server: {}", server_id);
    }

    Ok(Json(HeartbeatResponse {
        status: "alive".to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
    }))
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub status: String,
    pub timestamp: i64,
}

#[cfg(test)]
pub async fn handle_heartbeat_test(
    Json(msg): Json<Message>,
    _state: &mut crate::AppState,
    _link: crate::transport::ConnectionInfo,
) -> Json<HeartbeatPayload> {
    let payload: HeartbeatPayload = try_extract_payload(&msg).unwrap();
    Json(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Message, PayloadType};

    #[test]
    fn heartbeat_deserialize_works() {
        let json = r#"{
            "type": "HEARTBEAT",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [5678, 0],
            "payload": {},
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::Heartbeat);
        let _payload = try_extract_payload::<HeartbeatPayload>(&parsed).unwrap();
        // Payload is empty, just verify it parses correctly
    }
}
