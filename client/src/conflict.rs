use anyhow::{anyhow, Context, Result};
use serde_json::Value as JsonValue;
use std::path::Path;
use tracing::{debug, info, warn};

/// 冲突类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// 两端都修改
    ModifyModify,
    /// 一端修改，一端删除
    ModifyDelete,
    /// 二进制文件冲突
    BinaryConflict,
}

/// 冲突解决策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStrategy {
    /// 保留本地版本
    KeepLocal,
    /// 保留远程版本
    KeepRemote,
    /// 保留较新的版本
    KeepNewer,
    /// 自动合并
    AutoMerge,
    /// 手动解决
    Manual,
}

/// 合并结果
#[derive(Debug, Clone)]
pub enum MergeResult {
    /// 合并成功
    Merged(String),
    /// 无冲突（不需要合并）
    NoConflict,
    /// 有冲突，无法自动合并
    Conflict(String),
    /// 错误
    Error(String),
}

/// 冲突解决器
pub struct ConflictResolver {
    /// 默认解决策略
    default_strategy: ResolutionStrategy,

    /// 是否自动合并文本文件
    auto_merge_text: bool,

    /// 是否自动合并结构化文件
    auto_merge_structured: bool,
}

impl ConflictResolver {
    /// 创建新的冲突解决器
    pub fn new(
        default_strategy: ResolutionStrategy,
        auto_merge_text: bool,
        auto_merge_structured: bool,
    ) -> Self {
        Self {
            default_strategy,
            auto_merge_text,
            auto_merge_structured,
        }
    }

    /// 解决冲突
    pub fn resolve(
        &self,
        local_path: &Path,
        local_content: &str,
        remote_content: &str,
        base_content: Option<&str>,
        conflict_type: ConflictType,
    ) -> Result<MergeResult> {
        info!("解决冲突: {:?}, 类型: {:?}", local_path, conflict_type);

        match conflict_type {
            ConflictType::ModifyModify => {
                self.resolve_modify_modify(local_path, local_content, remote_content, base_content)
            }
            ConflictType::ModifyDelete => {
                self.resolve_modify_delete(local_path, local_content, remote_content, conflict_type)
            }
            ConflictType::BinaryConflict => Ok(MergeResult::Conflict(
                "二进制文件冲突无法自动解决".to_string(),
            )),
        }
    }

    /// 解决 ModifyModify 冲突
    fn resolve_modify_modify(
        &self,
        path: &Path,
        local_content: &str,
        remote_content: &str,
        base_content: Option<&str>,
    ) -> Result<MergeResult> {
        // 检查文件类型
        let file_type = crate::rules::detect_file_type(path);

        match file_type.as_str() {
            "text" | "md" | "rst" | "txt" => {
                if self.auto_merge_text {
                    self.merge_text(local_content, remote_content, base_content)
                } else {
                    Ok(self.create_conflict_marker(local_content, remote_content))
                }
            }
            "json" => {
                if self.auto_merge_structured {
                    self.merge_json(local_content, remote_content, base_content)
                } else {
                    Ok(self.create_conflict_marker(local_content, remote_content))
                }
            }
            "yaml" | "yml" => {
                if self.auto_merge_structured {
                    self.merge_yaml(local_content, remote_content, base_content)
                } else {
                    Ok(self.create_conflict_marker(local_content, remote_content))
                }
            }
            _ => {
                // 其他文件类型，使用默认策略
                match self.default_strategy {
                    ResolutionStrategy::KeepLocal => {
                        Ok(MergeResult::Merged(local_content.to_string()))
                    }
                    ResolutionStrategy::KeepRemote => {
                        Ok(MergeResult::Merged(remote_content.to_string()))
                    }
                    ResolutionStrategy::Manual => {
                        Ok(self.create_conflict_marker(local_content, remote_content))
                    }
                    _ => Ok(self.create_conflict_marker(local_content, remote_content)),
                }
            }
        }
    }

    /// 解决 ModifyDelete 冲突
    fn resolve_modify_delete(
        &self,
        path: &Path,
        local_content: &str,
        remote_content: &str,
        conflict_type: ConflictType,
    ) -> Result<MergeResult> {
        match self.default_strategy {
            ResolutionStrategy::KeepLocal => Ok(MergeResult::Merged(local_content.to_string())),
            ResolutionStrategy::KeepRemote => {
                if remote_content.is_empty() {
                    // 远程已删除，保留本地
                    Ok(MergeResult::Merged(local_content.to_string()))
                } else {
                    Ok(MergeResult::Merged(remote_content.to_string()))
                }
            }
            _ => Ok(self.create_conflict_marker(local_content, remote_content)),
        }
    }

    /// 合并文本文件（三方合并）
    fn merge_text(&self, local: &str, remote: &str, base: Option<&str>) -> Result<MergeResult> {
        if let Some(base) = base {
            // 简化的三方合并：检查是否有冲突
            let local_differs = base != local;
            let remote_differs = base != remote;
            let local_and_remote_different = local != remote;

            if local_differs && remote_differs && local_and_remote_different {
                // 双方都修改了且修改不同，创建冲突标记
                Ok(self.create_conflict_marker(local, remote))
            } else if local_differs {
                // 只有本地修改
                Ok(MergeResult::Merged(local.to_string()))
            } else if remote_differs {
                // 只有远程修改
                Ok(MergeResult::Merged(remote.to_string()))
            } else {
                // 都没修改
                Ok(MergeResult::NoConflict)
            }
        } else {
            // 没有基线版本，创建冲突标记
            Ok(self.create_conflict_marker(local, remote))
        }
    }

    /// 合并 JSON 文件（结构化合并）
    fn merge_json(&self, local: &str, remote: &str, base: Option<&str>) -> Result<MergeResult> {
        // 解析 JSON
        let local_value: JsonValue = serde_json::from_str(local).context("无法解析本地 JSON")?;
        let remote_value: JsonValue = serde_json::from_str(remote).context("无法解析远程 JSON")?;

        let merged = if let Some(base_str) = base {
            let base_value: JsonValue =
                serde_json::from_str(base_str).context("无法解析基线 JSON")?;
            self.merge_json_values(&base_value, &local_value, &remote_value)?
        } else {
            // 没有基线，尝试递归合并
            self.merge_json_values_without_base(&local_value, &remote_value)?
        };

        // 格式化输出
        let merged_str = serde_json::to_string_pretty(&merged).context("无法序列化合并的 JSON")?;

        Ok(MergeResult::Merged(merged_str))
    }

    /// 递归合并 JSON 值（有基线）
    fn merge_json_values(
        &self,
        base: &JsonValue,
        local: &JsonValue,
        remote: &JsonValue,
    ) -> Result<JsonValue> {
        match (base, local, remote) {
            // 都是对象，递归合并
            (
                JsonValue::Object(base_map),
                JsonValue::Object(local_map),
                JsonValue::Object(remote_map),
            ) => {
                let mut merged = serde_json::Map::new();

                // 收集所有键
                let all_keys: std::collections::HashSet<_> = base_map
                    .keys()
                    .chain(local_map.keys())
                    .chain(remote_map.keys())
                    .collect();

                for key in all_keys {
                    let base_value = base_map.get(key);
                    let local_value = local_map.get(key);
                    let remote_value = remote_map.get(key);

                    match (base_value, local_value, remote_value) {
                        // 三者都存在且相同
                        (_, Some(l), Some(r)) if l == r => {
                            merged.insert(key.clone(), l.clone());
                        }
                        // 三者都存在且不同，递归合并
                        (Some(b), Some(l), Some(r)) => {
                            let merged_value = self.merge_json_values(b, l, r)?;
                            merged.insert(key.clone(), merged_value);
                        }
                        // 基线没有，但本地和远程都有（需要选择策略，这里使用本地）
                        (None, Some(l), Some(_r)) => {
                            merged.insert(key.clone(), l.clone());
                        }
                        // 只有本地有
                        (_, Some(l), None) => {
                            merged.insert(key.clone(), l.clone());
                        }
                        // 只有远程有
                        (_, None, Some(r)) => {
                            merged.insert(key.clone(), r.clone());
                        }
                        // 都没有，跳过
                        (_, None, None) => {}
                    }
                }

                Ok(JsonValue::Object(merged))
            }
            // 都是数组，使用远程版本（需要领域知识）
            (JsonValue::Array(_), JsonValue::Array(_), JsonValue::Array(remote)) => {
                Ok(JsonValue::Array(remote.clone()))
            }
            // 其他类型，使用本地版本
            _ => Ok(local.clone()),
        }
    }

    /// 递归合并 JSON 值（无基线）
    fn merge_json_values_without_base(
        &self,
        local: &JsonValue,
        remote: &JsonValue,
    ) -> Result<JsonValue> {
        match (local, remote) {
            // 都是对象，递归合并
            (JsonValue::Object(local_map), JsonValue::Object(remote_map)) => {
                let mut merged = local_map.clone();

                for (key, remote_value) in remote_map {
                    if let Some(local_value) = local_map.get(key.as_str()) {
                        // 键都存在，递归合并
                        let merged_value =
                            self.merge_json_values_without_base(local_value, remote_value)?;
                        merged.insert(key.clone(), merged_value);
                    } else {
                        // 只有远程有
                        merged.insert(key.clone(), remote_value.clone());
                    }
                }

                Ok(JsonValue::Object(merged))
            }
            // 其他类型，使用本地版本
            _ => Ok(local.clone()),
        }
    }

    /// 合并 YAML 文件（转换为 JSON 后合并）
    fn merge_yaml(&self, local: &str, remote: &str, base: Option<&str>) -> Result<MergeResult> {
        // 解析 YAML
        let local_value: JsonValue = serde_yaml::from_str(local).context("无法解析本地 YAML")?;
        let remote_value: JsonValue = serde_yaml::from_str(remote).context("无法解析远程 YAML")?;

        let merged = if let Some(base_str) = base {
            let base_value: JsonValue =
                serde_yaml::from_str(base_str).context("无法解析基线 YAML")?;
            self.merge_json_values(&base_value, &local_value, &remote_value)?
        } else {
            self.merge_json_values_without_base(&local_value, &remote_value)?
        };

        // 格式化输出为 YAML
        let merged_str = serde_yaml::to_string(&merged).context("无法序列化合并的 YAML")?;

        Ok(MergeResult::Merged(merged_str))
    }

    /// 创建冲突标记（Git 风格）
    fn create_conflict_marker(&self, local: &str, remote: &str) -> MergeResult {
        let conflict = format!(
            "<<<<<<< LOCAL\n{}\n=======\n{}\n>>>>>>> REMOTE",
            local, remote
        );

        MergeResult::Conflict(conflict)
    }

    /// 应用默认策略
    pub fn apply_default_strategy(&self, local_content: &str, remote_content: &str) -> MergeResult {
        match self.default_strategy {
            ResolutionStrategy::KeepLocal => MergeResult::Merged(local_content.to_string()),
            ResolutionStrategy::KeepRemote => MergeResult::Merged(remote_content.to_string()),
            _ => self.create_conflict_marker(local_content, remote_content),
        }
    }
}

/// 文件类型检测器
pub struct FileTypeDetector;

impl FileTypeDetector {
    /// 检测文件类型
    pub fn detect(path: &Path) -> String {
        crate::rules::detect_file_type(path)
    }

    /// 检查是否是文本文件
    pub fn is_text_file(path: &Path) -> bool {
        crate::rules::is_text_file(path)
    }

    /// 检查是否是二进制文件
    pub fn is_binary_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            matches!(
                ext_str.as_str(),
                "exe" | "dll" | "so" | "dylib" | "png" | "jpg" | "jpeg" | "gif" | "pdf" | "zip"
            )
        } else {
            false
        }
    }

    /// 检查是否是配置文件
    pub fn is_config_file(path: &Path) -> bool {
        crate::rules::is_config_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_conflict_marker() {
        let resolver = ConflictResolver::new(ResolutionStrategy::Manual, true, true);
        let result = resolver.create_conflict_marker("local content", "remote content");

        match result {
            MergeResult::Conflict(markers) => {
                assert!(markers.contains("<<<<<<< LOCAL"));
                assert!(markers.contains("local content"));
                assert!(markers.contains("======="));
                assert!(markers.contains("remote content"));
                assert!(markers.contains(">>>>>>> REMOTE"));
            }
            _ => panic!("Expected Conflict result"),
        }
    }

    #[test]
    fn test_merge_json_no_conflict() {
        let resolver = ConflictResolver::new(ResolutionStrategy::Manual, true, true);

        let local = r#"{"name": "test", "value": 1}"#;
        let remote = r#"{"name": "test", "value": 2}"#;
        let base = r#"{"name": "test", "value": 0}"#;

        let result = resolver.merge_json(local, remote, Some(base));

        match result {
            Ok(MergeResult::Merged(merged)) => {
                assert!(merged.contains("test"));
            }
            _ => panic!("Expected Merged result"),
        }
    }

    #[test]
    fn test_merge_json_without_base() {
        let resolver = ConflictResolver::new(ResolutionStrategy::Manual, true, true);

        let local = r#"{"name": "test", "local_key": "local"}"#;
        let remote = r#"{"name": "test", "remote_key": "remote"}"#;

        let result = resolver.merge_json(local, remote, None);

        match result {
            Ok(MergeResult::Merged(merged)) => {
                assert!(merged.contains("local_key"));
                assert!(merged.contains("remote_key"));
            }
            _ => panic!("Expected Merged result"),
        }
    }

    #[test]
    fn test_file_type_detection() {
        assert!(FileTypeDetector::is_text_file(Path::new("test.md")));
        assert!(FileTypeDetector::is_text_file(Path::new("test.json")));
        assert!(!FileTypeDetector::is_text_file(Path::new("test.exe")));

        assert!(FileTypeDetector::is_binary_file(Path::new("test.exe")));
        assert!(FileTypeDetector::is_binary_file(Path::new("test.png")));
        assert!(!FileTypeDetector::is_binary_file(Path::new("test.md")));
    }
}
