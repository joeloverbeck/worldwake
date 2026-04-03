# S43COMTYPDIF-005: Golden tests — stress-filtered communication, class-aware acceptance, alarm propagation

**Status**: COMPLETED
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
5. Scenario A (stress-filtered) is stably exposable at the decision-trace boundary, not necessarily as a same-scenario committed Tell. The honest contract is: a stressed agent with both an alarm-class and gossip-class share candidate keeps the alarm candidate viable while the gossip candidate is suppressed. The committed alarm propagation proof is owned by Scenario C.
6. Scenario B (class-aware acceptance) requires two listeners with different `CommunicationProfile` settings. Speaker tells gossip-class info to both. The skeptical listener rejects; the default listener accepts.
7. Scenario C (alarm relay) requires three agents in a line of places. Agent A witnesses conflict, is under survival stress, tells B (alarm class, not suppressed). B relays to C (source degrades to Report → Testimony class). Prove the alarm reaches C.
8. Scenario isolation (precision rule 8): each scenario targets one specific branch. Competing affordances (eat, flee, trade) should be removed from setup by not providing the enabling profiles or resources, keeping only Tell-relevant infrastructure.
9. For scenario C, the original "alarm degrades to testimony on relay" idea is stale for live S43. `classify_communication()` treats `TellTopic::SocialObservation { WitnessedConflict }` as `Alarm` regardless of source. The honest live contract is: the observation's provenance degrades (`DirectObservation` → `Report` → `Rumor`), but the communication class remains `Alarm`, so stressed intermediaries still relay it.

## Architecture Check

1. Three focused golden scenarios, each proving one specific emergent property. No scenario tries to prove everything at once — this follows the golden test convention of narrow-scope high-confidence scenarios.
2. No production code changes. This ticket is tests-only.

## Verification Layers

1. Alarm survives stress suppression -> decision trace: stressed agent's ShareBelief(Alarm) candidate remains viable (generated or ranked), not suppressed
2. Gossip suppressed under stress -> decision trace: same agent's ShareBelief(Gossip) candidate IS suppressed
3. Class-aware acceptance differentiation -> authoritative world state: skeptical listener's belief store does NOT contain the gossip topic; default listener's does
4. Alarm relay through stressed intermediary -> authoritative world state: agent C's belief store contains the alarm content after relay chain A→B→C
5. Alarm-class resilience across relay -> decision trace + authoritative world state: B relays the conflict observation despite critical stress, while the underlying observation source still degrades lawfully across hops

## What to Change

### 1. Scenario A: Stress-filtered communication

In `crates/worldwake-ai/tests/golden_social.rs` (or a new `golden_communication.rs`):

- Create a stressed speaker with one alarm-class share topic and one gossip-class share topic plus a co-located listener so the social candidate path is live.
- Use the strongest honest assertion surface here: decision traces.
- Assert via decision trace that the alarm-class `ShareBelief` candidate remains viable while the gossip-class one is suppressed under the same stress state.

### 2. Scenario B: Class-aware acceptance

- Create three agents (speaker, default_listener, skeptical_listener) all co-located.
- Give speaker a Rumor-sourced entity belief (→ Gossip class).
- Give default_listener a default CommunicationProfile (gossip_acceptance: 600).
- Give skeptical_listener a CommunicationProfile with gossip_acceptance: Permille(0) (guaranteed rejection).
- Execute Tell to both listeners.
- Assert: default_listener's belief store updated with the gossip topic. skeptical_listener's belief store did NOT update.

### 3. Scenario C: Alarm propagation through a stressed intermediary

- Create three agents (A, B, C) with a relayable `WitnessedConflict` social observation and a setup that prevents A from directly telling C.
- Give A the conflict observation as direct observation and give B critical hunger (stress).
- Tick: A tells B. B receives the same `WitnessedConflict` content with degraded source (`Report { from: A, chain_len: 1 }`).
- Despite critical stress, B still relays the alarm-class observation to C because `ShareBelief(Alarm)` is never suppressed.
- C receives the same conflict observation with further degraded provenance.
- Assert: C's belief store contains the `WitnessedConflict` content, and the stored source degraded lawfully across hops even though the communication class stayed `Alarm`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify) — add scenarios A, B, C (or create new `golden_communication.rs`)

## Out of Scope

- Production code changes — all S43 production changes are in tickets 001–004
- Performance testing or stress testing
- Testing with >3 communication classes

## Acceptance Criteria

### Tests That Must Pass

1. Scenario A: `golden_alarm_survives_stress_suppression` — alarm-class ShareBelief remains viable under stress; gossip-class is suppressed
2. Scenario B: `golden_class_aware_acceptance` — skeptical listener rejects gossip; default listener accepts
3. Scenario C: `golden_alarm_relay_through_stressed_intermediary` — alarm content reaches C via A→B→C relay, and the stressed intermediary still relays it
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. All scenarios use deterministic seeding — replay produces identical results
2. Source degradation on relay: A's DirectObservation → B's Report → C's Rumor
3. Alarm content survives relay even though only provenance degrades — the communication class remains `Alarm` for `WitnessedConflict`

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

## Outcome

- **Completion date**: 2026-04-03
- **What changed**:
  - Added three new S43-specific goldens in `crates/worldwake-ai/tests/golden_social.rs`: `golden_alarm_survives_stress_suppression`, `golden_class_aware_acceptance`, and `golden_alarm_relay_through_stressed_intermediary`.
  - Refreshed generated golden inventory/docs in `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-scenario-map.md`.
- **Deviations from original plan**:
  - Scenario A landed at the decision-trace boundary rather than a same-scenario committed-Tell boundary; this was the strongest honest live proof surface for the stress-filtered alarm-vs-gossip contract.
  - Scenario C was corrected during reassessment: relay degrades the observation's provenance (`DirectObservation` → `Report` → `Rumor`) but does not degrade `CommunicationClass` for `WitnessedConflict`, so the live golden proves alarm-class resilience across relay instead of Alarm→Testimony degradation.
- **Verification results**:
  - `cargo test -p worldwake-ai --test golden_social`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
