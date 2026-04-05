# S52EVIDAFT-004: Evidence perception and belief integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new belief type, perception handler extension, investigation action extension
**Deps**: S52EVIDAFT-001, S52EVIDAFT-002

## Problem

Evidence exists on Place entities (from 001, emitted by 002) but agents cannot perceive it. Without perception integration, evidence is invisible to AI and cannot feed into the justice system's investigation pipeline.

## Assumption Reassessment (2026-04-05)

1. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs:723-738` has 3 Optional type-specific fields: `believed_activity`, `believed_artifact` (S45), `believed_contention`. Pattern established for `believed_evidence: Option<BelievedEvidenceState>`.
2. Perception system at `crates/worldwake-systems/src/perception.rs` iterates co-located entities via `world.entities_effectively_at(place)`. SceneEvidence on Place entities will be encountered during this generic iteration.
3. Investigation action at `crates/worldwake-systems/src/investigate_actions.rs:113-222` currently records `SocialObservation(WitnessedAbsence)` and `SuspectedTheft`. Does NOT currently check `SceneEvidence`.
4. `InvestigateViolation` goal at `goal.rs:88-91` has `violation_id: ViolationId` and `place: EntityId`. Investigation action targets a specific violation at a place.
5. Evidence perception should be purely co-location-based (P7). No `discovered_by` tracking on evidence entries (P14 compliance — removed during reassessment).

## Architecture Check

1. Evidence perception follows the established pattern: perception system reads SceneEvidence from Place entities, populates BelievedEvidenceState on agent's BelievedEntityState for that place. Same pattern as BelievedArtifactState and BelievedContentionState.
2. Investigation action extension reads SceneEvidence with higher confidence than passive perception — this is additive to the existing investigation logic, not a replacement.
3. No backward-compatibility shims.

## Verification Layers

1. Agent perceives evidence at co-located place → belief store assertion (believed_evidence populated)
2. Agent not co-located with evidence place → belief store absence assertion
3. Investigation action reads SceneEvidence with higher confidence → belief store comparison (investigation beliefs > passive perception beliefs)
4. Stale evidence belief after decay → belief retains old state until re-perceived
5. Cross-layer: perception (systems) reads SceneEvidence (core) → writes beliefs (core). Investigation (systems) reads SceneEvidence (core) → writes beliefs with higher confidence.

## What to Change

### 1. Add BelievedEvidenceState

In `crates/worldwake-core/src/belief.rs`:

```rust
pub struct BelievedEvidenceState {
    pub entries: Vec<BelievedEvidenceEntry>,
    pub observed_tick: Tick,
}

pub struct BelievedEvidenceEntry {
    pub kind: EvidenceKind,
    pub freshness: Tick,
}
```

Add `pub believed_evidence: Option<BelievedEvidenceState>` to `BelievedEntityState`.

### 2. Add evidence perception handler

In `crates/worldwake-systems/src/perception.rs`, within the entity observation handler:

When a perceived Place entity has `SceneEvidence` component:
1. Read all evidence entries.
2. Construct `BelievedEvidenceState` with entries converted to `BelievedEvidenceEntry` (kind + freshness from created_at).
3. Set `believed_entity_state.believed_evidence = Some(believed_evidence_state)`.
4. Standard confidence from observation_fidelity applies.

### 3. Extend investigation action

In `crates/worldwake-systems/src/investigate_actions.rs`, within `commit_investigate` (or equivalent):

After existing investigation logic:
1. Check if the violation place has `SceneEvidence`.
2. If evidence exists, convert relevant entries to investigation-grade beliefs (higher confidence than passive perception).
3. Evidence types matching the violation kind strengthen the accusation: `ContainerTampered` strengthens theft accusations, `BloodTrail` strengthens assault accusations.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)

## Out of Scope

- Evidence emission — ticket 002
- Evidence decay — ticket 003
- Golden tests — ticket 005
- Candidate generation changes (investigation candidates already exist from S27; evidence enriches the investigation, doesn't change when candidates are emitted)
- Evidence matching to suspects (forensics — deferred per spec)

## Acceptance Criteria

### Tests That Must Pass

1. Agent co-located with place having SceneEvidence perceives it: `believed_evidence` populated
2. Agent not co-located with evidenced place: no `believed_evidence`
3. Investigation action at place with SceneEvidence produces higher-confidence beliefs than passive perception
4. Decayed evidence (removed by decay system) no longer perceived on next perception pass
5. Multiple evidence entries on same place all perceived
6. Existing suite: `cargo test --workspace`

### Invariants

1. Evidence perception is co-location-based only (P7) — no global evidence queries
2. Perceived evidence state may be stale after decay (P14) — beliefs outlive evidence until re-perceived or retention expires
3. Investigation reads authoritative SceneEvidence — this is correct because investigation is an action, not planning (P14 applies to planning, not to action execution reading world state)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — BelievedEvidenceState construction tests
2. `crates/worldwake-systems/src/perception.rs` — Evidence perception: co-located perception, non-co-located absence, multiple entries
3. `crates/worldwake-systems/src/investigate_actions.rs` — Investigation with SceneEvidence present vs absent

### Commands

1. `cargo test -p worldwake-systems -- perception`
2. `cargo test -p worldwake-systems -- investigate`
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
