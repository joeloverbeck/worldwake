# S150CROGOABLO-006: Cross-goal blocker golden coverage

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test coverage only (new golden file)
**Deps**: S150CROGOABLO-001, S150CROGOABLO-002, S150CROGOABLO-003

## Problem

The S150 substrate migration (tickets 002, 003) and the per-scope TTL profile (ticket 001) need golden E2E coverage that proves the cross-goal blocker semantics work end-to-end through the live planner. The existing blocker goldens (`golden_portfolio_planning`, `golden_plan_repair`, `golden_contention_inspectability`, `golden_need_projection`) exercise only `BlockerScope::Exact(...)`-style blockers — they continue to pass after the migration because Exact-scope semantics are preserved, but they do not prove that a single `RouteSegment` blocker actually suppresses multiple goals or that a `Counterparty` blocker affects both `BuyCommodity` and `AskWitness` candidates. Without dedicated cross-goal coverage, a future regression that re-fragments cross-goal blockers (e.g., a refactor that accidentally treats `RouteSegment` blockers as goal-keyed) would slip past the existing golden suite.

This ticket adds `golden_cross_goal_blocker_scoping.rs` covering the 8 scenarios enumerated in S150 D10, exercising the full agent decision pipeline (candidate generation → feasibility probe → search → execution → recording).

## Assumption Reassessment (2026-05-17)

1. Golden E2E test harness lives in `crates/worldwake-ai/tests/` with shared utilities in `crates/worldwake-ai/tests/golden_harness/`. Pattern precedents:
   - `golden_portfolio_planning.rs` for scenarios that set up blocker memory and assert suppression
   - `golden_plan_repair.rs` for scenarios that exercise blocker recording during execution
   - `golden_need_projection.rs` for scenarios that assert per-scope behavior through trace surfaces
   Canonical golden authoring guide: `docs/golden-e2e-testing.md`.
2. Spec source: `specs/S150-cross-goal-blocker-scoping.md` D10's 8-scenario enumeration. The scenarios map to:
   - **Scenario A (RouteSegment multi-goal suppression)**: agent has `BlockerScope::RouteSegment(thornwall ↔ ashford)`; emits both `AcquireCommodity` (travel-trade to ashford) and `EscortToSafety` (travel-escort along same segment); assert both candidates suppressed in decision trace.
   - **Scenario B (Counterparty multi-goal suppression)**: agent has `BlockerScope::Counterparty(merchant_42)`; emits both `BuyCommodity` (trade with merchant_42) and `AskWitness` (Tell to merchant_42); assert both candidates suppressed.
   - **Scenario C (TTL expiry restores emission)**: scenario from A or B, advance ticks past `route_segment_blocker_ticks` (240) or `counterparty_blocker_ticks` (360); assert candidates resume emission.
   - **Scenario D (RouteRetraversedSafely clearing)**: scenario from A; agent traverses the segment safely (no danger event); assert `sweep_cleared` removes the RouteSegment blocker mid-TTL.
   - **Scenario E (CounterpartyAccepted clearing)**: scenario from B; agent completes a successful trade with merchant_42; assert Counterparty blocker cleared.
   - **Scenario F (DiscrepancyMemory parallel suppression)**: agent has `Discrepancy::RouteUnknown` keyed by `BlockerScope::RouteSegment(...)`; multiple goals affected by the same route are suppressed at the discrepancy gate.
   - **Scenario G (source_event provenance)**: trigger a blocker recording at each of the three recording sites; assert `Blocker.source_event` points to a real event in the agent's event log.
   - **Scenario H (Determinism)**: same scenario seed reproduced twice; assert `BlockerMemory.intents` byte-identical at the same tick.
3. Live planner surfaces: each scenario names the live `GoalKind` under test:
   - Scenarios A, C, D: `GoalKind::AcquireCommodity`, `GoalKind::EscortToSafety`
   - Scenarios B, E: `GoalKind::BuyCommodity`, `GoalKind::AskWitness`
   - Scenario F: `GoalKind::AcquireCommodity` (or analogous travel-bearing goal) with DiscrepancyMemory suppression
   - Scenarios G, H: any single GoalKind sufficient to trigger one recording-site
   The current operator surface for these goal kinds was confirmed during ticket 002's recording-site enumeration; no divergence expected. Per `docs/precision-rules.md` Rule 13, if the planner's emit-site for any named GoalKind differs from the spec's narrative at implementation time, correct the ticket scope before writing the scenario fixture.
4. Harness boundary: full action registries are required because the scenarios exercise multi-goal candidate flows across travel + trade + Tell. Local needs-only harness is insufficient. Per `docs/precision-rules.md` Rule 3.
5. Adjacent contradictions classified: (a) D9's "trait-bound regression tests" are split — the trait-bound portion landed in ticket 002 alongside the type definitions; the integration-side migration goldens (existing blocker goldens regress unchanged) are inherited from ticket 002's acceptance criteria. This ticket adds only the *new* cross-goal goldens. Required consequence of the split, not a separate bug.
6. Scenario isolation: each scenario in `golden_cross_goal_blocker_scoping.rs` intentionally excludes lawful competing affordances that would mask the contract under test. Specifically:
   - Scenario A removes alternative routes between the agent's location and the destination (only one route through the blocked segment exists), so the suppression test is unambiguous.
   - Scenario B configures only one counterparty at the agent's location, so suppression isolation is clean.
   - Scenarios D and E gate the safe-witnessing observation behind a controlled travel/trade trigger so the clearing predicate fires deterministically.
   Per `docs/precision-rules.md` Rule 8.

## Architecture Check

1. **Per-scenario isolation**: Each of the 8 scenarios is a separate test function with its own scenario fixture, so failures are diagnosable in isolation. No shared mutable state between scenarios.
2. **Trace-based assertions, not observer-output**: Each scenario asserts against decision-trace events (`SuppressionReason`-equivalent), event-log entries (`BlockerRecordedPayload.scope`), or live `BlockerMemory` state — not against observer-rendered text. This is per `docs/precision-rules.md` Rule 6 (decision-trace preference) and decouples this ticket from ticket 004's observer rendering.
3. **No new harness substrate**: Each scenario composes existing fixtures from `golden_harness/` (scenario loaders, agent builders, trace inspectors). If a scenario reveals a missing harness primitive (e.g., "verify Blocker.source_event points to an event in the log"), the helper is added to `golden_harness/` as a small new file alongside the scenario file — kept narrowly scoped to this ticket's needs.

## Verification Layers

1. Multi-goal suppression behavior — decision-trace assertions (Scenarios A, B): both suppressed candidates appear in the trace with the blocker-matched suppression reason.
2. TTL-based clearing — focused authoritative-state assertion (Scenario C): `BlockerMemory.intents.contains_key(&scope)` transitions from `true` to `false` at the TTL boundary.
3. Observation-based clearing — focused authoritative-state assertion + decision-trace check (Scenarios D, E): after the clearing observation tick, the blocker is removed AND the previously-suppressed candidate re-emerges.
4. Cross-store symmetry — event-log delta + authoritative-state (Scenario F): `DiscrepancyMemory.entries.contains_key(&scope)` for the same scope suppresses multiple goals.
5. `source_event` provenance — event-log delta (Scenario G): `Blocker.source_event` equals an `EventId` present in the agent's event log at the recording tick.
6. Determinism — byte-identical assertion (Scenario H): two runs of the same scenario seed produce identical `BlockerMemory.intents` serialization.

## What to Change

### 1. Create `golden_cross_goal_blocker_scoping.rs`

New file at `crates/worldwake-ai/tests/golden_cross_goal_blocker_scoping.rs` containing 8 test functions, one per scenario (A-H above).

Each test function follows the existing golden pattern:
- Load or build a scenario fixture (places, agents, routes/counterparties, blocker-memory seed)
- Run the simulation for a fixed tick budget through the live planner
- Capture trace + event log + `BlockerMemory` / `DiscrepancyMemory` state at the assertion points
- Assert against the scenario-specific invariants

### 2. Add minimal harness primitives if missing

If `golden_harness/` does not currently expose helpers for:
- `assert_blocker_present_with_scope(agent, scope)` — checks live `BlockerMemory.intents`
- `assert_blocker_source_event_resolves(agent, scope, event_log)` — checks `source_event` points to a real event
add them as small additions to existing `golden_harness/` modules (placement confirmed during implementation).

## Files to Touch

- `crates/worldwake-ai/tests/golden_cross_goal_blocker_scoping.rs` (new)
- Likely: `crates/worldwake-ai/tests/golden_harness/blocker_assertions.rs` (new, if no existing module accommodates the new helpers) — path to be confirmed during implementation; `grep -rn "blocker" crates/worldwake-ai/tests/golden_harness/` to identify the natural module first

## Out of Scope

- **Substrate migration** — landed in ticket 002.
- **`BlockerClearingCondition` variants** — landed in ticket 003.
- **Per-scope TTL fields** — landed in ticket 001.
- **Observer rendering** — ticket 004; the goldens here do not depend on observer output.
- **S144 diagnostics aggregation** — ticket 005; the goldens here do not assert against `BeliefMetrics.blocker_counts_by_scope`.
- **Existing blocker goldens** (`golden_portfolio_planning`, `golden_plan_repair`, `golden_contention_inspectability`, `golden_need_projection`) — already migrated in ticket 002 to use the scope-keyed shape (BlockerScope::Exact preservation). This ticket does not modify them.

## Acceptance Criteria

### Tests That Must Pass

1. All 8 scenarios in `golden_cross_goal_blocker_scoping.rs` pass on byte-stable seeds.
2. Existing blocker goldens (`golden_portfolio_planning`, `golden_plan_repair`, `golden_contention_inspectability`, `golden_need_projection`) continue to pass unchanged (regression guard against ticket 002's migration).
3. Determinism guard: Scenario H asserts identical `BlockerMemory.intents` serialization across two runs of the same seed.
4. Workspace: `./scripts/verify.sh` clean.

### Invariants

1. RouteSegment blockers suppress every candidate whose route traverses the segment, regardless of `GoalKind`.
2. Counterparty blockers suppress every candidate whose target equals the counterparty, regardless of `GoalKind`.
3. TTL expiry restores candidate emission at the exact tick `expires_tick` (no off-by-one).
4. `RouteRetraversedSafely` and `CounterpartyAccepted` clearing predicates remove the matching blocker on the safe-witnessing observation tick.
5. `DiscrepancyMemory` scope-keyed suppression works parallel to `BlockerMemory` (cross-store symmetry).
6. Every recorded `Blocker.source_event` is non-default and points to an `EventId` present in the agent's event log.
7. Same scenario seed → byte-identical `BlockerMemory.intents` (determinism preserved).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_cross_goal_blocker_scoping.rs` (new) — 8 scenario functions exercising the full cross-goal blocker behavior.
2. Likely: `crates/worldwake-ai/tests/golden_harness/blocker_assertions.rs` (new, if needed) — helper module for blocker-state and source_event assertions.

### Commands

1. `cargo test -p worldwake-ai --test golden_cross_goal_blocker_scoping`
2. `cargo test -p worldwake-ai --test golden_portfolio_planning --test golden_plan_repair --test golden_contention_inspectability --test golden_need_projection` — regression guard for existing blocker goldens.
3. `./scripts/verify.sh` for the full pre-PR gate.
