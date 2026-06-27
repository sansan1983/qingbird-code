use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use qbird_code_models::{EflowError, Result, RiskLevel};
use rust_i18n::t;

/// 工具定义
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
    pub risk_level: RiskLevel,
}

/// 工具输出
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub success: bool,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

/// 工具 trait
#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput>;
}

/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    allowed_paths: Vec<String>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            allowed_paths: Vec::new(),
        }
    }

    pub fn set_allowed_paths(&mut self, paths: Vec<String>) {
        self.allowed_paths = paths;
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let def = tool.definition();
        self.tools.insert(def.name, tool);
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// 获取所有工具定义（用于发送给 LLM）
    #[must_use]
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// 执行工具
    ///
    /// `task_id` 用于在 `RiskEscalated` 错误中携带真实任务 ID（fix v1.0.3 B2），
    /// 使上层能定位到具体任务
    pub async fn execute(
        &self,
        name: &str,
        params: serde_json::Value,
        task_id: uuid::Uuid,
    ) -> Result<ToolOutput> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| EflowError::Tool(t!("err_tool_not_found", name = name).to_string()))?;

        // 风险检查
        let def = tool.definition();
        if def.risk_level >= RiskLevel::L3 {
            return Err(EflowError::RiskEscalated {
                task_id: task_id.to_string(),
                reason: t!("err_tool_l3_required", name = name).to_string(),
            });
        }

        // 路径安全校验（仅对 L1+ 工具）
        if def.risk_level >= RiskLevel::L1
            && !self.allowed_paths.is_empty()
            && let Some(path) = params.get("path").and_then(|v| v.as_str())
        {
            let allowed = self.allowed_paths.iter().any(|p| path.starts_with(p));
            if !allowed {
                return Err(EflowError::PermissionDenied(
                    t!(
                        "err_permission_path",
                        path = path,
                        allowed = self.allowed_paths.join(", ")
                    )
                    .to_string(),
                ));
            }
        }

        tool.execute(params).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
