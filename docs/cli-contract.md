# eflow v1.3.2 CLI 契约（v2.0 GUI 套壳接口）

> **契约冻结 v1.3.0 起**（spec B2 ADR-0017）——v1.4+ 变更需走 ADR。

## 概述

GUI 通过 spawn `eflow session start` 进程 + stdio 通信：

```
┌─────────────────┐
│  GUI 进程        │
│  ┌────────────┐  │      spawn       ┌─────────────────┐
│  │  按钮/列表  │──┼─────────────────▶│  eflow 进程      │
│  │  ↓ 转换     │  │                  │  ┌────────────┐  │
│  │  spawn      │  │  stdout: NDJSON  │  │ Concierge  │  │
│  │  eflow      │◀─┼──────────────────│  └────────────┘  │
│  └────────────┘  │  stdin: JSON     └─────────────────┘
└─────────────────┘
```

**两条路径行为 100% 一致**：
- TUI 同进程 `CommandRegistry::dispatch()`（spec B1）
- GUI 跨进程 `eflow session start` + stdin/stdout 协议

**TUI / GUI 行为语义等价**——同一份 `SlashCommand` trait impl + 同一份 Concierge 业务逻辑，只是 dispatch 路径不同。

## 1. 进程生命周期

```bash
# 启动
eflow session start --lang en-US
# stdout 第 1 行 = SystemReady 事件
# 然后持续输出 events NDJSON，stdin 接收指令
# 退出码：0 / 1 / 2 / 130
```

## 2. stdout 协议（NDJSON）

### 2.1 SystemReady 事件（start 后第一行）

```json
{"event_type": "SystemReady", "task_id": "00000000-0000-0000-0000-000000000000", "started_at": "2026-06-18T12:34:56.789Z"}
```

**字段**：
- `event_type`: 字符串 `"SystemReady"`
- `task_id`: UUID v4（v1.3.2 阶段固定为 `Uuid::nil()`，未来改真随机）
- `started_at`: ISO 8601 / RFC 3339 时间戳（UTC）

### 2.2 6 个用户事件（task 生命周期）

```json
{"event_type": "TaskStarted", "task_id": "uuid-v4", "description": "review main.rs"}
{"event_type": "TaskCompleted", "task_id": "uuid-v4", "summary": "no issues found"}
{"event_type": "TaskFailed", "task_id": "uuid-v4", "error": "API timeout"}
{"event_type": "RiskEscalated", "task_id": "uuid-v4", "from": "L1", "to": "L2"}
{"event_type": "UserInputRequired", "prompt": "confirm? [y/n]"}
{"event_type": "SystemShutdown"}
```

**字段约定**：
- `event_type`: 字符串（6 个值之一）
- `task_id`: UUID v4 字符串
- 其它字段：见上 schema
- `RiskEscalated.from/to`: `RiskLevel` Debug 格式（v1.3.2 阶段 `L0` / `L1` / `L2` / `L3`）
- `SystemShutdown` 触发 → 进程退出码 0

**重要**：v1.3.2 **不**输出 stdin action 的 ack JSON（plan T9 §4 删）—— GUI 通过**事件流**推断状态（send → 期望 TaskStarted + TaskCompleted/Failed；end → 期望 SystemShutdown）。这条简化与 plan 原设计不同——见 deviation #12q。

## 3. stdin 协议

每行 1 条 JSON 指令（**解析失败不退出**，stderr 报错 + 继续读下一行）：

```json
{"action": "send", "task": "review main.rs"}
{"action": "send", "task_id": "uuid-v4", "task": "..."}    // task_id 可选 = 自动生成
{"action": "end", "task_id": "uuid-v4"}
{"action": "level", "task_id": "uuid-v4", "level": "simple"}
{"action": "lang", "task_id": "uuid-v4", "locale": "en-US"}
{"action": "lang", "locale": "en-US"}                       // task_id 可选
{"action": "help"}
```

**5 个 action 行为**：

| action | 必填字段 | 可选字段 | 行为 | handler 路径 |
|---|---|---|---|---|
| `send` | `task` | `task_id` | 派发 task 给 Concierge | `Concierge::handle_input`（v1.3.1 TaskDispatch 路径） |
| `end` | `task_id` | — | 退出会话（**不**取消已派发 task——spec B2 §3.6 简化） | stderr info + read_loop 返 0 |
| `level` | `task_id`, `level` | — | 切换工作流档位（`simple` / `standard` / `advanced` / `auto`） | `Concierge::dispatch_slash("level {level}")`（复用 LevelCmd parse_args 校验） |
| `lang` | `locale` | `task_id` | 切换语言（`zh-CN` / `en-US`） | `locale::init`（GUI trusted caller，跳过 LangCmd 校验） |
| `help` | — | — | 列可用 slash commands | `command_registry.list()` 输出到 stderr |

**非法 action** → stderr `error: stdin parse failed: <serde err>` + 继续读下一行（**不**退出）。

## 4. stderr 协议

人类可读，**GUI 不解析**——仅供用户调试。例：

```
error: config load failed: ...
error: LLM router init failed: ...
error: stdin parse failed: expected value at line 1 column 1
error: stdin read error: ...
error: end task <uuid>: 准备退出会话
info: init complete; run `eflow` to start TUI
info: init cancelled by user
可用命令:
  /help         列出所有命令
  /lang         切换语言
  /level        切换工作流档位
  /model        ...
  /profile      ...
  /quit         ...
```

## 5. exit code

| code | 含义 | 触发 |
|---|---|---|
| 0 | ok | 正常退出（end / EOF / SystemShutdown / init 完成） |
| 1 | 用户错误 | 参数非法 / 文件不存在 / KEY 无效 / 用户 Esc 取消 init |
| 2 | 系统错误 | 网络失败 / 文件 IO 错误 / 内部错误 |
| 130 | Ctrl+C | 用户按 Ctrl+C |

**EflowError → exit code 映射**（`src/cli/error.rs::exit_code`）：

| EflowError 变体 | exit code |
|---|---|
| `Config` / `LlmAuthFailed` / `ProfileNotFound` / `SkillNotFound` / `PermissionDenied` / `RiskEscalated` / `TaskCancelled` / `Serialization` | 1（用户错误） |
| `LlmProvider` / `RateLimited` / `Io` / `Memory` / `Internal` / `Tool` | 2（系统错误） |

## 6. Python 套壳示例

```python
import subprocess, json

proc = subprocess.Popen(
    ["eflow", "session", "start", "--lang", "en-US"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    bufsize=1,  # 行缓冲
)

# 读第一行 SystemReady（GUI 知道 eflow 启动完成）
first_event = json.loads(proc.stdout.readline())
assert first_event["event_type"] == "SystemReady"

# 写 send 指令
proc.stdin.write(json.dumps({"action": "send", "task": "review main.rs"}) + "\n")
proc.stdin.flush()

# 持续读 events
for line in proc.stdout:
    event = json.loads(line)
    if event["event_type"] == "TaskStarted":
        show_progress(event["description"])
    elif event["event_type"] == "TaskCompleted":
        handle_result(event["summary"])
    elif event["event_type"] == "TaskFailed":
        handle_error(event["error"])
    elif event["event_type"] == "SystemShutdown":
        break

# 写 end 指令
proc.stdin.write(json.dumps({"action": "end", "task_id": task_id}) + "\n")
proc.stdin.flush()
proc.wait()  # exit code 0
```

## 7. 事件时序

```
start 启动
  ↓
stdout 第 1 行: SystemReady
  ↓
stdin 收到 {action: "send", task: "..."}
  ↓
stdout 输出: TaskStarted
  ↓
stdout 输出: TaskCompleted / TaskFailed
  ↓
（循环 send / level / lang / help）
  ↓
stdin 收到 {action: "end", ...}
  ↓
stdout 输出: SystemShutdown
  ↓
进程退出码 0
```

## 8. 版本约定

- v1.3 契约**冻结**——不破坏性变更
- 新增事件 / action / exit code → 走 v1.4 ADR
- **不**引入 `protocol_version` 字段——GUI 和 eflow 必须同版本部署

## 9. 实现引用

- `src/cli/` —— 实现源码（start / init / stdin / handlers / output / error）
- `src/infrastructure/event.rs` —— Event enum 7 个变体
- `tests/gui_smoke_test.py` —— 9 步 Python 集成测试
- `docs/superpowers/specs/2026-06-17-eflow-v1.3-b2-cli-contract-design.md` —— 完整 spec

## 10. Plan deviations（v1.3.2 实施 vs plan 假设）

| # | 偏差 | 影响 |
|---|---|---|
| #12q | plan 假设 stdin handler 输出 `{"ack": ..., ...}` JSON | v1.3.2 实际**不**输出 ack——GUI 通过事件流推断状态 |
| #12r | plan 假设 TUI/GUI 共用 SlashCommand trait impl 路径 | 实际 handler 直调 Concierge，不绕 SlashCommand（除 level） |
| #12s | plan 假设 provider dir 是 `~/.eflow/providers/` | 实际是 `dirs::config_dir() + /eflow/providers/`（Linux 上是 `~/.config/eflow/providers/`） |
