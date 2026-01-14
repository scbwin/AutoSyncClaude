use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

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
        // TODO: 检查 token 是否存在
        Ok(false)
    }

    pub async fn get_user_id(&self) -> Result<String> {
        // TODO: 从 token 中获取用户 ID
        Ok(String::new())
    }

    pub async fn get_device_id(&self) -> Result<String> {
        // TODO: 从 token 中获取设备 ID
        Ok(String::new())
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

        config["sync"]["rules"]
            .as_array_mut()
            .unwrap_or(&mut vec![])
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
