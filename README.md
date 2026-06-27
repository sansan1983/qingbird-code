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
qingbird --help                              查看所有选项
```

## 交互模式命令

进入 `--interactive` 后，支持以下斜杠命令：

```
/help               显示帮助
/model <名称>       切换模型（当前: deepseek-v4-pro）
/temperature <n>    设置温度（当前: Some(0.7)）
/quit /exit         退出
```

对话历史自动保留，超出 50 条消息时自动截断（保留 system + 最近一半）。

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

# 或直接 cargo install
cargo install qingbird-code
```

## 架构

```
qingbird (binary CLI)
  └── qbird-code-agents    — ReactLoop 状态机 + 死循环检测
  └── qbird-code-tools     — 4 内置工具 (读/写/搜索/命令)
  └── qbird-code-infra     — 5 LLM Provider + HTTP 客户端
  └── qbird-code-models    — 核心类型 (Message/Error/RiskLevel)
```

## 许可证

MIT / Apache-2.0 dual-licensed.
