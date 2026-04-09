# AIDECREG-002: Reassess and fix `golden_witnessed_theft_accusation_chain`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — exact owning layer depends on reassessment of the tell/social-observation path
**Deps**: AIDECREG-001

## Problem

After `AIDECREG-001` fixed `golden_blocked_intent_memory_with_ttl_expiry`, the next broader `cargo test -p worldwake-ai` run exposed a different real failure in `crates/worldwake-ai/tests/golden_emergent.rs::golden_witnessed_theft_accusation_chain`. The failing assertion is `authority should learn the theft through Tell`. This now blocks honest same-crate full-suite verification.

## Assumption Reassessment (2026-04-09)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`, so it is not a broad-suite artifact.
2. The failing proof surface lives in `crates/worldwake-ai/tests/golden_emergent.rs` and currently expects the authority to receive a witnessed theft report through a committed `tell` action from the witness before accusation/punishment follow-through.
3. Archived ticket `archive/tickets/E17CRITHEJUS-013.md` and archived review `archive/tickets/completed/GOLDE2E-014-ordering-contracts-for-mixed-layer-goldens.md` both describe this golden as previously passing, so the live failure is either runtime drift in the social-report / tell / accusation chain or a stale golden setup.
4. The live boundary under audit is mixed-layer: theft social observation generation, tell candidate/execution, authority belief internalization, and the golden’s ordering/assertion surface around learning the theft via `Tell`.

## Architecture Check

1. A bounded reassessment ticket is cleaner than treating the newly exposed emergent golden as incidental fallout.
2. The ticket should fix the earliest concrete contradiction: stale setup/proof if the social-report path is still lawful, or production behavior if the tell/internalization chain has regressed.

## Verification Layers

1. Witness forms the theft social observation -> authoritative belief/social-observation state and/or focused lower-layer proof
2. Witness successfully tells the authority about the theft -> action trace (`tell` lifecycle and target/topic detail)
3. Authority internalizes the reported theft -> authoritative belief store / violation memory
4. Golden accusation-chain contract remains valid -> `golden_witnessed_theft_accusation_chain`

## What to Change

### 1. Reassess the failing golden against live code

- Name the exact tell/social-observation/internalization symbols under audit.
- Determine whether the current failure is stale setup, stale proof surface, or a production regression.

### 2. Land the smallest honest fix

- If the golden setup or assertion surface is stale, update it to match the live social-report contract.
- If production behavior regressed, fix the earliest concrete layer and keep the golden honest.

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
2. If the golden changes, its ordering/proof surface matches the live causal boundary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_witnessed_theft_accusation_chain` — repaired broader-suite blocker
2. Additional focused lower-layer tests only if reassessment proves the current golden lacks enough provenance

### Commands

1. `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
