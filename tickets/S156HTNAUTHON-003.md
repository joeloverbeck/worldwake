# S156HTNAUTHON-003: Remove dead `EntityCriterion` criteria + two dead methods

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` HTN criteria, methods, registry
**Deps**: specs/S156-htn-authority-honesty.md (D3)

## Problem

`EntityCriterion::{Witness, ViolationEvidence, Ledger}` always evaluate `false` in the selector
(`htn/selector.rs:97-99`) and in the strategic resolver they fall through to `goal.evidence_places`
(`search/strategic.rs:761-763`). The only methods that use them *as `LocationKnown` preconditions*
— `investigate_on_scene` (id 9, `ViolationEvidence`) and `escort_to_office` (id 13, `Ledger`) —
are therefore permanently unselectable dead methods. `Witness` is never constructed by any method
at all. These are fossilized paths (FND-28). This ticket deletes the three criteria, their
selector/strategic arms, and the two dead methods, while preserving `fulfill_bounty_investigation`
and `investigate_by_ledger`, which reference the same concepts only via the *different*
`ArtifactTemplate` enum in their subgoals.

## Assumption Reassessment (2026-05-20)

1. `EntityCriterion` (`crates/worldwake-ai/src/htn/method_schema.rs:110-119`) has variants
   `Witness { topic }`, `ViolationEvidence { violation }`, `Ledger { institution }` plus the live
   `Target`/`Workstation`/`ResourceSource`/`Seller`. Selector arms returning `false` are at
   `htn/selector.rs:97-99`; the strategic fallthrough returning `goal.evidence_places` is at
   `search/strategic.rs:761-763` (function `resolve_entity_criterion_places`). Confirmed by grep.
2. The only methods using these criteria as `LocationKnown` preconditions are `investigate_on_scene`
   (`htn/methods.rs:383-410`, id 9, `ViolationEvidence`) and `escort_to_office`
   (`htn/methods.rs:503-533`, id 13, `Ledger`). `Witness` is never constructed by any method
   (grep returns only the two `=> false` / fallthrough arms). Both methods are permanently
   unselectable today.
3. Shared boundary under audit: the `EntityCriterion` enum and its two exhaustive match sites —
   `evaluate_precondition`'s `LocationKnown` arm (`htn/selector.rs:78-100`) and
   `resolve_entity_criterion_places` (`search/strategic.rs:739-765`). No cross-crate boundary
   (cross-crate grep for these variants is empty).
4. Preserved methods: `fulfill_bounty_investigation` (id 2, `htn/methods.rs:142-171`) and
   `investigate_by_ledger` (id 11, `htn/methods.rs:439-471`) reference `ViolationEvidence`/`Ledger`
   only via `SubgoalTemplate::InspectArtifact(ArtifactTemplate::…)` (`methods.rs:158, 162, 452, 460`)
   — `ArtifactTemplate` (`method_schema.rs:177-189`) is a separate enum not gated by the selector.
   Direct grep confirmed `methods.rs:158` is `ArtifactTemplate::ViolationEvidence`, not
   `EntityCriterion`. These methods are untouched.
5. Existing tests on the changed surface: `htn/registry.rs` inline test `registry_builds_with_13_methods`
   (line 75) asserts `registry.len() == 13` — deleting two methods makes the count 11, so this
   assertion (and its name) must be updated. Golden `htn_methods.rs` covers id 2
   (`generated_bounty_candidate_selects_fulfill_bounty_investigation`, :918) and id 12
   (`generated_escort_candidate_selects_escort_to_home`, :1002) — neither covers the deleted id 9 /
   id 13, confirming they are dead; these goldens must still pass unchanged. The strategic resolver
   inline tests (`search/strategic.rs`) do not exercise the evidence-place fallthrough arm directly.
6. Adjacent contradiction classification: none. The dead methods and `Witness` variant are
   genuinely unreachable; removal is a required consequence of the criteria deletion, not a
   separate bug. `goal.evidence_places` (`goal_model.rs:2231`) remains on `GoalOffer` for other
   uses — only the criterion-resolution arm that read it is removed.

## Architecture Check

1. Removing always-`false` criteria and the methods that depend on them eliminates unreachable
   code paths rather than leaving them as "would work if witness/evidence/ledger resolution
   existed" placeholders. Per the spec triage, that capability returns later *with* real
   enforcement, as a fresh design — not as a dormant dead path.
2. No shim: the two methods and their registry entries are deleted, and the exhaustive matches are
   reduced to the surviving variants. No catch-all arm is introduced that would silently absorb a
   future variant.

## Verification Layers

1. Deleted methods absent from the registry -> updated inline registry test asserting
   `registry.len() == 11` and that no method has id 9 or id 13 (D7 distributed).
2. Preserved bounty/ledger methods still selected -> existing goldens
   `generated_bounty_candidate_selects_fulfill_bounty_investigation` and
   `generated_escort_candidate_selects_escort_to_home` pass unchanged.
3. Exhaustive matches reduced cleanly (no unreachable arm, no missing arm) -> `cargo clippy
   --workspace --all-targets -- -D warnings`.
4. Single dominant layer (AI search-control); no authoritative-state or action-lifecycle ordering
   is affected because the removed paths never executed.

## What to Change

### 1. Remove the three `EntityCriterion` variants

Delete `Witness`, `ViolationEvidence`, and `Ledger` from `EntityCriterion` (`method_schema.rs`).

### 2. Remove their selector and strategic arms

In `htn/selector.rs`, delete the `Witness | ViolationEvidence | Ledger => false` arm (lines 97-99).
In `search/strategic.rs`, delete the `Witness | ViolationEvidence | Ledger => goal.evidence_places…`
fallthrough arm in `resolve_entity_criterion_places` (lines 761-763). Both matches must remain
exhaustive over the surviving variants without a catch-all.

### 3. Delete the two dead methods and their registry entries

Delete `investigate_on_scene` (id 9) and `escort_to_office` (id 13) from `htn/methods.rs`, and
remove their `insert` lines from `build_method_registry` in `htn/registry.rs`. Remove any imports
left unused by their deletion.

### 4. Update the registry count test

In `htn/registry.rs`, rename/adjust `registry_builds_with_13_methods` to assert the new count
(11) and, for D7 coverage, assert that the registry contains no method with id 9 or id 13.

## Files to Touch

- `crates/worldwake-ai/src/htn/method_schema.rs` (modify)
- `crates/worldwake-ai/src/htn/selector.rs` (modify)
- `crates/worldwake-ai/src/search/strategic.rs` (modify)
- `crates/worldwake-ai/src/htn/methods.rs` (modify)
- `crates/worldwake-ai/src/htn/registry.rs` (modify)

## Out of Scope

- `MethodPrecondition::AgentRole` / `RoleTag` (S156HTNAUTHON-002).
- `MethodSchema` field removal (S156HTNAUTHON-004).
- `fulfill_bounty_investigation` and `investigate_by_ledger` — explicitly preserved.
- Trace/fallback restructuring (S156HTNAUTHON-005). The `goal.evidence_places` field on `GoalOffer`
  is not removed.

## Acceptance Criteria

### Tests That Must Pass

1. Updated registry test asserts 11 methods and the absence of ids 9 and 13.
2. Existing goldens `generated_bounty_candidate_selects_fulfill_bounty_investigation` and
   `generated_escort_candidate_selects_escort_to_home` pass unchanged.
3. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. No `EntityCriterion` variant evaluates to a constant result; the surviving variants are all
   state-dependent (FND-28: no dead paths).
2. `fulfill_bounty_investigation` and `investigate_by_ledger` are untouched — they use
   `ArtifactTemplate`, not `EntityCriterion`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/htn/registry.rs` (test module) — update method-count assertion to 11
   and add absent-id assertions for the deleted methods.

### Commands

1. `cargo test -p worldwake-ai htn::registry`
2. `cargo test -p worldwake-ai --test golden_ai htn_methods`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh` (before PR)
