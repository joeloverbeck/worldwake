# S43COMTYPDIF-005: Golden tests — stress-filtered communication, class-aware acceptance, alarm propagation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S43COMTYPDIF-003, S43COMTYPDIF-004

## Problem

The communication type differentiation system (S43) has no end-to-end proof that alarm, testimony, and gossip classes produce different emergent behavior. Focused unit tests (tickets 001–004) verify individual components, but the golden tests prove the full causal chain: candidate generation → class attachment → suppression → ranking → Tell commitment → acceptance. Without these, regressions in class-aware behavior would be undetectable.

## Assumption Reassessment (2026-04-03)

1. After tickets 001–004: `CommunicationClass` exists, `classify_communication()` works, `GoalKind::ShareBelief` carries `communication_class`, goal policy applies class-specific suppression, ranking applies alarm boost, Tell handler uses class-specific acceptance.
2. Golden social tests live at `crates/worldwake-ai/tests/golden_social.rs` — confirmed. This is the natural home for S43 golden tests.
3. Golden test infrastructure: scenarios use `TestHarness` with full action registries, deterministic `ChaCha8Rng` seeding, and tick-by-tick execution with decision trace inspection.
4. `PerceptionProfile` required on agents that need to observe post-production output — golden production tests require this. Also needed here: agents must perceive each other and world events to have beliefs worth sharing.
5. Scenario A (stress-filtered) requires an agent with critical hunger plus both alarm-class and gossip-class beliefs. The agent must have a listener co-located. Suppress the gossip but not the alarm under stress.
6. Scenario B (class-aware acceptance) requires two listeners with different `CommunicationProfile` settings. Speaker tells gossip-class info to both. The skeptical listener rejects; the default listener accepts.
7. Scenario C (alarm relay) requires three agents in a line of places. Agent A witnesses conflict, is under survival stress, tells B (alarm class, not suppressed). B relays to C (source degrades to Report → Testimony class). Prove the alarm reaches C.
8. Scenario isolation (precision rule 8): each scenario targets one specific branch. Competing affordances (eat, flee, trade) should be removed from setup by not providing the enabling profiles or resources, keeping only Tell-relevant infrastructure.
9. For scenario C, the relay chain involves source degradation: A's `DirectObservation` → B receives via Tell → B's source is `Report { from: A, chain_len: 1 }` → B's `classify_communication` for that topic returns `Testimony` (not `Alarm`). This is the intended degradation behavior per spec.

## Architecture Check

1. Three focused golden scenarios, each proving one specific emergent property. No scenario tries to prove everything at once — this follows the golden test convention of narrow-scope high-confidence scenarios.
2. No production code changes. This ticket is tests-only.

## Verification Layers

1. Alarm survives stress suppression -> decision trace: agent under critical stress emits ShareBelief(Alarm) candidate, candidate is NOT suppressed
2. Gossip suppressed under stress -> decision trace: same agent's ShareBelief(Gossip) candidate IS suppressed
3. Class-aware acceptance differentiation -> authoritative world state: skeptical listener's belief store does NOT contain the gossip topic; default listener's does
4. Alarm relay through stressed intermediary -> authoritative world state: agent C's belief store contains the alarm content after relay chain A→B→C
5. Source degradation on relay -> authoritative world state or action trace: B's shared topic is classified as Testimony (source is Report), not Alarm

## What to Change

### 1. Scenario A: Stress-filtered communication

In `crates/worldwake-ai/tests/golden_social.rs` (or a new `golden_communication.rs`):

- Create two agents (speaker, listener) co-located at one place.
- Give speaker critical hunger (HomeostaticNeeds with hunger at maximum depletion) to trigger stress.
- Give speaker a `WitnessedConflict` social observation (→ Alarm) and a `CoPresence` social observation for a third entity (→ Gossip).
- Give speaker a TellProfile, PerceptionProfile, CommunicationProfile, UtilityProfile with social_weight.
- Tick until candidate generation.
- Assert via decision trace: ShareBelief(Alarm) candidate emitted and NOT suppressed. ShareBelief(Gossip) candidate emitted and IS suppressed (or not emitted due to stress suppression).

### 2. Scenario B: Class-aware acceptance

- Create three agents (speaker, default_listener, skeptical_listener) all co-located.
- Give speaker a Rumor-sourced entity belief (→ Gossip class).
- Give default_listener a default CommunicationProfile (gossip_acceptance: 600).
- Give skeptical_listener a CommunicationProfile with gossip_acceptance: Permille(0) (guaranteed rejection).
- Execute Tell to both listeners.
- Assert: default_listener's belief store updated with the gossip topic. skeptical_listener's belief store did NOT update.

### 3. Scenario C: Alarm propagation under stress

- Create three agents (A, B, C) at three places in a line (place1—place2—place3). A and B co-located at place1. B and C co-located at place2 (or B travels to place2 after receiving from A).
- Simpler setup: A and B at place1, B and C at place2. B must travel or be at both — use two-place setup where B is at place1 initially, then moves to place2.
- Give A a `WitnessedConflict` observation (→ Alarm). Give A critical hunger (stress).
- Tick: A tells B (alarm, not suppressed despite stress). B receives it (source degrades to Report).
- Move B to place2 (or B is already there). B now has the belief with Report source. B classifies it as Testimony when sharing.
- B tells C. C receives (Testimony-class acceptance).
- Assert: C's belief store contains the WitnessedConflict observation (original content preserved, source chain extended).

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify) — add scenarios A, B, C (or create new `golden_communication.rs`)

## Out of Scope

- Production code changes — all S43 production changes are in tickets 001–004
- Performance testing or stress testing
- Testing with >3 communication classes

## Acceptance Criteria

### Tests That Must Pass

1. Scenario A: `golden_alarm_survives_stress_suppression` — alarm-class ShareBelief is not suppressed under critical stress; gossip-class is suppressed
2. Scenario B: `golden_class_aware_acceptance` — skeptical listener rejects gossip; default listener accepts
3. Scenario C: `golden_alarm_relay_through_stressed_intermediary` — alarm content reaches C via A→B→C relay with source degradation
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. All scenarios use deterministic seeding — replay produces identical results
2. Source degradation on relay: A's DirectObservation → B's Report → C's Rumor (or Report with chain_len+1)
3. Alarm content survives relay even though classification degrades from Alarm to Testimony — the observation itself is preserved

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` (or `golden_communication.rs`) — Scenario A: stress-filtered alarm vs gossip
2. Same file — Scenario B: class-aware acceptance with skeptical listener
3. Same file — Scenario C: alarm relay chain with source degradation

### Commands

1. `cargo test -p worldwake-ai -- golden_alarm_survives`
2. `cargo test -p worldwake-ai -- golden_class_aware`
3. `cargo test -p worldwake-ai -- golden_alarm_relay`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`
