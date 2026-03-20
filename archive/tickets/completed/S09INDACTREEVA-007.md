# S09INDACTREEVA-007: Collapse `ActionDuration` into a finite runtime duration newtype

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` runtime duration type shape, save/load schema version, downstream finite-duration call sites/tests
**Deps**: archive/tickets/completed/S09INDACTREEVA-006.md, specs/S09-indefinite-action-re-evaluation.md

## Problem

S09 already removed indefinite live durations from runtime behavior, planning, CLI display, and defend semantics. The remaining drift is narrower: [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs) still models runtime duration as `pub enum ActionDuration { Finite(u32) }` and still exposes `fixed_ticks() -> Option<u32>`.

That shape is now misleading. The runtime no longer has multiple duration kinds, and the optional accessor still advertises extinct branching at exactly the boundary that should now be most concrete: remaining ticks on an active action instance.

This is a type-surface cleanup, not a behavior fix. The architecture already behaves correctly post-S09, but the runtime representation is still weaker and more general than the current invariant.

## Assumption Reassessment (2026-03-20)

1. The major S09 architecture described in [specs/S09-indefinite-action-re-evaluation.md](/home/joeloverbeck/projects/worldwake/specs/S09-indefinite-action-re-evaluation.md) has already landed. Current code already uses `DurationExpr::ActorDefendStance` in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs), [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs), and [combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs). The ticket’s earlier scope claiming those changes were still pending was incorrect.
2. `ActionDuration` currently remains a single-variant enum in [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs): `pub enum ActionDuration { Finite(u32) }`, with `fixed_ticks(self) -> Option<u32>` and `advance(&mut self) -> bool`. This is the real remaining mismatch between runtime representation and runtime invariant.
3. The authoritative runtime meaning of `ActionDuration` is “resolved remaining duration for an active action instance,” stored in [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs) as `ActionInstance::remaining_duration`. It is no longer the semantic branching surface for fixed vs dynamic duration sources; that boundary is `DurationExpr` in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs).
4. Current production/runtime call sites are already finite-only. I checked:
   - active instance storage in [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs)
   - reservation handling in [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
   - countdown/commit in [tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs)
   - belief/runtime estimation in [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs)
   - CLI display in [tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/tick.rs) and [world_overview.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/world_overview.rs)
   - AI search cost extraction in [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs)
   No live `ActionDuration::Indefinite` or `DurationExpr::Indefinite` production references remain under `crates/`.
5. The current API still causes representation drift in secondary layers. In tests/helpers such as [affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs) and [trade_valuation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/trade_valuation.rs), `duration.fixed_ticks().map(ActionDuration::Finite)` recreates the now-obsolete “maybe a runtime duration exists” shape even though only finite runtime durations are legal.
6. `ActionDuration` is part of persisted state through [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs), [simulation_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/simulation_state.rs), and [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs). Collapsing the enum to a newtype will change the serialized payload layout. Per repo policy, the correct cleanup is to bump `SAVE_FORMAT_VERSION` rather than add compatibility shims.
7. Existing coverage already spans the correct layers for this narrower cleanup:
   - focused/unit: `action_duration::tests::finite_duration_exposes_ticks_and_counts_down_to_completion`, `action_duration::tests::action_duration_roundtrips_through_bincode`
   - focused persistence: `action_instance::tests::action_instance_roundtrips_with_some_local_state`, `action_instance::tests::action_instance_roundtrips_with_no_local_state`
   - focused runtime: `start_gate::tests::start_action_supports_dynamic_defend_duration_when_no_reservations_are_needed`, `start_gate::tests::start_action_supports_finite_duration_when_reservations_are_required`, `tick_action::tests::tick_action_decrements_finite_duration_and_reinserts_active_instance`, `tick_action::tests::tick_action_commits_when_finite_duration_reaches_zero`
   - save/load boundary: `save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state`, `save_load::tests::load_rejects_wrong_version`, `save_load::tests::loaded_state_continues_identically_to_uninterrupted_execution`
   - workspace command discovery: `cargo test -p worldwake-sim -- --list`, `cargo test -p worldwake-ai -- --list`
8. This is not an AI-regression ticket. Candidate generation, ranking, plan search, action start failure handling, and trace semantics should remain behaviorally unchanged. The only AI-facing code likely touched is finite-duration extraction in [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs), and that change should be representational only.
9. This ticket does not change action-lifecycle ordering, event-log ordering, or authoritative world-state ordering. The contract remains countdown storage and finite reservation-span derivation; those are already finite-only today.
10. Mismatch + correction: the earlier ticket overstated scope by re-owning already-completed S09 work across `CombatProfile`, `DurationExpr`, defend action semantics, and multiple CLI/runtime surfaces. The corrected scope is limited to collapsing the now-single-variant runtime duration enum into a concrete finite value type, updating persistence/versioning, and normalizing the remaining callers/tests to that honest representation.

## Architecture Check

1. A finite runtime duration newtype is cleaner than the current single-variant enum because it encodes the exact invariant directly: an active action instance has a concrete remaining tick count. That aligns better with Principle 3 in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) than preserving a pseudo-sum-type with extinct branching semantics.
2. The representation change is worth doing, but only at the runtime boundary. `DurationExpr` should continue to own semantic branching and dynamic resolution. That separation is already the clean architecture; this ticket should finish it instead of reopening broader S09 refactors.
3. `u32` remains the right storage shape because `0` is a lawful runtime state during countdown/commit boundaries and reservation handling. `NonZeroU32` would incorrectly erase a real state the runtime already uses.
4. No backward-compatibility aliasing, multi-version deserializers, or deprecated helper layer should be added. If the representation changes, the save format version should change with it and older saves should continue to fail through the existing unsupported-version path.

## Verification Layers

1. Runtime duration exposes a concrete finite-only API with no obsolete optional branch -> focused unit coverage in [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs)
2. Active action instances still serialize/deserialize correctly with the new representation -> focused persistence coverage in [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs)
3. Countdown and reservation behavior remain unchanged under the new type -> focused runtime coverage in [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs) and [tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs)
4. Save/load intentionally reflects the representation change and rejects old versions explicitly -> save/load boundary tests in [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs)
5. Additional trace-layer mapping is not applicable because this ticket does not change AI reasoning, request resolution, or action lifecycle semantics; focused runtime and persistence boundaries are the real contract

## What to Change

### 1. Replace the runtime single-variant enum with a finite value type

In [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs):

- replace `pub enum ActionDuration { Finite(u32) }` with a dedicated finite runtime duration value type
- preferred shape:
  - `pub struct ActionDuration(u32);`
  - constructor/accessor APIs such as `new`, `ticks`, or `remaining_ticks`
  - preserve `0` as a valid runtime value
- remove `fixed_ticks() -> Option<u32>` because it advertises extinct runtime branching
- keep countdown behavior explicit and deterministic

### 2. Normalize finite-duration consumers to the concrete API

Update remaining finite-only consumers to stop reconstructing enum semantics, especially:

- [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs)
- [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
- [tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs)
- [affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs)
- [trade_valuation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/trade_valuation.rs)
- [tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/tick.rs)
- [world_overview.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/world_overview.rs)
- [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs)

Callers should use direct finite accessors/constructors rather than `ActionDuration::Finite(...)` pattern syntax or `fixed_ticks().map(...)`.

### 3. Make the serialization boundary explicit

In [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs):

- bump `SAVE_FORMAT_VERSION`
- update save/load tests to reflect the intentional wire-format change
- keep the existing unsupported-version rejection path
- do not add compatibility loaders or alternate schema branches

### 4. Strengthen focused coverage around the value-type contract

- update `action_duration` tests to assert the new finite-only constructor/accessor surface
- keep `action_instance` serde coverage
- update save/load tests so the version bump is deliberate and verified
- update focused runtime tests in `start_gate` / `tick_action` only where API usage changes

## Files to Touch

- `crates/worldwake-sim/src/action_duration.rs` (modify)
- `crates/worldwake-sim/src/action_instance.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-sim/src/tick_action.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify)
- `crates/worldwake-sim/src/affordance_query.rs` (modify)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify if direct finite accessor cleanup remains necessary)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify)
- focused tests that construct or assert runtime durations in `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` (modify as needed)

## Out of Scope

- Reopening S09 defend-duration architecture that is already complete
- Changing `DurationExpr`, belief estimation semantics, or defend behavior
- Planner-cost, candidate-generation, or plan-failure behavior changes
- Save-file migration shims, compatibility loaders, or alias representations for the old enum shape

## Acceptance Criteria

### Tests That Must Pass

1. `action_duration` focused tests prove the runtime type is finite-only and countdown behavior is unchanged
2. `action_instance` and `save_load` persistence tests prove the new representation and save-version boundary are correct
3. Existing suite: `cargo test -p worldwake-sim`
4. Existing suite: `cargo test --workspace`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. `ActionDuration` no longer exposes enum-shaped or optional APIs that imply non-finite runtime cases
2. `ActionDuration` still represents `0` remaining ticks as a lawful runtime state
3. No backward-compatibility save shim is added; the wire-format change is made explicit through `SAVE_FORMAT_VERSION`
4. Runtime countdown, reservation-span derivation, planner duration extraction, and CLI display remain behaviorally unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_duration.rs` — replace enum-shaped API assertions with direct finite value-type checks; rationale: this file owns the invariant under change.
2. `crates/worldwake-sim/src/action_instance.rs` — keep serde/bincode roundtrip coverage with the new runtime duration representation; rationale: this is the smallest persisted boundary for active action duration.
3. `crates/worldwake-sim/src/save_load.rs` — update full-state roundtrip and version-rejection assertions; rationale: this ticket intentionally changes the serialized payload contract.
4. `crates/worldwake-sim/src/start_gate.rs` — adjust reservation/start assertions only where the runtime API changes; rationale: this confirms reservation span handling remains stable.
5. `crates/worldwake-sim/src/tick_action.rs` — adjust countdown/commit assertions only where the runtime API changes; rationale: this confirms active-action countdown semantics remain stable.

### Commands

1. `cargo test -p worldwake-sim action_duration::tests::`
2. `cargo test -p worldwake-sim action_instance::tests::action_instance_roundtrips_with_some_local_state`
3. `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state`
4. `cargo test -p worldwake-sim`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-20
- What actually changed:
  - Reassessed the ticket and corrected its assumptions to match current code: `DurationExpr::ActorDefendStance`, defend finite-duration behavior, and related S09 work were already complete.
  - Collapsed [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs) from a single-variant enum into a finite runtime newtype with direct `new()` / `ticks()` APIs.
  - Updated runtime, planner, CLI, and test call sites to use the concrete finite API instead of enum matching or `fixed_ticks().map(...)` reconstruction.
  - Bumped `SAVE_FORMAT_VERSION` in [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) to make the wire-format change explicit without compatibility shims.
- Deviations from original plan:
  - The original ticket claimed ownership of broader S09 defend-duration and `DurationExpr` work that had already landed. That scope was removed rather than reimplemented.
  - No behavioral defend/planner/runtime changes were necessary; the delivered work was a type-surface and persistence-boundary cleanup only.
- Verification results:
  - `cargo test -p worldwake-sim action_duration::tests::`
  - `cargo test -p worldwake-sim action_instance::tests::action_instance_roundtrips_with_some_local_state`
  - `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state`
  - `cargo test -p worldwake-ai search::tests::build_successor_estimates_defend_ticks_from_combat_profile`
  - `cargo test -p worldwake-sim`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
