# S178PERFOOSPO-006: Spoiled-food candidate gating and freshness ranking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — Eat candidate emitter in `candidate_generation.rs` ranks by believed lot condition and gates spoiled-food candidates by `MetabolismProfile.spoiled_food_hunger_threshold`. `ranking.rs` gains a freshness factor in `motive_score` for Eat candidates.
**Deps**: 002, 005

## Problem

D5 makes Eat candidate emission respect lot freshness. Fresher believed lots are preferred (ranking via `motive_score`); spoiled-food candidates are suppressed unless current hunger exceeds the per-agent `spoiled_food_hunger_threshold` (D7 from ticket 002). All reads route through `GoalBeliefView::lot_condition`/`lot_freshness_band` (D8 from ticket 005) — never directly through authoritative `PerishableState` for remote lots. Implements `worldwake-validation-patterns.md` Candidate Scoring Architecture (emitters gate, ranking orders). Satisfies the spec's "Authoritative-to-AI Impact Analysis" point #2 flag.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Eat emitter lives in `crates/worldwake-ai/src/candidate_generation.rs` around lines 1764-1795 (lot iteration via `view.local_controlled_lots_for(agent, current_place, commodity)`) and lines 1772-1791 (belief-store-emitted candidates for remote lots reading `last_known_place` and `last_known_inventory`). Lines 2504-2510 carry the hunger-relieving commodity filter (`view.item_lot_consumable_profile(item).is_some_and(|p| p.hunger_relief_per_unit.value() > 0)`). Current ranking does not sort by freshness; no desperation gate exists. The emitter reads from `GoalBeliefView` exclusively — no direct authoritative `PerishableState` read today. `#[cfg(test)]` boundary at line 7593.
2. Spec D5 verified against current `specs/S178-perishable-food-spoilage.md`. FND-14B mandates belief-mediated planner reads; the desperation gate's predicate is `believed_hunger >= profile.spoiled_food_hunger_threshold` (self-state hunger; per-agent profile-state threshold).
3. Shared abstraction boundary (precision-rules §1 — Phase Distinction): candidate emission as the gate point (per `worldwake-validation-patterns.md` Candidate Scoring Architecture — emitters gate via emit-or-skip; ranking orders emitted candidates via `motive_score`). Spoiled gating belongs in the emitter (gate logic before `emit_candidate_with_trace`); freshness preference belongs in `ranking.rs` via `motive_score` formula extension. No score field embedded in `GroundedGoal`.
4. Live `GoalKind` under test (precision-rules §13): the relevant goal family for Eat candidate emission is the existing hunger-relief goal kind (verify exact name at `crates/worldwake-ai/src/candidate_generation.rs` Eat emitter declaration). The emitter's existing iteration over `local_controlled_lots_for` is the integration point.

## Architecture Check

1. Reading lot freshness through `GoalBeliefView::lot_freshness_band` (not directly through `PerishableState`) is the FND-14B-compliant path. A remote cache believed Fresh emits a candidate; on arrival the agent re-ranks against authoritative observation (action-commit FND-14A read via ticket 004 / belief-update via ticket 005's perception write).
2. Splitting the emission gate (spoiled-or-not) from the ranking score (fresher-preferred) follows `worldwake-validation-patterns.md` Candidate Scoring Architecture — gates in emitter, ordering in `ranking.rs`. No score field on `GroundedGoal`.

## Verification Layers

1. Spoiled-food candidate suppressed when believed hunger < threshold → decision-trace assertion (candidate-generation focused/unit coverage per precision-rules §3).
2. Spoiled-food candidate emitted when believed hunger ≥ threshold → decision-trace assertion at the desperation boundary.
3. Fresher lot ranked above stale lot when both emitted → `motive_score` unit test on the freshness factor.
4. No direct `world.get_component_perishable_state` call in candidate emission for remote lots → grep-regression in the new tests + diff inspection during implementation (Auth-to-AI Impact #2 verification).

## What to Change

### 1. Desperation gate in Eat emitter (locally-controlled lots)

In `crates/worldwake-ai/src/candidate_generation.rs`, modify the Eat candidate emission loop at lines 1764-1795:

```rust
for lot in view.local_controlled_lots_for(agent, current_place, commodity) {
    let band = view.lot_freshness_band(lot);
    if band == Some(Freshness::Spoiled) {
        let hunger = view.agent_hunger(agent);
        let threshold = view.metabolism_profile(agent)
            .map(|p| p.spoiled_food_hunger_threshold)
            .unwrap_or_else(|| Permille::new_unchecked(800));
        if hunger.value() < threshold.value() {
            continue; // suppress spoiled candidate when not desperate
        }
    }
    // ... existing emission code
}
```

`view.metabolism_profile(agent)` is the existing accessor for per-agent metabolism (verify exact name during implementation; if absent, add as a belief-view accessor — it would be a sibling addition to D8's lot-condition accessors).

### 2. Desperation gate for belief-store-emitted candidates

Apply the same gate to the belief-store-emitted Eat candidates at lines 1772-1791 — these are candidates for remote lots the agent remembers. `view.lot_freshness_band(remote_lot)` returns the believed band (Fresh / Stale / Spoiled) via ticket 005's belief-backed read; if Spoiled and hunger < threshold, skip.

### 3. Freshness preference in `motive_score`

In `crates/worldwake-ai/src/ranking.rs`, extend `motive_score` for Eat-goal candidates to include a freshness factor:

```rust
let freshness_factor = match view.lot_freshness_band(lot) {
    Some(Freshness::Fresh) => Permille::new_unchecked(1000),
    Some(Freshness::Stale) => Permille::new_unchecked(700),
    Some(Freshness::Spoiled) => Permille::new_unchecked(300),
    None => Permille::new_unchecked(1000), // unknown — assume fresh; observation corrects on arrival
};
let scaled = (base_score as u32 * freshness_factor.value() as u32 / 1000) as u16;
```

Integer arithmetic only (AGENTS.md Determinism invariant). The factor multiplies the existing hunger-pressure-derived base score; relative ordering Fresh > Stale > Spoiled is preserved.

### 4. Auth-to-AI Impact #2 verification

Verify no candidate emission path reads `world.get_component_perishable_state(lot)` directly for a remote lot. Add a new test (`candidate_emission_for_remote_lot_uses_belief_view_only`) that asserts the emitter never resolves a remote lot's authoritative `PerishableState` — only belief-view accessors are called.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — Eat emitter gate at lines 1764-1795 + belief-store emitter gate at lines 1772-1791)
- `crates/worldwake-ai/src/ranking.rs` (modify — freshness factor on Eat `motive_score`)

## Out of Scope

- Eat precondition changes (Eat continues to allow spoiled food — gate is in emission only, not precondition).
- Belief-view accessor implementation (ticket 005).
- Forensic record (ticket 007).
- Per-commodity desperation thresholds (the field on `MetabolismProfile` is per-agent only per ticket 002's scope).
- Authoritative-state reads for action commit (ticket 004 — uses direct read at action commit per FND-14A).

## Acceptance Criteria

### Tests That Must Pass

1. `spoiled_lot_not_emitted_when_hunger_below_threshold` — decision-trace shows zero candidates emitted for the spoiled lot when `view.agent_hunger(agent) < spoiled_food_hunger_threshold`.
2. `spoiled_lot_emitted_when_hunger_above_threshold` — decision-trace shows candidate emitted with spoiled-band evidence when hunger exceeds threshold.
3. `fresh_lot_ranks_above_stale_lot_when_both_emitted` — `motive_score(fresh_candidate) > motive_score(stale_candidate)` given identical other inputs.
4. `candidate_emission_for_remote_lot_uses_belief_view_only` — diff inspection + runtime assertion: no `world.get_component_perishable_state` call in emitter path for remote lots.
5. `desperation_gate_applies_to_belief_store_emitted_candidates` — gate fires for remote lots emitted via belief-store iteration, not just `local_controlled_lots_for` iteration.
6. Existing: `cargo test -p worldwake-ai candidate_generation::tests::eat`.

### Invariants

1. Spoiled-food candidate emission is gated by `believed_hunger >= profile.spoiled_food_hunger_threshold`, never by authoritative remote read (FND-14B compliance).
2. Ranking respects freshness band ordering (`Fresh > Stale > Spoiled`) consistently across decision ticks.
3. No `GroundedGoal` carries a score field — scoring stays in `ranking.rs::motive_score` per `worldwake-validation-patterns.md` Candidate Scoring Architecture.
4. Integer arithmetic only — no floats in freshness factor (AGENTS.md Determinism invariant).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — 5 unit tests (suppression below threshold, emission above threshold, belief-view-only read verification, belief-store-emission gate, fresh-vs-stale ranking input).
2. `crates/worldwake-ai/src/ranking.rs` `#[cfg(test)]` — 1 unit test for the freshness factor in `motive_score`.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::spoiled candidate_generation::tests::fresh`
2. `cargo test --workspace`
3. `./scripts/verify.sh`
