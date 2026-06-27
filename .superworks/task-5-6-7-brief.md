# Tasks 5-7: Security + Streaming + Version bump

These are the final three tasks for v0.2.14. They affect completely different files and are independent.

---

## Task 5: 安全模块接线（SecurityConfig.allowed_paths）

**Files:**
- Modify: `crates/qbird-code-tools/src/registry.rs`
- Modify: `crates/qbird-code/src/main.rs`
- Modify: `locales/zh-CN.yml`
- Modify: `locales/en-US.yml`

**What to do:**

### 5a. 在 ToolRegistry 中增加 allowed_paths 字段和路径校验

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    allowed_paths: Vec<String>,  // 新增
}
```

添加方法:
```rust
pub fn set_allowed_paths(&mut self, paths: Vec<String>) {
    self.allowed_paths = paths;
}
```

在 `execute()` 中，在风险检查之后、执行之前增加路径校验：

```rust
// 路径安全校验（仅对 L1+ 工具）
if def.risk_level >= RiskLevel::L1 && !self.allowed_paths.is_empty() {
    if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
        let allowed = self.allowed_paths.iter().any(|p| path.starts_with(p));
        if !allowed {
            return Err(EflowError::PermissionDenied(
                t!("err_permission_path", path = path, allowed = self.allowed_paths.join(", ")).to_string()
            ));
        }
    }
}
```

注意：参数中的 `"path"` 仅对 `read_file`、`write_file`、`read_dir`、`glob`、`search_code` 等工具有效。对 `execute_command` 这类没有 path 参数的跳过。

### 5b. 在 main.rs 中接线

在 `main.rs` 中，config 加载和 registry 创建之后，增加：
```rust
tool_registry.set_allowed_paths(cfg.security.allowed_paths.clone());
```

由于 `tool_registry` 当前是 `Arc<ToolRegistry>`，需要改成 `Arc::make_mut` 或在注册工具前设置：
```rust
// 在 build tool_registry 时
from_url.registry.set_allowed_paths(cfg.security.allowed_paths.clone());
let tool_registry = Arc::new(registry);
```

或者更简单：把 `set_allowed_paths` 调用放在 `let tool_registry = Arc::new(registry);` 之前。

### 5c. i18n

`zh-CN.yml`:
```yaml
err_permission_path: "路径 '%{path}' 不在许可列表中。允许的路径：%{allowed}"
```

`en-US.yml`:
```yaml
err_permission_path: "Path '%{path}' is not in the allowed list. Allowed: %{allowed}"
```

---

## Task 6: 流式输出接口准备

**Files:**
- Modify: `crates/qbird-code-infra/src/providers/mod.rs`
- Create: `crates/qbird-code-infra/src/providers/stream.rs`

### 6a. Provider trait 增加 stream 方法

在 `crates/qbird-code-infra/src/providers/mod.rs` 的 `Provider` trait 中增加一个带默认实现的 `stream` 方法：

```rust
/// 发送流式请求。返回完整响应（默认回退到非流式）
async fn stream(
    &self,
    http_client: &HttpLlmClient,
    messages: &[Message],
    config: &RequestConfig,
) -> Result<ChatResponse> {
    // 默认实现：走非流式
    let mut req_config = config.clone();
    req_config.stream = true;
    let body = self.build_request_body(messages, &req_config);
    let response_json = http_client.send(self, &body).await?;
    self.parse_response(&response_json).await
}
```

### 6b. 创建 stream.rs

```rust
use qbird_code_models::Result;

/// SSE 流式响应解析器（当前为 stub，实际实现留给后续版本）
pub struct SseStream;

impl SseStream {
    /// 解析 SSE 行，解析出 data 内容
    pub fn parse_line(line: &str) -> Option<&str> {
        line.strip_prefix("data: ")
    }

    /// 判断是否为流结束标记
    pub fn is_done(line: &str) -> bool {
        line.trim() == "data: [DONE]"
    }
}
```

### 6c. 在 providers/mod.rs 中导出

```rust
pub mod stream;
```

---

## Task 7: V0.2.14 版本收尾

**Files:**
- Modify: `Cargo.toml`
- Modify: `CHANGELOG.md`

### 7a. 版本号 bump

在 `Cargo.toml` 中将 workspace version 从 `"0.2.13"` 改为 `"0.2.14"`。

### 7b. CHANGELOG

在 `CHANGELOG.md` 顶部增加 v0.2.14 条目：

```
## [0.2.14] - 2026-06-27

### Added

- **3 个新工具**: glob（文件搜索）、list_dir（目录列表）、web_fetch（URL 内容抓取）
- **流式接口准备**: Provider trait 新增 `stream()` 方法 + SSE 解析器 stub
- **安全模块接线**: SecurityConfig.allowed_paths 现在实际生效，阻止写入未许可路径

### Fixed

- **Mock 测试 flaky**: 修复 TCP RST 问题 + 增加就绪信号机制，测试稳定通过
```

---

## 验证全部

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```
