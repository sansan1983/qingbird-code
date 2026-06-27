# Task 1 Report: Fix flaky mock integration test

## What I implemented

Fixed the flaky `react_loop_with_mock_test` by addressing the real root cause: **improper HTTP request body consumption**.

**Root cause analysis:**
The mock server read HTTP headers (stopping at `\r\n\r\n`) but never consumed the request body. On Windows, dropping a `TcpStream` with unread data in the receive buffer sends a **TCP RST** instead of a graceful FIN. This caused `reqwest` to fail on subsequent connections with `"error sending request for url"`, as the RST interfered with HTTP connection management.

**Fix applied:**
1. Parse `Content-Length` from request headers (case-insensitive)
2. Read the full request body before responding
3. Call `stream.flush()` after writing the response
4. Changed chunk size from 1024 to 4096 for efficiency

The plan's suggested `oneshot`/`mpsc` ready-signal approach was tried but proved insufficient — the true cause was the RST from unread body data, not thread startup timing.

## Test results

- **Flaky test (10 consecutive runs):** All passed ✓
- **Full gate suite:** 112 passed, 1 ignored (Ollama smoke — needs local Ollama), 0 failed
- **Clippy:** Clean (no warnings with `-D warnings`)
- **Fmt:** Clean

## Files changed

- `crates/qbird-code-agents/tests/react_loop_with_mock_test.rs`

## Commit

```
7f3655b fix: mock server flaky test — consume full HTTP request body before responding
```

## Self-review findings

No concerns. The test is now deterministic — verified by 10 consecutive passes.

## Issues or concerns

None.
