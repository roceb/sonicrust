use anyhow::Result;
use rand::{Rng, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub secret: String,
    pub theme: Theme,
    pub search: SearchConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Theme {
    pub bg: String,
    pub fg: String,
    pub bold: bool,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: "DarkGrey".to_string(),
            fg: "".to_string(),
            bold: true,
        }
    }
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
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // TODO: make a gui to allow people to input their config
            let config = Config::default();
            config.save()?;
            Ok(config)
        } else {
            let contents = fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        }
    }
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }
    fn config_path() -> Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
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
