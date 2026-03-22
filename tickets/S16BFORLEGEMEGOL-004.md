# S16BFORLEGEMEGOL-004: Suite 12 — Contested Force State Propagates Through Belief System

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: S16BFORLEGEMEGOL-001 (shared helpers), S16b spec Suite 12

## Problem

No test proves that the `contested: true` flag from a multi-claimant force office is concrete information that propagates physically through the world. Existing tests (Scenario 20: tests contested state + yield but not belief propagation; Scenario 21: tests only uncontested belief; Suite 11: tests uncontested hostility + belief) leave contested-state belief propagation unproven.

## Assumption Reassessment (2026-03-22)

1. `OfficeForceState.contested_since` is set when 2+ active claimants exist — verified at `offices.rs:436,481` which emit `ForceContested { claimant_count }`.
2. `office_controller` is cleared during contested state — no sole controller when multiple claimants are present. Verified via `golden_offices.rs:2743,2779` assertions.
3. `InstitutionalClaim::ForceControl { office, controller, contested, effective_tick }` includes a `contested: bool` field — confirmed at `golden_offices.rs:278`.
4. `force_control_claims_for_event()` at `perception.rs:551` reads `contested = projection.contested.unwrap_or(false)` — spec reference. When the contested political event fires, the projected claim carries `contested: true`.
5. `ForceContested { claimant_count: 2 }` is a live `OfficeSuccessionOutcome` variant — confirmed at `golden_offices.rs:2825` and `politics_trace.rs:90`.
6. Scenario agents: A + B both human-controlled claimants (deterministic), C (Witness) AI social_weight=pm(600), D (Remote Listener) passive at OrchardFarm.
7. `PrototypePlace::OrchardFarm` exists as `ORCHARD_FARM` constant in `golden_harness/mod.rs:58`.
8. Scenario isolation: Two human-controlled claimants keep contest deterministic. Neither yields. Focus is belief propagation of contested flag, not resolution.
9. `InstitutionalBeliefKey::ForceControllerOf { office }` is the key used to query the contested belief — confirmed at `golden_offices.rs:276`.
10. D's institutional belief should contain `contested == true` after C's Tell. The `InstitutionalClaim::ForceControl.contested` field carries through the Tell mechanism (same claim struct).

## Architecture Check

1. Follows established Suite 6/7/11 pattern: multi-phase test (claim → perception → travel → tell), human-controlled inputs for determinism in the political action, AI-controlled witness for social propagation.
2. No backward-compatibility shims.

## Verification Layers

1. Contested state → authoritative: `office_controller(office) == None` while contested, `OfficeForceState.contested_since.is_some()`
2. Politics trace → `ForceContested { claimant_count: 2 }` after second claim
3. C's belief → institutional belief read: C's ForceControllerOf has `contested == true`
4. Action ordering → action trace: C commits `travel` then `tell` to D
5. D's belief after tell → institutional belief read: D's ForceControllerOf has `contested == true`
6. Negative: D has no force-control belief before C's tell
7. Negative: `office_holder` is NOT set during the contested phase
8. Determinism → replay companion

## What to Change

### 1. Add `run_contested_force_state_propagates_through_belief_system` function to `golden_emergent.rs`

**Setup**:
- Seed force-law office ("War Chief") at VILLAGE_SQUARE, succession_period=5, no eligibility rules
- Agent A ("Claimant Alpha"): human-controlled, at VILLAGE_SQUARE
- Agent B ("Claimant Beta"): human-controlled, at VILLAGE_SQUARE
- Agent C ("Witness"): AI-controlled, social_weight=pm(600), low enterprise_weight, at VILLAGE_SQUARE. Perception profile (institutional_memory_capacity sufficient). Tell profile.
- Agent D ("Remote Listener"): at ORCHARD_FARM. Perception profile. Accepting tell profile. No initial institutional belief about office.
- Enable action tracing, decision tracing, politics tracing.

**Phase 1** (~10 ticks):
- Issue A's PressForceClaim input. Run ticks until A is established as sole controller.
- Issue B's PressForceClaim input. Run ticks.
- Assert: `office_controller == None` (contested), `OfficeForceState.contested_since.is_some()`.
- Assert: politics trace contains `ForceContested { claimant_count: 2 }`.

**Phase 2** (~10 ticks):
- Assert: C's institutional belief for `ForceControllerOf { office }` has `contested == true`.

**Phase 3** (~60 ticks):
- C travels to ORCHARD_FARM. C tells D about the office.
- Assert: D's institutional belief for `ForceControllerOf { office }` has `contested == true`.
- Assert: D had no force-control belief before C's tell.
- Assert: `office_holder` was never set during the contested phase.

### 2. Add main test and replay companion

Following the standard pattern.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — add Suite 12: ~150 lines)

## Out of Scope

- Any engine/production code changes
- Contest resolution mechanics (yield, combat)
- Hostility creation (that's Suite 11)
- Force controller departure (that's Suite 10)
- Modifications to perception or Tell mechanisms
- Changes to `golden_harness/mod.rs` beyond what S16BFORLEGEMEGOL-001 provides

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_contested_force_state_propagates_through_belief_system` — new test passes
2. `cargo test -p worldwake-ai golden_contested_force_state_propagates_through_belief_system_replays_deterministically` — replay companion passes
3. `cargo test -p worldwake-ai --test golden_emergent` — all existing emergent tests still pass
4. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Append-only event log — no mutation of existing events
2. Determinism — same seed produces identical world and event log hashes
3. `contested == true` is concrete information carried through the belief system, not an abstract score
4. D's belief arrives ONLY via Tell from C (PerceptionSource::Report), not via direct observation or injection
5. `office_holder` must NOT be set while the office is contested
6. `office_controller` must be `None` while the office is contested (no sole controller)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_contested_force_state_propagates_through_belief_system` — proves contested flag propagates through perception→travel→Tell
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_contested_force_state_propagates_through_belief_system_replays_deterministically` — determinism companion

### Commands

1. `cargo test -p worldwake-ai golden_contested_force_state_propagates_through_belief_system`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo clippy -p worldwake-ai`
