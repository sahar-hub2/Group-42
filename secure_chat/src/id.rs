// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Universal identifier types

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Universal identifier that can represent users, servers, or broadcast.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Identifier {
    /// Used for broadcasting messages to all users/servers.
    /// Serializes/deserializes as "*"
    Broadcast,
    /// Regular UUID identifier for users or servers.
    /// Serializes/deserializes as UUID string
    Id(Id),
    /// Bootstrap address for joining the network.
    /// Serializes/deserializes as "host:port" string
    Bootstrap(String),
}

impl Identifier {
    /// Create a new random identifier.
    pub fn new() -> Self {
        Identifier::Id(Id::new())
    }

    /// Create a bootstrap identifier from host and port.
    pub fn bootstrap(host: impl Into<String>, port: u16) -> Self {
        Identifier::Bootstrap(format!("{}:{}", host.into(), port))
    }

    /// Get the string representation of the identifier.
    pub fn as_str(&self) -> String {
        match self {
            Identifier::Id(id) => id.to_string(),
            Identifier::Broadcast => "*".to_owned(),
            Identifier::Bootstrap(addr) => addr.to_owned(),
        }
    }

    /// Check if this is a broadcast identifier.
    pub fn is_broadcast(&self) -> bool {
        matches!(self, Identifier::Broadcast)
    }

    /// Get the inner Id if this is an Id variant.
    pub fn as_id(&self) -> Option<&Id> {
        match self {
            Identifier::Id(id) => Some(id),
            _ => None,
        }
    }

    /// Get the bootstrap address if this is a Bootstrap variant.
    pub fn as_bootstrap(&self) -> Option<&str> {
        match self {
            Identifier::Bootstrap(addr) => Some(addr),
            _ => None,
        }
    }
}

impl Default for Identifier {
    fn default() -> Self {
        Identifier::new()
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identifier::Id(id) => write!(f, "{}", id),
            Identifier::Broadcast => write!(f, "*"),
            Identifier::Bootstrap(addr) => write!(f, "{}", addr),
        }
    }
}

impl FromStr for Identifier {
    type Err = uuid::Error;

    /// Create an identifier from a UUID string or bootstrap address.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "*" {
            Ok(Identifier::Broadcast)
        } else if let Ok(id) = Id::from_str(s) {
            Ok(Identifier::Id(id))
        } else {
            // Treat as bootstrap address
            Ok(Identifier::Bootstrap(s.to_string()))
        }
    }
}

impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Id(id) => serializer.serialize_str(&id.to_string()),
            Self::Broadcast => serializer.serialize_str("*"),
            Self::Bootstrap(addr) => serializer.serialize_str(addr),
        }
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "*" {
            Ok(Identifier::Broadcast)
        } else if s == "public" {
            // Map 'public' to a default UUID (could use a fixed UUID or random)
            Ok(Identifier::Id(Id::default()))
        } else {
            // Try to parse as UUID
            match Id::from_str(&s) {
                Ok(id) => Ok(Identifier::Id(id)),
                Err(_) => Ok(Identifier::Bootstrap(s)),
            }
        }
    }
}

/// A UUID-based identifier that can represent either users or servers.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(Uuid);

impl Id {
    /// Create a new random identifier.
    pub fn new() -> Self {
        Id(Uuid::new_v4())
    }

    /// Get the inner UUID.
    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for Id {
    fn default() -> Self {
        Id::new()
    }
}

impl FromStr for Id {
    type Err = uuid::Error;

    /// Create an identifier from a UUID string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(s)?;
        Ok(Id(uuid))
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for Id {
    type Error = uuid::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Id::from_str(value)
    }
}

impl TryFrom<String> for Id {
    type Error = uuid::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Id::from_str(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_serialization() {
        let id = Id::new();
        let user_identifier = Identifier::Id(id.clone());
        let broadcast_identifier = Identifier::Broadcast;
        let bootstrap_identifier = Identifier::Bootstrap("127.0.0.1:8080".to_string());

        // Test serialization produces plain strings
        let user_json = serde_json::to_string(&user_identifier).unwrap();
        let broadcast_json = serde_json::to_string(&broadcast_identifier).unwrap();
        let bootstrap_json = serde_json::to_string(&bootstrap_identifier).unwrap();

        // Remove quotes to get the actual serialized value
        let user_str = user_json.trim_matches('"');
        let broadcast_str = broadcast_json.trim_matches('"');
        let bootstrap_str = bootstrap_json.trim_matches('"');

        // Broadcast should serialize to "*"
        assert_eq!(broadcast_str, "*");

        // UUID should serialize to its string representation
        assert_eq!(user_str, id.to_string());

        // Bootstrap should serialize to address
        assert_eq!(bootstrap_str, "127.0.0.1:8080");

        // Test deserialization
        let user_back: Identifier = serde_json::from_str(&user_json).unwrap();
        let broadcast_back: Identifier = serde_json::from_str(&broadcast_json).unwrap();
        let bootstrap_back: Identifier = serde_json::from_str(&bootstrap_json).unwrap();

        assert_eq!(user_back, user_identifier);
        assert_eq!(broadcast_back, Identifier::Broadcast);
        assert_eq!(bootstrap_back, bootstrap_identifier);
    }

    #[test]
    fn test_identifier_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let broadcast_str = "*";
        let bootstrap_str = "127.0.0.1:8080";

        let id_result = Identifier::from_str(uuid_str).unwrap();
        let broadcast_result = Identifier::from_str(broadcast_str).unwrap();
        let bootstrap_result = Identifier::from_str(bootstrap_str).unwrap();

        match id_result {
            Identifier::Id(_) => (),
            _ => panic!("Should be Id variant"),
        }

        assert_eq!(broadcast_result, Identifier::Broadcast);
        assert_eq!(
            bootstrap_result,
            Identifier::Bootstrap("127.0.0.1:8080".to_string())
        );
    }

    #[test]
    fn test_identifier_methods() {
        let id = Identifier::new();
        let broadcast = Identifier::Broadcast;
        let bootstrap = Identifier::Bootstrap("127.0.0.1:8080".to_string());

        assert!(!id.is_broadcast());
        assert!(broadcast.is_broadcast());
        assert!(!bootstrap.is_broadcast());

        assert!(id.as_id().is_some());
        assert!(broadcast.as_id().is_none());
        assert!(bootstrap.as_id().is_none());

        assert!(bootstrap.as_bootstrap().is_some());
        assert_eq!(bootstrap.as_bootstrap().unwrap(), "127.0.0.1:8080");
    }

    #[test]
    fn test_backward_compatibility() {
        // Test that the type aliases work
        let user_id = Id::new();
        let server_id = Id::new();

        // Should be able to use them in Identifier
        let user_identifier = Identifier::Id(user_id);
        let server_identifier = Identifier::Id(server_id);

        // Test serialization still works
        let user_json = serde_json::to_string(&user_identifier).unwrap();
        let server_json = serde_json::to_string(&server_identifier).unwrap();

        // Should deserialize back correctly
        let _user_back: Identifier = serde_json::from_str(&user_json).unwrap();
        let _server_back: Identifier = serde_json::from_str(&server_json).unwrap();
    }

    #[test]
    fn test_display_formatting() {
        let id = Id::new();
        let identifier = Identifier::Id(id.clone());
        let broadcast = Identifier::Broadcast;
        let bootstrap = Identifier::Bootstrap("127.0.0.1:8080".to_string());

        assert_eq!(format!("{}", identifier), format!("{}", id));
        assert_eq!(format!("{}", broadcast), "*");
        assert_eq!(format!("{}", bootstrap), "127.0.0.1:8080");
    }

    #[test]
    fn test_bootstrap_constructor() {
        let bootstrap = Identifier::bootstrap("localhost", 3000);
        assert_eq!(format!("{}", bootstrap), "localhost:3000");
        assert_eq!(bootstrap.as_bootstrap(), Some("localhost:3000"));
    }
}
