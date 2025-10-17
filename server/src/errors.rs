// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use axum::http::StatusCode;
use thiserror::Error;
use tracing::{debug, error};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to deserialize JSON: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("Failed to serialize JSON: {0}")]
    Serialization(String),

    #[error("Invalid signature")]
    InvalidSig,

    #[error("Invalid payload type: expected {expected:?}, got {actual:?}")]
    InvalidPayloadType {
        expected: &'static str,
        actual: String,
    },

    #[error("Failed to extract payload: {0}")]
    PayloadExtraction(String),
}

#[derive(Error, Debug)]
#[error("Internal server error")]
pub struct ServerError(#[from] anyhow::Error);

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("Client error: {0}")]
    Client(#[from] ClientError),

    #[error(transparent)]
    Server(#[from] ServerError),
}

impl axum::response::IntoResponse for HandlerError {
    fn into_response(self) -> axum::response::Response {
        match self {
            HandlerError::Client(err) => {
                debug!("Client error: {err}");
                (StatusCode::BAD_REQUEST, err.to_string()).into_response()
            }
            HandlerError::Server(err) => {
                error!("Internal server error: {err}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}
