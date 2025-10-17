// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! User register handler

use axum::Json;
use axum::extract::ws::WebSocket;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, Status, try_extract_payload};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserRegisterPayload {
    pub username: String,
    pub pubkey: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserRegisterResponse {
    pub status: Status,
    pub user: Option<String>,
}

pub async fn handle_user_register(
    Json(msg): Json<Message>,
    _state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<UserRegisterResponse>, HandlerError> {
    if msg.payload_type != PayloadType::UserRegister {
        return Err(ClientError::InvalidPayloadType {
            expected: "UserRegister",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let _payload: UserRegisterPayload = try_extract_payload(&msg)?;
    // TODO: Implement actual registration logic
    Ok(Json(UserRegisterResponse {
        status: Status::NotImplemented,
        user: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_placeholder() {
        let resp = UserRegisterResponse {
            status: Status::NotImplemented,
            user: None,
        };
        assert_eq!(resp.status, Status::NotImplemented);
        assert!(resp.user.is_none());
    }
}
