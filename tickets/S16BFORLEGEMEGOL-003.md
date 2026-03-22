# S16BFORLEGEMEGOL-003: Suite 11 — Force Claim Creates Hostility Witnessed and Propagated

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: S16BFORLEGEMEGOL-001 (shared helpers), S16b spec Suite 11

## Problem

No test proves the FULL chain: force claim → hostility creation (side effect) + institutional belief projection → physical transport by witness → remote Tell propagation. Existing tests (Suite 5: pre-seeds hostility; Scenario 21: tests uncontested belief but not hostility creation) leave this cross-system emergence unproven.

## Assumption Reassessment (2026-03-22)

1. `press_force_claim` handler creates `hostile_to(B, A)` relation as side effect — spec references `office_actions.rs:820`. This is the key cross-system side effect being tested. Verified: `add_hostility` in Suite 5 is pre-seeded, NOT emergent; this suite proves the emergent path.
2. `force_control_claims_for_event()` at perception.rs projects `ForceControl` institutional claim from witnessed political events — spec references `perception.rs:491-563`. C perceives the event because C is co-located with the press_force_claim action.
3. `GoalKind::ShareBelief { listener, subject }` is the goal kind for social Tell — confirmed by Suite 6/7/9 in `golden_emergent.rs`.
4. `InstitutionalBeliefKey::ForceControllerOf { office }` is the institutional belief key — confirmed at `golden_offices.rs:276` and `golden_harness/mod.rs` imports.
5. Scenario agents: A (Incumbent) human-controlled office_holder; B (Challenger) human-controlled issues PressForceClaim; C (Witness) AI social_weight=pm(600) low enterprise_weight; D (Remote Listener) passive at BanditCamp. C and D have perception + tell profiles.
6. C's AI generates `ShareBelief` goal driven by social_weight=pm(600). C has no political ambition (low enterprise_weight).
7. BanditCamp exists as `PrototypePlace::BanditCamp` — verified in Suite 8 at `golden_emergent.rs:2450`.
8. Scenario isolation: Only one force claim (no contested state). D is passive. Focus is hostility side effect + belief propagation, not contest resolution.
9. `hostile_to` relation: checked via `h.world.is_hostile(subject, target)` or event log delta with `RelationValue::HostileTo`.
10. D's institutional belief transitions from absent/Unknown to populated with force-control knowledge after C's tell.

## Architecture Check

1. Follows established Suite 6/7 pattern for social Tell propagation tests, extended with force-control-specific setup.
2. No backward-compatibility shims.

## Verification Layers

1. Hostility creation → authoritative state: `h.world.is_hostile(B, A)` after `press_force_claim` commits
2. Action ordering → action trace: B's `press_force_claim` committed, then C's `travel`, then C's `tell` to D
3. C's belief acquisition → institutional belief read: C's belief store has `ForceControllerOf { office }` with controller=B after perception
4. D's belief before tell → negative assertion: D has no force-control institutional belief before C's tell
5. D's belief after tell → institutional belief read: D's belief store has `ForceControllerOf { office }` with controller=B
6. C's AI motivation → decision trace: C generates `ShareBelief` candidate containing the office entity
7. Determinism → replay companion

## What to Change

### 1. Add `run_force_claim_creates_hostility_witnessed_and_propagated` function to `golden_emergent.rs`

**Setup (Phase 1 — Force claim)**:
- Seed force-law office ("War Chief") at VILLAGE_SQUARE, succession_period=5, no eligibility rules
- Agent A ("Incumbent"): human-controlled, installed as `office_holder` at VILLAGE_SQUARE. Perception profile.
- Agent B ("Challenger"): human-controlled, at VILLAGE_SQUARE. Issue `PressForceClaim` via human input.
- Agent C ("Witness"): AI-controlled, social_weight=pm(600), low enterprise_weight, at VILLAGE_SQUARE. Perception profile. Tell profile (`focused_accepting_tell_profile`). Entity beliefs about A, B, office.
- Agent D ("Remote Listener"): at BanditCamp. Perception profile. Accepting tell profile. No institutional belief about office.
- Enable action tracing, decision tracing, politics tracing.

**Phase 1 tick loop** (~20 ticks):
- Issue B's PressForceClaim input.
- Wait for `press_force_claim` to commit.
- Assert hostility: `h.world.is_hostile(B, A)` is true.
- Assert C's institutional belief contains ForceControl with controller=B.

**Phase 2 tick loop** (~60 ticks):
- C should travel to BanditCamp (seed C's belief about D at BanditCamp, or about the remote place).
- C should Tell D about the office.

**Phase 3 assertions**:
- D's institutional belief contains ForceControllerOf with controller=B.
- D had no force-control belief before C's tell.

### 2. Add main test and replay companion

Following the standard pattern.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — add Suite 11: ~150 lines)

## Out of Scope

- Any engine/production code changes
- Modifications to press_force_claim handler (hostility creation is already implemented)
- Contested force state (that's Suite 12)
- Testing perception itself (E14 coverage)
- Testing Tell mechanism itself (E16c coverage)
- Changes to `golden_harness/mod.rs` beyond what S16BFORLEGEMEGOL-001 provides

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_force_claim_creates_hostility_witnessed_and_propagated` — new test passes
2. `cargo test -p worldwake-ai golden_force_claim_creates_hostility_witnessed_and_propagated_replays_deterministically` — replay companion passes
3. `cargo test -p worldwake-ai --test golden_emergent` — all existing emergent tests still pass
4. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Append-only event log — no mutation of existing events
2. Determinism — same seed produces identical world and event log hashes
3. Hostility relation is EMERGENT from the press_force_claim action, NOT pre-seeded via `add_hostility` harness helper
4. D's belief arrives ONLY via Tell from C (PerceptionSource::Report), not via direct observation or injection
5. C's travel to BanditCamp is causally necessary — D cannot learn the belief without physical carrier

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_force_claim_creates_hostility_witnessed_and_propagated` — proves force-claim→hostility+belief→travel→Tell chain
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_force_claim_creates_hostility_witnessed_and_propagated_replays_deterministically` — determinism companion

### Commands

1. `cargo test -p worldwake-ai golden_force_claim_creates_hostility_witnessed_and_propagated`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo clippy -p worldwake-ai`
