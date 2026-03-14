# E14PERBEL-015: Remove Remote Infrastructure Discovery Leaks From PerAgentBeliefView Planner Reads

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` belief snapshot schema, `worldwake-sim` belief read-model contract, `worldwake-ai` planner/candidate-generation read paths, and focused regression coverage around remote infrastructure knowledge
**Deps**: `archive/tickets/completed/E14PERBEL-004.md`, `archive/tickets/completed/E14PERBEL-013.md`, `archive/specs/E14-perception-beliefs.md`, `specs/IMPLEMENTATION-ORDER.md`

## Problem

`PerAgentBeliefView` still leaks remote infrastructure discovery through authoritative world reads that are broader than a clean belief-mediated planner boundary:

- `matching_workstations_at()` scans authoritative entities at a place
- `resource_sources_at()` scans authoritative entities at a place
- `workstation_tag()` and `resource_source()` also read authoritative components directly once the planner has an entity id
- a live planner unit test currently proves the leak: reachable remote harvest sources produce an acquire goal without any explicit knowledge fixture for the remote workstation/resource source

That architecture is workable, but it is not the clean long-term shape:

1. it lets the planner discover remote affordance-bearing entities without any explicit information path
2. it mixes genuinely public topology knowledge with discovered local infrastructure/resource presence
3. it leaves affordance-bearing facility metadata outside the belief snapshot, so stale or absent facility knowledge is not modeled explicitly

The result is a boundary that still behaves more omniscient than the project’s causal-information model wants.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` currently answers `matching_workstations_at()` and `resource_sources_at()` by scanning authoritative `World` entities at a place. It also answers `workstation_tag()` and `resource_source()` straight from authoritative components instead of belief snapshots.
2. `archive/specs/E14-perception-beliefs.md` currently classifies `place_has_tag`, `workstation_tag`, `matching_workstations_at`, and `resource_sources_at` as authoritative “topology queries.” That grouping is too coarse for affordance-bearing facility/resource knowledge.
3. The live codebase already contains harness regressions proving generic remote entity knowledge is not seeded by default:
   - `crates/worldwake-ai/tests/golden_harness/mod.rs::setup_does_not_seed_remote_beliefs_by_default`
   - `crates/worldwake-ai/tests/golden_harness/mod.rs::explicit_local_belief_seeding_is_bounded_to_colocated_entities`
   This ticket should therefore focus on the narrower remaining leak: remote infrastructure/resource discovery.
4. The live planner unit test `crates/worldwake-ai/src/candidate_generation.rs::remote_harvest_source_within_travel_horizon_emits_acquire_goal` demonstrates the current leak directly. The planner can discover a reachable remote harvest source from route connectivity plus authoritative place scans alone.
5. `archive/tickets/completed/E14PERBEL-013.md` exposed this seam operationally. Some route/restock goldens still required explicit actor-scoped prior-world fixtures because the planner depends on known remote place identity across the public route graph. That does not justify free discovery of remote workstation/resource entities.
6. `PerAgentBeliefView` is still the active E14 read adapter, but `archive/specs/E14-perception-beliefs.md` already notes that its broad trait surface is interim and may be split later. This ticket should tighten the contract now without waiting for a full trait redesign.
7. `place_has_tag()` is not currently shown to be the same class of leak. In live AI code, remote goal generation is driven by workstation/resource discovery, while `place_has_tag()` is primarily consumed later by planning-state/precondition logic. Unless implementation uncovers a concrete tag leak, this ticket should treat place tags as route/place structure rather than the main bug.
8. No active ticket in `tickets/` currently owns this planner/belief boundary cleanup.

## Architecture Check

1. The cleaner architecture is to separate:
   - public topology/navigation knowledge
   - discovered infrastructure/resource presence at places
   - authoritative execution-time validation
2. The cleanest implementation path is to make affordance-bearing facility/resource facts part of `BelievedEntityState` itself:
   - workstation identity/tag is believed entity metadata
   - resource-source state is believed entity metadata, including quantity
   - place/entity discovery methods compose from those believed snapshots instead of rescanning `World`
3. This is better than the current design because it keeps belief-only planning honest and preserves staleness for facility/resource observations rather than auto-refreshing them from authoritative state.
4. The recommended change is not to make agents ignorant of the map. The route graph can remain public while remote facility/resource presence becomes belief-mediated or explicitly modeled as prior knowledge.
5. This is cleaner than preserving the current coarse authoritative fallback because it gives the code one defensible rule: the planner may use global navigation structure, but it may not invent remote opportunity-bearing entities without a lawful knowledge path.
6. No backwards-compatibility shim or alias path is acceptable. If a `BeliefView` method is too broad for clean semantics, narrow or replace the usage now rather than preserving the leak.

## What to Change

### 1. Reclassify remote infrastructure/resource discovery away from “public topology”

Update the E14 contract so that:

- adjacency and travel-time graph knowledge may remain globally available
- remote place identity and route connectivity may remain globally available if needed
- remote facility presence, workstation matching, workstation metadata, and remote resource-source presence are no longer treated as free authoritative planner knowledge

This likely requires updating `archive/specs/E14-perception-beliefs.md` first so the code and spec agree on the boundary.

### 2. Move affordance-bearing facility/resource facts into belief snapshots

Extend the belief-side entity snapshot so the planner can reason from explicit known entities rather than rescanning authoritative world state:

- `BelievedEntityState` should carry the facility/resource facts needed for planning, at minimum:
  - workstation tag when the believed entity is a workstation/facility
  - resource-source snapshot when the believed entity exposes a harvestable/source affordance
- `build_believed_entity_state()` and explicit test belief seeding should populate those fields
- belief-view queries should answer workstation/resource methods from believed entity snapshots plus self-authoritative local state where appropriate

This keeps the architecture extensible: if later facility-affecting facts matter to planning, they have an explicit place in belief state rather than another hidden authoritative side channel.

### 3. Remove authoritative facility/resource scans from planner belief reads

Refactor `PerAgentBeliefView` and the planner-facing callers so that AI planning no longer depends on:

- authoritative `matching_workstations_at(place, tag)` for remote discovery
- authoritative `resource_sources_at(place, commodity)` for remote discovery
- authoritative `workstation_tag(entity)` / `resource_source(entity)` for non-self remote entity knowledge

Execution-time validation and world systems may continue using `World` directly where appropriate.

### 4. Add regressions for the cleaned boundary

Add tests proving the planner:

- can still navigate the public route graph
- cannot discover a remote opportunity-bearing entity without explicit knowledge
- can act on a remote opportunity once that knowledge is explicitly seeded or lawfully perceived

Include at least one regression at the `PerAgentBeliefView` boundary and at least one end-to-end AI regression.

## Files to Touch

- `archive/specs/E14-perception-beliefs.md` (modify)
- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify only if snapshot assembly needs to respect the narrowed belief contract)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify if new explicit knowledge helpers are needed)
- `crates/worldwake-ai/tests/golden_production.rs` and/or `crates/worldwake-ai/tests/golden_trade.rs` (modify if a golden remote-opportunity regression is added)

## Out of Scope

- Rumor/report propagation beyond existing E14/E15 scope
- A full redesign of every `BeliefView` method
- Action-handler or authoritative world validation changes unrelated to planner read semantics
- Reintroducing omniscient harness behavior
- Merchant-selling market-presence work from `S04`

## Acceptance Criteria

### Tests That Must Pass

1. Planner-side remote facility/resource discovery no longer comes from authoritative place scans or authoritative non-self facility/resource component reads through `PerAgentBeliefView`.
2. A regression proves an agent can still traverse public route topology without knowing remote opportunity-bearing entities by default.
3. A regression proves an agent does not pursue a remote harvest/trade/care opportunity until the relevant remote facility/resource entity is explicitly known.
4. A regression proves the same remote opportunity becomes usable once that facility/resource knowledge is explicitly seeded or lawfully perceived.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo test --workspace`
7. Existing lint: `cargo clippy --workspace`

### Invariants

1. Belief-only planning does not discover remote opportunity-bearing entities through authoritative world scans.
2. Public route graph knowledge and discovered infrastructure/resource entity knowledge are modeled as distinct concepts.
3. Execution-time validation remains authoritative even if planner-side discovery becomes belief-mediated.
4. No backwards-compatibility alias or hidden omniscient fallback is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` and/or `crates/worldwake-sim/src/per_agent_belief_view.rs` — add regressions proving affordance-bearing workstation/resource facts are captured in belief snapshots and unavailable without explicit knowledge while route-graph reads still work.
   Rationale: locks the read-model boundary directly where the leak currently lives.
2. `crates/worldwake-ai/src/candidate_generation.rs` or `crates/worldwake-ai/src/planning_snapshot.rs` tests — prove remote opportunity generation requires explicit knowledge of the remote entity/infrastructure, not just route connectivity.
   Rationale: covers the planner-side consequence of the boundary cleanup.
3. `crates/worldwake-ai/tests/golden_trade.rs` or `crates/worldwake-ai/tests/golden_production.rs` — add or update one end-to-end remote-opportunity scenario showing route knowledge alone is insufficient, and explicit seeded knowledge unlocks the journey.
   Rationale: proves the cleanup survives outside the simplest decision-only case.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - extended `BelievedEntityState` to snapshot affordance-bearing facility/resource facts via `workstation_tag` and `resource_source`
  - changed `PerAgentBeliefView` so remote workstation/resource discovery comes from believed entity snapshots rather than authoritative place scans
  - kept route topology, place tags, and place identity public so planners can still navigate the map without remote facility omniscience
  - updated the merchant restock golden to use explicit remote workstation knowledge instead of broad actor-scoped world belief seeding
  - added regressions covering the belief-view boundary, the remote restock negative case, and the positive explicit-knowledge path
- Deviations from the original draft:
  - `place_has_tag()` was not changed because the live bug was workstation/resource discovery, not place-tag reads
  - the implementation needed one additional boundary correction beyond the original draft: `EntityKind::Place` remains public so explicit remote facility knowledge can be acted on without reintroducing broad prior-world fixtures
- Verification results:
  - `cargo test -p worldwake-sim per_agent_belief_view -- --nocapture` ✅
  - `cargo test -p worldwake-ai --test golden_trade -- --nocapture` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
