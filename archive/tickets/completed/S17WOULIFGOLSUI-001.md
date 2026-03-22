# S17WOULIFGOLSUI-001: Golden Scenario 29 — Deprivation Wound Worsening Consolidates Not Duplicates

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S11 (wound lifecycle audit — `worsen_or_create_deprivation_wound`), E12 (wound schema), E09 (deprivation exposure), E13 (decision architecture)

## Problem

The `worsen_or_create_deprivation_wound` consolidation invariant (repeated deprivation fires merge into one wound with preserved `WoundId`, increasing severity, updated `inflicted_at`) is only tested via focused unit tests. No golden E2E test exercises this path through the live needs system dispatch. Existing deprivation goldens (Scenarios 8, 8d, 9d) use fragile agents (`wound_capacity: pm(200)` + pre-existing wounds) that die from ONE fire, never surviving long enough for a second fire to exercise worsening.

## Assumption Reassessment (2026-03-21)

1. `worsen_or_create_deprivation_wound` exists in `crates/worldwake-systems/src/needs.rs` and is called from `apply_deprivation_consequences()` when `DeprivationExposure.hunger_critical_ticks` reaches `MetabolismProfile.starvation_tolerance_ticks`. Existing focused coverage already proves the helper and the live needs-system path at the unit/runtime layer: `worsen_creates_new_when_no_existing`, `worsen_increases_existing_severity`, `worsen_caps_at_permille_max`, `different_kinds_create_separate_wounds`, `worsen_updates_inflicted_at`, `needs_system_requires_another_full_tolerance_period_before_second_wound`, and `needs_system_second_starvation_threshold_worsens_existing_wound` in `crates/worldwake-systems/src/needs.rs`. No golden test currently exercises this path — confirmed by `cargo test -p worldwake-ai --test golden_emergent -- --list` and by the absence of a Scenario 29 entry from `docs/golden-e2e-coverage.md`, `docs/golden-e2e-scenarios.md`, and `docs/generated/golden-e2e-inventory.md`.
2. `DeprivationExposure` lives in `crates/worldwake-core/src/needs.rs` and stores the per-need critical tick counters; `starvation_tolerance_ticks` in `MetabolismProfile` is the authoritative firing cadence. `WoundList::find_deprivation_wound[_mut]` in `crates/worldwake-core/src/wounds.rs` is the identity-preserving merge point the helper relies on. The helper increases severity by the agent's current `needs.hunger`, not by the hunger critical threshold. That concrete-state coupling matches the S17/S11 architectural intent, but it also means the original `hunger = pm(920)` + default-threshold setup cannot survive a second fire.
3. This is not an AI-reasoning regression ticket. The contract is authoritative wound-state evolution through live needs dispatch, not candidate generation, ranking, or planner behavior. Full action registries are still part of the existing golden harness, but the scenario does not require decision-trace assertions and should stay focused on authoritative-state proof.
4. No ordering-dependent assertions. The invariant is wound-count stability (always ≤ 1) and wound identity preservation (same `WoundId`), not action sequence ordering.
5. No heuristic removal. This ticket adds coverage for existing behavior.
6. Not a stale-request or start-failure ticket.
7. Not a political office-claim ticket.
8. No ControlSource manipulation. Agent uses default AI control.
9. **Isolation choice**: No food available (so no lawful self-feeding branch), all non-hunger metabolism rates zeroed, no other agents, no workstations, and a single location. Those removals are intentional because the contract is repeated deprivation firing through needs dispatch, not competition with travel, trade, production, or social affordances.
10. Ticket corrections required before implementation:
    - cite the exact current focused tests and current golden scenario/test names instead of the older shorthand labels "8 / 8d / 9d"
    - use the current deterministic golden replay pattern (same-seed rerun and hash/outcome comparison), not a nonexistent `replay_and_verify` helper
    - treat local-belief seeding as optional setup detail rather than a required architectural precondition, because this scenario is proved through authoritative world state rather than AI reasoning
    - replace the original `HomeostaticNeeds::new(pm(920), ...)` + default-threshold setup with a lawful custom hunger threshold band and a lower above-critical hunger value; otherwise the second fire is guaranteed to be fatal under the current architecture, so the proposed "survive two fires" proof is impossible

## Architecture Check

1. A golden on top of the existing live needs stack is cleaner than moving more assertions into focused tests or adding harness-only shortcuts. The focused tests already prove the helper and system cadence; this ticket closes the remaining architectural gap by proving the same invariant under the real multi-system golden harness without introducing a second implementation path. The right fix is to align the scenario setup with the existing concrete-state model, not to weaken the deprivation rules for test convenience.
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
- Custom hunger threshold band with `critical = pm(400)` so two starvation fires can lawfully occur before fatal wound load
- `HomeostaticNeeds::new(pm(420), pm(0), pm(0), pm(0), pm(0))` — hunger remains above the custom critical threshold while keeping two-fire survival possible
- Custom `MetabolismProfile`: `starvation_tolerance_ticks: nz(5)`, `hunger_rate: pm(0)`, all other rates `pm(0)`, all other tolerance ticks set high
- `DeprivationExposure` pre-seeded with `hunger_critical_ticks: 4` (1 tick from first fire)
- `CombatProfile`: `wound_capacity: pm(1000)`, `natural_recovery_rate: pm(0)`, `natural_clot_resistance: pm(0)`
- Empty `WoundList`
- No food, no workstations, no other agents, no recipes
- `seed_actor_local_beliefs` is optional and only acceptable if the concrete test setup already uses it consistently; the invariant does not depend on belief seeding
- Override the agent's hunger `DriveThresholds` band to the custom lower critical threshold; other threshold bands can stay at defaults

**Assertions**:
1. After first fire (~tick 1): `wounds.len() == 1`, capture `WoundId`, capture severity S1, capture `inflicted_at` T1
2. After second fire (~tick 6): `wounds.len() == 1`, same `WoundId`, severity S2 > S1, `inflicted_at` T2 > T1
3. Every tick: `wounds.len() <= 1` (consolidation invariant)
4. Agent alive throughout (never exceeds `wound_capacity`)

### 2. Add replay companion

Use the current golden pattern: run the scenario twice with the same seed and compare returned hashes / captured outcome data.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — add Scenario 29 test function)
- `docs/generated/golden-e2e-inventory.md` (modify — mechanical inventory refresh from `scripts/golden_inventory.py --write --check-docs`)

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
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_deprivation_wound_worsening_consolidates_not_duplicates_replays_deterministically` — same-seed deterministic replay companion using the repo’s existing golden pattern

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-21
- What changed:
  - Added `golden_deprivation_wound_worsening_consolidates_not_duplicates`
  - Added `golden_deprivation_wound_worsening_consolidates_not_duplicates_replays_deterministically`
  - Refreshed `docs/generated/golden-e2e-inventory.md`
  - Corrected the ticket assumptions to match the current architecture and harness
- Deviations from original plan:
  - The original ticket setup (`hunger = pm(920)` with default hunger thresholds) could not survive a second deprivation fire because live severity worsening uses concrete `needs.hunger`, not the threshold value. The implemented scenario uses a custom lower hunger critical threshold and `hunger = pm(420)` so two lawful fires can occur without changing production architecture.
  - The replay companion uses the repo's existing same-seed replay pattern rather than a nonexistent `replay_and_verify` helper.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening` passed
  - `cargo test -p worldwake-ai --test golden_emergent` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `python3 scripts/golden_inventory.py --write --check-docs` passed
