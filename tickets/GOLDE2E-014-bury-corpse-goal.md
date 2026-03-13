# GOLDE2E-014: BuryCorpse Goal

**Status**: PENDING
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: Likely — `BuryCorpse` goal generation, bury action, and burial site mechanics may be incomplete
**Deps**: Death cascade proven in scenario 8

## Problem

`GoalKind::BuryCorpse` is listed in the goal enum but has never been exercised end-to-end. This requires a corpse, a burial site, and an agent motivated to bury. The complete feature — candidate generation, action definition, handler, and burial site mechanics — is untested.

## Report Reference

Backlog item **P16** in `reports/golden-e2e-coverage-analysis.md` (Tier 3, composite score 1).

## Assumption Reassessment (2026-03-13)

1. `GoalKind::BuryCorpse` exists in `worldwake-core/src/goal.rs`.
2. A `bury` action definition and handler may or may not exist — verify.
3. Burial site mechanics (place tag, container, or special entity) may be unimplemented.
4. `BuryCorpse` candidate generation may be unimplemented in `candidate_generation.rs`.

## Architecture Check

1. Burial should follow the same pattern as other care/loot actions: corpse detection → goal generation → action execution.
2. Burial sites should be tagged places or workstations, not special-case entities.
3. The buried corpse should become inaccessible to further loot actions.

## Engine-First Mandate

If implementing this e2e suite reveals that `BuryCorpse` goal generation, bury action definition/handler, or burial site mechanics are missing or architecturally unsound — do NOT create a minimal hack. Instead, design and implement a comprehensive architectural solution that makes corpse burial clean, robust, and extensible within the existing action framework and place graph. Document any engine changes in the ticket outcome.

## What to Change

### 1. Verify/implement bury infrastructure

- `BuryCorpse` candidate generation in `candidate_generation.rs`
- `bury` action definition in the action def registry
- Bury action handler in `worldwake-systems`
- Burial site mechanics (place tag or equivalent)

### 2. New golden test in `golden_combat.rs`

**Setup**: A corpse exists at a location with a burial site. A living agent is co-located and motivated to bury (social relation, community duty, or similar driver).

**Assertions**:
- Agent generates `BuryCorpse` goal from the local corpse + burial site.
- Agent executes the bury action through the real AI loop.
- Corpse becomes buried (inaccessible to loot).
- Conservation holds.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify, if `BuryCorpse` generation missing)
- `crates/worldwake-systems/src/` (new or modify, bury action handler)
- `crates/worldwake-sim/src/action_def_registry.rs` (modify, if bury action def missing)
- Engine files TBD based on what infrastructure is missing

## Out of Scope

- Mass burial or burial ceremonies
- Burial-site capacity limits
- Corpse decay mechanics

## Acceptance Criteria

### Tests That Must Pass

1. `golden_bury_corpse` — agent buries a corpse at a burial site through the real AI loop
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Buried corpse is inaccessible to loot
3. Conservation holds (burial does not destroy items — they transfer to the burial site or become inaccessible)
4. GoalKind coverage increases: `BuryCorpse` → Yes

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update GoalKind coverage: `BuryCorpse` → Yes
- Remove P16 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_bury_corpse` — proves corpse burial path

### Commands

1. `cargo test -p worldwake-ai golden_bury_corpse`
2. `cargo test --workspace && cargo clippy --workspace`
