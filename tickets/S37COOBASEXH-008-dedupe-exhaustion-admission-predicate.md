# S37COOBASEXH-008: Deduplicate exhaustion admission predicate in planning and trace summarization

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `agent_tick/planning.rs` helper extraction and focused regression coverage
**Deps**: `specs/S37-cooldown-based-exhaustion.md`, S37COOBASEXH-004, S37COOBASEXH-006

## Problem

The cooldown-aware exhaustion admission predicate currently exists twice in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs): once in `build_candidate_plans()` and once in `summarize_same_goal_planning_trace()`. Both sites currently agree, but they duplicate the same `OpportunityKey` + exhaustion-cache + `current_tick` decision. That duplication is a maintenance risk: if one site changes and the other does not, decision traces can stop describing the planner’s real admission behavior. This is worth fixing because it directly affects architectural legibility and debuggability, not just style.

## Assumption Reassessment (2026-03-29)

1. The exact boundary under audit is the exhaustion-driven admission contract for one `OpportunityKey` at one `Tick`, as read from `runtime.exhaustion_cache` by `build_candidate_plans()` and `summarize_same_goal_planning_trace()` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs).
2. Live code already implements the correct cooldown semantics. Both functions currently reject `Some(entry) if entry.suppresses_planning()` and `Some(entry) if !entry.is_retry_eligible(current_tick)`, and otherwise admit the opportunity. The issue is duplication, not incorrect behavior.
3. This is a mixed proof-surface ticket, but still a single-module change. The important shared contract is “trace summarization must derive admission from the same predicate that controls real planning admission,” not a broader cross-crate data contract.
4. No failing golden scenario motivates this ticket. The invariant is architectural: trace reporting must stay causally aligned with actual planner admission so debugging remains trustworthy under `docs/FOUNDATIONS.md` Principle 27 and the repo’s decision-trace guidance.
5. This is planner-path work, but not tied to one live `GoalKind`. The predicate applies uniformly to every ranked opportunity keyed by `OpportunityKey`, regardless of whether the underlying goal is `AcquireCommodity`, `Sleep`, `TreatWounds`, or another family.
6. This is not an AI behavior regression ticket. Focused unit coverage in the `agent_tick::planning` module is the correct verification layer; no full action registries or golden harness setup should be needed to prove the helper extraction.
7. No ordering claim is under audit. The concern is admission equivalence between two planner-path readers of the same exhaustion state, not ranking or branch ordering.
8. This ticket is not weakening a heuristic or filter. It preserves the current cooldown-aware filter exactly and centralizes it so future changes do not create planner/trace divergence.
9. No stale-request, contested-affordance, or action start-failure boundary is involved.
10. No ControlSource, queued input, or driver reset behavior is involved.
11. Existing focused coverage already proves the current semantics around this predicate: `agent_tick::planning::tests::cooldown_ineligible_entry_is_filtered_out_of_candidate_plans`, `agent_tick::planning::tests::cooldown_ineligible_entry_does_not_block_later_same_goal_sibling`, `agent_tick::planning::tests::has_pending_budget_retry_detects_retryable_budget_entries`, and `agent_tick::planning::tests::frontier_exhaustion_suppresses_planning_but_budget_retry_does_not`.
12. Adjacent contradiction classification: the duplication itself is future cleanup worth fixing now; it is not a separate production bug and does not require broadening S37COOBASEXH-005, -006, or -007.
13. Mismatch + correction: none of the remaining active S37 tickets currently owns this DRY/alignment cleanup. S37COOBASEXH-005 is about recording, S37COOBASEXH-006 is about exposing exhaustion state in traces, and S37COOBASEXH-007 is about serialization/versioning. Overloading any of them would blur ownership.
14. The proposed cleanup aligns with `docs/FOUNDATIONS.md`: it improves P27 debuggability by keeping trace summaries mechanically tied to planner truth, and it avoids introducing a parallel alias path or compatibility layer.
15. No cumulative arithmetic or threshold math changes are proposed. Cooldown timing remains entirely governed by `ExhaustionEntry::is_retry_eligible(current_tick)`.

## Architecture Check

1. Extracting a single private helper for exhaustion-based admission is cleaner than maintaining two inline `match` blocks. It reduces drift risk at the exact spot where planner behavior and planner explanation must stay identical.
2. This is more robust than leaving duplication in place because upcoming exhaustion-related work, especially trace-oriented work in S37COOBASEXH-006, is more likely to touch this area. One helper lowers the chance of “trace says admitted/skipped” while the planner did something else.
3. The helper should remain narrow and local to `agent_tick/planning.rs`. This is not a new cross-module abstraction and should not become a generic policy service.
4. No backwards-compatibility aliasing or shims are introduced. The old duplicated path should be replaced, not wrapped and retained.

## Verification Layers

1. Real planning admission and same-goal trace summarization consult the same exhaustion predicate -> focused unit test in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
2. Cooldown-ineligible retry entries remain filtered from actual search admission -> existing focused unit tests in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
3. Frontier exhaustion remains a hard suppression path while eligible retries remain admissible -> existing focused unit tests in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs)
4. This ticket does not rely on a later trace rendering layer as a proxy for admission correctness. The contract is proven directly at the planning helper and call-site behavior.
5. This is effectively a single-module cleanup ticket. Additional authoritative/event-log/action-trace mapping is not applicable because no world-state mutation changes.

## What to Change

### 1. Extract one private exhaustion-admission helper

In [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), add a small private helper such as:

```rust
fn opportunity_admitted_by_exhaustion(
    exhaustion_cache: &BTreeMap<OpportunityKey, ExhaustionEntry>,
    opportunity: OpportunityKey,
    current_tick: Tick,
) -> bool
```

The helper should preserve the current semantics exactly:

- reject `FrontierExhausted`,
- reject cooldown-ineligible `BudgetRetryPending`,
- admit missing entries and retry-eligible entries.

### 2. Replace the duplicated inline `match` blocks

Use the helper from both:

- `build_candidate_plans()`
- `summarize_same_goal_planning_trace()`

No behavior change is intended. This is a single-source-of-truth refactor within the same module.

### 3. Add one focused equivalence regression test

Add a test that proves the same opportunity set is admitted by both code paths for a mixed exhaustion cache containing:

- a frontier-suppressed opportunity,
- a cooldown-ineligible retry opportunity,
- a retry-eligible opportunity,
- and a fresh opportunity with no exhaustion entry.

The test should fail if one call site is updated and the other is not.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)

## Out of Scope

- `ExhaustionEntry` schema or cooldown arithmetic changes
- `has_pending_budget_retry()` behavior changes
- Exhaustion recording changes
- Decision trace rendering/output formatting changes
- Save/load or wire-format changes
- Any cross-module helper extraction beyond `agent_tick/planning.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `build_candidate_plans()` and `summarize_same_goal_planning_trace()` derive admission from the same helper and stay behaviorally aligned
2. Existing focused cooldown-admission tests still pass unchanged
3. Existing suite: `cargo test -p worldwake-ai -- planning`

### Invariants

1. There is exactly one planner-local implementation of the exhaustion-based opportunity admission rule inside `agent_tick/planning.rs`
2. Same-goal planning traces cannot drift from actual planner admission due to duplicated exhaustion-filter logic

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — add a focused regression test proving `build_candidate_plans()` and `summarize_same_goal_planning_trace()` stay aligned across frontier-suppressed, cooldown-ineligible, retry-eligible, and fresh opportunities
2. [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — keep existing cooldown-admission tests as the surrounding semantic proof surface

### Commands

1. `cargo test -p worldwake-ai -- planning`
2. `cargo test -p worldwake-ai -- cooldown`
3. `cargo clippy --workspace`
