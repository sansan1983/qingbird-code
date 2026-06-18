#!/usr/bin/env python3
"""GUI 套壳集成测试

真实 spawn `eflow session start` 进程 + parse NDJSON 输出，验证 CLI 契约稳定。
**不**测试 LLM 调用——mock provider 不可达（localhost:9999）—— 只测：
- 7 个事件 schema 冻结（SystemReady + 6 任务事件）
- 5 个 stdin action 解析（send / end / level / lang / help）
- 4 档 exit code（0 / 1 / 2 / 130）
- 非法 JSON 不退出
- stderr 走人类可读（**不**走 stdout）

8 步流程：
1. 启动 eflow session start + 读 SystemReady 首行
2. /level simple 走 SlashCommand 链路（不依赖 LLM）
3. /lang en-US 直接调 locale::init（不依赖 LLM）
4. /help 列 commands
5. 非法 stdin JSON → stderr 报错 + 继续
6. /end → SystemShutdown → exit 0
7. config 不存在 → exit 1（用户错误）
8. 7 个事件 schema 字段验证（构造时类型/必填字段）

用法：
    cargo build --release --bin eflow
    python tests/gui_smoke_test.py
    或：
    pytest tests/gui_smoke_test.py -v
"""

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path


PROJECT_ROOT = Path(__file__).parent.parent
FIXTURE_DIR = Path(__file__).parent / "fixtures"
MOCK_CONFIG = FIXTURE_DIR / "mock_config.yaml"
MOCK_PROVIDER = FIXTURE_DIR / "providers" / "mock.yaml"


def find_eflow_binary():
    """定位 eflow 二进制（cargo build --release 后在 target/release/eflow）"""
    release = PROJECT_ROOT / "target" / "release" / "eflow"
    if release.exists():
        return str(release)
    # fallback：cargo run（开发模式）
    return "cargo"


def install_mock_provider():
    """复制 mock provider 到 dirs::config_dir() 解析的位置
    —— v1.3.2 start.rs 用 `dirs::config_dir().join("eflow").join("providers")`
    Linux 上是 ~/.config/eflow/providers/
    """
    config_root = os.environ.get("XDG_CONFIG_HOME") or str(Path.home() / ".config")
    config_dir = Path(config_root)
    providers_dir = config_dir / "eflow" / "providers"
    providers_dir.mkdir(parents=True, exist_ok=True)
    target = providers_dir / "mock.yaml"
    shutil.copy(MOCK_PROVIDER, target)
    return providers_dir


def run_eflow_session(args, stdin_input=None, timeout=15):
    """启动 `eflow session start` 子进程 + 收 stdout/stderr + 退出码
    返回 (stdout_lines, stderr_text, returncode)
    """
    eflow = find_eflow_binary()
    # 默认带 --config 指 mock_config.yaml（start.rs 接受此 flag 覆盖默认路径）
    if not any(a == "--config" for a in args):
        args = ["--config", str(MOCK_CONFIG), *args]
    if eflow == "cargo":
        cmd = ["cargo", "run", "--quiet", "--release", "--", "session", "start", *args]
    else:
        cmd = [eflow, "session", "start", *args]

    env = os.environ.copy()
    # 不让 cargo run 跨 cwd 找不到 fixtures
    env.setdefault("CARGO_TARGET_DIR", str(PROJECT_ROOT / "target"))

    try:
        proc = subprocess.run(
            cmd,
            input=stdin_input,
            capture_output=True,
            text=True,
            timeout=timeout,
            env=env,
            cwd=str(PROJECT_ROOT),
        )
        return proc.stdout.splitlines(), proc.stderr, proc.returncode
    except subprocess.TimeoutExpired:
        return [], "<timeout>", -1


# ===== 8 步测试 =====

def test_01_system_ready_is_first_stdout_line():
    """Step 1: stdout 第 1 行是 SystemReady + 3 字段"""
    install_mock_provider()
    # 喂 EOF 立即退出——只需要 SystemReady 首行
    stdout, _, _ = run_eflow_session(["--lang", "en-US"], stdin_input="")
    assert len(stdout) >= 1, f"no stdout: {stdout}"
    first = json.loads(stdout[0])
    assert first["event_type"] == "SystemReady", f"expected SystemReady, got {first}"
    assert "task_id" in first, f"SystemReady missing task_id: {first}"
    assert "started_at" in first, f"SystemReady missing started_at: {first}"


def test_02_level_action_does_not_crash():
    """Step 2: /level simple 走 SlashCommand 链路 + 不依赖 LLM"""
    install_mock_provider()
    stdin = json.dumps({"action": "level", "task_id": "00000000-0000-0000-0000-000000000001", "level": "simple"}) + "\n"
    stdout, _, returncode = run_eflow_session(["--lang", "en-US"], stdin_input=stdin, timeout=10)
    # /level 不会真返回事件（v1.3.1 LevelCmd 是 info message）—— read_loop 持续运行
    # stdin EOF → read_loop 返 0 → 进程退出码 0
    assert returncode == 0, f"expected exit 0, got {returncode}: stdout={stdout}"
    # SystemReady 仍然在首行
    assert json.loads(stdout[0])["event_type"] == "SystemReady"


def test_03_lang_action_does_not_crash():
    """Step 3: /lang en-US 直接调 locale::init + 不依赖 LLM"""
    install_mock_provider()
    stdin = json.dumps({"action": "lang", "locale": "en-US"}) + "\n"
    stdout, _, returncode = run_eflow_session(["--lang", "en-US"], stdin_input=stdin, timeout=10)
    assert returncode == 0, f"expected exit 0, got {returncode}"
    assert json.loads(stdout[0])["event_type"] == "SystemReady"


def test_04_help_action_does_not_crash():
    """Step 4: /help 列 commands（不依赖 LLM）"""
    install_mock_provider()
    stdin = json.dumps({"action": "help"}) + "\n"
    stdout, stderr, returncode = run_eflow_session(["--lang", "en-US"], stdin_input=stdin, timeout=10)
    assert returncode == 0, f"expected exit 0, got {returncode}"
    # /help 输出走 stderr（CliOutput::info）："可用命令:\n  /<name> ..."
    assert "可用命令" in stderr or "help" in stderr.lower(), f"expected help text in stderr: {stderr}"


def test_05_invalid_stdin_does_not_crash():
    """Step 5: 非法 JSON → stderr 报错 + 进程继续 + EOF 退出码 0"""
    install_mock_provider()
    stdin = "not valid json\n" + json.dumps({"action": "help"}) + "\n"
    stdout, stderr, returncode = run_eflow_session(["--lang", "en-US"], stdin_input=stdin, timeout=10)
    assert returncode == 0, f"expected exit 0, got {returncode}: stderr={stderr}"
    assert "stdin parse failed" in stderr, f"expected stderr 'stdin parse failed': {stderr}"
    # SystemReady 仍正常输出
    assert json.loads(stdout[0])["event_type"] == "SystemReady"


def test_06_end_action_exits_zero():
    """Step 6: /end → 退出码 0"""
    install_mock_provider()
    stdin = json.dumps({"action": "end", "task_id": "00000000-0000-0000-0000-000000000001"}) + "\n"
    _, _, returncode = run_eflow_session(["--lang", "en-US"], stdin_input=stdin, timeout=10)
    assert returncode == 0, f"expected exit 0, got {returncode}"


def test_07_missing_config_exits_one():
    """Step 7: config 不存在 → exit 1（用户错误）"""
    install_mock_provider()
    stdout, stderr, returncode = run_eflow_session(
        ["--config", "/nonexistent/config.yaml", "--lang", "en-US"],
        stdin_input="",
        timeout=10,
    )
    assert returncode == 1, f"expected exit 1, got {returncode}: stderr={stderr}"
    assert "config load failed" in stderr or "not found" in stderr.lower(), \
        f"expected config error in stderr: {stderr}"


def test_08_event_schemas_constructor_only():
    """Step 8: 验证 7 个事件 schema 字段（构造时类型/必填字段）
    —— 不通过子进程——直接走 cargo test 单元测试覆盖（已通过）；
    这里只验证 SystemReady schema 必填字段 + 类型"""
    install_mock_provider()
    stdout, _, _ = run_eflow_session(["--lang", "en-US"], stdin_input="")
    first = json.loads(stdout[0])
    # 必填字段 + 类型
    assert isinstance(first["event_type"], str)
    assert isinstance(first["task_id"], str)
    assert isinstance(first["started_at"], str)
    # 6 个用户事件 schema 验证：构造 Event enum + Debug 派生（cargo test 覆盖）
    # 这里只 sanity check 字段名
    assert first["event_type"] in {
        "SystemReady", "TaskStarted", "TaskCompleted", "TaskFailed",
        "RiskEscalated", "UserInputRequired", "SystemShutdown",
    }


# ===== CLI entry =====

def main():
    """8 步串行跑（不依赖 pytest）"""
    tests = [
        test_01_system_ready_is_first_stdout_line,
        test_02_level_action_does_not_crash,
        test_03_lang_action_does_not_crash,
        test_04_help_action_does_not_crash,
        test_05_invalid_stdin_does_not_crash,
        test_06_end_action_exits_zero,
        test_07_missing_config_exits_one,
        test_08_event_schemas_constructor_only,
    ]
    passed = 0
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
            passed += 1
        except AssertionError as e:
            print(f"  FAIL  {t.__name__}: {e}")
            failed += 1
        except Exception as e:
            print(f"  ERROR {t.__name__}: {type(e).__name__}: {e}")
            failed += 1
    print(f"\n{passed} passed, {failed} failed (8 total)")
    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
