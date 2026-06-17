//! v1.3 LLM 抽象扩展：扫目录加载 ProviderConfig
//!
//! 规则（spec A §4.2）：
//! - 目录不存在 → 返回空 Vec
//! - load_all 时单文件解析失败 → 跳过 + warn，不中断
//! - load_one 时单文件解析失败 → 返回 Err
//! - id 重复 → 保留第一个 + warn
//! - `${ENV_VAR}` 展开复用 common::env::expand_env_vars

use std::path::Path;

use crate::common::env::expand_env_vars;
use crate::common::error::Result;
use crate::infrastructure::llm::types::ProviderConfig;

pub struct PresetLoader;

impl PresetLoader {
    /// 扫描 `dir/*.yaml`，返回所有 ProviderConfig
    ///
    /// - 目录不存在 → `Ok(vec![])`
    /// - 单文件解析失败 → 跳过 + `tracing::warn!`，不中断
    /// - id 重复 → 保留第一个 + warn
    pub fn load_all(dir: &Path) -> Result<Vec<ProviderConfig>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut presets = Vec::new();
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        let entries = std::fs::read_dir(dir).map_err(crate::common::error::EflowError::Io)?;
        // 按文件名排序保证确定性顺序（便于测试）
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "yaml" || s == "yml")
                    .unwrap_or(false)
            })
            .collect();
        paths.sort();

        for path in paths {
            match Self::load_one(&path) {
                Ok(cfg) => {
                    if !seen_ids.insert(cfg.id.clone()) {
                        tracing::warn!(
                            "provider id 重复: {}（{}），已保留第一个",
                            cfg.id,
                            path.display()
                        );
                        continue;
                    }
                    presets.push(cfg);
                }
                Err(e) => {
                    tracing::warn!("解析 provider 配置 {} 失败: {}，已跳过", path.display(), e);
                }
            }
        }

        Ok(presets)
    }

    /// 单文件加载（测试用 + init 命令用）
    ///
    /// 失败时返回 `Err`（**不** warn 不跳过）
    pub fn load_one(path: &Path) -> Result<ProviderConfig> {
        let content =
            std::fs::read_to_string(path).map_err(crate::common::error::EflowError::Io)?;
        let expanded = expand_env_vars(&content);
        let cfg: ProviderConfig = serde_yaml::from_str(&expanded).map_err(|e| {
            crate::common::error::EflowError::Config(format!(
                "解析 provider YAML {} 失败: {}",
                path.display(),
                e
            ))
        })?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_all_empty_dir_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let result = PresetLoader::load_all(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn load_all_nonexistent_dir_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let nonexistent = dir.path().join("does_not_exist");
        let result = PresetLoader::load_all(&nonexistent).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn load_all_single_valid_file() {
        let dir = TempDir::new().unwrap();
        let yaml = r#"
id: deepseek
display_name: DeepSeek
protocol: openai_compatible
base_url: https://api.deepseek.com
api_key: "test-key"
default_model: deepseek-v4-pro
"#;
        fs::write(dir.path().join("deepseek.yaml"), yaml).unwrap();

        let result = PresetLoader::load_all(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "deepseek");
        assert_eq!(result[0].default_model, "deepseek-v4-pro");
    }

    #[test]
    fn load_all_skips_invalid_yaml_files() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("invalid.yaml"),
            "this is: not: valid: yaml: : :",
        )
        .unwrap();
        fs::write(
            dir.path().join("valid.yaml"),
            r#"
id: test
display_name: Test
protocol: openai_compatible
base_url: https://api.test.com
api_key: "k"
default_model: m
"#,
        )
        .unwrap();

        let result = PresetLoader::load_all(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "test");
    }

    #[test]
    fn load_all_duplicate_id_keeps_first() {
        let dir = TempDir::new().unwrap();
        let yaml_a = r#"
id: same_id
display_name: A
protocol: openai_compatible
base_url: https://a.com
api_key: "k"
default_model: a
"#;
        let yaml_b = r#"
id: same_id
display_name: B
protocol: openai_compatible
base_url: https://b.com
api_key: "k"
default_model: b
"#;
        fs::write(dir.path().join("a.yaml"), yaml_a).unwrap();
        fs::write(dir.path().join("b.yaml"), yaml_b).unwrap();

        let result = PresetLoader::load_all(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].display_name, "A"); // 排序后 a.yaml 先
        assert_eq!(result[0].base_url, "https://a.com");
    }
}
