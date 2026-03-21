# S16S09GOLVAL-003: Golden — Multi-Agent Divergent Re-Evaluation (Principle 20)

**Status**: 🚫 NOT IMPLEMENTED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S16S09GOLVAL-001 (shared helpers in harness)

## Problem

No existing golden test proves that two agents with different profile parameters produce divergent **post-defend** behavior after a shared combat branch collapses. The repo already has golden coverage for generic weight-driven divergence outside combat, but it does not currently prove the specific S09 promise that finite defend expiry can feed two different agents back into different non-combat branches from the same post-threat world state.

## Assumption Reassessment (2026-03-20)

1. The original ticket overstated the gap. Existing golden coverage already proves generic profile-driven divergence in non-combat scenarios:
   - `golden_wound_vs_hunger_pain_first` / `golden_wound_vs_hunger_hunger_first` in `crates/worldwake-ai/tests/golden_emergent.rs`
   - `golden_care_weight_divergence_under_observation` in `crates/worldwake-ai/tests/golden_emergent.rs`
   What is still missing is a golden that proves divergence specifically after `defend` expires and combat pressure disappears.
2. Existing defend-specific golden coverage in `crates/worldwake-ai/tests/golden_combat.rs` is narrower:
   - `golden_defend_replans_after_finite_stance_expires` proves defend commits and the agent re-enters planning.
   - `golden_defend_changed_conditions` proves one defender switches away from `ReduceDanger` after the attacker dies.
   Neither test proves two defenders with different profiles diverge from the same post-threat state.
3. The ranking layer cited by the ticket is correct but incomplete as written. The relevant symbol is `rank_candidates()` in `crates/worldwake-ai/src/ranking.rs`, and there is already focused unit coverage for weight-sensitive care ranking there, including `self_treat_wounds_uses_pain_weight_for_motive` and `high_pain_weight_prioritizes_self_care_over_other_care`. This ticket is therefore missing golden/E2E coverage, not focused/unit coverage.
4. This remains a golden E2E ticket in `crates/worldwake-ai/tests/golden_combat.rs`, because the contract spans action lifecycle (`defend` expiry), changed combat conditions, decision re-evaluation, and durable state deltas.
5. Ordering contract: DefenderA's seeded finite `defend` resolves before DefenderB's. In current test architecture, the strongest reliable proof surface for that seeded occupancy boundary is authoritative active-action plus `CombatStance` resolution, not an action-trace commit event. The ordering is still action-lifecycle ordering, driven by `CombatProfile.defend_stance_ticks` via the real `defend` action duration path. The later divergence is not a pure same-state weight-only claim in the strictest sense, because the defenders re-enter planning at different times. The stronger invariant is: once concrete combat pressure is gone for both agents, each agent's first post-defend non-combat branch reflects its own utility profile.
6. Not removing or weakening any heuristic, filter, or suppression rule.
7. Not a stale-request, contested-affordance, start-failure, or political-office ticket.
8. No `ControlSource` manipulation or driver reset assumptions are involved.
9. Current runtime reassessment found one more mismatch in the original setup: a single starving hostile plus pre-seeded `defend` is not sufficient to keep two AI defenders on a concrete combat branch. Without a real attack lifecycle, the defenders can lawfully drop straight into other actions. The scenario must therefore use concrete attack pressure per defender rather than abstract hostility-only setup.
10. Scenario isolation choice: each defender should face a doomed human-controlled attacker already committed to a harmless `attack` action, so `defend` begins as a lawful response to concrete combat pressure and later expires into a non-combat branch after the attackers die from deprivation. The two defender-attacker pairs should be placed in mirrored but separate local arenas so unrelated `tell`/social branches do not crowd out the combat-to-self-care contract. Both defenders still get the same concrete self-care affordances (`Bread x1`, `Medicine x1`), and `no_recovery_combat_profile()` disables natural wound recovery so wound relief only comes from explicit care actions. Initial hunger should be low enough that combat pressure lawfully preserves `defend`, with elevated metabolism and different stance lengths creating the later hunger-vs-pain tradeoff.
11. Scope correction: the ticket should no longer claim "no existing golden test proves divergence" in general. It should claim "no existing golden test proves multi-agent divergence after finite defend expiry under changed combat conditions," and the setup must use real attack pressure to preserve that contract.

## Architecture Check

1. A new combat golden is still the right shape. Existing emergent goldens already prove generic weight-driven divergence, so the cleaner architecture move is to add one narrowly-scoped defend-specific scenario rather than broadening production code or duplicating the non-combat divergence suites.
2. No backwards-compatibility shims.

## Verification Layers

1. DefenderA seeded defend resolves before DefenderB seeded defend -> authoritative active-action plus `CombatStance` state
2. Concrete combat pressure disappears after the doomed attackers die -> authoritative world state plus post-resolution decision trace (no immediate `ReduceDanger` reselection for the observed post-defend branch)
3. DefenderA's first post-defend non-combat branch is self-care-first -> decision trace for selected goal, authoritative world state for wound-before-hunger delta
4. DefenderB's first post-defend non-combat branch is hunger-first -> decision trace for selected goal, authoritative world state for hunger-before-wound delta
5. Both defenders eventually address both needs -> authoritative world state
6. Deterministic replay -> world hash + event log hash

## What to Change

### 1. Add `golden_multi_agent_divergent_reevaluation` test to `golden_combat.rs`

Setup:
- `GoldenHarness::new(Seed([51; 32]))`
- Four agents across two mirrored isolated combat arenas:
  - **DefenderA**: reuse the same combat-stat substrate as `build_defend_changed_conditions_scenario` in `crates/worldwake-ai/tests/golden_combat.rs`, with `no_recovery_combat_profile()` only used to zero natural recovery and `defend_stance_ticks: nz(3)`. `UtilityProfile { pain_weight: pm(800), hunger_weight: pm(300), .. }`. Pre-seeded wound in the same moderate range as the existing changed-conditions defend golden (around severity 120), not a severe self-care emergency. Start hunger in the same moderate range as the existing changed-conditions defend golden (around pm(300)) but with elevated hunger metabolism so combat pressure still preserves `defend` while hunger becomes more meaningful by re-evaluation. Give Medicine x1, Bread x1.
  - **DefenderB**: same combat-stat substrate, wound substrate, and initial hunger/metabolism substrate as DefenderA, but `defend_stance_ticks: nz(8)` and `UtilityProfile { pain_weight: pm(300), hunger_weight: pm(800), .. }`.
  - **Doomed AttackerA**: human-controlled, already committed to a real `attack` against DefenderA using the existing living-combat attacker substrate, but configured to die from deprivation shortly after the scenario starts.
  - **Doomed AttackerB**: human-controlled, already committed to a real `attack` against DefenderB using the same attacker substrate, likewise doomed by deprivation.
- Add hostility and active attack instances per pair.
- Seed both defenders into active defend actions with respective durations.
- Seed local beliefs for all agents.
- Enable decision tracing and action tracing.

Observation loop (up to 80 ticks):
- Track per-agent defend-resolution ticks via active-action/stance observation, first post-defend non-combat selected goal, and state deltas (`hunger`, `wound_load`).

Assertions:
1. DefenderA's defend resolves before DefenderB's.
2. Both doomed attackers die.
3. DefenderA's first post-defend non-combat selected goal is care/self-care oriented, and wound load decreases before hunger decreases.
4. DefenderB's first post-defend non-combat selected goal is food/consumption oriented, and hunger decreases before wound load decreases.
5. Both defenders eventually address both needs.
6. Deterministic replay.

### 2. Add `golden_multi_agent_divergent_reevaluation_replays_deterministically` companion

Standard two-run hash comparison.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add two tests)

## Out of Scope

- Any engine/production code changes
- Broadening or duplicating existing non-combat divergence coverage in `golden_emergent.rs`
- Changes to the golden harness module
- Testing more than two divergent agents
- Asserting specific tick numbers for defend resolution (only relative ordering)
- Testing domain crossing (that's S16S09GOLVAL-004)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_multi_agent_divergent_reevaluation` — new test passes
2. `cargo test -p worldwake-ai golden_multi_agent_divergent_reevaluation_replays_deterministically` — replay passes
3. `cargo test -p worldwake-ai` — full suite, no regressions

### Invariants

1. Append-only event log is never mutated
2. Conservation invariants hold
3. Determinism: identical seed produces identical hashes
4. Both agents use the same shared planning/ranking pipeline after defend expiry; divergence comes from profile and local state, not agent-specific branching code

## Test Plan

### New/Modified Tests

1. `golden_multi_agent_divergent_reevaluation` in `crates/worldwake-ai/tests/golden_combat.rs` — proves defend-expiry changed-conditions divergence for two agents with different profiles
2. `golden_multi_agent_divergent_reevaluation_replays_deterministically` in `crates/worldwake-ai/tests/golden_combat.rs` — proves the new combat-divergence scenario is deterministic

### Commands

1. `cargo test -p worldwake-ai golden_multi_agent_divergent_reevaluation`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`

## Outcome

- Date: 2026-03-21
- What actually changed: reassessed the ticket against current coverage and runtime behavior; corrected the ticket’s assumptions and scope; investigated the multi-agent defend path; wrote a follow-up design report at `reports/2026-03-21-combat-commitment-substrate.md`.
- Why the ticket was not implemented: the current engine exposes `defend` as a real authoritative action lifecycle, but not as a stable AI-owned multi-agent combat commitment. Under current architecture the requested golden cannot be made robust without first adding an explicit combat-commitment substrate.
- Deviations from the original plan: no `golden_multi_agent_divergent_reevaluation` test was added; no production code changed.
- Verification results:
  - `cargo test -p worldwake-ai golden_defend_changed_conditions -- --nocapture` passed on 2026-03-21
  - exploratory multi-agent defend scenarios were attempted and reverted because they did not expose a stable defend-lifecycle contract under the current runtime
