# CLI 完整参考

> qingbird 命令行界面的全部选项与交互命令。

[English Summary](#english-summary)

---

## 启动模式

qingbird 有两种启动模式，必须二选一：

| 模式 | Flag | 说明 |
|------|------|------|
| 单次执行 | `--execute <prompt>` / `-e` | 发送一条 prompt，得到回复后退出 |
| 交互模式 | `--interactive` / `-i` | REPL 多轮对话，支持斜杠命令 |

```bash
# 单次执行
qingbird --execute "分析当前目录结构"

# 交互模式
qingbird --interactive
```

> 如果同时省略 `--execute` 和 `--interactive`，qingbird 会打印用法提示并以 exit code 1 退出。

---

## 启动参数（CLI Flags）

### `--execute <prompt>` / `-e`

执行单次任务。prompt 作为用户消息发送给 LLM，回复输出到 stdout 后进程退出。

```bash
qingbird -e "列出所有 .rs 文件"
```

### `--interactive` / `-i`

进入交互式 REPL。支持多轮对话、斜杠命令、会话持久化。

```bash
qingbird -i
```

### `--provider <name>`

临时覆盖 `qingbird.yaml` 中的 `llm.active`。可选值：

| 值 | Provider |
|----|----------|
| `deepseek` | DeepSeek（OpenAI 协议） |
| `deepseek-anthropic` | DeepSeek（Anthropic 协议） |
| `ollama` | Ollama 本地模型 |
| `openai` | OpenAI |
| `anthropic` | Anthropic |

```bash
qingbird --provider ollama -i
```

### `--model <name>`

临时覆盖当前 provider 的 `default_model`。

```bash
qingbird --model deepseek-v4-flash -e "Hello"
```

### `--temperature <value>`

设置 LLM 温度参数。范围 `0.0 ~ 2.0`。

```bash
qingbird --temperature 0.3 -e "写一首诗"
```

### `--lang <locale>`

覆盖 `qingbird.yaml` 中的 `core.language`。可选值：`zh-CN`、`en-US`。

```bash
qingbird --lang en-US -i
```

### `--profile <name>`

加载指定的用户 Profile。Profile 文件位于 `<data_dir>/qingbird/profiles/<name>.yaml`。

优先级：`--profile` > `profiles.default` > 无 profile。

```bash
qingbird --profile researcher -i
```

### `--stream` / `--no-stream`

控制流式输出模式。默认关闭（整块输出）。

- `--stream`：启用打字机式逐字输出
- `--no-stream`：禁用流式输出（覆盖 `--stream`）

```bash
qingbird --stream -e "解释 Rust 生命周期"
```

### `--help`

打印所有可用选项并退出。

```bash
qingbird --help
```

### `--version`

打印版本号并退出。

```bash
qingbird --version
```

---

## 交互模式斜杠命令

进入 `--interactive` 后，可使用以下斜杠命令：

### `/quit` / `/exit`

退出交互模式。退出前自动保存当前会话。

### `/help`

显示所有可用斜杠命令的帮助信息。

### `/model [name]`

- 无参数：显示当前模型名称
- 有参数：切换到指定模型

```
> /model
当前模型: deepseek-v4-pro
> /model deepseek-v4-flash
已切换到模型: deepseek-v4-flash
```

### `/temperature [value]`

- 无参数：显示当前温度值
- 有参数：设置温度（0.0 ~ 2.0）

```
> /temperature
当前温度: None
> /temperature 0.5
温度已设为: 0.5
```

### `/provider [name]`

- 无参数：显示当前 provider
- 有参数：切换到指定 provider（同 `--provider` 可选值）

```
> /provider
当前 Provider: deepseek
> /provider ollama
已切换到 Provider: ollama
```

> 注意：切换 provider 时如果之前通过 `/model` 设置过自定义模型，会自动重置为新 provider 的默认模型。

### `/usage`

显示当前会话的 token 用量统计：prompt tokens、completion tokens、总 tokens、cache hit tokens（如有）、以及估算费用。

### `/sessions`

列出所有历史会话（ID、名称、消息数、最后更新时间）。

### `/session load <id>`

加载指定 ID 的会话，恢复对话历史。

```
> /session load abc123
已加载会话 abc123，共 12 条消息
```

### `/session delete <id>`

删除指定会话（归档到 `sessions.archive/` 目录）。如果删除的是当前会话，会自动生成新的 session ID。

### `/session rename <id> <new_name>`

重命名指定会话。

```
> /session rename abc123 我的项目讨论
会话 abc123 已重命名为: 我的项目讨论
```

### `/sdd run <input>`

启动 SDD（Specification-Driven Development）工作流。生成 proposal 后需用 `/sdd confirm` 确认。

### `/sdd confirm`

确认当前待处理的 SDD proposal，解除 hard gate 阻塞。

### `/sdd status`

显示 SDD 技能列表和当前 proposal 状态（pending/idle）。

### `/undo`

撤销上一次文件写入操作（从 undo 栈中弹出并恢复原内容）。

```
> /undo
已撤销对 src/main.rs 的修改
```

### `/profile [name | list]`

- 无参数：显示当前 profile 名称
- `list`：列出所有可用 profile
- `<name>`：切换到指定 profile（会替换系统提示词、工具白名单、风险阈值等）

```
> /profile
当前 Profile: developer
> /profile list
可用 Profiles:
  developer
  researcher
> /profile researcher
已加载 Profile: researcher
```

> 注意：profile 中的 `provider` / `model` 字段在会话中切换时无法重新初始化 LLM 客户端，会显示警告。

---

## 优先级规则

参数覆盖优先级（高→低）：

1. **CLI flag**（`--provider`、`--model`、`--temperature`、`--lang`、`--profile`）
2. **`qingbird.yaml`** 配置文件
3. **环境变量**（仅 API Key：`DEEPSEEK_API_KEY` / `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`）
4. **内置默认值**

---

## Subagent 系统（v0.3.1+）

qingbird 在 v0.3.1 引入 subagent 机制：主 agent 在 ReAct 循环里通过
`delegate_task` 工具把子任务分发给一个独立 ReAct 循环实例，LLM 透明地
并行/串行调用子 agent 来完成大任务。

### 5 个内置 profile

| Profile | 工具策略 | 适用场景 |
|---|---|---|
| `general` | 继承主 agent | 多步任务、读写文件、运行命令 |
| `explore` | 只读 | 快速浏览文件、搜索、只读分析 |
| `code-writer` | 继承 | 实现功能、修 bug |
| `planner` | 只读 | 制定方案、出实施计划 |
| `reviewer` | 只读 | 代码审查、找问题 |

### LLM 自主调用

`delegate_task` 是注册到主 `ToolRegistry` 的内置工具，LLM 在需要时
自动决定何时调用（不需要用户显式命令）：

```json
{
  "label": "审查登录模块",
  "prompt": "阅读 src/auth/login.rs 找出 3 个最严重的错误",
  "profile": "reviewer"
}
```

一次调用 `delegate_task` 阻塞等子 agent 跑完，把 `ChildRecord`
（`child_id` / `status` / `summary` / `usage` / `profile` /
`tool_policy` / `duration_ms`）以 pretty JSON 返回给主 agent 上下文。

### 安全边界

- 子 agent **共享主 agent** 的 `allowed_tools` / `risk_threshold` /
  `allowed_paths`（构造 `SubagentExecutor` 时 clone `ToolRegistry` 快照）。
- 子 agent **不递归** 看到 `delegate_task` 工具（避免无限派发）。
- 主 agent 的 profile / user config 中可以 yaml 覆盖 5 个内置 profile 的
  任意字段（description / system_prompt / tools / temperature 等）。

### Side Session 持久化（v0.3.1+）

主 agent 会话存到 `SessionStore` 时，子 agent 历史以 `relation: 'side'` +
`parent_session_id` 关联挂载。`/sessions` 默认不显示子会话，避免
列表爆炸；未来 `/sessions --include-side` 复用同一个
`list_sessions_filtered(bool)` API。

---

## English Summary

### Startup Modes
- `--execute <prompt>` / `-e`: Single-shot execution, exits after response
- `--interactive` / `-i`: REPL multi-turn conversation

### All Flags
| Flag | Description |
|------|-------------|
| `--provider <name>` | Override `llm.active` (deepseek/deepseek-anthropic/ollama/openai/anthropic) |
| `--model <name>` | Override current provider's default model |
| `--temperature <0.0-2.0>` | Set LLM temperature |
| `--lang <zh-CN\|en-US>` | Override UI locale |
| `--profile <name>` | Load user profile |
| `--stream` | Enable streaming output |
| `--no-stream` | Disable streaming (overrides --stream) |
| `--help` | Print help |
| `--version` | Print version |

### Slash Commands (interactive mode)
| Command | Description |
|---------|-------------|
| `/quit` `/exit` | Exit |
| `/help` | Show help |
| `/model [name]` | Show/switch model |
| `/temperature [n]` | Show/set temperature |
| `/provider [name]` | Show/switch provider |
| `/usage` | Show token usage & cost |
| `/sessions` | List saved sessions |
| `/session load\|delete\|rename <id> [name]` | Manage sessions |
| `/sdd run\|confirm\|status` | SDD workflow |
| `/undo` | Undo last file write |
| `/profile [name\|list]` | Show/switch/list profiles |

### Subagent (v0.3.1+)
`delegate_task` is a built-in tool the LLM autonomously invokes to dispatch
a child ReAct loop with one of 5 built-in profiles: `general` (inherits),
`explore` / `planner` / `reviewer` (read-only), `code-writer` (inherits).
Children share the parent's tool registry safety settings but do **not**
see `delegate_task` themselves (no recursion).
