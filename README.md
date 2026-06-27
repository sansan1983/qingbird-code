# qingbird (青鸟)

> 高效 Rust Agent 协作框架 · 单二进制，多 Provider，ReAct 循环

## 快速开始

```bash
# 1. 构建
cargo build --release

# 2. 设置 API Key
export DEEPSEEK_API_KEY="sk-..."

# 3. 运行
./target/release/qingbird --execute "分析当前目录结构"
```

## 配置

配置文件位置：`qingbird.yaml`（当前目录）或 `~/.qingbird/config.yaml`

```yaml
llm:
  active: deepseek                    # 默认 Provider
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"    # 支持环境变量引用
    base_url: "https://api.deepseek.com"
    default_model: "deepseek-chat"
    thinking_enabled: true
    thinking_effort: "high"
    timeout_secs: 30
    max_retries: 3
    retry_backoff_ms: 1000
```

不配置也行，只要设置了对应环境变量（`DEEPSEEK_API_KEY` / `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`）即可运行。

## CLI 用法

```
qingbird --execute "提示词"                 单次执行
qingbird --interactive                       交互模式（多轮对话）
qingbird --provider ollama --execute "..."   临时切换 Provider
qingbird --model deepseek-chat --execute "..."  临时切换模型
qingbird --temperature 0.3 --execute "..."   临时设置温度
qingbird --lang zh-CN --execute "..."        指定界面语言（zh-CN / en-US）
qingbird --profile developer --execute "..." 加载用户级 Profile
qingbird --help                              查看所有选项
```

## 交互模式命令

进入 `--interactive` 后，支持以下 7 个斜杠命令（中英双语）：

```
/help               显示帮助
/model <名称>       切换模型
/temperature <n>    设置温度
/usage              查看 token 用量与成本
/sessions           列出历史会话
/session load <id>  加载指定会话
/session delete <id> 删除会话
/sdd run <input>    启动 SDD 工作流
/sdd confirm        确认 SDD proposal
/sdd status         查看 SDD 状态
/quit /exit         退出
```

v0.3.0 计划新增：`/undo` / `/profile` / `/provider`（在 `/help` 中以 `[planned]` 标记）。

## 支持的 Provider

| Provider | 配置 active 值 | 环境变量 |
|----------|---------------|---------|
| DeepSeek（OpenAI 协议） | `deepseek` | `DEEPSEEK_API_KEY` |
| DeepSeek（Anthropic 协议） | `deepseek-anthropic` | `DEEPSEEK_API_KEY` |
| Ollama（本地） | `ollama` | 不需要 |
| OpenAI | `openai` | `OPENAI_API_KEY` |
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` |

```bash
# 例子：用 Ollama 本地模型
qingbird --provider ollama --interactive

# 例子：用 GPT-4o
export OPENAI_API_KEY="sk-..."
qingbird --provider openai --model gpt-4o --execute "Hello"
```

## 安装

```bash
# 从源码
git clone <repo>
cd qingbird-code
cargo build --release
./target/release/qingbird --execute "..."
```

## 架构

```
qingbird (binary CLI)
  └── qbird-code-agents    — ReactLoop 状态机 + 死循环检测 + Subagent 池
  └── qbird-code-tools     — 7 内置工具 (读/写/搜索/命令/glob/list_dir/web_fetch)
  └── qbird-code-infra     — 5 LLM Provider + HTTP 客户端 + Profile + Stream
  └── qbird-code-models    — 核心类型 (Message/Error/RiskLevel/PermissionSet)
```

## 用户文档

详细参考见 `docs/` 目录（v0.3.0 起提供 CLI / Configuration / Profiles 完整文档）。

## 许可证

MIT / Apache-2.0 dual-licensed.
