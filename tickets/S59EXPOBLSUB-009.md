# S59EXPOBLSUB-009: search_place action

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new action in worldwake-systems
**Deps**: S59EXPOBLSUB-002, S59EXPOBLSUB-005

## Problem

Agents need to physically search a place for a missing person. The `search_place` action checks for the target entity's presence, reads `SceneEvidence` for relevant traces, and produces a `SearchResult` that drives downstream report and escort actions.

## Assumption Reassessment (2026-04-06)

1. `SceneEvidence` component at `crates/worldwake-core/src/evidence.rs:57-60` with `evidence: Vec<EvidenceEntry>` containing `EvidenceKind` entries.
2. `EvidenceKind` variants at `evidence.rs:23-43`: ContainerTampered, BloodTrail, DisturbanceMarker, MovementTrace. `search_place` reads these for relevant traces.
3. `ActionDomain::Epistemic` at `crates/worldwake-core/src/action_domain.rs:10`.
4. Existing `investigate` action at `crates/worldwake-systems/src/investigate_actions.rs` provides the closest pattern — medium-duration epistemic action that reads scene state.
5. `SearchResult` and `SearchCondition` types from ticket 001 are the action's output types.
6. World API provides `component_*` for checking entity presence at a place and reading wound/incapacitation state.

## Architecture Check

1. Follows the investigate action pattern — medium-duration epistemic action at a specific place. Reads existing SceneEvidence (S52 integration) rather than creating a new evidence subsystem.
2. No backward compatibility shims.

## Verification Layers

1. Target found alive at place → SearchResult::FoundAlive with correct condition → action trace + authoritative world state
2. Target found dead → SearchResult::FoundDead → action trace
3. Target absent but evidence present → SearchResult::FoundEvidence → action trace
4. Target absent and no evidence → SearchResult::NothingFound → action trace
5. Expectation record updated on resolution → authoritative world state
6. LastSeenMemory updated with search result → authoritative world state

## What to Change

### 1. Create search_place action

Create `crates/worldwake-systems/src/search_actions.rs`:

- Domain: `ActionDomain::Epistemic`
- Preconditions: Actor at the place to search. Actor has a SearchTarget.
- Duration: Medium (5-8 ticks, investigation action)
- on_commit:
  1. Check if target entity is at the place (co-located entities)
  2. If found: determine condition (alive+healthy, wounded, incapacitated, dead) → produce SearchResult
  3. If not found: read SceneEvidence for relevant traces (blood trails, movement traces matching target) → produce SearchResult with evidence or NothingFound
  4. Update actor's LastSeenMemory (if found: record sighting; if evidence: record partial info)
  5. Update actor's ExpectationRecord if applicable (resolve with outcome)
- Affordance targets: the place itself (self-targeted at current location)
- Affordance payloads: enumerate from search targets with place candidates

### 2. Register action

In `crates/worldwake-systems/src/action_registry.rs`, add `register_search_place_action()` and update completeness test.

## Files to Touch

- `crates/worldwake-systems/src/search_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register + test)

## Out of Scope

- Route search (SearchTarget::RouteSearch) — the action handles place-level search only; route search emerges from sequential place searches via travel
- escort_to_safety triggered by finding a wounded person — separate ticket 010
- Candidate generation — ticket 011

## Acceptance Criteria

### Tests That Must Pass

1. Target entity present and healthy → FoundAlive { condition: Healthy }
2. Target entity present and wounded → FoundAlive { condition: Wounded }
3. Target entity dead at place → FoundDead
4. Target absent, BloodTrail evidence present → FoundEvidence
5. Target absent, no evidence → NothingFound
6. Actor's LastSeenMemory updated after successful find
7. Actor's ExpectationRecord resolved on find
8. Action rejected when actor not at the search place
9. Action registry completeness test includes "search_place"
10. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Search checks authoritative entity presence at the place (not belief state) — the actor is physically present and observing
2. SceneEvidence is read, not modified (read-only access to S52 state)
3. LastSeenMemory respects capacity bounds

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/search_actions.rs` — unit tests for all SearchResult branches
2. `crates/worldwake-systems/src/action_registry.rs` — updated completeness test

### Commands

1. `cargo test -p worldwake-systems search`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
