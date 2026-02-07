use crate::error::MicroClawError;
use serde::{Deserialize, Serialize};
use tracing::warn;

fn default_telegram_bot_token() -> String {
    String::new()
}
fn default_bot_username() -> String {
    String::new()
}
fn default_llm_provider() -> String {
    "anthropic".into()
}
fn default_api_key() -> String {
    String::new()
}
fn default_model() -> String {
    String::new()
}
fn default_max_tokens() -> u32 {
    8192
}
fn default_max_tool_iterations() -> usize {
    25
}
fn default_max_history_messages() -> usize {
    50
}
fn default_data_dir() -> String {
    "./data".into()
}
fn default_timezone() -> String {
    "UTC".into()
}
fn default_max_session_messages() -> usize {
    40
}
fn default_compact_keep_recent() -> usize {
    20
}
fn default_whatsapp_webhook_port() -> u16 {
    8080
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_telegram_bot_token")]
    pub telegram_bot_token: String,
    #[serde(default = "default_bot_username")]
    pub bot_username: String,
    #[serde(default = "default_llm_provider")]
    pub llm_provider: String,
    #[serde(default = "default_api_key")]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub llm_base_url: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_max_tool_iterations")]
    pub max_tool_iterations: usize,
    #[serde(default = "default_max_history_messages")]
    pub max_history_messages: usize,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default)]
    pub openai_api_key: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub allowed_groups: Vec<i64>,
    #[serde(default = "default_max_session_messages")]
    pub max_session_messages: usize,
    #[serde(default = "default_compact_keep_recent")]
    pub compact_keep_recent: usize,
    #[serde(default)]
    pub whatsapp_access_token: Option<String>,
    #[serde(default)]
    pub whatsapp_phone_number_id: Option<String>,
    #[serde(default)]
    pub whatsapp_verify_token: Option<String>,
    #[serde(default = "default_whatsapp_webhook_port")]
    pub whatsapp_webhook_port: u16,
    #[serde(default)]
    pub discord_bot_token: Option<String>,
    #[serde(default)]
    pub discord_allowed_channels: Vec<u64>,
}

impl Config {
    /// Load config from YAML file, with fallback to env vars for backward compatibility.
    pub fn load() -> Result<Self, MicroClawError> {
        // 1. Check MICROCLAW_CONFIG env var for custom path
        let yaml_path = if let Ok(custom) = std::env::var("MICROCLAW_CONFIG") {
            if std::path::Path::new(&custom).exists() {
                Some(custom)
            } else {
                return Err(MicroClawError::Config(format!(
                    "MICROCLAW_CONFIG points to non-existent file: {custom}"
                )));
            }
        } else if std::path::Path::new("./config.yaml").exists() {
            Some("./config.yaml".into())
        } else if std::path::Path::new("./config.yml").exists() {
            Some("./config.yml".into())
        } else {
            None
        };

        if let Some(path) = yaml_path {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| MicroClawError::Config(format!("Failed to read {path}: {e}")))?;
            let mut config: Config = serde_yaml::from_str(&content)
                .map_err(|e| MicroClawError::Config(format!("Failed to parse {path}: {e}")))?;
            config.post_deserialize()?;
            return Ok(config);
        }

        // Backward compat: try loading from env vars if .env exists
        if std::path::Path::new("./.env").exists() {
            warn!("Loading from .env is deprecated. Please migrate to config.yaml (run `microclaw setup`).");
            return Self::from_env();
        }

        // No config file found at all
        Err(MicroClawError::Config(
            "No config.yaml found. Run `microclaw setup` to create one.".into(),
        ))
    }

    /// Apply post-deserialization normalization and validation.
    fn post_deserialize(&mut self) -> Result<(), MicroClawError> {
        self.llm_provider = self.llm_provider.trim().to_lowercase();

        // Apply provider-specific default model if empty
        if self.model.is_empty() {
            self.model = match self.llm_provider.as_str() {
                "anthropic" => "claude-sonnet-4-20250514".into(),
                _ => "gpt-4o".into(),
            };
        }

        // Validate timezone
        self.timezone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| MicroClawError::Config(format!("Invalid timezone: {}", self.timezone)))?;

        // Filter empty llm_base_url
        if let Some(ref url) = self.llm_base_url {
            if url.trim().is_empty() {
                self.llm_base_url = None;
            }
        }

        // Validate required fields
        if self.telegram_bot_token.is_empty() && self.discord_bot_token.is_none() {
            return Err(MicroClawError::Config(
                "At least one of telegram_bot_token or discord_bot_token must be set".into(),
            ));
        }
        if self.api_key.is_empty() {
            return Err(MicroClawError::Config("api_key is required".into()));
        }

        Ok(())
    }

    /// Save config as YAML to the given path.
    #[allow(dead_code)]
    pub fn save_yaml(&self, path: &str) -> Result<(), MicroClawError> {
        let content = serde_yaml::to_string(self)
            .map_err(|e| MicroClawError::Config(format!("Failed to serialize config: {e}")))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Legacy: load from environment variables (.env file).
    fn from_env() -> Result<Self, MicroClawError> {
        // Load .env file into process env
        if let Ok(content) = std::fs::read_to_string(".env") {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = trimmed.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if std::env::var(key).is_err() {
                        std::env::set_var(key, value);
                    }
                }
            }
        }

        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| MicroClawError::Config("TELEGRAM_BOT_TOKEN not set".into()))?;
        let bot_username = std::env::var("BOT_USERNAME")
            .map_err(|_| MicroClawError::Config("BOT_USERNAME not set".into()))?;

        let llm_provider = std::env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "anthropic".into())
            .trim()
            .to_lowercase();

        let api_key = std::env::var("LLM_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .map_err(|_| {
                MicroClawError::Config("LLM_API_KEY (or ANTHROPIC_API_KEY) not set".into())
            })?;

        let default_model = match llm_provider.as_str() {
            "anthropic" => "claude-sonnet-4-20250514",
            _ => "gpt-4o",
        };
        let model = std::env::var("LLM_MODEL")
            .or_else(|_| std::env::var("CLAUDE_MODEL"))
            .unwrap_or_else(|_| default_model.into());

        let llm_base_url = std::env::var("LLM_BASE_URL").ok().filter(|s| !s.is_empty());

        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".into());
        let max_tokens = std::env::var("MAX_TOKENS")
            .unwrap_or_else(|_| "8192".into())
            .parse::<u32>()
            .map_err(|e| MicroClawError::Config(format!("Invalid MAX_TOKENS: {e}")))?;
        let max_tool_iterations = std::env::var("MAX_TOOL_ITERATIONS")
            .unwrap_or_else(|_| "25".into())
            .parse::<usize>()
            .map_err(|e| MicroClawError::Config(format!("Invalid MAX_TOOL_ITERATIONS: {e}")))?;
        let max_history_messages = std::env::var("MAX_HISTORY_MESSAGES")
            .unwrap_or_else(|_| "50".into())
            .parse::<usize>()
            .map_err(|e| MicroClawError::Config(format!("Invalid MAX_HISTORY_MESSAGES: {e}")))?;

        let openai_api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());

        let timezone = std::env::var("TIMEZONE").unwrap_or_else(|_| "UTC".into());
        timezone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| MicroClawError::Config(format!("Invalid TIMEZONE: {timezone}")))?;

        let max_session_messages = std::env::var("MAX_SESSION_MESSAGES")
            .unwrap_or_else(|_| "40".into())
            .parse::<usize>()
            .map_err(|e| MicroClawError::Config(format!("Invalid MAX_SESSION_MESSAGES: {e}")))?;
        let compact_keep_recent = std::env::var("COMPACT_KEEP_RECENT")
            .unwrap_or_else(|_| "20".into())
            .parse::<usize>()
            .map_err(|e| MicroClawError::Config(format!("Invalid COMPACT_KEEP_RECENT: {e}")))?;

        let whatsapp_access_token = std::env::var("WHATSAPP_ACCESS_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        let whatsapp_phone_number_id = std::env::var("WHATSAPP_PHONE_NUMBER_ID")
            .ok()
            .filter(|s| !s.is_empty());
        let whatsapp_verify_token = std::env::var("WHATSAPP_VERIFY_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        let whatsapp_webhook_port = std::env::var("WHATSAPP_WEBHOOK_PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse::<u16>()
            .map_err(|e| MicroClawError::Config(format!("Invalid WHATSAPP_WEBHOOK_PORT: {e}")))?;

        let allowed_groups = std::env::var("ALLOWED_GROUPS")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                s.trim().parse::<i64>().map_err(|e| {
                    MicroClawError::Config(format!("Invalid ALLOWED_GROUPS entry '{s}': {e}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let discord_bot_token = std::env::var("DISCORD_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());

        let discord_allowed_channels = std::env::var("DISCORD_ALLOWED_CHANNELS")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect();

        Ok(Config {
            telegram_bot_token,
            bot_username,
            llm_provider,
            api_key,
            model,
            llm_base_url,
            max_tokens,
            max_tool_iterations,
            max_history_messages,
            data_dir,
            openai_api_key,
            timezone,
            allowed_groups,
            max_session_messages,
            compact_keep_recent,
            whatsapp_access_token,
            whatsapp_phone_number_id,
            whatsapp_verify_token,
            whatsapp_webhook_port,
            discord_bot_token,
            discord_allowed_channels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn test_config() -> Config {
        Config {
            telegram_bot_token: "tok".into(),
            bot_username: "bot".into(),
            llm_provider: "anthropic".into(),
            api_key: "key".into(),
            model: "claude-sonnet-4-20250514".into(),
            llm_base_url: None,
            max_tokens: 8192,
            max_tool_iterations: 25,
            max_history_messages: 50,
            data_dir: "./data".into(),
            openai_api_key: None,
            timezone: "UTC".into(),
            allowed_groups: vec![],
            max_session_messages: 40,
            compact_keep_recent: 20,
            whatsapp_access_token: None,
            whatsapp_phone_number_id: None,
            whatsapp_verify_token: None,
            whatsapp_webhook_port: 8080,
            discord_bot_token: None,
            discord_allowed_channels: vec![],
        }
    }

    #[test]
    fn test_config_struct_clone_and_debug() {
        let config = test_config();
        let cloned = config.clone();
        assert_eq!(cloned.telegram_bot_token, "tok");
        assert_eq!(cloned.max_tokens, 8192);
        assert_eq!(cloned.max_tool_iterations, 25);
        assert_eq!(cloned.max_history_messages, 50);
        assert!(cloned.openai_api_key.is_none());
        assert_eq!(cloned.timezone, "UTC");
        assert!(cloned.allowed_groups.is_empty());
        assert_eq!(cloned.max_session_messages, 40);
        assert_eq!(cloned.compact_keep_recent, 20);
        assert!(cloned.discord_bot_token.is_none());
        assert!(cloned.discord_allowed_channels.is_empty());
        let _ = format!("{:?}", config);
    }

    #[test]
    fn test_config_default_values() {
        let mut config = test_config();
        config.openai_api_key = Some("sk-test".into());
        config.timezone = "US/Eastern".into();
        config.allowed_groups = vec![123, 456];
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.data_dir, "./data");
        assert_eq!(config.openai_api_key.as_deref(), Some("sk-test"));
        assert_eq!(config.timezone, "US/Eastern");
        assert_eq!(config.allowed_groups, vec![123, 456]);
    }

    #[test]
    fn test_config_yaml_roundtrip() {
        let config = test_config();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.telegram_bot_token, "tok");
        assert_eq!(parsed.max_tokens, 8192);
        assert_eq!(parsed.llm_provider, "anthropic");
    }

    #[test]
    fn test_config_yaml_defaults() {
        let yaml = "telegram_bot_token: tok\nbot_username: bot\napi_key: key\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.llm_provider, "anthropic");
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.max_tool_iterations, 25);
        assert_eq!(config.data_dir, "./data");
        assert_eq!(config.timezone, "UTC");
    }

    #[test]
    fn test_config_post_deserialize() {
        let yaml = "telegram_bot_token: tok\nbot_username: bot\napi_key: key\nllm_provider: ANTHROPIC\n";
        let mut config: Config = serde_yaml::from_str(yaml).unwrap();
        config.post_deserialize().unwrap();
        assert_eq!(config.llm_provider, "anthropic");
        assert_eq!(config.model, "claude-sonnet-4-20250514");
    }
}
