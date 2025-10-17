// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Protocol messages
pub use secure_chat::id::Identifier;

use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;

use crate::errors::ClientError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayloadType {
    /// Parsing error - invalid payload type.
    InvalidType(String),

    /// Server

    /// Server joining the network. Used for bootstrapping.
    ServerHelloJoin,
    /// Server already in the network welcoming the new server.
    ServerWelcome,
    /// Server broadcasting presence.
    ServerAnnounce,
    /// Advertise a local user.
    UserAdvertise,
    /// Remove a user the network from on disconnect.
    UserRemove,
    /// Forward message to remote user.
    ServerDeliver,
    /// Server health.
    Heartbeat,

    /// User

    /// User announces presence to local server.
    UserHello,
    /// List users request
    ListUsers,
    /// User login request
    UserLogin,
    /// User register request
    UserRegister,
    /// Messaging (both users are on the same local server).
    MsgDirect,
    /// Messaging (deliver to a user on a remote server).
    UserDeliver,

    /// Public channels

    /// Join public channel.
    PublicChannelAdd,
    /// Update public channel.
    PublicChannelUpdated,
    /// Public channel key share.
    PublicChannelKeyShare,
    /// Send a message in the public channel.
    MsgPublicChannel,

    /// File transfer

    /// File transfer start.
    FileStart,
    /// A chunk of the file being transferred.
    FileChunk,
    /// End of the file transfer.
    FileEnd,

    ///  Acknowledgements

    /// ACKs.
    Ack,
    /// Error.
    Error,
}

impl From<&str> for PayloadType {
    fn from(value: &str) -> Self {
        match value {
            "SERVER_HELLO_JOIN" => PayloadType::ServerHelloJoin,
            "SERVER_WELCOME" => PayloadType::ServerWelcome,
            "SERVER_ANNOUNCE" => PayloadType::ServerAnnounce,
            "USER_ADVERTISE" => PayloadType::UserAdvertise,
            "USER_REMOVE" => PayloadType::UserRemove,
            "SERVER_DELIVER" => PayloadType::ServerDeliver,
            "HEARTBEAT" => PayloadType::Heartbeat,
            "USER_HELLO" => PayloadType::UserHello,
            "LIST_USERS" => PayloadType::ListUsers,
            "USER_LOGIN" => PayloadType::UserLogin,
            "USER_REGISTER" => PayloadType::UserRegister,
            "MSG_DIRECT" => PayloadType::MsgDirect,
            "USER_DELIVER" => PayloadType::UserDeliver,
            "PUBLIC_CHANNEL_ADD" => PayloadType::PublicChannelAdd,
            "PUBLIC_CHANNEL_UPDATED" => PayloadType::PublicChannelUpdated,
            "PUBLIC_CHANNEL_KEY_SHARE" => PayloadType::PublicChannelKeyShare,
            "MSG_PUBLIC_CHANNEL" => PayloadType::MsgPublicChannel,
            "FILE_START" => PayloadType::FileStart,
            "FILE_CHUNK" => PayloadType::FileChunk,
            "FILE_END" => PayloadType::FileEnd,
            "ACK" => PayloadType::Ack,
            "ERROR" => PayloadType::Error,
            _ => PayloadType::InvalidType(value.to_owned()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// Parsing error - invalid error code.
    InvalidErrorCode,
    /// User ID doesn't exist.
    UserNotFound,
    /// Invalid signature.
    InvalidSig,
    /// Invalid public key.
    BadKey,
    /// Operation timed out.
    Timeout,
    /// Payload type is unknown.
    UnknownType,
    /// Username is already in use.
    NameInUse,
}

// Status used in handler responses. Keep this in messages so it's
// available to any module that works with protocol response payloads.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Error,
    NotImplemented,
}

/// Payload for SERVER_HELLO_JOIN message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHelloJoinPayload {
    pub host: String,
    pub port: u16,
    pub pubkey: String,
}

/// Payload for SERVER_WELCOME message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerWelcomePayload {
    pub assigned_id: String,
    pub servers: Vec<ServerInfo>,
    pub clients: Vec<ClientInfo>,
}

/// Server information for SERVER_WELCOME payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_id: String,
    pub host: String,
    pub port: u16,
    pub pubkey: String,
}

/// Client information for SERVER_WELCOME payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user_id: String,
    pub pubkey: String,
    pub server_id: String,
}

/// Payload for SERVER_ANNOUNCE message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAnnouncePayload {
    pub host: String,
    pub port: u16,
    pub pubkey: String,
}

impl From<&str> for ErrorCode {
    fn from(value: &str) -> Self {
        match value {
            "USER_NOT_FOUND" => ErrorCode::UserNotFound,
            "INVALID_SIG" => ErrorCode::InvalidSig,
            "BAD_KEY" => ErrorCode::BadKey,
            "TIMEOUT" => ErrorCode::Timeout,
            "UNKNOWN_TYPE" => ErrorCode::UnknownType,
            "NAME_IN_USE" => ErrorCode::NameInUse,
            _ => ErrorCode::InvalidErrorCode,
        }
    }
}

/// Base protocol message format
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    /// Payload type.
    #[serde(rename = "type")]
    pub payload_type: PayloadType,
    /// Sender ID.
    pub from: Identifier,
    /// Recipient ID.
    pub to: Identifier,
    /// Timestamp of message (Unix timestamp in milliseconds).
    pub ts: TimeDelta,
    /// Message payload. The exact structure depends on the payload type.
    pub payload: serde_json::Value,
    /// Signature of the message.
    #[serde_as(as = "Base64")]
    pub sig: String,
}

/// Utility to create a Message from a typed payload.
pub fn message_from_payload<T: serde::Serialize>(
    ptype: PayloadType,
    from: Identifier,
    to: Identifier,
    ts: TimeDelta,
    payload: T,
    sig: String,
) -> Message {
    Message {
        payload_type: ptype,
        from,
        to,
        ts,
        payload: serde_json::to_value(payload).expect("Failed to serialize payload"),
        sig,
    }
}

/// Utility to extract typed payload from Message.
pub fn try_extract_payload<T: for<'de> serde::Deserialize<'de>>(
    msg: &Message,
) -> Result<T, ClientError> {
    serde_json::from_value(msg.payload.clone()).map_err(ClientError::Deserialization)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_type_from_str_server_messages() {
        assert_eq!(
            PayloadType::from("SERVER_HELLO_JOIN"),
            PayloadType::ServerHelloJoin
        );
        assert_eq!(
            PayloadType::from("SERVER_WELCOME"),
            PayloadType::ServerWelcome
        );
        assert_eq!(
            PayloadType::from("SERVER_ANNOUNCE"),
            PayloadType::ServerAnnounce
        );
        assert_eq!(
            PayloadType::from("USER_ADVERTISE"),
            PayloadType::UserAdvertise
        );
        assert_eq!(PayloadType::from("USER_REMOVE"), PayloadType::UserRemove);
        assert_eq!(
            PayloadType::from("SERVER_DELIVER"),
            PayloadType::ServerDeliver
        );
        assert_eq!(PayloadType::from("HEARTBEAT"), PayloadType::Heartbeat);
    }

    #[test]
    fn test_payload_type_from_str_user_messages() {
        assert_eq!(PayloadType::from("USER_HELLO"), PayloadType::UserHello);
        assert_eq!(PayloadType::from("LIST_USERS"), PayloadType::ListUsers);
        assert_eq!(PayloadType::from("USER_LOGIN"), PayloadType::UserLogin);
        assert_eq!(
            PayloadType::from("USER_REGISTER"),
            PayloadType::UserRegister
        );
        assert_eq!(PayloadType::from("MSG_DIRECT"), PayloadType::MsgDirect);
        assert_eq!(PayloadType::from("USER_DELIVER"), PayloadType::UserDeliver);
    }

    #[test]
    fn test_payload_type_from_str_public_channel_messages() {
        assert_eq!(
            PayloadType::from("PUBLIC_CHANNEL_ADD"),
            PayloadType::PublicChannelAdd
        );
        assert_eq!(
            PayloadType::from("PUBLIC_CHANNEL_UPDATED"),
            PayloadType::PublicChannelUpdated
        );
        assert_eq!(
            PayloadType::from("PUBLIC_CHANNEL_KEY_SHARE"),
            PayloadType::PublicChannelKeyShare
        );
        assert_eq!(
            PayloadType::from("MSG_PUBLIC_CHANNEL"),
            PayloadType::MsgPublicChannel
        );
    }

    #[test]
    fn test_payload_type_from_str_file_transfer_messages() {
        assert_eq!(PayloadType::from("FILE_START"), PayloadType::FileStart);
        assert_eq!(PayloadType::from("FILE_CHUNK"), PayloadType::FileChunk);
        assert_eq!(PayloadType::from("FILE_END"), PayloadType::FileEnd);
    }

    #[test]
    fn test_payload_type_from_str_acknowledgement_messages() {
        assert_eq!(PayloadType::from("ACK"), PayloadType::Ack);
        assert_eq!(PayloadType::from("ERROR"), PayloadType::Error);
    }

    #[test]
    fn test_payload_type_from_str_invalid() {
        let invalid_type = PayloadType::from("INVALID_MESSAGE_TYPE");
        match invalid_type {
            PayloadType::InvalidType(msg) => assert_eq!(msg, "INVALID_MESSAGE_TYPE"),
            _ => panic!("Expected InvalidType variant"),
        }

        let empty_type = PayloadType::from("");
        match empty_type {
            PayloadType::InvalidType(msg) => assert_eq!(msg, ""),
            _ => panic!("Expected InvalidType variant"),
        }

        let random_type = PayloadType::from("RANDOM_STRING_1234567890");
        match random_type {
            PayloadType::InvalidType(msg) => assert_eq!(msg, "RANDOM_STRING_1234567890"),
            _ => panic!("Expected InvalidType variant"),
        }
    }

    #[test]
    fn test_payload_type_from_str_case_sensitive() {
        // Test that the conversion is case-sensitive
        let lowercase_type = PayloadType::from("server_hello_join");
        match lowercase_type {
            PayloadType::InvalidType(msg) => assert_eq!(msg, "server_hello_join"),
            _ => panic!("Expected InvalidType variant for lowercase input"),
        }

        let mixed_case_type = PayloadType::from("Server_Hello_Join");
        match mixed_case_type {
            PayloadType::InvalidType(msg) => assert_eq!(msg, "Server_Hello_Join"),
            _ => panic!("Expected InvalidType variant for mixed case input"),
        }
    }

    #[test]
    fn test_error_code_from_str_valid() {
        assert_eq!(ErrorCode::from("USER_NOT_FOUND"), ErrorCode::UserNotFound);
        assert_eq!(ErrorCode::from("INVALID_SIG"), ErrorCode::InvalidSig);
        assert_eq!(ErrorCode::from("BAD_KEY"), ErrorCode::BadKey);
        assert_eq!(ErrorCode::from("TIMEOUT"), ErrorCode::Timeout);
        assert_eq!(ErrorCode::from("UNKNOWN_TYPE"), ErrorCode::UnknownType);
        assert_eq!(ErrorCode::from("NAME_IN_USE"), ErrorCode::NameInUse);
    }

    #[test]
    fn test_error_code_from_str_invalid() {
        assert_eq!(
            ErrorCode::from("INVALID_ERROR_CODE"),
            ErrorCode::InvalidErrorCode
        );
        assert_eq!(
            ErrorCode::from("NONEXISTENT_ERROR"),
            ErrorCode::InvalidErrorCode
        );
        assert_eq!(ErrorCode::from(""), ErrorCode::InvalidErrorCode);
        assert_eq!(
            ErrorCode::from("random_string"),
            ErrorCode::InvalidErrorCode
        );
    }

    #[test]
    fn test_error_code_from_str_case_sensitive() {
        // Test that the conversion is case-sensitive
        assert_eq!(
            ErrorCode::from("user_not_found"),
            ErrorCode::InvalidErrorCode
        );
        assert_eq!(
            ErrorCode::from("User_Not_Found"),
            ErrorCode::InvalidErrorCode
        );
        assert_eq!(ErrorCode::from("INVALID_sig"), ErrorCode::InvalidErrorCode);
    }

    #[test]
    fn test_error_code_from_str_whitespace() {
        // Test that whitespace is not handled
        assert_eq!(
            ErrorCode::from(" USER_NOT_FOUND"),
            ErrorCode::InvalidErrorCode
        );
        assert_eq!(
            ErrorCode::from("USER_NOT_FOUND "),
            ErrorCode::InvalidErrorCode
        );
        assert_eq!(
            ErrorCode::from(" USER_NOT_FOUND "),
            ErrorCode::InvalidErrorCode
        );
    }
}
