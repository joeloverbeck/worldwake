# E14PERBEL-007: Belief Isolation Integration Coverage

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No production behavior change expected; test coverage only
**Deps**: E14PERBEL-005 (perception system running), E14PERBEL-006 (migration complete, `OmniscientBeliefView` deleted)

## Problem

Phase 3 still needs a hard proof for T10 at the AI/runtime boundary, not just unit coverage inside the perception and belief-view modules. The missing gap is end-to-end coverage that an agent's planning inputs stay belief-mediated across the actual perception -> belief store -> `PerAgentBeliefView` -> candidate-generation/runtime path.

## Assumption Reassessment (2026-03-14)

1. Phase 3 gate still explicitly requires `T10: Belief isolation — agent does not react to unseen theft, death, or camp migration` in [specs/IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) — confirmed.
2. The original ticket overstated how much net-new coverage E14 still lacks. Several scenarios are already covered by focused unit tests:
   - same-place witness belief updates
   - participants-only witness filtering
   - adjacent-place spillover
   - `memory_capacity` eviction
   - witnessed cooperation capture
   - hidden-entity filtering in `PerAgentBeliefView`
   - stale belief preservation in `PerAgentBeliefView`
   - compile-time planner-boundary check that AI belief-read modules do not depend on `World`
3. The original ticket's `unseen theft` scenario is not an honest E14 test target. Theft/crime semantics belong to later work (`E17`, plus unseen-crime discovery gate `T25`). E14 can and should prove generic unseen state-change isolation without inventing crime-specific behavior.
4. The original `camp migration` wording is also too domain-specific for current code. The architecture currently supports generic unseen relocation and stale location beliefs; tests should target that directly instead of introducing camp-specific fixtures.
5. `archive/tickets/completed/E14PERBEL-004.md` remains decisive: `PerAgentBeliefView` is an honest interim adapter, not the final ideal planner boundary. This ticket must validate belief isolation without asserting stronger subjectivity guarantees than the current architecture actually provides.
6. `archive/tickets/E14PERBEL-006.md` already removed `OmniscientBeliefView` from non-archived code. This ticket only needs a narrow regression check, not a second migration.
7. `tickets/E14PERBEL-011.md` documents the passive-local-observation gap: nearby static state is not learned automatically unless an event drives perception. These integration tests must not assume passive omniscient discovery of co-located agents/items.

## Architecture Check

1. The strongest missing value is AI/runtime integration coverage, not more duplicate unit tests in separate files.
2. Reusing existing crate-local test modules is cleaner than introducing broad new integration harnesses that restate already-covered behavior.
3. The right architectural standard is:
   - direct perception can create new subjective knowledge
   - unknown entities remain hidden until perceived
   - stale beliefs remain usable until refreshed
   - planner/runtime reads continue to flow through `PerAgentBeliefView`
4. The long-term ideal architecture is still the `E14PERBEL-009` trait-boundary split plus the `E14PERBEL-011` passive-local-observation follow-up. This ticket should not paper over those follow-ups by asserting capabilities the code intentionally does not have yet.

## What to Change

### 1. Add AI integration tests for the missing T10 coverage

Extend the existing `crates/worldwake-ai/src/agent_tick.rs` test module with end-to-end tests that exercise the actual runtime read phase:

- **Unknown seller stays hidden until directly perceived**
  - A hungry agent with no food is co-located with a seller but has no prior belief entry.
  - Before any observed event, candidate generation/runtime ranking must not discover that seller.
  - After a same-place observed event involving that seller, the agent's belief store must gain a `DirectObservation` snapshot and the AI read phase must now surface the acquisition path.

- **Unseen relocation preserves stale planning knowledge**
  - After the direct observation above, relocate the seller away without a fresh observation for the actor.
  - Verify the actor still believes the seller is at the old place and the AI read phase still plans from that stale belief rather than silently refreshing from authoritative state.

- **Unseen death does not create corpse/death reactions**
  - After an actor has an old alive belief about another agent, move that other agent away and kill them off-screen.
  - Verify the actor's belief view still treats that agent as not-dead and the AI read phase does not emit corpse/death-driven reactions for that entity.

These scenarios satisfy the intent of T10 without introducing theft/crime/camp-specific test scaffolding that belongs outside E14.

### 2. Keep existing perception/belief-view unit coverage as the lower-level proof

Do not duplicate the already-covered scenarios into new files. Rely on the existing tests in:

- [crates/worldwake-systems/src/perception.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs)
- [crates/worldwake-sim/src/per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs)
- [crates/worldwake-ai/src/agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs)

### 3. Keep the no-omniscience regression check focused

Retain verification that `OmniscientBeliefView` remains absent from non-archived production code, but treat this as a regression assertion around E14 state rather than the main substance of the ticket.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add belief-isolation runtime integration tests)
- `tickets/E14PERBEL-007.md` (modify — corrected scope and assumptions)

## Out of Scope

- New dedicated integration test files under `crates/worldwake-systems/tests/` or `crates/worldwake-ai/tests/` for scenarios already covered by unit tests
- Re-testing same-place witness updates, adjacent spillover, or memory-capacity eviction in duplicate harnesses
- Theft/crime-specific behavior (`E17` / `T25`)
- Rumor/report propagation (`E15`)
- Office/faction behavior (`E16`)
- Passive local-state synchronization without events (`E14PERBEL-011`)
- Redesigning the mixed subjective/authoritative `BeliefView` boundary (`E14PERBEL-009`)
- Any production-code refactor unrelated to making the missing integration proof testable

## Acceptance Criteria

### Tests That Must Pass

1. Existing lower-level perception tests remain green
2. Existing `PerAgentBeliefView` boundary tests remain green
3. New AI/runtime integration tests prove:
   - unknown co-located entities do not become discoverable before perception
   - direct perception can create subjective planning knowledge
   - unseen relocation leaves stale location knowledge in place
   - unseen death does not create corpse/death reactions without re-observation
4. `cargo test -p worldwake-ai`
5. `cargo test -p worldwake-sim`
6. `cargo test -p worldwake-systems`
7. `cargo clippy --workspace`
8. `cargo test --workspace`
9. Grep for `OmniscientBeliefView` in non-archived code stays clean

### Invariants

1. T10 is validated through generic unseen state-change scenarios that the current E14 architecture actually models.
2. Unknown entities do not become discoverable through planner/runtime reads until perception seeds a belief entry.
3. Stale beliefs persist until refreshed; the runtime path does not silently pull authoritative non-self updates from `World`.
4. Integration coverage must stay honest about the current interim architecture:
   - subjective planning reads are belief-mediated
   - self/topology/public-structure/runtime helpers may still be authoritative where documented
   - tests must not pretend the `E14PERBEL-009` split already exists
5. No backwards-compatibility alias or omniscient shim is reintroduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs`
   - add T10-focused runtime integration tests for unseen relocation/death and perception-seeded discovery
2. No new duplicate integration files unless the existing crate-local test modules prove insufficient

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - corrected the ticket scope to match the real E14 architecture and current coverage
  - added three runtime-level tests in `crates/worldwake-ai/src/agent_tick.rs` proving:
    - unknown co-located sellers stay hidden until an observed event seeds belief
    - a same-place perceived event propagates into `AgentBeliefStore` and then into runtime candidate generation
    - unseen relocation and unseen death leave stale non-self beliefs in place instead of silently refreshing from authoritative state
- Deviations from original plan:
  - did not add new dedicated integration test files because the original ticket duplicated lower-level scenarios already covered in `perception.rs` and `per_agent_belief_view.rs`
  - removed the original theft/camp-specific test expectations because those assumptions were not honest for current E14 scope; the finished coverage uses generic unseen relocation/death cases that the current architecture actually models
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
