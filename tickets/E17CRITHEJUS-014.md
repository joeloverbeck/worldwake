# E17CRITHEJUS-014: Workspace verification and documentation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: E17CRITHEJUS-001 through E17CRITHEJUS-016 (all implementation and golden tickets)

## Problem

After all E17 implementation tickets are complete, a final workspace-wide verification pass is needed to confirm all tests pass, no clippy warnings exist, golden coverage docs are current, and `IMPLEMENTATION-ORDER.md` reflects E17 completion.

## Assumption Reassessment (2026-03-26)

1. `specs/IMPLEMENTATION-ORDER.md` tracks completion status. E17 is listed under Step 13 with dependencies on E15, S01, S03, E16c, S27 (all completed).
2. Golden coverage documentation exists and must be updated with new E17 golden scenarios.
3. This is a documentation-and-verification-only ticket.
4. N/A — no AI regression.
5. N/A.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch: a separate shared investigate/runtime contradiction remains outside E17-specific crime scope. `tickets/S27INVPROF-001.md` tracks the `ViolationDispositionProfile` requirement cleanup for profile-less investigate fallback/resolution behavior. This ticket should verify the E17 deliverables as implemented, not silently absorb that unrelated shared-runtime change.
12. N/A.

## Architecture Check

1. Final verification ensures no regressions across the entire workspace. Documentation updates ensure the project's implementation tracking remains accurate.
2. No backwards-compatibility aliasing.

## Verification Layers

1. `cargo test --workspace` passes -> full regression confirmation
2. `cargo clippy --workspace` has no new warnings -> code quality confirmation
3. `IMPLEMENTATION-ORDER.md` updated -> documentation accuracy
4. Golden coverage docs updated -> test documentation accuracy
5. Single-layer ticket; additional mapping not applicable.

## What to Change

### 1. Run full workspace verification

- `cargo test --workspace` — all tests pass
- `cargo clippy --workspace` — no new warnings
- `cargo build --workspace` — clean build

### 2. Update `specs/IMPLEMENTATION-ORDER.md`

- Mark E17 as COMPLETED in the dependency graph
- Update Step 13 status
- Update Phase 3 gate checkboxes (T25: Unseen crime discovery, etc.)
- Update active spec inventory (strikethrough E17)

### 3. Update golden coverage documentation

- Add new E17 golden scenarios to whatever golden inventory/docs exist
- Ensure scenario numbering is consistent

### 4. Phase 3 gate assessment

Verify the E17-specific Phase 3 gate items:
- [ ] Crimes discovered through defined pathways (witness observation, inventory violation, possession sighting)
- [ ] No omniscient NPCs (theft at empty location remains unknown until discovery)
- [ ] Causal chains from crime -> discovery -> accusation -> punishment are fully traceable

## Files to Touch

- `specs/IMPLEMENTATION-ORDER.md` (modify)
- Golden coverage docs (modify — location depends on current project structure)

## Out of Scope

- Implementing any code changes (this is verification-only)
- Archiving the E17 spec (separate archival workflow per `docs/archival-workflow.md`)
- Starting E19 or other E17-dependent work
- Phase 3 gate sign-off for non-E17 items (OmniscientBeliefView removal, FND-02 closure)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — zero failures
2. `cargo clippy --workspace` — zero new warnings
3. `cargo build --workspace` — clean build

### Invariants

1. All existing golden tests still pass (no regressions from E17 changes)
2. `IMPLEMENTATION-ORDER.md` accurately reflects E17 completion
3. Golden coverage docs account for all new E17 scenarios
4. E17 spec in `specs/` is ready for archival (but not archived in this ticket)

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. `cargo build --workspace`
