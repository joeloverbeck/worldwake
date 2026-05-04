# S134CANEFFSCH-004: Needs schemas

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals in 5 needs actions and switches their commit handler bodies to `apply_effects(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 (Action handler migration) requires migrating the 5 needs-domain actions registered in `crates/worldwake-systems/src/needs_actions.rs` (eat, drink, sleep, toilet/relieve_wilderness, wash) — populating their `EffectSchema` fields and switching commit handlers to `apply_effects(..., Authoritative)`. The needs file carries a per-action `BindingStrictness` override at lines 183–188 (a match assigning different `BindingStrictness` variants per action name); this ticket preserves that override on `ActionDef.binding_strictness` (the targeting layer is independent of the new effect-schema layer per spec Non-Goals). The planner continues to use the old `apply_hypothetical_transition` path; goldens for needs actions must produce bitwise-identical event logs.

## Assumption Reassessment (2026-05-04)

1. Needs registrations live at `crates/worldwake-systems/src/needs_actions.rs` via the composite `register_needs_actions` function (line ~13–20) which registers 6 handlers internally for actions: eat, drink, sleep, toilet, relieve_wilderness, wash. Per-action `BindingStrictness` override at lines 183–188 (match by action name).
2. After ticket 001, each `ActionDef` literal in `needs_actions.rs` has `effect_schema: EffectSchema::empty()`. This ticket replaces each empty schema with a real one. The `BindingStrictness` override is preserved unchanged — it lives on `ActionDef.binding_strictness`, separate from `effect_schema`.
3. Shared abstraction boundary under audit: each needs action's authoritative commit must mutate the same `HomeostaticNeedId` component (`Hunger`, `Thirst`, `Fatigue`, `Bladder`, `Dirtiness`) deltas as today, with the same event-log emissions. The bitwise-identical event-log invariant is the contract.
4. Existing focused/unit coverage: `needs_actions.rs` `#[cfg(test)]` block + needs-touching goldens including `golden_survival_baseline.rs`, `golden_survival_scattered.rs`, `golden_survival_contested.rs`, `golden_eat_*.rs`, `golden_sleep_*.rs`, `golden_wash_*.rs`. Conformance tests `conformance_eat_smoke_test`, `conformance_drink`, `conformance_sleep`, `conformance_relieve`, `conformance_wash` (currently dual-impl) will be replaced in ticket 010 — they continue to exercise the imperative path until then.
5. Bitwise-identical event-log invariant: schema-driven needs commits must emit the same `EventTag::NeedSatisfied` (or category-specific) events with identical payload field values, the same `Hunger`/`Thirst`/etc. component deltas, and the same canonical state hash post-replay.
6. `BindingStrictness` per-action override at lines 183–188 is a targeting concern (S108), not an effect concern — preserved verbatim.

## Architecture Check

1. Per-action declarative schemas replace per-action imperative handlers, eliminating drift between authoritative commit and the planner's hypothetical projection (the unification target). Both modes share the schema once ticket 010 lands.
2. The S108 `BindingStrictness` override and the new `EffectSchema` are layered concerns: targeting (which entity satisfies the slot) vs. evaluation (what the action does once bound). Keeping them separate matches the spec's Non-Goal statement and preserves S108's design.
3. `EffectPrecondition::QuantityAvailable` and `EffectStep::Consume` provide the language for needs-restoration semantics (consume one unit of food, deplete the food source) using existing core types — no new commodity-key abstraction required.

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on `golden_survival_*` and any needs-specific goldens.
2. Per-need delta invariant → action trace: each `commit_eat`/`commit_drink`/etc. produces the same component-mutation sequence (e.g., `Hunger` delta, food-source `Quantity` decrement, container content update if relevant).
3. `BindingStrictness` per-action override invariant → focused unit/runtime test: the existing `needs_actions.rs:183–188` match still resolves to the same `BindingStrictness` variant per action name post-ticket; targeting behavior is unchanged.
4. Canonical state hash invariant → soak: identical `blake3` hashes on the three soak scenarios pre- and post-ticket.

## What to Change

### 1. Construct `EffectSchema` literals for 5 needs actions

For each of eat, drink, sleep, toilet/relieve_wilderness, wash, build the schema from the current handler logic. Sketch:

- **eat / drink**: preconditions — `CoLocated { actor, target: food_source_or_container }`, `QuantityAvailable { source, commodity: CommodityKind::Food (or Water), min: 1 }`. Steps — `Consume { source, commodity, quantity: 1 }`, restore the corresponding `HomeostaticNeedId` delta (a need-restoration `EffectStep` variant — likely add `EffectStep::SatisfyNeed { agent: EntityId, need: HomeostaticNeedId, delta: NeedSeverity }` if not already present from ticket 001), `EmitEvent { tag: EventTag::NeedSatisfied }`.
- **sleep**: preconditions — `CoLocated { actor, target: sleep_site }`, sleep-quality precondition. Steps — `SatisfyNeed { agent, need: HomeostaticNeedId::Fatigue, delta }`, `EmitEvent { tag: EventTag::Sleep }`.
- **toilet / relieve_wilderness**: preconditions — `CoLocated` with appropriate facility or place tag. Steps — `SatisfyNeed { need: HomeostaticNeedId::Bladder, delta }`, `EmitEvent`.
- **wash**: preconditions — `CoLocated` with `Well` workstation or water source. Steps — `SatisfyNeed { need: HomeostaticNeedId::Dirtiness, delta }`, `EmitEvent`.

`EffectStep::SatisfyNeed` (or equivalent) is likely the natural new variant — confirm against ticket 001's enum during reassessment; if absent, add it here and implement the sink method in both impls.

### 2. Replace needs commit handler bodies with `apply_effects` delegation

Each `commit_*` handler in `needs_actions.rs` shrinks to the standard delegation pattern (see ticket 003 for the canonical shape).

### 3. Preserve `BindingStrictness` override at lines 183–188

The match assigning per-action `BindingStrictness` is preserved verbatim — it lives on `ActionDef.binding_strictness`, not in the new schema.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — 5 `EffectSchema` literals, 5 commit handler body replacements; preserve `BindingStrictness` override at lines 183–188)
- `crates/worldwake-sim/src/effect_schema.rs` (modify if needs surface new `EffectStep` variants such as `SatisfyNeed`)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` (modify if new sink methods are added)
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify if new sink methods are added)

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

1. Each of the 5 needs actions has a non-empty `EffectSchema` post-ticket.
2. `BindingStrictness` override at `needs_actions.rs:183–188` is preserved verbatim — same match arms, same variant assignments.
3. Bitwise-identical canonical state hash on the three soak scenarios pre- and post-ticket.
4. `HomeostaticNeedId` deltas applied by schema-driven path match imperative-handler deltas exactly (verify via action trace).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` `#[cfg(test)]` block — modify existing tests to exercise schema-driven path; add focused tests covering precondition-failure cases (e.g., eat with empty food source yields `Discrepancy::NoLegalBinding` or `Discrepancy::SourceInvalidated`).
2. Existing needs goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems needs`
2. `cargo test -p worldwake-ai golden_survival`
3. `cargo test -p worldwake-ai conformance_eat conformance_drink conformance_sleep conformance_relieve conformance_wash`
4. `./scripts/verify.sh`
