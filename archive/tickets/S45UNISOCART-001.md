# S45UNISOCART-001: Core types, EntityKind migration, component registration, and belief types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — EntityKind enum (remove 2 variants, add 1), new component types, component schema, belief module, save format version
**Deps**: None

## Problem

The simulation has no substrate for social artifacts (bounties, notices, warrants, contracts, debts). `EntityKind::Contract` and `EntityKind::Rumor` exist as empty placeholder variants with zero component registrations and zero system consumers. This ticket introduces the unified `SocialArtifact` entity kind, all core types, belief structures, and component registrations needed by downstream action/perception/AI tickets.

## Assumption Reassessment (2026-04-04)

1. `EntityKind` has 11 variants including `Contract` and `Rumor` — confirmed at `crates/worldwake-core/src/entity.rs:7-20`. `Contract` appears only in `ALL_ENTITY_KINDS` test constant. `Rumor` appears only in `ALL_ENTITY_KINDS` and a test pattern match in `crates/worldwake-sim/src/action_semantics.rs:1148`. Neither has component registrations in `component_schema.rs`.
2. `component_schema.rs` uses `with_component_schema_entries!` macro with closures filtering by `EntityKind` — confirmed at `crates/worldwake-core/src/component_schema.rs`. Macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) import types by bare name.
3. `BelievedEntityState` has Optional type-specific fields (`believed_contention: Option<BelievedContentionState>`, `believed_activity: Option<BelievedActivity>`) — confirmed at `crates/worldwake-core/src/belief.rs:700-713`. Pattern established for adding `believed_artifact: Option<BelievedArtifactState>`.
4. `SAVE_FORMAT_VERSION` is 17 — confirmed at `crates/worldwake-sim/src/save_load.rs:5`.
5. `GoalKind` derives `Copy` — confirmed at `crates/worldwake-core/src/goal.rs:16`. All new types referenced by GoalKind must be Copy.
6. `InstitutionalClaim` exists with 7 variants — confirmed at `crates/worldwake-core/src/institutional.rs:18-60`. Used by `NoticeTopic::Institutional`.
7. The user's `specs/S45-unified*` reference resolves to `specs/S45-unified-social-artifact-model.md`. The ticket's shorthand is directionally correct but its exact filename was stale.
8. This is not purely `worldwake-core` fallout. `BelievedEntityState` is constructed directly in sim, systems, and AI helpers/tests, and `EntityKind::Rumor` still appears in `crates/worldwake-sim/src/action_semantics.rs`. The real proof surface is core-first but cross-crate.

## Architecture Check

1. Consolidating Contract/Rumor into SocialArtifact with ArtifactKind discriminant is cleaner than maintaining 3 separate entity kinds for the same domain. Per Principle 28, empty placeholders are removed rather than left alongside the new unified kind.
2. No backward-compatibility shims. Contract and Rumor variants are deleted, not aliased.

## Verification Layers

1. EntityKind has SocialArtifact, no Contract/Rumor → `cargo test -p worldwake-core` (entity module tests, ALL_ENTITY_KINDS constant)
2. Components registered on SocialArtifact → component_schema unit tests
3. BelievedArtifactState accessible on BelievedEntityState → belief module compilation + unit test
4. Save format version bumped → save_load module constant check
5. All macro expansion sites compile with new types → `cargo test -p worldwake-core`
6. Shared belief/entity fallout across sim, systems, and AI compiles cleanly → `cargo test --workspace`
7. CI-matching lint surface remains clean after entity-kind and shared-model changes → `cargo clippy --workspace --all-targets -- -D warnings`

## What to Change

### 1. Remove `EntityKind::Contract` and `EntityKind::Rumor`, add `SocialArtifact`

In `crates/worldwake-core/src/entity.rs`:
- Remove `Contract` and `Rumor` variants from `EntityKind` enum.
- Add `SocialArtifact` variant.
- Update `ALL_ENTITY_KINDS` test constant to the new 10-variant list.
- Update any exhaustive matches in the crate.

In `crates/worldwake-sim/src/action_semantics.rs:1148`: Update the test pattern match that uses `EntityKind::Rumor` to use a different variant (e.g., `EntityKind::SocialArtifact`).

### 2. Add new social artifact types module

Create `crates/worldwake-core/src/social_artifact.rs` with:
- `ArtifactHeader` component struct (kind, issuer, issuing_authority, created_at, expires_at, state, jurisdiction)
- `ArtifactKind` enum (Bounty, Notice)
- `ArtifactState` enum (Active, Fulfilled, Expired, Withdrawn, Destroyed)
- `BountyTerms` component struct (target, proof_requirement, reward_commodity, reward_quantity, reward_source, claim_place)
- `BountyTarget` enum (EliminateEntity, DeliverCommodity)
- `ProofRequirement` enum (PhysicalEvidence, WitnessTestimony, SelfReport)
- `RewardSource` enum (InstitutionalTreasury, PersonalFunds, ReservedLot)
- `NoticeContent` component struct (topic)
- `NoticeTopic` enum (ThreatWarning, OfficeVacancy, CommodityShortage, Institutional)

All types derive appropriate traits (Serialize, Deserialize, Clone, Debug, Eq, PartialEq; Copy where applicable).

### 3. Add belief types

In `crates/worldwake-core/src/belief.rs`:
- Add `BelievedArtifactState` struct (kind, state, issuer, expires_at, bounty_terms, notice_topic, observed_tick).
- Add `BelievedBountyTerms` struct (target, reward_commodity, reward_quantity, claim_place).
- Add `pub believed_artifact: Option<BelievedArtifactState>` field to `BelievedEntityState`.

### 4. Register components in `component_schema.rs`

Register on `EntityKind::SocialArtifact`:
- `ArtifactHeader`
- `BountyTerms`
- `NoticeContent`
- Existing placement/location-relevant components that should lawfully apply to artifact entities (extend their closures to include `EntityKind::SocialArtifact` where the live schema currently gates them by `EntityKind`).

Ensure all new type names are imported at each macro expansion site.

### 5. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs`: change `SAVE_FORMAT_VERSION` from 17 to 18.

### 6. Re-export new types from `crates/worldwake-core/src/lib.rs`

Add `pub mod social_artifact;` and re-export key types following existing patterns.

## Files to Touch

- `crates/worldwake-core/src/entity.rs` (modify)
- `crates/worldwake-core/src/social_artifact.rs` (new)
- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify — macro expansion site imports)
- `crates/worldwake-core/src/delta.rs` (modify — macro expansion site imports)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro expansion site imports)
- `crates/worldwake-sim/src/save_load.rs` (modify)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — update Rumor test reference)

## Out of Scope

- Action handlers (PostBounty, ClaimBounty, PostNotice) — ticket 002/003
- Perception integration — ticket 004
- AI candidate generation — ticket 005
- Golden tests — ticket 006
- CLI display of social artifacts

## Acceptance Criteria

### Tests That Must Pass

1. `EntityKind` test: `ALL_ENTITY_KINDS` includes `SocialArtifact`, excludes `Contract` and `Rumor`
2. Component schema test: `ArtifactHeader`, `BountyTerms`, `NoticeContent` registered on `EntityKind::SocialArtifact`
3. `BelievedEntityState` compiles with `believed_artifact: Option<BelievedArtifactState>` field
4. Existing suite: `cargo test --workspace`

### Invariants

1. `EntityKind` has exactly 10 variants after removal of 2 and addition of 1
2. No code references `EntityKind::Contract` or `EntityKind::Rumor`
3. All component schema macro expansion sites compile with new types in scope
4. `SAVE_FORMAT_VERSION == 18`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/entity.rs` — Update `ALL_ENTITY_KINDS` constant and exhaustive match tests
2. `crates/worldwake-core/src/social_artifact.rs` — Unit tests for Serialize/Deserialize round-trip of all new types
3. `crates/worldwake-core/src/belief.rs` — Test `BelievedArtifactState` construction and Default behavior
4. Shared constructor fallout in sim/systems/AI should compile cleanly after `BelievedEntityState` and `EntityKind` changes

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completed on 2026-04-04.

Delivered the unified social-artifact substrate in `worldwake-core` with the new `social_artifact.rs` model family, the `EntityKind::SocialArtifact` migration, authoritative component registration for `ArtifactHeader`, `BountyTerms`, and `NoticeContent`, crate-root re-exports, and the matching `BelievedArtifactState` / `BelievedBountyTerms` belief additions on `BelievedEntityState`. The implementation also absorbed the real shared fallout the ticket exposed: cross-crate `BelievedEntityState` fixture and helper updates across sim, systems, and AI; the stale `EntityKind::Rumor` sim test reference; and the lifecycle classification fix in `World::requires_physical_placement(...)` so `SocialArtifact` follows the existing placed-entity contract.

The save boundary changed as expected, so `SAVE_FORMAT_VERSION` was bumped from 17 to 18. The originally implied core-only scope was corrected before closeout: macro expansion imports were updated in `world.rs`, `delta.rs`, and `component_tables.rs`, and the shared belief-shape fallout was handled across the workspace rather than being left as compile-only surprises.

Verification completed with:

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
