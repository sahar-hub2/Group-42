// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use std::path::Path;

use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub skip_bootstrap: bool,
    pub bootstrap_servers: Vec<BootstrapServer>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapServer {
    pub host: String,
    pub port: u16,
    pub pubkey: String,
}

impl ServerConfig {
    /// Load configuration from a YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path_str = path.as_ref().to_str().ok_or_else(|| {
            ConfigError::Message("Invalid path: contains non-UTF8 characters".to_string())
        })?;

        let config = Config::builder()
            .add_source(File::with_name(path_str))
            .build()?;

        config.try_deserialize()
    }

    /// Load configuration from a YAML file with default fallback
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config.yaml").required(false))
            .add_source(File::with_name("config.yml").required(false))
            .build()?;

        config.try_deserialize()
    }

    /// Load configuration with custom file path and environment variable override
    pub fn load_with_env(config_path: Option<&str>) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // Add config file source
        if let Some(path) = config_path {
            builder = builder.add_source(File::with_name(path));
        } else {
            // Try default config file names
            builder = builder
                .add_source(File::with_name("config.yaml").required(false))
                .add_source(File::with_name("config.yml").required(false));
        }

        // Build and deserialize
        let config = builder.build()?;
        config.try_deserialize()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            skip_bootstrap: false,
            bootstrap_servers: vec![BootstrapServer {
                host: "127.0.0.1".to_string(),
                port: 8080,
                pubkey: "".to_string(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_config_deserialization() {
        let yaml_content = r#"
skip_bootstrap: true
bootstrap_servers:
  - host: "192.0.1.2"
    port: 12345
    pubkey: "BASE64URL(RSA-4096-PUB)"
  - host: "198.50.100.3"
    port: 5432
    pubkey: "BASE64URL(RSA-4096-PUB)"
  - host: "203.0.113.21"
    port: 1212
    pubkey: "BASE64URL(RSA-4096-PUB)"
"#;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_config.yaml");
        fs::write(&file_path, yaml_content).unwrap();

        let config = ServerConfig::from_file(file_path).unwrap();

        assert!(config.skip_bootstrap);
        assert_eq!(config.bootstrap_servers.len(), 3);
        assert_eq!(config.bootstrap_servers[0].host, "192.0.1.2");
        assert_eq!(config.bootstrap_servers[0].port, 12345);
        assert_eq!(config.bootstrap_servers[1].host, "198.50.100.3");
        assert_eq!(config.bootstrap_servers[1].port, 5432);
        assert_eq!(config.bootstrap_servers[2].host, "203.0.113.21");
        assert_eq!(config.bootstrap_servers[2].port, 1212);
    }

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert!(!config.skip_bootstrap);
        assert_eq!(config.bootstrap_servers.len(), 1);
        assert_eq!(config.bootstrap_servers[0].host, "127.0.0.1");
        assert_eq!(config.bootstrap_servers[0].port, 8080);
    }
}
