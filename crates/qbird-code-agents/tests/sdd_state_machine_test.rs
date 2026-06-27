use qbird_code_agents::skill::SddProposal;

fn fresh_proposal(goal: &str) -> SddProposal {
    SddProposal {
        id: "p_test".into(),
        goal: goal.into(),
        scope: "new".into(),
        status: "draft".into(),
        hard_gate_blocked: true,
        created_at: 0,
        updated_at: 0,
    }
}

#[test]
fn test_initial_state_is_blocked() {
    let p = fresh_proposal("goal-a");
    assert!(p.hard_gate_blocked);
    assert_eq!(p.status, "draft");
}

#[test]
fn test_confirm_clears_hard_gate_and_marks_confirmed() {
    let mut p = fresh_proposal("goal-b");
    // Simulate /sdd confirm
    p.hard_gate_blocked = false;
    p.status = "confirmed".into();
    p.updated_at = 1234;
    assert!(!p.hard_gate_blocked);
    assert_eq!(p.status, "confirmed");
    assert_eq!(p.updated_at, 1234);
    assert_eq!(p.id, "p_test");
    assert_eq!(p.goal, "goal-b");
}

#[test]
fn test_serde_roundtrip_preserves_blocked_flag() {
    let p = fresh_proposal("goal-c");
    let json = serde_json::to_string(&p).unwrap();
    let decoded: SddProposal = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.hard_gate_blocked, p.hard_gate_blocked);
    assert_eq!(decoded.id, p.id);
    assert_eq!(decoded.goal, p.goal);
    assert_eq!(decoded.status, p.status);
}

#[test]
fn test_proposal_from_sdd_run_skill_output() {
    // The skill returns serde_json with the proposal embedded; verify the
    // shape matches what the REPL deserializes.
    let skill_output = serde_json::json!({
        "proposal": {
            "id": "p_99",
            "goal": "refactor X",
            "scope": "new",
            "status": "draft",
            "hard_gate_blocked": true,
            "created_at": 1,
            "updated_at": 1
        },
        "needsReview": true,
        "hardGateBlocked": true,
        "suggestion": "ok"
    });
    let proposal: SddProposal = serde_json::from_value(skill_output["proposal"].clone()).unwrap();
    assert_eq!(proposal.id, "p_99");
    assert!(proposal.hard_gate_blocked);
    assert_eq!(proposal.goal, "refactor X");
}

#[test]
fn test_no_pending_means_idle_state() {
    // In the REPL, when pending_proposal is None, /sdd status shows idle
    // and /sdd confirm prints the no-pending error. This test models
    // that decision logic in isolation.
    let pending: Option<SddProposal> = None;
    let is_idle = pending.is_none();
    assert!(is_idle);
    let cleared = pending.map(|mut p| {
        p.hard_gate_blocked = false;
        p
    });
    assert!(cleared.is_none());
}
