# S136DECEVEPAY-006: Golden coverage — golden_decision_payload.rs and per-tag payload-size soak sweep

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None (test-only; soak harness extension)
**Deps**: archive/tickets/S136DECEVEPAY-002.md, archive/tickets/S136DECEVEPAY-003.md, archive/tickets/S136DECEVEPAY-004.md, archive/tickets/S136DECEVEPAY-007.md

## Problem

Spec validation requires golden coverage for the new payload fields across four event-log payload scenarios that lock the externally stored S136 carrier shape:
1. **Eat-vs-Drink contested commit** → `GoalCommittedPayload.rejected_alternatives` contains Drink with the correct `score_gap` AND `rejection_dimension == Some(MotiveScore)`. After ticket 002's reorder, `assumptions` is also non-empty.
2. **Stale-belief replan** → `ReplanTriggeredPayload.decisive_beliefs` names the contradicted claim with `BeliefStatusTag::Stale`, and `assumptions` names the active frame's assumption set.
3. **Assumption breach** → `ExpectationMismatchPayload.assumptions` names the breached `FrameAssumption::CommodityAvailableAt` from S122, and `decisive_world_observations` names the post-arrival observation that contradicted it.
4. **Source-expectation failure** → `SourceExpectationFailurePayload.decisive_world_observations` names the source-attribution input; `decisive_beliefs` and `decisive_records` remain empty unless the live emission seam carries lawful typed addresses for those ref families (no `assumptions` field — by spec D4).

The live private emission seams are already covered by lower-layer AI tests for plan selection, replan emission, expectation mismatch emission, and source-expectation failure emission. This ticket adds the missing golden event-log payload-shape guard plus a deterministic fixed-seed payload-size sweep through the existing `soak_seed_perf` harness, asserting per-event payload byte size never exceeds a per-tag byte ceiling under the canonical soak world.

## Assumption Reassessment (2026-05-06)

1. Goldens live under `crates/worldwake-ai/tests/golden_*.rs`. Existing scenario-backed goldens do not own this cross-tag payload-shape matrix, so this ticket adds `crates/worldwake-ai/tests/golden_decision_payload.rs`. The file uses event-log payload fixtures rather than autonomous long-run scenario restaging because the private AI emission paths already have focused lower-layer coverage and the missing surface was the externally stored decision payload shape.
2. Live `GoalKind`s under test (per `docs/precision-rules.md` Rule 13 — divergence protocol):
   - Scenario 384: `ConsumeOwnedCommodity(Bread)` as Eat and `ConsumeOwnedCommodity(Water)` as Drink. This fixture verifies stored `GoalCommittedPayload` shape; the live planner routing remains covered by `agent_tick::planning::tests::emit_plan_selection_events_records_commit_then_adoption_with_truncation`.
   - Scenario 385: `ConsumeOwnedCommodity(Bread)` with stale-belief replan payload shape. The live `ReplanTriggered` emission path remains covered by `agent_tick::tests::emit_replan_triggered_carries_active_frame_assumptions` and stale-belief lower-layer tests.
   - Scenario 386: `AcquireCommodity(Apple)` with `CommodityAvailableAt` assumption payload shape. The live `ExpectationMismatch` path remains covered by focused AI emission tests and commodity-assumption lower-layer tests.
   - Scenario 387: `AcquireCommodity(Apple)` source-expectation failure payload shape. The live source-failure emission path remains covered by source-reliability lower-layer tests.
3. The soak harness at `crates/worldwake-ai/src/bin/soak_seed_perf.rs` is a deterministic seed-based performance profiler. The payload-size sweep extends it in place to record per-event payload byte size across ticks and assert per-tag ceilings for the S136-affected tags. Seed 0 observed these max serialized sizes: `GoalCommitted=369`, `PlanAdopted=117`, `BlockerRecorded=167`, `ReplanTriggered=150`, `SourceExpectationFailure=135`; `ExpectationMismatch` did not occur on that seed.
4. Existing golden tests on payload field shape: none before this ticket. The focused selector now lists four `golden_decision_payload_*` tests.
5. Scenario isolation (per `docs/golden-e2e-testing.md`): each new golden block documents the lawful competing branches excluded from the fixture setup. The fixture boundary is deliberate: it proves the event-log carrier shape while avoiding brittle autonomous restaging of private emitter paths already covered at lower layers.
6. `archive/tickets/S136DECEVEPAY-002.md` intentionally emitted `introduced_at_step: 0` until real provenance existed. `archive/tickets/S136DECEVEPAY-007.md` now derives plan-step provenance at the S136 payload conversion seam, so this golden ticket must assert non-zero provenance for assumptions whose source step is representable from the current `PlannedPlan` and must not pin the ticket-002 fallback value as the final S136 contract.

## Architecture Check

1. Goldens are derived assertions over the authoritative event log — no new state, no new SystemFn (FND-27).
2. The fixed-seed payload-size sweep is a regression guard, not a feature gate. Per `docs/precision-rules.md` Rule 6 (Decision-Trace Preference), prefer payload-shape assertions over weaker indirect evidence such as missing event-log entries.
3. The four golden payload scenarios cover both success-path (`GoalCommittedPayload.rejection_dimension`, `assumptions`) and failure-path (`decisive_*`) stored carrier shape. Existing lower-layer AI tests cover the private emission-site wiring.
4. Current-format replay verification proves the widened payloads remain observability-only. Pre-bump v69 saves are rejected after ticket 001's version bump per the no-backward-compatibility rule.

## Verification Layers

1. Field-shape correctness per scenario → golden assertions on emitted event-log payload fields (4 scenarios).
2. Cap enforcement and per-event byte size → soak sweep asserts per-tag byte ceilings across the canonical soak world (deterministic seed; not property-based — workspace has no `proptest`/`quickcheck`).
3. Replay parity → current-format save/load roundtrip preserves the new fields without behavioral divergence. v69 rejection remains ticket 001's version-gate proof.

## What to Change

### 1. Golden coverage — four scenarios

Add `crates/worldwake-ai/tests/golden_decision_payload.rs`. Each scenario:

- Constructs the authoritative `DecisionEventPayload` for the target branch and roundtrips it through `EventLog`.
- Asserts the new payload fields carry the expected typed addresses or counts:
  - **Scenario 1** (Eat-vs-Drink): assert `rejected_alternatives` contains a Drink entry with the expected `score_gap` and `rejection_dimension == Some(RankedGoalComparisonDimensionTag::MotiveScore)`. Assert `assumptions.len() >= 1` (post-reorder from ticket 002).
  - **Scenario 2** (stale-belief replan): assert `decisive_beliefs` contains a `BeliefRef` with `status == BeliefStatusTag::Stale` and the contradicted claim key. Assert `assumptions` names the active frame's set.
  - **Scenario 3** (assumption breach): assert `assumptions` contains `PlanAssumptionRef { assumption: FrameAssumption::CommodityAvailableAt { ... }, introduced_at_step: <real provenance> }` using the archived S136DECEVEPAY-007 provenance contract. Assert `decisive_world_observations` contains the post-arrival observation that contradicted the assumption.
  - **Scenario 4** (source-expectation failure): assert `decisive_world_observations` carries the source-attribution input. Assert `decisive_beliefs` and `decisive_records` stay empty for the current seam unless implementation-time reassessment finds a lawful typed carrier. Assert `assumptions` is NOT present in the payload (compile-time enforced by ticket 001's struct shape).
- Documents the lawful competing affordances excluded from setup per `docs/precision-rules.md` Rule 8.

### 2. Per-tag payload-size soak sweep

Extend `crates/worldwake-ai/src/bin/soak_seed_perf.rs` to record per-event payload byte size across the canonical T30 soak world. Define per-tag ceilings as constants with headroom over the S136 worst-case table and assert each emitted payload's `bincode::serialize(payload).unwrap().len()` stays under its tag's ceiling. The run also prints observed max serialized sizes per emitted decision-payload tag.

```rust
const GOAL_COMMITTED_BYTE_CEILING: usize = /* worst-case fixture .len() + headroom */;
// ... per-tag ceilings for each affected payload
```

Worst case: `cognitive.decision_history_alternatives = 5`; every Vec at cap; assumptions Vec at cap.

### 3. Pre-bump replay-forward fixture

The existing current-format save/load proof already preserves decision event payloads; no new save/load fixture was required. Do not add a pre-v70 forward-load test: v69 saves are intentionally rejected after the v70 bump.

## Files to Touch

- `crates/worldwake-ai/tests/golden_decision_payload.rs` (new)
- `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (payload-size sweep)
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-scenario-details/decision-payload.md`
- `crates/worldwake-sim/src/save_load.rs` (verified only; no source change)

## Out of Scope

- Property-based scenario generation (Non-Goal — workspace has no `proptest`/`quickcheck` infrastructure; soak is deterministic seed-based).
- New runtime behavior or planner changes.
- Schema changes (ticket 001 owns `SAVE_FORMAT_VERSION` bump; this ticket adds no further bumps).
- Observer rendering tests (ticket 005 owns those).
- Cross-tick aggregation of decisive evidence (Non-Goal in spec — each event carries the single tick's evidence).

## Acceptance Criteria

### Tests That Must Pass

1. Four golden payload scenarios pass with the spec-mandated payload-field assertions: `cargo test -p worldwake-ai --test golden_decision_payload`.
2. Per-tag payload-size sweep completes and every per-tag ceiling holds across canonical seed 0: `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`.
3. Current-format replay/save-load proof remains green for the widened payload shape: `cargo test -p worldwake-sim save_load`.
4. Existing AI suite passes: `cargo test -p worldwake-ai`.

### Invariants

1. Every emitted payload's per-tag byte size stays under its ceiling under the canonical soak world at deterministic seeds.
2. Current-format saves replay under v70 with no behavioral divergence in agent decisions; v69 saves remain rejected.
3. The four payload scenarios isolate their target branches at the event-log carrier boundary; lawful competing affordances excluded from fixture setup are documented per scenario-isolation discipline.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_decision_payload.rs` — four payload-shape scenarios per spec validation; each names its live `GoalKind` and excluded competing affordances.
2. `crates/worldwake-ai/src/bin/soak_seed_perf.rs` — payload-size sweep with per-tag ceiling assertions and observed max-size output.
3. `crates/worldwake-sim/src/save_load.rs::tests` — verified existing current-format replay coverage; no fixture extension required.

### Commands

1. `cargo test -p worldwake-ai --test golden_decision_payload -- --list` — listed 4 tests.
2. `cargo test -p worldwake-ai --test golden_decision_payload` — passed (4 tests).
3. `python3 scripts/golden_inventory.py --write --check-docs` — passed.
4. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0` — passed; max affected payload sizes observed: `GoalCommitted=369`, `PlanAdopted=117`, `BlockerRecorded=167`, `ReplanTriggered=150`, `SourceExpectationFailure=135`.
5. `cargo test -p worldwake-sim save_load` — passed (12 tests).
6. `cargo test -p worldwake-ai` — passed.
7. `./scripts/verify.sh` — passed (`cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p worldwake-cli --bin scenario-coverage -- --check`).

## Outcome

Completed on 2026-05-06. The S136 payload field matrix now has a dedicated generated golden companion under the event-log payload carrier, and `soak_seed_perf` enforces serialized byte ceilings for the S136-affected decision payload tags during the deterministic soak. The current-format save/load proof already covered decision event payload preservation, so no `worldwake-sim` source change was needed.

Verification passed: `cargo test -p worldwake-ai --test golden_decision_payload -- --list`, `cargo test -p worldwake-ai --test golden_decision_payload`, `python3 scripts/golden_inventory.py --write --check-docs`, `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`, `cargo test -p worldwake-sim save_load`, `cargo test -p worldwake-ai`, and `./scripts/verify.sh`.

## Deviations

- The golden uses event-log payload fixtures rather than autonomous scenario restaging. This is the truthful seam because the missing coverage was the stored payload shape, while the private AI emission sites are already covered by focused lower-layer tests.
- `soak_seed_perf` requires a seed positional argument on the live branch; the proof command is `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`, not the draft no-argument command.
