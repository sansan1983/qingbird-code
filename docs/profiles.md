# Profile 系统用户指南

> Profile 是用户级预设配置，让你快速切换不同的工作模式。

[English Summary](#english-summary)

---

## 什么是 Profile

Profile 是一个 YAML 文件，定义了一组运行时覆盖：

- **系统提示词**（`system_prompt`）— 替换默认提示词
- **工具白名单**（`tools_allow`）— 限制可用工具
- **风险阈值**（`risk_threshold`）— 调整安全级别
- **Provider / Model** — 指定 LLM 提供商和模型

Profile 的设计目标：让你在「开发者模式」「研究者模式」「安全模式」等之间一键切换，无需手动编辑配置文件。

---

## 文件格式

每个 Profile 是一个 `.yaml` 文件，放在 Profile 目录下：

```
<data_dir>/qingbird/profiles/
├── developer.yaml
├── researcher.yaml
└── my-custom.yaml
```

Profile 目录位置：

| 平台 | 路径 |
|------|------|
| Linux | `$XDG_DATA_HOME/qingbird/profiles/`（通常 `~/.local/share/qingbird/profiles/`） |
| macOS | `~/Library/Application Support/qingbird/profiles/` |
| Windows | `%APPDATA%\qingbird\profiles\` |

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 是 | Profile 名称（与文件名一致） |
| `description` | string | 否 | 描述信息 |
| `system_prompt` | string | 否 | 系统提示词（替换默认值，非追加） |
| `tools_allow` | list\<string\> | 否 | 工具白名单。空列表 = 不覆盖。非空 = 只允许列表中的工具 |
| `risk_threshold` | string | 否 | 风险阈值：`L0` / `L1` / `L2` / `L3` |
| `provider` | string | 否 | LLM Provider（需重启生效） |
| `model` | string | 否 | LLM 模型（需重启生效） |

### 工具名称参考

可用的工具名称（用于 `tools_allow`）：

| 工具名 | 功能 |
|--------|------|
| `read_file` | 读取文件 |
| `write_file` | 写入文件 |
| `edit` | 编辑文件（带 undo 支持） |
| `execute_command` | 执行 shell 命令 |
| `search_code` | 代码搜索 |
| `glob` | 文件名模式匹配 |
| `list_dir` | 列出目录 |
| `web_fetch` | 获取网页内容 |

---

## 内置 Profile

qingbird 首次启动时会自动创建两个示例 Profile：

### `developer` — Rust 开发助手

```yaml
name: developer
description: "Rust development assistant"
system_prompt: "你是一个专业的 Rust 开发助手。使用中文回复，代码注释保持英文。"
tools_allow: []
risk_threshold: L3
```

特点：
- 允许所有工具（`tools_allow` 为空 = 不限制）
- 最高风险阈值 L3（允许执行任意命令）
- 适合日常开发

### `researcher` — 研究助手（只读）

```yaml
name: researcher
description: "Research assistant (read-only)"
system_prompt: "你是一个研究助手，专注于信息检索和分析。只使用只读工具。"
tools_allow:
  - read_file
  - search_code
  - glob
  - list_dir
  - web_fetch
risk_threshold: L1
```

特点：
- 只允许只读工具（不能写文件、不能执行命令）
- 低风险阈值 L1
- 适合代码审查、文档研究

---

## 使用方式

### 方式 1：CLI 启动参数

```bash
# 使用 researcher profile
qingbird --profile researcher -i

# 配合其他参数
qingbird --profile developer --provider ollama -e "列出项目结构"
```

### 方式 2：配置文件默认 Profile

在 `qingbird.yaml` 中设置：

```yaml
profiles:
  default: developer
```

这样每次启动自动加载 developer profile，无需手动指定。

### 方式 3：交互模式切换

在 `--interactive` 模式中使用 `/profile` 命令：

```
> /profile researcher
已加载 Profile: researcher

> /profile developer
已加载 Profile: developer

> /profile list
可用 Profiles:
  developer
  researcher

> /profile
当前 Profile: developer
```

---

## Walkthrough：创建 → 使用 → 切换

以下演示完整的 Profile 使用流程。

### 第 1 步：查看现有 Profile

```bash
qingbird -i
> /profile list
可用 Profiles:
  developer
  researcher
```

### 第 2 步：使用 developer Profile

```bash
> /profile developer
已加载 Profile: developer
```

此时系统提示词被替换为「你是一个专业的 Rust 开发助手」，所有工具可用，风险阈值为 L3。

### 第 3 步：创建工作 Profile

手动创建文件 `<data_dir>/qingbird/profiles/analyst.yaml`：

```yaml
name: analyst
description: "代码分析专用，禁止写入和执行"
system_prompt: |
  你是一个代码分析师。专注于理解代码结构、发现潜在问题、提供改进建议。
  不要修改任何文件，不要执行任何命令。
  使用中文回复，代码引用保持英文。
tools_allow:
  - read_file
  - search_code
  - glob
  - list_dir
risk_threshold: L0
```

### 第 4 步：切换到 analyst Profile

```
> /profile analyst
已加载 Profile: analyst
```

现在只能使用只读工具，风险阈值降到最低 L0。

### 第 5 步：切回 developer 继续开发

```
> /profile developer
已加载 Profile: developer
```

所有工具恢复可用。

---

## Profile 合并规则

Profile 应用时遵循**替换**而非追加原则：

| 字段 | 行为 |
|------|------|
| `system_prompt` | **替换**整个系统提示词 |
| `tools_allow` | **替换**工具白名单（非空时） |
| `risk_threshold` | **替换**风险阈值 |
| `provider` | **替换** Provider（需重启生效） |
| `model` | **替换** Model（需重启生效） |

### 优先级

```
CLI --profile > qingbird.yaml profiles.default > 无 Profile
```

### Provider/Model 限制

v0.3.0 中，LLM 客户端在 Profile 加载之前初始化。如果 Profile 指定了不同的 `provider` 或 `model`，会在启动时显示警告，但**不会重新初始化 LLM 客户端**。

要让 provider/model 生效，需重启 qingbird 并通过 `--profile` 加载。

---

## 自定义 Profile 最佳实践

1. **用途明确**：每个 Profile 有清晰的使用场景（开发、审查、研究…）
2. **最小权限**：`tools_allow` 只包含必要工具
3. **风险匹配**：高风险操作用高阈值，只读场景用低阈值
4. **提示词精准**：`system_prompt` 明确角色和行为约束

---

## English Summary

### What are Profiles
Profiles are YAML preset files that override system prompt, allowed tools, risk threshold, provider, and model at runtime.

### File Location
`<data_dir>/qingbird/profiles/*.yaml`
- Linux: `~/.local/share/qingbird/profiles/`
- macOS: `~/Library/Application Support/qingbird/profiles/`
- Windows: `%APPDATA%\qingbird\profiles\`

### Built-in Profiles
- **developer**: All tools, L3 risk, Rust dev assistant
- **researcher**: Read-only tools, L1 risk, research assistant

### Usage
```bash
# CLI
qingbird --profile researcher -i

# Config default
profiles:
  default: developer

# Interactive
> /profile list
> /profile researcher
> /profile developer
```

### Merge Rules
- `system_prompt`: replaces (not appends)
- `tools_allow`: replaces when non-empty
- `risk_threshold`: replaces when set
- `provider`/`model`: replaces (requires restart)

### Priority
`--profile` CLI flag > `profiles.default` in yaml > no profile
