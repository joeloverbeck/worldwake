# S05MERSTOSTALL-009: Prove facility stock uses the existing audit and investigation path

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: No — focused facility-stock audit proof unless the live path proves incomplete
**Deps**: S05MERSTOSTALL-005, S05MERSTOSTALL-008

## Problem

Facility controllers need a proven path from missing facility stock to `EntityMissing` → `InvestigateViolation` → `SuspectedTheft`. The live code already has a generic expectation-mismatch investigation pipeline, but this ticket must prove that stored/displayed merchant stock actually participates in that path instead of assuming a separate audit subsystem is still missing.

## Assumption Reassessment (2026-04-01)

1. Passive local perception already iterates `entities_effectively_at(place)`, so contained facility stock is locally observable when the controller has line-of-presence at the place.
2. Generic belief-mismatch discovery already emits `EntityMissing` when a previously believed local entity is gone.
3. `InvestigateViolation` and owner-side upgrade to `SuspectedTheft` already exist in the E17 pipeline.
4. The real remaining question is facility-specific proof: whether merchant stock in storage/display containers is actually represented in beliefs strongly enough that the generic path fires after theft.
5. Add engine changes only if that facility-specific proof exposes a real missing belief or perception edge.

## Architecture Check

1. Facility stock audit must reuse the existing expectation-mismatch → investigate → suspected-theft path. Do not introduce a separate audit subsystem if the live generic path already covers contained facility stock.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Contained facility stock is locally observed strongly enough to become a belief subject → perception-focused test
2. Removing previously believed facility stock produces `EntityMissing` through passive local observation → discovery/perception test
3. Missing facility stock emits `InvestigateViolation` through the generic AI violation path → candidate generation test
4. Owner investigation of missing facility stock produces `SuspectedTheft` without a special merchant-only path → focused investigate test or integration proof

## What to Change

### 1. Prove contained facility stock is part of the normal local observation surface

In perception/placement modules: prove that a facility controller can build and later violate beliefs about contained stock/display lots through the ordinary local observation path.

### 2. Prove missing facility stock reuses the generic mismatch path

When previously believed facility stock is stolen or otherwise removed, passive local observation should record `EntityMissing` rather than requiring a separate stock-audit mechanism.

### 3. Prove owner investigation upgrades that facility mismatch into theft suspicion

Use the existing `InvestigateViolation` / `investigate` path and prove that the owner-side missing-stock case still produces `SuspectedTheft` for contained facility stock. Only add engine changes if this proof exposes a real gap.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify tests; production only if proof fails)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify tests; production only if proof fails)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify tests if a facility-specific investigate proof is needed)

## Out of Scope

- Spoilage or degradation of stored goods
- Institutional inventory records (beyond agent beliefs)
- Golden tests for audit scenarios (010)
- Building a separate stock-audit subsystem if the live generic path already works

## Acceptance Criteria

### Tests That Must Pass

1. Previously believed contained facility stock can be observed, then later discovered missing through passive local observation
2. Missing facility stock produces `EntityMissing`
3. Missing facility stock produces `InvestigateViolation`
4. Owner investigation of missing facility stock produces `SuspectedTheft`
5. No merchant-specific parallel audit or investigation path is introduced
6. Existing focused suites stay green

### Invariants

1. Information locality — controller detects missing stock through local observation and stale belief mismatch, not global queries
2. Belief-only planning — investigation still starts from beliefs, not authoritative stock scans
3. E17 pipeline reused — no parallel audit or investigation mechanism

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — contained facility stock participates in passive local observation and missing-entity discovery
2. `crates/worldwake-ai/src/candidate_generation.rs` — missing facility stock emits `InvestigateViolation`
3. `crates/worldwake-systems/src/investigate_actions.rs` — owner investigation of missing facility stock records `SuspectedTheft`

### Commands

1. `cargo test -p worldwake-systems -- perception`
2. `cargo test -p worldwake-systems -- investigate`
3. `cargo test -p worldwake-ai -- investigate`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-01
- What changed:
  - corrected the ticket to the live architectural boundary: facility stock already reused the generic expectation-mismatch and investigation pipeline, so the remaining work was facility-specific proof rather than a new audit subsystem
  - added focused systems coverage in `crates/worldwake-systems/src/perception.rs` proving missing displayed facility stock emits `EntityMissing`
  - added focused systems coverage in `crates/worldwake-systems/src/investigate_actions.rs` proving owner investigation of missing displayed facility stock records `SuspectedTheft`
  - added focused AI coverage in `crates/worldwake-ai/src/candidate_generation.rs` proving missing facility stock emits `InvestigateViolation`
- Deviations from original plan:
  - no production-code changes were needed because the generic audit and investigation path was already implemented
  - the original ticket scope was stale and was narrowed before implementation to a proof-focused facility-stock ticket
- Verification results:
  - `cargo test -p worldwake-systems missing_displayed_facility_stock_emits_entity_missing_discovery -- --nocapture`
  - `cargo test -p worldwake-systems owner_investigating_missing_displayed_facility_stock_records_suspected_theft -- --nocapture`
  - `cargo test -p worldwake-ai missing_facility_stock_emits_investigate_candidate -- --nocapture`
  - `cargo test -p worldwake-systems -- perception`
  - `cargo test -p worldwake-systems -- investigate`
  - `cargo test -p worldwake-ai -- investigate`
  - `cargo clippy --workspace --all-targets -- -D warnings`
