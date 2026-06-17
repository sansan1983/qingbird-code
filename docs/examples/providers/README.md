# eflow v1.3 Preset Provider 样例

把下面 6 个 YAML 复制到 `~/.eflow/providers/{id}.yaml` 即可使用对应 LLM。

| 文件 | 厂商 | 需要 env var |
|---|---|---|
| `deepseek.yaml` | DeepSeek | `DEEPSEEK_API_KEY` |
| `minimax.yaml` | MiniMax（minimaxi.com） | `MINIMAX_API_KEY` |
| `agnes-ai.yaml` | Agnes AI | `AGNES_API_KEY` |
| `opencode-go.yaml` | OpenCode Go（订阅型 LLM 网关） | `OPENCODE_GO_API_KEY` |
| `anthropic.yaml` | Anthropic（env var 退化路径默认） | `ANTHROPIC_API_KEY` |
| `openai.yaml` | OpenAI（env var 退化路径默认） | `OPENAI_API_KEY` |

**OpenCode Go 特殊**：13 个模型分 2 个 endpoint 路径（`/v1/chat/completions` 和 `/v1/messages`），用 `list_models` 数组显式指定。

复制示例（Windows PowerShell）：

```powershell
mkdir ~/.eflow/providers -Force
cp docs/examples/providers/deepseek.yaml $env:USERPROFILE\.eflow\providers\
```

复制示例（Linux/macOS）：

```bash
mkdir -p ~/.eflow/providers
cp docs/examples/providers/deepseek.yaml ~/.eflow/providers/
```

详见 `docs/migration-v1.2-to-v1.3.md`。
