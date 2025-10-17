// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Direct Message handling
use axum::extract::State;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::ws::WebSocket;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, try_extract_payload};

#[derive(Debug, Deserialize, Serialize)]
pub struct DirectMessagePayload {
    pub content: String,
    pub client: String,
    pub pubkey: String,
    pub enc_pubkey: String,
}

/// HTTP handler for sending a direct message
pub async fn direct_message_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(msg): Json<Message>,
) -> Result<(), crate::HandlerError> {
    use secure_chat::id::Identifier;
    if msg.payload_type != PayloadType::MsgDirect {
        return Err(ClientError::InvalidPayloadType {
            expected: "MsgDirect",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    // Only support Identifier::Id for both from and to
    let from_id = match &msg.from {
        Identifier::Id(id) => id.clone(),
        _ => {
            return Err(ClientError::InvalidPayloadType {
                expected: "Identifier::Id",
                actual: format!("{:?}", msg.from),
            }
            .into());
        }
    };
    let to_id = match &msg.to {
        Identifier::Id(id) => id.clone(),
        _ => {
            return Err(ClientError::InvalidPayloadType {
                expected: "Identifier::Id",
                actual: format!("{:?}", msg.to),
            }
            .into());
        }
    };
    // Check if recipient is a local user (compare by string representation)
    let to_id_str = to_id.to_string();
    let found = {
        let state = state.lock().unwrap();
        let local_users = state.local_users.lock().unwrap();
        info!(
            "Looking for recipient: {} in {} local users",
            to_id_str,
            local_users.len()
        );
        local_users.keys().any(|k| k.to_string() == to_id_str)
    };
    if found {
        // Wrap as USER_DELIVER for local delivery
        use std::collections::VecDeque;
        // Lookup sender display name and pubkey
        let sender_id_str = from_id.to_string();
        let sender_display_name = {
            let state = state.lock().unwrap();
            let user_locations = state.user_locations.lock().unwrap();
            user_locations
                .get(&from_id)
                .cloned()
                .unwrap_or(sender_id_str.clone())
        };
        let sender_pub = match &msg.payload {
            serde_json::Value::Object(map) => map
                .get("sender_pub")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            _ => "".to_string(),
        };
        let ciphertext = match &msg.payload {
            serde_json::Value::Object(map) => map
                .get("ciphertext")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            _ => serde_json::Value::Null,
        };
        let content_sig = match &msg.payload {
            serde_json::Value::Object(map) => map
                .get("content_sig")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            _ => serde_json::Value::Null,
        };
        // Compose USER_DELIVER message
        let server_id = {
            let state = state.lock().unwrap();
            state.server_id.clone()
        };
        let deliver_msg = crate::messages::Message {
            payload_type: crate::messages::PayloadType::UserDeliver,
            from: crate::messages::Identifier::Id(server_id),
            to: crate::messages::Identifier::Id(to_id.clone()),
            ts: msg.ts,
            payload: serde_json::json!({
                "sender": sender_display_name,
                "sender_pub": sender_pub,
                "ciphertext": ciphertext,
                "content_sig": content_sig
            }),
            sig: "server_sig_placeholder".to_string(), // TODO: real server signature
        };
        let mut state = state.lock().unwrap();
        let entry = state
            .pending_messages
            .entry(to_id.clone())
            .or_insert_with(VecDeque::new);
        entry.push_back(deliver_msg);
        info!(
            "Direct message from {} to {} queued as USER_DELIVER for HTTP polling",
            from_id, to_id
        );
        Ok(())
    } else {
        info!(
            "Direct message recipient {} not found in local users",
            to_id
        );
        Err(ClientError::InvalidPayloadType {
            expected: "Connected local user",
            actual: to_id.to_string(),
        }
        .into())
    }
}

/// HTTP handler for polling pending direct messages
pub async fn poll_direct_messages_http(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<PollDirectMessagesPayload>,
) -> Result<Json<Vec<Message>>, crate::HandlerError> {
    use secure_chat::id::Id;
    use std::str::FromStr;
    let user_id = Id::from_str(&payload.user_id).map_err(|_| ClientError::InvalidPayloadType {
        expected: "valid user_id",
        actual: payload.user_id.clone(),
    })?;
    let mut state = state.lock().unwrap();
    let queue = state
        .pending_messages
        .entry(user_id)
        .or_insert_with(std::collections::VecDeque::new);
    let messages: Vec<Message> = queue.drain(..).collect();
    Ok(Json(messages))
}

#[derive(Deserialize)]
pub struct PollDirectMessagesPayload {
    pub user_id: String,
}

/// # Direct Message Handler
/// Handles a direct message between users.
pub async fn handle_direct_message(
    Json(msg): Json<Message>,
    _state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<DirectMessagePayload>, HandlerError> {
    if msg.payload_type != PayloadType::MsgDirect {
        return Err(ClientError::InvalidPayloadType {
            expected: "MsgDirect",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let payload: DirectMessagePayload = try_extract_payload(&msg)?;
    info!(
        "DirectMessage received: from {} to {}: {}",
        msg.from, msg.to, payload.content
    );
    Ok(Json(payload))
}

#[cfg(test)]
pub async fn handle_direct_message_test(
    Json(msg): Json<Message>,
    state: &mut crate::AppState,
    link: crate::transport::ConnectionInfo,
) -> Json<DirectMessagePayload> {
    use std::sync::{Arc, Mutex};
    let user_id = match &msg.from {
        secure_chat::id::Identifier::Id(id) => id.clone(),
        _ => {
            return Json(DirectMessagePayload {
                content: "Error: Invalid identifier".to_owned(),
                client: "".to_owned(),
                pubkey: "".to_owned(),
                enc_pubkey: "".to_owned(),
            });
        }
    };

    if let Ok(mut local_users) = state.local_users.lock() {
        local_users.insert(user_id, Arc::new(Mutex::new(link)));
    } else {
        return Json(DirectMessagePayload {
            content: "Error: Failed to acquire local_users lock".to_owned(),
            client: "".to_owned(),
            pubkey: "".to_owned(),
            enc_pubkey: "".to_owned(),
        });
    }

    let payload: DirectMessagePayload = match try_extract_payload(&msg) {
        Ok(p) => p,
        Err(_) => {
            return Json(DirectMessagePayload {
                content: "Error: Failed to extract payload".to_owned(),
                client: "".to_owned(),
                pubkey: "".to_owned(),
                enc_pubkey: "".to_owned(),
            });
        }
    };
    Json(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_message_missing_field() {
        let json = r#"{
            "type": "MSG_DIRECT",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [1234, 0],
            "payload": {
                "client": "cli-v1",
                "pubkey": "alice_pubkey",
                "enc_pubkey": "alice_enc_pubkey"
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to parse Message");
        let result = try_extract_payload::<DirectMessagePayload>(&parsed);
        assert!(result.is_err()); // content missing
    }

    #[test]
    fn direct_message_edge_empty_pubkey() {
        let payload = DirectMessagePayload {
            content: "Hi!".to_owned(),
            client: "cli-v1".to_owned(),
            pubkey: "".to_owned(),
            enc_pubkey: "enc123".to_owned(),
        };
        assert_eq!(payload.pubkey, "");
    }

    #[test]
    fn direct_message_edge_empty_enc_pubkey() {
        let payload = DirectMessagePayload {
            content: "Hi!".to_owned(),
            client: "cli-v1".to_owned(),
            pubkey: "pubkey123".to_owned(),
            enc_pubkey: "".to_owned(),
        };
        assert_eq!(payload.enc_pubkey, "");
    }

    #[test]
    fn direct_message_deserialize_works() {
        let json = r#"{
            "type": "MSG_DIRECT",
            "from": "550e8400-e29b-41d4-a716-446655440000",
            "to": "550e8400-e29b-41d4-a716-446655440001",
            "ts": [1234, 0],
            "payload": {
                "content": "Hello Bob!",
                "client": "cli-v1",
                "pubkey": "alice_pubkey",
                "enc_pubkey": "alice_enc_pubkey"
            },
            "sig": ""
        }"#;
        let parsed: Message = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(parsed.payload_type, PayloadType::MsgDirect);
        let payload: DirectMessagePayload =
            try_extract_payload(&parsed).expect("Failed to extract payload");
        assert_eq!(payload.content, "Hello Bob!");
    }

    #[test]
    fn direct_message_payload_fields() {
        let payload = DirectMessagePayload {
            content: "Hi!".to_owned(),
            client: "cli-v1".to_owned(),
            pubkey: "pubkey123".to_owned(),
            enc_pubkey: "enc123".to_owned(),
        };
        assert_eq!(payload.content, "Hi!");
        assert_eq!(payload.client, "cli-v1");
        assert_eq!(payload.pubkey, "pubkey123");
        assert_eq!(payload.enc_pubkey, "enc123");
    }
}
