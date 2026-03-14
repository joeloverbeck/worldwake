# E14PERBEL-009: Split Planner Belief Reads from Authoritative Execution Helpers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — AI/sim boundary redesign across `worldwake-sim` and `worldwake-ai`
**Deps**: E14PERBEL-006 (AI migrated off `OmniscientBeliefView`), E14PERBEL-007 (belief-isolation integration coverage in place), `specs/E14-perception-beliefs.md`, `specs/S02-goal-decision-policy-unification.md`, `specs/S03-planner-target-identity-and-affordance-binding.md`

## Problem

`BeliefView` currently mixes two different concerns:

1. subjective planning knowledge that should be constrained by `AgentBeliefStore`
2. authoritative helper queries that exist for execution, facility scheduling, reservation checks, and concrete world metadata

`PerAgentBeliefView` in `archive/tickets/completed/E14PERBEL-004.md` made this boundary honest, but it did not solve the underlying architectural problem. As long as the trait stays mixed, future AI work will keep accreting methods onto the wrong abstraction, and every new planning feature will have to decide ad hoc whether to:

- read beliefs,
- fall back to world truth,
- or expand the belief schema opportunistically.

That is not a clean long-term architecture. It weakens locality, makes belief isolation harder to reason about, and invites future omniscient leakage behind an otherwise belief-facing API.

## Assumption Reassessment (2026-03-14)

1. `BeliefView` in `crates/worldwake-sim/src/belief_view.rs` currently includes clearly subjective queries (`effective_place`, `is_alive`, `commodity_quantity` for others) alongside clearly authoritative helper queries (`reservation_ranges`, facility-queue accessors, `can_control`, `resource_source`, direct possession/container graph helpers) — confirmed.
2. `AgentBeliefStore` currently stores only `BelievedEntityState { last_known_place, last_known_inventory, alive, wounds, observed_tick, source }` plus `social_observations`; it does not model reservation state, facility queue state, control state, transit state, or metadata such as `ResourceSource` / `WorkstationTag` for others — confirmed.
3. `archive/tickets/completed/E14PERBEL-004.md` intentionally implemented `PerAgentBeliefView` as an interim adapter with explicit authoritative fallbacks because the current trait cannot be answered purely from stored beliefs — confirmed.
4. `tickets/E14PERBEL-006.md` is responsible for removing `OmniscientBeliefView`, but it explicitly does not redesign the trait boundary — confirmed.
5. `specs/S02-goal-decision-policy-unification.md` and `specs/S03-planner-target-identity-and-affordance-binding.md` both assume AI policy/planning reads should stay belief-facing and locality-preserving — confirmed.
6. `specs/S06-commodity-opportunity-valuation.md` plans to extend `BeliefView` further. Doing that before fixing the mixed boundary would compound the problem by adding more planning concerns onto an already impure interface — confirmed.
7. No active ticket currently owns the actual split or the alternative schema expansion required to make the planner boundary clean — confirmed.

## Architecture Check

1. The clean design is to separate planner-facing subjective knowledge from authoritative execution helpers. Planning code should depend on a trait whose semantics are unambiguously belief-local, while authoritative validation/execution code should keep using `World` or a separate authoritative helper surface.
2. This ticket must not introduce alias paths or compatibility wrappers that preserve the mixed interface indefinitely. If the boundary changes, callers must be updated to the new shape.
3. The redesign should choose one of two honest outcomes and implement it fully:
   - split the trait into a subjective planner interface plus a separate authoritative helper interface, or
   - expand the stored/readable subjective model until the planner-facing trait can truly be answered from beliefs without world fallbacks.
4. The first option is likely cleaner and more robust for current scope because many mixed methods are execution/runtime concerns, not agent-memory concerns. They do not belong in the planner belief interface at all.
5. This cleanup should happen before any future ticket extends planner reads further, especially before work like `S06` adds more `BeliefView` surface area.

## What to Change

### 1. Reassess and classify every `BeliefView` method

Audit each method in `crates/worldwake-sim/src/belief_view.rs` and classify it into exactly one bucket:

- subjective planner query
- self-authoritative planner query
- public-topology/public-structure query
- authoritative execution/helper query that does not belong in the planner belief interface

The classification must be written down in the ticket implementation or adjacent code comments/tests so the boundary is explicit and stable.

### 2. Replace the mixed interface with a clean boundary

Implement one clean architecture, not both:

- preferred: introduce a planner-facing subjective trait and migrate AI/planning/ranking/search/candidate-generation code to it; remove planner dependencies on the execution/helper methods that do not belong there
- acceptable alternative only if strongly justified by code reality: keep one planner trait but remove/migrate the mixed authoritative helper methods off it and update callers accordingly

Examples of methods that likely need to leave the planner belief surface:
- reservation queries
- facility queue/grant internals
- `can_control`
- direct possession/container graph helpers for unknown others
- raw resource-source metadata where the agent has no belief representation yet

### 3. Decide how the remaining planner needs are represented

For each current AI use of a mixed method, do one of:

- convert the planner to use an already-available subjective/public query instead
- move the check into authoritative execution/validation where it belongs
- or add the missing subjective model to beliefs if the planner genuinely needs that knowledge

Do not keep a method on the planner trait just because a current caller happens to use it.

### 4. Update specs/tickets that still assume the mixed interface is final

At minimum, correct active references that currently imply “no trait changes needed” or that propose extending the current mixed boundary without reassessment. This includes the active E14/S-series material that would otherwise keep building on the old abstraction.

### 5. Strengthen tests around the new boundary

Add coverage that proves:

- AI/planning code compiles against the new clean planner interface
- unknown entities do not leak through planner reads
- execution/runtime helpers no longer piggyback on the planner belief trait
- future additions to the planner interface require an explicit classification decision

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — redesign or replace mixed trait)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — conform to new boundary)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — consume new planner-facing boundary)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — remove mixed helper dependence as needed)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — consume new planner-facing boundary)
- `crates/worldwake-ai/src/planning_state.rs` (modify — conform to new boundary)
- `crates/worldwake-ai/src/ranking.rs` (modify — conform to new boundary)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — move authoritative checks out of planner surface where appropriate)
- `specs/E14-perception-beliefs.md` (modify if needed — correct the final boundary assumptions)
- `specs/S06-commodity-opportunity-valuation.md` (modify if needed — avoid extending the wrong interface)
- Any other active AI/sim file that still depends on the mixed planner/executor boundary

## Out of Scope

- Reintroducing `OmniscientBeliefView` or any alias/shim around it
- Implementing rumor/report propagation (E15 scope)
- Office/faction or crime-system work outside the planner boundary change
- Adding speculative new belief fields that no planner use actually requires
- Weakening belief isolation tests to make the current mixed interface look cleaner than it is

## Acceptance Criteria

### Tests That Must Pass

1. AI/planning code no longer depends on a mixed planner/executor belief trait
2. Unknown entities cannot be discovered through the planner-facing query surface
3. Methods that remain on the planner-facing trait are all intentionally subjective, self-authoritative, or public-topology/public-structure reads
4. Any authoritative execution/helper queries that remain needed are available through a separate non-planner path
5. Active specs/tickets no longer instruct future work to extend the old mixed boundary blindly
6. `cargo test -p worldwake-ai`
7. `cargo test -p worldwake-sim`
8. `cargo clippy --workspace`
9. `cargo test --workspace`

### Invariants

1. Planner reasoning reads beliefs/local public structure, not hidden authoritative world truth
2. Execution/runtime helper concerns are not smuggled back through planner-facing interfaces
3. No backwards-compatibility alias preserves the mixed interface after the redesign
4. Future planner-surface additions require explicit classification as subjective, self-authoritative, or public-structure data

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` or the new planner-boundary module — add tests that prove the planner-facing interface cannot discover unknown entities through fallback helpers
   Rationale: protects the core locality guarantee this cleanup exists to preserve.
2. `crates/worldwake-ai/src/*` planner-facing unit tests that currently stub `BeliefView` — update them to the new clean boundary and add at least one regression proving a formerly mixed helper is no longer available there
   Rationale: forces the AI layer to compile against the new abstraction instead of the old one by habit.
3. `crates/worldwake-ai/tests/belief_isolation.rs` or equivalent integration coverage — strengthen end-to-end assertions that planning behavior does not depend on hidden authoritative entity discovery
   Rationale: validates the architectural change at the behavior level, not just at the trait-definition level.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
