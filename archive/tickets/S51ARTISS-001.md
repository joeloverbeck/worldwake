# S51ARTISS-001: Core types: GoalKind variants and UtilityProfile extension

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new GoalKind variants, UtilityProfile field additions, persisted profile fallout
**Deps**: None

## Problem

Agents can fulfill bounties and read notices but the AI never generates goals to *post* them. This ticket adds the foundational types: `GoalKind::PostBounty` and `GoalKind::PostNotice` variants, plus `bounty_posting_weight` and `notice_posting_weight` fields on `UtilityProfile` to control agent motivation for posting.

## Assumption Reassessment (2026-04-05)

1. `GoalKind` derives Copy at `crates/worldwake-core/src/goal.rs:9`. Currently 20 variants. `FulfillBounty` exists (line 64). `PostBounty`/`PostNotice` do not exist yet.
2. `BountyTarget` derives Copy at `crates/worldwake-core/src/social_artifact.rs:46`. Variants: `EliminateEntity { target: EntityId }`, `DeliverCommodity { commodity, quantity, destination }`. Safe for inclusion in GoalKind.
3. `NoticeTopic` derives Copy at `crates/worldwake-core/src/social_artifact.rs:77`. Variants: `ThreatWarning`, `OfficeVacancy`, `CommodityShortage`, `Institutional`. Safe for inclusion in GoalKind.
4. `UtilityProfile` at `crates/worldwake-core/src/utility_profile.rs:8-22` has 13 Permille fields. Default impl at lines 24-44.
5. `GoalKey` derived from `GoalKind` via `From<GoalKind> for GoalKey` at `goal.rs:141-170`. New variants need mapping.
6. UtilityProfile is a universal profile — `Default` impl required, already deserialized directly in AgentDef (no separate Def wrapper needed). Per `docs/spec-drafting-rules.md` section 5.
7. Ticket says `AgentDef/RON changes` are out of scope because UtilityProfile is directly deserialized, but live code has explicit `utility_profile: (...)` RON literals in `scenarios/cli-evaluation.ron`. Adding fields without updating those literals would break the active CLI evaluation scenario load. Correction applied: scenario RON fallout is in scope for this ticket because it is a direct shape change on a live deserialization surface.
8. `UtilityProfile` is authoritative persisted component state and `SAVE_FORMAT_VERSION` is currently `20` at `crates/worldwake-sim/src/save_load.rs:6`. Adding fields changes the save/load contract. Correction applied: save-format version bump is in scope for this ticket because the persisted component shape changes.
9. Ticket says CLI display and golden closeout are ticket `004`, but only `001`, `002`, and `003` existed in `tickets/` before reassessment. Correction applied: add `S51ARTISS-004` immediately so the active S51 chain honestly owns the remaining display + closeout slice.
10. Introducing new `GoalKind` variants also forces exhaustive AI/CLI tables to acknowledge them immediately, even before planner and candidate support are live. Correction applied: bounded inert handling in dispatch/ranking/policy/display tables is in scope for this ticket as compile fallout, while real planner ops and candidate emission remain deferred to `002` and `003`.

## Architecture Check

1. Motive context (why the agent wants to post) stays in candidate generation ranking metadata, NOT in GoalKind. This keeps GoalKey deduplication clean — two agents posting the same bounty target at the same place deduplicate regardless of motive.
2. New UtilityProfile fields default to 0 — agents don't autonomously post unless explicitly configured. Existing direct `UtilityProfile` literals and active scenario RON must still be updated to carry explicit zero values where the struct shape is named directly.
3. No backward-compatibility shims.

## Verification Layers

1. GoalKind::PostBounty and PostNotice compile with Copy -> focused core test
2. GoalKey extracts the posting place while preserving target/topic in the `kind` payload -> focused core test
3. UtilityProfile Default has 0-valued posting weights -> focused core test
4. Active CLI evaluation scenario still deserializes after the UtilityProfile shape change -> focused CLI scenario test
5. Save/load version gate advances with the persisted UtilityProfile shape change -> authoritative version constant + workspace verification

## What to Change

### 1. Add GoalKind variants

In `crates/worldwake-core/src/goal.rs`:

```rust
PostBounty {
    target: BountyTarget,
    posting_place: EntityId,
},
PostNotice {
    topic: NoticeTopic,
    posting_place: EntityId,
},
```

### 2. Add GoalKey mappings

In `From<GoalKind> for GoalKey` impl: add mappings for PostBounty and PostNotice. `GoalKey.kind` already carries the posting target/topic, so the new canonical extraction only needs to surface `posting_place` in the optional `place` slot.

### 3. Add UtilityProfile fields

In `crates/worldwake-core/src/utility_profile.rs`:

```rust
pub bounty_posting_weight: Permille,
pub notice_posting_weight: Permille,
```

Update Default impl:
```rust
bounty_posting_weight: Permille::new_unchecked(0),
notice_posting_weight: Permille::new_unchecked(0),
```

### 4. Update direct profile-shape consumers

Update direct `UtilityProfile { ... }` literals and active scenario RON payloads that name the full profile shape so the new fields deserialize and compile cleanly with explicit zero defaults.

### 5. Bump save format version

In `crates/worldwake-sim/src/save_load.rs`, increment `SAVE_FORMAT_VERSION` because the persisted `UtilityProfile` component shape changed.

### 6. Add inert exhaustive handling for the new GoalKind variants

Update exhaustive AI and CLI matches that must acknowledge every `GoalKind` variant even before S51 planner/candidate behavior is live. These placeholder branches should keep `PostBounty` / `PostNotice` non-live and non-plannable for now rather than silently misclassifying them as an older goal family.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-core/src/utility_profile.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/goal_policy.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify)
- `crates/worldwake-cli/src/display.rs` (modify)
- `scenarios/cli-evaluation.ron` (modify)
- direct `UtilityProfile` literal sites discovered by compile fallout (modify)

## Out of Scope

- PlannerOpKind variants — ticket 002
- Candidate generation — ticket 003
- Broader user-facing display polish and artifact-issuance closeout — ticket 004
- Golden tests — ticket 004
- Non-zero posting-weight tuning for showcase agents — ticket 004

## Acceptance Criteria

### Tests That Must Pass

1. GoalKind::PostBounty and PostNotice are Copy
2. GoalKey preserves posting target/topic in `kind` and extracts `posting_place` into the canonical `place` slot
3. UtilityProfile::default() has `bounty_posting_weight == 0` and `notice_posting_weight == 0`
4. Active CLI evaluation scenario still deserializes after the UtilityProfile shape change
5. Save format version advances for the persisted profile-shape change
6. Existing suite: `cargo test --workspace`

### Invariants

1. GoalKind remains Copy — all fields are Copy types
2. Default UtilityProfile has zero posting weights — no behavior change in existing scenarios
3. GoalKey deduplication includes target/topic through `kind` and posting place through `place`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — GoalKey mapping tests for PostBounty/PostNotice
2. `crates/worldwake-core/src/utility_profile.rs` — Default impl asserts new fields are zero
3. `crates/worldwake-cli/src/scenario/types.rs` — existing scenario deserialization coverage proves the active RON shape still loads after profile changes

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-cli test_scenario_def_deserialize_full`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-05

- Added `GoalKind::PostBounty` and `GoalKind::PostNotice` in `crates/worldwake-core/src/goal.rs`, including `GoalKey` extraction of `posting_place`.
- Added `bounty_posting_weight` and `notice_posting_weight` to `crates/worldwake-core/src/utility_profile.rs` with zero defaults.
- Updated direct `UtilityProfile` shape consumers and active deserialization surfaces, including `scenarios/cli-evaluation.ron`, `crates/worldwake-cli/src/scenario/types.rs`, and direct full-profile literals in test/helper code.
- Bumped `SAVE_FORMAT_VERSION` to `21` in `crates/worldwake-sim/src/save_load.rs` because the persisted profile shape changed.
- Added bounded inert handling for the new goal variants in non-owning AI/CLI surfaces so the workspace stays compiling and architecturally truthful before planner and candidate support land in later S51 tickets.
- Created `tickets/S51ARTISS-004.md` during reassessment so the remaining S51 display/showcase/golden closeout work had an explicit owner.

Deviations from original plan:

- The ticket absorbed direct scenario-deserialization and save-format fallout because `UtilityProfile` is a live persisted/deserialized shape.
- The ticket also absorbed bounded compile-fallout updates in downstream AI/CLI tables because the new shared goal variants had to be acknowledged immediately even though their live behavior remains deferred to `S51ARTISS-002` and `S51ARTISS-003`.
- `format_goal_kind()` support for the new variants landed here as part of that inert exhaustive-handling sweep, so `S51ARTISS-004` no longer owns first-time CLI rendering.

Verification:

- `cargo test -p worldwake-core`
- `cargo test -p worldwake-cli test_scenario_def_deserialize_full`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
