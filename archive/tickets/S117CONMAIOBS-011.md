# S117CONMAIOBS-011: Baseline `AcuteNeedSpike` triage and architectural disposition

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — investigation, trace analysis, roadmap/spec disposition, and follow-up ticket creation only
**Deps**: `archive/tickets/S117CONMAIOBS-005.md`, `archive/tickets/S117CONMAIOBS-010.md`, `S117CONMAIOBS-007`, `specs/S117-convergence-maintenance-observer-smells.md`, `docs/golden-e2e-testing.md`

## Problem

`ACUTE_NEED_SPIKE` fires on `scenarios/survival-baseline.ron`, and unlike the convergence false positive, the evidence currently suggests these are real high-need episodes rather than detector noise. The corrected `MAINTENANCE_STARVATION` detector from `S117CONMAIOBS-010` also leaves three severe baseline hunger/thirst windows. In the same observer dump, `Agent B` hits existing `SUSTAINED_CRITICAL_NEED` anomalies, and the critical-window forensics show repeated oscillation between a food-only site (`Fertile Fields`) and water/wash/sleep sites (`Riverside Camp` / `Forest Clearing`). Before changing the detector, the scenario, or planner behavior, the project needs an explicit disposition ticket that decides which layer is wrong and why, using repo contracts plus outside primary research on homeostatic decision systems.

## Assumption Reassessment (2026-04-18)

1. Live acute detector code in [`crates/worldwake-cli/src/bin/observer.rs`](../crates/worldwake-cli/src/bin/observer.rs) is mechanically simple: it emits one anomaly per maximal 30–99 tick run at or above each agent's authored critical threshold (`detect_acute_need_spike`, lines 1548-1615). Unlike `MaintenanceStarvation`, the current baseline evidence does not show an internal contradiction in the detector's own predicate.
2. The baseline scenario still declares a healthy survival envelope in [`scenarios/survival-baseline.ron`](../scenarios/survival-baseline.ron), lines 8-13: max authored-critical run `100`, all five self-care families required. The same scenario also authors a structurally split substrate: the only food source is apples at `Fertile Fields`, while water and wash facilities exist at `Riverside Camp` / `Forest Clearing` (lines 371-381).
3. Shared abstraction boundary under audit: the survival baseline's coupled need-management behavior across scenario topology, planner choice/revision, and observer diagnostic output. This is intentionally mixed-layer; the ticket's job is to decide which layer owns the contradiction.
4. The immediate regression invariant is not "acute spikes must never happen." It is: if `survival-baseline.ron` is still intended to be healthy per its authored contract and roadmap role, then repeated critical acute windows plus existing sustained-critical windows on `Agent B` require a concrete explanation and a specific owning fix boundary.
5. Existing observer forensics from `/tmp/baseline-dump.md` show concrete windows:
   - Hunger `994..1044`
   - Hunger `1282..1379`
   - Thirst `994..1037`
   During those windows, `Agent B` alternates between places with `food=yes, water=no, wash=no, sleep=no` and `food=no, water=yes, wash=yes, sleep=yes`, indicating a lawful but unhealthy coupled-need oscillation rather than a bogus detector readout.
6. Existing repo expectations still treat `survival-baseline.ron` as healthy. [`crates/worldwake-ai/tests/golden_survival_baseline.rs`](../crates/worldwake-ai/tests/golden_survival_baseline.rs), lines 288-420, states that all agents should survive 1440 ticks and keep critical runs below the authored bound. The ignored golden is not currently a clean oracle either — running `all_agents_survive_1440_ticks` now fails early in the harness on an effective-place assumption before reaching the health assertion.
7. External primary-source guidance is relevant here because the architectural question is about homeostatic action selection under competing physiological needs, not a repo-local naming issue. Primary research reviewed during implementation points in the same direction as the local traces: efficient regulation should be anticipatory and trade-off aware, not purely reactive after critical-band errors accumulate. Anchors:
   - Keramati & Gutkin, 2014, *Homeostatic reinforcement learning for integrating reward collection and physiological stability* (eLife): https://pmc.ncbi.nlm.nih.gov/articles/PMC4270100/
   - Sterling, 2012, *Allostasis: a model of predictive regulation* (Physiology & Behavior): https://pubmed.ncbi.nlm.nih.gov/21684297/
   - van den Briel et al., 2004, *Effective Approaches for Partial Satisfaction (Over-Subscription) Planning* (AAAI): https://cdn.aaai.org/AAAI/2004/AAAI04-090.pdf
8. `docs/FOUNDATIONS.md` constraints that must govern the disposition:
   - Principle 3 / 27: detector output remains derived, never truth
   - Principle 14 / 20 / 21: agents reason from local beliefs and revisable commitments, not omniscient safety scores
   - Principle 22A: any learned/anticipatory fix must be concrete state, not hidden drama tuning
   - Principle 29A / 31-style traceability: the chosen fix must leave the acute windows explainable after the fact
9. Archived `S117CONMAIOBS-010.md` and completed `S117CONMAIOBS-012.md` together show that the remaining baseline maintenance windows are not a second observer bug; they are corroborating evidence for the same split-support coupled-need contradiction under audit here.
10. Adjacent contradictions are already split:
   - `GEOGRAPHIC_CONVERGENCE` lawful single-source false positives belong to `S117CONMAIOBS-009`
   - `MAINTENANCE_STARVATION` merged-window correctness belonged to `S117CONMAIOBS-010`; the remaining baseline maintenance disposition is now absorbed into this ticket as shared root-cause evidence

## Architecture Check

1. A disposition ticket is cleaner than prematurely weakening `ACUTE_NEED_SPIKE` to make baseline quiet. The current evidence points to a real survival problem, so suppressing the detector first would be a workaround that violates `FOUNDATIONS.md`.
2. Separating investigation/disposition from implementation keeps the eventual fix honest. Live traces plus the reviewed literature point to planner-side coupled-need anticipation, not scenario-name cleanup or detector dedup, as the next honest implementation owner.

## Verification Layers

1. Baseline acute windows are real and not detector arithmetic bugs -> observer Section 3 + Section 9 forensic evidence
2. The baseline scenario still claims a healthy authored envelope -> `survival-baseline.ron` plus `golden_survival_baseline.rs`
3. The corrected baseline maintenance windows are corroborating evidence for the same contradiction rather than a separate detector bug -> archived `S117CONMAIOBS-010.md`, completed `S117CONMAIOBS-012.md`, observer Section 3, and the shared local-place forensic summaries
4. The owning contradiction layer (scenario vs planner vs observer redundancy) is identified from the strongest available traces -> decision trace, action trace, local-place forensic summaries, and scenario substrate audit
5. Outside-repo guidance is used only to inform the architectural disposition, not to override local contracts -> cited primary literature in ticket closeout or created follow-up ticket
6. This is an investigation/disposition ticket, so no stronger mutation-layer proof applies until the owning implementation ticket is created

## What to Change

### 1. Audit the baseline acute and corroborating maintenance windows end-to-end

Use the existing observer Section 9 forensics, corrected maintenance anomalies, decision traces, action traces, and scenario substrate to explain exactly why `Agent B` enters the acute hunger/thirst runs on baseline and why the same split-support oscillation also produces severe maintenance windows.

### 2. Evaluate remedy classes against `FOUNDATIONS.md`

Compare at least these remedy classes explicitly:

- keep detector unchanged; fix baseline scenario substrate
- keep detector unchanged; fix planner/AI coupled-need reasoning or anticipation
- keep underlying behavior unchanged; reduce observer redundancy only (for example, suppress acute entries wholly subsumed by `SUSTAINED_CRITICAL_NEED`)

For each class, record why it does or does not satisfy local-belief planning, revisable commitments, concrete state, and traceability.

### 3. Produce a concrete owning follow-up

At ticket closeout, create or update exactly one owning follow-up path for the implementation work:

- scenario repair ticket, or
- planner/AI behavior ticket, or
- bounded observer dedup ticket

If more than one layer truly needs work, split them explicitly and document the dependency order.

## Files to Touch

- `tickets/S117CONMAIOBS-007.md` (modify if the owning baseline-regression handoff needs a factual note)
- `specs/S117-convergence-maintenance-observer-smells.md` (modify only if the acute-detector spec claim is proven factually wrong)
- `tickets/S117CONMAIOBS-013.md` (new planner/AI implementation follow-up)

## Out of Scope

- Implementing the eventual planner/scenario/observer fix in this ticket
- Weakening `ACUTE_NEED_SPIKE` just to preserve the current baseline narrative
- Rewriting the survival baseline golden harness unless the investigation proves the harness itself is the blocker

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. The ticket concludes with an explicit owning layer for the baseline acute-spike contradiction, backed by code/trace/scenario evidence.
2. Any recommended fix path remains aligned with `docs/FOUNDATIONS.md` and does not rely on omniscient planner scoring or scenario-name detector suppression.

## Test Plan

### New/Modified Tests

1. `None — investigation/disposition ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. `cargo test -p worldwake-cli`
4. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`

## Outcome

Completed on 2026-04-18.

- Audited the live observer anomalies, Section 9 critical-window forensics, baseline scenario substrate, and the ignored survival golden. The acute hunger/thirst runs and corroborating maintenance windows are real symptoms of one split-support survival failure mode, not observer arithmetic noise.
- The local trace evidence points to a planner-side ownership boundary. `rank_candidates()` in `crates/worldwake-ai/src/ranking.rs` scores self-care goals per need independently, `emit_need_driven_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` emits one-need-at-a-time self-care goals, and `build_candidate_plans()` in `crates/worldwake-ai/src/agent_tick/planning.rs` repeatedly selects/reactivates single-goal travel plans. On `survival-baseline.ron`, that produces repeated remote apple-travel selection without enough complementary water/sleep/wash buffering before or during split-support travel.
- Reviewed primary research and found it consistent with the local traces: Keramati & Gutkin (2014) argue efficient physiological control should learn predictive behavior to preclude future need violations; Sterling (2012) frames allostasis as anticipatory regulation rather than purely reactive correction; van den Briel et al. (2004) supports explicit trade-off handling when not all goals can be satisfied at once. Together with the repo's FOUNDATIONS, that supports a planner-side follow-up rather than scenario-name suppression or observer-only dedup.
- Created `tickets/S117CONMAIOBS-013.md` to own the planner/AI implementation work for split-support survival preparation, updated `tickets/S117CONMAIOBS-007.md` so the remaining baseline blocker points at that new ticket, and corrected `specs/S117-convergence-maintenance-observer-smells.md` where it previously claimed `ACUTE_NEED_SPIKE` and `SUSTAINED_CRITICAL_NEED` were disjoint by construction.

## Deviations

- The drafted ticket left scenario repair, planner behavior, and observer dedup open as co-equal remedy classes. Live implementation evidence narrowed that choice: this ticket resolves to a planner/AI follow-up, not a scenario retune or observer cleanup ticket.
- The drafted verification command `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored` was too broad for honest disposition evidence. The landed closeout uses the exact failing baseline-survival selector instead, because the current value of that command is the harness-level failure it exposes (`golden survival agents should always have an effective place`), not a green full-suite proof.

## Verification Result

- Passed `cargo test -p worldwake-cli --test golden_observer_anomalies`
- Passed `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
- Passed `cargo test -p worldwake-cli`
- Observed expected investigation failure in `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact` (`golden survival agents should always have an effective place`)
