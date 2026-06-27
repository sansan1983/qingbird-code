# qingbird (青鸟)

> 高效 Rust 多层 Agent 协作框架 · 5-crate workspace, ReAct 循环, 多 Provider

[![Rust](https://img.shields.io/badge/rust-2024-orange)]()
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)]()

**一键**完成复杂任务：ReAct 循环 + 工具调用，Agent 团队式协作。

## 快速开始

```bash
# 安装
cargo install qingbird-code

# 配置——只需一行环境变量
export DEEPSEEK_API_KEY="sk-..."

# 执行
qingbird --execute "分析当前目录结构并总结项目"

# 交互模式
qingbird --interactive
```

配置文件（可选）`~/.qingbird/config.yaml`：

```yaml
llm:
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"
    default_model: "deepseek-chat"
```

## 架构

```
qingbird (binary)
  └── qbird-code-agents    — ReAct 循环 + Subagent + 死循环检测
  └── qbird-code-tools     — 工具系统 (读/写/搜索/命令)
  └── qbird-code-infra     — 4 家 LLM Provider + 配置 + HTTP 客户端
  └── qbird-code-models    — 核心类型 (Message/Error/ProviderKind)
```

严格依赖方向：下层禁止引用上层。

## 支持的 Provider

| Provider | 协议 | 状态 |
|----------|------|------|
| DeepSeek | OpenAI + Anthropic 双协议 | ✅ 完整 |
| Ollama | OpenAI 兼容 | ✅ 完整 |
| OpenAI | OpenAI | ✅ 骨架 |
| Anthropic | Anthropic | ✅ 骨架 |

## 许可证

MIT / Apache-2.0 dual-licensed.
