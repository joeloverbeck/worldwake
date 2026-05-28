# S175FATCOLFAI-004: Exhaustion collapse + recovery golden scenarios

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None (scenario authoring + golden tests + generated-doc regen)
**Deps**: S175FATCOLFAI-001, S175FATCOLFAI-002, S175FATCOLFAI-003

## Problem

S175's FND-31 validation requires goldens proving the end-to-end fatigue-collapse chain and its dampener: (A) repeated failed rest → Exhaustion wounds → wound-load death attributed to `Fatigue`, with the failed-rest forensic chain intact; and (B) recovery before terminal load prevents collapse (the path is not a one-way death spiral). This ticket authors both scenarios (D7 A & B), the golden tests asserting the chain, and regenerates the golden inventory docs.

## Assumption Reassessment (2026-05-28)

1. The upstream substrate exists after deps land: `DeprivationKind::Exhaustion` (001), the fatigue branch + `Fatigue` death attribution (002), and `CriticalWindowReport.exhaustion_collapse_observed` (003). S174 substrate is already in the codebase: `FailedRestOpportunity` and `FailedRestKind` (`crates/worldwake-ai/src/survival_forensics.rs:47`), `SleepFailureCause::HostileProximity` (`crates/worldwake-core/src/decision_event_payload.rs`), `RestCapacity` component (`crates/worldwake-core/src/rest_site.rs:8`), authored via `PlaceDef.rest_capacity: Option<u32>` (`crates/worldwake-cli/src/scenario/types.rs:474`).
2. Scenario authoring conventions (confirmed against `scenarios/survival-rest-interrupted-by-danger.ron` and `scenarios/survival-failed-rest-cascade.ron`): `exhaustion_collapse_ticks` is authored directly inside the bare `metabolism_profile: ( … )` block as a plain integer (RON deserializes into `NonZeroU32`); there is **no** `MetabolismProfileDef` wrapper and **no** `nz(60)` RON syntax. A hostile interrupter uses bidirectional `hostilities: [(subject, target), (target, subject)]` plus a bidirectional edge so the hostile agent reaches the sleeper and aborts sleep via `SleepFailureCause::HostileProximity`. A place with rough-sleep-only omits `rest_capacity` (no `RestCapacity` component).
3. Scenario A (`survival-exhaustion-collapse.ron`): one rough-sleep-only place + a hostile agent that periodically interrupts; tired agent with low `exhaustion_collapse_ticks` (e.g. `60`). Scenario B (`survival-exhaustion-recovery.ron`): hostile-interrupted shelter at X + safe place Y a short travel away; agent fails at X then recovers at Y.
4. Golden conventions: per `tickets/README.md`, `docs/generated/golden-e2e-inventory.md` is the canonical `golden_*` test-name inventory and `docs/generated/golden-scenario-index.md` the gameplay overview; both (plus `golden-scenario-details/`) regenerate with `python3 scripts/golden_inventory.py --write --check-docs`. New golden scenario tests live under `crates/worldwake-ai/tests/scenarios/`. Confirm exact harness entrypoint and existing `golden_*` naming with `cargo test -p worldwake-ai -- --list` before writing test names.
5. Determinism (CLAUDE.md invariant): both scenarios must assert identical wound-creation tick, death tick, and recorded `failed_rest_opportunities` across a replay run (seeded `ChaCha8Rng`, no floats/wall-clock). Scenario A additionally asserts the S81 invariant that no action starts post-death.
6. Cumulative-arithmetic envelope (Scenario A): with `exhaustion_collapse_ticks = 60` and every rough-sleep attempt aborted, `fatigue_critical_ticks` reaches 60 → first Exhaustion wound; continued failure adds/worsens wounds until `wound_load >= wound_capacity` → death. Tune the scenario horizon and the agent's `CombatProfile.wound_capacity` / wound severity so death is reachable within a tractable tick budget; cross-check against `survival-failed-rest-cascade.ron`'s horizon as a baseline.

## Architecture Check

1. Two separate scenarios (collapse vs recovery) prove both the positive-feedback loop *and* its dampener, satisfying FND-31's "multiple independent patterns + negative case" requirement — a single collapse scenario would not prove the loop is escapable. Authoring via existing RON + golden harness (no new test infrastructure) keeps the proof on the canonical golden surface (FND-31).
2. No backwards-compatibility concerns — net-new scenario files and golden tests.

## Verification Layers

1. Exhaustion-wound creation at the threshold tick -> authoritative `WoundList` assertion in the golden (`WoundCause::Deprivation(DeprivationKind::Exhaustion)`).
2. Wound-load death attributed to `Fatigue` -> authoritative `DeadAt` payload assertion (`DeathCause::NeedDeprivation { need: Fatigue }`) + event-log `EventTag::Death` delta.
3. No post-death action start -> action-trace / scheduler assertion (S81 invariant).
4. Forensic chain (flag + failed-rest records) -> `CriticalWindowReport.exhaustion_collapse_observed == true` and per-frame `CriticalWindowFrame.failed_rest_opportunities` aggregated across frames carry `SleepFailureCause::HostileProximity` (derived read-model assertion).
5. Recovery dampener (Scenario B) -> `fatigue_critical_ticks` resets to 0, no Exhaustion wound, `exhaustion_collapse_observed == false`, window ends in successful rest.
6. Determinism -> replay-equivalence assertion (identical wound/death ticks + recorded opportunities).

## What to Change

### 1. Author Scenario A — `survival-exhaustion-collapse.ron`

Rough-sleep-only place (no `rest_capacity`), a hostile interrupter (bidirectional `hostilities` + edge), one tired agent with `exhaustion_collapse_ticks: 60` (and a `wound_capacity`/severity envelope making death reachable). Every Sleep aborts via `SleepFailureCause::HostileProximity`.

### 2. Author Scenario B — `survival-exhaustion-recovery.ron`

Hostile-interrupted shelter at X + safe place Y reachable via a short bidirectional edge; tired agent fails rest at X then travels to Y and sleeps successfully (fatigue drops below critical).

### 3. Golden test for Scenario A

Assert: fatigue_critical_ticks accumulation; Exhaustion wound at `fatigue_critical_ticks >= 60`; counter reset on creation; second wound/worsening; `EventTag::Death` with `DeathCause::NeedDeprivation { need: Fatigue }`; no post-death action start; `exhaustion_collapse_observed == true`; aggregated `failed_rest_opportunities` all `HostileProximity`; deterministic replay.

### 4. Golden test for Scenario B

Assert: accumulation at X; successful sleep at Y drops fatigue below critical; `fatigue_critical_ticks` resets; no Exhaustion wound; `exhaustion_collapse_observed == false`; window frames list the X failures but the window ends in successful rest; deterministic replay.

### 5. Regenerate golden inventory docs

Run `python3 scripts/golden_inventory.py --write --check-docs` and commit the updated `docs/generated/golden-e2e-inventory.md`, `golden-scenario-index.md`, and `golden-scenario-details/` entries for the two new scenarios.

## Files to Touch

- `scenarios/survival-exhaustion-collapse.ron` (new)
- `scenarios/survival-exhaustion-recovery.ron` (new)
- `crates/worldwake-ai/tests/scenarios/` (new golden test file(s) — exact path/module per existing `survival_*` scenario test layout; confirm with `cargo test -p worldwake-ai -- --list`)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/` (modify — regenerated entries for the two scenarios)

## Out of Scope

- Engine/logic changes (all in 001–003).
- New sleep mechanics or wake-reason work (S174, already landed).
- An incapacitation-without-death state (spec Open Question 1).
- Mixed-deprivation death-attribution scenarios (spec Open Question 2 — deferred).

## Acceptance Criteria

### Tests That Must Pass

1. Scenario A golden proves the full chain: accumulation → Exhaustion wound at tick 60 → reset → worsening → `Fatigue` death → no post-death action → `exhaustion_collapse_observed == true` → all failed-rest records `HostileProximity`.
2. Scenario B golden proves recovery: reset, no wound, `exhaustion_collapse_observed == false`, window ends in successful rest.
3. Both scenarios replay deterministically (identical wound/death ticks and recorded opportunities).
4. `python3 scripts/golden_inventory.py --write --check-docs` reports the docs in sync.
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Collapse is reachable only through sustained critical exposure produced by ordinary world processes (failed rest), never a scripted event (FND-1).
2. Recovery genuinely prevents collapse — the loop is escapable (FND-11 dampener).
3. The full causal chain is inspectable from authoritative state + event log + the derived report (FND-29).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/<exhaustion_collapse module>.rs` — Scenario A golden. Rationale: D7 Scenario A, FND-31 positive pattern.
2. `crates/worldwake-ai/tests/scenarios/<exhaustion_recovery module>.rs` — Scenario B golden. Rationale: D7 Scenario B, FND-31 dampener/negative case. (May share one file if the harness convention groups sibling scenarios.)

### Commands

1. `cargo test -p worldwake-ai -- --list` (confirm golden test naming/target before finalizing)
2. `cargo test -p worldwake-ai` (run the new goldens; CI-only ignored goldens run via the golden-survival workflow per existing convention)
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `scripts/verify.sh`
