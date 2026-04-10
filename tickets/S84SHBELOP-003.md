# S84SHBELOP-003: Extend PlanAttemptTrace with frontier-exhaustion diagnostics

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — decision trace types (`worldwake-ai`)
**Deps**: None

## Problem

When ShareBelief goals (or any goal) frontier-exhaust at depth 0, the `PlanAttemptTrace` records only the outcome (`FrontierExhausted`) and expansion count. It does not record why no candidates were found — how many operators were checked, how many target entities matched, or why zero targets appeared. This makes debugging frontier exhaustion difficult without adding ad-hoc logging (FND-29).

## Assumption Reassessment (2026-04-10)

1. **`PlanAttemptTrace` structure confirmed**: At `decision_trace.rs:856-866`, fields are `goal`, `opportunity_anchor`, `outcome`, `target_belief_presence`, `binding_rejections`, `expansion_summaries`. No existing field captures operator/target counts for frontier-exhausted goals.
2. **`CandidateGenerationDiagnostics` exists but is separate**: At `candidate_generation.rs:159-168`, this struct tracks social candidate omissions at the candidate generation stage. The diagnostic enhancement here targets the search stage — a separate concern.
3. **Shared boundary**: The diagnostic data is produced in `search/mod.rs` or `search/candidates.rs` during the search loop, and recorded in `PlanAttemptTrace` which is already the trace sink for plan attempts.

## Architecture Check

1. Extending `PlanAttemptTrace` with search-stage diagnostics is the natural location — the struct already records per-attempt outcomes and binding rejections. Adding operator/target counts follows the existing pattern.
2. No backward-compatibility shims. New fields use `Default` (zero counts, empty reason) so existing trace consumers are unaffected.

## Verification Layers

1. Frontier-exhausted trace records operator count -> focused unit test
2. Frontier-exhausted trace records zero-target reason -> focused unit test
6. Single-layer ticket: purely additive diagnostic fields on an existing trace struct.

## What to Change

### 1. Add diagnostic fields to `PlanAttemptTrace`

In `crates/worldwake-ai/src/decision_trace.rs`, add to `PlanAttemptTrace`:

```rust
/// Number of action defs checked during search (from relevant_ops).
pub operators_checked: u16,
/// Number of target entities found in snapshot matching the operator's TargetSpec.
pub snapshot_targets_found: u16,
/// If zero targets: the reason. Empty when targets were found.
pub zero_target_reason: Option<ZeroTargetReason>,
```

Add a new enum:

```rust
pub enum ZeroTargetReason {
    /// No entities of the required kind at the actor's place in the snapshot.
    NoMatchingEntitiesAtPlace,
    /// Listener entity is in the snapshot's entity set but not indexed at the actor's place.
    EvidenceEntityNotIndexedAtPlace,
    /// No agents in the snapshot at all.
    NoAgentsInSnapshot,
}
```

### 2. Populate diagnostics during search

In `crates/worldwake-ai/src/search/mod.rs`, when building the `PlanAttemptTrace` after search completes with `FrontierExhausted`:
- Count the relevant action defs checked
- Count the target entities found in the snapshot matching each def's `TargetSpec`
- If zero targets, determine the reason by checking the snapshot's entity index

### 3. Add focused tests

Test that a frontier-exhausted search produces a `PlanAttemptTrace` with correct operator count, zero target count, and appropriate reason.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add fields and enum)
- `crates/worldwake-ai/src/search/mod.rs` (modify — populate diagnostics)

## Out of Scope

- Changing candidate generation diagnostics (`CandidateGenerationDiagnostics`)
- Using diagnostics to gate search (that's S84SHBELOP-002)
- Fixing the root cause of frontier exhaustion (that's S84SHBELOP-001)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: frontier-exhausted `PlanAttemptTrace` records correct `operators_checked` count
2. New focused test: frontier-exhausted `PlanAttemptTrace` records `snapshot_targets_found: 0` and appropriate `ZeroTargetReason`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `PlanAttemptTrace` remains `Serialize`/`Deserialize` — new fields and enum must derive the same traits
2. Non-frontier-exhausted traces have `zero_target_reason: None` and non-zero counts
3. Existing trace consumers compile without changes (new fields have defaults)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` or `crates/worldwake-ai/src/search/mod.rs` (test module) — frontier exhaustion diagnostic fields

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
