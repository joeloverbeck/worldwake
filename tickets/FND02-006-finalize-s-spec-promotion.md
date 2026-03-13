# FND02-006: Finalize S01-S06 Spec Promotion

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — spec-only changes
**Deps**: Phase 2 complete, FND02-004 (dampening audit provides patterns for Section H)

## Problem

Six DRAFT specs were renamed from `DRAFT-*.md` to `S01-S06` and integrated into `specs/IMPLEMENTATION-ORDER.md` as part of FND-02 creation. However, finalization is incomplete:

1. All six specs still have `**Status**: DRAFT` — should be updated to reflect their formal status.
2. Section H analyses (information-path, positive-feedback, dampeners, stored vs derived) may be missing — required per CLAUDE.md Spec Drafting Rules.
3. Explicit dependency declarations may need verification against the implementation order.
4. Phase placement should be explicitly stated in each spec header.

## Assumption Reassessment (2026-03-13)

1. S01-S06 files exist at `specs/S0{1-6}-*.md` — confirmed.
2. No `DRAFT-*.md` files remain in `specs/` — needs verification.
3. All six appear in `IMPLEMENTATION-ORDER.md` active spec inventory — confirmed.
4. S01 lacks Section H analysis — confirmed by exploration. Need to check S02-S06.
5. Status in all files is `DRAFT` — confirmed.

## Architecture Check

1. Spec-only changes — no code impact. Ensures all specs meet the quality bar before implementation begins.
2. No backwards-compatibility shims — updating specs to meet standards.

## What to Change

### 1. Verify no DRAFT files remain

Confirm `specs/DRAFT-*.md` files no longer exist. If any do, rename via `git mv`.

### 2. Update status in each S-spec

Change `**Status**: DRAFT` to `**Status**: APPROVED` (or appropriate status) in:
- `specs/S01-production-output-ownership-claims.md`
- `specs/S02-goal-decision-policy-unification.md`
- `specs/S03-planner-target-identity-and-affordance-binding.md`
- `specs/S04-merchant-selling-market-presence.md`
- `specs/S05-merchant-stock-storage-and-stalls.md`
- `specs/S06-commodity-opportunity-valuation.md`

### 3. Add phase placement to each spec header

Each spec must declare its phase and step:
- S01, S02, S03: Phase 3, Step 10 (parallel after E14)
- S04, S05, S06: Phase 4+, Step 14 (economy deepening)

### 4. Verify dependency declarations

Each spec must explicitly list its dependencies:
- S01: E14
- S02: E14
- S03: E14
- S04: E14
- S05: S04, S01
- S06: S04

### 5. Add FND-01 Section H analysis to each spec (if missing)

For each spec that lacks it, add:
- **Information-path analysis**: How information reaches agents in this system.
- **Positive-feedback analysis**: Amplifying loops.
- **Concrete dampeners**: Physical mechanisms.
- **Stored state vs. derived read-model list**: What is authoritative vs. transient.

## Files to Touch

- `specs/S01-production-output-ownership-claims.md` (modify)
- `specs/S02-goal-decision-policy-unification.md` (modify)
- `specs/S03-planner-target-identity-and-affordance-binding.md` (modify)
- `specs/S04-merchant-selling-market-presence.md` (modify)
- `specs/S05-merchant-stock-storage-and-stalls.md` (modify)
- `specs/S06-commodity-opportunity-valuation.md` (modify)

## Out of Scope

- Do NOT implement any S-spec code — these are spec amendments only.
- Do NOT modify `specs/IMPLEMENTATION-ORDER.md` — S-specs are already listed there.
- Do NOT modify E14 spec — that is FND02-001.
- Do NOT change the spec content beyond status, placement, dependencies, and Section H.
- Do NOT restructure spec document organization.

## Acceptance Criteria

### Tests That Must Pass

1. No files matching `specs/DRAFT-*.md` exist.
2. Six files `specs/S01-*.md` through `specs/S06-*.md` exist.
3. Each S-spec has status updated from DRAFT.
4. Each S-spec has explicit dependency declarations.
5. Each S-spec has phase and step placement.
6. Each S-spec has FND-01 Section H analysis (information-path, positive-feedback, dampeners, stored vs derived).
7. All S-specs appear in `specs/IMPLEMENTATION-ORDER.md` active spec inventory.

### Invariants

1. No `f32`, `f64`, `HashMap`, or `HashSet` in any authoritative state definitions within the specs.
2. All cross-references to other specs are accurate.
3. Dependency declarations consistent with `specs/IMPLEMENTATION-ORDER.md`.
4. Spec content integrity — Section H additions must not contradict existing spec content.

## Test Plan

### New/Modified Tests

1. No code tests — spec-only changes.

### Commands

1. `ls specs/DRAFT-*.md 2>/dev/null` — verify no DRAFT files remain
2. `ls specs/S0{1,2,3,4,5,6}-*.md` — verify all S-specs exist
3. `grep -l "Status.*DRAFT" specs/S0*.md` — verify no specs still marked DRAFT
4. `grep -l "Section H\|Information-path\|Positive-feedback\|dampener\|Stored state" specs/S0*.md` — verify Section H present in all
