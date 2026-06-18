# v1.3.1 手工验证清单（14 步）

> **状态**：待用户在**真 TTY 终端**跑。Claude 不可执行环境下无法验证（ratatui 需 TTY，无 TTY 立即 panic）。
>
> **执行要求**：在 PR 合并前由 reviewer 在真终端按顺序跑完本表，每步填 ✅/❌。
> 如有 ❌，按对应"Expected"行下面的诊断提示排查，参考 [TROUBLESHOOTING](#troubleshooting) 节。

## 验证环境前置

```bash
# 1. 切到 v1.3.1 分支（已 checkout）
git branch --show-current   # 应输出 v1.3.1

# 2. 编译 release（手工验证用 release，与 dev 行为基本一致；release 启动更快）
cargo build --release

# 3. 备份现有配置（防止误操作清掉）
[ -f ~/.eflow/config.yaml ] && cp ~/.eflow/config.yaml ~/.eflow/config.yaml.bak
```

## 14 步验证

| # | 步骤 | 期望 | 结果 |
|---|------|------|------|
| 1 | bare TUI 启动：`rm -rf ~/.eflow/config.yaml; echo "n" \| cargo run --release` | TUI 启动，header 显示 `⚠ 未配置 LLM provider` | ☐ |
| 2 | 向导走完：`rm -rf ~/.eflow/config.yaml; echo "y" \| cargo run --release` | 走完 7 步向导，写 `~/.eflow/config.yaml` + `~/.eflow/providers/{id}.yaml` | ☐ |
| 3 | `eflow init` 强制：`cargo run --release -- init` | 强制进向导（无论配置是否存在） | ☐ |
| 4 | preset 跳过 protocol：向导 step 2 选 1（deepseek） | step 3 显示 "(预设厂商已自动选择协议，已跳过)" → step 4 KEY → step 5 model → step 6 确认 | ☐ |
| 5 | TUI /help：正常 TUI 启动（已有 config）→ prompt 输入 `/help` | 列出 6 个命令（model / profile / lang / level / help / quit） | ☐ |
| 6 | TUI /quit：TUI 输入 `/quit` | 显示"正在退出..." → 退出 | ☐ |
| 7 | /level 空壳：TUI 输入 `/level simple` | 显示 `/level 档位切换将在 v1.3.2 启用（spec C）` | ☐ |
| 8 | 向导 Esc 取消：向导 step 1 按 Esc | 退出向导，**不**写任何文件 | ☐ |
| 9 | /model 子视图：TUI 输入 `/model` | 弹 SelectList（输入序号 / ↑↓ 键 / Enter 选中 / Esc 取消） | ☐ |
| 10 | /profile 切换：TUI 输入 `/profile` | 弹 profile 列表，**真改** active_profile（重启后 /show-config 显示新 profile） | ☐ |
| 11 | /lang 切换：TUI 输入 `/lang en-US` | 显示 `Language changed to en-US`，后续消息走英文 | ☐ |
| 12 | 未知命令：TUI 输入 `/nonexistent` | 显示 `Unknown slash command: /nonexistent. Type /help for available commands` | ☐ |
| 13 | 非斜杠兼容：TUI 输入 `读 README` | 走原 v1.2 任务派发，显示派发响应 | ☐ |
| 14 | Ctrl+C 优雅退出：TUI 运行中按 Ctrl+C | 关闭 TUI，pool shutdown，进程退出码 0 | ☐ |

## 验证脚本（自动化辅助，非强制）

```bash
#!/usr/bin/env bash
# scripts/verify-v1.3.1.sh —— 半自动验证（不能完全替代人工，因 ratatui 需真 TTY）
# 跑法：bash scripts/verify-v1.3.1.sh
set -e
cd "$(git rev-parse --show-toplevel)"

echo "=== T12 verify 1-3: bare TUI / 向导 / init 子命令（需真 TTY，提示用户手动跑）==="
echo "Step 1: rm -rf ~/.eflow/config.yaml; echo 'n' | cargo run --release"
echo "Step 2: rm -rf ~/.eflow/config.yaml; echo 'y' | cargo run --release"
echo "Step 3: cargo run --release -- init"
echo "Step 4-14: 在 TUI 内手动跑，参考本表 Expected 列"

echo ""
echo "=== T12 verify 自动化部分 ==="

# 验证 4 门禁（已在 T11 跑过，CI 跑；这里再跑一次做 PR 前 sanity check）
cargo build && echo "✓ cargo build"
cargo clippy --all-targets -- -D warnings && echo "✓ cargo clippy"
cargo fmt --check && echo "✓ cargo fmt"
cargo test --quiet 2>&1 | grep -E "test result:" | awk '{s+=$4; f+=$6} END {if (f == 0) print "✓ cargo test (passed=" s ")"; else exit 1}'

# 验证 T24 TODO 注释已加
echo ""
echo "=== T12 verify T24 TODO 标注 ==="
grep -l "TODO(v1.4 spec D)" src/interaction/wizard/mod.rs src/interaction/tui.rs && echo "✓ T24 TODO 标注在 wizard/mod.rs + tui.rs"
grep -q "v1.3.1 已知偏差" src/interaction/tui.rs && echo "✓ tui.rs 顶部 v1.3.1 已知偏差注释"
grep -q "specs/2026-06-17-eflow-v1.3-b1-wizard-slash-design.md §12" src/interaction/wizard/mod.rs src/interaction/tui.rs && echo "✓ spec §12 引用"

# 验证版本号 + CHANGELOG
grep -q 'version = "1.3.1"' Cargo.toml && echo "✓ Cargo.toml 1.3.1"
grep -q "## \[1.3.1\]" CHANGELOG.md && echo "✓ CHANGELOG.md [1.3.1] 段"
```

## 验证通过判定

- 14 步手工验证**全过** = ✅
- 任一 ❌ = 阻塞 PR 合并；按 troubleshooting 排查

## 验证人签字

- 验证者：
- 日期：
- 设备（OS / 终端 / 终端类型）：
- 备注：

---

## TROUBLESHOOTING

| 现象 | 可能原因 | 排查 |
|------|---------|------|
| bare TUI 启动后 header 不显 `⚠ 未配置` | `TuiBackend::with_bare_mode()` 未生效，main.rs 用了默认 ctor | `src/main.rs` 启动 TUI 前的 `with_bare_mode` 调用是否在配置检测前 |
| 向导 step 2 选 1 后 step 3 没跳过 | preset 与自定义分叉逻辑失效 | `WizardState::skip_protocol_step` 是否在 provider 选 preset 时设 `true` |
| `/model` 输入后没弹 SelectList | `SlashOutput::OpenSubView` 走 err_subview_render_failed 兜底 | `tui.rs` 是否实现 OpenSubView 处理分支（v1.3.1 阶段可能未实装——查 spec §6.3） |
| `/quit` 不退出 | `SlashOutput::Shutdown` 在 main 循环里没读 | TUI 主循环需要支持 `Event::SystemShutdown` 或 `SlashOutput::Shutdown` 走特殊路径 |
| Ctrl+C 不退出 | crossterm event handler 没接 SIGINT | `crossterm::event::EventStream` 应能 catch Ctrl+C；如不退出，pool.shutdown() 是否被调 |
| 测试 `passed=283` 数字对不上 | T11 加了 key 但 T9 加的测试计数漂移 | 跑 `cargo test --quiet 2>&1 \| grep "test result:"` 看每段 split |
| 向导写不出 `~/.eflow/config.yaml` | `dirs::config_dir()` 在某些 env 返回 None | 检查 `eflow init` stderr 是否报"无法确定配置目录" |

## 与 spec §12 已知偏差的对应

- **WizardStep::render()** 直接调 ratatui → 写 TODO 在 `src/interaction/wizard/mod.rs:1-4` ✓
- **SelectList::render()** 直接调 ratatui → 在 `src/interaction/widgets/select_list.rs` 也加 TODO（v1.3.2 spec B2 实施时统一处理）
- **TuiBackend render()** 直接调 ratatui → 写 TODO 在 `src/interaction/tui.rs:1-5` ✓

v1.4 spec D 接手时按 3 个 TODO 锚点找代码改写。
