# S52EVIDAFT-004: Evidence perception and belief integration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new belief type, perception handler extension, investigation action extension
**Deps**: S52EVIDAFT-001, S52EVIDAFT-002, S52EVIDAFT-003

## Problem

Evidence exists on Place entities (from 001, emitted by 002) but agents cannot perceive it. Without perception integration, evidence is invisible to AI and cannot feed into the justice system's investigation pipeline.

## Assumption Reassessment (2026-04-05)

1. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs` already carries type-specific optional snapshots (`believed_activity`, `believed_artifact`, `believed_contention`). Adding `believed_evidence: Option<BelievedEvidenceState>` fits the current projection shape.
2. Passive perception iterates co-located entities via `world.entities_effectively_at(place)`, but that does not include the place entity itself. `SceneEvidence` therefore needs an explicit current-place projection path instead of relying on the generic co-located entity loop.
3. Investigation action at `crates/worldwake-systems/src/investigate_actions.rs` currently records `SocialObservation(WitnessedAbsence)` and `SuspectedTheft`, but it does not update the investigating agent's believed state for the place itself.
4. The live belief model does not store a per-entity confidence scalar. Confidence is derived from provenance plus staleness (`PerceptionSource` + `observed_tick` via `belief_confidence()`), so investigation-grade strengthening must be represented by a fresher direct-observation belief update rather than a new stored confidence field.
5. `InvestigateViolation` goal at `goal.rs:88-91` still targets a specific place-bound violation. This ticket remains about perception and belief enrichment only; investigation candidates are already emitted from violation memory.

## Architecture Check

1. Evidence perception follows the existing believed-state projection pattern, but place-bound evidence needs an explicit current-place observation path because place entities are not yielded by `entities_effectively_at(place)`.
2. Investigation action extension should reuse the same place snapshot projection boundary and refresh the believed place state at the current tick, rather than inventing a second evidence-to-belief conversion path.
3. No backward-compatibility shims.

## Verification Layers

1. Agent perceives evidence at co-located place → belief store assertion (believed_evidence populated)
2. Agent not co-located with evidence place → belief store absence assertion
3. Investigation action refreshes place evidence beliefs at a newer direct-observation tick → belief store comparison on `observed_tick` / derived freshness
4. Stale evidence belief after decay → belief retains old state until re-perceived
5. Cross-layer: perception (systems) reads SceneEvidence (core) → writes beliefs (core). Investigation (systems) reads the same authoritative evidence through the shared place snapshot boundary and refreshes the investigating agent's place belief at the current tick.

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

When the observer is at a place that either currently has `SceneEvidence` or is already remembered with `believed_evidence`:
1. Build a believed snapshot for the current place entity.
2. Populate `believed_entity_state.believed_evidence` from `SceneEvidence` entries (kind + freshness from created_at).
3. Record that place belief through the standard direct-observation path so evidence can both appear and clear on later re-perception.
4. Standard confidence remains derived from direct observation provenance plus staleness; no new stored confidence field is introduced.

### 3. Extend investigation action

In `crates/worldwake-systems/src/investigate_actions.rs`, within `commit_investigate` (or equivalent):

After existing investigation logic:
1. Check if the violation place has `SceneEvidence`.
2. If evidence exists, refresh the investigating agent's believed place state from the authoritative place snapshot at the current tick.
3. Evidence types matching the violation kind enrich the agent's place belief and social observations without changing candidate generation or accusation ownership in this ticket.

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
3. Investigation action at place with SceneEvidence refreshes the place evidence belief at a newer observation tick than an older passive perception snapshot
4. Decayed evidence (removed by decay system) no longer perceived on next perception pass
5. Multiple evidence entries on same place all perceived
6. Existing suite: `cargo test --workspace`

### Invariants

1. Evidence perception is co-location-based only (P7) — no global evidence queries
2. Perceived evidence state may be stale after decay (P14) — beliefs outlive evidence until re-perceived or retention expires
3. Investigation reads authoritative SceneEvidence through the same canonical place snapshot boundary — this is correct because investigation is action execution, not planning

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — BelievedEvidenceState construction tests
2. `crates/worldwake-systems/src/perception.rs` — Evidence perception: co-located perception, non-co-located absence, multiple entries
3. `crates/worldwake-systems/src/investigate_actions.rs` — Investigation refreshes place evidence belief at current tick

### Commands

1. `cargo test -p worldwake-systems -- perception`
2. `cargo test -p worldwake-systems -- investigate`
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-05
- Landed `BelievedEvidenceEntry` / `BelievedEvidenceState` and added `believed_evidence` to `BelievedEntityState` in `crates/worldwake-core/src/belief.rs`, with crate-root re-exports from `crates/worldwake-core/src/lib.rs`.
- Extended passive perception in `crates/worldwake-systems/src/perception.rs` to project `SceneEvidence` from the current place entity through an explicit current-place observation path, including clearing stale place evidence on later reobservation.
- Extended `commit_investigate` in `crates/worldwake-systems/src/investigate_actions.rs` to refresh the investigating agent's believed place state through the canonical place snapshot boundary at the current tick.
- Bumped `SAVE_FORMAT_VERSION` to `23` in `crates/worldwake-sim/src/save_load.rs` because the persisted belief shape changed.
- Deviation from original plan: investigation does not store a separate stronger confidence field. The live model derives confidence from provenance and staleness, so the implemented strengthening is a fresher direct-observation belief refresh instead.
- Verification:
  - `cargo test -p worldwake-core build_observed_entity_snapshot_projects_evidence_state_for_places -- --nocapture`
  - `cargo test -p worldwake-systems passive_perception_projects_scene_evidence_for_current_place -- --nocapture`
  - `cargo test -p worldwake-systems passive_perception_clears_stale_place_evidence_after_reobservation -- --nocapture`
  - `cargo test -p worldwake-systems investigate_action_refreshes_place_evidence_belief_at_commit_tick -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
