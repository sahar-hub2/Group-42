// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! User login handler

use axum::Json;
use axum::extract::ws::WebSocket;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::HandlerError;
use crate::errors::ClientError;
use crate::messages::{Message, PayloadType, Status, try_extract_payload};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserLoginPayload {
    pub username: String,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserLoginResponse {
    pub status: Status,
    pub user: Option<String>,
}

pub async fn handle_user_login(
    Json(msg): Json<Message>,
    _state: &mut AppState,
    _link: &mut WebSocket,
) -> Result<Json<UserLoginResponse>, HandlerError> {
    if msg.payload_type != PayloadType::UserLogin {
        return Err(ClientError::InvalidPayloadType {
            expected: "USER_LOGIN",
            actual: format!("{:?}", msg.payload_type),
        }
        .into());
    }
    let _payload: UserLoginPayload = try_extract_payload(&msg)?;
    // TODO: Implement actual login logic
    Ok(Json(UserLoginResponse {
        status: Status::NotImplemented,
        user: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_placeholder() {
        let resp = UserLoginResponse {
            status: Status::NotImplemented,
            user: None,
        };
        assert_eq!(resp.status, Status::NotImplemented);
        assert!(resp.user.is_none());
    }
}
