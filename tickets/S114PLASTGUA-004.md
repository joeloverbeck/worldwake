# S114PLASTGUA-004: ExpectationBasis::PlanStepCompletion variant + ranking cascade

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new `ExpectationBasis` variant; exhaustive-match arm in `ranking.rs`; `SAVE_FORMAT_VERSION` bump.
**Deps**: `archive/tickets/S114PLASTGUA-001.md`

## Problem

S114 D4 requires persisting plan-step expectations through the existing `ExpectationStore` / `ExpectationRecord` infrastructure. The new `PlanStepCompletion { step_index, kind_tag }` variant is the hook; it must remain `Copy`-safe so `ExpectationBasis` and `ExpectationRecord` retain their current `Copy` derives. Landing the variant first unblocks tickets 008 (plan-adoption writes) and 009 (AI-side interpretation of `Overdue` records) without tangling them with cascade bookkeeping.

## Assumption Reassessment (2026-04-21)

1. `ExpectationBasis` at `crates/worldwake-core/src/expectation.rs:22` has five variants today (`DutyAssignment`, `DeliveryCommitment`, `RoutineReturn`, `EscortObligation`, `SocialPromise`) and derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `ExpectationRecord` at `expectation.rs:59` also derives `Copy`. The new variant's payload `{ step_index: u16, kind_tag: ExpectationKindTag }` is two `Copy` primitives, preserving both derives.
2. S114 spec D4 at `specs/S114-plan-step-guards.md:239-266` defines the variant shape and documents the exhaustive-match cascade: `ExpectationBasis` is matched exhaustively at `crates/worldwake-ai/src/ranking.rs:1133-1135`. Per the spec, `PlanStepCompletion` contributes no ranking-relevant weight and maps to `0` (plan-step expectations are agent-internal, not overdue-social-obligation-grade).
3. Shared boundary under audit: the `ExpectationBasis` enum layout. Other sites (`per_agent_belief_view.rs`, `save_load.rs`, `expectation_check.rs` tests, golden tests, `search_actions.rs`, `report_actions.rs`, `ask_about_person_actions.rs`) currently construct specific variants and do not exhaustively match — confirmed via `rg 'match .*\.basis|match basis' crates/worldwake-systems` returning matches only for specific-variant construction. No cascade edit required at those sites.
4. Sim-side `check_overdue_expectations` (`crates/worldwake-systems/src/expectation_check.rs:7`) does **not** exhaustive-match `ExpectationBasis` — it operates on `ExpectationState` transitions only, filtering `state == Active` at `expectation_check.rs:59` regardless of basis. Confirmed by reading the function body. Adding the new variant requires no change to this function. (This is the post-F1-fix architecture landing in the S114 spec.)
5. `SAVE_FORMAT_VERSION = 36` at `crates/worldwake-sim/src/save_load.rs:6` — adding an enum variant changes the variant-tag bincode encoding; bump to 38 (tickets 002 and 005 land ahead of this in dependency order, each bumping by 1; this ticket targets 38 unless merge order differs — adjust at implementation time to current + 1).

## Architecture Check

1. Additive variant only — no existing variants renamed, no construction sites migrated, no `handle_plan_failure` branching added. Tickets 008 and 009 do all the consumer work.
2. The ranking-cascade arm returns `0` because plan-step expectations are agent-internal plan state; they do not rank against duty/delivery/escort obligations which exist at the institutional-social layer.

## Verification Layers

1. Exhaustive-match coverage (ranking.rs adds arm) → `cargo check -p worldwake-ai` compiles with no `non_exhaustive_patterns` warning.
2. Serialization contract (variant round-trips through bincode) → focused unit test appended to `expectation.rs` tests module at line 189.
3. Save-format-bump contract (`SAVE_FORMAT_VERSION` incremented) → existing `load_format_errors_on_outdated_save` test at `save_load.rs:1120` passes with new value.
4. No sim-side cascade required → confirmed via absence of exhaustive match in `check_overdue_expectations` body (Assumption 4). Single-layer ticket beyond that.

## What to Change

### 1. Add the variant

In `crates/worldwake-core/src/expectation.rs:22`, append to `pub enum ExpectationBasis`:

```rust
/// A plan step expects completion by `deadline_tick`. The rich
/// `PlanExpectation` (with its `StatePredicate` / `ObservationPredicate`)
/// lives on the runtime `PlannedStep`; the monitor cross-references by
/// `(step_index, kind_tag)` against the agent's current plan.
PlanStepCompletion { step_index: u16, kind_tag: ExpectationKindTag },
```

Import `ExpectationKindTag` from `crate::plan_step_guards` (re-exported from ticket 001).

### 2. Add the ranking cascade arm

In `crates/worldwake-ai/src/ranking.rs:1133-1135`, extend the exhaustive `match basis { ... }` with:

```rust
ExpectationBasis::PlanStepCompletion { .. } => 0,
```

Add a brief inline comment: `// plan-step expectations are agent-internal; no social-obligation weight`.

### 3. Add bincode round-trip test

Append to `crates/worldwake-core/src/expectation.rs` tests module (line 189+):

```rust
#[test]
fn expectation_basis_plan_step_completion_round_trips_through_bincode() {
    let basis = ExpectationBasis::PlanStepCompletion {
        step_index: 3,
        kind_tag: ExpectationKindTag::State,
    };
    let bytes = bincode::serialize(&basis).unwrap();
    let decoded: ExpectationBasis = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded, basis);

    // Existing variants still round-trip unchanged
    let existing = ExpectationBasis::RoutineReturn;
    assert_eq!(
        bincode::deserialize::<ExpectationBasis>(&bincode::serialize(&existing).unwrap()).unwrap(),
        existing,
    );
}
```

### 4. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, increment the constant by 1 relative to whatever value is current when this ticket lands (after tickets 002 and 005 may have also bumped).

## Files to Touch

- `crates/worldwake-core/src/expectation.rs` (modify — variant + test)
- `crates/worldwake-ai/src/ranking.rs` (modify — cascade arm at line ~1133)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump)

## Out of Scope

- Writing `ExpectationRecord`s with the new basis (ticket 008).
- Reading / interpreting records with the new basis (ticket 009).
- Cross-agent variant consumers (none identified — confirmed in Assumption 3).

## Acceptance Criteria

### Tests That Must Pass

1. `expectation_basis_plan_step_completion_round_trips_through_bincode` (new) passes.
2. Existing `ExpectationBasis` tests in `expectation.rs` tests module stay green — all pre-S114 variants continue to round-trip unchanged.
3. `cargo check -p worldwake-ai` reports no `non_exhaustive_patterns` warning on the ranking.rs cascade after the arm is added.
4. Existing suite: `cargo test -p worldwake-core expectation` and `cargo test -p worldwake-ai ranking` stay green.

### Invariants

1. `ExpectationBasis` and `ExpectationRecord` retain their `Copy` derives.
2. The cascade arm in `ranking.rs` returns `0` for `PlanStepCompletion` — documenting the choice inline.
3. `SAVE_FORMAT_VERSION` increments by exactly 1 for this ticket's serialization change.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/expectation.rs::expectation_basis_plan_step_completion_round_trips_through_bincode` (new).

### Commands

1. `cargo test -p worldwake-core expectation`
2. `cargo test -p worldwake-ai ranking`
3. `cargo clippy -p worldwake-core -p worldwake-ai --all-targets -- -D warnings`
