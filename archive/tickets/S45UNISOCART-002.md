# S45UNISOCART-002: Artifact lifecycle system and posting actions (PostBounty, PostNotice)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new SystemId variant, new system function, two new action handlers, action registration
**Deps**: S45UNISOCART-001

## Problem

Social artifacts exist as types (from 001) but cannot be created or expired. This ticket adds the runtime machinery: an expiration system that transitions Active artifacts to Expired, and two posting actions (PostBounty, PostNotice) that create `SocialArtifact` entities with appropriate components at a physical place.

## Assumption Reassessment (2026-04-04)

1. `SystemId` is defined via `define_system_ids!` in `crates/worldwake-sim/src/system_manifest.rs`. Adding `ArtifactLifecycle` is straightforward, but the live engine has only one ordinary per-tick system phase plus action start/progress before that phase.
2. `tick_step()` in `crates/worldwake-sim/src/tick_step.rs` processes inputs and action progression before `run_systems()`. So a normal post-action system cannot enforce “expired before claim start on the expiration tick.”
3. To keep expiry timing lawful under Principles 8, 9, and 12, this ticket must own a real pre-action system phase for artifact expiry instead of relying on a later ticket to patch authoritative start timing.
4. System dispatch table in `crates/worldwake-systems/src/lib.rs` still maps one handler per `SystemId`. The clean live boundary is: `ArtifactLifecycle` is dispatched through the same table, but from a pre-action manifest in sim rather than the ordinary post-action canonical manifest.
5. Action registration follows `register_*_action` pattern in `crates/worldwake-systems/src/action_registry.rs:23-54`, but this ticket also needs new payload types in `crates/worldwake-sim/src/action_payload.rs` because PostBounty/PostNotice are payload-bearing actions.
6. `ActionDomain::Social` exists at `crates/worldwake-core/src/action_domain.rs:10`. PostBounty and PostNotice belong there.
7. `WorldTxn::create_entity(kind: EntityKind)` plus `WorldTxn::set_ground_location(...)` form the live runtime path for posting placed artifact entities.
8. `ContentionPolicy` race mode already exists in `crates/worldwake-core/src/contention.rs`. Claim contention on bounty entities also requires a `ContentionQueue`, so PostBounty must seed both authoritative components at creation time.

## Architecture Check

1. Expiration timing is part of world meaning, not incidental engine order. This ticket therefore owns both the artifact lifecycle handler and the minimal sim pre-action dispatch support needed to make `expires_at == current_tick` authoritative before any claim or interaction starts.
2. Posting actions remain ordinary `ActionDomain::Social` handlers. They create artifact entities and seed their shared authoritative components, but do not bypass the normal action/event pipeline.
3. No backward-compatibility shims. The live engine gets an explicit pre-action artifact lifecycle phase rather than an implicit “fix it later in ClaimBounty” workaround.

## Verification Layers

1. Pre-action expiry transitions Active→Expired at correct tick before same-tick action admission → authoritative world state assertion in focused tick-step proof
2. PostBounty creates SocialArtifact entity with correct components → event-log delta + authoritative world state
3. PostNotice creates SocialArtifact entity with correct components → event-log delta + authoritative world state
4. Bounty posting seeds both `ContentionPolicy` race mode and an empty `ContentionQueue` → authoritative world state check
5. Pre-action dispatch ordering: ArtifactLifecycle runs before input/action processing while ordinary canonical systems remain post-action → focused sim ordering test

## What to Change

### 1. Add `SystemId::ArtifactLifecycle`

In `crates/worldwake-sim/src/system_manifest.rs`:
- Add `(ArtifactLifecycle, "artifact_lifecycle")` to `define_system_ids!` macro.
- Add a `SystemManifest::pre_action()` ordering surface that contains `ArtifactLifecycle`.
- Keep `SystemManifest::canonical()` as the ordinary post-action world-update order.

In `crates/worldwake-sim/src/tick_step.rs`:
- Run the pre-action manifest before input drain and action progression.
- Count the pre-action phase as part of `systems_ran`.

### 2. Implement `artifact_lifecycle_system()`

Create `crates/worldwake-systems/src/artifact_lifecycle.rs`:
- Iterate all entities with `ArtifactHeader` component.
- For each Active artifact where `expires_at <= current_tick`, transition to `ArtifactState::Expired`.
- Emit expiration event with artifact entity as target.
- This handler is dispatched through the new pre-action system phase, not the ordinary post-action canonical manifest.

### 3. Implement PostBounty action handler

Create `crates/worldwake-systems/src/artifact_actions.rs`:
- `on_start`: Validate issuer is co-located with the posting place and that the payload names the bound posting place. Validate the reward source shape is lawful:
  - `InstitutionalTreasury`: issuer is the current holder of `issuing_authority`
  - `PersonalFunds`: issuer matches actor and actor currently controls enough commodity
  - `ReservedLot`: actor currently controls the reserved lot and it contains the promised commodity/quantity
- `on_commit`: Create `SocialArtifact` entity via `WorldTxn::create_entity`. Set `ArtifactHeader` + `BountyTerms` components. Place artifact at the posting location. Seed both `ContentionPolicy { max_waiters: Some(0), ... }` and `ContentionQueue::default()` for later claim contention. Emit the normal action commit event through the action pipeline.
- Duration: 1-2 ticks.
- Target: actor place.
- Payload: posting place + artifact header fields + bounty terms.

### 4. Implement PostNotice action handler

In same `crates/worldwake-systems/src/artifact_actions.rs`:
- `on_start`: Validate issuer is co-located with the posting place and the payload names the bound posting place.
- `on_commit`: Create `SocialArtifact` entity with `ArtifactHeader` + `NoticeContent`. Place at posting location. Emit the normal action commit event through the action pipeline.
- Duration: 1 tick.

### 5. Register actions and system

In `crates/worldwake-systems/src/action_registry.rs`: Add `register_artifact_actions()` call.
In `crates/worldwake-systems/src/lib.rs`: Export `artifact_lifecycle_system` plus `register_artifact_actions()`, and wire `ArtifactLifecycle` into the dispatch table.
In `crates/worldwake-sim/src/action_payload.rs` and `crates/worldwake-sim/src/lib.rs`: Add and re-export the new posting payload types.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-sim/src/system_manifest.rs` (modify)
- `crates/worldwake-sim/src/action_payload.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify)
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

1. Pre-action artifact expiry transitions Active artifact to Expired when `current_tick >= expires_at` before same-tick action admission
2. `artifact_lifecycle_system` does not transition artifacts without `expires_at`
3. PostBounty creates SocialArtifact entity with ArtifactHeader (kind=Bounty, state=Active) and BountyTerms
4. PostBounty attaches both `ContentionPolicy` race mode and `ContentionQueue` to the bounty entity
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
4. `crates/worldwake-sim/src/tick_step.rs` or `system_manifest.rs` — Focused proof that ArtifactLifecycle runs in the pre-action phase before action admission

### Commands

1. `cargo test -p worldwake-sim system_manifest`
2. `cargo test -p worldwake-sim tick_step`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

Completed: 2026-04-04

What changed:
- Added a real pre-action artifact lifecycle phase in `crates/worldwake-sim/src/tick_step.rs` and `crates/worldwake-sim/src/system_manifest.rs` so artifact expiry is processed before input drain and action admission.
- Added `SystemId::ArtifactLifecycle` and dispatched it through `crates/worldwake-systems/src/artifact_lifecycle.rs` via the normal systems dispatch table in `crates/worldwake-systems/src/lib.rs`.
- Added `post_bounty` and `post_notice` action handlers in `crates/worldwake-systems/src/artifact_actions.rs`, with registration in `crates/worldwake-systems/src/action_registry.rs`.
- Added posting payloads in `crates/worldwake-sim/src/action_payload.rs` and re-exported them from `crates/worldwake-sim/src/lib.rs`.
- Extended `crates/worldwake-core/src/component_schema.rs` so `ContentionPolicy` and `ContentionQueue` are valid on `EntityKind::SocialArtifact`, and `PostBounty` now seeds race-mode contention state on the created bounty artifact.
- Absorbed the real shared-enum fallout from the new payload variants, including boxing the `RequestedAffordanceUnavailable.payload_override` carrier in `crates/worldwake-sim/src/tick_step.rs` to satisfy CI-sized clippy constraints.
- Repaired the workspace golden fallout in `crates/worldwake-ai/tests/golden_integration.rs`:
  - `T29` now asserts the strongest honest cross-domain coverage boundary for the scenario (`Transport` + `Social`)
  - `T22` now gives the witness explicit full-fidelity perception because the scenario contract requires deterministic acquisition of the raid observation

Deviations from original plan:
- The ticket expanded beyond a normal post-action lifecycle system. To keep expiry lawful under Principles 8, 9, and 12, it had to own a true pre-action system phase instead of leaving same-tick expiry enforcement to later claim-start validation.
- The actual implementation boundary also included shared payload/error-carrier fallout and AI golden recalibration, which were not fully reflected in the original file list.

Verification results:
- `cargo test -p worldwake-sim tick_step`
- `cargo test -p worldwake-sim system_manifest`
- `cargo test -p worldwake-systems artifact_lifecycle`
- `cargo test -p worldwake-systems artifact_actions`
- `cargo test -p worldwake-systems`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p worldwake-ai t29_wrongful_accusation_seed_2 -- --nocapture`
- `cargo test -p worldwake-ai t22_camp_reconstitution_seed_1 -- --nocapture`
- `cargo test -p worldwake-ai t22_camp_reconstitution_seed_2 -- --nocapture`
- `cargo test --workspace`
