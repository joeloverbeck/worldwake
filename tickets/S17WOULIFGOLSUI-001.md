# S17WOULIFGOLSUI-001: Golden Scenario 29 — Deprivation Wound Worsening Consolidates Not Duplicates

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S11 (wound lifecycle audit — `worsen_or_create_deprivation_wound`), E12 (wound schema), E09 (deprivation exposure), E13 (decision architecture)

## Problem

The `worsen_or_create_deprivation_wound` consolidation invariant (repeated deprivation fires merge into one wound with preserved `WoundId`, increasing severity, updated `inflicted_at`) is only tested via focused unit tests. No golden E2E test exercises this path through the live needs system dispatch. Existing deprivation goldens (Scenarios 8, 8d, 9d) use fragile agents (`wound_capacity: pm(200)` + pre-existing wounds) that die from ONE fire, never surviving long enough for a second fire to exercise worsening.

## Assumption Reassessment (2026-03-21)

1. `worsen_or_create_deprivation_wound` exists in `crates/worldwake-systems/src/needs.rs` and is called by the needs system during deprivation threshold firing. Focused tests exist in `needs.rs` unit tests. No golden test exercises this path — confirmed by absence in `golden-e2e-coverage.md` and `golden-e2e-scenarios.md`.
2. `DeprivationExposure` component stores per-need critical tick counters and is incremented by the needs system each tick an agent spends above a critical threshold. `starvation_tolerance_ticks` in `MetabolismProfile` controls firing frequency. Both documented in `specs/S11-wound-lifecycle-audit.md` and `docs/FOUNDATIONS.md`.
3. Not an AI regression ticket. The test exercises the needs system dispatch path, not AI decision-making. The agent is idle (no plannable goals) throughout the test window. Full action registries are NOT required for the core invariant, but the golden harness includes them by default for emergent fidelity.
4. No ordering-dependent assertions. The invariant is wound-count stability (always ≤ 1) and wound identity preservation (same `WoundId`), not action sequence ordering.
5. No heuristic removal. This ticket adds coverage for existing behavior.
6. Not a stale-request or start-failure ticket.
7. Not a political office-claim ticket.
8. No ControlSource manipulation. Agent uses default AI control.
9. **Isolation choice**: No food available (no `ConsumeOwnedCommodity` goal), all non-hunger metabolism rates zeroed (no competing needs goals), no other agents (no social/trade/combat), single location (no travel). Agent is idle; only system activity is needs ticking and deprivation firing.
10. No mismatch found.

## Architecture Check

1. Pure test-addition ticket. Uses existing live system stack (needs system dispatch → deprivation exposure → threshold fire → `worsen_or_create_deprivation_wound`). No production code changes.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. `wounds.len() <= 1` every tick → authoritative world state (KEY consolidation invariant)
2. Same `WoundId` after second fire → authoritative world state (wound identity preservation)
3. Higher severity after second fire → authoritative world state (worsening progression)
4. Later `inflicted_at` after second fire → authoritative world state (timestamp update)
5. Agent alive throughout → authoritative world state (`wound_load < wound_capacity`)
6. Deterministic replay → replay companion
7. Single-layer ticket (authoritative state); no action ordering or AI decision assertions needed because agent is idle.

## What to Change

### 1. Add `golden_deprivation_wound_worsening_consolidates_not_duplicates` to `golden_emergent.rs`

**Setup**:
- Single agent at `VILLAGE_SQUARE`
- `HomeostaticNeeds::new(pm(920), pm(0), pm(0), pm(0), pm(0))` — hunger above critical threshold
- Custom `MetabolismProfile`: `starvation_tolerance_ticks: nz(5)`, `hunger_rate: pm(0)`, all other rates `pm(0)`, all other tolerance ticks set high
- `DeprivationExposure` pre-seeded with `hunger_critical_ticks: 4` (1 tick from first fire)
- `CombatProfile`: `wound_capacity: pm(1000)`, `natural_recovery_rate: pm(0)`, `natural_clot_resistance: pm(0)`
- Empty `WoundList`
- No food, no workstations, no other agents, no recipes
- `seed_actor_local_beliefs` with `DirectObservation`
- Default `DriveThresholds`

**Assertions**:
1. After first fire (~tick 1): `wounds.len() == 1`, capture `WoundId`, capture severity S1, capture `inflicted_at` T1
2. After second fire (~tick 6): `wounds.len() == 1`, same `WoundId`, severity S2 > S1, `inflicted_at` T2 > T1
3. Every tick: `wounds.len() <= 1` (consolidation invariant)
4. Agent alive throughout (never exceeds `wound_capacity`)

### 2. Add replay companion

Standard deterministic replay companion test using `replay_and_verify`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — add Scenario 29 test function)

## Out of Scope

- Any production code changes (no changes to `needs.rs`, `combat.rs`, `ranking.rs`, or any `src/` file)
- Any harness structural changes (use existing inline txn setup pattern)
- Scenario 30 (recovery-aware priority boost) — separate ticket S17WOULIFGOLSUI-002
- Docs updates — separate ticket S17WOULIFGOLSUI-003
- Re-testing wound creation, bleed, or death (already covered by Scenarios 7g, 8, 8d, 9d)
- Adding new wound mechanics or changing the recovery gate
- Testing deprivation + combat wound coexistence

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening` — new scenario passes
2. `cargo test -p worldwake-ai --test golden_emergent` — full emergent suite unchanged
3. `cargo test -p worldwake-ai` — full AI crate suite unchanged
4. `cargo test --workspace` — no regressions
5. `cargo clippy --workspace --all-targets -- -D warnings` — no warnings

### Invariants

1. Wound count never exceeds 1 across the entire test window (consolidation, not duplication)
2. `WoundId` preserved across multiple deprivation fires (Principle 4: persistent identity)
3. Severity strictly increases between fires (Principle 9: outcomes leave aftermath)
4. `inflicted_at` updates to tick of each fire
5. Agent remains alive throughout (wound_load < wound_capacity pm(1000))
6. Deterministic replay produces identical state hash
7. No production code modified — this is a coverage-only ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_deprivation_wound_worsening_consolidates_not_duplicates` — proves consolidation through live needs dispatch
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_deprivation_wound_worsening_consolidates_not_duplicates_replay` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
