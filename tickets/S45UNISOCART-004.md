# S45UNISOCART-004: Artifact perception integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — perception system extended with artifact-specific belief creation
**Deps**: S45UNISOCART-001

## Problem

Social artifact entities exist in the world (types from 001, created by actions in 002/003) but agents cannot perceive them. The perception system generically iterates co-located entities but does not extract artifact-specific content (bounty terms, notice topics) into agent beliefs. Without perception, agents cannot plan around artifacts.

## Assumption Reassessment (2026-04-04)

1. Perception system at `crates/worldwake-systems/src/perception.rs:194-262` uses `world.entities_effectively_at(place)` to iterate all co-located entities generically. No entity-kind-specific iteration needed — SocialArtifact entities are already included.
2. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs:700-713` has Optional type-specific fields. `believed_artifact: Option<BelievedArtifactState>` added by ticket 001.
3. Perception handler pattern: `observe_passive_local_entities()` creates/updates `BelievedEntityState` for each perceived entity. The handler reads components from the entity and populates belief fields.
4. `AgentBeliefStore` at `crates/worldwake-core/src/belief.rs` stores `BTreeMap<EntityId, BelievedEntityState>` for known entities. Artifact entities are keyed by their EntityId like any other entity.
5. For notices with `NoticeTopic::ThreatWarning` or `NoticeTopic::Institutional`, content should also be internalized as an institutional belief or entity belief depending on topic — this follows the pattern in `crates/worldwake-core/src/institutional.rs` for `BelievedInstitutionalClaim`.
6. `PerceptionProfile.observation_fidelity` at `crates/worldwake-core/src/perception_types.rs` affects whether artifacts are perceived. Standard fidelity checks apply.

## Architecture Check

1. Artifact perception reuses the existing generic entity perception pipeline — no new entity-kind-specific iteration loop. The only addition is an artifact-specific content extraction handler that runs when a perceived entity has `ArtifactHeader`.
2. Following the existing pattern: just as `BelievedContentionState` is populated when an entity has `ContentionQueue`, `BelievedArtifactState` is populated when an entity has `ArtifactHeader`.
3. No backward-compatibility shims.

## Verification Layers

1. Agent perceives bounty → believed_artifact populated on BelievedEntityState → authoritative belief store check
2. Agent perceives notice → believed_artifact populated + notice content internalized → belief store check
3. Stale bounty belief (bounty expired after perception) → belief retains old state until re-perceived → belief store vs world state comparison
4. Agent not co-located with artifact → no belief about artifact → belief store absence check
5. Single-layer ticket (perception system only) — cross-system verification deferred to golden tests (006).

## What to Change

### 1. Add artifact content extraction to perception handler

In `crates/worldwake-systems/src/perception.rs`, within the entity observation handler:

When a perceived entity has `ArtifactHeader` component:
1. Read `ArtifactHeader` fields (kind, state, issuer, expires_at).
2. If `kind == ArtifactKind::Bounty`, read `BountyTerms` and construct `BelievedBountyTerms` (target, reward_commodity, reward_quantity, claim_place).
3. If `kind == ArtifactKind::Notice`, read `NoticeContent` and store `notice_topic`.
4. Construct `BelievedArtifactState` with all extracted data + `observed_tick`.
5. Set `believed_entity_state.believed_artifact = Some(believed_artifact_state)`.

### 2. Internalize notice content as beliefs

For notices, additionally process the topic:
- `ThreatWarning { place }`: Update believed danger/route caution for the referenced place.
- `OfficeVacancy { office }`: Update believed office state (vacant).
- `CommodityShortage { commodity, place }`: Update believed inventory at place (low stock).
- `Institutional { claim }`: Add to institutional beliefs following existing `BelievedInstitutionalClaim` pattern.

### 3. Handle perception source and staleness

Set `BelievedArtifactState.observed_tick` to current tick. The parent `BelievedEntityState.source` already tracks perception source (DirectObservation vs Report vs Rumor). Standard confidence and staleness policies apply — no artifact-specific confidence logic needed.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)

## Out of Scope

- AI candidate generation from perceived artifacts — ticket 005
- Golden tests — ticket 006
- Tell-based artifact knowledge sharing (already works via existing Tell system — agent shares BelievedEntityState which now includes believed_artifact)
- Artifact-specific perception fidelity (uses standard observation_fidelity)

## Acceptance Criteria

### Tests That Must Pass

1. Agent co-located with bounty artifact perceives it: `believed_artifact` populated with correct kind, state, terms
2. Agent co-located with notice artifact perceives it: `believed_artifact` populated with correct topic
3. Agent not co-located with artifact: no `believed_artifact` on any BelievedEntityState
4. Expired bounty perceived correctly: `believed_artifact.state == Expired`
5. Notice with ThreatWarning updates agent's place-related beliefs
6. Existing suite: `cargo test --workspace`

### Invariants

1. Agents only perceive artifacts at their current location (Principle 7 — locality)
2. Perceived artifact state may be stale (Principle 14 — world state ≠ belief state)
3. Perception does not modify artifact entities — read-only from world, write-only to beliefs

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — Unit tests for artifact perception: bounty perception, notice perception, non-co-located absence, stale belief retention
2. `crates/worldwake-systems/src/perception.rs` — Test notice topic internalization (threat warning → place belief update)

### Commands

1. `cargo test -p worldwake-systems -- perception`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
