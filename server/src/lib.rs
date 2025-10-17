// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use secure_chat::crypto::RsaUtil;
use secure_chat::id::Id;

use crate::config::ServerConfig;
use crate::transport::ConnectionInfo;

pub mod bootstrap;
pub mod config;
pub mod constants;
pub mod errors;
pub mod handlers;
pub mod messages;
pub mod network_utils;
pub mod transport;

pub use errors::HandlerError;

/// Application state to be shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// user_id -> last heartbeat time
    pub user_heartbeat: HashMap<Id, Instant>,
    /// user_id -> queue of pending messages (for HTTP polling)
    pub pending_messages:
        std::collections::HashMap<Id, std::collections::VecDeque<crate::messages::Message>>,

    /// Set of user IDs in the public channel (all users by default)
    pub public_channel_members: std::collections::HashSet<Id>,
    /// Current version of the public channel (bumped on add/update)
    pub public_channel_version: u64,
    /// Current public channel key (opaque string for now)
    pub public_channel_key: Option<String>,
    /// Channel ID for the public channel (single channel for now)
    pub public_channel_id: Option<String>,
    /// Channel name (optional, for display)
    pub public_channel_name: Option<String>,
    /// Channel description (optional)
    pub public_channel_description: Option<String>,
    /// Recent public channel messages (for polling)
    pub public_channel_messages: std::collections::VecDeque<
        crate::handlers::public_channel_message::PublicChannelMessagePayload,
    >,
    /// Recent public channel file events (for polling)
    pub public_channel_file_events: std::collections::VecDeque<crate::messages::Message>,
    /// Server configuration
    pub config: ServerConfig,
    /// Whether the server has completed the bootstrap process
    pub bootstrapped: Arc<Mutex<bool>>,
    /// This server's unique identifier
    pub server_id: Id,
    /// This server's cryptography utilities
    pub server_crypto: Arc<RsaUtil>,
    /// This server's public key (base64url encoded)
    pub server_pubkey: String,

    /// Connections to other servers
    pub servers: Arc<Mutex<HashMap<Id, Arc<Mutex<ConnectionInfo>>>>>,
    /// Mapping of server IDs to their advertised addresses
    pub server_addrs: Arc<Mutex<HashMap<Id, (String, u16)>>>,
    /// Mapping of server IDs to their public keys
    pub server_pubkeys: Arc<Mutex<HashMap<Id, String>>>,
    /// Connections to local users
    pub local_users: Arc<Mutex<HashMap<Id, Arc<Mutex<ConnectionInfo>>>>>,
    /// Mapping of user IDs to the server IDs where they are connected
    pub user_locations: Arc<Mutex<HashMap<Id, String>>>,
    /// Mapping of user IDs to their public keys
    pub user_pubkeys: Arc<Mutex<HashMap<Id, String>>>,
}

impl Default for AppState {
    fn default() -> Self {
        // Dummy crypto for default - will be overridden
        let dummy_crypto = Arc::new(RsaUtil::new().expect("Failed to create dummy crypto"));
        let dummy_pubkey = dummy_crypto.pubkey_base64url().unwrap_or_default();

        Self {
            user_heartbeat: HashMap::new(),
            pending_messages: HashMap::new(),
            public_channel_members: std::collections::HashSet::new(),
            public_channel_version: 0,
            public_channel_key: None,
            public_channel_id: None,
            public_channel_name: None,
            public_channel_description: None,
            public_channel_messages: std::collections::VecDeque::new(),
            public_channel_file_events: std::collections::VecDeque::new(),
            config: ServerConfig::default(),
            bootstrapped: Arc::new(Mutex::new(false)),
            server_id: Id::new(),
            server_crypto: dummy_crypto,
            server_pubkey: dummy_pubkey,
            servers: Arc::new(Mutex::new(HashMap::new())),
            server_addrs: Arc::new(Mutex::new(HashMap::new())),
            server_pubkeys: Arc::new(Mutex::new(HashMap::new())),
            local_users: Arc::new(Mutex::new(HashMap::new())),
            user_locations: Arc::new(Mutex::new(HashMap::new())),
            user_pubkeys: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Utility functions for common operations
pub mod utils {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use rsa::signature::SignatureEncoding;
    use secure_chat::crypto::CryptoUtil;
    use serde::Serialize;

    use crate::errors::ClientError;

    /// Sign a message and return the base64-encoded signature
    pub fn sign_message<T: Serialize>(
        message: &T,
        crypto: &dyn CryptoUtil,
    ) -> Result<String, ClientError> {
        let msg_json = serde_json::to_string(message)
            .map_err(|e| ClientError::Serialization(e.to_string()))?;
        let sig = crypto.sign(msg_json.as_bytes());
        Ok(STANDARD_NO_PAD.encode(sig.to_vec()))
    }
}
