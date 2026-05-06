# S136DECEVEPAY-006: Golden coverage — golden_decision_payload.rs and per-tag payload-size soak sweep

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None (test-only; soak harness extension)
**Deps**: archive/tickets/S136DECEVEPAY-002.md, tickets/S136DECEVEPAY-003.md, tickets/S136DECEVEPAY-004.md, archive/tickets/S136DECEVEPAY-007.md

## Problem

Spec validation requires golden coverage for the new payload fields across four scenarios that exercise the full S136 wire-up:
1. **Eat-vs-Drink contested commit** → `GoalCommittedPayload.rejected_alternatives` contains Drink with the correct `score_gap` AND `rejection_dimension == Some(MotiveScore)`. After ticket 002's reorder, `assumptions` is also non-empty.
2. **Stale-belief replan** → `ReplanTriggeredPayload.decisive_beliefs` names the contradicted claim with `BeliefStatusTag::Stale`, and `assumptions` names the active frame's assumption set.
3. **Assumption breach** → `ExpectationMismatchPayload.assumptions` names the breached `FrameAssumption::CommodityAvailableAt` from S122, and `decisive_world_observations` names the post-arrival observation that contradicted it.
4. **Source-expectation failure** → `SourceExpectationFailurePayload.decisive_*` names the source-attribution input (no `assumptions` field — by spec D4).

Plus a deterministic fixed-seed payload-size sweep through the existing `soak_seed_perf` harness, asserting per-event payload byte size never exceeds a per-tag byte ceiling under the canonical scenarios.

## Assumption Reassessment (2026-05-06)

1. Goldens live under `crates/worldwake-ai/tests/golden_*.rs`. The spec proposes a new file `golden_decision_payload.rs`. **Confirm at implementation time** whether existing scenarios (e.g., `golden_survival_*.rs`, `golden_replan_*.rs`, `golden_perception_*.rs`) cover the four cases — if so, prefer extending those scenarios with new payload-shape assertions over creating a parallel golden. The new file is the fallback when no existing scenario fits.
2. Live `GoalKind`s under test (per `docs/precision-rules.md` Rule 13 — divergence protocol):
   - Scenario 1: `Eat` (committed), `Drink` (rejected). Verify both surfaces still route through the live planner.
   - Scenario 2: `Eat` with stale-belief replan. Verify the live `ReplanTriggered` emission path.
   - Scenario 3: `Eat` with `CommodityAvailableAt` assumption (S122 frame assumption). Verify the live `ExpectationMismatch` path with this assumption variant.
   - Scenario 4: A source-expectation failure scenario — check at implementation time which existing goldens exercise `SourceExpectationFailure` and whether their setup can be reused.
3. The soak harness at `crates/worldwake-ai/src/bin/soak_seed_perf.rs` is a deterministic seed-based performance profiler. The payload-size sweep extends it (or adds a parallel binary) to record per-event payload byte size across ticks and assert per-tag ceilings. Per-tag ceilings derive from the worst-case table in the spec's Risks section: with `cognitive.decision_history_alternatives = 5`, `BlockerRecordedPayload` / `ReplanTriggeredPayload` / `ExpectationMismatchPayload` are the largest (gain 4 Vecs of cap 5). Compute exact ceilings at implementation time using `bincode::serialize(payload).unwrap().len()` on representative worst-case fixtures and bake the values as constants in the sweep.
4. Existing golden tests on payload field shape: none today — the field set didn't exist before ticket 001. This ticket establishes the contract. Verify test names against `cargo test -p worldwake-ai -- --list | grep golden_` before committing assertion paths.
5. Scenario isolation (per `docs/precision-rules.md` Rule 8): each of the four scenarios isolates a single causal branch. Document the lawful competing affordances each scenario excludes from setup. For example, scenario 2 (stale-belief replan) excludes alternative goals that would lead to a non-replan outcome (those would obscure the `ReplanTriggered` emission). Document each exclusion explicitly in the golden's setup comments.
6. `archive/tickets/S136DECEVEPAY-002.md` intentionally emitted `introduced_at_step: 0` until real provenance existed. `archive/tickets/S136DECEVEPAY-007.md` now derives plan-step provenance at the S136 payload conversion seam, so this golden ticket must assert non-zero provenance for assumptions whose source step is representable from the current `PlannedPlan` and must not pin the ticket-002 fallback value as the final S136 contract.

## Architecture Check

1. Goldens are derived assertions over the authoritative event log — no new state, no new SystemFn (FND-27).
2. The fixed-seed payload-size sweep is a regression guard, not a feature gate. Per `docs/precision-rules.md` Rule 6 (Decision-Trace Preference), prefer payload-shape assertions over weaker indirect evidence such as missing event-log entries.
3. The four scenarios cover both success-path (`GoalCommittedPayload.rejection_dimension`, `assumptions`) and failure-path (`decisive_*`) wiring — exercises the full S136 emission surface.
4. Current-format replay verification proves the widened payloads remain observability-only. Pre-bump v69 saves are rejected after ticket 001's version bump per the no-backward-compatibility rule.

## Verification Layers

1. Field-shape correctness per scenario → golden assertions on emitted payload fields (4 scenarios).
2. Cap enforcement and per-event byte size → soak sweep asserts per-tag byte ceilings across canonical scenarios (deterministic seed; not property-based — workspace has no `proptest`/`quickcheck`).
3. Replay parity → current-format save/load roundtrip preserves the new fields without behavioral divergence. v69 rejection remains ticket 001's version-gate proof.

## What to Change

### 1. Golden coverage — four scenarios

Add `crates/worldwake-ai/tests/golden_decision_payload.rs` (or extend existing siblings if reassessment reveals overlap with current goldens). Each scenario:

- Constructs a deterministic seed + scenario configuration that isolates its target branch.
- Runs the simulation for enough ticks to produce the target event.
- Asserts the new payload fields carry the expected typed addresses or counts:
  - **Scenario 1** (Eat-vs-Drink): assert `rejected_alternatives` contains a Drink entry with the expected `score_gap` and `rejection_dimension == Some(RankedGoalComparisonDimensionTag::MotiveScore)`. Assert `assumptions.len() >= 1` (post-reorder from ticket 002).
  - **Scenario 2** (stale-belief replan): assert `decisive_beliefs` contains a `BeliefRef` with `status == BeliefStatusTag::Stale` and the contradicted claim key. Assert `assumptions` names the active frame's set.
  - **Scenario 3** (assumption breach): assert `assumptions` contains `PlanAssumptionRef { assumption: FrameAssumption::CommodityAvailableAt { ... }, introduced_at_step: <real provenance> }` using the archived S136DECEVEPAY-007 provenance contract. Assert `decisive_world_observations` contains the post-arrival observation that contradicted the assumption.
  - **Scenario 4** (source-expectation failure): assert `decisive_*` carries the source-attribution input. Assert `assumptions` is NOT present in the payload (compile-time enforced by ticket 001's struct shape).
- Documents the lawful competing affordances excluded from setup per `docs/precision-rules.md` Rule 8.

### 2. Per-tag payload-size soak sweep

Extend `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (or add a sibling binary `soak_payload_size`) to record per-event payload byte size across the canonical scenarios. Define per-tag ceilings as constants derived at implementation time from a worst-case fixture serialization. Suggested ceiling discipline:

```rust
const GOAL_COMMITTED_BYTE_CEILING: usize = /* worst-case fixture .len() + headroom */;
// ... per-tag ceilings for each affected payload
```

Worst case: `cognitive.decision_history_alternatives = 5`; every Vec at cap; assumptions Vec at cap. Encode the ceilings as constants and assert each emitted payload's `bincode::serialize(payload).unwrap().len()` stays under its tag's ceiling.

### 3. Pre-bump replay-forward fixture

Extend the current-format save/load proof added by ticket 001 if the golden payload field coverage needs a broader replay fixture. Do not add a pre-v70 forward-load test: v69 saves are intentionally rejected after the v70 bump.

## Files to Touch

- Likely: `crates/worldwake-ai/tests/golden_decision_payload.rs` (new — confirm existing scenarios don't cover the four cases first; otherwise extend them and skip the new file)
- `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (modify — payload-size sweep) OR new sibling `crates/worldwake-ai/src/bin/soak_payload_size.rs` (depending on whether mixing concerns inside the existing soak harness is acceptable to reviewers)
- `crates/worldwake-sim/src/save_load.rs` (verify only unless current-format replay coverage needs a focused fixture extension)

## Out of Scope

- Property-based scenario generation (Non-Goal — workspace has no `proptest`/`quickcheck` infrastructure; soak is deterministic seed-based).
- New runtime behavior or planner changes.
- Schema changes (ticket 001 owns `SAVE_FORMAT_VERSION` bump; this ticket adds no further bumps).
- Observer rendering tests (ticket 005 owns those).
- Cross-tick aggregation of decisive evidence (Non-Goal in spec — each event carries the single tick's evidence).

## Acceptance Criteria

### Tests That Must Pass

1. Four golden scenarios pass with the spec-mandated payload-field assertions: `cargo test -p worldwake-ai golden_decision_payload` (or, if extending existing files, the named sibling tests).
2. Per-tag payload-size sweep completes and every per-tag ceiling holds across the canonical scenario seeds: `cargo run --release -p worldwake-ai --bin soak_seed_perf` (or the sibling binary added in this ticket).
3. Current-format replay/save-load proof remains green for the widened payload shape: `cargo test -p worldwake-sim save_load`.
4. Existing AI suite passes: `cargo test -p worldwake-ai`.

### Invariants

1. Every emitted payload's per-tag byte size stays under its ceiling under canonical scenarios at deterministic seeds.
2. Current-format saves replay under v70 with no behavioral divergence in agent decisions; v69 saves remain rejected.
3. The four scenarios isolate their target branches; lawful competing affordances are documented per scenario-isolation discipline.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_decision_payload.rs` (new or sibling extensions) — four scenarios per spec validation; each names its live `GoalKind` and excluded competing affordances.
2. `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (or `soak_payload_size.rs`) — payload-size sweep with per-tag ceiling assertions.
3. `crates/worldwake-sim/src/save_load.rs::tests` — only extend current-format replay coverage if the new golden payload assertions require an additional save/load fixture.

### Commands

1. `cargo test -p worldwake-ai golden_decision_payload`
2. `cargo test -p worldwake-ai` (full AI suite)
3. `cargo run --release -p worldwake-ai --bin soak_seed_perf`
4. `cargo test -p worldwake-sim save_load`
5. `./scripts/verify.sh`
