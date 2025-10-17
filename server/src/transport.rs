// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Transport layer for the server, handling HTTP and WebSocket connections.

use std::sync::{Arc, Mutex};

use axum::extract::Path;
use axum::extract::ws::{Message as WSMessage, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

use crate::AppState;
use crate::handlers::direct_message::handle_direct_message;
use crate::handlers::direct_message::{direct_message_http, poll_direct_messages_http};
use crate::handlers::file_transfer_chunk::{file_transfer_chunk_http, handle_file_transfer_chunk};
use crate::handlers::file_transfer_end::{file_transfer_end_http, handle_file_transfer_end};
use crate::handlers::file_transfer_start::{
    file_transfer_start_http, handle_file_transfer_start, poll_file_events_http,
};
use crate::handlers::heartbeat::handle_heartbeat;
use crate::handlers::list_users::handle_list_users;
use crate::handlers::list_users::list_users_http;
use crate::handlers::server_announce::handle_server_announce;
use crate::handlers::server_deliver::handle_server_deliver;
use crate::handlers::server_hello_join::handle_server_hello_join;
use crate::handlers::server_welcome::handle_server_welcome;
use crate::handlers::user_advertise::handle_user_advertise;
use crate::handlers::user_hello::handle_user_hello;
use crate::handlers::user_hello::heartbeat_http;
use crate::handlers::user_hello::user_hello_http;
use crate::handlers::user_login::handle_user_login;
use crate::handlers::user_register::handle_user_register;
use crate::handlers::user_remove::handle_user_remove;
use crate::messages::{Message, PayloadType};

pub enum ConnectionType {
    Unknown,
    Client,
    Server,
}

/// Represents a connection to a client or another server.
pub struct ConnectionInfo {
    pub conn_type: ConnectionType,
    pub socket: Option<WebSocket>,
    pub pubkey: Option<String>,
}

impl ConnectionInfo {
    /// Create an empty ConnectionInfo (no socket).
    pub fn new() -> Self {
        Self {
            conn_type: ConnectionType::Unknown,
            socket: None,
            pubkey: None,
        }
    }

    /// Create a ConnectionInfo that owns a WebSocket.
    pub fn with_socket(socket: WebSocket) -> Self {
        Self {
            conn_type: ConnectionType::Unknown,
            socket: Some(socket),
            pubkey: None,
        }
    }

    /// Remove and return the owned socket, leaving None.
    pub fn take_socket(&mut self) -> Option<WebSocket> {
        self.socket.take()
    }
}

impl Default for ConnectionInfo {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Mutex<AppState>>>,
) -> Response {
    let state = state.clone();
    ws.on_upgrade(move |socket| {
        let state = state.clone();
        async move {
            ws_handle_socket(socket, state).await;
        }
    })
}

/// Handle persistent Websocket connections between other servers and clients.
async fn ws_handle_socket(mut socket: WebSocket, state: Arc<Mutex<AppState>>) {
    let mut conn_info = ConnectionInfo::new();

    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(m) => m,
            Err(err) => {
                error!("Fatal WS error: {err}");
                return;
            }
        };

        let msg_text: String;
        match &msg {
            WSMessage::Text(text) => {
                msg_text = text.to_string();
            }
            WSMessage::Binary(bin) => match String::from_utf8(bin.to_vec()) {
                Ok(t) => msg_text = t,
                Err(err) => {
                    error!("Failed to parse binary WS message as UTF-8: {err}");
                    continue;
                }
            },
            WSMessage::Close(close_frame) => {
                if let Some(cf) = &close_frame {
                    info!(
                        "WS connection closing: code={}, reason={}",
                        cf.code, cf.reason
                    );
                } else {
                    info!("WS connection closing: no close frame provided");
                }
                return;
            }
            // Ping and pong messages are automatically handled
            WSMessage::Ping(_) | WSMessage::Pong(_) => {
                continue;
            }
        }

        let message = match serde_json::from_str::<Message>(msg_text.as_str()) {
            Ok(m) => m,
            Err(err) => {
                error!("Failed to parse WS message: {err}");
                continue;
            }
        };

        // Handle received messages
        match &message.payload_type {
            PayloadType::ServerHelloJoin => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ =
                    handle_server_hello_join(Json(message.clone()), &mut state_clone, &mut socket)
                        .await;
                info!("Processed ServerHelloJoin from {}", &message.from);
            }
            PayloadType::ServerWelcome => {
                let state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ =
                    handle_server_welcome(Json(message.clone()), state_clone, &mut socket).await;
                info!("Processed ServerWelcome from {}", &message.from);
            }
            PayloadType::ServerAnnounce => {
                let state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ =
                    handle_server_announce(Json(message.clone()), state_clone, &mut socket).await;
                info!("Processed ServerAnnounce from {}", &message.from);
            }
            PayloadType::UserAdvertise => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ = handle_user_advertise(Json(message.clone()), &mut state_clone, &mut socket)
                    .await;
                info!("Processed UserAdvertise from {}", &message.from);
            }
            PayloadType::UserRemove => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ =
                    handle_user_remove(Json(message.clone()), &mut state_clone, &mut socket).await;
                info!("Processed UserRemove from {}", &message.from);
            }
            PayloadType::ServerDeliver => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ = handle_server_deliver(Json(message.clone()), &mut state_clone, &mut socket)
                    .await;
                info!("Processed ServerDeliver from {}", &message.from);
            }
            PayloadType::Heartbeat => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ =
                    handle_heartbeat(Json(message.clone()), &mut state_clone, &mut socket).await;
                info!("Processed Heartbeat from {}", &message.from);
            }
            PayloadType::UserHello => {
                conn_info.conn_type = ConnectionType::Client;
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ = handle_user_hello(Json(message.clone()), &mut state_clone, socket).await;
                break;
            }
            PayloadType::MsgDirect => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ = handle_direct_message(Json(message.clone()), &mut state_clone, &mut socket)
                    .await;
            }
            PayloadType::ListUsers => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ =
                    handle_list_users(Json(message.clone()), &mut state_clone, &mut socket).await;
            }
            PayloadType::UserLogin => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ =
                    handle_user_login(Json(message.clone()), &mut state_clone, &mut socket).await;
            }
            PayloadType::UserRegister => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ = handle_user_register(Json(message.clone()), &mut state_clone, &mut socket)
                    .await;
            }
            PayloadType::PublicChannelAdd
            | PayloadType::PublicChannelUpdated
            | PayloadType::PublicChannelKeyShare
            | PayloadType::MsgPublicChannel => {
                warn!("Received public channel operation over WebSocket; ignored (HTTP-only)");
            }
            PayloadType::UserDeliver => {
                // UserDeliver should be handled by the direct message handler
                // This is for messages that need to be delivered to users on remote servers
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ = handle_direct_message(Json(message.clone()), &mut state_clone, &mut socket)
                    .await;
            }
            PayloadType::FileStart => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ = handle_file_transfer_start(
                    Json(message.clone()),
                    &mut state_clone,
                    &mut socket,
                )
                .await;
            }
            PayloadType::FileChunk => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ = handle_file_transfer_chunk(
                    Json(message.clone()),
                    &mut state_clone,
                    &mut socket,
                )
                .await;
            }
            PayloadType::FileEnd => {
                let mut state_clone = {
                    let state_guard = state.lock().unwrap();
                    state_guard.clone()
                };
                let _ =
                    handle_file_transfer_end(Json(message.clone()), &mut state_clone, &mut socket)
                        .await;
            }
            PayloadType::Ack => {
                // Acknowledgement messages - for now just log them
                info!("Received acknowledgement from {}", message.from);
            }
            PayloadType::Error => {
                // Error messages - log the error
                warn!(
                    "Received error message from {}: {:?}",
                    message.from, message.payload
                );
            }
            PayloadType::InvalidType(t) => {
                warn!("Sender {} sent invalid payload type of {t}", &message.from);
            }
        }

        // TODO: sending messages
        // Only send messages when you must - do not send a message back every time!
        // if let Err(err) = socket.send(msg).await {
        //     error!("Fatal WS send error: {err}");
        //     return;
        // }
    }
}

async fn get_user_pubkey(
    Path(user_id): Path<String>,
    state: axum::extract::State<Arc<Mutex<AppState>>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use secure_chat::id::Id;
    info!("get_user_pubkey called for user_id {}", user_id);
    let state = state.lock().unwrap();
    use std::str::FromStr;
    let id = match Id::from_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid user_id format: {}", user_id);
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let local_users = state.local_users.lock().unwrap();
    if let Some(conn) = local_users.get(&id) {
        let conn = conn.lock().unwrap();
        if let Some(pubkey) = &conn.pubkey {
            info!("Found pubkey for user_id {}", user_id);
            Ok(Json(serde_json::json!({ "pubkey": pubkey })))
        } else {
            warn!("No pubkey set for user_id {}", user_id);
            Err(StatusCode::NOT_FOUND)
        }
    } else {
        warn!("No local_user found for user_id {}", user_id);
        Err(StatusCode::NOT_FOUND)
    }
}

pub fn app_router(state: Arc<Mutex<AppState>>) -> Router {
    Router::new()
        .route("/api/user_hello", post(user_hello_http))
        .route("/api/users/online", get(list_users_http))
        .route("/api/heartbeat", post(heartbeat_http))
        .route("/api/direct_message", post(direct_message_http))
        .route("/api/poll_direct_messages", post(poll_direct_messages_http))
        .route("/api/users/pubkey/{user_id}", get(get_user_pubkey))
        .route("/api/file_start", post(file_transfer_start_http))
        .route("/api/file_chunk", post(file_transfer_chunk_http))
        .route("/api/file_end", post(file_transfer_end_http))
        .route("/api/poll_file_events", post(poll_file_events_http))
        .route(
            "/api/public_channel/add",
            post(crate::handlers::public_channel_add::handle_public_channel_add),
        )
        .route(
            "/api/public_channel/updated",
            post(crate::handlers::public_channel_updated::handle_public_channel_updated),
        )
        .route(
            "/api/public_channel/key_share",
            post(crate::handlers::public_channel_key_share::handle_public_channel_key_share),
        )
        .route(
            "/api/public_channel/message",
            post(crate::handlers::public_channel_message::public_channel_message_http),
        )
        .route(
            "/api/public_channel/messages",
            get(crate::handlers::public_channel_message::poll_public_channel_messages),
        )
        .route(
            "/api/public_channel/file_start",
            post(crate::handlers::public_channel_message::public_channel_file_start),
        )
        .route(
            "/api/public_channel/file_chunk",
            post(crate::handlers::public_channel_message::public_channel_file_chunk),
        )
        .route(
            "/api/public_channel/file_end",
            post(crate::handlers::public_channel_message::public_channel_file_end),
        )
        .route(
            "/api/public_channel/file_events",
            get(crate::handlers::public_channel_message::poll_public_channel_file_events),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}
