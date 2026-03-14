# E14PERBEL-015: Remove Remote Infrastructure Discovery Leaks From PerAgentBeliefView Planner Reads

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief read-model contract, `worldwake-ai` planner/candidate-generation read paths, and focused regression coverage around remote infrastructure knowledge
**Deps**: `archive/tickets/completed/E14PERBEL-004.md`, `archive/tickets/completed/E14PERBEL-013.md`, `specs/E14-perception-beliefs.md`, `specs/IMPLEMENTATION-ORDER.md`

## Problem

`PerAgentBeliefView` still leaks remote infrastructure discovery through authoritative world reads that are broader than a clean belief-mediated planner boundary:

- `place_has_tag()` reads authoritative place tags directly
- `matching_workstations_at()` scans authoritative entities at a place
- `resource_sources_at()` scans authoritative entities at a place
- route/restock goldens currently need explicit actor-scoped prior-world fixtures because the planner still depends on known remote place and facility identity to act on distant opportunities

That architecture is workable, but it is not the clean long-term shape:

1. it lets the planner discover remote affordance-bearing entities without any explicit information path
2. it mixes genuinely public topology knowledge with discovered local infrastructure/resource presence
3. it forces tests to encode hidden planner assumptions about what counts as “public knowledge” versus “belief-backed knowledge”

The result is a boundary that still behaves more omniscient than the project’s causal-information model wants.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` currently answers `place_has_tag()`, `matching_workstations_at()`, and `resource_sources_at()` from authoritative `World` data rather than from agent belief state.
2. `specs/E14-perception-beliefs.md` currently describes topology queries, including `place_has_tag`, `matching_workstations_at`, and `resource_sources_at`, as authoritative because the place graph is public infrastructure.
3. The live code and newly-updated goldens show that this grouping is too coarse:
   - public route graph knowledge is plausibly global
   - remote facility existence and remote resource-source presence are not equivalent to “the place graph is public”
4. `archive/tickets/completed/E14PERBEL-013.md` exposed this seam directly. Some goldens needed explicit actor-scoped prior-world fixtures because the current planner assumes remote place/facility identity knowledge before any lawful observation path exists.
5. `PerAgentBeliefView` is still the active E14 read adapter, but `specs/E14-perception-beliefs.md` already notes that its broad trait surface is interim and may be split later. This ticket should tighten the contract without waiting for a full trait redesign.
6. No active ticket in `tickets/` currently owns this planner/belief boundary cleanup. `E14PERBEL-014` is about corpse belief evidence and is distinct.

## Architecture Check

1. The cleaner architecture is to separate:
   - public topology/navigation knowledge
   - discovered infrastructure/resource presence at places
   - authoritative execution-time validation
2. This is better than the current design because it prevents planner-side remote discovery from piggybacking on “public topology” and keeps belief-only planning honest.
3. The recommended change is not to make agents ignorant of the map. The route graph can remain public while remote facility/resource presence becomes belief-mediated or explicitly modeled as prior knowledge.
4. This is cleaner than preserving the current coarse authoritative fallback because it gives the code one defensible rule: the planner may use global navigation structure, but it may not invent remote opportunity-bearing entities without a lawful knowledge path.
5. No backwards-compatibility shim or alias path is acceptable. If a `BeliefView` method is too broad for clean semantics, narrow or replace the usage now rather than preserving the leak.

## What to Change

### 1. Reclassify remote infrastructure/resource discovery away from “public topology”

Update the E14 contract so that:

- adjacency and travel-time graph knowledge may remain globally available
- remote place identity and route connectivity may remain globally available if needed
- remote facility presence, workstation matching, and remote resource-source presence are no longer treated as free authoritative planner knowledge

This likely requires updating `specs/E14-perception-beliefs.md` first so the code and spec agree on the boundary.

### 2. Introduce an explicit planner-facing knowledge path for remote opportunity-bearing entities

Replace the current implicit discovery path with one of these explicit models:

- belief-backed remote infrastructure/resource knowledge stored in agent belief state
- a narrow prior-knowledge component/read-model for public institutional infrastructure the actor is allowed to know
- a split read surface where public-topology methods stay authoritative but facility/resource discovery methods require belief-backed evidence

The exact implementation may vary, but the planner must stop learning about remote harvest/trade/care opportunities merely by scanning authoritative entities at a place.

### 3. Remove authoritative facility/resource scans from planner belief reads

Refactor `PerAgentBeliefView` and the planner-facing callers so that AI planning no longer depends on:

- authoritative `matching_workstations_at(place, tag)` for remote discovery
- authoritative `resource_sources_at(place, commodity)` for remote discovery
- authoritative place-tag reads where the tag encodes actionable local infrastructure rather than route graph structure

Execution-time validation and world systems may continue using `World` directly where appropriate.

### 4. Add regressions for the cleaned boundary

Add tests proving the planner:

- can still navigate the public route graph
- cannot discover a remote opportunity-bearing entity without explicit knowledge
- can act on a remote opportunity once that knowledge is explicitly seeded or lawfully perceived

Include at least one regression at the `PerAgentBeliefView` boundary and at least one end-to-end AI regression.

## Files to Touch

- `specs/E14-perception-beliefs.md` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify if the snapshot contract changes)
- `crates/worldwake-ai/src/agent_tick.rs` (modify if planner/runtime call sites need updated read semantics)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify if new explicit knowledge helpers are needed)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify if a production-oriented remote opportunity regression is added)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify if merchant remote-restock behavior is used as the end-to-end regression)

## Out of Scope

- Rumor/report propagation beyond existing E14/E15 scope
- A full redesign of every `BeliefView` method
- Action-handler or authoritative world validation changes unrelated to planner read semantics
- Reintroducing omniscient harness behavior
- Merchant-selling market-presence work from `S04`

## Acceptance Criteria

### Tests That Must Pass

1. Planner-side remote facility/resource discovery no longer comes from authoritative place scans through `PerAgentBeliefView`.
2. A regression proves an agent can still traverse public route topology without knowing remote opportunity-bearing entities by default.
3. A regression proves an agent does not pursue a remote harvest/trade/care opportunity until that opportunity is explicitly known.
4. A regression proves the same remote opportunity becomes usable once that knowledge is explicitly seeded or lawfully perceived.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo test --workspace`
7. Existing lint: `cargo clippy --workspace`

### Invariants

1. Belief-only planning does not discover remote opportunity-bearing entities through authoritative world scans.
2. Public route graph knowledge and discovered local infrastructure/resource knowledge are modeled as distinct concepts.
3. Execution-time validation remains authoritative even if planner-side discovery becomes belief-mediated.
4. No backwards-compatibility alias or hidden omniscient fallback is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — add regressions proving remote facility/resource discovery is unavailable without explicit knowledge while route-graph reads still work.
   Rationale: locks the read-model boundary directly where the leak currently lives.
2. `crates/worldwake-ai/src/candidate_generation.rs` or `crates/worldwake-ai/src/planning_snapshot.rs` tests — prove remote opportunity generation requires explicit knowledge of the remote entity/infrastructure, not just route connectivity.
   Rationale: covers the planner-side consequence of the boundary cleanup.
3. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — add or update an end-to-end remote-travel regression showing route knowledge alone is insufficient, and explicit seeded knowledge unlocks the journey.
   Rationale: preserves the real behavior contract at AI runtime level.
4. `crates/worldwake-ai/tests/golden_trade.rs` or `crates/worldwake-ai/tests/golden_production.rs` — add or update one nontrivial remote-opportunity scenario after the boundary change.
   Rationale: proves the cleanup survives outside the simplest decision-only case.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
