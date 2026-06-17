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
