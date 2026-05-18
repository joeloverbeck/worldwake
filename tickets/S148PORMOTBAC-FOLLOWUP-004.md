# S148PORMOTBAC-FOLLOWUP-004: Repair survival-preferences familiar-source depletion contract

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - likely `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`, or `crates/worldwake-ai/tests/golden_survival_preferences.rs`
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

`S148PORMOTBAC-FOLLOWUP-003` tightened self-care acquisition admission to concrete evidence and fixed the legal-control boundary for loose item lots. During broad regression, `golden_survival_preferences::survival_preferences_keeps_proactive_diversification_alive_under_survival` still failed, but live diagnostics showed a different root cause: the familiar orchard source did not deplete during the run, so Scout Ilen never had a concrete local depletion event to turn into durable source-reliability failure memory.

The golden's current assertion expects "locally observed familiar-source depletion" to become stored source reliability. That remains a foundation-aligned behavior only if the depletion arises from lawful local world state, agent beliefs, and explicit action contention. It must not be forced by authored outcome logic or hidden ranking patches.

## Assumption Reassessment (2026-05-18)

1. The motivating failing test is `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`.
2. Diagnostic output during `S148PORMOTBAC-FOLLOWUP-003` showed the familiar orchard resource source stayed at `available_quantity: Quantity(1)` through the run, while the agent source-reliability record for the familiar orchard kept `failed_attempts: 0`.
3. The live assertion is therefore not currently proving a failed local expectation; it is expecting an authored branch that the current simulation path does not produce.
4. Per `docs/FOUNDATIONS.md`, the repair must preserve FND-1 local causality, FND-8 explicit contention/duration, FND-14/FND-14A belief/local observation boundaries, FND-17 expectation violation, and FND-22A concrete learning state.
5. The first implementation question is whether the familiar-source branch is supposed to be produced by scenario initial conditions, source preference/ranking, candidate generation, or the golden assertion itself.
6. This ticket owns the survival-preferences contract only. It must not reopen the S148 probe pressure escape or legal-control loose-lot fix.

## Architecture Check

1. A lawful repair must create or observe real source depletion through normal actions, queues, resource quantities, and local observation.
2. If the live scenario no longer reliably exercises familiar-source depletion, update the golden contract or setup so it proves the actual intended invariant instead of relying on a stale branch.
3. Do not add a hidden "try familiar first" rule unless it is explainable from concrete source reliability, route cost, local belief, or agent preference state.

## Verification Layers

1. Familiar-source depletion path -> action trace and authoritative `ResourceSource.available_quantity` around the familiar orchard.
2. Durable learned failure -> `SourceReliability` record for the familiar orchard with `failed_attempts > 0`.
3. Later diversification choice -> decision trace showing familiar orchard discounted and Novel Grove selected for apple acquisition.
4. Survival regression -> golden survival-preferences test remains green without authored outcome triggers.

## What to Change

### 1. Reassess the live golden branch

Inspect the scenario, action trace, decision trace, source reliability state, and resource quantities to determine why the familiar orchard does not deplete under current behavior.

### 2. Repair the correct owner

Choose the narrowest foundation-aligned owner:

- scenario/golden truthing if the assertion no longer matches the intended branch
- candidate/ranking behavior if accessible local familiar evidence should lawfully beat novel evidence under current beliefs
- source-reliability observation if a real local depletion is observed but not persisted

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_preferences.rs` (modify if assertion/setup owns the drift)
- `scenarios/survival-preferences.ron` (modify only if initial conditions are the truthful owner)
- `crates/worldwake-ai/src/candidate_generation.rs` or `crates/worldwake-ai/src/ranking.rs` (modify only if live reassessment proves AI ownership)

## Out of Scope

- Reintroducing pressure-only self-care acquisition probe admission.
- Reintroducing rejected-slot planning bypasses.
- Forcing familiar-source depletion with hidden quest/script logic.
- Changing unrelated survival goldens or observer thresholds.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`
2. Any focused unit or runtime test added for the exact repaired owner.

### Invariants

1. Familiar-source failure memory is backed by a concrete local expectation violation.
2. Any changed preference/ranking behavior is derived from belief, route/source reliability, or concrete local state.
3. The repair does not depend on authored outcome triggers.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_preferences.rs` - keep or repair the golden so it proves the real survival-preferences branch.
2. Focused lower-layer test if production behavior changes.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`
