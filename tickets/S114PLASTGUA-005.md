# S114PLASTGUA-005: Widen ExpectationMismatchPayload with expectation_kind and mismatch_detail

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `ExpectationMismatchPayload` fields widened per FND-28; `SAVE_FORMAT_VERSION` bump.
**Deps**: `archive/tickets/S114PLASTGUA-001.md`

## Problem

S110 pre-declared that S114 would widen `ExpectationMismatchPayload` in place (`archive/specs/S110-decision-history-events.md:237-244`). The ticket 009 AI-side tick step needs the `expectation_kind` + `mismatch_detail` fields to record which of the four kinds failed and what specifically fired (guard invalidator, unmet state predicate, or missing observation). Landing the widening before ticket 009 keeps the new emission path type-safe end-to-end.

## Assumption Reassessment (2026-04-21)

1. `ExpectationMismatchPayload` lives at `crates/worldwake-core/src/decision_event_payload.rs:213-218` with four fields (`agent`, `goal_key`, `step_index`, `expected_materializations`). It is emitted through the `DecisionEventPayload::ExpectationMismatch` arm. No widening is required in `DecisionEventPayload` itself — just the inner struct.
2. Construction sites (5 total, all literal-enumerating):
   - `crates/worldwake-core/src/decision_event_payload.rs:378` (test in same module)
   - `crates/worldwake-ai/src/agent_tick/observation.rs:109` (production)
   - `crates/worldwake-ai/src/agent_tick/observation.rs:1043` (test)
   - `crates/worldwake-cli/src/bin/observer.rs:4240` (production — observer rendering)
   - `crates/worldwake-sim/src/save_load.rs:794` (test)
3. Shared boundary under audit: the `DecisionEventPayload::ExpectationMismatch` wire format. FND-28 permits and mandates no backward-compat decode path — save files pre-dating this widening are intentionally unreadable. `SAVE_FORMAT_VERSION` must bump by 1.
4. The widening introduces two new core-side types (`MismatchDetail`, `InvalidatorTag`) that are already defined in ticket 001's `plan_step_guards` module and re-exported from `worldwake-core`. No additional type-definition work here.
5. The current production emission site at `observation.rs:109` fires mismatch when `expected_materializations` diverges from actual post-action output. After S114, that path continues to work with `expectation_kind: None, mismatch_detail: None` (pre-S114-style detection). The new AI-side tick step (ticket 009) is the site that will populate both with `Some(_)`.

## Architecture Check

1. Widening in place per S110's pre-declaration — no parallel "v2 payload" or shim. Old decode path is deleted, not wrapped.
2. `Option<_>` wrapping on both new fields is deliberate: the pre-S114 emission path at `observation.rs:109` keeps emitting with `None` for both, distinguishing S114-style mismatches from materialization-based mismatches without requiring callers to synthesize placeholder detail.

## Verification Layers

1. Type contract (`ExpectationMismatchPayload` widened fields compile) → `cargo check -p worldwake-core` + dependent crates.
2. All 5 construction sites updated (no compile error post-widening) → `cargo build --workspace` succeeds.
3. Existing tests continue to pass with `expectation_kind: None, mismatch_detail: None` on pre-S114 emission paths → `cargo test -p worldwake-core decision_event_payload`, `cargo test -p worldwake-ai agent_tick::observation`.
4. Save-format-bump contract → existing `load_format_errors_on_outdated_save` at `save_load.rs:1120` asserts the new value rejects prior saves.
5. Single-layer ticket: event-log wire format only. Behavioral population of the new fields arrives in tickets 007 (guard-breach path) and 009 (overdue-expectation path).

## What to Change

### 1. Widen the payload

In `crates/worldwake-core/src/decision_event_payload.rs:213-218`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectationMismatchPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub expected_materializations: Vec<MaterializationTag>,
    /// Which of the four expectation kinds failed. `None` on pre-S114-style
    /// materialization-divergence emissions; `Some(_)` when the emission
    /// originates from the AI-side plan-step tick step (S114 D6).
    pub expectation_kind: Option<ExpectationKindTag>,
    /// Breach diagnostic: guard invalidator tag, unmet state predicate, or
    /// missing observation predicate. `None` when mismatch was detected
    /// pre-S114-style via `expected_materializations` alone.
    pub mismatch_detail: Option<MismatchDetail>,
}
```

Import `ExpectationKindTag` and `MismatchDetail` from `crate::plan_step_guards`.

### 2. Update all 5 construction sites

At each of the 5 sites, add:

```rust
expectation_kind: None,
mismatch_detail: None,
```

These are production-path or test-fixture sites that fire before S114 D6 lands; tickets 007 and 009 will populate both with `Some(_)` where appropriate.

### 3. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, increment by 1 (relative to whatever value is current at implementation time — tickets 002 / 004 / 006 all bump independently).

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — struct + test at line 378)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — production site at line 109, test at line 1043)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — site at line 4240)
- `crates/worldwake-sim/src/save_load.rs` (modify — test at line 794; version bump at line 6)

## Out of Scope

- Populating `expectation_kind` / `mismatch_detail` with non-`None` values. Ticket 009 (AI-side tick step) populates from guard-invalidator / state-predicate / observation-predicate sources. Ticket 007 (revalidation guard-check pass) may also trigger emission with populated values.
- Any new variant on `DecisionEventPayload` — the widening is entirely inside `ExpectationMismatchPayload`.

## Acceptance Criteria

### Tests That Must Pass

1. `crates/worldwake-core/src/decision_event_payload.rs` existing round-trip test covering `ExpectationMismatchPayload` (line ~378) passes with widened struct — update test to include one case with `expectation_kind: Some(_)` and one with `None`.
2. `crates/worldwake-ai/src/agent_tick/observation.rs::*` tests at line 1043 area stay green — update the test fixture's construction to include the two new fields.
3. `crates/worldwake-sim/src/save_load.rs` existing ExpectationMismatch round-trip test at line 794 passes.
4. `load_format_errors_on_outdated_save` at `save_load.rs:1120` asserts prior save is rejected at the new version.
5. Existing suite: `cargo test -p worldwake-core decision_event_payload`, `cargo test -p worldwake-ai agent_tick::observation`, `cargo test -p worldwake-sim save_load` stay green.

### Invariants

1. All 5 `ExpectationMismatchPayload { ... }` construction sites explicitly enumerate every field — no `..Default::default()` spread, no `Default` impl introduced.
2. FND-28: no shim, no v2-payload alias, no transitional fallback path.
3. `SAVE_FORMAT_VERSION` increments by exactly 1 for this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs` (modified test at ~line 378) — add populated-detail case.
2. `crates/worldwake-ai/src/agent_tick/observation.rs` (modified test at ~line 1043) — field-update to new shape.
3. `crates/worldwake-sim/src/save_load.rs` (modified test at ~line 794) — field-update.

### Commands

1. `cargo test -p worldwake-core decision_event_payload`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-sim save_load`
4. `cargo clippy --workspace --all-targets -- -D warnings`
