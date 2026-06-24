# qingbird (青鸟)

> 高效 Rust 多层 Agent 协作框架 · *传统 AI 框架的冗余 Layer 在 qingbird 中被斩断*

[![Rust](https://img.shields.io/badge/rust-2024-orange)]()
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)]()

**一键**完成复杂任务：深检索→编码→自检回路，Agent 团队式协作。

## 快速开始

```bash
# 安装
cargo install qingbird-code

# 配置——只需一行环境变量
export DEEPSEEK_API_KEY="sk-..."

# 执行
qingbird --execute "分析当前目录结构并总结项目"
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
交互层       →  CLI (--execute) + TUI (ratatui)
编排层       →  Concierge (零阻塞) → Orchestrator (分解+并行派发)
能力层       →  Decisioner → Executor → Feedbacker (管线段)
基础设施层   →  DeepSeek LLM / 三层记忆 / Event Bus / Profile / Tools
```

四层严格向下依赖，下层禁止引用上层。

## 许可证

MIT / Apache-2.0 dual-licensed.
