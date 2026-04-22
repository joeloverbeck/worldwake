# S114PLASTGUA-004: ExpectationBasis::PlanStepCompletion variant + AI expectation cascades

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new `ExpectationBasis` variant; exhaustive-match arms in `ranking.rs` and `candidate_generation.rs`; explicit AI-side exclusion from missing-response candidate emission; `SAVE_FORMAT_VERSION` bump.
**Deps**: `archive/tickets/S114PLASTGUA-001.md`

## Problem

S114 D4 requires persisting plan-step expectations through the existing `ExpectationStore` / `ExpectationRecord` infrastructure. The new `PlanStepCompletion { step_index, kind_tag }` variant is the hook; it must remain `Copy`-safe so `ExpectationBasis` and `ExpectationRecord` retain their current `Copy` derives. Landing the variant first unblocks tickets 008 (plan-adoption writes) and 009 (AI-side interpretation of `Overdue` records) without tangling them with cascade bookkeeping.

## Assumption Reassessment (2026-04-21)

1. `ExpectationBasis` at `crates/worldwake-core/src/expectation.rs:17` has five variants today (`DutyAssignment`, `DeliveryCommitment`, `RoutineReturn`, `EscortObligation`, `SocialPromise`) and derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `ExpectationRecord` at `expectation.rs:52` also derives `Copy`. The new variant payload `{ step_index: u16, kind_tag: ExpectationKindTag }` is two `Copy` primitives, preserving both derives.
2. S114 spec D4 at `specs/S114-plan-step-guards.md:239-266` defines the variant shape and the `0`-weight ranking intent, but the live exhaustive-ish consumers are broader than the draft ticket stated: `ExpectationBasis` is matched at `crates/worldwake-ai/src/ranking.rs:1131-1135` and at `crates/worldwake-ai/src/candidate_generation.rs:4048-4054`.
3. `candidate_generation.rs` is not only a compile-fix site. `emit_search_candidates` at `candidate_generation.rs:3871-3945` currently turns any overdue expectation into `GoalKind::SearchForMissing` / `GoalKind::ReportMissing`, and its record ordering uses `expectation_basis_weight(record)`. `PlanStepCompletion` overdue records are agent-internal plan-monitoring state, not social missing-person obligations, so this ticket must exclude that basis from the missing-response emitter rather than merely assign it weight `0`.
4. Shared boundary under audit: the `ExpectationBasis` enum layout plus its two current AI consumers. Other sites (`per_agent_belief_view.rs`, `save_load.rs`, `expectation_check.rs` tests, golden tests, `search_actions.rs`, `report_actions.rs`, `ask_about_person_actions.rs`) currently construct specific variants and do not exhaustive-match. Sim-side `check_overdue_expectations` (`crates/worldwake-systems/src/expectation_check.rs:7`) still operates on `ExpectationState` transitions only, so no system-side branch is required here.
5. `SAVE_FORMAT_VERSION` is already `37` at `crates/worldwake-sim/src/save_load.rs:5` on the live branch, not `36`. Adding an enum variant changes the serialized variant-tag layout, so this ticket must bump to `38` (current + 1).

## Architecture Check

1. Additive variant only — no existing variants renamed, no construction sites migrated, no `handle_plan_failure` branching added. Tickets 008 and 009 do all the consumer work.
2. The ranking / candidate-ordering cascade returns `0` for `PlanStepCompletion` because plan-step expectations are agent-internal plan state; they do not rank against duty/delivery/escort obligations which exist at the institutional-social layer.
3. The missing-response emitter must not treat `PlanStepCompletion` as a social missing-person contract. Overdue plan-step expectations are for ticket 009's AI-side plan discrepancy handling, not `SearchForMissing` / `ReportMissing`.

## Verification Layers

1. Exhaustive-match coverage (`ranking.rs` and `candidate_generation.rs` add arms) → `cargo check -p worldwake-ai` compiles with no `non_exhaustive_patterns` warning.
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

### 2. Add the AI cascade arms and social-path exclusion

In `crates/worldwake-ai/src/ranking.rs:1131-1135`, extend the exhaustive `match basis { ... }` with:

```rust
ExpectationBasis::PlanStepCompletion { .. } => 0,
```

Add a brief inline comment: `// plan-step expectations are agent-internal; no social-obligation weight`.

In `crates/worldwake-ai/src/candidate_generation.rs`, update the local `expectation_basis_weight(record)` helper with the same `PlanStepCompletion => 0` arm.

Also exclude `ExpectationBasis::PlanStepCompletion { .. }` from `emit_search_candidates` before records are considered for `SearchForMissing` / `ReportMissing`, with a short comment explaining that this basis is for plan-monitoring follow-up rather than the social missing-response path.

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
- `crates/worldwake-ai/src/ranking.rs` (modify — cascade arm at line ~1131)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — cascade arm + missing-response exclusion + test)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump)

## Out of Scope

- Writing `ExpectationRecord`s with the new basis (ticket 008).
- Reading / interpreting records with the new basis (ticket 009).
- Cross-agent variant consumers (none identified — confirmed in Assumption 3).

## Acceptance Criteria

### Tests That Must Pass

1. `expectation_basis_plan_step_completion_round_trips_through_bincode` (new) passes.
2. Existing `ExpectationBasis` tests in `expectation.rs` tests module stay green — all pre-S114 variants continue to round-trip unchanged.
3. `cargo check -p worldwake-ai` reports no `non_exhaustive_patterns` warning on the `ranking.rs` or `candidate_generation.rs` cascades after the arms are added.
4. Overdue `ExpectationBasis::PlanStepCompletion` records do not emit `SearchForMissing` or `ReportMissing` candidates from `candidate_generation.rs`.
5. Existing suite: `cargo test -p worldwake-core expectation`, `cargo test -p worldwake-ai ranking`, and focused candidate-generation coverage stay green.

### Invariants

1. `ExpectationBasis` and `ExpectationRecord` retain their `Copy` derives.
2. The cascade arm in `ranking.rs` returns `0` for `PlanStepCompletion` — documenting the choice inline.
3. `SAVE_FORMAT_VERSION` increments by exactly 1 for this ticket's serialization change.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/expectation.rs::expectation_basis_plan_step_completion_round_trips_through_bincode` (new).
2. `crates/worldwake-ai/src/candidate_generation.rs::plan_step_completion_expectations_do_not_emit_missing_response_goals` (new).

### Commands

1. `cargo test -p worldwake-core expectation`
2. `cargo test -p worldwake-ai ranking`
3. `cargo test -p worldwake-ai candidate_generation`
4. `cargo clippy -p worldwake-core -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-22.

- Added `ExpectationBasis::PlanStepCompletion { step_index, kind_tag }` in `crates/worldwake-core/src/expectation.rs` while preserving the existing `Copy`-safe derive surface.
- Added the `PlanStepCompletion => 0` branch in both AI expectation-weight cascades: `crates/worldwake-ai/src/ranking.rs` and `crates/worldwake-ai/src/candidate_generation.rs`.
- Excluded overdue `PlanStepCompletion` records from `emit_search_candidates` so plan-step monitoring state does not leak into the social `SearchForMissing` / `ReportMissing` goal path.
- Added focused coverage for bincode round-trip of the new basis and for the candidate-generation exclusion, and bumped `SAVE_FORMAT_VERSION` from `37` to `38`.

## Deviations

- Reassessment disproved the draft's "ranking-only cascade" scope. The live branch also matched `ExpectationBasis` in `candidate_generation.rs`, and that site needed both the new `0`-weight arm and an explicit semantic exclusion from missing-response candidate emission.
- Reassessment also disproved the drafted save-version claim. The live branch was already at `SAVE_FORMAT_VERSION = 37`, so the honest bump for this ticket was `38`, not `37` or a deferred `current + 1` placeholder.

## Verification Result

- Passed `cargo test -p worldwake-core expectation`
- Passed `cargo test -p worldwake-ai ranking`
- Passed `cargo test -p worldwake-ai candidate_generation`
- Passed `cargo clippy -p worldwake-core -p worldwake-ai --all-targets -- -D warnings`
