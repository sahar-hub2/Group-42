// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Bootstrap process for connecting to existing servers in the network

use std::sync::{Arc, Mutex};

use anyhow::{Error, Result, anyhow};
use chrono::TimeDelta;
use futures_util::SinkExt;
use futures_util::stream::StreamExt;
use secure_chat::id::{Id, Identifier};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tracing::{error, info, warn};

use crate::AppState;
use crate::messages::{
    Message, PayloadType, ServerAnnouncePayload, ServerHelloJoinPayload, ServerWelcomePayload,
    message_from_payload, try_extract_payload,
};
use crate::transport::ConnectionInfo;
use crate::utils::sign_message;

/// Performs the bootstrap process to connect this server to the existing network
pub async fn bootstrap_process(mut state: AppState, host: String, port: u16) -> Result<(), Error> {
    let this_host = host;
    let this_port = port;
    let mut successful = false;
    info!("Begin bootstrap process");

    for bs in state.config.bootstrap_servers.iter().cloned() {
        // TODO: use wss?
        let url = format!("ws://{}:{}", bs.host, bs.port);
        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();

                // Prepare HELLO_JOIN message
                info!("Preparing HELLO_JOIN for bootstrap server at {url}");
                let ts = TimeDelta::milliseconds(chrono::Utc::now().timestamp_millis());
                let to = Identifier::Bootstrap(format!("{}:{}", bs.host, bs.port));
                let payload = ServerHelloJoinPayload {
                    host: this_host.clone(),
                    port: this_port,
                    pubkey: state.server_pubkey.clone(),
                };

                let mut msg = message_from_payload(
                    PayloadType::ServerHelloJoin,
                    Identifier::Id(state.server_id.clone()),
                    to,
                    ts,
                    payload,
                    "".to_owned(), // Allowed to be empty for HELLO_JOIN
                );

                // Sign the message
                let msg_without_sig = {
                    let mut m = msg.clone();
                    m.sig = "".to_owned();
                    m
                };
                msg.sig = sign_message(&msg_without_sig, state.server_crypto.as_ref())
                    .map_err(|e| anyhow!("Failed to sign message: {e}"))?;

                let msg_text = serde_json::to_string(&msg)?;
                info!("Sending HELLO_JOIN to bootstrap server at {url}");
                if let Err(e) = write.send(WsMessage::Text(msg_text)).await {
                    error!("Failed to send HELLO_JOIN: {e}");
                    continue;
                }

                // Wait for WELCOME with timeout
                let timeout = std::time::Duration::from_secs(10); // 10 second timeout
                let receive_future = read.next();

                match tokio::time::timeout(timeout, receive_future).await {
                    Ok(Some(Ok(WsMessage::Text(text)))) => {
                        let resp: Message = match serde_json::from_str(&text) {
                            Ok(m) => m,
                            Err(e) => {
                                error!("Failed to parse WELCOME: {e}");
                                continue;
                            }
                        };
                        if resp.payload_type == PayloadType::ServerWelcome {
                            let welcome_payload: ServerWelcomePayload =
                                match try_extract_payload(&resp) {
                                    Ok(p) => p,
                                    Err(e) => {
                                        error!("Failed to extract WELCOME payload: {}", e);
                                        continue;
                                    }
                                };

                            if welcome_payload.assigned_id != state.server_id.to_string() {
                                state.server_id =
                                    match Id::try_from(welcome_payload.assigned_id.clone()) {
                                        Ok(id) => id,
                                        Err(e) => {
                                            error!("Invalid assigned ID: {}", e);
                                            continue;
                                        }
                                    };
                            }

                            // Add bootstrap server to known
                            let introducer_id = match resp.from.as_id() {
                                Some(id) => id.clone(),
                                None => {
                                    error!("Invalid from in WELCOME");
                                    continue;
                                }
                            };
                            let conn = ConnectionInfo::default();
                            if let Ok(mut servers) = state.servers.lock() {
                                servers.insert(introducer_id.clone(), Arc::new(Mutex::new(conn)));
                            } else {
                                error!("Failed to acquire servers lock");
                                continue;
                            }

                            if let Ok(mut server_addrs) = state.server_addrs.lock() {
                                server_addrs.insert(introducer_id.clone(), (bs.host, bs.port));
                            } else {
                                error!("Failed to acquire server_addrs lock");
                                continue;
                            }

                            // Note: bs.pubkey should be used, but assuming it's pinned in config
                            if let Ok(mut server_pubkeys) = state.server_pubkeys.lock() {
                                server_pubkeys.insert(introducer_id.clone(), bs.pubkey);
                            } else {
                                error!("Failed to acquire server_pubkeys lock");
                                continue;
                            }

                            // Update with servers from welcome
                            for server in welcome_payload.servers {
                                let server_id = match Id::try_from(server.server_id) {
                                    Ok(id) => id,
                                    Err(e) => {
                                        error!("Invalid server ID in welcome: {}", e);
                                        continue;
                                    }
                                };
                                info!(
                                    "Added server {}: {}:{}",
                                    server_id, server.host, server.port
                                );
                                if let Ok(mut server_addrs) = state.server_addrs.lock() {
                                    server_addrs
                                        .insert(server_id.clone(), (server.host, server.port));
                                } else {
                                    error!(
                                        "Failed to acquire server_addrs lock for server {}",
                                        server_id
                                    );
                                    continue;
                                }

                                if let Ok(mut server_pubkeys) = state.server_pubkeys.lock() {
                                    server_pubkeys.insert(server_id.clone(), server.pubkey);
                                } else {
                                    error!(
                                        "Failed to acquire server_pubkeys lock for server {}",
                                        server_id
                                    );
                                    continue;
                                }
                            }

                            // Update with clients from welcome
                            for client in welcome_payload.clients {
                                let user_id = match Id::try_from(client.user_id) {
                                    Ok(id) => id,
                                    Err(e) => {
                                        error!("Invalid user ID in welcome: {}", e);
                                        continue;
                                    }
                                };
                                if let Ok(mut user_locations) = state.user_locations.lock() {
                                    user_locations
                                        .insert(user_id.clone(), client.server_id.clone());
                                } else {
                                    error!(
                                        "Failed to acquire user_locations lock for user {}",
                                        user_id
                                    );
                                    continue;
                                }

                                if let Ok(mut user_pubkeys) = state.user_pubkeys.lock() {
                                    user_pubkeys.insert(user_id.clone(), client.pubkey.clone());
                                } else {
                                    error!(
                                        "Failed to acquire user_pubkeys lock for user {}",
                                        user_id
                                    );
                                    continue;
                                }
                                info!(
                                    "Added remote user {} on server {}",
                                    user_id, client.server_id
                                );
                            }

                            successful = true;
                            info!("Bootstrapped with introducer {introducer_id}");

                            // Properly close the WebSocket connection
                            if let Err(e) = write.close().await {
                                warn!("Failed to close WebSocket connection cleanly: {e}");
                            }
                            break;
                        }
                    }
                    Ok(Some(Ok(msg))) => {
                        warn!("Received non-text message during bootstrap: {:?}", msg);
                        // Try to close connection cleanly before continuing
                        let _ = write.close().await;
                        continue;
                    }
                    Ok(Some(Err(e))) => {
                        error!("WebSocket error during bootstrap: {e}");
                        // Try to close connection cleanly before continuing
                        let _ = write.close().await;
                        continue;
                    }
                    Ok(None) => {
                        warn!("Connection closed while waiting for WELCOME");
                        continue;
                    }
                    Err(_) => {
                        error!("Timeout waiting for WELCOME from {url}");
                        // Try to close connection cleanly before continuing
                        let _ = write.close().await;
                        continue;
                    }
                }
            }
            Err(e) => error!("Connection to {url} failed: {e}"),
        }
    }

    if successful {
        *state
            .bootstrapped
            .lock()
            .map_err(|_| anyhow!("Failed to acquire bootstrapped lock"))? = true;
        announce_to_network(state, this_host, this_port).await?;
        info!("Server announced to network");
    } else {
        warn!("Bootstrap failed - server starting in isolated mode");
    }

    Ok(())
}

/// Broadcasts server announcement to all known servers after successful bootstrap
async fn announce_to_network(state: AppState, host: String, port: u16) -> Result<(), Error> {
    let ts = TimeDelta::milliseconds(chrono::Utc::now().timestamp_millis());
    let payload = ServerAnnouncePayload {
        host,
        port,
        pubkey: state.server_pubkey.clone(),
    };

    let mut announce = message_from_payload(
        PayloadType::ServerAnnounce,
        Identifier::Id(state.server_id.clone()),
        Identifier::Broadcast,
        ts,
        payload,
        "".to_string(),
    );

    // Sign the announcement
    let announce_without_sig = {
        let mut m = announce.clone();
        m.sig = "".to_string();
        m
    };
    announce.sig = sign_message(&announce_without_sig, state.server_crypto.as_ref())
        .map_err(|e| anyhow!("Failed to sign announcement: {e}"))?;

    let announce_text = serde_json::to_string(&announce)?;

    // Collect server addresses first to avoid holding the lock across await points
    let server_addrs: Vec<(Id, (String, u16))> = state
        .server_addrs
        .lock()
        .map_err(|_| anyhow!("Failed to acquire server_addrs lock for announcements"))?
        .iter()
        .map(|(id, addr)| (id.clone(), addr.clone()))
        .collect();

    // Send to all known servers (excluding ourselves)
    for (id, addr) in server_addrs {
        // Don't announce to ourselves
        if id != state.server_id {
            let url = format!("ws://{}:{}", addr.0, addr.1);
            info!("Announcing to server {id} at {url}");

            // Attempt to connect and send announcement
            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, _read) = ws_stream.split();
                    if let Err(e) = write.send(WsMessage::Text(announce_text.clone())).await {
                        error!("Failed to send ANNOUNCE to {id}: {e}");
                    } else {
                        info!("Successfully announced to server {id}");
                    }

                    // Properly close the WebSocket connection
                    if let Err(e) = write.close().await {
                        warn!("Failed to close WebSocket connection to {id} cleanly: {e}");
                    }
                }
                Err(e) => {
                    error!("Failed to connect to server {id} for announcement: {e}");
                }
            }
        } else {
            info!("Skipping announcement to self ({id})");
        }
    }

    info!("Server announcement broadcast completed");
    Ok(())
}
