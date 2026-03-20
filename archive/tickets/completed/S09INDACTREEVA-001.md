# S09INDACTREEVA-001: Add `defend_stance_ticks` field to `CombatProfile`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `CombatProfile` struct gains 11th field
**Deps**: None

## Problem

The defend action currently uses `DurationExpr::Indefinite`, which has no finite endpoint. To replace it with a profile-driven duration (`DurationExpr::ActorDefendStance`), the `CombatProfile` struct must first carry the `defend_stance_ticks` parameter. This is the prerequisite for all subsequent S09 tickets.

## Assumption Reassessment (2026-03-20)

1. `CombatProfile` currently has exactly 10 fields in `crates/worldwake-core/src/combat.rs`, and `CombatProfile::new()` currently takes 10 positional parameters. Focused coverage already exists in `combat::tests::combat_profile_new_stores_every_field` and `combat::tests::combat_profile_roundtrips_through_bincode`.
2. The broader design in `specs/S09-indefinite-action-re-evaluation.md` is still valid, but this ticket is only the schema prerequisite slice. Current production/runtime code still carries `Indefinite` handling outside the defend definition itself in `crates/worldwake-sim/src/action_semantics.rs`, `crates/worldwake-sim/src/belief_view.rs`, `crates/worldwake-sim/src/start_gate.rs`, `crates/worldwake-ai/src/search.rs`, and CLI display helpers. That broader cleanup remains deferred to later S09 tickets.
3. This is not an AI behavior regression ticket. It does touch AI crate fixtures because `CombatProfile::new()` is used in `crates/worldwake-ai/src/goal_model.rs`, `crates/worldwake-ai/src/plan_revalidation.rs`, `crates/worldwake-ai/src/search.rs`, and golden tests, but there is no intended candidate-generation, planning, or runtime behavior change here.
4. No ordering contract is involved. This ticket changes a shared data contract only; it does not assert action lifecycle ordering, event-log ordering, or authoritative world-state ordering.
5. No heuristic, filter, or planner special-case is removed here. `Indefinite` duration semantics and defend behavior remain unchanged until later S09 tickets.
6. This is not a stale-request, contested-affordance, or start-failure ticket. Shared runtime surfaces such as `tick_step` affordance reproduction and `start_action` reservation handling were checked only to confirm they remain out of scope for this slice.
7. This is not a political ticket.
8. No `ControlSource`, queued input, driver reset, or runtime intent retention behavior is touched.
9. No golden scenario isolation change is required. Existing golden tests are only constructor-fixture updates in this ticket.
10. Mismatch corrected: the previous ticket text claimed 39 `CombatProfile::new()` call sites across ~18 files. Current code has 37 call sites across 19 files (`rg -n "CombatProfile::new\\(" crates -g '!target'`). Scope and file list below are updated to match the actual tree.

## Architecture Check

1. Adding `defend_stance_ticks` to `CombatProfile` is the correct prerequisite because defend duration is an actor property, not a global constant or action-local magic number. This matches the existing `unarmed_attack_ticks` pattern and keeps agent diversity profile-driven.
2. Keeping this ticket narrow is cleaner than bundling it with the `Indefinite` removal. It isolates the shared schema expansion from later behavior changes across `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, and CLI layers, which reduces mixed-layer risk and keeps verification precise.
3. No backwards-compatibility shim constructor or alias path should be introduced. All `CombatProfile::new()` call sites should move atomically to the 11-argument contract.
4. Architectural note: the current many-argument `CombatProfile::new()` constructor is mechanically brittle. A future cleanup toward named builders or fixtures could reduce cross-crate churn when profiles grow, but that refactor is intentionally out of scope for this prerequisite ticket.

## Verification Layers

1. `CombatProfile::new()` accepts and stores `defend_stance_ticks` -> focused unit coverage in `crates/worldwake-core/src/combat.rs`
2. `CombatProfile` serialization and local helper fixtures remain valid with the new field -> focused unit coverage in `crates/worldwake-core/src/combat.rs`
3. Cross-crate constructor contract stays coherent after the field addition -> targeted package tests for `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai`
4. No decision trace, action trace, or event-log mapping is required yet because this ticket does not change candidate generation, action start, action ticking, or authoritative mutation semantics
5. Full-workspace regression remains required because this is a shared core type used across all runtime layers

## What to Change

### 1. Add `defend_stance_ticks` field to `CombatProfile`

In `crates/worldwake-core/src/combat.rs`:
- Add `pub defend_stance_ticks: NonZeroU32` as the 11th field on the struct (after `unarmed_attack_ticks`)
- Add the 11th parameter to `CombatProfile::new()` constructor
- Update `sample_combat_profile()` to include `defend_stance_ticks: nz(10)` (or use `NonZeroU32::new(10).unwrap()`)

### 2. Update all `CombatProfile::new()` call sites

Every call site must pass a `defend_stance_ticks` value as the 11th argument. Default: `nz(10)` unless a specific test needs a different value.

Call sites currently present in the repo (37 occurrences across 19 files):

| Crate | File | Count | Context |
|-------|------|-------|---------|
| core | `combat.rs` | 1 | `sample_combat_profile()` |
| core | `component_tables.rs` | 2 | Component-table tests |
| core | `world.rs` | 1 | Test helper |
| core | `delta.rs` | 1 | Delta roundtrip fixture |
| core | `wounds.rs` | 1 | Wound helper fixture |
| sim | `action_validation.rs` | 1 | Validation test fixture |
| sim | `action_semantics.rs` | 1 | Duration resolution test fixture |
| sim | `start_gate.rs` | 1 | Start gate test fixture |
| systems | `combat.rs` | 10 | Combat fixtures and defend-duration tests |
| systems | `office_actions.rs` | 2 | Office action tests |
| systems | `tests/e12_combat_integration.rs` | 2 | Integration tests |
| ai | `goal_model.rs` | 2 | Goal model tests |
| ai | `plan_revalidation.rs` | 1 | Plan revalidation test |
| ai | `search.rs` | 1 | Search test |
| ai | `tests/golden_harness/mod.rs` | 1 | Golden test harness default |
| ai | `tests/golden_combat.rs` | 4 | Combat golden tests |
| ai | `tests/golden_emergent.rs` | 3 | Emergent golden tests |
| ai | `tests/golden_production.rs` | 1 | Production golden tests |
| ai | `tests/golden_offices.rs` | 1 | Office golden tests |

## Files to Touch

- `crates/worldwake-core/src/combat.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/wounds.rs` (modify)
- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/action_validation.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-systems/src/combat.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `crates/worldwake-systems/tests/e12_combat_integration.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)

## Out of Scope

- Adding `DurationExpr::ActorDefendStance` (ticket 002)
- Removing `DurationExpr::Indefinite` or `ActionDuration::Indefinite` (ticket 004)
- Changing the defend action definition (ticket 003)
- Any behavioral changes to defend or the planner
- Modifying any `DurationExpr` or `ActionDuration` enums
- Golden tests for defend re-evaluation (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core` — all core tests compile and pass with 11-field `CombatProfile`
2. `cargo test -p worldwake-sim` — sim fixtures compile and pass with 11-field `CombatProfile`
3. `cargo test -p worldwake-systems` — system-layer fixtures compile and pass with 11-field `CombatProfile`
4. `cargo test -p worldwake-ai` — AI fixtures and golden harness compile and pass with 11-field `CombatProfile`
5. `cargo test --workspace` — all workspace tests compile and pass (no regressions from call site updates)
6. `cargo clippy --workspace` — no new warnings

### Invariants

1. `CombatProfile` has exactly 11 fields, all `pub`, with `defend_stance_ticks: NonZeroU32` as the last field
2. `CombatProfile::new()` is `const fn` and accepts 11 positional parameters
3. No behavioral change to any existing action, system, AI, or CLI logic — this ticket only expands the shared combat profile contract
4. Later S09 tickets remain responsible for introducing `DurationExpr::ActorDefendStance`, removing `Indefinite`, and changing defend behavior
5. Existing assertions should only need updates where focused unit tests explicitly check the `CombatProfile` field set or constructor signature

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/combat.rs` — extend the focused `CombatProfile` field-storage and bincode-roundtrip assertions to include `defend_stance_ticks`; this is the direct proof that the new authoritative field is preserved.
2. Existing fixtures across `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` will be modified mechanically to pass the new constructor argument; no behavior assertions should change in this ticket.
3. `crates/worldwake-cli/src/scenario/types.rs` — update the full RON deserialization fixture to include `defend_stance_ticks`; this keeps the scenario schema aligned with the authoritative `CombatProfile` contract instead of introducing a serde default shim.

### Commands

1. `cargo test -p worldwake-core combat::tests::combat_profile_new_stores_every_field`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-20
- Actual changes:
  - Added `defend_stance_ticks: NonZeroU32` to `CombatProfile` and extended `CombatProfile::new()` to 11 positional parameters.
  - Updated all 37 live `CombatProfile::new()` call sites across core, sim, systems, AI, golden harness/tests, and the CLI scenario test fixture.
  - Strengthened focused core coverage by asserting the new field in `combat_profile_new_stores_every_field`.
  - Corrected the ticket’s reassessment, call-site counts, verification scope, and architectural boundary before implementation.
- Deviations from original plan:
  - The original ticket understated the call-site/file reality and did not account for the CLI scenario deserialization fixture. Full-workspace verification exposed that serialized fixture coupling, so `crates/worldwake-cli/src/scenario/types.rs` was updated as part of the authoritative schema migration.
  - No behavior-layer S09 work was pulled forward. `DurationExpr::Indefinite`, `ActionDuration::Indefinite`, defend action semantics, and planner duration handling remain intentionally deferred to later S09 tickets.
- Verification results:
  - `cargo test -p worldwake-core combat::tests::combat_profile_new_stores_every_field`
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-cli`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
