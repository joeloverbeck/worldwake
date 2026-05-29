# SCATSELFCARETRIP-001: Distant agent cannot sustain self-care trips to a remote single shared facility (`survival_scattered` Agent B dirtiness runaway)

**Status**: PENDING
**Priority**: HIGH
**Engine Changes**: Yes — likely planner search budget / heuristic for multi-hop self-care, Wash-goal structural-blocking after frontier exhaustion, and/or goal valuation of a degraded remote facility (`worldwake-ai`). No authoritative-action change expected.
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

## Architecture Check

1. This is a distinct architectural concern from SANBASINCLEAN-001 (co-located dampener). Bundling a planner-search/goal-management fix into the dampener ticket would conflate two subsystems; keeping them separate preserves clean architectural boundaries (FOUNDATIONS preamble).
2. No authoritative-action or backward-compat change expected; the fix is planner-side. Do not relax the `scattered` contract and do not author the basin closer / co-locate it as "the fix" (that would avoid the real distant-self-care problem — FOUNDATIONS preamble).

## Verification Layers

1. Agent B sustains enough `Wash` trips to the remote basin to stay within the authored `dirtiness` critical-run limit → **golden E2E** (`survival_scattered::all_agents_survive_1440_ticks`).
2. The multi-hop `[travel, …, wash]` plan to a remote known basin is found (not frontier-exhausted) for a distant agent under realistic budgets → **focused planner/search test**.
3. Determinism preserved → existing `survival_scattered` replay/diagnostics coverage.

## What to Change

### 1. Make remote multi-hop self-care reachable for distant agents (`worldwake-ai`)

Diagnose and fix the frontier exhaustion on `[travel, travel, wash]` to a remote `WashBasin` (heuristic/landmark/travel-pruning or budget policy — per reassessment, not a naked constant). Ensure critical survival needs are not starved by structural blocking after transient plan-search failure.

### 2. (If reassessment confirms) goal valuation of degraded remote facilities

If the `Wash` goal trip is deprioritized because the believed basin is partially dirty, reconcile the valuation so an agent at critical dirtiness still commits the trip (cleaning the basin on arrival via the SANBASINCLEAN-001 dampener as needed).

## Files to Touch

- `crates/worldwake-ai/src/search/*` (modify — multi-hop reachability / heuristic / budget policy, per reassessment)
- `crates/worldwake-ai/src/failure_handling.rs` (modify if structural blocking of critical-need goals is the binding issue)
- `crates/worldwake-ai/src/ranking.rs` / goal valuation (modify only if §2 is confirmed)
- `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` (re-confirm green; possibly add a focused multi-hop self-care planner test)

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

1. `crates/worldwake-ai/src/search/tests.rs` — focused multi-hop remote self-care reachability test (distant actor, known remote basin).
2. `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` — re-confirm `all_agents_survive_1440_ticks` green.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`
3. `./scripts/verify.sh`
