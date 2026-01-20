use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// 认证 Token 信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub device_id: String,
    pub expires_at: i64,
}

pub struct ConfigManager {
    config_dir: PathBuf,
    config_file: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_dir = Self::config_dir();
        let config_file = config_dir.join("config.json");

        Self {
            config_dir,
            config_file,
        }
    }

    pub fn config_file_path(&self) -> PathBuf {
        self.config_file.clone()
    }

    fn config_dir() -> PathBuf {
        // Windows: C:\Users\Username\AppData\Roaming\claude-sync-gui
        // Linux/Mac: ~/.config/claude-sync-gui
        if cfg!(windows) {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("claude-sync-gui")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("claude-sync-gui")
        }
    }

    pub async fn init_config(&self) -> Result<Value> {
        // 确保配置目录存在
        fs::create_dir_all(&self.config_dir).await?;

        let default_config = serde_json::json!({
            "server": {
                "address": "http://localhost:50051",
                "health_check_address": "http://localhost:8080",
                "timeout": 30
            },
            "sync": {
                "claude_dir": dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".claude")
                    .to_string_lossy()
                    .to_string(),
                "interval": 60,
                "auto_start": false,
                "exclude_patterns": ["cache/**", "tmp/**", "*.tmp"]
            },
            "ui": {
                "theme": "system",
                "language": "zh-CN",
                "minimize_to_tray": true,
                "show_notifications": true
            }
        });

        // 保存默认配置
        let content = serde_json::to_string_pretty(&default_config)?;
        fs::write(&self.config_file, content).await?;

        Ok(default_config)
    }

    pub async fn get_config(&self) -> Result<Value> {
        if !self.config_file.exists() {
            return self.init_config().await;
        }

        let content = fs::read_to_string(&self.config_file).await?;
        let config: Value = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub async fn update_config(&self, config: Value) -> Result<()> {
        let content = serde_json::to_string_pretty(&config)?;
        fs::write(&self.config_file, content).await?;
        Ok(())
    }

    pub async fn is_logged_in(&self) -> Result<bool> {
        let tokens = self.get_auth_tokens().await?;
        Ok(tokens.is_some())
    }

    pub async fn get_user_id(&self) -> Result<String> {
        let tokens = self.get_auth_tokens().await?;
        Ok(tokens.map(|t| t.user_id).unwrap_or_default())
    }

    pub async fn get_device_id(&self) -> Result<String> {
        let tokens = self.get_auth_tokens().await?;
        Ok(tokens.map(|t| t.device_id).unwrap_or_default())
    }

    /// 保存认证 Token
    pub async fn save_auth_tokens(&self, tokens: AuthTokens) -> Result<()> {
        let mut config = self.get_config().await?;
        config["auth"] = serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "user_id": tokens.user_id,
            "device_id": tokens.device_id,
            "expires_at": tokens.expires_at
        });
        self.update_config(config).await
    }

    /// 获取认证 Token
    pub async fn get_auth_tokens(&self) -> Result<Option<AuthTokens>> {
        let config = self.get_config().await?;
        if let Some(auth) = config.get("auth") {
            let access_token = auth["access_token"].as_str().unwrap_or("").to_string();
            if access_token.is_empty() {
                return Ok(None);
            }
            Ok(Some(AuthTokens {
                access_token,
                refresh_token: auth["refresh_token"].as_str().unwrap_or("").to_string(),
                user_id: auth["user_id"].as_str().unwrap_or("").to_string(),
                device_id: auth["device_id"].as_str().unwrap_or("").to_string(),
                expires_at: auth["expires_at"].as_i64().unwrap_or(0),
            }))
        } else {
            Ok(None)
        }
    }

    /// 清除认证 Token
    pub async fn clear_auth_tokens(&self) -> Result<()> {
        let mut config = self.get_config().await?;
        config["auth"] = serde_json::json!({});
        self.update_config(config).await
    }

    pub async fn get_rules(&self) -> Result<Vec<Value>> {
        let config = self.get_config().await?;
        let rules = config["sync"]["rules"]
            .as_array()
            .unwrap_or(&vec![])
            .clone();
        Ok(rules)
    }

    pub async fn add_rule(
        &self,
        name: String,
        rule_type: String,
        pattern: String,
        file_type: Option<String>,
        priority: i32,
    ) -> Result<()> {
        let mut config = self.get_config().await?;

        let rule = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "name": name,
            "type": rule_type,
            "pattern": pattern,
            "file_type": file_type,
            "priority": priority,
            "enabled": true
        });

        // 确保 sync.rules 数组存在
        if config["sync"]["rules"].is_null() {
            config["sync"]["rules"] = serde_json::json!([]);
        }

        config["sync"]["rules"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("rules is not an array"))?
            .push(rule);

        self.update_config(config).await?;
        Ok(())
    }

    pub async fn remove_rule(&self, rule_id: String) -> Result<()> {
        let mut config = self.get_config().await?;

        if let Some(rules) = config["sync"]["rules"].as_array_mut() {
            rules.retain(|rule| rule["id"] != rule_id);
        }

        self.update_config(config).await?;
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
