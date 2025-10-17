// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use axum::Json;
use axum::extract::ws::WebSocket;
use chrono::TimeDelta;
use secure_chat::id::Identifier;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::{error, info};

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{
    ClientInfo, Message, PayloadType, ServerHelloJoinPayload, ServerInfo, ServerWelcomePayload,
    message_from_payload, try_extract_payload,
};
use crate::transport::ConnectionInfo;
use crate::utils::sign_message;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ServerHelloJoinResponse {
    pub status: Status,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Error,
}

/// Handle a SERVER_HELLO_JOIN message.
/// This is received by a bootstrap server when another server wants to join the network.
/// Responds with a SERVER_WELCOME message containing network information.
pub async fn handle_server_hello_join(
    Json(msg): Json<Message>,
    state: &mut AppState,
    socket: &mut WebSocket,
) -> Result<Json<ServerHelloJoinResponse>, HandlerError> {
    if msg.payload_type != PayloadType::ServerHelloJoin {
        return Err(ClientError::InvalidPayloadType {
            expected: "ServerHelloJoin",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }

    let payload: ServerHelloJoinPayload = try_extract_payload(&msg)?;

    info!(
        "Received ServerHelloJoin from {} requesting to join network",
        &msg.from
    );

    // Extract the joining server's ID
    let joining_server_id = match msg.from.as_id() {
        Some(id) => id.clone(),
        None => {
            error!("Invalid sender ID in ServerHelloJoin");
            return Err(ClientError::PayloadExtraction("Invalid sender ID".to_owned()).into());
        }
    };

    // Add the joining server to our known servers
    let conn = ConnectionInfo::new();
    state
        .servers
        .lock()
        .map_err(|_| ClientError::PayloadExtraction("Failed to acquire servers lock".to_owned()))?
        .insert(joining_server_id.clone(), Arc::new(Mutex::new(conn)));

    // Store server address and public key
    state
        .server_addrs
        .lock()
        .map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire server_addrs lock".to_owned())
        })?
        .insert(
            joining_server_id.clone(),
            (payload.host.clone(), payload.port),
        );

    state
        .server_pubkeys
        .lock()
        .map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire server_pubkeys lock".to_owned())
        })?
        .insert(joining_server_id.clone(), payload.pubkey.clone());

    info!(
        "Added joining server {} at {}:{} to known servers",
        joining_server_id, payload.host, payload.port
    );

    // Prepare ServerWelcome response with current network state
    let mut servers = Vec::new();
    {
        let server_addrs = state.server_addrs.lock().map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire server_addrs lock".to_owned())
        })?;
        let server_pubkeys = state.server_pubkeys.lock().map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire server_pubkeys lock".to_owned())
        })?;

        info!(
            "Building ServerWelcome for {}: server_addrs contains {} servers",
            joining_server_id,
            server_addrs.len()
        );

        for (server_id, addr) in server_addrs.iter() {
            info!(
                "Considering server {} at {}:{} for ServerWelcome",
                server_id, addr.0, addr.1
            );
            // Exclude the joining server from the list (they already know about themselves)
            if server_id != &joining_server_id {
                if let Some(pubkey) = server_pubkeys.get(server_id) {
                    info!("Including server {} in ServerWelcome", server_id);
                    servers.push(ServerInfo {
                        server_id: server_id.to_string(),
                        host: addr.0.clone(),
                        port: addr.1,
                        pubkey: pubkey.clone(),
                    });
                } else {
                    info!("Skipping server {} - no public key found", server_id);
                }
            } else {
                info!("Excluding joining server {} from ServerWelcome", server_id);
            }
        }
    } // Close the mutex lock block

    // Prepare clients list from current user locations
    let mut clients = Vec::new();
    {
        let user_locations = state.user_locations.lock().map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire user_locations lock".to_owned())
        })?;
        let user_pubkeys = state.user_pubkeys.lock().map_err(|_| {
            ClientError::PayloadExtraction("Failed to acquire user_pubkeys lock".to_owned())
        })?;

        for (user_id, server_id) in user_locations.iter() {
            if let Some(pubkey) = user_pubkeys.get(user_id) {
                clients.push(ClientInfo {
                    user_id: user_id.to_string(),
                    server_id: server_id.clone(),
                    pubkey: pubkey.clone(),
                });
            }
        }
    } // Close the mutex lock block

    let welcome_payload = ServerWelcomePayload {
        assigned_id: joining_server_id.to_string(), // Assign the same ID they requested
        servers,
        clients,
    };

    let ts = TimeDelta::milliseconds(chrono::Utc::now().timestamp_millis());
    let mut welcome_msg = message_from_payload(
        PayloadType::ServerWelcome,
        Identifier::Id(state.server_id.clone()),
        Identifier::Id(joining_server_id.clone()),
        ts,
        welcome_payload,
        "".to_owned(),
    );

    // Sign the welcome message
    let welcome_without_sig = {
        let mut m = welcome_msg.clone();
        m.sig = "".to_owned();
        m
    };
    welcome_msg.sig =
        sign_message(&welcome_without_sig, state.server_crypto.as_ref()).map_err(|e| {
            error!("Failed to sign ServerWelcome message: {}", e);
            ClientError::PayloadExtraction(format!("Failed to sign message: {}", e))
        })?;

    // Send the ServerWelcome response
    let welcome_text = serde_json::to_string(&welcome_msg).map_err(|e| {
        error!("Failed to serialize ServerWelcome: {}", e);
        ClientError::PayloadExtraction(format!("Failed to serialize message: {}", e))
    })?;

    if let Err(e) = socket
        .send(axum::extract::ws::Message::Text(welcome_text.into()))
        .await
    {
        error!("Failed to send ServerWelcome response: {}", e);
        return Err(
            ClientError::PayloadExtraction(format!("Failed to send response: {}", e)).into(),
        );
    }

    info!("Sent ServerWelcome to joining server {}", joining_server_id);

    Ok(Json(ServerHelloJoinResponse { status: Status::Ok }))
}
