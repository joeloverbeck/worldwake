# SANBASINCLEAN-001: Proactive basin-cleaning so the FND-11 dampener engages before full block

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner candidate generation / `GoalKind::Wash` plan search (`worldwake-ai`); a profile-driven "wash-worthwhile" threshold (`worldwake-core`); no authoritative-action change required
**Deps**: `archive/specs/S176-sanitation-facility-degradation-consequences.md` (introduced the wash-effectiveness scaling + `clean_wash_basin` recovery labor this ticket completes)

## Problem

S176 made wash relief scale down as a basin's own `dirtiness_level` rises toward `max_effective_dirtiness`, and added `clean_wash_basin` recovery labor. But the cleaning labor — the **only** dampener on basin `dirtiness_level` (the spec deliberately gave the basin no natural decay; grime requires active cleaning) — is only inserted by the planner when the **hard precondition fails**, i.e. `dirtiness_level >= max_effective_dirtiness` (full block). See `crates/worldwake-sim/src/affordance_query.rs:340-343` and `crates/worldwake-sim/src/action_validation.rs:114-117`:

```rust
Precondition::TargetWashBasinNotTooDirty { target_index } => /* basin */
    .is_some_and(|state| state.dirtiness_level < state.max_effective_dirtiness),
```

In realistic multi-agent competition this dampener never engages. Evidence — `golden-survival / scattered` (a single shared `Crossing Basin` for 3 agents, default basin state, full food/water competition): the basin climbs to ~550‰ (effectiveness ~45%), at which point agents **abandon** the now-ineffective basin (the costly travel to the single far basin loses to higher-priority hunger/thirst). The basin freezes mid-degraded, **never reaches the full-block threshold (1000‰), so `clean_wash_basin` is never committed by any agent**, and all three agents' `dirtiness` runs away to permanent critical (Agent B: 1028 consecutive ticks above critical vs. authored limit 680).

This is a **FND-11 violation**: the basin-degradation positive-feedback loop (use → dirtier basin → weaker wash → basin stays dirty) has a single dampener that does not engage in the regime where it is needed. The loop is effectively undampened below full block.

The feature's own goldens pass only because they sidestep this regime: `survival-sanitation-breakdown-1440.ron` authors a low `max_effective_dirtiness: 800` **and** zeros hunger/thirst (`needs: (hunger: 0, thirst: 0, …)`), so agents have no competing pressure and the basin is pushed to the hard block. No golden currently exercises basin degradation under realistic need competition.

## Assumption Reassessment (2026-05-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Live wash-effectiveness arithmetic** — confirmed in `crates/worldwake-systems/src/needs_actions.rs:1368-1378` (`apply_wash`): `agent_dirtiness_delta = raw_delta * (max_effective_dirtiness - dirtiness_level) / max_effective_dirtiness`, evaluated on pre-use basin dirtiness. So effectiveness degrades continuously from `dirtiness_level == 0` (100%) to `max_effective_dirtiness` (0%), while the **precondition** (`TargetWashBasinNotTooDirty`) only fails at the endpoint. The band just below the threshold is "wash allowed but near-useless, cleaning not triggered" — the dead zone.
2. **Live cleaning-insertion surface** — `PlannerOpKind::CleanWashBasin` is in the `GoalKind::Wash` relevant-op handling in `crates/worldwake-ai/src/goal_model.rs` (~line 1609). It advances the goal by establishing `Precondition::TargetWashBasinNotTooDirty`; the GOAP search only inserts it when that precondition is unsatisfied — i.e. at/above the hard block. This is the exact surface to extend.
3. **Live GoalKind under test**: `GoalKind::Wash`. Current prerequisite surface: `clean_wash_basin` (`PlannerOpKind::CleanWashBasin`) inserted only when `TargetWashBasinNotTooDirty` is violated. Candidate emission: `emit_wash_goal` (`crates/worldwake-ai/src/candidate_generation.rs:4654`), which reads facility condition via existing belief-view accessors (`facility_wash_basin_state` / `wash_basin_state`) — no new accessor is needed.
4. **Intended invariant restated** (the `scattered` golden narrative is motivating but must not be trusted blindly): *an agent whose dirtiness is critical, that can reach a usable-but-degraded shared basin, must be able to restore its dirtiness over time* — by cleaning the basin first when washing it would no longer give worthwhile relief. "Stay permanently dirty because the only basin is half-dirty and nobody cleans it" is an undampened-loop failure, not valid emergence. The `scattered` survival contract (`dirtiness` critical-run limit 680, `survival-scattered.ron:22`) is the correct contract and must **not** be relaxed.
5. **Heuristic/threshold being added** — the proactive trigger needs a "this basin is too degraded to be worth washing without cleaning first" threshold. Per FND-2/FND-3 (no naked dials, concrete agent/facility state) this must be a concrete, scenario-authored parameter, **not** a hardcoded constant. Two candidate homes, to be decided at implementation:
   - (preferred) a per-agent decision parameter — a minimum-acceptable-wash-effectiveness / wash-worthwhile floor on the agent profile (e.g. `MetabolismProfile` in `crates/worldwake-core/src/needs.rs`, universal agent profile per S128), read by the planner: "if washing here would relieve less than my floor, clean the basin first." This keeps the hard authoritative gate (`max_effective_dirtiness`) untouched and models the *agent's* policy as a concrete property, symmetric for Human/AI.
   - (alternative) a per-facility soft `clean_trigger_dirtiness` field on `WashBasinState`, distinct from and below `max_effective_dirtiness`. Cleaner authoring per facility but adds authoritative state for what is fundamentally a planner decision.
6. **Intended layer**: planner candidate generation / `GoalKind::Wash` plan search (`worldwake-ai`), plus one new profile field (`worldwake-core`). **No change to the authoritative `apply_wash` effect or the `TargetWashBasinNotTooDirty` precondition** — the hard block stays as the absolute legality gate; this ticket adds a *softer, planner-side* trigger above the agent's worthwhile floor. The `agent_tick` harness needs full action registries (the fix only manifests end-to-end through real wash/clean/travel actions under competition), so verification is golden-level, not needs-only.
7. **Ordering**: the inserted `clean_wash_basin` op must be ordered **before** the terminal `wash` op in the plan (same as the existing full-block insertion). Confirm the relevant-op/terminal-ordering logic in `goal_model.rs` treats the proactive insertion identically to `QueueForFacilityUse`/the existing `CleanWashBasin` insertion.
8. **Weakening a filter?** — No filter is being removed. We are *adding* a trigger condition (soft threshold) that currently does not exist, standing in for the missing substrate "agents should maintain a degraded shared facility before it becomes unusable." The hard `max_effective_dirtiness` gate is untouched, so no unrelated wash-legality regression is reopened.
12. **Scenario isolation note for the new golden**: the existing `survival-sanitation-breakdown-1440.ron` intentionally zeros food/water to isolate the sanitation loop — which is exactly why it cannot prove proactive cleaning *under competition*. The new golden (Acceptance) must keep realistic food/water pressure present so the agent must trade off the cleaning trip against eating/drinking.
13. **Adjacent contradiction classification**: the S176 spec claim "Default … leaves existing scenarios behaviorally unchanged until they author dirt" (`archive/specs/S176-…:105`) is contradicted by the default `dirtiness_per_use: 50` + no decay (`crates/worldwake-core/src/place_dirtiness.rs`). That mismatch is the *symptom*; this ticket fixes the *root* (dampener engagement). Separately recalibrating defaults or opting individual scenarios out was considered and **rejected** (see Architecture Check). The `golden-scenario-diagnostics` fixture drift from the same S176 change is an independent, already-resolved Sub-path B regeneration (committed separately), not part of this ticket.
15. **Cumulative arithmetic / survivability envelope**: dirtiness accrues at `dirtiness_rate` (1‰/tick in `scattered`), so ~1440‰ over the day; basin `dirtiness_per_use` default is 50‰/wash with no decay (`place_dirtiness.rs`). For the dampener to keep an agent recoverable, the planner must trigger cleaning while effectiveness is still high enough that the post-clean wash clears more dirtiness than accrues during the round-trip. The new golden asserts the basin's `dirtiness_level` crosses the trigger and is reset by `clean_wash_basin` at least once, and that no agent exceeds the authored `dirtiness` critical-run limit.

16. **Threshold home decision (resolved)**: the floor lives on `MetabolismProfile` as `wash_worthwhile_effectiveness_floor: Permille`, default 500‰ (clean when a wash would deliver < 50% relief), serde-defaulted so existing scenarios engage the dampener with no edit and no `types.rs` change. The per-facility `WashBasinState.clean_trigger_dirtiness` alternative was rejected: it adds authoritative state for what is fundamentally a per-agent planner policy (FND-2/FND-3 — model the agent's decision parameter, not a clamp on the facility).

17. **Cross-golden interference (resolved)**: with the default floor engaged, `survival-sanitation-breakdown-1440.ron` (authored `max_effective_dirtiness: 800`, basin starting clean) would be cleaned proactively at `dirtiness > 400` and never reach the hard block, so its `DegradedSelfCareCause::BasinTooDirty` forensic (which keys on a `TargetWashBasinNotTooDirty` precondition failure) would never fire. Fix: set `wash_worthwhile_effectiveness_floor: 0` on that scenario's agents — it specifically exercises the HARD-block path, and disabling the soft floor there keeps that regime under test while the new `survival-basin-competition-1440.ron` proves the proactive (soft-floor) path. `survival-basin-dirty-dirty.ron` is unaffected: its basin starts at 900 (already above the hard block), so the first clean is hard-block-driven and its `BasinTooDirty/Cleaned` forensic still fires.

18. **`scattered` re-diagnosis (resolved — scope correction)**: observer-harness tracing on the landed branch DISPROVED the ticket's original causal story for `survival_scattered`. Agent B's `dirtiness` runs away starting ~tick 400 while the shared basin is still **clean** (~350‰, well below the 500‰ trigger); the basin only crosses 500 at ~tick 701, after B has already failed. After degradation, agents are almost never co-located with the basin (A:3, B:1, C:33 ticks). B makes only 7 `Wash` plan attempts in 1440 ticks (4 frontier-exhausted: the planner cannot find the multi-hop `[travel, travel, wash]` to the far basin within budget, then structurally blocks the goal). B's failure is wash **frequency** for a distant agent + a remote basin, not the co-located degradation dead-zone — even a 45%-effective wash at 550‰ takes B from 1000→550 (below critical). The proactive-cleaning dampener is therefore necessary and proven for the genuine dead-zone (`survival-basin-competition-1440`) but **cannot** fix `scattered`. Per the FOUNDATIONS preamble (comprehensive solutions, clean architectural separation), `scattered`'s distinct distant-self-care/planner-reachability failure is owned by the follow-up ticket `SCATSELFCARETRIP-001`; this ticket does **not** relax the `scattered` contract.

## Architecture Check

1. **Why proactive cleaning is the right fix (vs. the alternatives considered):**
   - *Neutralize the basin default (`dirtiness_per_use: 0`)*: makes degradation opt-in-only, reducing default emergence — the S176 FND-1 headline "a camp's only basin grows filthy under multi-agent use" would no longer happen by default. Anti-FND-1, and it does not fix the dampener gap (any authored-degradation scenario under competition still stalls). Rejected.
   - *Relax `scattered`'s dirtiness contract*: enshrines the undampened loop as acceptable ("agents stay permanently dirty"). Direct FND-11 violation. Rejected.
   - *Author `scattered`'s basin to opt out*: green CI, but per the FOUNDATIONS preamble it is "a localized fix that avoids the real problem" — it removes one scenario from the loop while leaving the engine gap for every other realistic scenario. Rejected as the fix (a scenario may still legitimately author a non-degrading basin for unrelated design reasons, but that is not how this regression is resolved).
   - *This ticket — proactive cleaning*: makes the spec's named dampener ("cleaning labor lowers the state — an agent can always restore the facility", S176 Section H dampener (a)) actually engage in the regime where it matters, satisfying FND-11 with a concrete world process (maintenance labor) rather than a numeric clamp. It is the architecturally comprehensive solution.
2. **No backwards-compatibility shims**: the existing full-block insertion path is generalized, not duplicated. The hard `max_effective_dirtiness` precondition remains the single authoritative legality gate; the soft trigger is a planner-side refinement layered above it, with no alias or dual legality path.

## Verification Layers

1. Planner inserts `clean_wash_basin` before `wash` when the basin is degraded past the agent's worthwhile floor (but below the hard block) → **decision trace** (plan for `GoalKind::Wash` contains `[clean_wash_basin, wash]`) on a focused runtime test.
2. `clean_wash_basin` action lifecycle (start/commit, occupancy held/released) → **action trace**.
3. Basin `dirtiness_level` crosses the trigger and is reset to ZERO by the cleaning commit; agent `dirtiness` recovers below critical thereafter → **authoritative world state** / event-log delta (`WashFacilityUsed` + cleaning commit + basin state).
4. End-to-end survival under realistic competition → **golden E2E**: the originally-regressed `survival_scattered::all_agents_survive_1440_ticks` plus a new competition-sanitation golden (below).
5. Determinism → the new scenario's `_replays_deterministically` golden (state-affecting planner change must preserve replay equivalence).

## What to Change

### 1. Profile-driven worthwhile-wash threshold (`worldwake-core`)

Add the chosen threshold parameter (preferred: a wash-worthwhile / minimum-acceptable-effectiveness floor on `MetabolismProfile`, `crates/worldwake-core/src/needs.rs`, with a `Default` and serde-defaulted scenario contract via `AgentDef.metabolism_profile`; alternative: a `clean_trigger_dirtiness` field on `WashBasinState`, `crates/worldwake-core/src/place_dirtiness.rs`, plus the `WashBasinStateDef` contract). Decide and justify the home in the implementation's reassessment. No hardcoded threshold constant anywhere.

### 2. Proactive cleaning gate in the `Wash` plan search + revalidation (`worldwake-ai`)

In `crates/worldwake-ai/src/search/candidates.rs`, the shared `wash_basin_below_worthwhile_floor` helper (generic over the belief view) drives two gates: (a) `goal_synthesized_candidates` refuses to synthesize a below-floor wash root, and (b) the affordance filter suppresses a co-located below-floor wash affordance — both forcing the search to insert `clean_wash_basin` first. Re-exported `pub(crate)` from `search/mod.rs`. The hard `TargetWashBasinNotTooDirty` block is untouched; belief-backed discipline preserved (no proactive cleaning for a fully-unknown remote basin — `search_does_not_synthesize_clean_for_remote_unknown_basin` still passes).

**Scope extension** (reassessment §18): the same gate is applied at plan revalidation (`crates/worldwake-ai/src/plan_revalidation.rs`) so a committed wash whose basin is below the floor at co-located arrival is invalidated, replanning into clean-then-wash. This is the matching surface for plans committed while the remote belief was still clean. (Note: tracing showed this is not what `scattered` needs — see §18 — but it is the correct completion of the planner-side dampener and is exercised by the co-located competition golden.)

### 3. New competition-sanitation golden (`scenarios/survival-basin-competition-1440.ron`)

A 1440-tick golden where a single shared default-scale basin (`max_effective_dirtiness: 1000`) degrades under **realistic** food/water competition (hunger/thirst not zeroed), proving agents proactively clean and stay recoverable, and that the basin never reaches its hard block (so the recovery is the *proactive* trigger). `survival_scattered` re-green is reassigned to follow-up `SCATSELFCARETRIP-001` (reassessment §18).

## Files to Touch (as landed)

- `crates/worldwake-core/src/needs.rs` (modify — `wash_worthwhile_effectiveness_floor: Permille` on `MetabolismProfile`, serde-defaulted to 500‰, seeded in `new()`/`Default`; unit tests). **No `crates/worldwake-cli/src/scenario/types.rs` change needed** — `AgentDef.metabolism_profile: Option<MetabolismProfile>` deserializes the core type directly, so the `#[serde(default)]` field is the contract (the `clean_basin_duration_ticks` precedent). Auto-correction: the original ticket listed `place_dirtiness.rs` (alternative) and `types.rs`; neither was touched.
- `crates/worldwake-ai/src/search/candidates.rs` (modify — `wash_basin_below_worthwhile_floor` helper + search-time gate in `goal_synthesized_candidates` and the affordance filter). Auto-correction: the gate lives in `search/candidates.rs` (the live synthesized/affordance surface), not `goal_model.rs`/`candidate_generation.rs` as the original ticket guessed.
- `crates/worldwake-ai/src/search/mod.rs` (modify — `pub(crate)` re-export of the helper).
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — revalidation-time gate so a committed wash at a below-floor basin replans into clean-then-wash on co-located arrival). **Scope extension** (see reassessment §16).
- `crates/worldwake-ai/src/search/tests.rs` (modify — focused proactive-clean plan-shape test).
- `scenarios/survival-basin-competition-1440.ron` (new — competition-sanitation golden scenario).
- `crates/worldwake-ai/tests/scenarios/survival_basin_competition.rs` + `crates/worldwake-ai/tests/scenarios/mod.rs` (new/modify — golden).
- `scenarios/survival-sanitation-breakdown-1440.ron` (modify — `wash_worthwhile_effectiveness_floor: 0` on its agents to preserve its hard-block forensic contract; see reassessment §17).
- `.github/workflows/golden-survival.yml` (modify — register `basin_competition` in the matrix). **No `docs/scenario-roadmap.md` change** — the sibling sanitation goldens are not roadmap feature rows; the new scenario is a CI-owned golden, consistent with `survival-sanitation-breakdown-1440`.
- `docs/generated/golden-e2e-inventory.md`, `docs/generated/scenario-coverage.md` (regenerated — new scenario/golden).
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (regenerated — the default floor changes survival-baseline wash behavior over 1440 ticks).

## Out of Scope

- Changing the authoritative `apply_wash` effectiveness formula or the `max_effective_dirtiness` hard precondition (the hard block stays as-is).
- Latrine analog (`empty_latrine` proactive trigger) — file a sibling ticket if `LatrineFullness` shows the same dead-zone under competition; the latrine has a binary fill gate (no continuous effectiveness scaling), so it likely does not, but confirm.
- Basin natural decay — explicitly rejected by S176 design (grime requires labor); do not add it as a shortcut dampener.
- Recalibrating `WashBasinState`/`PlaceDirtiness` defaults or opting any scenario out of degradation.

## Acceptance Criteria

### Tests That Must Pass

1. Focused runtime: a `GoalKind::Wash` plan at a degraded-but-not-blocked basin contains `clean_wash_basin` before `wash` (decision trace). → `search::tests::search_inserts_proactive_clean_at_basin_below_worthwhile_floor`.
2. New competition-sanitation golden: all agents stay within authored `dirtiness` critical-run limits over 1440 ticks; basin `dirtiness_level` is reset by `clean_wash_basin` at least once under live food/water competition; the basin stays below its hard block (proving the *proactive* trigger). → `scenarios::survival_basin_competition::proactive_cleaning_keeps_agents_recoverable_under_competition`.
3. New scenario `_replays_deterministically` golden. → `scenarios::survival_basin_competition::proactive_cleaning_competition_is_replay_deterministic`.
4. **Corrected (see reassessment §18)**: `survival_scattered` is NOT a valid acceptance vehicle for this ticket — its Agent B failure is a distinct distant-agent/remote-basin wash-frequency problem (B fails at a *clean* basin ~tick 400, before any degradation), not the FND-11 dead-zone this ticket fixes. `scattered` remains red (pre-existing S176 regression) and is owned by follow-up `SCATSELFCARETRIP-001`. This ticket does not relax the `scattered` contract.
5. Whole gated family `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1` passes **except** `survival_scattered::all_agents_survive_1440_ticks` (the separate, follow-up-owned failure above). All other survival/sanitation goldens green under the engaged default floor (per the Authoritative-to-AI Impact Rule).
6. `./scripts/verify.sh`.

### Invariants

1. The basin-degradation feedback loop has a dampener (`clean_wash_basin`) that engages under realistic competition before the basin becomes unusable (FND-11).
2. The hard authoritative wash-legality gate (`TargetWashBasinNotTooDirty` at `max_effective_dirtiness`) is unchanged; the proactive trigger is planner-side only, with no dual legality path (no-backcompat).
3. The threshold is a concrete scenario-authored profile parameter, never a hardcoded constant (FND-2/FND-3).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/<new competition-sanitation>.rs` — proves proactive cleaning + survival under realistic food/water competition (the gap `survival-sanitation-breakdown-1440` cannot cover because it zeros food/water).
2. Focused runtime test in the planner/goal-model test module — asserts the `[clean_wash_basin, wash]` plan shape at a degraded-but-unblocked basin.
3. `survival_scattered` golden — re-confirmed green with the default basin (regression guard for the originating failure).

### Commands

1. `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1` (full gated `golden-survival` family)
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-29.

**What changed**
- `worldwake-core`: added `MetabolismProfile.wash_worthwhile_effectiveness_floor: Permille` (serde-defaulted 500‰, seeded in `new()`/`Default`). No CLI scenario-types change (the core type deserializes directly).
- `worldwake-ai`: added `wash_basin_below_worthwhile_floor` (generic over the belief view) in `search/candidates.rs`, gating both the synthesized wash root and the co-located wash affordance so the search inserts `clean_wash_basin` first when the basin is below the agent's worthwhile floor (but below the hard block). Re-exported from `search/mod.rs` and applied at `plan_revalidation.rs` so a committed wash at a below-floor basin replans into clean-then-wash on co-located arrival. The hard `TargetWashBasinNotTooDirty` block is unchanged.
- New golden `scenarios/survival-basin-competition-1440.ron` + `tests/scenarios/survival_basin_competition.rs`: 3 agents share one default-scale basin under live food/water competition; proves proactive cleaning engages, the basin crosses the floor and is reset, all survive, and the basin never reaches its hard block. Plus a focused planner test `search::tests::search_inserts_proactive_clean_at_basin_below_worthwhile_floor`.
- `survival-sanitation-breakdown-1440.ron`: `wash_worthwhile_effectiveness_floor: 0` on its agents to preserve its hard-block forensic contract (reassessment §17).
- Registered `basin_competition` in `.github/workflows/golden-survival.yml`; regenerated `docs/generated/golden-e2e-inventory.md`, `docs/generated/scenario-coverage.md`, and `tests/fixtures/expected-scenario-diagnostics.json`.

**Deviations from the original ticket**
- Threshold home: `MetabolismProfile` (not `WashBasinState`); no `types.rs` change (reassessment §16).
- Gate lives in `search/candidates.rs` + `plan_revalidation.rs`, not `goal_model.rs`/`candidate_generation.rs` as guessed.
- **Scope correction (reassessment §18)**: `survival_scattered` is NOT fixed by this ticket and was reassigned to follow-up `SCATSELFCARETRIP-001`. Observer tracing disproved the ticket's causal story for scattered: Agent B's dirtiness runs away at ~tick 400 while the basin is still clean (~350‰); the basin only crosses the 500‰ trigger at ~tick 701; B makes only 7 Wash attempts (4 frontier-exhausted) — a distant-agent/remote-basin/planner-reachability failure, not the co-located degradation dead-zone. The `scattered` contract was NOT relaxed (FND-11).

**Verification**
- Passed `cargo test -p worldwake-core` (incl. new `wash_worthwhile_*` unit tests), `cargo test -p worldwake-sim -p worldwake-systems`.
- Passed `cargo test -p worldwake-ai` (1781 lib + non-ignored goldens incl. `survival-basin-dirty-dirty`).
- Passed the new competition golden + focused planner test (release, `--ignored`).
- Passed `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check`.
- Full gated `golden-survival` family (`--ignored`) green **except** `survival_scattered::all_agents_survive_1440_ticks` (the follow-up-owned distant-self-care failure); the regenerated diagnostics fixture and replay goldens pass.
