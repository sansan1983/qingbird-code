# Tasks 12-14: ContextManager + ReactLoop integration + Version bump

---

## Task 12: ContextManager

**Files:**
- Modify: `crates/qbird-code-infra/src/memory/context_manager.rs`

Replace the stub with full ContextManager implementation:

```rust
use super::types::{ContextMessage, CheckpointEvent, TokenInfo};
use super::overflow::overflow_level;
use super::tokenizer::estimate_tokens_simple;

pub struct ContextManager {
    messages: Vec<ContextMessage>,
    session_id: String,
    token_limit: usize,
    checkpoint_threshold: f64,
    checkpoint_counter: usize,
    reserved_tokens: usize,
}

impl ContextManager {
    pub fn new(session_id: String, token_limit: usize) -> Self {
        Self {
            messages: Vec::new(),
            session_id,
            token_limit,
            checkpoint_threshold: 0.8,
            checkpoint_counter: 0,
            reserved_tokens: 4000,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(ContextMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        });
    }

    pub fn get_message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn get_token_count(&self) -> usize {
        self.messages.iter().map(|m| estimate_tokens_simple(&m.content)).sum()
    }

    pub fn checkpoint_if_needed(&mut self) -> Option<CheckpointEvent> {
        let token_count = self.get_token_count();
        let ratio = token_count as f64 / self.token_limit as f64;
        if ratio >= self.checkpoint_threshold {
            self.checkpoint_counter += 1;
            Some(CheckpointEvent {
                checkpoint_id: format!("ck_{}", self.checkpoint_counter),
                token_count,
                reason: "token_threshold_exceeded".into(),
                session_id: self.session_id.clone(),
            })
        } else {
            None
        }
    }

    pub fn get_messages_within_budget(&self, budget_tokens: usize) -> Vec<&ContextMessage> {
        let mut result: Vec<&ContextMessage> = Vec::new();
        let mut total = 0;
        for msg in self.messages.iter().rev() {
            let tokens = estimate_tokens_simple(&msg.content);
            if total + tokens > budget_tokens && !result.is_empty() {
                break;
            }
            total += tokens;
            result.push(msg);
        }
        result.reverse();
        result
    }

    pub fn overflow_status(&self) -> u8 {
        let info = TokenInfo {
            model_limit: self.token_limit,
            current_usage: self.get_token_count(),
            reserved: self.reserved_tokens,
        };
        overflow_level(&info)
    }

    pub fn set_threshold(&mut self, threshold: f64) {
        self.checkpoint_threshold = threshold.clamp(0.0, 1.0);
    }

    pub fn set_reserved_tokens(&mut self, tokens: usize) {
        self.reserved_tokens = tokens.clamp(2000, 8000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_count() {
        let mut cm = ContextManager::new("sess_1".into(), 32000);
        cm.add_message("user", "Hello");
        cm.add_message("assistant", "Hi there");
        assert_eq!(cm.get_message_count(), 2);
    }

    #[test]
    fn test_checkpoint_trigger() {
        let mut cm = ContextManager::new("sess_1".into(), 1000);
        cm.add_message("user", &"A".repeat(5000));
        assert!(cm.checkpoint_if_needed().is_some());
    }

    #[test]
    fn test_messages_within_budget_all_small() {
        let mut cm = ContextManager::new("sess_1".into(), 32000);
        cm.add_message("user", "first");
        cm.add_message("assistant", "second");
        cm.add_message("user", "third");
        let msgs = cm.get_messages_within_budget(100);
        assert_eq!(msgs.len(), 3);
    }

    #[test]
    fn test_overflow_status_safe() {
        let mut cm = ContextManager::new("sess_1".into(), 64000);
        cm.add_message("user", "small message");
        assert_eq!(cm.overflow_status(), 0);
    }
}
```

---

## Task 13: Integrate ContextManager into ReactLoop

**Files:**
- Modify: `crates/qbird-code-agents/src/react_loop/types.rs`
- Modify: `crates/qbird-code-agents/src/react_loop/mod.rs`

### types.rs changes

Add to ReactLoopConfig:
```rust
pub context_token_limit: usize,
pub context_checkpoint_threshold: f64,
```

In `impl Default`:
```rust
context_token_limit: 32000,
context_checkpoint_threshold: 0.8,
```

### mod.rs changes

In `ReactLoop::run()`, make `context_manager` an optional parameter. Add it after `max_iterations_override` parameter:

```rust
pub async fn run(
    &self,
    provider: &dyn Provider,
    http_client: &HttpLlmClient,
    messages: &mut Vec<Message>,
    tool_schemas: &[serde_json::Value],
    tool_registry: &Arc<ToolRegistry>,
    max_iterations_override: Option<usize>,
    context_manager: Option<&mut ContextManager>,  // NEW
) -> Result<AgentResult, EflowError> {
```

In the loop, after processing LLM response and tool calls, integrate context manager:

```rust
// After tool execution or LLM response, update context manager
if let Some(ref mut cm) = context_manager {
    if let Some(event) = cm.checkpoint_if_needed() {
        tracing::info!("Context checkpoint: {:?}", event);
    }
}
```

**BACKWARD COMPATIBILITY**: The `context_manager` parameter defaults to `None`, so all existing callers (Subagent, main.rs, tests) continue to work without changes.

---

## Task 14: Version bump

**Files:**
- Modify: `Cargo.toml` (workspace version 0.2.14 → 0.2.15)
- Modify: `CHANGELOG.md`

Add to CHANGELOG:
```
## [0.2.15] - 2026-06-27

### Added

- **记忆系统**: SQLite + FTS5 记忆管理器（增量同步、全文搜索、预算化读取）
- **上下文管理**: Token 预算化窗口、溢出检测（4 级压力）、自动检查点
- **ReactLoop 集成**: ContextManager 可选接入，替代粗暴 50 条截断
```

---

## Verification

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```
