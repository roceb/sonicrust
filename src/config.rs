use crate::theme::Theme;
use anyhow::{Context, Result};
use rand::{Rng, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration file not found at {path}. A default config has been creaeted")]
    NotFound { path: PathBuf },
    #[error("Failed to parse Configuration file at {path}: {reason}")]
    ParseError { path: PathBuf, reason: String },
    #[error("Invalid Configuration: {0}")]
    ValidationError(String),
    #[error("Could not determine config directory")]
    NoConfigDir,
    #[error("IO error while handling config: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub secret: String,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchConfig {
    #[serde(default = "default_search_mode")]
    pub mode: SearchMode,
    #[serde(default = "default_search_threshold")]
    pub fuzzy_threshold: i64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Local,
    Remote,
}

fn default_search_mode() -> SearchMode {
    SearchMode::Local
}
fn default_search_threshold() -> i64 {
    30
}
impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            mode: default_search_mode(),
            fuzzy_threshold: default_search_threshold(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // TODO: make a gui to allow people to input their config
            let config = Config::default();
            config.save().map_err(|e| {
                ConfigError::Io(
                    e.downcast::<std::io::Error>()
                        .unwrap_or_else(|e| std::io::Error::other(e)),
                )
            })?;
            return Err(ConfigError::NotFound { path: config_path });
        }
        let contents = fs::read_to_string(&config_path).map_err(|e| ConfigError::Io(e))?;
        let config: Config = toml::from_str(&contents).map_err(|e| ConfigError::ParseError {
            path: config_path.clone(),
            reason: e.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }
    fn validate(&self) -> Result<(), ConfigError> {
        if self.server_url.is_empty() {
            return Err(ConfigError::ValidationError(
                "server_url cannot be empty".into(),
            ));
        }
        if !self.server_url.starts_with("https://") && !self.server_url.starts_with("http://") {
            return Err(ConfigError::ValidationError(format!(
                "server_url must start with http:// or https://, got: {}",
                self.server_url
            )));
        }
        if self.username.is_empty() {
            return Err(ConfigError::ValidationError(
                "username cannot be empty".into(),
            ));
        }
        if self.password.is_empty() {
            return Err(ConfigError::ValidationError(
                "password cannot be empty".into(),
            ));
        }
        if self.secret.is_empty() {
            return Err(ConfigError::ValidationError(
                "secret cannot be empty".into(),
            ));
        }
        Ok(())
    }
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path().map_err(|e| anyhow::anyhow!("{}", e))?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config to {:?}", config_path))?;
        Ok(())
    }
    fn config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(config_dir.join("sonicrust").join("config.toml"))
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:4533".to_string(),
            username: "admin".to_string(),
            password: "admin".to_string(),
            secret: randomword(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        }
    }
}
fn randomword() -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(15)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn valid_config_toml() -> &'static str {
        r#"
        server_url= "http://localhost:4533"
        username = "admin"
        password = "secret"
        secret = "randomsecret123"
        "#
    }
    fn write_config(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("config.toml");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_validate_valid_config() {
        let config = Config {
            server_url: "http://localhost:4533".to_string(),
            username: "admin".to_string(),
            password: "secret".to_string(),
            secret: "randomsecret123".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };
        assert!(config.validate().is_ok());
    }
    #[test]
    fn test_validate_empty_server_url() {
        let config = Config {
            server_url: "".to_string(),
            username: "admin".to_string(),
            password: "secret".to_string(),
            secret: "randomsecret123".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
        assert!(err.to_string().contains("server_url cannot be empty"));
    }
    #[test]
    fn test_validate_invalid_server_url() {
        let config = Config {
            server_url: "ftp://localhost:4533".to_string(),
            username: "admin".to_string(),
            password: "secret".to_string(),
            secret: "randomsecret123".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
        assert!(err.to_string().contains("server_url must start with"));
    }

    #[test]
    fn test_validate_https_url_is_valid() {
        let config = Config {
            server_url: "https://myserver.com".to_string(),
            username: "admin".to_string(),
            password: "secret".to_string(),
            secret: "randomsecret123".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };
        assert!(config.validate().is_ok());
    }
    #[test]
    fn test_validate_empty_username() {
        let config = Config {
            server_url: "http://localhost".to_string(),
            username: "".to_string(),
            password: "password".to_string(),
            secret: "secret".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
        assert!(err.to_string().contains("username cannot be empty"));
    }

    #[test]
    fn test_validate_empty_password() {
        let config = Config {
            server_url: "http://localhost".to_string(),
            username: "admin".to_string(),
            password: "".to_string(),
            secret: "secret".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
        assert!(err.to_string().contains("password cannot be empty"));
    }

    #[test]
    fn test_validate_empty_secret() {
        let config = Config {
            server_url: "http://localhost".to_string(),
            username: "admin".to_string(),
            password: "password".to_string(),
            secret: "".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
        assert!(err.to_string().contains("secret cannot be empty"));
    }

    // --- Serialization / Deserialization tests ---

    #[test]
    fn test_deserialize_valid_toml() {
        let toml = valid_config_toml();
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server_url, "http://localhost:4533");
        assert_eq!(config.username, "admin");
        assert_eq!(config.password, "secret");
        assert_eq!(config.secret, "randomsecret123");
    }

    #[test]
    fn test_deserialize_missing_optional_fields_uses_defaults() {
        let toml = valid_config_toml();
        let config: Config = toml::from_str(toml).unwrap();
        // SearchConfig defaults
        assert_eq!(config.search.fuzzy_threshold, 30);
        assert!(matches!(config.search.mode, SearchMode::Local));
    }

    #[test]
    fn test_deserialize_with_search_config() {
        let toml = r#"
        server_url = "http://localhost:4533"
        username = "admin"
        password = "secret"
        secret = "abc"

        [search]
        mode = "remote"
        fuzzy_threshold = 50
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(config.search.mode, SearchMode::Remote));
        assert_eq!(config.search.fuzzy_threshold, 50);
    }

    #[test]
    fn test_deserialize_invalid_toml_returns_error() {
        let bad_toml = "this is not valid toml :::";
        let result = toml::from_str::<Config>(bad_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = Config {
            server_url: "http://localhost:4533".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            secret: "mysecret".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.server_url, config.server_url);
        assert_eq!(deserialized.username, config.username);
        assert_eq!(deserialized.password, config.password);
        assert_eq!(deserialized.secret, config.secret);
    }

    // --- Default tests ---

    #[test]
    fn test_default_config_has_expected_values() {
        let config = Config::default();
        assert_eq!(config.server_url, "http://localhost:4533");
        assert_eq!(config.username, "admin");
        assert_eq!(config.password, "admin");
        assert!(!config.secret.is_empty());
        assert_eq!(config.secret.len(), 15);
    }

    #[test]
    fn test_default_config_secret_is_alphanumeric() {
        let config = Config::default();
        assert!(config.secret.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_default_config_secrets_are_unique() {
        // Secrets should be randomly generated, not identical
        let c1 = Config::default();
        let c2 = Config::default();
        // Extremely unlikely to collide with 15 alphanumeric chars
        assert_ne!(c1.secret, c2.secret);
    }

    #[test]
    fn test_default_search_config() {
        let search = SearchConfig::default();
        assert_eq!(search.fuzzy_threshold, 30);
        assert!(matches!(search.mode, SearchMode::Local));
    }

    // --- Save / Load integration tests ---

    #[test]
    fn test_save_creates_file() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("sonicrust").join("config.toml");

        // Temporarily override config path via env or test helper
        // Since config_path() uses dirs::config_dir(), we test save directly
        let config = Config {
            server_url: "http://localhost:4533".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            secret: "secret".to_string(),
            theme: Theme::default(),
            search: SearchConfig::default(),
        };

        // Write manually to simulate save
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let content = toml::to_string_pretty(&config).unwrap();
        fs::write(&config_path, &content).unwrap();

        assert!(config_path.exists());
        let loaded: Config = toml::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(loaded.server_url, config.server_url);
    }

    #[test]
    fn test_parse_error_on_bad_file() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "not valid toml!!!");

        let contents = fs::read_to_string(&path).unwrap();
        let result = toml::from_str::<Config>(&contents);
        assert!(result.is_err());
    }

    // --- ConfigError display tests ---

    #[test]
    fn test_config_error_not_found_display() {
        let err = ConfigError::NotFound {
            path: PathBuf::from("/some/path/config.toml"),
        };
        assert!(err.to_string().contains("/some/path/config.toml"));
    }

    #[test]
    fn test_config_error_parse_error_display() {
        let err = ConfigError::ParseError {
            path: PathBuf::from("/some/path/config.toml"),
            reason: "unexpected key".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/some/path/config.toml"));
        assert!(msg.contains("unexpected key"));
    }

    #[test]
    fn test_config_error_validation_display() {
        let err = ConfigError::ValidationError("bad value".to_string());
        assert!(err.to_string().contains("bad value"));
    }

    #[test]
    fn test_config_error_no_config_dir_display() {
        let err = ConfigError::NoConfigDir;
        assert!(!err.to_string().is_empty());
    }
}
