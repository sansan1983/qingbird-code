# eflow v1.2 → v1.3 配置迁移指南

v1.3 起 LLM provider **不**在 `eflow.yaml` 里配置，**改**用 `~/.eflow/providers/{name}.yaml` 独立文件管理。

## 破坏性变更

### `eflow.yaml::llm.providers` 字段删除

**v1.2 形态**（废弃）：
```yaml
llm:
  providers:
    anthropic:
      api_key: ${ANTHROPIC_API_KEY}
      default_model: claude-sonnet-4-6
    openai:
      api_key: ${OPENAI_API_KEY}
      default_model: gpt-4o
  routing:
    strong: anthropic
    medium: anthropic
    light: openai
  cache:
    l1_enabled: true
```

**v1.3 形态**：
```yaml
llm:
  routing:                       # 引用 provider id（任意字符串）
    strong: "deepseek"
    medium: "deepseek"
    light: "minimax"
  cache:
    l1_enabled: true
```

`routing.{strong,medium,light}` 字段名不变，**值**从 "anthropic"/"openai" 变成任意 provider id（用户在 `~/.eflow/providers/*.yaml` 定义）。

## 迁移步骤

### Step 1: 创建 `~/.eflow/providers/` 目录

**Windows**：
```powershell
mkdir $env:USERPROFILE\.eflow\providers
```

**Linux/macOS**：
```bash
mkdir -p ~/.eflow/providers
```

### Step 2: 复制 preset 样例

从 `docs/examples/providers/` 复制你需要的厂商到 `~/.eflow/providers/`：

```bash
# 例：只用 DeepSeek + MiniMax
cp docs/examples/providers/deepseek.yaml ~/.eflow/providers/
cp docs/examples/providers/minimax.yaml ~/.eflow/providers/
```

每个文件长这样（deepseek.yaml）：
```yaml
id: deepseek
display_name: "DeepSeek"
protocol: openai_compatible
base_url: "https://api.deepseek.com"
api_key: "${DEEPSEEK_API_KEY}"
default_model: "deepseek-v4-pro"
preset_models:
  - "deepseek-v4-pro"
  - "deepseek-v4-flash"
list_models_endpoint: "/models"
```

`api_key` 里的 `${DEEPSEEK_API_KEY}` 会**自动**展开成环境变量值。

### Step 3: 设置环境变量

```bash
# 至少设置你用的 provider
export DEEPSEEK_API_KEY="sk-..."
export MINIMAX_API_KEY="ey..."
```

### Step 4: 修改 `eflow.yaml` 删 `providers` 字段

把 v1.2 的：
```yaml
llm:
  providers:
    anthropic: ...
    openai: ...
```

删掉。

把 v1.2 的：
```yaml
routing:
  strong: anthropic
  medium: anthropic
  light: openai
```

改成：
```yaml
routing:
  strong: "deepseek"   # 引用 ~/.eflow/providers/deepseek.yaml 的 id 字段
  medium: "deepseek"
  light: "minimax"
```

### Step 5: 启动验证

```bash
eflow
```

启动后 header 区应显示 `profile: deepseek`（v1.2 TUI 已有）。**首次启动**会检测到无 `~/.eflow/config.yaml`，**不会**走 v1.3.1 的向导（spec B 范围）；如果没配置就 bare TUI 起。

## 不改 v1.2 行为

- `ANTHROPIC_API_KEY` / `ANTHROPIC_BASE_URL` env var **完全保留**——`~/.eflow/providers/` 为空时自动 fallback
- `LlmProvider` trait 接口稳定——spec B CLI 契约不破

## 加自定义 provider

复制 `docs/examples/providers/openai.yaml` 改一下：

```yaml
id: my-proxy
display_name: "My OpenAI Proxy"
protocol: openai_compatible
base_url: "https://my-proxy.example.com/v1"
api_key: "sk-..."
default_model: "gpt-4o"
```

保存到 `~/.eflow/providers/my-proxy.yaml`，eflow 重启后即用。

## OpenCode Go 特殊说明

OpenCode Go 13 个模型分 2 个 endpoint 路径（`/v1/chat/completions` 和 `/v1/messages`），
**必须**用 `list_models` 数组显式指定（**不**能用 string 列表）。详见
`docs/examples/providers/opencode-go.yaml`。

## 故障排查

**启动报错 `err_no_llm_providers`**：说明 `~/.eflow/providers/` 为空 + env var 也没设。检查：
1. `~/.eflow/providers/*.yaml` 文件存在？
2. `api_key: "${XXX_API_KEY}"` 里 `XXX_API_KEY` 环境变量设了？
3. `dirs::config_dir()` 解析路径？Windows 通常是 `%APPDATA%\eflow\providers\`，Unix 是 `~/.config/eflow/providers/`。
