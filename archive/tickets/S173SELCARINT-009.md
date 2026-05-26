# S173SELCARINT-009: Scenario E — Repeated interruption → deprivation collapse

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None (golden scenario only)
**Deps**: `archive/tickets/S173SELCARINT-004.md` (wash/toilet contract), `archive/tickets/S173SELCARINT-005.md` (atomic-action abort traces), `archive/tickets/S173SELCARINT-006.md` (emitter filter), `archive/tickets/S173SELCARINT-007.md` (Scenario C release pattern), `archive/specs/S173-self-care-interruption-occupancy.md` (D8, Scenario E)

## Problem

Before this ticket, the spec's most ambitious validation claim was that repeated self-care interruption can lawfully end in deprivation collapse. Live reassessment narrowed the death axis: `DeprivationExposure::dirtiness_critical_ticks`, `fatigue_critical_ticks`, and `bladder_critical_ticks` accumulate, but `crates/worldwake-systems/src/needs.rs::apply_deprivation_consequences` only created deprivation wounds for hunger and thirst. This ticket proved repeated Wash interruption as the self-care loop and hunger starvation as the existing deprivation-death substrate, rather than preserving the disproved drafted dirtiness-death path.

## Assumption Reassessment (2026-05-26)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Live `GoalKind` surface under test: `GoalKind::Wash`, selected autonomously by an AI-controlled dirty agent through the existing candidate/planner path in `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs`.
2. Live deprivation-death substrate: `crates/worldwake-systems/src/needs.rs::apply_deprivation_consequences` wounds only `DeprivationKind::Starvation` and `DeprivationKind::Dehydration`; bladder critical exposure causes an accident, and dirtiness/fatigue exposure currently has no wound/death consequence. The landed proof uses hunger (`HomeostaticNeedId::Hunger`) with `MetabolismProfile::starvation_tolerance_ticks` as the wound cadence.
3. Interruption source: the landed harness applies repeated external local cancellations after the AI selects and starts Wash. This is a controlled golden seam for the already-landed abort/occupancy contract, not an authored predator/patrol simulation. The deviation keeps the death cause emergent through the normal needs and wound systems and avoids adding a scenario-only interruption mechanism.
4. Cumulative arithmetic: the scenario starts hunger below critical, raises it with `MetabolismProfile::hunger_rate`, shortens `starvation_tolerance_ticks` to `2`, and asserts at least three Wash interruptions before `DeathCause::NeedDeprivation { need: Hunger }`. Wound load then crosses the existing `CombatProfile::wound_capacity` threshold.
5. CI lane placement: the landed tests are ignored goldens owned by the golden-survival workflow lane.
6. Cross-system interactions: the scenario composes `GoalKind::Wash` candidate/planner selection, `SelfCareOccupancy` lifecycle, `ActionTraceDetail::SelfCareInterrupted`, `EventTag::ActionAborted`, `DeprivationExposure::hunger_critical_ticks`, starvation wound creation, `DeadAt`, and `EventTag::Death`.
7. Replay determinism is asserted by a companion ignored golden that compares the full observation from two runs at the same seed.

## Architecture Check

1. The scenario uses no new production mechanism. The only harness control is repeated cancellation after the AI has selected and started Wash; death still arises from existing needs, wound, and death systems.
2. The proof is causal: interruption, occupancy release, starvation exposure, starvation wound, and death are exposed through event/action trace and authoritative world state.
3. Per FND-31 systemic validation, the scenario covers the positive collapse case and proves no silent rescue or post-death action start.

## Verified Layers

1. Repeated interruption count → action/event trace: the observation records at least three AI-selected Wash starts cancelled before commit, each adding an `EventTag::ActionAborted` and releasing `SelfCareOccupancy`.
2. Deprivation accumulator → authoritative world state: `DeprivationExposure::hunger_critical_ticks` reaches critical exposure before starvation wounds reset it.
3. Deprivation wound emergence → authoritative world state: the agent's `WoundList` contains `DeprivationKind::Starvation`.
4. Death → event log/world state: `DeadAt` carries `DeathCause::NeedDeprivation { need: Hunger }`; `EventTag::Death` is covered by the same death substrate exercised by S81 and by the post-death no-action assertion.
5. Replay determinism → same observation from two runs at the same seed.

## Landed Changes

### 1. Authored Scenario E golden

Extended `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs` with Scenario 481 and a replay companion:

- One AI-controlled dirty agent with `MetabolismProfile::hunger_rate` configured to accumulate hunger and `starvation_tolerance_ticks` shortened to make the proof bounded.
- One `WashBasin`-tagged `Facility` co-located with the agent's start position; `WashBasinState` configured with enough `clean_water_units` that water is not the bottleneck.
- Repeated controlled cancellation after AI-selected Wash starts.
- Tick budget bounded to 160 ticks by the shortened starvation tolerance.
- Ignored lane placement via golden-survival workflow.

Assertions match the verified layers above.

### 2. Identified the deprivation-wound threshold field

Documented the live threshold truth in this ticket/spec: `dirtiness_critical_ticks` is not a wound-producing death path today; starvation wounds use `starvation_tolerance_ticks`.

### 3. Updated golden inventory

Regenerated `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-scenario-details/`, and `docs/generated/golden-coverage-matrix.md` via `python3 scripts/golden_inventory.py --write --check-docs`.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` (verification-hygiene clippy cleanup for the prior S173 player-POV helper)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/` (regenerated subdirectory)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `archive/specs/S173-self-care-interruption-occupancy.md` (truth-sync for the live hunger-deprivation Scenario E seam)

## Out of Scope

- New deprivation-wound mechanism — existing S17/S81 substrate is reused.
- New `EventTag` variant — `EventTag::Death` reused.
- Scenario A/B/C (standard goldens) — owned by ticket 007.
- Scenario D (player POV) — landed by `archive/tickets/S173SELCARINT-008.md`.
- Recovery-memory blocker (deferred per spec P1.3 and `Out of Scope (Tracked Elsewhere)` section).
- Production default rebalancing — the scenario authors its own profile values; production defaults are untouched.

## Acceptance Result

### Tests That Passed

1. New ignored golden: `golden_repeated_self_care_interruption_can_end_in_deprivation_death` — repeated Wash interruption occurs; starvation wound appears; `DeathCause::NeedDeprivation { need: Hunger }` fires; no post-death actions start.
2. Replay companion: `golden_repeated_self_care_interruption_collapse_replays_deterministically`.
3. Existing self-care interruption goldens pass.
4. Golden inventory regenerates cleanly: `python3 scripts/golden_inventory.py --write --check-docs` exits 0.

### Invariants

1. The death event is caused by accumulated hunger deprivation and starvation wounds, not by an external death trigger.
2. Repeated interruption is proven as repeated Wash start/cancel/abort/release cycles before death.
3. Replay determinism holds at the configured seed.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs` — Scenario 481 and replay companion.
2. `docs/generated/*` regenerated.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_self_care_interruption::golden_repeated_self_care_interruption_can_end_in_deprivation_death -- --ignored --exact`
2. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_self_care_interruption::golden_repeated_self_care_interruption_collapse_replays_deterministically -- --ignored --exact`
3. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_self_care_interruption -- --ignored`
4. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_self_care_interruption`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent -- --exact`
7. `cargo test -p worldwake-ai`
8. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
9. `./scripts/verify.sh` remains the final family pre-push gate owned by the harness.

## Outcome

Completed on 2026-05-26.

- Added ignored Scenario 481 in the existing S173 self-care interruption golden suite.
- Added a replay companion that compares the full repeated-interruption collapse observation at the same seed.
- Regenerated golden inventory/index/detail/coverage docs.
- Truth-synced the parent spec to the live deprivation-death substrate: hunger/thirst create wounds today; dirtiness/fatigue do not, and bladder causes an accident.
- Cleaned up the prior S173 player-POV helper's tuple return into a named struct so the affected crate all-target clippy gate passes.

## Deviations

- The drafted dirtiness-deprivation death path was disproved by live code. The landed scenario keeps Wash as the repeated self-care interruption loop but uses hunger starvation as the existing wound/death substrate.
- The drafted new file `survival_self_care_deprivation_collapse.rs` was not created; the existing `survival_self_care_interruption.rs` suite is the stronger owner for S173 Scenario E.
- The interruption source is controlled local cancellation after AI-selected Wash starts, not predator/patrol emergence. The death remains non-scripted and arises from normal needs/wound/death systems.
- `./scripts/verify.sh` is waived for this ticket iteration because this harness owns the final full pre-push gate after the family closes.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai repeated_self_care_interruption -- --list` (selector discovery; resolved the full test IDs after an initial zero-test exact false start).
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_self_care_interruption::golden_repeated_self_care_interruption_can_end_in_deprivation_death -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_self_care_interruption::golden_repeated_self_care_interruption_collapse_replays_deterministically -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_self_care_interruption -- --ignored`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_self_care_interruption`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent -- --exact`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Waived `./scripts/verify.sh` for this ticket iteration because `implement-spec-tickets` owns the final family pre-push gate.
