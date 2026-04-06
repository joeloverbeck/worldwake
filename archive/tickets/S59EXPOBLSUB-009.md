# S59EXPOBLSUB-009: search_place action

**Status**: COMPLETED
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
7. Mismatch + correction from `S59EXPOBLSUB-008`: there is no live stored `SearchTarget` carrier on this branch. `search_place` must not gate on “actor has a SearchTarget” or enumerate from that nonexistent substrate. The honest boundary is a direct missing-subject payload derived from overdue-expectation search context and/or planner-goal binding.
8. `SearchTarget` still exists only as an unused shared type in `crates/worldwake-core/src/expectation.rs` and re-export in `crates/worldwake-core/src/lib.rs`; no live runtime consumer remains. This ticket can absorb that small cleanup while landing the direct-payload search path.
9. The current branch has no generic search-result commit-trace carrier analogous to Tell. `SearchResult` can still be used internally to drive branch logic, but result-specific proof should stay on authoritative state plus action identity unless a later ticket explicitly widens the trace surface.

## Architecture Check

1. Follows the investigate action pattern — medium-duration epistemic action at a specific place. Reads existing SceneEvidence (S52 integration) rather than creating a new evidence subsystem.
2. No backward compatibility shims.

## Verification Layers

1. Target found alive at place → FoundAlive branch with correct condition → authoritative world state + action trace identity
2. Target found dead → FoundDead branch → authoritative world state + action trace identity
3. Target absent but evidence present → FoundEvidence branch → authoritative world state + action trace identity
4. Target absent and no evidence → NothingFound branch → authoritative world state + action trace identity
5. Expectation record updated on resolution → authoritative world state
6. LastSeenMemory updated with search result → authoritative world state

## What to Change

### 1. Create search_place action

Create `crates/worldwake-systems/src/search_actions.rs`:

- Domain: `ActionDomain::Epistemic`
- Preconditions: Actor at the place to search. Payload binds the missing subject directly rather than depending on a stored `SearchTarget` component.
- Duration: Medium (5-8 ticks, investigation action)
- on_commit:
  1. Check if target entity is at the place (co-located entities)
  2. If found: determine condition (alive+healthy, wounded, incapacitated, dead) → produce SearchResult
  3. If not found: read SceneEvidence for relevant traces (blood trails, movement traces matching target) → produce SearchResult with evidence or NothingFound
  4. Update actor's LastSeenMemory (if found: record sighting; if evidence handling is still lawful after reassessment, record the bounded result on the canonical carrier)
  5. Update actor's ExpectationRecord if applicable (resolve with outcome)
- Affordance targets: the place itself (self-targeted at current location)
- Affordance payloads: enumerate from the live missing-subject search carrier chosen during implementation, not from `SearchTarget`

### 2. Remove dead `SearchTarget` substrate

- Delete the unused `SearchTarget` type and its re-export/tests from `worldwake-core` so the shared search substrate matches the live direct-payload model.

### 3. Register action

In `crates/worldwake-systems/src/action_registry.rs`, add `register_search_place_action()` and update completeness test.

## Files to Touch

- `crates/worldwake-core/src/expectation.rs` (modify — remove dead `SearchTarget`)
- `crates/worldwake-core/src/lib.rs` (modify — remove re-export)
- `crates/worldwake-sim/src/action_payload.rs` (modify — add direct search payload)
- `crates/worldwake-sim/src/action_trace.rs` (modify — add search action identity detail)
- `crates/worldwake-sim/src/lib.rs` (modify — export new payload type)
- `crates/worldwake-systems/src/search_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register + test)

## Out of Scope

- Route-level multi-place search sequencing — the action handles place-level search only; broader route search emerges from sequential place searches via travel
- escort_to_safety triggered by finding a wounded person — separate ticket 010
- Candidate generation — ticket 011

## Acceptance Criteria

### Tests That Must Pass

1. Target entity present and healthy → FoundAlive branch with `Healthy` condition
2. Target entity present and wounded/incapacitated → FoundAlive branch with wounded/unconscious condition
3. Target entity dead at place → FoundDead branch
4. Target absent, subject-specific evidence present → FoundEvidence branch
5. Target absent, no relevant evidence → NothingFound branch
6. Actor's LastSeenMemory updated after successful find
7. Actor's ExpectationRecord resolved on find
8. Action rejected when actor not at the search place
9. Action registry completeness test includes "search_place"
10. `SearchTarget` no longer exists as a shared dead type on the current branch
11. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Search checks authoritative entity presence at the place (not belief state) — the actor is physically present and observing
2. SceneEvidence is read, not modified (read-only access to S52 state)
3. The action uses the canonical live missing-subject carrier chosen during implementation rather than introducing a parallel `SearchTarget` path
4. Result-specific branch proof does not invent a new generic search trace carrier on this ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/search_actions.rs` — unit tests for all SearchResult branches
2. `crates/worldwake-systems/src/action_registry.rs` — updated completeness test
3. `crates/worldwake-core/src/expectation.rs` — shared-type roundtrip/bounds coverage after `SearchTarget` removal
4. `crates/worldwake-sim/src/action_payload.rs` / `action_trace.rs` — new payload/detail roundtrip coverage

### Commands

1. `cargo test -p worldwake-systems search`
2. `cargo test -p worldwake-core expectation`
3. `cargo test -p worldwake-sim action_payload`
4. `cargo test -p worldwake-sim action_trace`
5. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- Completed on 2026-04-06.
- Corrected the ticket before implementation: there was no live stored `SearchTarget` carrier and no lawful generic search-result commit trace on this branch, so the landed action uses direct subject payloads from overdue expectations and keeps result proof on authoritative state plus action identity.
- Removed the dead `SearchTarget` shared substrate from `crates/worldwake-core/src/expectation.rs` and the corresponding re-export from `crates/worldwake-core/src/lib.rs`.
- Added `SearchPlaceActionPayload` and `search_place` action-trace identity support in `crates/worldwake-sim/src/action_payload.rs`, `crates/worldwake-sim/src/action_trace.rs`, and `crates/worldwake-sim/src/lib.rs`.
- Added `search_place` in `crates/worldwake-systems/src/search_actions.rs`, registered it through the systems catalog, and updated completeness coverage in `crates/worldwake-systems/src/action_registry.rs`.
- The landed action enumerates overdue subjects at the actor's current place, uses `DurationExpr::ActorInvestigationDisposition`, resolves expectations only when the subject is actually found, records direct-observation `LastSeenMemory` on successful finds, and leaves overdue expectations untouched on evidence-only or empty search results.

## Verification Result

- Passed `cargo test -p worldwake-core expectation`
- Passed `cargo test -p worldwake-sim action_payload`
- Passed `cargo test -p worldwake-sim action_trace`
- Passed `cargo test -p worldwake-systems search_actions`
- Passed `cargo test -p worldwake-systems action_registry`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
