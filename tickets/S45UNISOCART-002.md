# S45UNISOCART-002: Artifact lifecycle system and posting actions (PostBounty, PostNotice)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new SystemId variant, new system function, two new action handlers, action registration
**Deps**: S45UNISOCART-001

## Problem

Social artifacts exist as types (from 001) but cannot be created or expired. This ticket adds the runtime machinery: an expiration system that transitions Active artifacts to Expired, and two posting actions (PostBounty, PostNotice) that create `SocialArtifact` entities with appropriate components at a physical place.

## Assumption Reassessment (2026-04-04)

1. `SystemId` is defined via `define_system_ids!` macro in `crates/worldwake-sim/src/system_manifest.rs:52-62`. Currently 9 system IDs. Adding `ArtifactLifecycle` follows the same pattern.
2. System dispatch table in `crates/worldwake-systems/src/lib.rs:68-80` maps system functions. New system added to the handlers array.
3. Canonical system ordering in `SystemManifest::canonical()` at `system_manifest.rs:97-107` determines tick execution order. `ArtifactLifecycle` runs before action domain systems (expiration must precede claims).
4. Action registration follows `register_*_action` pattern in `crates/worldwake-systems/src/action_registry.rs:23-54`. Each action module exports a registration function.
5. `ActionDomain::Social` exists at `crates/worldwake-core/src/action_domain.rs:10`. PostBounty and PostNotice use this domain.
6. `WorldTxn::create_entity(kind: EntityKind)` at `crates/worldwake-core/src/world_txn.rs:224` creates entities at runtime. PostBounty/PostNotice use this to spawn SocialArtifact entities.
7. `ContentionPolicy` with `max_waiters: Some(0)` (race mode) is used for bounty claim contention — policy attached at PostBounty creation time. `ContentionPolicy` at `crates/worldwake-core/src/contention.rs:47-60`.

## Architecture Check

1. Separate lifecycle system (expiration) from action handlers (posting) keeps systems decoupled per Principle 26. The lifecycle system reads tick + artifact state; posting actions create entities. Neither depends on the other's internals.
2. No backward-compatibility shims. New system and actions added cleanly.

## Verification Layers

1. Expiration transitions Active→Expired at correct tick → authoritative world state assertion in unit test
2. PostBounty creates SocialArtifact entity with correct components → event-log delta + authoritative world state
3. PostNotice creates SocialArtifact entity with correct components → event-log delta + authoritative world state
4. ContentionPolicy attached to bounty at creation → authoritative world state check
5. System ordering: ArtifactLifecycle runs before action domains → system manifest ordering test

## What to Change

### 1. Add `SystemId::ArtifactLifecycle`

In `crates/worldwake-sim/src/system_manifest.rs`:
- Add `(ArtifactLifecycle, "artifact_lifecycle")` to `define_system_ids!` macro.
- Add to canonical ordering: before action domain systems, after perception (or alongside Contention).

### 2. Implement `artifact_lifecycle_system()`

Create `crates/worldwake-systems/src/artifact_lifecycle.rs`:
- Iterate all entities with `ArtifactHeader` component.
- For each Active artifact where `expires_at <= current_tick`, transition to `ArtifactState::Expired`.
- Emit expiration event with artifact entity as target.

### 3. Implement PostBounty action handler

Create `crates/worldwake-systems/src/artifact_actions.rs`:
- `on_start`: Validate issuer is co-located with posting place. Validate issuer has authority (office holder) or personal funds.
- `on_commit`: Create `SocialArtifact` entity via `WorldTxn::create_entity`. Set `ArtifactHeader` + `BountyTerms` components. Place artifact at posting location. Attach `ContentionPolicy` with `max_waiters: Some(0)` for race-mode claim contention. Emit bounty-posted event.
- Duration: 1-2 ticks.
- Affordance targets: posting place (co-located).
- Affordance payloads: bounty terms (target, proof, reward).

### 4. Implement PostNotice action handler

In same `crates/worldwake-systems/src/artifact_actions.rs`:
- `on_start`: Validate issuer co-located with posting place.
- `on_commit`: Create `SocialArtifact` entity with `ArtifactHeader` + `NoticeContent`. Place at posting location. Emit notice-posted event.
- Duration: 1 tick.

### 5. Register actions and system

In `crates/worldwake-systems/src/action_registry.rs`: Add `register_artifact_actions()` call.
In `crates/worldwake-systems/src/lib.rs`: Add `artifact_lifecycle_system` to dispatch table. Add `pub mod artifact_actions; pub mod artifact_lifecycle;`.

## Files to Touch

- `crates/worldwake-sim/src/system_manifest.rs` (modify)
- `crates/worldwake-systems/src/artifact_lifecycle.rs` (new)
- `crates/worldwake-systems/src/artifact_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify)
- `crates/worldwake-systems/src/lib.rs` (modify)

## Out of Scope

- ClaimBounty action — ticket 003
- Perception of artifacts — ticket 004
- AI candidate generation — ticket 005
- Golden tests — ticket 006
- Affordance payload enumeration for AI-driven posting (AI doesn't post bounties yet — only office holders via manual action or institutional logic)

## Acceptance Criteria

### Tests That Must Pass

1. `artifact_lifecycle_system` transitions Active artifact to Expired when `current_tick >= expires_at`
2. `artifact_lifecycle_system` does not transition artifacts without `expires_at`
3. PostBounty creates SocialArtifact entity with ArtifactHeader (kind=Bounty, state=Active) and BountyTerms
4. PostBounty attaches ContentionPolicy with race mode to bounty entity
5. PostNotice creates SocialArtifact entity with ArtifactHeader (kind=Notice) and NoticeContent
6. PostBounty fails if issuer not co-located with posting place
7. Existing suite: `cargo test --workspace`

### Invariants

1. Artifacts transition to Expired only when `current_tick >= expires_at` — never prematurely
2. PostBounty/PostNotice create entities with stable identity (EntityId persists across ticks)
3. All created artifacts have placement (exist at a physical place)
4. ContentionPolicy on bounty entity has `max_waiters: Some(0)` (race mode)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_lifecycle.rs` — Unit tests for expiration transitions, no-expiration passthrough
2. `crates/worldwake-systems/src/artifact_actions.rs` — Unit tests for PostBounty/PostNotice creation, precondition failures
3. `crates/worldwake-systems/src/action_registry.rs` — Update completeness test to include new actions

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
