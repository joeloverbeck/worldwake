# S134CANEFFSCH-004: Needs schemas

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals in 6 needs actions and switches their commit handler bodies to `apply_effects_with_context(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 (Action handler migration) requires migrating the 6 needs-domain actions registered in `crates/worldwake-systems/src/needs_actions.rs` (eat, drink, sleep, toilet, relieve_wilderness, wash) — populating their `EffectSchema` fields and switching commit handlers to `apply_effects_with_context(..., Authoritative)`. The needs file carries a per-action `BindingStrictness` override in `register_def`; this ticket preserves that override on `ActionDef.binding_strictness` (the targeting layer is independent of the new effect-schema layer per spec Non-Goals). The planner continues to use the old `apply_hypothetical_transition` path; goldens for needs actions must preserve existing event-log behavior.

## Assumption Reassessment (2026-05-04)

1. Needs registrations live at `crates/worldwake-systems/src/needs_actions.rs` via the composite `register_needs_actions` function (line ~13–20) which registers 6 handlers internally for actions: eat, drink, sleep, toilet, relieve_wilderness, wash. Per-action `BindingStrictness` override at lines 183–188 (match by action name).
2. After ticket 001, each `ActionDef` literal in `needs_actions.rs` has `effect_schema: EffectSchema::empty()`. This ticket replaces each empty schema with a real one. The `BindingStrictness` override is preserved unchanged — it lives on `ActionDef.binding_strictness`, separate from `effect_schema`.
3. Shared abstraction boundary under audit: each needs action's authoritative commit must mutate the same `HomeostaticNeedId` component (`Hunger`, `Thirst`, `Fatigue`, `Bladder`, `Dirtiness`) deltas as today, with the same event-log emissions. The bitwise-identical event-log invariant is the contract.
4. Existing focused/unit coverage: `needs_actions.rs` `#[cfg(test)]` block + needs-touching goldens including `golden_survival_baseline.rs`, `golden_survival_scattered.rs`, `golden_survival_contested.rs`. Conformance tests `conformance_eat_smoke_test`, `conformance_drink`, `conformance_sleep`, `conformance_relieve`, `conformance_wash` (currently dual-impl) will be replaced in ticket 010 — they continue to exercise the old planner hypothetical path until then.
5. Event-log preservation invariant: schema-driven needs commits must emit the same category-specific events and payload field values as before (`SleepEpisodeEnded`, `WasteCreated`, `WashFacilityUsed` where applicable), with the same `Hunger`/`Thirst`/etc. component deltas and canonical state behavior under the survival goldens.
6. `BindingStrictness` per-action override at lines 183–188 is a targeting concern (S108), not an effect concern — preserved verbatim.

## Architecture Check

1. Per-action declarative schemas replace per-action imperative handlers, eliminating drift between authoritative commit and the planner's hypothetical projection (the unification target). Both modes share the schema once ticket 010 lands.
2. The S108 `BindingStrictness` override and the new `EffectSchema` are layered concerns: targeting (which entity satisfies the slot) vs. evaluation (what the action does once bound). Keeping them separate matches the spec's Non-Goal statement and preserves S108's design.
3. Needs restoration is represented by branch-specific `EffectStep` variants interpreted by the needs-owned authoritative sink. This preserves concrete consumable profiles, sleep lifecycle state, latrine/wilderness aftermath, and wash-basin payloads without inventing a parallel generic need-delta path.

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on `golden_survival_*` and any needs-specific goldens.
2. Per-need delta invariant → action trace: each `commit_eat`/`commit_drink`/etc. produces the same component-mutation sequence (e.g., `Hunger` delta, food-source `Quantity` decrement, container content update if relevant).
3. `BindingStrictness` per-action override invariant → focused unit/runtime test: the existing `needs_actions.rs:183–188` match still resolves to the same `BindingStrictness` variant per action name post-ticket; targeting behavior is unchanged.
4. Canonical state hash invariant → soak: identical `blake3` hashes on the three soak scenarios pre- and post-ticket.

## What to Change

### 1. Construct `EffectSchema` literals for 6 needs actions

For each of eat, drink, sleep, toilet, relieve_wilderness, and wash, build the schema from the current handler logic. Landed shape:

- **eat / drink**: `ConsumeTargetConsumable { target: Target(0), effect: Hunger|Thirst }`. The needs-owned authoritative sink consumes one unit of the bound lot and applies the same consumable-profile deltas as the old handler.
- **sleep**: `EndSleepEpisode`. Fatigue recovery remains in `tick_sleep`; commit only ends the episode and emits the existing `SleepEpisodeEnded` payload.
- **toilet**: `UseToilet`. The needs-owned sink creates waste, updates latrine/place dirtiness, resets bladder, and emits `WasteCreated` only on the same overcapacity branch as before.
- **relieve_wilderness**: `RelieveWilderness`. The needs-owned sink creates waste/evidence, resets bladder, applies wilderness dirtiness penalty, updates place dirtiness, and emits the existing wilderness `WasteCreated` payload.
- **wash**: `CoLocated { Actor, Target(0) }`; `UseWashBasin { basin: Target(0) }`. The needs-owned sink consumes basin water, applies proportional dirtiness deltas, and emits `WashFacilityUsed`.

No generic `EffectStep::SatisfyNeed` was added. The live needs semantics are branch-specific because sleep recovery occurs during tick, consumable relief comes from the target lot profile, and toilet/wilderness/wash carry domain payloads and aftermath.

### 2. Replace needs commit handler bodies with `apply_effects` delegation

Each `commit_*` handler in `needs_actions.rs` now delegates through `apply_needs_effect_schema(...)`, which calls `apply_effects_with_context(..., EffectMode::Authoritative)`.

### 3. Preserve `BindingStrictness` override at lines 183–188

The match assigning per-action `BindingStrictness` is preserved verbatim — it lives on `ActionDef.binding_strictness`, not in the new schema.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modified — 6 `EffectSchema` literals, 6 commit handler body replacements, needs-owned authoritative sink, preserved `BindingStrictness` override)
- `crates/worldwake-sim/src/effect_schema.rs` (modified — added needs-specific `EffectStep` variants and default rejecting `EffectSink` methods)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` (no change — the authoritative interpretation is needs-owned inside `needs_actions.rs`, matching the combat-owned sink pattern from ticket 003)
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (no change — ticket 010 still owns hypothetical-mode activation; default rejecting sink methods keep the staged state explicit)

## Out of Scope

- Migrating non-needs actions (per-category tickets 003, 005–009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Deleting `apply_hypothetical_transition`, `PlannerTransitionKind`, `apply_planner_step` (ticket 010).
- Changing `BindingStrictness` semantics or the per-action override at lines 183–188 (preserved verbatim).
- Conformance test rewrite (ticket 010).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_baseline.rs`, `golden_survival_scattered.rs`, `golden_survival_contested.rs` produce bitwise-identical event logs to pre-ticket baseline.
2. All needs-touching goldens (enumerate during reassessment) pass without source change.
3. `cargo test -p worldwake-systems needs` — existing inline tests pass with the schema-driven path.
4. Conformance tests `conformance_eat_smoke_test`, `conformance_drink`, `conformance_sleep`, `conformance_relieve`, `conformance_wash` continue to pass — they still compare imperative vs. `apply_hypothetical_transition` (which is unchanged in this ticket).
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Each of the 6 needs actions has a non-empty `EffectSchema` post-ticket.
2. `BindingStrictness` override at `needs_actions.rs:183–188` is preserved verbatim — same match arms, same variant assignments.
3. Bitwise-identical canonical state hash on the three soak scenarios pre- and post-ticket.
4. `HomeostaticNeedId` deltas applied by schema-driven path match imperative-handler deltas exactly (verify via action trace).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` `#[cfg(test)]` block — modified the existing registration test to assert non-empty needs schemas and preserved `BindingStrictness`; existing runtime tests continue to exercise needs commit outcomes and existing precondition/race-condition surfaces.
2. Existing needs goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems needs`
2. `cargo test -p worldwake-ai golden_survival`
3. `cargo test -p worldwake-ai --test golden_survival_baseline`
4. `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
5. `cargo test -p worldwake-ai --test golden_survival_scattered`
6. `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
7. `cargo test -p worldwake-ai --test golden_survival_contested`
8. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
9. `cargo test -p worldwake-ai --test planner_conformance conformance_eat_smoke_test`
10. `cargo test -p worldwake-ai --test planner_conformance conformance_drink`
11. `cargo test -p worldwake-ai --test planner_conformance conformance_sleep`
12. `cargo test -p worldwake-ai --test planner_conformance conformance_relieve`
13. `cargo test -p worldwake-ai --test planner_conformance conformance_wash`
14. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-04.

- Added needs-specific schema steps to `EffectStep`: `ConsumeTargetConsumable`, `EndSleepEpisode`, `UseToilet`, `RelieveWilderness`, and `UseWashBasin`.
- Populated non-empty effect schemas for all 6 needs actions.
- Replaced the 6 needs commit bodies with `apply_needs_effect_schema(...)`, delegating to `apply_effects_with_context(..., EffectMode::Authoritative)`.
- Added a needs-owned authoritative effect sink in `needs_actions.rs` that preserves the previous branch-specific component mutations and event payloads.
- Extended the existing needs registration test to assert non-empty schemas and preserved `BindingStrictness` assignments.
- Left planner hypothetical evaluation on the old path until `S134CANEFFSCH-010`.

## Deviations

- The draft counted 5 needs actions by grouping `toilet/relieve_wilderness`; the live registry has 6 action definitions and all 6 now carry non-empty schemas.
- No `EffectStep::SatisfyNeed` was added. Needs commits require branch-specific authoritative operations rather than one generic need-delta step.
- The drafted `cargo test -p worldwake-ai conformance_eat conformance_drink ...` command is not valid Cargo syntax. Verification used the live `planner_conformance` integration test and ran each selector separately.
- The shorthand `cargo test -p worldwake-ai golden_survival` only matched one live non-ignored test, so the three ticket-named survival binaries were run directly, including their ignored long-run scenario tests.
- No separate new precondition-failure test was added. Existing needs runtime tests already cover the local precondition/race-condition surfaces, and this ticket's new focused assertion is the schema-registration and `BindingStrictness` invariant.

## Verification Result

Passed:

1. `cargo test -p worldwake-systems --lib needs_actions::tests::register_needs_actions_adds_all_six_defs_and_handlers -- --exact`
2. `cargo test -p worldwake-systems --lib needs_actions`
3. `cargo test -p worldwake-systems needs`
4. `cargo test -p worldwake-ai --test planner_conformance conformance_eat_smoke_test`
5. `cargo test -p worldwake-ai --test planner_conformance conformance_drink`
6. `cargo test -p worldwake-ai --test planner_conformance conformance_sleep`
7. `cargo test -p worldwake-ai --test planner_conformance conformance_relieve`
8. `cargo test -p worldwake-ai --test planner_conformance conformance_wash`
9. `cargo test -p worldwake-ai golden_survival` (one matching live test)
10. `cargo test -p worldwake-ai --test golden_survival_baseline`
11. `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
12. `cargo test -p worldwake-ai --test golden_survival_scattered`
13. `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
14. `cargo test -p worldwake-ai --test golden_survival_contested`
15. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
16. `cargo fmt --all`
17. `cargo clippy --workspace --all-targets -- -D warnings`
18. `./scripts/verify.sh` (live gates: `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p worldwake-cli --bin scenario-coverage -- --check`)
