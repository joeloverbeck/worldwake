# S13POLEMEGOLSUI-001: Combat Death Triggers Force-Law Succession

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None expected; reassess if the current harness lacks event-log assertion support for succession ordering
**Deps**: `specs/S13-political-emergence-golden-suites.md`, existing `golden_combat.rs` and `golden_offices.rs` coverage

## Problem

The current political golden coverage proves force-law succession only when the losing contender is already dead in setup. The suite does not yet prove the full emergent chain where real combat produces a death, the office becomes vacant through authoritative state, and force-law succession installs the surviving contender without any combat-specific political hook.

## Assumption Reassessment (2026-03-18)

1. Existing force-law office coverage is `golden_force_succession_sole_eligible` and `golden_force_succession_deterministic_replay` in `crates/worldwake-ai/tests/golden_offices.rs`. That scenario pre-seeds a dead rival with `DeadAt(Tick(0))` and proves only that `resolve_force_succession()` in `crates/worldwake-systems/src/offices.rs` installs the sole living contender after the vacancy timer elapses. It does not prove combat-created vacancy.
2. Existing combat golden coverage in `crates/worldwake-ai/tests/golden_combat.rs` includes `golden_combat_between_living_agents`, `golden_death_cascade_and_opportunistic_loot`, and `golden_death_while_traveling`. These cover combat wounds/death and downstream corpse behavior, but none assert political vacancy or office installation after a combat death.
3. Existing focused coverage already proves the authoritative office-vacancy path at the system layer in `crates/worldwake-systems/src/offices.rs`, including `vacancy_activation_sets_vacancy_since_clears_relation_and_emits_visible_event`, `force_succession_installs_only_uncontested_eligible_present_agent`, and `force_succession_blocks_when_multiple_contenders_are_present`. The gap is specifically golden E2E cross-system coverage, not missing unit/system coverage.
4. The intended verification layer is golden E2E with full action registries, because the scenario must exercise real combat execution, authoritative `DeadAt` mutation from `crates/worldwake-systems/src/combat.rs`, and later political succession in the same runtime.
5. Current test helpers in `crates/worldwake-ai/tests/golden_harness/mod.rs` already provide the setup surface for the scenario (`seed_office`, `seed_agent`, `add_hostility`, `enterprise_weighted_utility`, perception helpers, action tracing). However, succession installation is a system transaction from `succession_system()` rather than an action lifecycle event, so ordering cannot be asserted purely through action traces.
6. The golden E2E docs named in this ticket are already stale relative to the current test inventory. Any doc update in this ticket must correct counts and matrices from the real `cargo test -p worldwake-ai -- --list` output after the new scenario lands, not increment from the old numbers in place.

## Architecture Check

1. Add this to `crates/worldwake-ai/tests/golden_emergent.rs`, not `golden_offices.rs`, because the point of the scenario is cross-system emergence between combat and politics rather than another office-isolated behavior proof. That keeps `golden_offices.rs` focused on politics in relative isolation and `golden_emergent.rs` focused on multi-system chains.
2. The scenario must prove Principle 24 through state-mediated coupling only: combat writes wound/death state, `succession_system()` in `crates/worldwake-systems/src/offices.rs` reacts to authoritative vacancy state, and no direct combat-to-politics call path or compatibility shim is introduced.
3. The clean verification split is:
   - combat execution/liveness through action tracing
   - vacancy and installation ordering through authoritative world state plus event-log ordering
   This is cleaner than trying to invent a synthetic political action trace for a system tick mutation.

## What to Change

### 1. Add the emergent combat-to-politics golden scenario

Add `golden_combat_death_triggers_force_succession` and `golden_combat_death_triggers_force_succession_replays_deterministically` to `crates/worldwake-ai/tests/golden_emergent.rs`.

The scenario should:
- Set up a force-law office at `VillageSquare` with no eligibility filter.
- Create an incumbent office holder and a hostile challenger with real combat/perception profiles.
- Assign the incumbent as the initial office holder so the office becomes vacant only when combat kills that holder.
- Use the real combat path (`EngageHostile` -> attack lifecycle -> wound/death) instead of pre-seeding `DeadAt`.
- Assert combat through action traces and assert that the fatal death/vacancy event-log mutations precede the later political installation event.
- Assert the authoritative end state: incumbent dead, challenger installed, no commodity creation.

### 2. Update golden E2E documentation in the same ticket

Review and update the relevant `docs/golden-e2e*` docs so they reflect the new cross-system political-emergence coverage after this scenario lands.

At minimum:
- `docs/golden-e2e-coverage.md`
- `docs/golden-e2e-scenarios.md`

Update suite counts and the cross-system interaction matrix only for coverage that exists after this ticket. Do not pre-document S13 scenarios that are not implemented yet.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Changing authoritative combat, wound, death, or succession logic to make the test easier
- Moving or refactoring existing `golden_combat.rs` or `golden_offices.rs` scenarios
- Adding new political mechanics beyond the existing force-law vacancy semantics
- Broad doc rewrites outside the new scenario and resulting suite-count/matrix updates

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession`
2. `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_offices golden_force_succession_sole_eligible`
4. `cargo test -p worldwake-ai --test golden_combat golden_death_cascade_and_opportunistic_loot`
5. Existing suite: `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. Combat and politics remain connected only through authoritative world state and event history; no direct system-to-system call path is added.
2. Force-law succession still installs only living eligible contenders after the configured succession delay and still does not rely on support declarations.
3. The ordering contract is mixed on purpose: fatal combat is observed via action trace/event log, while office installation is observed via event-log/world-state ordering because succession is a system mutation, not an action lifecycle event.
4. Commodity totals remain conserved across the scenario; the new test must not legitimize item creation or disappearance as part of the death-to-succession chain.
5. Same-seed replay remains deterministic at both world-hash and event-log-hash level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — add the combat-death-to-force-succession scenario and replay companion; use action traces for combat and event-log/world-state inspection for succession ordering.
2. `docs/golden-e2e-coverage.md` — record the new interaction coverage and revise counts from the actual current suite.
3. `docs/golden-e2e-scenarios.md` — add the new scenario catalog entry and cross-system chain summary.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession`
2. `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai --test golden_offices golden_force_succession_sole_eligible`
5. `cargo test -p worldwake-ai --test golden_combat golden_death_cascade_and_opportunistic_loot`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-03-18

- Added `golden_combat_death_triggers_force_succession` and `golden_combat_death_triggers_force_succession_replays_deterministically` to `crates/worldwake-ai/tests/golden_emergent.rs`.
- The final assertions use the architecturally correct split:
  - combat execution is verified through action traces
  - death, office vacancy, and installation ordering are verified through authoritative event-log deltas and world state
- Updated `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` to record the new cross-system chain and correct suite counts from the current repository state.

Deviations from original plan:

- The original ticket proposed asserting office-installation ordering through action traces. That was corrected before implementation because succession is a system transaction in `crates/worldwake-systems/src/offices.rs`, not an action lifecycle event.
- Verification was broadened beyond the initial ticket commands to include `cargo test --workspace` so the archived outcome reflects the stricter validation actually performed.

Verification results:

- `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession -- --exact`
- `cargo test -p worldwake-ai --test golden_emergent golden_combat_death_triggers_force_succession_replays_deterministically -- --exact`
- `cargo test -p worldwake-ai --test golden_emergent`
- `cargo test -p worldwake-ai --test golden_offices golden_force_succession_sole_eligible -- --exact`
- `cargo test -p worldwake-ai --test golden_combat golden_death_cascade_and_opportunistic_loot -- --exact`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
