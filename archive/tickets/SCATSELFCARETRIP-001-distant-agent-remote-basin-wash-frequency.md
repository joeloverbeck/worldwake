# SCATSELFCARETRIP-001: Distant agent cannot sustain self-care trips to a remote single shared facility (`survival_scattered` Agent B dirtiness runaway)

**Status**: COMPLETED
**Priority**: HIGH
**Engine Changes**: Yes — the binding root cause was Wash-goal **structural blocking after frontier exhaustion** (`PermanentUntilInvalidator`) whose derived invalidation conditions omitted `PositionChanged`, permanently suppressing the goal for a distant agent. Fix is a one-line condition addition in `crates/worldwake-ai/src/exhaustion.rs`. No planner-search/budget/heuristic change and no goal-valuation (§2) change were needed (see reassessment §6). No authoritative-action change.
**Deps**: `archive/specs/S176-sanitation-facility-degradation-consequences.md` (introduced wash-effectiveness scaling + basin degradation). Sibling: `tickets/SANBASINCLEAN-001-*` (landed the proactive-cleaning FND-11 dampener for the *co-located* dead-zone; this ticket owns the *remote/distant* failure that SANBASINCLEAN-001 reassessment proved is a separate root cause).

## Problem

`golden-survival / scattered` (`survival_scattered::all_agents_survive_1440_ticks`) fails: **Agent B** keeps `dirtiness` above its authored critical threshold for **1028 consecutive ticks** vs. the authored limit of 680 (`scenarios/survival-scattered.ron:22`). The single shared `Crossing Basin` is at `River Crossing`; Agent B starts isolated at `Ravine Shelter` and spends most of the run at `Orchard Hollow` (food), far from the basin.

SANBASINCLEAN-001 originally attributed this to the FND-11 basin-degradation dead-zone (basin climbs to ~550, becomes ineffective, agents abandon it). **Reassessment during SANBASINCLEAN-001 implementation disproved that causal story for scattered** (traced with the observer harness on the landed branch):

1. The basin first exceeds the proactive-clean trigger (`dirtiness_level > 500`) only at **tick ~701**. Agent B's dirtiness already runs away starting **~tick 400**, while the basin is still clean (~350‰). So B's runaway begins *before* any basin degradation — the degradation is a later, secondary effect of A/C's occasional use, not the cause of B's failure.
2. After the basin degrades, agents are almost never co-located with it (over the ~740 remaining ticks: Agent A = 3 ticks, Agent B = 1 tick, Agent C = 33 ticks co-located while basin > 500). So the co-located proactive-cleaning dampener has essentially no opportunity to engage in this topology.
3. Agent B makes only **7 `Wash` plan attempts** across 1440 ticks despite `dirtiness == 1000` (max critical) for ~1000 ticks; **4 of the 7 frontier-exhaust** — the planner cannot find the multi-hop `[travel, travel, wash]` plan to the far basin within Agent B's node-expansion budget. Repeated failures then structurally block the `Wash` goal (`structural_block_ticks: 200`), further starving wash attempts.
4. B's failure is wash **frequency**, not wash effectiveness: even a 45%-effective wash at a 550‰ basin takes B from 1000→550 (below the 900 critical threshold). If B washed roughly every ~350 ticks it would stay under the 680-tick limit. B simply does not make the trips.

So the binding constraint is a **distant agent failing to sustain self-care trips to a remote single shared facility under food/water competition** — a planner reachability / goal-management / goal-valuation concern, orthogonal to the SANBASINCLEAN-001 dampener (which is proven by `survival-basin-competition-1440.ron`).

The `scattered` `dirtiness` critical-run contract (680, `survival-scattered.ron:22`) is the correct contract and must **not** be relaxed (FND-11; the agent can reach a usable basin and must be able to stay recoverable over time).

## Assumption Reassessment (2026-05-29)

<!-- This ticket inherits a partially-verified reassessment from SANBASINCLEAN-001's observer-harness trace. Re-confirm the planner symbols before implementation. -->

1. **Live `GoalKind` under test**: `GoalKind::Wash`. The failing surface is plan *search* (multi-hop travel to a remote `WashBasin`) plus `Wash`-goal blocking/valuation, not candidate emission (`emit_wash_goal` does generate the goal — B records 7 attempts).
2. **Frontier exhaustion**: re-confirm against `crates/worldwake-ai/src/search/` (frontier/heuristic/landmarks) whether the multi-hop `[travel, travel, wash]` plan exhausts B's `max_node_expansions` (640 in `survival-scattered.ron`) or the travel-branch cap (`max_travel_candidates_per_expansion: 4`). Decide whether the fix is a heuristic/landmark improvement, a budget bump, or a travel-pruning change — not a naked constant bump (FND-2).
3. **Structural blocking**: confirm whether repeated `Wash` frontier exhaustion drives `structural_block_ticks` blocking that suppresses later attempts even when `dirtiness` is critical. If a critical survival need is being structurally blocked, name the exact blocking surface (`BlockerMemory` / failure_handling) and decide the correct policy.
4. **Goal valuation under S176 effectiveness scaling**: confirm whether the `Wash` goal's expected-relief valuation incorporates the (believed) basin effectiveness, and whether a partially-dirty remote basin deprioritizes the wash trip against hunger/thirst. If so, decide whether degraded-effectiveness should lower trip priority (valid) or whether critical dirtiness should still command the trip.
5. **Belief currency**: B believes the basin is clean (200‰, stale) for ~1000 ticks. Confirm this is lawful locality (FND-7/FND-15) and that the fix does not require omniscient remote-facility reads.

### Implementation reassessment findings (2026-05-29, post-trace)

Confirmed with a focused per-tick decision-trace diagnostic over the real 1440-tick `survival-scattered` run (Agent B):

6. **Root cause isolated — structural blocking, not search budget.** Over 1440 ticks B *generates* a Wash candidate on **481 ticks** but reaches plan *search* only **7 times**, the last at **t413**; after t413 it makes **zero** Wash attempts for ~1000 ticks while `dirtiness == 1000`. The gap is the exhaustion cache: `GoalKind::Wash` declares `FrontierExhaustionStrategy::PermanentUntilInvalidator` (`goal_schema.rs:385`), so each of B's 4 frontier-exhaustions writes a `FrontierExhausted` cache entry that `suppresses_planning()` until an invalidation condition fires (`decision_runtime.rs:135`). Wash's invalidation strategy is `NeedWithFacilities(Dirtiness)`, whose derived conditions were `{ NeedChangedBands{Dirtiness}, FacilitiesChanged }` (`exhaustion.rs:161`). Once dirtiness saturates at its **Critical** band it can never change band, and B's believed basin set is stable — so **neither invalidator can ever fire** and the goal is suppressed permanently. The ticket's original "search budget / heuristic / travel-pruning" framing (and the `max_node_expansions: 640` assumption) was wrong: the `Wash` effective budget is actually `GoalPlanningBudget::SELF_CARE` (`max_node_expansions: 96`), and the sibling golden `no_budget_exhaustion_on_survival_goals` proves Wash never hits `BudgetExhausted`. The failures are `FrontierExhausted`, not budget.
7. **The missing condition is `PositionChanged`.** A facility-satisfied need is travel-gated: the facility sits at a remote place the actor must reach, so a frontier exhaustion reflects the actor's *position at search time*, not a permanent impossibility. Every other position-dependent goal includes `PositionChanged` in its invalidators; in particular `ProduceCommodity` — the *other* facilities-gated goal — already derives **both** `PositionChanged` **and** `FacilitiesChanged` (`produce_commodity_conditions`, `exhaustion.rs:220`). `need_with_facilities_conditions` (shared by `Wash` and `Sleep`) was the lone facilities-gated family omitting it. `Sleep` did not exhibit the bug only because it uses `CooldownRetry` (it re-attempts on a backoff regardless); `Wash`'s `PermanentUntilInvalidator` exposed the gap. Adding `PositionChanged` is the architecturally-consistent fix (FND-11: the dampener — re-planning the trip when the agent moves within reach — is restored). `PositionChanged` only fires on arrival (`!currently_in_transit`), so there is no per-tick thrash.
8. **Problem (B) — frontier exhaustion at far positions — is real but non-fatal.** When B *does* search Wash it sometimes frontier-exhausts after only 1–3 expansions from a far/poorly-perceived position, but it finds the `[travel, …, wash]` plan from many other positions. Post-fix tracing shows the exhaustion entry is lifted on B's next arrival and the goal is re-searched: e.g. `FrontierExhausted@t236 → Found@t279`. Across the run B's Wash attempts rise 7 → 34 and are spread evenly t38…t1439, and **critical-dirtiness ticks fall from 1028 → 0**. So the residual single-position search weakness needs no separate fix to satisfy the `scattered` contract; it is left as-is (lawful locality — at some positions B genuinely cannot perceive a path) rather than forcing a heuristic/budget change the evidence does not require (FOUNDATIONS preamble: do not over-engineer).
9. **§2 goal valuation not implicated.** B believes the basin is *clean* (200‰) the whole run, so the S176 effectiveness-scaling valuation never deprioritises the trip. The conditional §2 work is therefore **not** performed; the `ranking.rs` / valuation surface is untouched.

## Architecture Check

1. This is a distinct architectural concern from SANBASINCLEAN-001 (co-located dampener). Bundling a planner-search/goal-management fix into the dampener ticket would conflate two subsystems; keeping them separate preserves clean architectural boundaries (FOUNDATIONS preamble).
2. No authoritative-action or backward-compat change expected; the fix is planner-side. Do not relax the `scattered` contract and do not author the basin closer / co-locate it as "the fix" (that would avoid the real distant-self-care problem — FOUNDATIONS preamble).

## Verification Layers

1. Agent B sustains enough `Wash` trips to the remote basin to stay within the authored `dirtiness` critical-run limit → **golden E2E** (`survival_scattered::all_agents_survive_1440_ticks`). ✅ critical-dirtiness ticks 1028 → 0.
2. A frontier-exhausted `Wash` goal for a distant agent at saturated (critical-band) dirtiness is **not** permanently suppressed — the structural block lifts when the agent arrives at a new place, so the trip is re-planned → **focused exhaustion-invalidation unit test** (`exhaustion::tests::wash_frontier_exhaustion_lifts_only_after_the_distant_agent_moves`). (Corrected from the original "planner/search reachability" framing: the binding surface is exhaustion invalidation, not plan search — reassessment §6–§8.)
3. Determinism preserved → existing `survival_scattered` replay/diagnostics coverage and the full gated `golden_ai` family.

## What to Change

### 1. Add `PositionChanged` to the facilities-gated need invalidators (`worldwake-ai`)

In `crates/worldwake-ai/src/exhaustion.rs`, `need_with_facilities_conditions` (shared by `Wash` and `Sleep`) inserts `ExhaustionInvalidationCondition::PositionChanged` alongside the existing `NeedChangedBands` + `FacilitiesChanged`. This mirrors `produce_commodity_conditions`, the other facilities-gated family. It lets a frontier-exhausted (`PermanentUntilInvalidator`) `Wash` goal be re-searched once the distant agent moves within reach, instead of staying suppressed forever while its need is pinned at the critical band and its believed facilities are stable. See reassessment §6–§7.

### ~~2. Goal valuation of degraded remote facilities~~ — NOT NEEDED

Reassessment §9 disproved this path: B believes the basin is clean (200‰) for the whole run, so the S176 effectiveness-scaling valuation never deprioritises the trip. No `ranking.rs` / valuation change.

## Files to Touch (as landed)

- `crates/worldwake-ai/src/exhaustion.rs` (modify — `need_with_facilities_conditions` adds `PositionChanged`; new focused unit test `wash_frontier_exhaustion_lifts_only_after_the_distant_agent_moves`).
- `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` — re-confirmed green; **no change landed** (a temporary diagnostic test was used during reassessment and removed). Auto-correction: the original ticket guessed `search/*`, `failure_handling.rs`, and `ranking.rs`; none were touched — the binding surface is exhaustion invalidation.

## Out of Scope

- The co-located proactive-cleaning dampener (owned and landed by SANBASINCLEAN-001).
- Relaxing the `scattered` `dirtiness` critical-run contract.
- Authoring the basin closer to agents or co-locating it (scenario opt-out is not the fix).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored --test-threads=1` (all agents within authored critical-run limits, including Agent B `dirtiness`).
2. Focused planner test: a distant agent with critical dirtiness and a known remote basin finds a `[travel, …, wash]` plan within realistic budgets.
3. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1` (whole gated family stays green per the Authoritative-to-AI Impact Rule).
4. `./scripts/verify.sh`.

### Invariants

1. A distant agent that can lawfully reach a remote shared self-care facility can sustain enough trips to stay within its authored need critical-run limits (no permanent-critical runaway from planner unreachability).
2. The fix is planner-side; no authoritative-action change and no relaxation of the `scattered` contract.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` — focused `wash_frontier_exhaustion_lifts_only_after_the_distant_agent_moves`: derives Wash invalidators (asserts `PositionChanged` present), builds a frontier-exhausted entry, proves it persists while the agent stays put with a saturated need and clears on arrival at a new place. (Corrected from `search/tests.rs` — reassessment §6–§8.)
2. `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` — re-confirm `all_agents_survive_1440_ticks` green.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-29.

**What changed**
- `worldwake-ai/src/exhaustion.rs`: `need_with_facilities_conditions` now also inserts `ExhaustionInvalidationCondition::PositionChanged`. This restores the FND-11 dampener for travel-gated facility needs: a `Wash` goal (which uses `PermanentUntilInvalidator`) that frontier-exhausts while the distant agent is far from the only basin is now re-searched once the agent arrives somewhere new, instead of being suppressed forever because its dirtiness is pinned at the critical band and its believed facilities are stable. `Sleep` (the other `NeedWithFacilities` family) shares the helper; it already retried via `CooldownRetry`, and now also clears on arrival — strictly more responsive, benign.
- Added focused unit test `exhaustion::tests::wash_frontier_exhaustion_lifts_only_after_the_distant_agent_moves`.
- No other files changed. A temporary per-tick decision-trace diagnostic in `survival_scattered.rs` was used to isolate the root cause (481 candidate-gen ticks vs 7 plan attempts; last attempt t413) and removed.

**Deviations from the original ticket (all recorded in reassessment §6–§9)**
- Binding surface is **exhaustion invalidation** (`exhaustion.rs`), not plan search / heuristic / budget (`search/*`), failure handling, or ranking as the original ticket guessed. No naked constant bump.
- The original `max_node_expansions: 640` premise was wrong — `Wash` uses `GoalPlanningBudget::SELF_CARE` (96); failures were `FrontierExhausted`, not budget.
- §2 (degraded-facility valuation) was disproved and not implemented.
- The focused test lives in `exhaustion.rs`, not `search/tests.rs`.

**Verification**
- Passed `cargo test -p worldwake-ai --lib` (1782 incl. new exhaustion test) and `cargo test -p worldwake-ai --lib exhaustion`.
- Passed `survival_scattered::all_agents_survive_1440_ticks` (release, `--ignored`); diagnostic confirmed Agent B critical-dirtiness ticks 1028 → 0 and Wash attempts 7 → 34 spread across the full run.
- Full gated `golden_ai --ignored` family + `./scripts/verify.sh`: see close-out notes.
