# S126NEEPROTIM-003: evaluate_assumptions arm and Discrepancy recording

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — replaces ticket 001's placeholder arm in `evaluate_assumptions` with the real projection re-evaluation logic. Adds `current_tick: Tick` to `evaluate_assumptions` and `failed_assumption: FrameAssumption` to `record_assumption_failure`. Extends `record_assumption_failure` to construct `Discrepancy::NeedHorizonExceeded` with `DiscrepancyClearing::TtlExpiry` when the failed assumption is `NeedSafeUntilTick`.
**Deps**: S126NEEPROTIM-001

## Problem

Ticket 001 introduces the `NeedSafeUntilTick` variant but routes it through a no-op placeholder arm in `evaluate_assumptions`. Ticket 002 starts producing the assumption from `populate_assumptions`. This ticket closes the loop: replace the placeholder with the real re-evaluation, and route the resulting `CriticalFailure(NeedSafeUntilTick)` into `record_assumption_failure` so the typed `Discrepancy::NeedHorizonExceeded` lands in `DiscrepancyMemory` with the structural-block-ticks TTL.

The two signature changes ripple across the AI crate: `evaluate_assumptions` gains `current_tick` (2 production callers + 10 test sites), and `record_assumption_failure` gains `failed_assumption: FrameAssumption` (1 production caller + 5 test sites — 4 in `frame.rs::tests`, 1 in `agent_tick/tests.rs:7587`).

## Assumption Reassessment (2026-04-26)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `evaluate_assumptions` (`crates/worldwake-ai/src/agent_tick/frame.rs:339-392`) currently has signature `(assumptions: &[FrameAssumption], view: &dyn RuntimeBeliefView, agent: EntityId, ranked_candidates: Option<&OrderedRanked<'_>>) -> AssumptionEvalResult`. Match arms cover `TargetAlive`, `RouteExists`, `NoCriticalThreat`, `CommodityAvailableAt`. Ticket 001 added a `NeedSafeUntilTick { .. } => {}` placeholder. This ticket replaces the placeholder with a projection re-evaluation arm. Existing unit tests in `agent_tick/frame.rs::tests`: `target_alive_dead_produces_critical_failure` (1134), `route_exists_severed_produces_recoverable_route_blocked` (1152), `no_critical_threat_with_critical_candidate_produces_survival_need` (1170), `all_assumptions_pass_returns_all_pass` (1187), `no_critical_threat_without_candidates_returns_deferred` (1208), `evaluate_commodity_available_at_returns_critical_failure_when_refuted` (1221), `evaluate_commodity_available_at_returns_all_pass_when_believed` (1249), `evaluate_commodity_available_at_returns_deferred_when_unknown` (1275), `evaluate_commodity_available_at_co_located_resource_source_returns_all_pass` (1294), `critical_failure_transitions_to_exhausted` (1327). All 10 tests pass `&assumptions, &view, agent, ranked_candidates` and need `current_tick: Tick` appended.
2. `record_assumption_failure` (`crates/worldwake-ai/src/agent_tick/frame.rs:496-527`) currently has signature `(frame, agent_place, blocker_target, discrepancy_memory, tick, structural_block_ticks)`. It constructs `Discrepancy::BeliefContradicted` (when target present) or `PartialExecutionDrift` (when target absent), with `DiscrepancyClearing::CommodityAvailabilityChanged` (when `frame.expected_commodity()` is `Some`) or `TtlExpiry` (otherwise). This ticket adds `failed_assumption: FrameAssumption` and routes `NeedSafeUntilTick` failures to `Discrepancy::NeedHorizonExceeded` with `DiscrepancyClearing::TtlExpiry`. Existing test sites: `record_assumption_failure_uses_structural_block_ticks_with_target` (1616), `record_assumption_failure_for_expected_commodity_clears_on_reavailability` (1652), `record_assumption_failure_uses_structural_block_ticks_without_target` (1700), `record_assumption_failure_overwrites_prior_entry_for_same_key` (1724), plus 1 site in `agent_tick/tests.rs:7587-7608`.
3. Spec authority: `specs/S126-need-projection-time-budget.md` D5 and D6 part 2 (the recording-side body; D6 part 1 — variant addition — landed in ticket 001).
4. Shared abstraction boundary: this ticket changes both `evaluate_assumptions` and `record_assumption_failure` signatures. The end-to-end contract: when an assumption breaches, the evaluator returns `CriticalFailure(failed_assumption)`; the caller passes `failed_assumption` into the recorder; the recorder constructs the typed discrepancy that will surface in S110's `BlockerRecorded` payload.
5. Production caller for `evaluate_assumptions`: `agent_tick/mod.rs:1028` (the main per-tick refresh) and `agent_tick/mod.rs:1213` (the deferred re-evaluation path). Production caller for `record_assumption_failure`: `agent_tick/mod.rs:1048` (inside the `CriticalFailure` branch). At `mod.rs:1048` the `assumption` variable is already bound by destructuring `AssumptionEvalResult::CriticalFailure(assumption)` at line 1039 — passing it forward is a small forwarding change.
6. Layer precision: this is an AI-layer change inside `agent_tick`; the function is `pub(super)`. The harness boundary for new behavioral tests: local needs-only harness is sufficient — `evaluate_assumptions` reads only the belief view; `record_assumption_failure` writes only to `DiscrepancyMemory`. No action registries are required.
7. Ordering contract: the discrepancy is recorded with `expires_tick: tick + u64::from(structural_block_ticks)` (preserving the existing TTL semantics from `frame.rs:524`). `DiscrepancyMemory::is_suppressed` (`discrepancy.rs:70-74`) gates re-adoption of the same goal until expiry — matching the suppression-duration contract S122 documented.
8. Adjacent contradiction: ticket 002's `populate_assumptions` may push a `NeedSafeUntilTick` even on a tick where physiology has already decreased enough for the assumption to immediately re-evaluate as `Holds`. This is a required consequence of the per-tick re-derivation pattern (FND-27 cache-not-truth) — not a separate bug.

## Architecture Check

1. The `failed_assumption` parameter on `record_assumption_failure` makes the discrepancy class derivable from the assumption variant — the recorder no longer needs to inspect frame structure to guess the failure class. The design is cleaner than dispatching on `frame.expected_commodity()` alone (the existing fallback at `frame.rs:505-509`), which would silently misclassify `NeedSafeUntilTick` failures as `BeliefContradicted`/`PartialExecutionDrift`.
2. `Discrepancy::NeedHorizonExceeded` carries `projected_breach_tick` but not `until_tick` — the `until_tick` is already on the `FrameAssumption::NeedSafeUntilTick` payload that the trace surface (added in ticket 001's D7 arm) already renders. Avoiding the duplicate field follows FND-3 (no parallel authoritative copies) and matches the spec's M3 finding.
3. No backward-compatibility shims around the old signatures.

## Verification Layers

1. `evaluate_assumptions` returns `CriticalFailure(NeedSafeUntilTick { .. })` when projection collapses below `until_tick` → focused unit test in `agent_tick/frame.rs::tests`.
2. `evaluate_assumptions` returns `AllPass` when projection still ≥ `until_tick` → focused unit test.
3. `evaluate_assumptions` returns `Deferred` when physiology profiles are missing → focused unit test.
4. `record_assumption_failure` writes `Discrepancy::NeedHorizonExceeded { need, projected_breach_tick }` when `failed_assumption` is `NeedSafeUntilTick` → focused unit test against `DiscrepancyMemory`.
5. `record_assumption_failure` writes `Discrepancy::BeliefContradicted` (preserving existing semantics) when `failed_assumption` is `CommodityAvailableAt` → focused unit test (regression guard for existing behavior).
6. `record_assumption_failure` uses `DiscrepancyClearing::TtlExpiry` for `NeedHorizonExceeded` and `CommodityAvailabilityChanged` for `CommodityAvailableAt` (existing) — verified via the entry's `clearing_condition` field → focused unit test.
7. Single-layer ticket (focused unit coverage on AI-internal helpers) — additional layer mapping (action trace, event-log delta, authoritative world state) is not applicable because `evaluate_assumptions` and `record_assumption_failure` produce values and write to `DiscrepancyMemory` (an ECS component) without committing actions or emitting events. The end-to-end "suppression survives across ticks" assertion is left to ticket 004's golden coverage.

## What to Change

### 1. Update `evaluate_assumptions` signature and replace placeholder arm

In `crates/worldwake-ai/src/agent_tick/frame.rs:339-344`, extend the signature with `current_tick: Tick`:

```rust
pub(super) fn evaluate_assumptions(
    assumptions: &[FrameAssumption],
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    ranked_candidates: Option<&OrderedRanked<'_>>,
    current_tick: Tick,
) -> AssumptionEvalResult
```

Replace ticket 001's `NeedSafeUntilTick { .. } => {}` placeholder with the real arm per spec D5:

```rust
FrameAssumption::NeedSafeUntilTick { need, until_tick } => {
    let (Some(metabolism), Some(needs), Some(thresholds)) = (
        view.metabolism_profile(agent),
        view.homeostatic_needs(agent),
        view.drive_thresholds(agent),
    ) else {
        has_deferred = true;
        continue;
    };
    let projected = needs.projected_tick_of(
        need,
        thresholds.high(need),
        metabolism.rate(need),
        current_tick,
    );
    if let Some(breach_tick) = projected
        && breach_tick < until_tick
    {
        return AssumptionEvalResult::CriticalFailure(*assumption);
    }
}
```

`*assumption` carries `NeedSafeUntilTick { need, until_tick }` forward to the trace formatter (ticket 001's D7 arm) and to `record_assumption_failure` (this ticket, §3 below).

### 2. Update `evaluate_assumptions` production callers

- `crates/worldwake-ai/src/agent_tick/mod.rs:1028` — append `tick` (the in-scope per-tick value) as the new `current_tick` argument.
- `crates/worldwake-ai/src/agent_tick/mod.rs:1213` — append the in-scope tick value as the new `current_tick` argument.

### 3. Update `record_assumption_failure` signature and body

In `crates/worldwake-ai/src/agent_tick/frame.rs:496-527`, extend the signature with a new parameter:

```rust
pub(super) fn record_assumption_failure(
    frame: &IntentionFrame,
    agent_place: Option<EntityId>,
    blocker_target: Option<EntityId>,
    discrepancy_memory: &mut DiscrepancyMemory,
    tick: Tick,
    structural_block_ticks: u32,
    failed_assumption: FrameAssumption,
)
```

Replace the body's discrepancy/clearing construction with a branch on `failed_assumption`:

```rust
let target = blocker_target.or_else(|| frame_blocker_target(&frame.domain));
let (discrepancy, clearing_condition) = match failed_assumption {
    FrameAssumption::NeedSafeUntilTick { need, until_tick } => {
        // `projected_breach_tick` is the freshly computed breach; for the
        // recorded discrepancy we use `until_tick` from the failed assumption
        // because that is the breach tick the agent was budgeting against.
        // The trace consumer reads `until_tick` from the FrameAssumption
        // payload via the CriticalFailure carrier; this field captures the
        // recorded breach for downstream queries on DiscrepancyMemory.
        (
            Discrepancy::NeedHorizonExceeded {
                need,
                projected_breach_tick: until_tick,
            },
            DiscrepancyClearing::TtlExpiry,
        )
    }
    _ => {
        let clearing = frame
            .expected_commodity()
            .map_or(DiscrepancyClearing::TtlExpiry, |(commodity, place)| {
                DiscrepancyClearing::CommodityAvailabilityChanged { commodity, place }
            });
        let discrepancy = if target.is_some() {
            Discrepancy::BeliefContradicted
        } else {
            Discrepancy::PartialExecutionDrift
        };
        (discrepancy, clearing)
    }
};
discrepancy_memory.record(DiscrepancyEntry {
    blocker_key: BlockerKey {
        goal_key: frame.goal,
        place: agent_place,
        target,
        action_def: None,
    },
    discrepancy,
    observed_tick: tick,
    expires_tick: tick + u64::from(structural_block_ticks),
    clearing_condition,
});
```

Note: the `projected_breach_tick` field on the new `Discrepancy::NeedHorizonExceeded` variant is populated from `until_tick` of the failed assumption — this is the breach tick the agent was budgeting against, which is the most useful value for downstream queries on `DiscrepancyMemory`. A future refinement could carry the freshly-computed breach tick separately if needed, but this ticket's spec text (D6) treats them as equivalent for the recording surface.

### 4. Update `record_assumption_failure` production caller

- `crates/worldwake-ai/src/agent_tick/mod.rs:1048` — append `assumption` (already destructured at line 1039 from `AssumptionEvalResult::CriticalFailure(assumption)`) as the new `failed_assumption` argument.

### 5. Update existing tests (15 sites)

- 10 `evaluate_assumptions` test calls in `agent_tick/frame.rs::tests` (lines 1138, 1157, 1174, 1195, 1211, 1230, 1262, 1281, 1312, plus the 1 in `apply_assumption_result` regression at line ~1327): append `Tick(0)` (or the test-local tick) as `current_tick`.
- 4 `record_assumption_failure` test calls in `agent_tick/frame.rs::tests` (lines 1624, 1672, 1707, 1731, 1739 — note 5 sites total; double-check during implementation): append a representative `failed_assumption` matching what the test setup implies (most existing tests use commodity-availability framing, so `FrameAssumption::CommodityAvailableAt { commodity: <test-commodity>, place: <test-place> }` is the appropriate value; for tests that don't set up commodity context, `FrameAssumption::TargetAlive(<test-entity>)` is a safe substitute).
- 1 `record_assumption_failure` test call in `agent_tick/tests.rs:7587-7608`: append the appropriate `failed_assumption` argument matching the test's intent.

### 6. Add new unit tests for need-horizon evaluation and recording

Add new focused unit tests in `agent_tick/frame.rs::tests`:
- `evaluate_need_safe_until_tick_returns_critical_failure_when_breach_before_until_tick` — set hunger=400, hunger_rate=50, hunger.high()=700, current_tick=Tick(10), assumption `NeedSafeUntilTick { need: Hunger, until_tick: Tick(20) }`. Expected: `breach_tick = 16 < 20` → `CriticalFailure(NeedSafeUntilTick { ... })`.
- `evaluate_need_safe_until_tick_returns_all_pass_when_breach_at_or_after_until_tick` — same setup but `until_tick=Tick(15)`. Expected: `breach_tick = 16 >= 15` → `AllPass`.
- `evaluate_need_safe_until_tick_returns_deferred_when_profile_missing` — mock view returns `None` for `metabolism_profile`. Expected: `Deferred`.
- `record_assumption_failure_writes_need_horizon_exceeded` — call `record_assumption_failure` with `failed_assumption = NeedSafeUntilTick { need: Hunger, until_tick: Tick(20) }`, then read the resulting `DiscrepancyMemory` entry. Expected: `discrepancy == NeedHorizonExceeded { need: Hunger, projected_breach_tick: Tick(20) }` and `clearing_condition == TtlExpiry`.
- `record_assumption_failure_preserves_commodity_availability_clearing` — regression: call with `failed_assumption = CommodityAvailableAt { ... }` and a frame whose `expected_commodity()` is `Some`. Expected: `clearing_condition == CommodityAvailabilityChanged { ... }` (existing behavior preserved).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify) — replace placeholder arm with real evaluation, update both function signatures, update 14 existing tests, add 5 new tests
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify) — update 2 `evaluate_assumptions` callers + 1 `record_assumption_failure` caller
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — update 1 `record_assumption_failure` caller at line 7587-7608

## Out of Scope

- `populate_assumptions` extension — ticket 002 (D4)
- Variant additions and helper methods — ticket 001 (D1, D2, D3, D6 part 1, D7)
- Golden test coverage for the full assumption→discrepancy→suppression chain — ticket 004 (D8)
- Adding a richer `DiscrepancyClearing::NeedLevelDecreasedBelow` arm — explicit non-goal per spec D6 ("a future spec may add a richer clearing condition")

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `evaluate_need_safe_until_tick_returns_critical_failure_when_breach_before_until_tick`.
2. New unit test: `evaluate_need_safe_until_tick_returns_all_pass_when_breach_at_or_after_until_tick`.
3. New unit test: `evaluate_need_safe_until_tick_returns_deferred_when_profile_missing`.
4. New unit test: `record_assumption_failure_writes_need_horizon_exceeded`.
5. New unit test: `record_assumption_failure_preserves_commodity_availability_clearing`.
6. Existing tests: all 10 `evaluate_assumptions` tests pass with the new `current_tick` parameter; all 5 `record_assumption_failure` tests pass with the new `failed_assumption` parameter.
7. Existing suite: `cargo test -p worldwake-ai --lib agent_tick`.
8. Existing suite: `cargo test --workspace`.
9. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `evaluate_assumptions` returns `CriticalFailure(failed_assumption)` for `NeedSafeUntilTick` only when the recomputed projection breaches before `until_tick` — re-evaluation can transition the assumption back to `AllPass` when physiology shifts.
2. `record_assumption_failure` constructs `Discrepancy::NeedHorizonExceeded` only when the failed assumption is `NeedSafeUntilTick`; all other failure variants follow the existing recording path.
3. `DiscrepancyMemory::is_suppressed` gates re-adoption of the same goal until `tick + structural_block_ticks` for need-horizon failures (same TTL semantics as commodity-availability failures).
4. The placeholder arm from ticket 001 is fully removed — no comments, no shims.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs` — 5 new tests under `mod tests`; 14 existing test sites updated to pass new parameters.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — 1 existing test site updated.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::frame::tests::evaluate`
2. `cargo test -p worldwake-ai --lib agent_tick::frame::tests::record_assumption_failure`
3. `cargo test -p worldwake-ai`
4. `cargo build --workspace`
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-26.

- Added `current_tick: Tick` parameter to `evaluate_assumptions` and replaced the placeholder `NeedSafeUntilTick { .. }` arm in `crates/worldwake-ai/src/agent_tick/frame.rs` with the real per-tick projection re-evaluation: it reads physiology via `RuntimeBeliefView`, calls `HomeostaticNeeds::projected_tick_of`, and returns `CriticalFailure(*assumption)` when the freshly-computed projected breach falls before `until_tick`. Missing-profile cases mark `has_deferred = true` and continue.
- Added `failed_assumption: FrameAssumption` parameter to `record_assumption_failure` and rewrote the discrepancy-construction body to dispatch on the failed-assumption variant. `NeedSafeUntilTick` failures now construct `Discrepancy::NeedHorizonExceeded { need, projected_breach_tick: until_tick }` with `DiscrepancyClearing::TtlExpiry`. All other variants preserve the existing `BeliefContradicted`/`PartialExecutionDrift` + `CommodityAvailabilityChanged`/`TtlExpiry` paths.
- Updated 2 production callers of `evaluate_assumptions` (`agent_tick/mod.rs:1029, 1214`) to pass `tick`, and the 1 production caller of `record_assumption_failure` (`agent_tick/mod.rs:1049`) to forward the destructured `assumption`.
- Updated 9 existing `evaluate_assumptions` tests in `frame.rs::tests` and 4 existing `record_assumption_failure` tests in the same module, plus the 1 site in `agent_tick/tests.rs:7608`, with the new parameters.
- Added 5 new focused unit tests:
  - `evaluate_need_safe_until_tick_returns_critical_failure_when_breach_before_until_tick`
  - `evaluate_need_safe_until_tick_returns_all_pass_when_breach_at_or_after_until_tick`
  - `evaluate_need_safe_until_tick_returns_deferred_when_profile_missing`
  - `record_assumption_failure_writes_need_horizon_exceeded`
  - `record_assumption_failure_preserves_commodity_availability_clearing`

## Deviations

- **Test math recalibrated to live `DriveThresholds::default().hunger.high() = 750`** (ticket text §6 referenced `700`). Auto-corrected during reassessment: tests use `current_tick=10`, `hunger=400`, `hunger_rate=50`, producing breach at `Tick(17)` (`= 10 + ⌈(750-400)/50⌉`), and the ticket math was rewritten to match the live default thresholds.

- **Absorbed a `populate_assumptions` correctness fix originally in ticket 002's surface.** With ticket 003's evaluation live, two existing goldens (`golden_local_depleted_source_regenerates_without_spurious_failure_memory`, `golden_goal_switching_during_multi_leg_travel`) failed because agents starting with already-breached needs (e.g. `hunger=900` above `high=750`) caused `projected_tick_of` to return `Some(current_tick)`, which trivially satisfied `breach_tick < plan_completion_tick` for any non-trivial plan and pushed a pre-falsified `NeedSafeUntilTick`. Fix: in `populate_assumptions` (frame.rs), only push the assumption when `breach_tick > current_tick && breach_tick < plan_completion_tick`. The assumption "I will stay safe until X" is meaningful only when the agent is currently safe; once already past the high band, the assumption is incoherent and the agent's reactive ranking handles the urgency. Recorded in code via an explanatory comment at the populate site.

- **Rewrote `golden_goal_switching_during_multi_leg_travel` to assert the post-S126 lawful contract.** The original test exercised pre-S126 reactive thirst interruption (agent travels through medium/high thirst and detours at critical). Under S126's horizon-aware planning, the projection assumption fires before the multi-leg journey can complete and the food-acquisition goal is suppressed via `Discrepancy::NeedHorizonExceeded`. The rewritten test asserts: (1) the agent commits to the orchard journey at least once, (2) a `NeedHorizonExceeded` discrepancy for `Thirst` is recorded, (3) that discrepancy uses `DiscrepancyClearing::TtlExpiry`. End-to-end "shorter alternative wins next ranking round" coverage is owned by ticket 004.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agent_tick::frame::tests::evaluate` (7 tests, includes 3 new need-horizon tests)
- Passed `cargo test -p worldwake-ai --lib agent_tick::frame::tests::record_assumption_failure` (6 tests, includes 2 new need-horizon recording tests)
- Passed `cargo test -p worldwake-ai` (full crate: 1484 unit tests + 37 golden tests + sub-crate suites all green)
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo fmt --all -- --check`
- Passed `./scripts/verify.sh` (full pre-PR gate)
