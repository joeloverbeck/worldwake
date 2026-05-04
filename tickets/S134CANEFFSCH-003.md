# S134CANEFFSCH-003: Combat schemas

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals in 7 combat actions and switches their commit handler bodies to `apply_effects(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, S134CANEFFSCH-002

## Problem

S134 deliverable D5 (Action handler migration) requires every action's imperative handler body to be replaced by a constructed `EffectSchema` and a delegating call to `apply_effects(..., Authoritative)`. This ticket migrates the 7 combat-domain actions in `crates/worldwake-systems/src/combat.rs` (attack, defend, loot, bury, heal, queue_for_corpse_use, queue_for_care_target) — populating their schemas with real preconditions and effect steps, and shrinking their commit handler bodies to a delegation to `apply_effects`. The planner continues to use the old `apply_hypothetical_transition` path; no planner-side change in this ticket. Goldens for combat actions must produce bitwise-identical event logs before and after.

## Assumption Reassessment (2026-05-04)

1. Combat action registrations live at `crates/worldwake-systems/src/combat.rs` with 7 `register_*_action` functions: `register_attack_action`, `register_defend_action`, `register_loot_action`, `register_bury_action`, `register_heal_action`, `register_queue_for_corpse_use_action`, `register_queue_for_care_target_action`. Each currently has an imperative `commit_*` handler body that mutates ECS through the scheduler.
2. After ticket 001, each `ActionDef` literal in `combat.rs` has `effect_schema: EffectSchema::empty()`. This ticket replaces each empty schema with a real one and switches handler bodies to delegate.
3. Shared abstraction boundary under audit: each combat action's authoritative effect set must produce the same component mutations and event-log entries as today's imperative handler. The bitwise-identical event-log invariant is the contract.
4. Existing focused/unit coverage exercising combat handlers: tests in `combat.rs` `#[cfg(test)]` block (combat-domain handler tests around `start_attack`, `tick_attack`, `commit_attack`, `abort_attack` etc.) plus broader goldens — `crates/worldwake-ai/tests/golden_combat_*.rs`, `golden_combat_smoke.rs`, `golden_dragon_attack.rs`, `golden_loot_*.rs`, `golden_heal_*.rs`. Enumerate during reassessment via `rg -l "attack\|loot\|heal\|bury\|queue_for_(corpse|care)" crates/worldwake-ai/tests/golden_*.rs`.
5. Bitwise-identical event-log invariant: the schema-driven path must produce the same `EventTag` emissions, the same component mutations (wounds, body parts, contention-grant consumptions, container transfers for loot), and the same canonical state hash post-replay as the pre-ticket imperative path.
6. `WoundCause` taxonomy lives at `crates/worldwake-core/src/wounds.rs:44–50`; `WoundSeverity` does NOT exist (per spec D1 type-naming notes). The `EffectStep::ApplyWound` variant constructs wounds using existing wound shape primitives only.

## Architecture Check

1. Schema-driven evaluation through `apply_effects(..., Authoritative)` produces identical authoritative effects to the imperative handler because both paths write through the same scheduler/event-log write surface. The interpretation layer differs; the world meaning does not (FND-12 — performance compresses computation, never causality).
2. Per-action `EffectSchema` is the single declarative truth; the imperative body is removed, not preserved as a fallback (FND-28). No alias path remains.
3. `EffectPrecondition` failures classify into the existing `Discrepancy` taxonomy (S109) rather than introducing combat-specific failure types, keeping the seam with the existing planner fault-handling pipeline (FND-26).

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta against pre-ticket baseline on combat-touching goldens (e.g., `golden_combat_smoke`, `golden_dragon_attack`, `golden_loot_*`).
2. Per-action authoritative effects invariant → action trace: `commit_attack`, `commit_loot`, etc. produce the same `ActionTraceSink` events as today (wound application order, contention-grant consumption order).
3. Action-precondition failure invariant → focused runtime/unit test: the schema's `EffectPrecondition`s reject the same cases the imperative handler rejected (verify with adversarial inputs that previously triggered handler-internal validation failures).
4. Canonical state hash invariant → soak: 1440-tick replay of `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` produce identical `blake3` hashes pre- and post-ticket.

## What to Change

### 1. Construct `EffectSchema` literals for the 7 combat actions

In each `register_*_action` function, replace `effect_schema: EffectSchema::empty()` with a real `EffectSchema { preconditions: vec![…], steps: vec![…] }` literal. Preconditions encode current handler-internal validation (target alive, co-location, weapon equipped, contention grant held for queued corpse/care use, etc.); steps encode the authoritative effects (wound application, body-part damage, contention-grant consumption, item transfer for loot, corpse marking for bury, heal application).

Per-action sketch (final form determined during reassessment after reading current handler bodies):

- **attack**: preconditions — `CoLocated { actor, target }`, target-is-alive precondition. Steps — `ApplyWound { target, cause: WoundCause::Combat(weapon) }`, `EmitEvent { tag: EventTag::CombatAttack }`.
- **defend**: preconditions — `CoLocated { actor, target }`. Steps — defensive component mutation, `EmitEvent { tag: EventTag::CombatDefend }`.
- **loot**: preconditions — `CoLocated { actor, target }`, target-is-corpse, contention-grant-held. Steps — `Transfer { source: target, dest: actor, commodity, quantity }`, `ConsumeContentionGrant { grant }`, `EmitEvent { tag: EventTag::Loot }`.
- **bury**: preconditions — `CoLocated`, target-is-corpse, grave-plot present. Steps — corpse-burial component mutation, `EmitEvent { tag: EventTag::Bury }`.
- **heal**: preconditions — `CoLocated`, target-has-wound, contention-grant-held. Steps — wound mitigation, `ConsumeContentionGrant`, `EmitEvent { tag: EventTag::Heal }`.
- **queue_for_corpse_use** / **queue_for_care_target**: preconditions — `CoLocated` with the queue substrate. Steps — queue-membership mutation, `EmitEvent { tag: EventTag::QueueJoin }`.

The exact `EffectStep` variants required may extend the enum from ticket 001 — if combat needs a step variant not already defined (e.g., a body-part-specific damage step), add it to `EffectStep` in this ticket and implement the corresponding sink method in both authoritative and hypothetical impls.

### 2. Replace combat commit handler bodies with `apply_effects` delegation

Each `commit_*` handler in `combat.rs` shrinks to:

```rust
fn commit_attack(...) -> ActionOutcome {
    let schema = action_def.effect_schema.clone();
    apply_effects(&schema, actor, &targets, &mut authoritative_sink, EffectMode::Authoritative)
        .map(ActionOutcome::Completed)
        .unwrap_or_else(|d| ActionOutcome::Failed(d))
}
```

The imperative body is deleted, not preserved.

### 3. `EffectStep` and sink-method extensions if needed

If combat surfaces an effect not yet covered by the foundation enum (most likely `ApplyWound` already covers it; defensive component mutation may need a new variant), add the variant to `EffectStep` in `effect_schema.rs` and implement the sink method in both authoritative and hypothetical impls. Document the addition in the ticket implementation log so subsequent category tickets know the variant exists.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — 7 `EffectSchema` literals, 7 commit handler body replacements; existing tests around handlers may need adjustment to construct the new schemas)
- `crates/worldwake-sim/src/effect_schema.rs` (modify if `EffectStep` needs new variants for combat)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` (modify if new sink methods are added)
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify if new sink methods are added)

## Out of Scope

- Migrating non-combat action handlers (per-category tickets 004–009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Deleting `apply_hypothetical_transition`, `PlannerTransitionKind`, `apply_planner_step` (ticket 010).
- Changing `BindingStrictness`, `guard_template`, or `expectation_template` on combat actions (preserved unchanged per spec Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. All combat-touching goldens in `crates/worldwake-ai/tests/golden_*.rs` (enumerate during reassessment) produce bitwise-identical event logs to pre-ticket baseline.
2. `cargo test -p worldwake-systems combat` — existing inline tests around combat handlers pass with the schema-driven path.
3. `cargo test -p worldwake-ai golden_survival` — soak goldens produce identical canonical state hashes.
4. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Each combat action has a non-empty `EffectSchema` post-ticket (verified by registry-iteration unit test or by future ticket 010's coverage assertion).
2. No imperative handler body remains in `combat.rs` for the 7 migrated actions — each `commit_*` is a `apply_effects` delegation only.
3. Bitwise-identical canonical state hash on `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` and on every combat-touching golden, before and after this ticket (FND-12).
4. The planner still uses `apply_hypothetical_transition` after this ticket — no planner-side change yet.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` `#[cfg(test)]` block — modify existing handler tests so they exercise the schema-driven path; add focused tests covering schema-precondition failure cases (e.g., attack with no co-location yields `Discrepancy::NoLegalBinding` from the schema).
2. Existing combat goldens — no source change; they verify behavior is unchanged.

### Commands

1. `cargo test -p worldwake-systems combat`
2. `cargo test -p worldwake-ai golden_combat`
3. `cargo test -p worldwake-ai golden_survival`
4. `./scripts/verify.sh`
