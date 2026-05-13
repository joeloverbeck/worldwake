# S143STABELVIE-004: Migrate co-located physical observation to `LocalPhysicalObservationView::colocated_entities`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — belief-view trait surface migration; observation call sites re-routed; `RuntimeBeliefView` supertrait extended.
**Deps**: archive/tickets/S143STABELVIE-001.md, archive/tickets/S143STABELVIE-002.md, archive/tickets/S143STABELVIE-003.md

## Problem

S143's compile-error guarantee for FND-14A requires `locally_observed_entities_at` to live on exactly one trait — `LocalPhysicalObservationView::colocated_entities` with the new `ObservedRead<Vec<EntityId>>` return shape. Currently the method is declared on `SpatialBeliefView` (`belief_view.rs:909`) AND has default-impl forwarding declarations on `GoalSpatialBeliefView` (`belief_view.rs:224`) and `GoalBeliefView` (`belief_view.rs:283`). Without consolidation, the planner can continue reading the method through `GoalBeliefView` with the legacy `Vec<EntityId>` shape, bypassing the spec's `ObservedRead` provenance wrapper.

## Assumption Reassessment (2026-05-13)

1. `locally_observed_entities_at` declarations and impls in `crates/worldwake-sim/src/belief_view.rs`:
   - `SpatialBeliefView::locally_observed_entities_at` declaration: line 909 (the canonical source).
   - `GoalSpatialBeliefView::locally_observed_entities_at` default impl: line 224 (forwards to `entities_at` by default, but is overridden by the blanket impl at line 1436 that forwards to `SpatialBeliefView::locally_observed_entities_at`).
   - `GoalBeliefView::locally_observed_entities_at` default impl: line 283 (similar; overridden by blanket impl at line 1527).
   Consumer files (per workspace grep): `worldwake-ai/src/theft.rs`, `worldwake-ai/src/candidate_generation.rs`, `worldwake-ai/src/agent_tick/observation.rs`, plus `worldwake-sim/src/per_agent_belief_view.rs` (provider).
2. Spec D3 audit table cites `EntityBeliefView (791)` as a "duplicate" source of `locally_observed_entities_at`. Correction: the method at `belief_view.rs:791` is `locally_observed_is_dead`, not `locally_observed_entities_at` (verified by direct read of lines 789–817). The actual duplicate surface that needs migrating consists of the `GoalSpatialBeliefView` and `GoalBeliefView` default impls plus their blanket-impl forwarders. The spec's table is updated implicitly by this ticket's What to Change; the audit-row inaccuracy is documented here and does not block ticket execution.
3. The new `LocalPhysicalObservationView::colocated_entities` returns `ObservedRead<Vec<EntityId>>` — `value` is the same `Vec<EntityId>` the current method returns; `observed_tick` is the current tick; `source` is `ObservationSource::CoLocatedSameTick`. The canonical `PerAgentBeliefView::colocated_entities` impl was introduced in completed ticket 002 with the co-located authoritative read path; this ticket removes the legacy declarations and migrates callers to that canonical method.
4. Adjacent contradiction (was item 13): per Step 2's 1-3-1 (a) approval, this ticket extends the spec's D3 audit table scope to also migrate the `GoalSpatialBeliefView` and `GoalBeliefView` default declarations. Classification: required consequence of the spec's compile-error guarantee reaching `GoalBeliefView` (the planner's primary read surface). The fix in this ticket is removal of the default-impl declarations from both Goal-prefixed surfaces, paired with the blanket-impl rewrites.

## Architecture Check

1. FND-28-clean: removing the old method declarations from all source traits (`SpatialBeliefView`, `GoalSpatialBeliefView`, `GoalBeliefView`) avoids alias paths. `LocalPhysicalObservationView::colocated_entities` is the single authoritative form.
2. The consolidation onto a single trait follows the spec's "no method appears on more than one of these" principle.
3. `ObservedRead<Vec<EntityId>>` shape gives observer rendering the `source: CoLocatedSameTick` provenance for free — no new provenance machinery needed; the wrapper carries it.
4. Test-mock cascade is mechanical: add empty `impl LocalPhysicalObservationView for <MockType> {}` blocks (~15 sites) that absorb the trait's default impls.

## Verification Layers

1. Compile-time surface: `colocated_entities` is reachable only via `LocalPhysicalObservationView` (or `RuntimeBeliefView` via the new supertrait + explicit `use`). Verified by `cargo build --workspace`.
2. Return-type contract: `ObservedRead<Vec<EntityId>>` carries `observed_tick` and `source` provenance. Verified by focused test.
3. Behavioral equivalence: for co-located queries, `colocated_entities(actor).value` equals the legacy `locally_observed_entities_at(actor, current_place)` result for the same agent/place. Verified by a new focused test on `PerAgentBeliefView` against scenario fixtures.
4. Existing golden coverage continues to pass — `worldwake-ai/tests/golden_theft_*.rs` and other goldens that exercise observation behavior.

## What to Change

### 1. Trait declaration changes in `crates/worldwake-sim/src/belief_view.rs`

- Remove `fn locally_observed_entities_at(...)` from `SpatialBeliefView` (line 909).
- Remove `fn locally_observed_entities_at(...)` (default impl) from `GoalSpatialBeliefView` (line 224).
- Remove `fn locally_observed_entities_at(...)` (default impl) from `GoalBeliefView` (line 283).
- Extend `RuntimeBeliefView`'s supertrait list at line 1403 — insert `+ LocalPhysicalObservationView` alongside `BelievedAuthorityView` (landed in archived ticket 003). Reassessment at implementation start must confirm the live supertrait list before patching.
- Update `GoalSpatialBeliefView` blanket impl (around line 1436) — remove the `locally_observed_entities_at` forwarding stanza (the method no longer lives on the source trait).
- Update `GoalBeliefView` blanket impl (around line 1527) — same.

### 2. Canonical impl updates in `crates/worldwake-sim/src/per_agent_belief_view.rs`

- Remove `locally_observed_entities_at` impl from `impl SpatialBeliefView for PerAgentBeliefView`.
- Preserve the `impl LocalPhysicalObservationView for PerAgentBeliefView` landed in completed ticket 002 — `colocated_entities(actor)` already provides the canonical impl: returns `ObservedRead { value: <co-located entities>, observed_tick: <current tick>, source: ObservationSource::CoLocatedSameTick }` using the same authoritative-state read path the removed `SpatialBeliefView::locally_observed_entities_at` impl used.

### 3. Consumer call-site migration

For each consumer file, add `use worldwake_sim::LocalPhysicalObservationView;` and replace `view.locally_observed_entities_at(actor, place)` with `view.colocated_entities(actor).value` (or pattern-match on `ObservedRead` when the call site needs `observed_tick`/`source` provenance). Files:

- `worldwake-ai/src/theft.rs` — observation site that gates theft candidate emission.
- `worldwake-ai/src/candidate_generation.rs` — observation site in the candidate emission pipeline.
- `worldwake-ai/src/agent_tick/observation.rs` — observation site in the per-tick observation pass.

D6 import narrowing (distributed): in each consumer file, evaluate whether the file's reads are entirely covered by `LocalPhysicalObservationView` + 0–2 other sub-traits; narrow the `RuntimeBeliefView` import where possible. The hard goal (per spec D6) is "no belief-view import in `worldwake-ai` reaches `DebugWorldView`" — verified by ticket 005's lint.

### 4. Test-mock cascade

Add an empty `impl LocalPhysicalObservationView for <MockType> {}` block at every site that currently `impl RuntimeBeliefView for <MockType> {}`. Same site list as ticket 003 (~15 sites across `worldwake-ai/src/**` and `worldwake-ai/tests/**`). The default impls in `LocalPhysicalObservationView` (returning empty `ObservedRead` per ticket 002) absorb the cascade — no method overrides needed at these sites unless the test specifically exercises co-located observation.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait declarations, blanket impls, supertrait composition)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — impl moves)
- `crates/worldwake-ai/src/theft.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- Test-mock cascade (~15 files — same as ticket 003 cascade list): add empty `impl LocalPhysicalObservationView for <MockType> {}` blocks

Likely: additional consumer files may surface during implementation. The implementer should re-grep at start (`grep -rn "locally_observed_entities_at" crates/`) to confirm the consumer list is current.

## Out of Scope

- Authority method migration (`believed_owner_of`, `believed_office_holder`) — ticket 003.
- CI lint (D7) — ticket 005.
- Golden coverage (D8, including the belief-wall trap regression) — ticket 006.
- Narrowing of test mocks to specific sub-traits — out of scope per spec D6.
- Migration of other `locally_observed_*` methods on existing sub-traits (`locally_observed_is_dead` on `EntityBeliefView`, `locally_observed_commodity_quantity` on `InventoryBeliefView`, `locally_observed_bandit_camp_faction_at` on `PoliticalBeliefView`) — explicitly retained on their current traits per spec D3's "Methods staying" table.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `LocalPhysicalObservationView::colocated_entities` on `PerAgentBeliefView` returns `ObservedRead { value, observed_tick: <current>, source: CoLocatedSameTick }` matching the legacy `locally_observed_entities_at` return value for a fixture scenario.
2. New focused test: `colocated_entities` returns an empty `ObservedRead` (or `source: CoLocatedSameTick` with empty `value`) when the agent is not co-located with any other entity.
3. Existing goldens in `crates/worldwake-ai/tests/` that exercise theft, candidate generation, or observation continue to pass — specifically `golden_theft_*.rs` and any other tests that touch the migrated consumers.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. `colocated_entities` is reachable only via `LocalPhysicalObservationView`.
2. No `SpatialBeliefView`, `GoalSpatialBeliefView`, or `GoalBeliefView` declaration contains `locally_observed_entities_at` after this ticket.
3. `ObservedRead::source` on `LocalPhysicalObservationView::colocated_entities` results is always `CoLocatedSameTick` on the canonical `PerAgentBeliefView` impl (the trait is for same-tick co-located reads only; the `BeliefStoreSnapshot` variant exists on `ObservationSource` for future belief-store-cached observation reads, not for this method).
4. FND-14A wall: `colocated_entities` returns only entities the agent is currently co-located with (the same set the removed `locally_observed_entities_at` returned).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — `LocalPhysicalObservationView::colocated_entities` behavior test exercising the canonical impl.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — equivalence test comparing the new return value against the legacy method's return value for a controlled scenario.
3. Existing goldens — verified to pass after consumer-call-site migration.

### Commands

1. `cargo test -p worldwake-sim local_physical_observation`
2. `cargo test -p worldwake-ai theft`
3. `cargo test -p worldwake-ai candidate_generation`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh`
