// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use axum::Json;
use axum::extract::ws::WebSocket;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::{error, info};

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, ServerAnnouncePayload, try_extract_payload};
use crate::transport::ConnectionInfo;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ServerAnnounceResponse {
    pub status: Status,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Error,
}

/// Handle a SERVER_ANNOUNCE message.
/// This is received when a server announces its presence to the network
/// after successfully bootstrapping.
pub async fn handle_server_announce(
    Json(msg): Json<Message>,
    state: AppState,
    _socket: &mut WebSocket,
) -> Result<Json<ServerAnnounceResponse>, HandlerError> {
    if msg.payload_type != PayloadType::ServerAnnounce {
        return Err(ClientError::InvalidPayloadType {
            expected: "ServerAnnounce",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }

    let payload: ServerAnnouncePayload = try_extract_payload(&msg)?;

    info!("Received ServerAnnounce from {}", &msg.from);

    // Extract the announcing server's ID
    let announcing_server_id = match msg.from.as_id() {
        Some(id) => id.clone(),
        None => {
            error!("Invalid sender ID in ServerAnnounce");
            return Err(ClientError::PayloadExtraction("Invalid sender ID".to_owned()).into());
        }
    };

    // Add/update the announcing server in our known servers
    let conn = ConnectionInfo::new();
    state
        .servers
        .lock()
        .map_err(|_| ClientError::PayloadExtraction("Failed to acquire servers lock".to_owned()))?
        .insert(announcing_server_id.clone(), Arc::new(Mutex::new(conn)));

    // Store/update server address and public key
    state
        .server_addrs
        .lock()
        .map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire server_addrs lock".to_owned())
        })?
        .insert(
            announcing_server_id.clone(),
            (payload.host.clone(), payload.port),
        );

    state
        .server_pubkeys
        .lock()
        .map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire server_pubkeys lock".to_owned())
        })?
        .insert(announcing_server_id.clone(), payload.pubkey.clone());

    info!(
        "Added/updated announcing server {} at {}:{} to known servers",
        announcing_server_id, payload.host, payload.port
    );

    Ok(Json(ServerAnnounceResponse { status: Status::Ok }))
}
