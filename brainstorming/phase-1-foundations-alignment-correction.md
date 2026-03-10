# FND-01: Phase 1 Foundations Alignment Corrections

**Status**: PROPOSED
**Priority**: BLOCKER
**Scope**: Amend E02, E04, E06, E07, and add Phase 2 entry gates

## Goal
Eliminate the remaining architectural loopholes that allow non-local knowledge and abstract route scores to masquerade as emergence.

## Non-Goals
- Do not rework deterministic IDs, world transaction journaling, event log structure, replay/save-load, or agent symmetry.
- Do not add backward-compatibility shims or alias paths.
- Do not replace explicit physical/structural properties with faux-emergent complexity.

## Required Changes

### A. E07 — Split agent knowledge from authoritative world reads
Replace the current single `KnowledgeView` concept with two distinct interfaces:

- `AuthorityView`
  - authoritative world queries used by legality checks, scheduler logic, and world systems
- `BeliefView`
  - agent-scoped epistemic view used by affordance query, AI planning, and human-facing action UI

#### Rules
- No agent-facing API may query arbitrary world state without an explicit `observer: EntityId`.
- `get_affordances(...)` MUST operate on `BeliefView`.
- `start_action(...)` and `commit_action(...)` MUST revalidate all hard physical preconditions against authoritative state.
- False beliefs MAY cause an attempted action to fail cleanly.
- Such failures MUST emit auditable failure / abort / replan records.
- `WorldKnowledgeView` MUST NOT be used on behalf of an agent.
- Human-controlled and AI-controlled agents MUST use the same `BeliefView`-based affordance pipeline.

### B. E05/E06/E07 — Complete the information pipeline
Add an explicit perception and reporting path:

- events resolve direct witnesses from local physical conditions only
- witnesses gain facts/beliefs through event observation
- non-witness agents learn only through:
  - co-located report exchange
  - documents or public records physically consulted
  - rumor/report carriers that travel the graph

#### Rules
- `PublicRecord` means “a record exists at a place/entity and can be consulted,” not “this becomes globally known.”
- `AdjacentPlaces` MUST be limited to immediate physical spillover only and MUST NOT be used as free multi-hop information spread.
- Any information propagation beyond direct perception MUST consume time per hop.
- Agent beliefs MUST be traceable to witness, report, record consultation, or prior belief state.

### C. E02 — Remove abstract route condition scores
`TravelEdge` MUST contain structural properties only.

#### Remove
- `danger`
- `visibility`

#### Allowed edge data
- endpoints
- travel time
- capacity
- stable structural tags / route features

#### Rules
- Derived route risk estimates MAY exist only as transient read-models computed from concrete state or agent belief.
- Derived route visibility estimates MAY exist only as transient read-models computed from concrete physical conditions.
- No route risk/visibility metric may be stored as authoritative world state.
- No system may branch on a stored authoritative `route_danger` or `route_visibility` field.

### D. E02/E05 — Add a concrete route presence model
Before route danger, ambush, patrol, interception, or witness-along-route systems are implemented, the world MUST support concrete presence on routes via one of:

1. intermediate route places / segment nodes
2. authoritative edge-occupancy / in-transit state with deterministic queries

#### Required capabilities
- determine which entities are on a route or route segment
- determine which travelers can physically encounter each other
- determine which agents can witness route events locally
- determine which route features affect ambush/visibility opportunities

#### Additional rule
- If edge occupancy is chosen, transit state MUST identify the occupied edge/segment and progress.
- `effective_place = None` by itself is NOT sufficient for encounter or witness logic.
- It is forbidden to reintroduce route danger as a stored score to compensate for missing route presence.

### E. E07 — Ban zero-tick world actions
All committed world actions MUST consume at least one tick.

#### Required change
- `DurationExpr::Fixed(NonZeroU32)` or equivalent registration-time validation

#### Rule
- Controller/UI operations that are not world actions MUST remain outside the action pipeline.

### F. E04 — Replace hidden load tables with explicit physical profiles
Keep `LoadUnits`, but move per-commodity and per-item load values out of free helper match tables and into named physical profile data.

#### Introduce
- `CommodityPhysicalProfile`
- `UniqueItemPhysicalProfile`

#### Rules
- these values are authoritative physical properties, not balancing knobs
- there are no hidden fallback/default values
- capacity logic may read them
- unrelated systems MUST NOT treat them as generic tuning multipliers
- future refinement into richer bulk/mass/fit modeling is allowed, but not required for this correction

### G. E05 / future social systems — Constrain scalar dispositions
`LoyalTo.strength` MAY remain, but only under these rules:

- initial values MUST come from seeded traits, background, or bootstrap events
- all changes MUST be event-sourced
- no direct script threshold of the form “if loyalty < X then betray”
- decisions involving loyalty MUST still flow through beliefs, goals, and utility

### H. Future system-spec rule
Every future system spec (E09+) MUST include:

- information-path analysis
- positive-feedback analysis
- concrete dampeners
- explicit list of stored state vs derived read-models

## Tests

- [ ] No agent-facing query API can return location, inventory, or liveness for arbitrary entities without an `observer`
- [ ] Two agents in the same world but with different belief states can receive different affordance sets
- [ ] A false belief can produce an attempted action that fails at start/commit and emits a failure/replan event
- [ ] `TravelEdge` has no authoritative `danger` or `visibility` fields
- [ ] Route encounter / ambush tests derive from concrete route presence, not stored route scores
- [ ] `PublicRecord` facts are learned only by consultation or report propagation
- [ ] Any information propagation beyond direct perception consumes time per hop
- [ ] No world action definition with zero duration can be registered
- [ ] Every commodity and unique item kind has an explicit physical profile
- [ ] Loyalty changes are traceable to events

## Phase Gates

- E13 (AI decision architecture) MUST NOT proceed until sections A and B are complete.
- E10/E11 (trade-route risk, crime, combat-on-route, patrol/interception) MUST NOT proceed until sections C and D are complete.
- Any Phase 2 action expansion MUST satisfy section E before new action definitions are added.

## Acceptance Criteria
Phase 1 is considered foundation-aligned when:

- no agent planning/UI path depends on omniscient world reads
- no authoritative route score stands in for concrete route state
- all world actions cost time
- physical carry/load properties are explicit data, not hidden logic
- future systems are forced to declare locality and dampeners up front