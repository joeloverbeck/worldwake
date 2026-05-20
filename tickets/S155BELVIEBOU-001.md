# S155BELVIEBOU-001: Belief-correct `effective_place` for non-self entities

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` belief view (`SpatialBeliefView::effective_place` impl)
**Deps**: None (parallel with S155BELVIEBOU-002; shares `per_agent_belief_view.rs`, different method)

## Problem

`PerAgentBeliefView`'s `SpatialBeliefView::effective_place` (`crates/worldwake-sim/src/per_agent_belief_view.rs:951`) leaks **current authoritative location** for non-self entities the agent has not co-located with this tick. The non-self path returns `believed_entity(entity).last_known_place`, then falls back via `knows_entity(entity).then(|| self.world.effective_place(entity))`. Because `knows_entity()` returns true for entities known **only** through `institutional_beliefs` (a social claim) or `LastSeenMemory` (stale, prior-tick), an agent that merely remembers or was told about a target obtains its *live* position — the omniscience FND-14/FND-14A forbid. This propagates into snapshot admission, strategic place selection, and candidate emission, producing intelligent-looking but omniscient pursuit.

## Assumption Reassessment (2026-05-20)

<!-- Spec S155 reassessed this session (/reassess-spec); abbreviated spot-check confirmed targets. -->

1. **Current code**: `effective_place` is the `SpatialBeliefView` impl at `per_agent_belief_view.rs:951`. Its non-self path uses the `knows_entity()`-gated `or_else` fallback to `self.world.effective_place(entity)`. `knows_entity()` (`:292`) returns true for `institutional_beliefs` subjects and `LastSeenMemory` records (both non-co-located). `has_authoritative_local_visibility()` (`:285`) is same-tick co-location only. `LastSeenRecord { place: EntityId, .. }` and `LastSeenMemory { records: BTreeMap<EntityId, LastSeenRecord>, .. }` are at `crates/worldwake-core/src/expectation.rs:126,136`; `get_component_last_seen_memory` exists (used by `knows_entity`). All confirmed.
2. **Current specs/docs**: `specs/S155-belief-view-boundary-correctness.md` D1 (post-reassessment, in-place). `docs/FOUNDATIONS.md` FND-14, FND-14A (same-tick co-location is the only legal non-self authoritative read; off-place/delayed knowledge must be belief-backed).
3. **Shared boundary under audit**: the `SpatialBeliefView::effective_place` accessor — the belief view's location answer consumed by planning/snapshot. The contract: non-self location is belief/last-seen only unless same-tick co-located or directly possessed.
4. **Intended invariant (restated before trusting any scenario)**: an agent that last saw entity T at P1 and received no new observation/testimony/record must read T's place as P1 (or `None`), never T's current P2.
5. **Existing focused coverage** (`per_agent_belief_view.rs` `#[cfg(test)]`): `self_expectation_and_last_seen_queries_are_authoritative_only_for_self:2457`, `directly_possessed_item_lot_quantity_uses_authoritative_quantity_over_stale_belief:2587`, `current_place_entities_use_authoritative_local_set_over_stale_beliefs:2621`, `stale_beliefs_do_not_auto_refresh_from_world:2846`. These cover adjacent accessors (expectation/last-seen self-queries, possessed-quantity, co-located set) but **not** `effective_place`'s non-self fallback specifically — confirm during implementation that none asserts the leaking behavior; if one does, it encodes a bug and must be corrected, not preserved (never adapt tests to bugs).
6. **AI regression layer**: this is a belief-view accessor fix; intended verification is a focused unit test on `PerAgentBeliefView` plus the full AI golden suite (place narrowing may shift existing golden traces — re-baseline expected trace shifts, not world-outcome regressions).
13. **Adjacent contradictions**: the `can_control` gate gap is a *separate* root cause handled by S155BELVIEBOU-002 — out of scope here.

## Architecture Check

1. Narrowing `effective_place` at its single source is cleaner than tagging snapshot admission downstream (the S157 deferred approach): the leak is closed where it originates, so every consumer reads belief-correct locations with no per-consumer guard. Mirrors the existing FND-14A discipline already present for self/possession/co-location reads in this same impl block.
2. No backwards-compatibility shim: the `knows_entity()`-gated `or_else` branch is deleted, not wrapped. No alias accessor is introduced.

## Verification Layers

1. Non-self stale-location returns belief/last-seen place, never live truth → focused unit test on `PerAgentBeliefView` (decision-trace not needed at this layer; the accessor return value is the contract).
2. Same-tick co-located / directly-possessed reads still return authoritative location (FND-14A preserved) → focused unit test (positive cases).
3. No omniscient pursuit emerges downstream → full AI golden suite (`cargo test -p worldwake-ai`); covered end-to-end by S155BELVIEBOU-003's stale-location golden, not asserted here.

## What to Change

### 1. Rewrite the non-self path of `SpatialBeliefView::effective_place`

In `crates/worldwake-sim/src/per_agent_belief_view.rs:951`, keep the `entity == self.agent` authoritative return. For non-self entities, reach authoritative `self.world.effective_place(entity)` **only** when `has_authoritative_local_visibility(entity)` (same-tick co-location) **or** `self.world.possessor_of(entity) == Some(self.agent)` (direct possession). Otherwise return, in order: `believed_entity(entity).and_then(|s| s.last_known_place)`, then the actor's last-seen record place (`get_component_last_seen_memory(self.agent).and_then(|m| m.records.get(&entity).map(|r| r.place))`), then `None`. Delete the `knows_entity()`-gated `or_else` fallback to live truth.

### 2. Add focused unit test(s) (TDD — write first, confirm they fail against current code)

Stale-location: agent A has a last-seen/believed record for T at P1; move T to P2 in authoritative world with no new observation; assert `effective_place(T) == Some(P1)` (or `None` if no record), never `Some(P2)`. Positive controls: co-located T returns its authoritative place; directly-possessed item returns authoritative place.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `effective_place` impl body at `:951`; new `#[cfg(test)]` unit test(s))

## Out of Scope

- `can_control` belief gate — S155BELVIEBOU-002.
- Golden E2E stale-location pursuit + control-source-swap symmetry, and the `planner-contracts.md` doc line — S155BELVIEBOU-003.
- Snapshot admission-source provenance tagging — deferred to S157.
- Any change to authoritative `World::effective_place` itself.

## Acceptance Criteria

### Tests That Must Pass

1. New: non-self stale-location returns belief/last-seen place (P1) or `None`, never the moved authoritative place (P2).
2. New: co-located and directly-possessed non-self reads still return authoritative location (FND-14A preserved).
3. Existing suite: `cargo test -p worldwake-sim per_agent_belief_view` and `cargo test -p worldwake-ai` (re-baseline expected golden trace shifts caused by place narrowing; no world-outcome regressions).

### Invariants

1. For a non-self entity that is neither same-tick co-located nor directly possessed by the actor, `effective_place` never consults authoritative `World::effective_place` (FND-14/FND-14A).
2. The FND-14A same-tick co-location read and the direct-possession read remain authoritative.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (`#[cfg(test)]`) — stale-location negative case + co-location/possession positive controls for `effective_place`.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-ai` (Authoritative-to-AI Impact Rule golden check; re-baseline expected trace shifts)
3. `./scripts/verify.sh`
