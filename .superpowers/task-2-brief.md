# Task 2: 新增 glob 工具

**Files:**
- Create: `crates/qbird-code-tools/src/glob.rs`
- Modify: `crates/qbird-code-tools/src/lib.rs`
- Modify: `crates/qbird-code/src/main.rs`
- Modify: `locales/zh-CN.yml`
- Modify: `locales/en-US.yml`

**具体要求：**

### 1. 创建 `glob.rs`

实现 `GlobTool` 结构体，实现 `Tool` trait：

```rust
pub struct GlobTool;
```

- Tool 名称: `"glob"`
- 描述: 走 i18n `t!("tool_glob_description")`
- risk_level: `RiskLevel::L0`
- 参数: `pattern` (string, required), `path` (string, optional, default `"."`)
- 实现逻辑:
  1. 读取参数 pattern 和 path
  2. 用 `walkdir::WalkDir` 遍历目录（项目已有 walkdir 依赖）
  3. 对每个文件用简易 glob 匹配（支持 `*`、`**`、`?`）
  4. 限制最多 200 个结果
  5. 有结果时输出 `t!("tool_glob_count", count = count)` + 文件列表
  6. 无结果时输出 `t!("tool_glob_no_match", pattern = pattern)`

简易 glob 匹配函数（内置，不需额外 crate）：
```rust
fn glob_match(pattern: &str, path: &str) -> bool {
    let regex_str = pattern
        .replace(".", "\\.")
        .replace("**", "☠")
        .replace("*", "[^/]*")
        .replace("☠", ".*")
        .replace("?", ".");
    let re = regex_lite::Regex::new(&format!("^{}$", regex_str)).ok();
    re.map(|r| r.is_match(path)).unwrap_or(false)
}
```

### 2. 在 `lib.rs` 中导出

在 `crates/qbird-code-tools/src/lib.rs` 中添加：
```rust
pub mod glob;
pub use glob::GlobTool;
```

### 3. 在 `main.rs` 中注册

在 `crates/qbird-code/src/main.rs` 的导入和 registry 注册处添加 `GlobTool`。

### 4. i18n keys

`locales/zh-CN.yml` 添加：
```yaml
tool_glob_description: "使用 glob 模式匹配文件路径"
tool_glob_no_match: "未找到匹配 '%{pattern}' 的文件"
tool_glob_count: "找到 %{count} 个匹配文件:"
```

`locales/en-US.yml` 添加：
```yaml
tool_glob_description: "Find files matching a glob pattern"
tool_glob_no_match: "No files matching '%{pattern}'"
tool_glob_count: "Found %{count} matching files:"
```

### 5. 验证

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```

### 注意
- 代码注释保持英文
- 不要做无关重构
- GlobTool 应放在 `crates/qbird-code-tools/src/glob.rs` 中，不要塞进 file.rs
