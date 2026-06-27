# Tasks 15-18: SDD Workflow (v0.2.16)

---

## Task 15: Skill plugin system

**Create: `crates/qbird-code-agents/src/skill/types.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoTrigger {
    Always,
    Conditional,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub level: String,
    pub blocking: bool,
    pub auto_trigger: AutoTrigger,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillContext {
    pub session_id: String,
    pub project_path: Option<String>,
    pub budget_remaining: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub metrics: SkillMetrics,
    pub errors: Vec<SkillError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetrics {
    pub duration_ms: u64,
    pub tokens_used: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillError {
    pub code: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddProposal {
    pub id: String,
    pub goal: String,
    pub scope: String,
    pub status: String,
    pub hard_gate_blocked: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[async_trait::async_trait]
pub trait Skill: Send + Sync {
    fn descriptor(&self) -> SkillDescriptor;
    async fn execute(&self, input: serde_json::Value, context: SkillContext) -> SkillResult;
}
```

**Create: `crates/qbird-code-agents/src/skill/registry.rs`**

```rust
use std::collections::HashMap;
use std::sync::Arc;

use super::types::{Skill, SkillDescriptor, SkillContext, SkillResult, AutoTrigger};
use qbird_code_models::EflowError;

pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self { skills: HashMap::new() }
    }

    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        let id = skill.descriptor().id.clone();
        self.skills.insert(id, skill);
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn Skill>> {
        self.skills.get(id)
    }

    pub fn list(&self) -> Vec<SkillDescriptor> {
        let mut all: Vec<SkillDescriptor> = self.skills.values().map(|s| s.descriptor()).collect();
        all.sort_by(|a, b| a.level.cmp(&b.level).then(a.id.cmp(&b.id)));
        all
    }

    pub fn match_auto(&self) -> Vec<SkillDescriptor> {
        self.skills.values()
            .filter(|s| s.descriptor().auto_trigger == AutoTrigger::Always)
            .map(|s| s.descriptor())
            .collect()
    }

    pub async fn execute(&self, id: &str, input: serde_json::Value, context: SkillContext) -> Result<SkillResult, EflowError> {
        let skill = self.skills.get(id)
            .ok_or_else(|| EflowError::SkillNotFound(id.to_string()))?;
        Ok(skill.execute(input, context).await)
    }
}
```

**Create: `crates/qbird-code-agents/src/skill/mod.rs`**

```rust
pub mod types;
pub mod registry;
pub mod sdd;

pub use types::*;
pub use registry::SkillRegistry;
```

Update `crates/qbird-code-agents/src/lib.rs`:
```rust
pub mod skill;
```

---

## Task 16: SDD 4 skills

**Create: `crates/qbird-code-agents/src/skill/sdd/mod.rs`**

```rust
pub mod proposal;
pub mod review_spec;
pub mod review_quality;
pub mod archive;

pub use proposal::SddProposalSkill;
pub use review_spec::SddReviewSpecSkill;
pub use review_quality::SddReviewQualitySkill;
pub use archive::SddArchiveSkill;

pub fn register_all(registry: &mut crate::skill::registry::SkillRegistry) {
    registry.register(SddProposalSkill::new());
    registry.register(SddReviewSpecSkill::new());
    registry.register(SddReviewQualitySkill::new());
    registry.register(SddArchiveSkill::new());
}
```

**Create: `crates/qbird-code-agents/src/skill/sdd/proposal.rs`**

```rust
use async_trait::async_trait;
use std::sync::Arc;

use super::super::types::{Skill, SkillDescriptor, SkillContext, SkillResult, SkillMetrics, SddProposal, AutoTrigger};

pub struct SddProposalSkill;

impl SddProposalSkill {
    pub fn new() -> Arc<Self> { Arc::new(Self) }
}

#[async_trait]
impl Skill for SddProposalSkill {
    fn descriptor(&self) -> SkillDescriptor {
        SkillDescriptor {
            id: "sdd-proposal".into(),
            name: "SDD Proposal".into(),
            description: "Generate structured SDD proposal from user requirements with HARD-GATE".into(),
            version: "1.0.0".into(),
            level: "extension".into(),
            blocking: true,
            auto_trigger: AutoTrigger::Conditional,
            input_schema: serde_json::json!({"type":"object","required":["userInput"],"properties":{"userInput":{"type":"string"}}}),
            output_schema: serde_json::json!({"type":"object","required":["proposal","needsReview","hardGateBlocked"]}),
            tags: vec!["sdd".into(), "design".into()],
        }
    }

    async fn execute(&self, input: serde_json::Value, _context: SkillContext) -> SkillResult {
        let user_input = input["userInput"].as_str().unwrap_or("");
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
        let proposal = SddProposal {
            id: format!("p_{}", now),
            goal: user_input.to_string(),
            scope: "new".into(),
            status: "draft".into(),
            hard_gate_blocked: true,
            created_at: now,
            updated_at: now,
        };
        SkillResult {
            success: true,
            output: serde_json::json!({"proposal":proposal,"needsReview":true,"hardGateBlocked":true,"suggestion":"Proposal generated. HARD-GATE blocked - waiting for user confirmation."}),
            metrics: SkillMetrics { duration_ms: 0, tokens_used: None },
            errors: vec![],
        }
    }
}
```

**Create: `crates/qbird-code-agents/src/skill/sdd/review_spec.rs`** (same pattern, id: "sdd-review-spec", non-blocking)
**Create: `crates/qbird-code-agents/src/skill/sdd/review_quality.rs`** (same pattern, id: "sdd-review-quality", non-blocking)
**Create: `crates/qbird-code-agents/src/skill/sdd/archive.rs`** (same pattern, id: "sdd-archive", non-blocking)

---

## Task 17: Integrate into ReactLoop + CLI

**Modify: `crates/qbird-code/src/main.rs`**

In main.rs:
1. Import SkillRegistry and sdd::register_all
2. After ToolRegistry, create and populate SkillRegistry:

```rust
let mut skill_registry = qbird_code_agents::skill::SkillRegistry::new();
qbird_code_agents::skill::sdd::register_all(&mut skill_registry);
let skill_registry = Arc::new(skill_registry);
```

In interactive mode `/sdd` command:
```rust
"/sdd" => {
    if arg.is_empty() {
        println!("SDD workflow commands:");
        println!("  /sdd run     Run SDD workflow");
        println!("  /sdd confirm  Confirm current HARD-GATE");
        println!("  /sdd status   Show SDD status");
    } else {
        // Handle sub-commands
    }
}
```

No changes to ReactLoop.run() signature - Skill system is used at the CLI level.

---

## Task 18: Version bump

Cargo.toml: 0.2.15 → 0.2.16

CHANGELOG:
## [0.2.16] - 2026-06-27

### Added
- **Skill 插件体系**: SkillRegistry + Skill trait 注册表
- **SDD 四阶段工作流**: Proposal（含 HARD-GATE）/ Spec Review / Quality Review / Archive
- **CLI 集成**: `/sdd` 斜杠命令组

---

## Verification

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```
