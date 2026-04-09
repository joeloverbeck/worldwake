# AIDECREG-002: Reassess and fix `golden_witnessed_theft_accusation_chain`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — reassessment shows a stale golden setup around listener communication acceptance
**Deps**: AIDECREG-001

## Problem

After `AIDECREG-001` fixed `golden_blocked_intent_memory_with_ttl_expiry`, the next broader `cargo test -p worldwake-ai` run exposed a different real failure in `crates/worldwake-ai/tests/golden_emergent.rs::golden_witnessed_theft_accusation_chain`. The failing assertion is `authority should learn the theft through Tell`. This now blocks honest same-crate full-suite verification.

## Assumption Reassessment (2026-04-09)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`, so it is not a broad-suite artifact.
2. The failing proof surface lives in `crates/worldwake-ai/tests/golden_emergent.rs` and currently expects the authority to receive a witnessed theft report through a committed `tell` action from the witness before accusation/punishment follow-through.
3. Archived ticket `archive/tickets/E17CRITHEJUS-013.md` and archived review `archive/tickets/completed/GOLDE2E-014-ordering-contracts-for-mixed-layer-goldens.md` both describe this golden as previously passing, so the live failure is either runtime drift in the social-report / tell / accusation chain or a stale golden setup.
4. The live boundary under audit is mixed-layer: theft social observation generation, tell candidate/execution, authority belief internalization, and the golden’s ordering/assertion surface around learning the theft via `Tell`.
5. `commit_tell()` in `crates/worldwake-systems/src/tell_actions.rs` gates social-observation transfer through the listener's `CommunicationProfile`, and `SuspectedTheft` is classified as `CommunicationClass::Testimony` in `crates/worldwake-core/src/communication.rs`.
6. The current golden leaves the authority listener on the default `CommunicationProfile`, whose `testimony_acceptance` is `800`, but the scenario asserts guaranteed tell-mediated learning on seed `[63; 32]`. Nearby same-file tell goldens already pin listener communication acceptance explicitly when the proof requires deterministic delivery.
7. Safe correction: this is stale setup/proof, not a demonstrated production regression. The owned fix is to make the listener-side communication acceptance explicit in `crates/worldwake-ai/tests/golden_emergent.rs` so the test proves the tell/accusation ordering contract rather than seed-sensitive testimony rejection.

## Architecture Check

1. A bounded reassessment ticket is cleaner than treating the newly exposed emergent golden as incidental fallout.
2. The cleanest fix is a golden-only setup correction that installs explicit listener communication acceptance for the deterministic tell-mediated proof, matching the established pattern used by adjacent tell goldens in the same file.

## Verification Layers

1. Witness forms the theft social observation -> authoritative belief/social-observation state inside `golden_witnessed_theft_accusation_chain`
2. Listener-side testimony acceptance is fixed to deterministic acceptance -> scenario setup proof in `golden_witnessed_theft_accusation_chain`
3. Witness successfully tells the authority about the theft -> action trace (`tell` lifecycle and target/topic detail)
4. Authority internalizes the reported theft -> authoritative belief store / violation memory
5. Golden accusation-chain contract remains valid -> `golden_witnessed_theft_accusation_chain`

## What to Change

### 1. Reassess the failing golden against live code

- Name the exact tell/social-observation/internalization symbols under audit.
- Determine whether the current failure is stale setup, stale proof surface, or a production regression.

### 2. Land the smallest honest fix

- Update the golden setup to give the authority an explicit accepting `CommunicationProfile` so testimony delivery is deterministic.
- Keep the existing tell/action-trace and authority-internalization assertions intact.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify) and/or the exact owning production files revealed by reassessment

## Out of Scope

- Further work on `golden_blocked_intent_memory_with_ttl_expiry`
- Broad emergent-suite cleanup unrelated to this theft/tell chain
- S76 observer-gap documentation work

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The final fix preserves the honest tell-mediated theft-report contract rather than papering over the failure
2. The scenario no longer depends on seed-sensitive default testimony rejection when asserting guaranteed tell delivery
3. If the golden changes, its ordering/proof surface matches the live causal boundary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_witnessed_theft_accusation_chain` — repaired broader-suite blocker with explicit listener communication acceptance
2. `None` — reassessment showed no production contradiction requiring additional lower-layer coverage

### Commands

1. `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Reassessed the failing theft/tell golden against the live tell path and confirmed the root cause was stale scenario setup, not a production regression.
- Updated `crates/worldwake-ai/tests/golden_emergent.rs` so the authority listener now has an explicit accepting `CommunicationProfile`, making testimony delivery deterministic for the scenario's tell-mediated proof.
- Preserved the existing action-trace and authority-internalization assertions; only the listener-side setup changed.
- Reran the broader `worldwake-ai` suite honestly after the fix. The original blocker is gone, but the rerun now exposes a different unrelated failing trade golden.

## Verification Result

- Passed `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`
- Failed broader `cargo test -p worldwake-ai` on an unrelated newly exposed blocker: `crates/worldwake-ai/tests/golden_trade.rs::golden_trade_rejection_reroutes_to_reliable_seller`
- Confirmed the unrelated broader blocker in isolation with `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
