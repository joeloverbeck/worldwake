# S178PERFOOSPO-006: Spoiled-food candidate gating and freshness ranking

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — self-consume candidate emission in `candidate_generation.rs` gates known-spoiled lot-backed food candidates by `MetabolismProfile.spoiled_food_hunger_threshold`. `ranking.rs` applies a freshness factor to self-consume drive motive scores using lot evidence from `GoalBeliefView`.
**Deps**: `archive/tickets/S178PERFOOSPO-002.md`, `archive/tickets/S178PERFOOSPO-005.md`

## Problem

D5 makes Eat candidate emission respect lot freshness. Fresher believed lots are preferred (ranking via `motive_score`); spoiled-food candidates are suppressed unless current hunger exceeds the per-agent `spoiled_food_hunger_threshold` (D7 from ticket 002). All reads route through `GoalBeliefView::lot_condition`/`lot_freshness_band` (D8 from ticket 005) — never directly through authoritative `PerishableState` for remote lots. Implements `worldwake-validation-patterns.md` Candidate Scoring Architecture (emitters gate, ranking orders). Satisfies the spec's "Authoritative-to-AI Impact Analysis" point #2 flag.

## Assumption Reassessment (2026-06-02)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The live self-consume emitter is `emit_need_driven_candidates`, with lot-backed evidence supplied through `local_owned_commodity_evidence`, `local_unpossessed_commodity_evidence`, and listed-sale-lot acquisition support. The older ticket text that described direct Eat loops over `local_controlled_lots_for` and `last_known_inventory` was stale.
2. Spec D5 verified against current `archive/specs/S178-perishable-food-spoilage.md`. FND-14B mandates belief-mediated planner reads; the desperation gate's predicate is `believed_hunger >= profile.spoiled_food_hunger_threshold` (self-state hunger; per-agent profile-state threshold).
3. Shared abstraction boundary (precision-rules §1 — Phase Distinction): candidate emission as the gate point (per `worldwake-validation-patterns.md` Candidate Scoring Architecture — emitters gate via emit-or-skip; ranking orders emitted candidates via `motive_score`). Spoiled gating belongs in the emitter (gate logic before `emit_candidate_with_trace`); freshness preference belongs in `ranking.rs` via `motive_score` formula extension. No score field embedded in `GroundedGoal`.
4. Live `GoalKind` under test (precision-rules §13): local immediate food relief emits `GoalKind::ConsumeOwnedCommodity`; remembered or remote lot-backed food relief emits `GoalKind::AcquireCommodity { purpose: CommodityPurpose::SelfConsume, .. }`.

## Closeout (2026-06-02)

1. `spoiled_food_allowed_for_agent` now suppresses known-spoiled lot evidence when the agent's believed hunger is below `MetabolismProfile::spoiled_food_hunger_threshold`. Unknown freshness remains neutral; missing hunger suppresses only known-spoiled lots.
2. The gate applies to local owned lots, local unpossessed lots, and listed sale lots before acquisition support is admitted. Sale listings remain seller-backed evidence; their existing evidence shape was preserved.
3. `ranking.rs` now applies freshness scaling to self-consume drive motive scores from both fallback motive scoring and drive-provenance scoring. Fresh stays neutral, stale is damped, and spoiled is strongly damped.
4. All freshness reads route through `GoalBeliefView::lot_freshness_band`; no direct authoritative `PerishableState` or `World::get_component_perishable_state` read was added to the AI emitter/ranker.

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

1. `spoiled_owned_food_is_not_emitted_when_hunger_below_desperation_threshold` — passed.
2. `spoiled_owned_food_is_emitted_when_hunger_reaches_desperation_threshold` — passed.
3. `fresh_food_lot_ranks_above_stale_lot_for_self_consumption` — passed.
4. Source scan over `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/ranking.rs` found no direct `get_component_perishable_state`, `PerishableState`, or `World` read in the edited AI paths.
5. `spoiled_remote_loose_food_is_not_acquired_when_hunger_below_desperation_threshold` — passed and covers the remote/belief-view acquisition gate.
6. Existing requested filter `cargo test -p worldwake-ai candidate_generation::tests::eat` passed but matched zero live tests.

### Invariants

1. Spoiled-food candidate emission is gated by `believed_hunger >= profile.spoiled_food_hunger_threshold`, never by authoritative remote read (FND-14B compliance).
2. Ranking respects freshness band ordering (`Fresh > Stale > Spoiled`) consistently across decision ticks.
3. No `GroundedGoal` carries a score field — scoring stays in `ranking.rs::motive_score` per `worldwake-validation-patterns.md` Candidate Scoring Architecture.
4. Integer arithmetic only — no floats in freshness factor (AGENTS.md Determinism invariant).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — added focused unit coverage for local spoiled suppression, local threshold emission, and remote loose-lot suppression.
2. `crates/worldwake-ai/src/ranking.rs` `#[cfg(test)]` — added focused unit coverage that a fresh lot's self-consume motive score exceeds an otherwise-identical stale lot's score.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::eat` — passed; matched zero live tests.
2. `cargo test -p worldwake-ai candidate_generation::tests::spoiled_` — passed.
3. `cargo test -p worldwake-ai ranking::tests::fresh_food_lot_ranks_above_stale_lot_for_self_consumption` — passed.
4. `cargo test -p worldwake-ai` — passed.
5. `cargo test --workspace` and `./scripts/verify.sh` were deferred to the final S178 queued closeout.

## Outcome

Completed 2026-06-02.

The AI candidate emitter now suppresses known-spoiled food lot candidates below the acting agent's believed spoiled-food hunger threshold for local owned, local unpossessed, and listed-sale-lot acquisition support. Freshness is read only through `GoalBeliefView::lot_freshness_band`; no authoritative perishable-state read was added to AI candidate generation.

The AI ranker now scales self-consume drive motive scores by lot freshness evidence for both normal motive scoring and drive-provenance scoring. Unknown freshness remains neutral; fresh beats stale; spoiled is heavily damped.

Deviation from the original plan: the live emitter did not match the ticket's stale line-number narrative or its direct Eat-loop description. The implementation used the live self-consume evidence functions and preserved existing seller-backed sale-lot evidence shape.

Verification results: `cargo test -p worldwake-ai candidate_generation::tests::eat` passed with zero matching live tests; `cargo test -p worldwake-ai candidate_generation::tests::spoiled_` passed; `cargo test -p worldwake-ai ranking::tests::fresh_food_lot_ranks_above_stale_lot_for_self_consumption` passed; `cargo test -p worldwake-ai` passed. A source scan over edited AI files found no `get_component_perishable_state`, `PerishableState`, or `World` read in the edited candidate/ranking paths. Workspace and `./scripts/verify.sh` gates remain owned by final S178 queued closeout.
