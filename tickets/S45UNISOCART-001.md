# S45UNISOCART-001: Core types, EntityKind migration, component registration, and belief types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — EntityKind enum (remove 2 variants, add 1), new component types, component schema, belief module, save format version
**Deps**: None

## Problem

The simulation has no substrate for social artifacts (bounties, notices, warrants, contracts, debts). `EntityKind::Contract` and `EntityKind::Rumor` exist as empty placeholder variants with zero component registrations and zero system consumers. This ticket introduces the unified `SocialArtifact` entity kind, all core types, belief structures, and component registrations needed by downstream action/perception/AI tickets.

## Assumption Reassessment (2026-04-04)

1. `EntityKind` has 11 variants including `Contract` and `Rumor` — confirmed at `crates/worldwake-core/src/entity.rs:7-20`. `Contract` appears only in `ALL_ENTITY_KINDS` test constant (line 45). `Rumor` appears only in `ALL_ENTITY_KINDS` (line 46) and a test pattern match in `crates/worldwake-sim/src/action_semantics.rs:1148`. Neither has component registrations in `component_schema.rs`.
2. `component_schema.rs` uses `with_component_schema_entries!` macro with closures filtering by `EntityKind` — confirmed at `crates/worldwake-core/src/component_schema.rs`. Macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) import types by bare name.
3. `BelievedEntityState` has Optional type-specific fields (`believed_contention: Option<BelievedContentionState>`, `believed_activity: Option<BelievedActivity>`) — confirmed at `crates/worldwake-core/src/belief.rs:700-713`. Pattern established for adding `believed_artifact: Option<BelievedArtifactState>`.
4. `SAVE_FORMAT_VERSION` is 17 — confirmed at `crates/worldwake-sim/src/save_load.rs:5`.
5. `GoalKind` derives `Copy` — confirmed at `crates/worldwake-core/src/goal.rs:16`. All new types referenced by GoalKind must be Copy.
6. `InstitutionalClaim` exists with 7 variants — confirmed at `crates/worldwake-core/src/institutional.rs:18-60`. Used by `NoticeTopic::Institutional`.
7. No active specs reference Contract, Rumor, bounty, or artifact — confirmed by grep of `specs/`.

## Architecture Check

1. Consolidating Contract/Rumor into SocialArtifact with ArtifactKind discriminant is cleaner than maintaining 3 separate entity kinds for the same domain. Per Principle 28, empty placeholders are removed rather than left alongside the new unified kind.
2. No backward-compatibility shims. Contract and Rumor variants are deleted, not aliased.

## Verification Layers

1. EntityKind has SocialArtifact, no Contract/Rumor → `cargo test -p worldwake-core` (entity module tests, ALL_ENTITY_KINDS constant)
2. Components registered on SocialArtifact → component_schema unit tests
3. BelievedArtifactState accessible on BelievedEntityState → belief module compilation + unit test
4. Save format version bumped → save_load module constant check
5. All macro expansion sites compile with new types → `cargo build --workspace`
6. Single-layer ticket (worldwake-core types only) — no cross-system verification needed.

## What to Change

### 1. Remove `EntityKind::Contract` and `EntityKind::Rumor`, add `SocialArtifact`

In `crates/worldwake-core/src/entity.rs`:
- Remove `Contract` and `Rumor` variants from `EntityKind` enum.
- Add `SocialArtifact` variant.
- Update `ALL_ENTITY_KINDS` test constant (count stays 11 → net change: -2 +1 = 10, then +1 = 10... actually remove 2, add 1 = 10 variants total).
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
- Existing placement components (extend their closures to include `EntityKind::SocialArtifact`).

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

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
