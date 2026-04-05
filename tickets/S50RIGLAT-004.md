# S50RIGLAT-004: Justice candidate generation with jurisdiction gating + golden test

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate generation logic, golden E2E test
**Deps**: S50RIGLAT-002, S50RIGLAT-003

## Problem

Justice-system candidate generation (`emit_recorded_violation_candidates`) does not distinguish between lawful enforcement (guard with jurisdictional authority at the current place) and unlawful force. A guard outside their jurisdiction should not generate `PunishAccused` candidates. This ticket uses `believed_rights()` to gate justice candidates by `JurisdictionalAuthority` and proves it with a golden E2E test.

## Assumption Reassessment (2026-04-05)

1. `emit_recorded_violation_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` emits `GoalKind::Accuse`, `GoalKind::PunishAccused`, `GoalKind::InvestigateViolation`. Verified via earlier Explore agent.
2. `GoalKind::PunishAccused { office, accused, accusation_entry, punishment }` at `crates/worldwake-core/src/goal.rs`. The `office` field is the authority source. Verified.
3. `believed_rights()` (from ticket 003) will be available on `GoalBeliefView` when this ticket executes.
4. `OfficeData.jurisdiction: BTreeSet<EntityId>` (from ticket 002) will be the live type when this ticket executes.
5. Current golden political/justice tests in `golden_offices.rs`, `golden_emergent.rs`, `golden_social.rs` do not test jurisdiction-gating of punishment. Verified — no existing test places a guard outside jurisdiction.
6. `emit_recorded_violation_candidates()` currently checks office holding and accusation records but does NOT check jurisdiction. The candidate is emitted if the agent holds the relevant office, regardless of location.

## Architecture Check

1. Jurisdiction gating is a precondition filter in candidate generation — the most appropriate layer. It prevents unlawful enforcement goals from being generated, rather than letting them fail at action validation. This is cleaner because it avoids wasted planning cycles.
2. The gate uses `believed_rights()` (belief-facing), not authoritative `effective_rights()` — consistent with P14 (agents plan from beliefs, never world state).
3. No backward-compatibility shim. The old behavior (no jurisdiction check) is replaced.
4. The golden test explicitly proves the gate: same agent, same violation, but different locations → punishment generated in-jurisdiction, not generated outside-jurisdiction.

## Verification Layers

1. Jurisdiction gating suppresses `PunishAccused` outside jurisdiction → decision trace showing candidate omitted
2. Jurisdiction gating allows `PunishAccused` inside jurisdiction → decision trace showing candidate emitted
3. Full punishment lifecycle with jurisdiction → golden E2E test (action trace + event-log delta + authoritative world state)
4. Deterministic replay → golden replay companion test

## What to Change

### 1. Add jurisdiction gate to emit_recorded_violation_candidates()

In `crates/worldwake-ai/src/candidate_generation.rs`, in the section that emits `PunishAccused` candidates:

```rust
// Before emitting PunishAccused, check jurisdiction
let agent_place = view.effective_place(agent)?;
let has_jurisdiction = view.believed_rights(agent, accused)
    .iter()
    .any(|r| r.kind == RightKind::JurisdictionalAuthority);

// Only emit PunishAccused if agent has jurisdictional authority
if !has_jurisdiction {
    // Record omission in diagnostics if tracing enabled
    continue; // or return without emitting
}
```

The exact integration point depends on the current structure of the emission loop. The key invariant: `PunishAccused` is only emitted when the acting agent holds an office with jurisdiction over the accused's location.

### 2. Add golden E2E test: jurisdiction-gated punishment

In `crates/worldwake-ai/tests/golden_offices.rs` (or `golden_emergent.rs` if cross-system):

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

### 3. Add deterministic replay companion

Standard replay companion test verifying save/load at a mid-point produces identical state hashes.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add jurisdiction gate to PunishAccused emission)
- `crates/worldwake-ai/tests/golden_offices.rs` or `golden_emergent.rs` (modify — add jurisdiction-gated punishment scenario)

## Out of Scope

- Jurisdiction gating for `Accuse` or `InvestigateViolation` (these may have different locality rules — defer to a follow-up if needed)
- Multi-place jurisdiction scenario content beyond the golden test
- Changing action-level preconditions for punishment (this gates at candidate generation)
- Applying jurisdiction checks to non-justice goals

## Acceptance Criteria

### Tests That Must Pass

1. Golden: guard inside jurisdiction generates `PunishAccused` and executes punishment
2. Golden: guard outside jurisdiction does NOT generate `PunishAccused`
3. Golden: deterministic replay companion for jurisdiction-gated scenario
4. All existing golden tests: `cargo test -p worldwake-ai`
5. Existing suite: `cargo test --workspace`

### Invariants

1. `PunishAccused` is only emitted when the acting agent holds an office with jurisdiction containing the accused's believed location
2. Existing punishment behavior is preserved for guards already at their jurisdiction place (single-place offices wrapped in BTreeSet behave identically to the old single-EntityId)
3. No golden hash changes for scenarios where guard is already at jurisdiction place

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs` — new scenario: jurisdiction-gated punishment (positive + negative + replay)

### Commands

1. `cargo test -p worldwake-ai -- jurisdiction`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
