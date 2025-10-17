// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! List users handler
use axum::extract::State;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::ws::WebSocket;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType};

#[derive(Serialize, Debug)]
pub struct ListUsersHttpUser {
    pub user_id: String,
    pub display_name: String,
}

#[derive(Serialize)]
pub struct ListUsersHttpResponse {
    pub users: Vec<ListUsersHttpUser>,
}

/// HTTP handler for listing online users
pub async fn list_users_http(
    State(state): State<Arc<Mutex<AppState>>>,
) -> Json<ListUsersHttpResponse> {
    let state = state.lock().unwrap();
    let local_users = state.local_users.lock().unwrap();
    let user_locations = state.user_locations.lock().unwrap();
    let users: Vec<ListUsersHttpUser> = local_users
        .keys()
        .map(|id| {
            let user_id = id.to_string();
            let display_name = user_locations
                .get(id)
                .cloned()
                .unwrap_or_else(|| user_id.clone());
            ListUsersHttpUser {
                user_id,
                display_name,
            }
        })
        .collect();
    info!(user_count = users.len(), ?users, "list_users_http called");
    Json(ListUsersHttpResponse { users })
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListUsersResponse {
    pub status: String,
    pub users: Vec<String>,
}

pub async fn handle_list_users(
    msg: Json<Message>,
    state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<ListUsersResponse>, HandlerError> {
    if msg.payload_type != PayloadType::ListUsers {
        return Err(ClientError::InvalidPayloadType {
            expected: "ListUsers",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    // Return the list of local users (keys of local_users)
    let local_users = state.local_users.lock().map_err(|_| {
        ClientError::PayloadExtraction("Failed to acquire local_users lock".to_string())
    })?;
    let users: Vec<String> = local_users.keys().map(|u| u.to_string()).collect();
    Ok(Json(ListUsersResponse {
        status: "ok".to_owned(),
        users,
    }))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // Basic AppState for testing
    struct AppStateTest {
        pub local_users: HashMap<String, Arc<Mutex<()>>>,
    }

    #[test]
    fn list_users_one_user() {
        let mut state = AppStateTest {
            local_users: HashMap::new(),
        };
        state
            .local_users
            .insert("alice".to_owned(), Arc::new(Mutex::new(())));
        let users: Vec<String> = state.local_users.keys().cloned().collect();
        assert_eq!(users.len(), 1);
        assert!(users.contains(&"alice".to_owned()));
    }

    #[test]
    fn list_users_five_users() {
        let mut state = AppStateTest {
            local_users: HashMap::new(),
        };
        let names = ["alice", "bob", "carol", "dave", "eve"];
        for name in &names {
            state
                .local_users
                .insert(name.to_string(), Arc::new(Mutex::new(())));
        }
        let users: Vec<String> = state.local_users.keys().cloned().collect();
        assert_eq!(users.len(), 5);
        for name in &names {
            assert!(users.contains(&name.to_string()));
        }
    }
}
