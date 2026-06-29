# 配置文件完整参考

> `qingbird.yaml` 的全部字段、默认值与校验规则。

[English Summary](#english-summary)

---

## 文件位置

qingbird 按以下顺序查找配置文件：

1. **当前目录** `qingbird.yaml`（优先）
2. **用户配置目录** `~/.config/qingbird/config.yaml`（Linux/macOS）或 `%APPDATA%\qingbird\config.yaml`（Windows）

找到第一个即加载，都不在则报错退出（exit code 1）。

---

## 完整字段表

### `core` — 核心设置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `core.language` | string | `"zh-CN"` | 界面语言。可选 `zh-CN` / `en-US` |
| `core.timezone` | string | `"Asia/Shanghai"` | 时区标识符（IANA 格式） |

```yaml
core:
  language: zh-CN
  timezone: Asia/Shanghai
```

### `llm` — LLM 配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `llm.active` | string | `"deepseek"` | 当前激活的 Provider。可选 `deepseek` / `deepseek-anthropic` / `ollama` / `openai` / `anthropic` |

```yaml
llm:
  active: deepseek
```

### `llm.deepseek` — DeepSeek Provider

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `api_key` | string? | `null` | API Key。支持 `${ENV_VAR}` 引用。也可通过 `DEEPSEEK_API_KEY` 环境变量设置 |
| `base_url` | string | `"https://api.deepseek.com"` | OpenAI 协议端点 |
| `base_url_anthropic` | string | `"https://api.deepseek.com/anthropic"` | Anthropic 协议端点（仅 `deepseek-anthropic` provider 使用） |
| `default_model` | string | `"deepseek-v4-pro"` | 默认模型 |
| `fast_model` | string | `"deepseek-v4-flash"` | 快速模型（用于低延迟场景） |
| `thinking_enabled` | bool | `true` | 是否启用思维链（thinking） |
| `thinking_effort` | string | `"high"` | 思维深度。可选 `low` / `medium` / `high` |
| `timeout_secs` | u64 | `30` | HTTP 请求超时（秒） |
| `max_retries` | u8 | `3` | 最大重试次数 |
| `retry_backoff_ms` | u64 | `1000` | 首次重试退避时间（毫秒），后续按 2x 递增，上限 30s |
| `cost_per_million_input_tokens` | f64 | `0.0` | 每百万输入 token 费用（USD）。`0.0` = 未知 |
| `cost_per_million_output_tokens` | f64 | `0.0` | 每百万输出 token 费用（USD）。`0.0` = 未知 |

```yaml
llm:
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"
    base_url: "https://api.deepseek.com"
    default_model: "deepseek-v4-pro"
    fast_model: "deepseek-v4-flash"
    thinking_enabled: true
    thinking_effort: "high"
    timeout_secs: 30
    max_retries: 3
    retry_backoff_ms: 1000
```

### `llm.ollama` — Ollama Provider（本地）

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `api_key` | string? | `null` | 本地运行不需要 |
| `base_url` | string | `"http://localhost:11434"` | Ollama 服务地址 |
| `default_model` | string | `"qwen2.5:14b"` | 默认模型 |
| `timeout_secs` | u64 | `30` | HTTP 请求超时（秒） |
| `max_retries` | u8 | `3` | 最大重试次数 |
| `retry_backoff_ms` | u64 | `1000` | 首次重试退避时间（毫秒） |
| `cost_per_million_input_tokens` | f64 | `0.0` | 本地模型默认 0 |
| `cost_per_million_output_tokens` | f64 | `0.0` | 本地模型默认 0 |

```yaml
llm:
  ollama:
    base_url: "http://localhost:11434"
    default_model: "qwen2.5:14b"
```

### `llm.openai` — OpenAI Provider

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `api_key` | string? | `null` | 支持 `${OPENAI_API_KEY}` 或环境变量 |
| `base_url` | string | `"https://api.openai.com"` | API 端点 |
| `default_model` | string | `"gpt-4o"` | 默认模型 |
| `timeout_secs` | u64 | `30` | HTTP 请求超时（秒） |
| `max_retries` | u8 | `3` | 最大重试次数 |
| `retry_backoff_ms` | u64 | `1000` | 首次重试退避时间（毫秒） |
| `cost_per_million_input_tokens` | f64 | `0.0` | 每百万输入 token 费用 |
| `cost_per_million_output_tokens` | f64 | `0.0` | 每百万输出 token 费用 |

```yaml
llm:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com"
    default_model: "gpt-4o"
```

### `llm.anthropic` — Anthropic Provider

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `api_key` | string? | `null` | 支持 `${ANTHROPIC_API_KEY}` 或环境变量 |
| `base_url` | string | `"https://api.anthropic.com"` | API 端点 |
| `default_model` | string | `"claude-sonnet-4-6"` | 默认模型 |
| `timeout_secs` | u64 | `30` | HTTP 请求超时（秒） |
| `max_retries` | u8 | `3` | 最大重试次数 |
| `retry_backoff_ms` | u64 | `1000` | 首次重试退避时间（毫秒） |
| `cost_per_million_input_tokens` | f64 | `0.0` | 每百万输入 token 费用 |
| `cost_per_million_output_tokens` | f64 | `0.0` | 每百万输出 token 费用 |

```yaml
llm:
  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    base_url: "https://api.anthropic.com"
    default_model: "claude-sonnet-4-6"
```

### `llm.cache` — 缓存配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `l1_enabled` | bool | `true` | 启用 L1 内存缓存 |
| `l2_enabled` | bool | `false` | 启用 L2 持久化缓存 |
| `l2_ttl_days` | u32 | `7` | L2 缓存过期天数 |

```yaml
llm:
  cache:
    l1_enabled: true
    l2_enabled: false
    l2_ttl_days: 7
```

### `memory` — 记忆系统

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `working_memory_limit` | usize | `1000` | 工作记忆条目上限。**必须 > 0** |
| `cleanup_interval_hours` | u64 | `24` | 会话自动清理间隔（小时） |

```yaml
memory:
  working_memory_limit: 1000
  cleanup_interval_hours: 24
```

### `security` — 安全设置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `risk_threshold` | RiskLevel | `L2`（serde default） | 全局风险阈值。可选 `L0` / `L1` / `L2` / `L3`。工具执行风险超过此阈值时被拒绝 |
| `allowed_paths` | list\<string\> | `[]`（空 = 允许所有） | 允许工具访问的路径前缀列表。支持 `~` 展开 |

```yaml
security:
  risk_threshold: L2
  allowed_paths:
    - ~/projects
    - ~/documents
```

> `allowed_paths` 为空列表时，工具可访问任意路径（安全默认值）。设为非空时，只允许列表中的路径前缀。

### `profiles` — Profile 配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `default` | string | `""`（空 = 不加载） | 启动时自动加载的 Profile 名称 |
| `available` | list\<string\> | `[]` | 可用 Profile 列表（仅作记录，实际以文件系统为准） |

```yaml
profiles:
  default: developer
  available:
    - developer
```

> Profile 文件位于 `<data_dir>/qingbird/profiles/` 目录。详见 [profiles.md](profiles.md)。

---

## 环境变量引用

`api_key` 字段支持 `${ENV_VAR}` 语法，加载时自动替换为环境变量值：

```yaml
llm:
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"    # 替换为 $DEEPSEEK_API_KEY 的值
  openai:
    api_key: "${OPENAI_API_KEY}"
```

如果环境变量未设置，该字段值为空字符串。

---

## 校验规则

qingbird 启动时自动校验配置。校验失败会打印所有错误并以 exit code 2 退出。

| 规则 | 字段 | 说明 |
|------|------|------|
| Provider 合法性 | `llm.active` | 必须是 `deepseek` / `deepseek-anthropic` / `ollama` / `openai` / `anthropic` 之一 |
| API Key 存在 | `llm.<provider>.api_key` | 远程 Provider（非 ollama）的 api_key 必须非空，或对应环境变量已设置 |
| Profile 存在 | `profiles.default` | 如果非空，必须指向已存在的 Profile 文件 |
| 记忆限制非零 | `memory.working_memory_limit` | 必须 > 0 |

---

## 完整示例

```yaml
core:
  language: zh-CN
  timezone: Asia/Shanghai

llm:
  active: deepseek
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"
    base_url: "https://api.deepseek.com"
    base_url_anthropic: "https://api.deepseek.com/anthropic"
    default_model: "deepseek-v4-pro"
    fast_model: "deepseek-v4-flash"
    thinking_enabled: true
    thinking_effort: "high"
    timeout_secs: 30
    max_retries: 3
    retry_backoff_ms: 1000
    cost_per_million_input_tokens: 0.27
    cost_per_million_output_tokens: 1.10
  ollama:
    base_url: "http://localhost:11434"
    default_model: "qwen2.5:14b"
  openai:
    api_key: "${OPENAI_API_KEY}"
    default_model: "gpt-4o"
  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    default_model: "claude-sonnet-4-6"
  cache:
    l1_enabled: true
    l2_enabled: false
    l2_ttl_days: 7

memory:
  working_memory_limit: 1000
  cleanup_interval_hours: 24

security:
  risk_threshold: L2
  allowed_paths:
    - ~/projects
    - ~/documents

profiles:
  default: developer
  available:
    - developer
```

---

## English Summary

### File Location
1. `./qingbird.yaml` (current directory, priority)
2. `~/.config/qingbird/config.yaml` (Linux/macOS) or `%APPDATA%\qingbird\config.yaml` (Windows)

### Sections
| Section | Key Fields |
|---------|------------|
| `core` | `language` (zh-CN/en-US), `timezone` (IANA) |
| `llm` | `active` (provider name), per-provider configs, `cache` |
| `llm.<provider>` | `api_key`, `base_url`, `default_model`, `timeout_secs`, `max_retries`, `retry_backoff_ms`, `cost_per_million_*` |
| `llm.cache` | `l1_enabled`, `l2_enabled`, `l2_ttl_days` |
| `memory` | `working_memory_limit` (>0), `cleanup_interval_hours` |
| `security` | `risk_threshold` (L0-L3), `allowed_paths` ([] = allow all) |
| `profiles` | `default`, `available` |

### Validation Rules (exit code 2)
- `llm.active` must be a known provider
- API key must be set (yaml or env var) for remote providers
- `profiles.default` must point to an existing file (if non-empty)
- `memory.working_memory_limit` must be > 0

### Env Var Substitution
`api_key` fields support `${VAR_NAME}` syntax, resolved at load time.
