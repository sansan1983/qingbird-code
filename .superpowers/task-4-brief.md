# Task 4: 新增 web_fetch 工具

**Files:**
- Create: `crates/qbird-code-tools/src/web_fetch.rs`
- Modify: `crates/qbird-code-tools/src/lib.rs`
- Modify: `crates/qbird-code/src/main.rs`
- Modify: `locales/zh-CN.yml`
- Modify: `locales/en-US.yml`

**具体要求：**

### 1. 创建 `web_fetch.rs`

```rust
use async_trait::async_trait;

use crate::registry::{Tool, ToolDefinition, ToolOutput};
use qbird_code_models::{EflowError, Result, RiskLevel};
use rust_i18n::t;

const MAX_RESPONSE_BYTES: usize = 512_000;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_fetch".into(),
            description: t!("tool_web_fetch_description").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "要抓取的 URL"},
                    "format": {"type": "string", "enum": ["markdown", "text", "html"], "description": "返回格式，默认 text"}
                },
                "required": ["url"]
            }),
            risk_level: RiskLevel::L0,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let url = params["url"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "url").to_string())
        })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| EflowError::Tool(format!("Failed to create HTTP client: {}", e)))?;

        let resp = client.get(url).send().await
            .map_err(|e| EflowError::Tool(format!("Request failed: {}", e)))?;

        let status = resp.status();
        let body = resp.bytes().await
            .map_err(|e| EflowError::Tool(format!("Read body failed: {}", e)))?;

        let mut content = String::from_utf8_lossy(&body[..body.len().min(MAX_RESPONSE_BYTES)]).to_string();
        if body.len() > MAX_RESPONSE_BYTES {
            content.push_str(&format!("\n\n[Response truncated at {} bytes]", MAX_RESPONSE_BYTES));
        }

        Ok(ToolOutput {
            success: status.is_success(),
            content,
            metadata: Some(serde_json::json!({
                "status": status.as_u16(),
                "content_type": resp.headers().get("content-type").and_then(|v| v.to_str().ok()),
                "bytes": body.len(),
                "truncated": body.len() > MAX_RESPONSE_BYTES,
            })),
        })
    }
}
```

### 2. 导出和注册

在 `lib.rs`:
```rust
pub mod web_fetch;
pub use web_fetch::WebFetchTool;
```

在 `main.rs` 导入并使用 `registry.register(Arc::new(WebFetchTool))`。

### 3. i18n keys

`zh-CN.yml`:
```yaml
tool_web_fetch_description: "抓取指定 URL 的内容并返回"
```

`en-US.yml`:
```yaml
tool_web_fetch_description: "Fetch content from a URL"
```

### 4. 验证

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```

### 注意
- 工具放在独立文件 `web_fetch.rs`，不要塞进 file.rs
- 代码注释保持英文
