# FND02-006: Finalize S01-S06 Spec Promotion

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None - spec-only changes
**Deps**: Phase 2 complete

## Problem

The rename and promotion from `DRAFT-*.md` to `S01-S06` already happened, and `specs/IMPLEMENTATION-ORDER.md` already records FND02-006 as done. The remaining work is narrower:

1. All six promoted specs still carry `**Status**: DRAFT`, which no longer matches their scheduled `S01-S06` identities in the active implementation order.
2. Several promoted specs still link to old `DRAFT-*.md` filenames that no longer exist.
3. Phase/step language and dependency headers are not consistently aligned with `specs/IMPLEMENTATION-ORDER.md`.

This ticket should finish metadata and cross-reference cleanup, not re-do the original promotion.

## Assumption Reassessment (2026-03-14)

1. `specs/S01-*.md` through `specs/S06-*.md` exist - confirmed.
2. No `specs/DRAFT-*.md` files remain - confirmed.
3. All six promoted specs already appear in `specs/IMPLEMENTATION-ORDER.md` active spec inventory - confirmed.
4. All six promoted specs already include `FND-01 Section H` analyses - confirmed.
5. All six promoted specs still use `**Status**: DRAFT` - confirmed.
6. Multiple promoted specs still contain stale links to removed `DRAFT-*.md` filenames - confirmed.
7. The active implementation order says FND02-006 was "already done as part of FND-02 creation"; therefore this ticket must be treated as cleanup of stale metadata and references, not as initial promotion work.

## Architecture Check

1. Spec-only changes - no runtime architecture change, but they remove documentation drift that would otherwise mislead future implementation work.
2. Promoting scheduled specs out of `DRAFT` status is architecturally beneficial because it matches the current planning model: these are active roadmap items with explicit dependencies, not loose brainstorms.
3. Fixing stale `DRAFT-*` links is required by the repo's no-backward-compatibility rule. Dead filenames should not survive as documentation aliases.

## What to Change

### 1. Normalize promoted spec status

Update the status in:
- `specs/S01-production-output-ownership-claims.md`
- `specs/S02-goal-decision-policy-unification.md`
- `specs/S03-planner-target-identity-and-affordance-binding.md`
- `specs/S04-merchant-selling-market-presence.md`
- `specs/S05-merchant-stock-storage-and-stalls.md`
- `specs/S06-commodity-opportunity-valuation.md`

Use `**Status**: PENDING`, not `APPROVED`. These are active scheduled specs that have not been implemented yet.

### 2. Normalize phase placement to the implementation order

Each promoted spec must declare the implementation-order placement explicitly:
- S01, S02, S03: Phase 3, Step 10
- S04, S05, S06: Phase 4+, Step 14

### 3. Normalize dependency headers

Each promoted spec's dependency header must include the scheduling dependency from `specs/IMPLEMENTATION-ORDER.md`:
- S01: E14
- S02: E14
- S03: E14
- S04: E14
- S05: S04, S01
- S06: S04

Background references to archived epics or downstream consumers may remain in the body, but the top-level dependency section should match the implementation order.

### 4. Retarget stale cross-references

Replace references to removed `DRAFT-*.md` filenames with the correct `S01-S06` filenames throughout the promoted specs.

## Files to Touch

- `tickets/FND02-006-finalize-s-spec-promotion.md` (modify)
- `specs/S01-production-output-ownership-claims.md` (modify)
- `specs/S02-goal-decision-policy-unification.md` (modify)
- `specs/S03-planner-target-identity-and-affordance-binding.md` (modify)
- `specs/S04-merchant-selling-market-presence.md` (modify)
- `specs/S05-merchant-stock-storage-and-stalls.md` (modify)
- `specs/S06-commodity-opportunity-valuation.md` (modify)

## Out of Scope

- Do NOT implement any S-spec code - these are spec amendments only.
- Do NOT modify `specs/IMPLEMENTATION-ORDER.md` - S-specs are already listed there.
- Do NOT modify E14 spec - that is FND02-001.
- Do NOT rewrite spec bodies beyond status, phase/step placement, dependency normalization, and stale cross-reference repair.
- Do NOT restructure spec document organization.

## Acceptance Criteria

### Tests That Must Pass

1. No files matching `specs/DRAFT-*.md` exist.
2. Six files `specs/S01-*.md` through `specs/S06-*.md` exist.
3. Each promoted S-spec has `**Status**: PENDING`.
4. Each promoted S-spec has explicit phase/step placement aligned with `specs/IMPLEMENTATION-ORDER.md`.
5. Each promoted S-spec has an explicit dependency header aligned with `specs/IMPLEMENTATION-ORDER.md`.
6. No promoted S-spec links to a removed `specs/DRAFT-*.md` filename.
7. All promoted S-specs still include `FND-01 Section H`.
8. `cargo test --workspace` passes.
9. `cargo clippy --workspace` passes.

### Invariants

1. No `f32`, `f64`, `HashMap`, or `HashSet` in any authoritative state definitions within the promoted specs.
2. All cross-references in the promoted specs point to existing spec files.
3. Dependency declarations are consistent with `specs/IMPLEMENTATION-ORDER.md`.
4. Metadata cleanup does not change the substantive architecture described by the specs.

## Test Plan

### New/Modified Tests

1. No new Rust tests expected - this is a spec/documentation cleanup ticket.
2. Verification must still include workspace test and lint runs to ensure the repo remains green.

### Commands

1. `rg --files specs | rg '(^|/)DRAFT-.*\\.md$|(^|/)S0[1-6]-.*\\.md$'`
2. `rg -n '^\\*\\*Status\\*\\*:|## Phase|## Dependencies|## FND-01 Section H' specs/S0{1,2,3,4,5,6}-*.md`
3. `rg -n 'DRAFT-' specs/S0{1,2,3,4,5,6}-*.md`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Reassessed the ticket against the live repository and corrected its stale assumptions first.
  - Updated `specs/S01-*.md` through `specs/S06-*.md` from `**Status**: DRAFT` to `**Status**: PENDING`.
  - Normalized promoted-spec phase/step headers and dependency headers to match `specs/IMPLEMENTATION-ORDER.md`.
  - Repaired stale `DRAFT-*.md` cross-references in the promoted specs so every reference points at current `S01-S06` filenames.
- Deviations from original plan:
  - No Section H work was needed because all six promoted specs already contained `FND-01 Section H`.
  - No file renames were needed because the `DRAFT-*.md` files had already been removed and the `S01-S06` filenames already existed.
  - Status normalization used `PENDING`, not `APPROVED`, because these specs are scheduled future work rather than implemented work.
- Verification results:
  - `rg --files specs | rg '(^|/)DRAFT-.*\\.md$|(^|/)S0[1-6]-.*\\.md$'` showed only the six promoted `S01-S06` files and no live `DRAFT-*.md` files.
  - `rg -n 'DRAFT-' specs/S0{1,2,3,4,5,6}-*.md` returned no matches after cleanup.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
