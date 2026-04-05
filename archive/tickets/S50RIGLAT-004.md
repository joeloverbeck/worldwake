# S50RIGLAT-004: Justice candidate generation with jurisdiction gating + golden test

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate generation logic, golden E2E test
**Deps**: S50RIGLAT-002, S50RIGLAT-003

## Problem

Justice-system punishment candidate generation does not distinguish between lawful enforcement (guard with jurisdictional authority at the current place) and unlawful force. A guard outside their jurisdiction should not generate `PunishAccused` candidates. This ticket uses `believed_rights()` to gate punishment candidates by `JurisdictionalAuthority` and proves it at both the focused AI layer and the existing justice/emergent golden surface.

## Assumption Reassessment (2026-04-05)

1. `emit_recorded_violation_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` delegates to `emit_punishment_candidates()` for `GoalKind::PunishAccused` emission. The owned gate belongs at that helper boundary, not at the outer unresolved-violation loop. Verified this session.
2. `GoalKind::PunishAccused { office, accused, accusation_entry, punishment }` at `crates/worldwake-core/src/goal.rs`. The `office` field is the authority source. Verified.
3. `believed_rights()` (from ticket 003) will be available on `GoalBeliefView` when this ticket executes.
4. `OfficeData.jurisdiction: BTreeSet<EntityId>` (from ticket 002) will be the live type when this ticket executes.
5. Existing punishment goldens already live in `crates/worldwake-ai/tests/golden_emergent.rs`, including accusation-to-punishment chains and punishment-specific decision-trace assertions. The strongest honest golden owner for this contract is that existing suite, not a new or office-only golden by default.
6. `emit_punishment_candidates()` currently checks office holding, consulted accusation records, and lawful fine/exile binding, but does NOT check jurisdiction. The punishment candidate is still emitted if the agent holds the relevant office, regardless of location.

## Architecture Check

1. Jurisdiction gating is a precondition filter in candidate generation — the most appropriate layer. It prevents unlawful enforcement goals from being generated, rather than letting them fail at action validation. This is cleaner because it avoids wasted planning cycles.
2. The gate uses `believed_rights()` (belief-facing), not authoritative `effective_rights()` — consistent with P14 (agents plan from beliefs, never world state).
3. No backward-compatibility shim. The old behavior (no jurisdiction check) is replaced.
4. Focused AI tests are part of the owned proof surface here because `TestBeliefView` must expose `believed_rights()` lawfully for candidate-generation coverage to stay meaningful after ticket 003.
5. The golden test should extend the existing justice/emergent punishment suite: same authority source and same violation family, but different locations → punishment generated in-jurisdiction, not generated outside-jurisdiction.

## Verification Layers

1. Focused candidate-generation test suppresses `PunishAccused` outside jurisdiction
2. Focused candidate-generation test preserves `PunishAccused` inside jurisdiction
3. Full punishment lifecycle with jurisdiction → golden E2E test in the existing justice/emergent suite (decision trace + action trace + authoritative world state)
4. Deterministic replay → golden replay companion test

## What to Change

### 1. Add jurisdiction gate to emit_punishment_candidates()

In `crates/worldwake-ai/src/candidate_generation.rs`, in `emit_punishment_candidates()` before emitting `PunishAccused`:

```rust
let has_jurisdiction = view.believed_rights(agent, accused)
    .iter()
    .any(|r| {
        r.kind == RightKind::JurisdictionalAuthority
            && r.via == Some(office)
    });

// Only emit PunishAccused if agent has jurisdictional authority
if !has_jurisdiction {
    continue; // or return without emitting
}
```

The exact integration point depends on the current helper structure. The key invariant: `PunishAccused` is only emitted when the acting agent believes the same issuing `office` holds `JurisdictionalAuthority` over the accused at the accused's believed location.

### 2. Add focused AI coverage for in-jurisdiction and out-of-jurisdiction punishment candidate generation

In `crates/worldwake-ai/src/candidate_generation.rs` test coverage:

- positive case: `PunishAccused` still emits when `believed_rights()` includes `JurisdictionalAuthority` via the issuing office
- negative case: `PunishAccused` is withheld when office holding exists but `believed_rights()` does not include `JurisdictionalAuthority` via that office
- update the `TestBeliefView` surface as needed so the new belief-facing gate is represented honestly in focused tests

### 3. Add golden E2E test: jurisdiction-gated punishment

In `crates/worldwake-ai/tests/golden_emergent.rs`:

**Scenario setup:**
- Two places: `VILLAGE_SQUARE` (in jurisdiction), `FOREST_PATH` (outside jurisdiction)
- One office with `jurisdiction: BTreeSet::from([VILLAGE_SQUARE])`
- One guard agent holding the office, positioned at `VILLAGE_SQUARE`
- One accused agent at `VILLAGE_SQUARE`
- One accusation record for the accused
- Run ticks → guard generates `PunishAccused` → punishment executes

**Negative case:**
- Same setup but guard at `FOREST_PATH` (outside jurisdiction)
- Run ticks → guard does NOT generate `PunishAccused`
- Assert: no punishment action committed

### 4. Add deterministic replay companion

Standard replay companion test verifying save/load at a mid-point produces identical state hashes.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add jurisdiction gate to PunishAccused emission and focused AI coverage)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — add jurisdiction-gated punishment scenario)

## Out of Scope

- Jurisdiction gating for `Accuse` or `InvestigateViolation` (these may have different locality rules — defer to a follow-up if needed)
- Multi-place jurisdiction scenario content beyond the golden test
- Changing action-level preconditions for punishment (this gates at candidate generation)
- Applying jurisdiction checks to non-justice goals

## Acceptance Criteria

### Tests That Must Pass

1. Focused AI test: in-jurisdiction authority emits `PunishAccused`
2. Focused AI test: out-of-jurisdiction authority does NOT emit `PunishAccused`
3. Golden: guard inside jurisdiction generates `PunishAccused` and executes punishment
4. Golden: guard outside jurisdiction does NOT generate `PunishAccused`
5. Golden: deterministic replay companion for jurisdiction-gated scenario
6. All existing golden tests: `cargo test -p worldwake-ai`
7. Existing suite: `cargo test --workspace`

### Invariants

1. `PunishAccused` is only emitted when the acting agent believes the specific issuing office has jurisdiction containing the accused's believed location
2. Existing punishment behavior is preserved for guards already at their jurisdiction place (single-place offices wrapped in BTreeSet behave identically to the old single-EntityId)
3. No golden hash changes for scenarios where guard is already at jurisdiction place

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused candidate-generation tests for punishment jurisdiction gating
2. `crates/worldwake-ai/tests/golden_emergent.rs` — new scenario: jurisdiction-gated punishment (positive + negative + replay)

### Commands

1. `cargo test -p worldwake-ai -- jurisdiction`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Added a jurisdiction gate in `crates/worldwake-ai/src/candidate_generation.rs` so `emit_punishment_candidates()` only emits `PunishAccused` when `believed_rights()` includes `RightKind::JurisdictionalAuthority` via the same issuing `office`
  - Extended the focused candidate-generation test harness in `crates/worldwake-ai/src/candidate_generation.rs` so `TestBeliefView` can expose `believed_rights()`, then added focused in-jurisdiction preservation and out-of-jurisdiction suppression coverage
  - Added Scenario 110 and its deterministic replay companion in `crates/worldwake-ai/tests/golden_emergent.rs` to prove punishment proceeds in-jurisdiction and is absent outside jurisdiction
  - Refreshed generated golden coverage docs in `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-scenario-map.md`
- **Deviations from original plan**:
  - The golden work stayed on the existing `golden_emergent.rs` ownership surface instead of adding a new office-only golden file
  - The legal contract tightened from generic jurisdiction presence to office-specific rights provenance: the matching `JurisdictionalAuthority` right must be carried via the same issuing `office`
  - CI-matching clippy required a small golden cleanup to remove redundant `.clone()` calls on `InstitutionalClaim`, but no architecture boundary changed
- **Verification results**:
  - `cargo test -p worldwake-ai justice_candidates_emit_fine_punishment_from_consulted_accusation -- --nocapture`
  - `cargo test -p worldwake-ai justice_candidates_do_not_emit_punishment_outside_jurisdiction -- --nocapture`
  - `cargo test -p worldwake-ai golden_jurisdiction_gated_punishment -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace -q`
