use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::debug;

/// 同步规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRule {
    /// 规则 ID
    pub id: String,

    /// 规则名称
    pub name: String,

    /// 规则类型（include/exclude）
    pub rule_type: RuleType,

    /// 文件模式（Glob 或正则表达式）
    pub pattern: String,

    /// 模式类型（glob/regex）
    pub pattern_type: PatternType,

    /// 文件类型（可选）
    pub file_type: Option<String>,

    /// 优先级（数字越大优先级越高）
    pub priority: i32,

    /// 是否启用
    pub enabled: bool,

    /// 描述（可选）
    pub description: Option<String>,
}

/// 规则类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleType {
    /// 包含规则
    Include,
    /// 排除规则
    Exclude,
}

/// 模式类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatternType {
    /// Glob 模式
    Glob,
    /// 正则表达式
    Regex,
}

/// 规则引擎
pub struct RuleEngine {
    /// 规则列表
    rules: Vec<SyncRule>,
}

impl RuleEngine {
    /// 创建新的规则引擎
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 从规则列表创建
    pub fn from_rules(rules: Vec<SyncRule>) -> Self {
        // 按优先级排序（从高到低）
        let mut sorted_rules = rules;
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        Self {
            rules: sorted_rules,
        }
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: SyncRule) {
        self.rules.push(rule);
        // 重新排序
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 移除规则
    pub fn remove_rule(&mut self, rule_id: &str) -> bool {
        if let Some(pos) = self.rules.iter().position(|r| r.id == rule_id) {
            self.rules.remove(pos);
            true
        } else {
            false
        }
    }

    /// 获取所有规则
    pub fn get_rules(&self) -> &[SyncRule] {
        &self.rules
    }

    /// 清空所有规则
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// 应用规则判断是否应该同步文件
    pub fn should_sync(&self, path: &Path, file_type: Option<&str>) -> bool {
        let mut should_sync = true; // 默认同步
        let mut highest_priority = i32::MIN;

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            // 检查文件类型
            if let Some(ref rule_file_type) = rule.file_type {
                if let Some(ft) = file_type {
                    if rule_file_type != ft {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // 检查模式匹配
            let matches = self.match_pattern(&rule.pattern_type, &rule.pattern, path);

            if matches {
                // 优先级更高的规则会覆盖之前的规则
                if rule.priority > highest_priority {
                    should_sync = match rule.rule_type {
                        RuleType::Include => true,
                        RuleType::Exclude => false,
                    };
                    highest_priority = rule.priority;

                    debug!(
                        "规则匹配: {:?} (规则: {}, 优先级: {}) -> {}",
                        path,
                        rule.name,
                        rule.priority,
                        if should_sync { "同步" } else { "跳过" }
                    );
                }
            }
        }

        should_sync
    }

    /// 匹配模式
    fn match_pattern(&self, pattern_type: &PatternType, pattern: &str, path: &Path) -> bool {
        match pattern_type {
            PatternType::Glob => {
                if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                    glob_pattern.matches_path(path)
                } else {
                    debug!("无效的 Glob 模式: {}", pattern);
                    false
                }
            }
            PatternType::Regex => {
                if let Ok(re) = regex::Regex::new(pattern) {
                    let path_str = path.to_string_lossy();
                    re.is_match(&path_str)
                } else {
                    debug!("无效的正则表达式: {}", pattern);
                    false
                }
            }
        }
    }

    /// 验证规则
    pub fn validate_rule(rule: &SyncRule) -> Result<()> {
        // 验证规则 ID
        if rule.id.is_empty() {
            anyhow::bail!("规则 ID 不能为空");
        }

        // 验证规则名称
        if rule.name.is_empty() {
            anyhow::bail!("规则名称不能为空");
        }

        // 验证模式
        if rule.pattern.is_empty() {
            anyhow::bail!("规则模式不能为空");
        }

        // 验证模式格式
        match rule.pattern_type {
            PatternType::Glob => {
                glob::Pattern::new(&rule.pattern)
                    .with_context(|| format!("无效的 Glob 模式: {}", rule.pattern))?;
            }
            PatternType::Regex => {
                regex::Regex::new(&rule.pattern)
                    .with_context(|| format!("无效的正则表达式: {}", rule.pattern))?;
            }
        }

        // 验证优先级范围
        if rule.priority < -100 || rule.priority > 100 {
            anyhow::bail!("规则优先级必须在 -100 到 100 之间");
        }

        Ok(())
    }

    /// 获取默认规则集
    pub fn default_rules() -> Vec<SyncRule> {
        vec![SyncRule {
            id: "default-include-all".to_string(),
            name: "默认包含所有文件".to_string(),
            rule_type: RuleType::Include,
            pattern: "**".to_string(),
            pattern_type: PatternType::Glob,
            file_type: None,
            priority: -100,
            enabled: true,
            description: Some("默认包含所有文件的兜底规则".to_string()),
        }]
    }

    /// 获取推荐的规则集
    pub fn recommended_rules() -> Vec<SyncRule> {
        vec![
            // 包含规则
            SyncRule {
                id: "include-agents".to_string(),
                name: "包含 agents 目录".to_string(),
                rule_type: RuleType::Include,
                pattern: "agents/**".to_string(),
                pattern_type: PatternType::Glob,
                file_type: Some("md".to_string()),
                priority: 50,
                enabled: true,
                description: Some("同步 agents 目录中的 Markdown 文件".to_string()),
            },
            SyncRule {
                id: "include-skills".to_string(),
                name: "包含 skills 目录".to_string(),
                rule_type: RuleType::Include,
                pattern: "skills/**".to_string(),
                pattern_type: PatternType::Glob,
                file_type: Some("md".to_string()),
                priority: 50,
                enabled: true,
                description: Some("同步 skills 目录中的 Markdown 文件".to_string()),
            },
            SyncRule {
                id: "include-plugins".to_string(),
                name: "包含 plugins 配置".to_string(),
                rule_type: RuleType::Include,
                pattern: "plugins/**".to_string(),
                pattern_type: PatternType::Glob,
                file_type: None,
                priority: 50,
                enabled: true,
                description: Some("同步 plugins 目录的所有文件".to_string()),
            },
            SyncRule {
                id: "include-config".to_string(),
                name: "包含配置文件".to_string(),
                rule_type: RuleType::Include,
                pattern: "*.{json,toml,yaml,yml}".to_string(),
                pattern_type: PatternType::Glob,
                file_type: None,
                priority: 60,
                enabled: true,
                description: Some("同步配置文件".to_string()),
            },
            // 排除规则
            SyncRule {
                id: "exclude-temp".to_string(),
                name: "排除临时文件".to_string(),
                rule_type: RuleType::Exclude,
                pattern: "*.tmp".to_string(),
                pattern_type: PatternType::Glob,
                file_type: None,
                priority: 100,
                enabled: true,
                description: Some("排除临时文件".to_string()),
            },
            SyncRule {
                id: "exclude-backup".to_string(),
                name: "排除备份文件".to_string(),
                rule_type: RuleType::Exclude,
                pattern: "*.bak".to_string(),
                pattern_type: PatternType::Glob,
                file_type: None,
                priority: 100,
                enabled: true,
                description: Some("排除备份文件".to_string()),
            },
            SyncRule {
                id: "exclude-swap".to_string(),
                name: "排除交换文件".to_string(),
                rule_type: RuleType::Exclude,
                pattern: ".*.swp".to_string(),
                pattern_type: PatternType::Glob,
                file_type: None,
                priority: 100,
                enabled: true,
                description: Some("排除 Vim 交换文件".to_string()),
            },
        ]
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ===== 辅助函数 =====

/// 识别文件类型
pub fn detect_file_type(path: &Path) -> String {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();

        match ext_str.as_str() {
            "md" | "rst" | "txt" => "text".to_string(),
            "json" => "json".to_string(),
            "yaml" | "yml" => "yaml".to_string(),
            "toml" => "toml".to_string(),
            "xml" => "xml".to_string(),
            "exe" | "dll" | "so" | "dylib" => "binary".to_string(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" => "image".to_string(),
            "pdf" => "pdf".to_string(),
            "zip" | "tar" | "gz" | "rar" | "7z" => "archive".to_string(),
            _ => ext_str,
        }
    } else {
        "unknown".to_string()
    }
}

/// 检查文件是否是文本文件
pub fn is_text_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();

        matches!(
            ext_str.as_str(),
            "md" | "rst" | "txt" | "json" | "yaml" | "yml" | "toml" | "xml"
        )
    } else {
        false
    }
}

/// 检查文件是否是配置文件
pub fn is_config_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();

        matches!(ext_str.as_str(), "json" | "yaml" | "yml" | "toml" | "xml")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_rule_engine() {
        let mut engine = RuleEngine::new();

        // 添加包含规则
        engine.add_rule(SyncRule {
            id: "include-md".to_string(),
            name: "包含 Markdown".to_string(),
            rule_type: RuleType::Include,
            pattern: "*.md".to_string(),
            pattern_type: PatternType::Glob,
            file_type: Some("text".to_string()),
            priority: 0,
            enabled: true,
            description: None,
        });

        // 添加排除规则（优先级更高）
        engine.add_rule(SyncRule {
            id: "exclude-temp".to_string(),
            name: "排除临时文件".to_string(),
            rule_type: RuleType::Exclude,
            pattern: "*-temp.md".to_string(),
            pattern_type: PatternType::Glob,
            file_type: Some("text".to_string()),
            priority: 10,
            enabled: true,
            description: None,
        });

        // 测试包含规则
        let test_path = PathBuf::from("test.md");
        assert!(engine.should_sync(&test_path, Some("text")));

        // 测试排除规则（优先级更高）
        let temp_path = PathBuf::from("test-temp.md");
        assert!(!engine.should_sync(&temp_path, Some("text")));
    }

    #[test]
    fn test_detect_file_type() {
        assert_eq!(detect_file_type(Path::new("test.md")), "text");
        assert_eq!(detect_file_type(Path::new("test.json")), "json");
        assert_eq!(detect_file_type(Path::new("test.yaml")), "yaml");
        assert_eq!(detect_file_type(Path::new("test.exe")), "binary");
        assert_eq!(detect_file_type(Path::new("test.png")), "image");
    }

    #[test]
    fn test_is_text_file() {
        assert!(is_text_file(Path::new("test.md")));
        assert!(is_text_file(Path::new("test.json")));
        assert!(!is_text_file(Path::new("test.exe")));
        assert!(!is_text_file(Path::new("test.png")));
    }

    #[test]
    fn test_validate_rule() {
        let valid_rule = SyncRule {
            id: "test-rule".to_string(),
            name: "测试规则".to_string(),
            rule_type: RuleType::Include,
            pattern: "*.md".to_string(),
            pattern_type: PatternType::Glob,
            file_type: None,
            priority: 0,
            enabled: true,
            description: None,
        };

        assert!(RuleEngine::validate_rule(&valid_rule).is_ok());

        // 测试无效模式
        let invalid_rule = SyncRule {
            pattern: "[invalid".to_string(),
            pattern_type: PatternType::Regex,
            ..valid_rule.clone()
        };

        assert!(RuleEngine::validate_rule(&invalid_rule).is_err());
    }

    #[test]
    fn test_recommended_rules() {
        let rules = RuleEngine::recommended_rules();
        assert!(!rules.is_empty());

        let engine = RuleEngine::from_rules(rules);

        // 测试 agents 目录
        let agent_path = PathBuf::from("agents/test-agent.md");
        assert!(engine.should_sync(&agent_path, Some("md")));

        // 测试临时文件排除
        let temp_path = PathBuf::from("test.tmp");
        assert!(!engine.should_sync(&temp_path, None));
    }
}
