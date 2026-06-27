# Task 1: 修复 flaky mock 集成测试

**Files:**
- Modify: `crates/qbird-code-agents/tests/react_loop_with_mock_test.rs`

**问题：**
当前 mock server 在独立线程启动，但线程启动有延迟，主线程可能在 mock server 就绪前就发送 HTTP 请求，导致 `error sending request for url` 错误。

**修复方案：**
在 `start_mock_server()` 函数中增加一个就绪信号机制——`std::sync::mpsc::Sender` channel，server 绑定 TCP 端口后立即发送信号，主线程等待信号后再返回。

**具体要求：**
1. 在函数开头绑定 listener 后创建 mpsc channel
2. server 线程绑定成功后通过 `tx.send(())` 通知
3. 主线程调用 `rx.recv()` 等待就绪后再返回 addr
4. 保持函数签名不变：`fn start_mock_server() -> (String, Arc<AtomicUsize>)`
5. 运行 `cargo test -p qbird-code-agents --test react_loop_with_mock_test -- --test-threads=1` 绿色

**不改变：**
- 不改变测试逻辑（仍然 2 轮 LLM 调用）
- 不改变 mock response 内容
- 不改变断言条件
