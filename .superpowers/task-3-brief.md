# Task 3: 新增 list_dir 工具

**Files:**
- Modify: `crates/qbird-code-tools/src/file.rs`（追加 ListDirTool 到文件末尾）
- Modify: `crates/qbird-code-tools/src/lib.rs`
- Modify: `crates/qbird-code/src/main.rs`
- Modify: `locales/zh-CN.yml`
- Modify: `locales/en-US.yml`

**具体要求：**

### 1. 在 `file.rs` 末尾添加 `ListDirTool`

实现 `ListDirTool`：

- Tool name: `"list_dir"`
- description: `t!("tool_list_dir_description")`
- risk_level: `RiskLevel::L0`
- 参数: `path` (string, optional, 默认 `"."`)
- 执行逻辑:
  1. 读取 path 参数
  2. 用 `std::fs::read_dir` 读取目录
  3. 区分文件/目录（`[DIR]` / `[FILE]` 前缀）
  4. 按名称排序
  5. 返回条目列表（每行一个）

```rust
pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".into(),
            description: t!("tool_list_dir_description").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "目录路径，默认当前目录"}
                },
                "required": []
            }),
            risk_level: RiskLevel::L0,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let path = params["path"].as_str().unwrap_or(".");
        let dir = std::path::Path::new(path);
        if !dir.is_dir() {
            return Err(EflowError::Tool(
                t!("err_tool_invalid_path", path = path).to_string(),
            ));
        }

        let mut entries: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(dir).map_err(|e| EflowError::Tool(format!("read_dir: {}", e)))? {
            let entry = entry.map_err(|e| EflowError::Tool(format!("entry: {}", e)))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().map_err(|e| EflowError::Tool(format!("file_type: {}", e)))?;
            let prefix = if file_type.is_dir() { "[DIR]" } else { "[FILE]" };
            entries.push(format!("{} {}", prefix, name));
        }
        entries.sort();

        Ok(ToolOutput {
            success: true,
            content: entries.join("\n"),
            metadata: Some(serde_json::json!({"count": entries.len()})),
        })
    }
}
```

### 2. 在 `lib.rs` 导出

在 `crates/qbird-code-tools/src/lib.rs` 中添加：
```rust
pub use file::{ReadFileTool, WriteFileTool, ListDirTool};
```

### 3. 在 `main.rs` 注册

在 `crates/qbird-code/src/main.rs` 导入 `ListDirTool` 并在 registry 中注册：
```rust
registry.register(Arc::new(ListDirTool));
```

### 4. i18n keys

`zh-CN.yml`:
```yaml
tool_list_dir_description: "列出指定目录中的文件和子目录"
```

`en-US.yml`:
```yaml
tool_list_dir_description: "List files and directories in a path"
```

### 5. 验证

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```

### 注意
- ListDirTool 放在 file.rs 文件末尾（因为都是文件/目录操作，放一起合理）
- 代码注释保持英文
