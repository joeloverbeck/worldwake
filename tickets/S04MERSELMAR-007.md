# S04MERSELMAR-007: `SellCommodity` candidate generation, satisfaction, feasibility, and relevant places

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI decision pipeline wiring for SellCommodity goal
**Deps**: S04MERSELMAR-004, S04MERSELMAR-006

## Problem

`SellCommodity` is currently a deferred enum variant that hard-returns `false` for satisfaction, uses `demand_memory_places()` for relevant places (chasing demand signals instead of going to `home_market`), and is never emitted by candidate generation. This ticket makes `SellCommodity` a real proactive seller behavior by wiring candidate emission, goal satisfaction, feasibility, and relevant-place resolution.

## Assumption Reassessment (2026-03-31)

1. `is_satisfied` for `SellCommodity` at `goal_model.rs:962` is grouped with other goals that hard-return `false`. Confirmed.
2. `goal_relevant_places` for `SellCommodity` at `goal_model.rs:1038` delegates to `demand_memory_places(state, actor, *commodity)`. Confirmed — this incorrectly chases demand signals instead of returning `home_market`.
3. `FeasibilityStrategy::SellCheck` at `feasibility.rs:146-151` checks commodity quantity then delegates to `check_evidence_places_local`. Confirmed — this queries demand memory places, not `home_market` reachability.
4. Candidate generation in `candidate_generation.rs` currently emits `SellCommodity` nowhere (confirmed by grepping for `SellCommodity` emission in that file — only in test code). The enterprise module (`enterprise.rs`) may emit restock signals but not sell signals.
5. `MerchandiseProfile` has `home_market: EntityId` and `sale_kinds: BTreeSet<CommodityKind>`. Confirmed.
6. Belief view will have `listed_sale_lots_at` after ticket 004.
7. `PlannerOpKind::StaffMarket` and `SELL_OPS` update from ticket 006 must be in place.
8. The spec (Section 7) says emit `SellCommodity` when: has `MerchandiseProfile`, commodity in `sale_kinds`, at `home_market`, has local stock, no local stock currently listed. This avoids bootstrapping deadlock.
9. No adjacent contradictions found.

## Architecture Check

1. Making `SellCommodity` a real candidate follows the exact same candidate-emission pattern as `RestockCommodity`, `ProduceCommodity`, and other enterprise goals. No new patterns introduced.
2. Satisfaction checks concrete world state (at market + listed lots) rather than abstract profile state. This aligns with Principle 3.
3. Feasibility checks reachability of `home_market` rather than demand memory places, correctly separating demand-signal ranking (ticket 010) from feasibility gating.
4. No backwards-compatibility shims.

## Verification Layers

1. Candidate emission conditions -> focused unit test in candidate_generation.rs
2. Goal satisfaction (at home_market + listed lot) -> focused unit test in goal_model.rs
3. Feasibility (has commodity + home_market reachable) -> focused unit test in feasibility.rs
4. Relevant places returns [home_market] -> focused unit test in goal_model.rs
5. No candidate when already listed -> focused unit test (prevents re-emission)

## What to Change

### 1. Add `SellCommodity` candidate emission in `candidate_generation.rs`

Emit `SellCommodity { commodity }` when:
- actor has `MerchandiseProfile`
- `commodity` is in `MerchandiseProfile.sale_kinds`
- actor is at `MerchandiseProfile.home_market`
- actor controls at least one local lot of `commodity`
- no local controlled lot of `commodity` currently has `SaleListing`

Iterate over each commodity in `sale_kinds` and emit one candidate per commodity meeting the conditions.

### 2. Fix `goal_relevant_places` for `SellCommodity` in `goal_model.rs`

Replace line 1038:
```rust
GoalKind::SellCommodity { commodity } => demand_memory_places(state, actor, *commodity),
```
With:
```rust
GoalKind::SellCommodity { .. } => {
    // Return home_market from MerchandiseProfile
    state.merchandise_profile(actor)
        .map(|p| vec![p.home_market])
        .unwrap_or_default()
}
```

### 3. Fix `is_satisfied` for `SellCommodity` in `goal_model.rs`

Remove `SellCommodity` from the `false`-returning arm (line 962). Add a new match arm:
```rust
GoalKind::SellCommodity { commodity } => {
    // Satisfied when at home_market and at least one local lot is listed
    let profile = state.merchandise_profile(actor);
    let at_market = profile.map_or(false, |p| state.effective_place(actor) == Some(p.home_market));
    at_market && !state.listed_sale_lots_at(
        state.effective_place(actor).unwrap_or(EntityId::INVALID),
        *commodity,
    ).is_empty()
}
```

### 4. Fix `FeasibilityStrategy::SellCheck` in `feasibility.rs`

Replace `check_evidence_places_local` delegation with:
- Check actor has at least one unit of the commodity (existing check — keep)
- Check `home_market` from `MerchandiseProfile` is reachable (pathfinding check or place-existence check)
- Return `Likely` if both pass, `Unlikely` otherwise

### 5. Update `GoalDispatchDecl` for `SellCommodity`

Verify that the dispatch declaration in `goal_dispatch_decl.rs` correctly wires `SellCommodity` to `SELL_OPS` (the updated ops from ticket 006), `FeasibilityStrategy::SellCheck`, and the correct invalidation strategy.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add emission logic)
- `crates/worldwake-ai/src/goal_model.rs` (modify — fix relevant places and is_satisfied)
- `crates/worldwake-ai/src/feasibility.rs` (modify — fix SellCheck strategy)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — verify dispatch wiring)

## Out of Scope

- Demand memory ranking boost (ticket 010)
- Blocked-intent dampening (ticket 010)
- Buyer-side `AcquireCommodity` changes (ticket 008)
- `staff_market` action handler (ticket 003)
- `PlannerOpKind::StaffMarket` definition (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. Merchant at `home_market` with unlisted local stock emits `SellCommodity` candidate
2. Merchant at `home_market` with already-listed local stock does NOT emit `SellCommodity`
3. Merchant not at `home_market` does NOT emit `SellCommodity`
4. Merchant without `MerchandiseProfile` does NOT emit `SellCommodity`
5. `goal_relevant_places` returns `[home_market]` for `SellCommodity`
6. `is_satisfied` returns `true` when at `home_market` with listed lot, `false` otherwise
7. `SellCheck` feasibility returns `Likely` when commodity present and `home_market` reachable
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `SellCommodity` is a real proactive goal, not a deferred placeholder
2. Relevant places come from `MerchandiseProfile.home_market`, not demand memory
3. Satisfaction depends on concrete `SaleListing` state, not profile inference
4. Candidate emission avoids bootstrapping deadlock — no demand memory required

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for SellCommodity emission conditions
2. `crates/worldwake-ai/src/goal_model.rs` — focused tests for relevant places and is_satisfied
3. `crates/worldwake-ai/src/feasibility.rs` — focused test for SellCheck with home_market reachability

### Commands

1. `cargo test -p worldwake-ai -- candidate_generation`
2. `cargo test -p worldwake-ai -- goal_model`
3. `cargo test -p worldwake-ai -- feasibility`
4. `cargo clippy --workspace && cargo test --workspace`
