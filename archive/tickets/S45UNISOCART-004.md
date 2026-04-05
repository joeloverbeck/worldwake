# S45UNISOCART-004: Artifact perception integration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — shared belief projection extended with artifact snapshots, plus perception-side notice internalization and route-threat notice consumption
**Deps**: S45UNISOCART-001

## Problem

Social artifact entities exist in the world (types from 001, created by actions in 002/003), but the shared belief projection path still drops artifact-specific content. Passive perception already iterates co-located entities generically, yet `build_believed_entity_state()` / `ObservedEntitySnapshot` only project generic inventory/life/contention state. That means artifact facts are lost before they reach `AgentBeliefStore`. Separately, notice topics currently have no lawful downstream effect except as inert metadata, so a discovered `ThreatWarning` would still not affect behavior.

## Assumption Reassessment (2026-04-04)

1. Perception system at `crates/worldwake-systems/src/perception.rs` already uses `world.entities_effectively_at(place)` to iterate all co-located entities generically. No new entity-kind-specific iteration loop is needed.
2. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs` already has `believed_artifact: Option<BelievedArtifactState>`, but the canonical builder path still leaves it `None`.
3. The canonical subjective projection boundary is shared: `build_observed_entity_snapshot()` and `build_believed_entity_state()` in `crates/worldwake-core/src/belief.rs`, plus `ObservedEntitySnapshot::to_believed_entity_state()`. Patching only `perception.rs` would leave witnessed-event snapshots and any other builder callers stale.
4. `AgentBeliefStore` stores known entities plus institutional beliefs. There is already a lawful belief lane for institutional notice topics (`InstitutionalClaim`, office vacancy via `OfficeHolder { holder: None }`), but there is no general stored belief substrate for threat-warning or shortage facts.
5. `worldwake-ai/src/route_threat.rs` already derives travel risk from remembered local evidence. The smallest lawful way to make `ThreatWarning` matter now is to consume believed active notice artifacts there instead of inventing a new parallel place-belief model.
6. `PerceptionProfile.observation_fidelity` in `crates/worldwake-core/src/belief.rs` still gates artifact perception through the standard observation checks.

## Architecture Check

1. Artifact perception reuses the existing generic entity perception pipeline, but the owned implementation boundary is the shared belief builder in `worldwake-core`, not a perception-local handler only.
2. Following the existing pattern: just as `BelievedContentionState` is populated when an entity has `ContentionQueue`, `BelievedArtifactState` must be populated when an entity has `ArtifactHeader`.
3. Notice internalization is split by live substrate:
   - `NoticeTopic::Institutional { claim }` becomes a `BelievedInstitutionalClaim`
   - `NoticeTopic::OfficeVacancy { office }` becomes `InstitutionalClaim::OfficeHolder { office, holder: None, ... }`
   - `NoticeTopic::ThreatWarning { place }` remains on the artifact belief and is consumed directly by route-threat estimation
   - `NoticeTopic::CommodityShortage { .. }` remains on the artifact belief for now; no new economic belief lane is invented in this ticket
4. No backward-compatibility shims.

## Verification Layers

1. Agent perceives bounty → shared builder projects `believed_artifact` → authoritative belief store check
2. Agent perceives institutional or office-vacancy notice → `believed_artifact` populated and institutional belief lane updated
3. Agent perceives threat-warning notice → `believed_artifact` populated and route-threat estimation increases for the warned place
4. Stale bounty belief (bounty expired after perception) → belief retains old state until re-perceived → belief store vs world state comparison
5. Agent not co-located with artifact → no belief about artifact → belief store absence check
6. Mixed-layer ticket: shared builder (core), perception/internalization (systems), and route-threat read-model (AI)

## What to Change

### 1. Add artifact projection to the shared belief builder

In `crates/worldwake-core/src/belief.rs`:
1. Extend `ObservedEntitySnapshot` so artifact facts can survive the shared snapshot carrier.
2. When an observed entity has `ArtifactHeader`, construct `BelievedArtifactState`.
3. If `kind == ArtifactKind::Bounty`, read `BountyTerms` and construct `BelievedBountyTerms` (target, reward_commodity, reward_quantity, claim_place).
4. If `kind == ArtifactKind::Notice`, read `NoticeContent` and store `notice_topic`.
5. Set `BelievedArtifactState.observed_tick` from the current perception tick in `build_believed_entity_state()`.

### 2. Internalize notice content where a live belief lane exists

In `crates/worldwake-systems/src/perception.rs`, after recording an observed snapshot:
- `NoticeTopic::Institutional { claim }`: add a `BelievedInstitutionalClaim`
- `NoticeTopic::OfficeVacancy { office }`: add `InstitutionalClaim::OfficeHolder { office, holder: None, effective_tick: current_tick }`

Do not invent a new generic place/economy belief substrate in this ticket.

### 3. Make threat warnings affect behavior through the existing route-threat model

In `crates/worldwake-ai/src/route_threat.rs`:
- treat believed active `ThreatWarning { place }` notice artifacts as remembered local threat evidence for that place
- derive confidence from the parent `BelievedEntityState` provenance and staleness just like other remembered evidence

### 4. Handle source and staleness

`BelievedArtifactState.observed_tick` tracks when the artifact was perceived. The parent `BelievedEntityState.source` remains the provenance marker. Standard confidence and staleness policies apply.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-ai/src/route_threat.rs` (modify)

## Out of Scope

- AI bounty candidate generation from perceived artifacts — ticket 005
- Golden tests — ticket 006
- Tell-based artifact knowledge sharing
- New generic place-belief or commodity-shortage-belief substrate
- Artifact-specific perception fidelity (uses standard observation_fidelity)

## Acceptance Criteria

### Tests That Must Pass

1. Agent co-located with bounty artifact perceives it: `believed_artifact` populated with correct kind, state, terms
2. Agent co-located with notice artifact perceives it: `believed_artifact` populated with correct topic
3. Institutional notice updates institutional beliefs
4. Office-vacancy notice updates office-holder institutional belief to `holder: None`
5. ThreatWarning notice increases route-threat / perceived travel cost for the warned place
6. Agent not co-located with artifact: no `believed_artifact` on any BelievedEntityState
7. Expired bounty perceived correctly: `believed_artifact.state == Expired`
8. Existing suite: `cargo test --workspace`

### Invariants

1. Agents only perceive artifacts at their current location (Principle 7 — locality)
2. Perceived artifact state may be stale (Principle 14 — world state != belief state)
3. Perception does not modify artifact entities — read-only from world, write-only to beliefs
4. ThreatWarning affects behavior through remembered evidence, not a global hazard oracle

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — Unit tests for artifact snapshot projection
2. `crates/worldwake-systems/src/perception.rs` — Tests for notice internalization into institutional beliefs
3. `crates/worldwake-ai/src/route_threat.rs` — Tests for threat-warning-driven route threat

### Commands

1. `cargo test -p worldwake-core belief`
2. `cargo test -p worldwake-systems perception`
3. `cargo test -p worldwake-ai route_threat`
4. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Extended the canonical belief carrier in `crates/worldwake-core/src/belief.rs` so `ObservedEntitySnapshot` and `build_believed_entity_state()` now project `BelievedArtifactState` from authoritative `ArtifactHeader`, `BountyTerms`, and `NoticeContent` instead of dropping artifact facts before they reach `AgentBeliefStore`.

Updated `crates/worldwake-systems/src/perception.rs` so direct observation now internalizes notice content where a live belief lane already exists:
- `NoticeTopic::Institutional { claim }` becomes a `BelievedInstitutionalClaim`
- `NoticeTopic::OfficeVacancy { office }` becomes `InstitutionalClaim::OfficeHolder { holder: None }`

Made `NoticeTopic::ThreatWarning { place }` behaviorally meaningful in `crates/worldwake-ai/src/route_threat.rs` by treating believed active warning notices as remembered local threat evidence for the warned place. This preserves the spec's behavioral promise without inventing a new parallel place-belief substrate in this ticket.

Updated serialized carrier fallout in `crates/worldwake-core/src/event_record.rs` and bumped `SAVE_FORMAT_VERSION` to `19` in `crates/worldwake-sim/src/save_load.rs` because the shared snapshot/event-belief shape changed.

Verification completed:
- `cargo test -p worldwake-core belief`
- `cargo test -p worldwake-systems perception`
- `cargo test -p worldwake-ai route_threat`
- `cargo test -p worldwake-sim save_load`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
