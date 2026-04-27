# S127QUAAWAACQ-009: Surface `AcquisitionQuantity` through the decision-trace pipeline

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: `worldwake-ai` (trace carriers + emitter), `worldwake-core` (optional new carrier)
**Deps**: S127QUAAWAACQ-008

## Problem

Spec S127 D11 promises that the existing `AcquireCommodity` decision-trace lines "add `desired_min`, `desired_target`, `horizon_ticks`". The implementation tickets (S127QUAAWAACQ-001..007) added the `quantity: AcquisitionQuantity` field to `GoalKind::AcquireCommodity` and wrote `derive_acquire_commodity_quantity` to compute the per-agent target from need projection + carry capacity. However, both points where the goal becomes observable to the trace layer — `emit_candidate_with_trace` in `crates/worldwake-ai/src/candidate_generation.rs:4794` and `From<GoalKind> for GoalKey` in `crates/worldwake-core/src/goal.rs:200-215` — apply the `GoalKey::from(kind)` normalization that collapses `quantity` to `AcquisitionQuantity::single()`. The collapsed value is what flows into:

- `OpportunityKey.goal_key.kind` (the only goal identity surfaced through the planner pipeline).
- `RankedGoalSummary.opportunity` (decision trace's per-tick ranked goal record).
- `format_goal_key` and `format_goal_kind` (decision-trace summary strings).

As a result, `desired_target` has no observable effect today: the per-agent variation that S126 + S127 are supposed to make visible at the trace layer is silently erased. Golden coverage for this surface (S127QUAAWAACQ-008 Golden 4) was narrowed during reassessment to "the candidate emitter emits AcquireCommodity within horizon and the agent harvests successfully" because the live trace surface cannot prove anything stronger.

## Architecture Check

1. The `quantity` field is intentionally excluded from goal identity (Design Goal 9) — two acquisition goals with the same commodity + purpose share a `GoalKey` so the planner does not double-emit. This is correct.
2. What's missing is a parallel observability carrier that preserves the per-emission `AcquisitionQuantity` for the trace layer without affecting goal identity.
3. The candidate emitter already has the value at `derive_acquire_commodity_quantity`'s return point. A bounded, optional field on `RankedGoalSummary` (or on `CandidateOfferDiagnostic`) would round-trip it through to the decision trace.

## What to Change

1. Add an optional `acquisition_quantity: Option<AcquisitionQuantity>` field to `RankedGoalSummary` (or to a new `RankedGoalDetail` carrier, if `RankedGoalSummary` should stay narrow). Populate it in the ranking-trace builder when the ranked goal is `GoalKind::AcquireCommodity`.
2. Thread the original `GoalKind` (or just the `AcquisitionQuantity` value) from `emit_candidate_with_trace` through to the diagnostics record so the ranking pass can read it without re-deriving.
3. Update `format_goal_kind` / `format_goal_key` callers that print the selected goal to include the quantity tuple when available.
4. Add a focused unit test that proves an agent with a high need projection sees `desired_target > 1` in the recorded `RankedGoalSummary.acquisition_quantity` for the AcquireCommodity goal.

## Out of Scope

- Changing goal identity (`GoalKey`) to include quantity.
- Changing `is_satisfied` semantics (which already uses `desired_min`).
- S131 wait-tick projection observability (separate spec).

## Acceptance Criteria

1. After implementation, `golden_s126_long_horizon_scales_desired_target` (in `golden_quantity_aware_acquisition.rs`) can be widened to assert `desired_target > 1` in the decision trace's ranked-goal record for the AcquireCommodity goal. Update that golden in this ticket.
2. `cargo test -p worldwake-ai` passes.
3. `./scripts/verify.sh` passes.

## Test Plan

1. Focused unit test in `crates/worldwake-ai/src/candidate_generation.rs` (or `decision_trace.rs`) verifying the new field is populated with the live derived value.
2. Widen `golden_s126_long_horizon_scales_desired_target` to assert the decision-trace surface reflects the derived `desired_target`.
3. `python3 scripts/golden_inventory.py --write --check-docs` — refresh inventory if scenario metadata changes.

## References

- S127QUAAWAACQ-008 Outcome §"Follow-up Gaps Identified" item 1.
- `crates/worldwake-core/src/goal.rs:200-215` — `GoalKey::from(GoalKind)` normalization.
- `crates/worldwake-ai/src/candidate_generation.rs:4794` — emission point where quantity is currently collapsed.
- `crates/worldwake-ai/src/candidate_generation.rs:2870` — `derive_acquire_commodity_quantity`.

## Assumption Reassessment

1. All referenced symbols exist live: `RankedGoalSummary` (decision_trace.rs:494), `summarize_ranked_goal` (agent_tick/planning.rs:310), `GoalKey::from(GoalKind)` normalization (goal.rs:200-215), `emit_candidate_with_trace` and `emit_candidate` (candidate_generation.rs), `derive_acquire_commodity_quantity` (candidate_generation.rs:2870), `golden_s126_long_horizon_scales_desired_target` (golden_quantity_aware_acquisition.rs:570).
2. Carrier choice: ticket suggested adding `acquisition_quantity` to `RankedGoalSummary` *or* `CandidateOfferDiagnostic`. Live data flow showed `summarize_ranked_goal` reads only the `AgendaEntry` (which holds a `GoalOffer`), and `CandidateOfferDiagnostic` is not consulted by the summarizer. Implemented threading via a new `acquisition_quantity: Option<AcquisitionQuantity>` field on `GoalOffer` — natural carrier through `AgendaEntry` → `RankedGoalSummary`. No diagnostic-level field needed.
3. Existing trace formatter `format_goal_kind` already prints quantity via `Debug` when the un-normalized kind is passed in, but every caller routes through `format_goal_key` (which uses the normalized `GoalKey.kind`). The new `acquisition_quantity_suffix` formatter consumes the un-normalized value from `RankedGoalSummary.acquisition_quantity` and is wired into both PLAN trace summaries (decision_trace.rs:182, 1630).
4. Existing golden setup (thirst=800 ‰) collapsed horizon to current_tick → `desired_target=1`. Lowered initial thirst to 300 ‰ in the widened scenario so projection runs (700-300)/3 = 134 ticks and `desired_target = ceil(134 × 3 / 320) = 2`, satisfying the ticket's "desired_target > 1" assertion. Tick budget extended from 20 → 40 to keep the harvest-completion proof intact under reduced motive.
5. `GoalOffer` field add reaches ~213 literal-init sites across 24 files. Sweep applied; per-file `cargo check` and `clippy --workspace --all-targets -- -D warnings` clean.

## Outcome

Completed on 2026-04-27.

- Added `pub acquisition_quantity: Option<AcquisitionQuantity>` to `GoalOffer` (goal_model.rs:2347) and to `RankedGoalSummary` (decision_trace.rs:502).
- Both `emit_candidate` and `emit_candidate_with_trace` now extract the per-emission `AcquisitionQuantity` from the un-normalized `GoalKind` via the new helper `goal_kind_acquisition_quantity` (candidate_generation.rs) before `GoalKey::from(kind)` collapses the value. The field is `None` for all non-`AcquireCommodity` goals.
- `summarize_ranked_goal` (agent_tick/planning.rs:310) copies `ranked.offer.acquisition_quantity` into the produced `RankedGoalSummary`.
- New `format_acquisition_quantity_summary` formatter surfaces `desired_min`/`desired_target`/`horizon_ticks` in both PLAN trace lines whenever the selected ranked summary carries an acquisition quantity.
- Updated all 213 `GoalOffer` literal-init sites across 24 files (production code + tests + binaries + harnesses) and all 22 `RankedGoalSummary` literal-init sites to populate the new field. Default in synthetic/test fixtures is `None`.
- Added focused unit test `candidate_gen_emits_goal_offer_with_acquisition_quantity_above_one` (candidate_generation.rs) proving an agent with a long-horizon need projection sees `desired_target > 1` on the emitted `GoalOffer.acquisition_quantity`, while the `GoalKey` identity stays collapsed to `AcquisitionQuantity::single()`.
- Added focused unit test `summarize_ranked_goal_preserves_acquisition_quantity` (agent_tick/planning.rs) proving the offer-level field round-trips into `RankedGoalSummary`.
- Widened `golden_s126_long_horizon_scales_desired_target` (Scenario 354) to:
  - Lower initial thirst from 800 ‰ → 300 ‰ via new `seed_thirsty_water_seeker_with_thirst` helper, producing a non-collapsed projection horizon.
  - Locate the `AcquireCommodity{Water}` summary in `planning.candidates.ranked` and assert `acquisition_quantity.desired_target > 1` and `horizon_ticks > 1`.
  - Assert that the `GoalKey.kind` quantity stays `AcquisitionQuantity::single()` to lock in the Design Goal 9 split between identity and per-emission carrier.
  - Updated scenario header comment to describe the new proof and add Principle 29 (Debuggability Is a Product Feature).
- Refreshed generated golden inventory (`docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-scenario-details/quantity-aware-acquisition.md`, `portfolio-planning.md`) via `python3 scripts/golden_inventory.py --write --check-docs` to capture the updated scenario metadata and shifted source line numbers.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_gen_emits_goal_offer_with_acquisition_quantity_above_one`
- Passed `cargo test -p worldwake-ai --lib summarize_ranked_goal_preserves_acquisition_quantity`
- Passed `cargo test -p worldwake-ai --test golden_quantity_aware_acquisition golden_s126_long_horizon_scales_desired_target`
- Passed `cargo test -p worldwake-ai` (full crate suite, all tests)
- Passed `./scripts/verify.sh` (fmt, full workspace tests, clippy with `--all-targets -- -D warnings`, scenario-coverage `--check`)
- Passed `python3 scripts/golden_inventory.py --write --check-docs` (32 files, 146 tests, 110 scenario blocks).
