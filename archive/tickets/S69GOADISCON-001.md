# S69GOADISCON-001: Expand GoalDispatchKey with payload-aware ShareBelief and PostNotice variants

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — enum variant expansion, no runtime behavior change
**Deps**: None

## Problem

`GoalDispatchKey` inconsistently discriminates payloads: `AcquireCommodity` splits by `CommodityPurpose` (3 variants) and `PunishAccused` by `PunishmentKind` (2 variants), but `ShareBelief` and `PostNotice` collapse all payload sub-families into single variants. This prevents using the dispatch key as a lookup for static per-goal-family metadata that depends on payload fields (e.g., `GoalFamilyPolicy` varies by `CommunicationClass` for `ShareBelief` and by `NoticeTopic` for `PostNotice`).

## Assumption Reassessment (2026-04-07)

1. `GoalDispatchKey` is defined in `crates/worldwake-ai/src/goal_dispatch_key.rs` with 34 variants. Confirmed at line 6-41.
2. `from_goal_kind()` is a `const fn` at line 87-128 that maps `GoalKind` to `GoalDispatchKey`. Currently maps all `ShareBelief { .. }` to `Self::ShareBelief` (line 116) and all `PostNotice { .. }` to `Self::PostNotice` (line 115).
3. `GoalDispatchKey::ALL` constant at line 44-79 lists all 34 variants. Count matches the enum definition.
4. `declaration()` match at `goal_dispatch_decl.rs:398-435` maps each `GoalDispatchKey` variant to a `&'static GoalDispatchDeclaration`. Has `Self::ShareBelief` and `Self::PostNotice` arms.
5. Test `ALL_KEYS` constant in `goal_dispatch_decl.rs:448-483` duplicates the ALL list. Must be updated in sync.
6. `CommunicationClass` is used in `GoalKind::ShareBelief` payload. Three variants: `Alarm`, `Testimony`, `Gossip`. Located in `worldwake-core`.
7. `NoticeTopic` is used in `GoalKind::PostNotice` payload. Has `ThreatWarning { .. }` and other variants. Located in `worldwake-core`.
8. Existing tests in `goal_dispatch_key.rs` include `test_goal_dispatch_key_exhaustive_coverage` (line 262) and `test_goal_dispatch_key_all_lists_each_dispatch_key_once` (line 363) — both will need updating.

## Architecture Check

1. This extends an existing discrimination pattern (AcquireCommodity, PunishAccused) to two more goal families where payload fields produce behaviorally distinct sub-families. The approach is consistent with the existing codebase convention.
2. No backward-compatibility shims. The old `ShareBelief` and `PostNotice` variants are removed and replaced entirely.

## Verification Layers

1. Exhaustive coverage invariant → `test_goal_dispatch_key_exhaustive_coverage` proves every `GoalKind` variant maps to a `GoalDispatchKey`
2. ALL list completeness → `test_goal_dispatch_key_all_lists_each_dispatch_key_once` proves the ALL constant matches the enum
3. Declaration completeness → `test_declaration_completeness` in `goal_dispatch_decl.rs` proves every dispatch key has a declaration entry
4. Single-layer ticket (AI planner internals only) — no cross-system verification needed

## What to Change

### 1. Replace `ShareBelief` and `PostNotice` enum variants

In `goal_dispatch_key.rs`, remove:
- `ShareBelief`
- `PostNotice`

Add:
- `ShareBeliefAlarm`
- `ShareBeliefTestimony`
- `ShareBeliefGossip`
- `PostNoticeWarning`
- `PostNoticeOther`

### 2. Update `from_goal_kind()`

Replace the single-arm mappings with payload-discriminating matches:

```rust
GoalKind::ShareBelief { communication_class, .. } => match communication_class {
    CommunicationClass::Alarm => Self::ShareBeliefAlarm,
    CommunicationClass::Testimony => Self::ShareBeliefTestimony,
    CommunicationClass::Gossip => Self::ShareBeliefGossip,
},
GoalKind::PostNotice { topic, .. } => match topic {
    NoticeTopic::ThreatWarning { .. } => Self::PostNoticeWarning,
    _ => Self::PostNoticeOther,
},
```

### 3. Update `ALL` constant

Replace `Self::ShareBelief` and `Self::PostNotice` with the 5 new variants. Update the array count from 34 to 37.

### 4. Update `declaration()` match in `goal_dispatch_decl.rs`

Replace `Self::ShareBelief => &DECL_SHARE_BELIEF` with three arms pointing to the existing `DECL_SHARE_BELIEF` constant (all three share the same declaration for now — ticket 002 will differentiate them when adding `family_policy`).

Replace `Self::PostNotice => &DECL_POST_NOTICE` with two arms pointing to the existing `DECL_POST_NOTICE` constant (same rationale).

### 5. Update test constants and test assertions

- Update `ALL_KEYS` in `goal_dispatch_decl.rs` tests
- Update `test_goal_dispatch_key_exhaustive_coverage` to cover ShareBelief sub-variants and PostNotice sub-variants
- Update `test_goal_dispatch_key_all_lists_each_dispatch_key_once` for the new count
- Add `test_goal_dispatch_key_payload_sensitive_share_belief_splits` (following the pattern of `test_goal_dispatch_key_payload_sensitive_acquire_splits`)
- Add `test_goal_dispatch_key_payload_sensitive_post_notice_splits`

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)

## Out of Scope

- Adding new fields to `GoalDispatchDeclaration` (ticket 002)
- Changing `GoalFamilyPolicy`, `GoalPriorityClass`, or any runtime behavior
- Modifying `goal_policy.rs`, `ranking.rs`, or `goal_model.rs`
- Adding new `GoalKind` variants

## Acceptance Criteria

### Tests That Must Pass

1. `test_goal_dispatch_key_exhaustive_coverage` — every GoalKind maps to a GoalDispatchKey
2. `test_goal_dispatch_key_all_lists_each_dispatch_key_once` — ALL has exactly 37 unique entries
3. `test_declaration_completeness` — every GoalDispatchKey has a declaration
4. New: `test_goal_dispatch_key_payload_sensitive_share_belief_splits` — ShareBelief Alarm/Testimony/Gossip map to distinct keys
5. New: `test_goal_dispatch_key_payload_sensitive_post_notice_splits` — PostNotice ThreatWarning vs other map to distinct keys
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Every `GoalKind` variant maps to exactly one `GoalDispatchKey` variant via `from_goal_kind()`
2. Every `GoalDispatchKey` variant has a corresponding entry in `declaration()`
3. `GoalDispatchKey::ALL` contains every variant exactly once

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_key.rs::test_goal_dispatch_key_payload_sensitive_share_belief_splits` — proves CommunicationClass discrimination
2. `crates/worldwake-ai/src/goal_dispatch_key.rs::test_goal_dispatch_key_payload_sensitive_post_notice_splits` — proves NoticeTopic discrimination
3. Modified: `test_goal_dispatch_key_exhaustive_coverage`, `test_goal_dispatch_key_all_lists_each_dispatch_key_once` — updated for new variant count

### Commands

1. `cargo test -p worldwake-ai -- goal_dispatch_key`
2. `cargo test -p worldwake-ai -- test_declaration`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Assumption Reassessment (2026-04-07)

1. Auto-correction: `ALL_KEYS.len()` assertion in `test_declaration_completeness` (goal_dispatch_decl.rs:650) hardcoded 34 — updated to 37.
2. Auto-correction: `representative_goal_for()` in goal_dispatch_decl.rs tests had single `PostNotice` and `ShareBelief` arms — expanded to 5 arms with representative GoalKind values for each new variant.
3. Confirmed `ArtifactPostingContext` is `Copy` — no `.clone()` needed in tests.
4. Confirmed `InstitutionalClaim` uses `OfficeHolder` (not `OfficeClaim`).

## Outcome

Completed on 2026-04-07.

- Replaced `ShareBelief` and `PostNotice` GoalDispatchKey variants with 5 payload-aware variants (ShareBeliefAlarm, ShareBeliefTestimony, ShareBeliefGossip, PostNoticeWarning, PostNoticeOther)
- Updated `from_goal_kind()` to discriminate by `CommunicationClass` and `NoticeTopic`
- Updated ALL constant (34 → 37), declaration() match, ALL_KEYS test constant, representative_goal_for() test helper
- Added 2 new payload-sensitive split tests

## Verification Result

- Passed `cargo test -p worldwake-ai -- goal_dispatch_key` (10 tests, 0 failures)
- Passed `cargo test -p worldwake-ai -- test_declaration` (3 tests, 0 failures)
- Passed `cargo clippy --workspace --all-targets -- -D warnings` (clean)
- Passed `cargo test -p worldwake-ai` (full suite, 0 failures)
