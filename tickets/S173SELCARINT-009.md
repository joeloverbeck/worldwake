# S173SELCARINT-009: Scenario E — Repeated interruption → deprivation collapse

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None (long-running golden scenario only)
**Deps**: `archive/tickets/S173SELCARINT-004.md` (wash/toilet contract), `archive/tickets/S173SELCARINT-005.md` (atomic-action abort traces), S173SELCARINT-006 (emitter filter), S173SELCARINT-007 (Scenario C release pattern), `specs/S173-self-care-interruption-occupancy.md` (D8, Scenario E)

## Problem

The spec's most ambitious validation claim is that repeated self-care interruption can lawfully drive an agent to deprivation collapse — death from accumulated `DeprivationExposure::<need>_critical_ticks` crossing the deprivation-wound threshold (S17/S81), distinct from sustained hunger starvation already proven in `archive/specs/S81-golden-gaps-simulation-remediation.md`. Without this proof, the loop "interruption → replan → interrupted again → rising exposure → wound → death" is asserted only in spec prose. This ticket implements the long-running golden scenario (~2000-4000 ticks per spec Risk #4) that demonstrates the chain end-to-end: target → start → interrupted → release → replan → repeat → exposure → wound → death. Per spec Risk #4 this may be a separate "ignored CI" lane rather than the standard golden lane; both placements are acceptable.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Live `GoalKind` surfaces: `GoalKind::Wash` (driven by `HomeostaticNeeds::dirtiness` rising), `GoalKind::Relieve` (`bladder`), `GoalKind::Sleep` (`fatigue`). The scenario picks one need axis (most likely dirtiness via `GoalKind::Wash` because it's the simplest setup) and drives it via `MetabolismProfile::dirtiness_rate` accumulation.
2. Deprivation-wound threshold field location — **path-level item flagged during reassessment, deferred to ticket time per spec Risks #5**. The accumulator lives at `DeprivationExposure::dirtiness_critical_ticks: u32` (`crates/worldwake-core/src/needs.rs:122`); the wound-firing threshold lives elsewhere. Likely in the deprivation system module — grep `dirtiness_critical_ticks` to find the comparison site. Identify the threshold field at implementation time and document in the scenario's setup.
3. Interruption source: the scenario uses ordinary world events as the interruption — a hostile predator with patrol-pattern presence at the basin's place, or a recurring higher-priority self-care goal (a fast-accumulating bladder need). Per FND-1 maximal emergence, the interruption must arise from local world processes, not scenario script injection. Verify the existing combat / patrol scenario primitives at implementation time.
4. Existing deprivation-death proof: the S81 simulation-gaps golden already proves the hunger-starvation path. This scenario's chain is structurally similar but with a different need axis and a different driver (repeated interruption rather than sustained denial). The S81 scenario file is the structural precedent — locate at implementation time via `archive/specs/S81-golden-gaps-simulation-remediation.md` and the corresponding test file.
5. Cumulative arithmetic per `docs/precision-rules.md` Rule 7: validate survivability and non-survivability explicitly. The scenario's tick budget must accommodate enough interruption cycles to cross the deprivation-wound threshold AND enough subsequent ticks for wound load to exceed capacity and trigger `DeathCause::NeedDeprivation`. Concrete budget at scenario-design time: assume `dirtiness_critical_ticks` accumulator increments by 1 per tick at critical pressure; threshold T (verified at /implement-ticket time) yields 1 wound per ~T accumulator ticks; collapse requires N wounds. Total tick budget ≈ T × N × interruption-cycle-overhead. Spec Risk #4 gives a ~2000-4000 tick estimate.
6. CI lane placement: standard golden lane vs `#[ignore]` long-running lane. Decision at /implement-ticket time per spec Risk #4. The default decision is `#[ignore]` long-running lane if the scenario exceeds ~15 seconds wall-clock; standard lane if under.
7. Cross-system interactions: this scenario composes the full chain — `SelfCareOccupancy` lifecycle (ticket 004), `ActionTraceDetail::SelfCareInterrupted` trace surface (tickets 004, 005), candidate-emitter occupancy filter (ticket 006), `DeprivationExposure` accumulator (existing), deprivation-wound emergence (S17, existing), and `DeathCause::NeedDeprivation` / `EventTag::Death` (existing). No new mechanism — only composition.
8. Replay determinism per `docs/precision-rules.md` Rule 14 + CLAUDE.md Determinism invariant: state-hash must remain stable across replays at the same seed. The scenario includes a replay-equivalence assertion.

## Architecture Check

1. The scenario uses no new mechanism — pure composition of existing carriers. Per FND-1 emergence, no scripted death trigger, no hidden rescue, no scenario-specific target injection.
2. The proof is causal: each link in the chain (interruption → release → replan → repeat → exposure → wound → death) is exposed in the event log + decision trace. Per FND-29A causal history, the chain is reconstructible end-to-end after the fact.
3. Per FND-31 systemic validation, the scenario covers both a positive case (death emerges) and a negative case (no silent rescue, no scripted interruption beyond the local-world-process source).

## Verification Layers

1. Interruption count → event-log delta: count of `EventTag::ActionAborted` events filtered by `actor_id == agent_id` and action_name in {`"wash"`, `"toilet"`, `"sleep"`} crosses a configured threshold (e.g., ≥ 20 interruptions before death).
2. Deprivation accumulator → authoritative world state: `DeprivationExposure::dirtiness_critical_ticks` (or the analogous field) climbs across the run; crosses the wound-firing threshold.
3. Deprivation wound emergence → focused unit/runtime test or authoritative state assertion: agent's wound-list grows (concrete wound entries appear per S17 severity ladder).
4. Death → event log: `EventTag::Death` fires with `DeathCause::NeedDeprivation` for the agent.
5. Replay determinism → state-hash stable across replays at the same seed (existing replay-equivalence harness).
6. Decision-trace chain reconstruction → decision trace exposes the per-tick reasoning: target selection, start attempt, interruption cause, release, replan target. Per `docs/precision-rules.md` Rule 6, decision-trace assertions are preferred over indirect evidence.
7. Action-trace ordering → `(tick, sequence_in_tick)` ordering of abort → release → replan is consistent across ticks.

## What to Change

### 1. Author Scenario E golden

Create `crates/worldwake-ai/tests/scenarios/survival_self_care_deprivation_collapse.rs` (or per canonical naming convention — verify at implementation time):

- One agent with `MetabolismProfile::dirtiness_rate` configured to accumulate dirtiness fast.
- One `WashBasin`-tagged `Facility` co-located with the agent's start position; `WashBasinState` configured with enough `clean_water_units` that water is not the bottleneck.
- Interruption source: a hostile predator with a patrol pattern that brings it into co-location with the agent on a regular cadence, OR a recurring higher-priority self-care goal emerging from a fast-accumulating second need (e.g., `bladder` rising while agent attempts wash). The exact interruption mechanic is chosen at implementation time to match what already exists in the survival-lane scenario library.
- Tick budget: ~2000-4000 ticks per spec Risk #4; calibrated at implementation time.
- Optional `#[ignore]` lane placement if wall-clock exceeds ~15 seconds.

Assertions per Verification Layers above.

### 2. Identify the deprivation-wound threshold field

At /implement-ticket reassessment time, grep `dirtiness_critical_ticks` in `crates/worldwake-core/` and `crates/worldwake-systems/` to find the threshold comparison site. Document the threshold field name and its source profile in the scenario's setup comments, replacing the spec's Risks #5 placeholder with a concrete reference.

### 3. Update golden inventory

Regenerate `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-scenario-details/` via `python3 scripts/golden_inventory.py --write --check-docs`.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/survival_self_care_deprivation_collapse.rs` (new)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/` (regenerated subdirectory)
- Likely: parent test-binary registration file (`tests/scenarios/mod.rs` or equivalent) — verify at implementation time.

## Out of Scope

- New deprivation-wound mechanism — existing S17/S81 substrate is reused.
- New `EventTag` variant — `EventTag::Death` reused.
- Scenario A/B/C (standard goldens) — owned by ticket 007.
- Scenario D (player POV) — owned by ticket 008.
- Recovery-memory blocker (deferred per spec P1.3 and `Out of Scope (Tracked Elsewhere)` section).
- Profile-driven re-balancing of `dirtiness_rate` or threshold defaults — the scenario authors its own profile values; production defaults are untouched.

## Acceptance Criteria

### Tests That Must Pass

1. New golden: `survival_self_care_deprivation_collapse` — full chain executes; `DeathCause::NeedDeprivation` fires; replay-determinism asserts state-hash stable across replays at the same seed.
2. Replay-equivalence harness sanity: the scenario is included in the replay-equivalence sweep if `#[ignore]` is not applied.
3. Existing deprivation-death goldens pass: `archive/specs/S81-*` related goldens.
4. Golden inventory regenerates cleanly: `python3 scripts/golden_inventory.py --write --check-docs` exits 0.

### Invariants

1. The death event is caused by accumulated `DeprivationExposure`, not by an external scenario trigger.
2. The interruption source is a lawful local world process (predator patrol or competing self-care goal), not a scripted injection.
3. The decision-trace chain exposes every link from target selection to death, with no missing provenance per `docs/precision-rules.md` Rule 15.
4. Replay determinism holds at the configured seed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_self_care_deprivation_collapse.rs` (new) — the long-running scenario.
2. `docs/generated/*` regenerated.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai survival_self_care_deprivation_collapse` (or with `-- --ignored` if the `#[ignore]` lane is chosen)
2. `cargo test -p worldwake-ai --test golden_ai survival` (full survival lane sanity)
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `./scripts/verify.sh` before commit.
