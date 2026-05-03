# S131SOURELWAI-004: Composite source-reliability ranking and trace surface

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` (`apply_source_reliability_discount` restructured to single composite path; `SourceReliabilityDiscount` trace struct extended; existing failure-only tests adjust to new composite semantics)
**Deps**: archive/tickets/S131SOURELWAI-001.md

## Problem

Today the AI ranks `AcquireCommodity` candidates by a single-axis source-trust discount (`apply_source_reliability_discount` at `ranking.rs:419` and its pending-failure variant at `:532`). Both functions return `None` when `failure_ratio == 0`, so an agent with no failure history at a source gets no source-reliability adjustment — even when it has waited 12 ticks for that source three times running, or when it last observed the source at capacity 0. This ticket extends the discount path into a single composite computation (trust − wait_penalty + capacity_signal) that fires on every per-candidate evaluation, drops the failure-only early-out, extends the `SourceReliabilityDiscount` decision-trace struct with the new components, and updates the four existing ranking tests whose assertions presuppose the early-out.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `fn apply_source_reliability_discount(candidate, context, motive_score) -> Option<SourceReliabilityDiscount>` lives at `crates/worldwake-ai/src/ranking.rs:419–452`. Its sibling `fn apply_source_reliability_discount_with_pending_failures(candidate, context, motive_score, pending_failures) -> Option<SourceReliabilityDiscount>` lives at `:532–579`. Both call `failure_ratio_permille(record)` and short-circuit `return None` when the result is zero (lines 436–438 and 564–566 respectively). `pub struct SourceReliabilityDiscount` lives at `crates/worldwake-ai/src/decision_trace.rs:546–552` with fields `source_entity, commodity, failure_ratio_permille, pre_discount_motive, post_discount_motive`. Display formatting (referenced by spec D6 as "lines 1952–1961") emits `source_reliability=entity=_ commodity=_ failure=_ pre=_ post=_`. Construction sites for `SourceReliabilityDiscount`: 4 files in `worldwake-ai` per Step 2 grep — the ranking module itself plus `goal_model.rs:2839`, `agent_tick/planning.rs:4139`, and decision-trace test fixtures. Existing ranking tests directly affected by the early-out removal: `source_reliability_discount_skips_non_commodity_goals:5563`, `source_reliability_discount_returns_none_without_experience:5582`, `source_reliability_discount_returns_none_without_preference_profile:5625`, `source_reliability_discount_applies_failure_ratio_proportionally:5670`.
2. The phase under change is **ranking** per `docs/precision-rules.md` Rule 1. Candidate generation, plan search, and authoritative outcome are unaffected — the discount adjusts `motive_score` for already-emitted candidates but does not gate emission. No `Authoritative-to-AI Impact Rule` checklist applies because the change does not touch action preconditions, validation functions, affordance generation, candidate emission, or goal satisfaction.
3. Cross-system boundary under audit: `apply_source_reliability_discount` consumes `SourceReliability` and `PreferenceProfile` via `GoalBeliefView::source_reliability` and `::preference_profile` (`crates/worldwake-sim/src/belief_view.rs`). The new fields added in ticket 001 flow through these accessors automatically — the returned `SourceReliability` and `PreferenceProfile` carry the extended shapes. No new accessor is required.
4. Ranking arithmetic the live composite must preserve: `source_trust_weight` is read at `ranking.rs:440, 552` and applied as `effective_discount = trust_weight × failure_ratio / 1000`. The new `wait_sensitivity_weight` (added in ticket 001) is read in the same context. `memory_retention_ticks` (existing field, default 400) bounds the capacity freshness window per spec D4 pseudocode. The composite formula must match the spec: `post = motive × (1000 − trust_discount) / 1000`, then `post = post.saturating_sub(wait_penalty).saturating_add(capacity_signal).max(1)`, with an early `None` return only when all computed axes are zero (`failure_ratio == 0`, `wait_penalty == 0`, and `capacity_signal == 0`). This preserves traceability for real observations whose score effect floors back to the original motive.
5. The four existing tests will need behavior updates rather than removal: `source_reliability_discount_skips_non_commodity_goals` is structural (no commodity → no discount) and stays unchanged; the other three currently assert `None` for absent-profile or absent-record cases and still return `None` before reaching the composite. The zero-failure case is renamed to `source_reliability_discount_returns_none_when_all_axes_zero` and now documents the no-signal early-out. Extend the failure-ratio test to also assert the new composite fields are zero when no wait/capacity observations exist, and add focused tests proving wait-only, capacity-fresh, and capacity-stale axes.
7. The composite refactor preserves `SourceReliabilityDiscount` as the trace name (rename to `SourceCompositeAdjustment` was rejected during reassessment to avoid cascading import edits in `agenda_manager`, `plan_selection`, and trace consumers — "discount" remains a fair label since trust+wait subtract from motive).
8. No heuristic is being removed; the failure-only discount is being subsumed into a strictly more general composite. The early-out removal is a precondition-relaxation: the new computation always runs once the source record and preference profile exist, but returns `None` when no axis has a signal.

## Architecture Check

1. Single composite path replaces parallel discount layers. An alternative ("keep failure-only path, layer wait/capacity as a separate discount") was considered and rejected: parallel discount paths would scatter `SourceReliability` reads across two functions, complicate trace reconstruction, and require a second `Option<SourceCompositeAdjustment>` field on `AgendaEntry`. A single composite returning the existing `SourceReliabilityDiscount` shape (extended with new fields) is FND-3 cleaner — one concrete derived score per candidate per tick — and FND-27 honest — the composite is a derived view of stored state, never authoritative.
2. No backwards-compatibility shim. The failure-ratio early-out is removed; the function's surface (`Option<SourceReliabilityDiscount>` return) is preserved. Old tests that relied on `None` for failure-ratio-zero cases without other observations continue to pass because the new all-zero-axis early-out covers that case. No deprecated `apply_failure_only_discount` shim is left behind.
3. Preserving the type name `SourceReliabilityDiscount` keeps consumers in `agenda_manager`, `plan_selection`, and the decision-trace formatter on a stable surface. Field additions are additive; the formatter line is the only consumer that needs updating to surface the new fields.

## Verification Layers

1. Composite computation correctness — focused unit tests in `ranking.rs` `#[cfg(test)]` block: feed `(motive, trust_weight, failure_ratio, wait_weight, average_wait, retention_ticks, capacity, capacity_age)` tuples and assert the returned `SourceReliabilityDiscount` matches the spec formula. Cover failure-only, wait-only, capacity-only, all-three, and stale-capacity (zero contribution) cases.
2. Early-out semantics — focused unit test asserting `None` when all three signal axes are zero (no observations of any kind), and `Some(_)` when at least one axis contributes a non-zero adjustment.
3. Trace surface fidelity — focused unit test or doc test asserting the extended `SourceReliabilityDiscount` Display formatter emits `source_reliability=entity=_ commodity=_ failure=_ wait_avg=_ wait_pen=_ cap=_ cap_age=_ cap_sig=_ pre=_ post=_` for a representative composite.
4. Decision-trace surface — the existing decision-trace integration tests (e.g., golden_experience_preferences) continue to render `SourceReliabilityDiscount` lines without panicking on the new fields.
5. Single architectural layer (ranking) — the action layer, event-log delta, and candidate emission are all unaffected. No action-trace mapping required.

## What to Change

### 1. Restructure `apply_source_reliability_discount`

In `crates/worldwake-ai/src/ranking.rs:419–452`:

- Remove the `if failure_ratio == 0 { return None; }` early-out (lines 436–438).
- Compute `failure_ratio = failure_ratio_permille(record)` regardless of value.
- Compute `trust_discount = trust_weight × failure_ratio / 1000`.
- Read `wait_weight = u32::from(profile.wait_sensitivity_weight.value())`.
- Compute `wait_penalty = record.average_wait_ticks.saturating_mul(wait_weight) / 1000`.
- Compute `capacity_freshness = context.current_tick.0.saturating_sub(record.last_observed_capacity_tick.0)`.
- Compute `capacity_signal`:
  - If `capacity_freshness > profile.memory_retention_ticks`: `0` (stale).
  - Else if `profile.memory_retention_ticks == 0` (degenerate guard): `u32::from(record.last_observed_capacity)`.
  - Else: `freshness_factor = 1000 - (capacity_freshness × 1000 / profile.memory_retention_ticks)`; `capacity_signal = u32::from(record.last_observed_capacity) × freshness_factor / 1000`.
- Compute `post = motive_score × (1000 − trust_discount) / 1000` (use `saturating_*`).
- Apply `post = post.saturating_sub(wait_penalty).saturating_add(capacity_signal).max(1)`.
- Early return: `if failure_ratio == 0 && wait_penalty == 0 && capacity_signal == 0 { return None; }` — no axis contributed a meaningful signal.
- Return `Some(SourceReliabilityDiscount { source_entity, commodity, failure_ratio_permille: failure_ratio, average_wait_ticks: record.average_wait_ticks, wait_penalty, last_observed_capacity: record.last_observed_capacity, capacity_freshness_ticks: capacity_freshness, capacity_signal, pre_discount_motive: motive_score, post_discount_motive: post })`.

### 2. Restructure `apply_source_reliability_discount_with_pending_failures`

In `crates/worldwake-ai/src/ranking.rs:532–579`, mirror the changes from Section 1 with the existing pending-failure semantics preserved:

- The function reads the agent's stored `record` and constructs a *synthetic* copy with `record.failed_attempts += 1` (line 562) to simulate the pending failure. This synthetic increment is used only for `failure_ratio` computation.
- Wait and capacity components must be computed from the **actual stored record** (not the synthetic copy) — pending failures don't change observed wait time or capacity.
- Remove the `if failure_ratio == 0 { return None; }` early-out (lines 564–566) — pending failures should always trigger the composite computation.
- Same early `None` return when all computed axes are zero.
- Same `SourceReliabilityDiscount` construction.

### 3. Extend `SourceReliabilityDiscount` struct

In `crates/worldwake-ai/src/decision_trace.rs:546–552`, add five fields after the existing `failure_ratio_permille` field and before `pre_discount_motive`:

- `pub average_wait_ticks: u32`
- `pub wait_penalty: u32`
- `pub last_observed_capacity: u16`
- `pub capacity_freshness_ticks: u64`
- `pub capacity_signal: u32`

Order: keep the existing fields' positions stable (`source_entity`, `commodity`, `failure_ratio_permille` first; `pre_discount_motive`, `post_discount_motive` last); insert the five new fields between `failure_ratio_permille` and `pre_discount_motive` so the trace output reads left-to-right by causal contribution.

### 4. Extend `SourceReliabilityDiscount` Display formatter

In `crates/worldwake-ai/src/decision_trace.rs` around the existing formatting (spec cites lines 1952–1961; locate by reading 30 lines around the cited range during implementation):

- Replace the current `source_reliability=entity=_ commodity=_ failure=_ pre=_ post=_` template with `source_reliability=entity=_ commodity=_ failure=_ wait_avg=_ wait_pen=_ cap=_ cap_age=_ cap_sig=_ pre=_ post=_`.
- Use the same field-formatting style as the existing line (sibling discount formatters in the same file are the reference).

### 5. Update construction sites for `SourceReliabilityDiscount`

Add the five new fields (defaulted to zero unless the test specifically asserts on them) at every literal construction site outside `apply_source_reliability_discount` and its variant:

- `crates/worldwake-ai/src/ranking.rs` (lines around 5721, 5780, 5923-5924, 6003, 6122 per Step 2 grep — most are test fixtures asserting on the discount shape).
- `crates/worldwake-ai/src/goal_model.rs:2839` (likely a test fixture).
- `crates/worldwake-ai/src/agent_tick/planning.rs:4139` (likely a test fixture).
- Any decision-trace test fixtures that construct the struct literally.

### 6. Update the four existing ranking tests

Behaviorally:

- `source_reliability_discount_skips_non_commodity_goals:5563` — no change required (still returns `None` for non-commodity goals via `source_reliability_discount_scope`).
- `source_reliability_discount_returns_none_without_experience:5582` — agent has no `SourceReliability`; `?` propagation at the existing line still returns `None` before reaching the composite. No change required.
- `source_reliability_discount_returns_none_without_preference_profile:5625` — agent has no `PreferenceProfile`; same `?` propagation. No change required.
- `source_reliability_discount_applies_failure_ratio_proportionally:5670` — currently asserts the failure-only discount math. Extend to also confirm the new composite fields are zero (`average_wait_ticks: 0`, `wait_penalty: 0`, `last_observed_capacity: 0`, `capacity_signal: 0`) when no wait/capacity observations exist. The original `pre_discount_motive` / `post_discount_motive` math continues to hold.

Add new tests after line 5670:

- `source_reliability_discount_applies_wait_penalty_alone_when_no_failures`: agent has `average_wait_ticks: 12`, `wait_observation_count: 3`, no failures, no capacity observations; assert returned `wait_penalty > 0` and `post_discount_motive < pre_discount_motive`.
- `source_reliability_discount_applies_capacity_signal_within_freshness_window`: agent has `last_observed_capacity: 18` with capacity freshness of 100 ticks and `memory_retention_ticks: 400`; assert `capacity_signal > 0` (75% freshness factor; `18 × 750 / 1000 == 13`) and `post_discount_motive > pre_discount_motive`.
- `source_reliability_discount_zeroes_capacity_signal_when_stale`: agent has `last_observed_capacity: 18` at Tick(100), current tick Tick(600), `memory_retention_ticks: 400`; assert `capacity_freshness_ticks > memory_retention_ticks` and `capacity_signal == 0`.
- `source_reliability_discount_returns_none_when_all_axes_zero`: agent has a `SourceReliability` entry but `failure_ratio == 0`, `average_wait_ticks == 0`, `last_observed_capacity == 0`; assert returned `None`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify) — restructure both discount functions, update fixture construction sites, extend existing test, add 4 new tests.
- `crates/worldwake-ai/src/decision_trace.rs` (modify) — extend `SourceReliabilityDiscount` struct + Display formatter.
- `crates/worldwake-ai/src/goal_model.rs` (modify) — fixture update at line 2839.
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify) — fixture update at line 4139.
- `archive/specs/S131-source-reliability-wait-capacity.md` (modify) — truth-sync the composite early-out pseudocode to the landed all-zero-axis contract.

## Out of Scope

- Adding new fields to `ReliabilityRecord` or `PreferenceProfile` — covered by ticket 001 (this ticket assumes those fields exist).
- Wait observation hooks at grant promotion — covered by ticket 002 (this ticket assumes `average_wait_ticks` is populated by S131SOURELWAI-002 when actual wait events occur, but the ranking math itself runs correctly with all-zero observations from any source).
- Capacity observation in perception — covered by ticket 003 (same dependency relationship as ticket 002).
- Cross-tick golden coverage — covered by ticket 005.
- Renaming `SourceReliabilityDiscount` — explicitly preserved; rename was rejected during spec reassessment.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai source_reliability_discount_applies_wait_penalty_alone_when_no_failures`
2. `cargo test -p worldwake-ai source_reliability_discount_applies_capacity_signal_within_freshness_window`
3. `cargo test -p worldwake-ai source_reliability_discount_zeroes_capacity_signal_when_stale`
4. `cargo test -p worldwake-ai source_reliability_discount_returns_none_when_all_axes_zero`
5. `cargo test -p worldwake-ai source_reliability_discount_applies_failure_ratio_proportionally` — extended assertions still pass.
6. `cargo test -p worldwake-ai source_reliability_discount_skips_non_commodity_goals` — unchanged behavior.
7. `cargo test -p worldwake-ai source_reliability_discount_returns_none_without_experience` — unchanged behavior.
8. `cargo test -p worldwake-ai source_reliability_discount_returns_none_without_preference_profile` — unchanged behavior.
9. Existing decision-trace and ranking integration tests continue to pass (`golden_experience_preferences`, `golden_ai_decisions`, etc.).
10. Existing suite: `cargo test --workspace`.

### Invariants

1. The composite computation runs on every per-candidate evaluation that has both `SourceReliability` and `PreferenceProfile` for the agent — no failure-history precondition. Wait and capacity signals always contribute when their underlying observations are non-zero and fresh.
2. The composite produces no adjustment (returns `None`) when all three axes are zero — agents with no observations at all still see uniform motive scores across sources (FND-16: ignorance is first-class; ranking falls back to default ordering).
3. `SourceReliabilityDiscount` field order keeps `source_entity, commodity, failure_ratio_permille` first and `pre_discount_motive, post_discount_motive` last; the five new fields are inserted between for left-to-right causal readability.
4. The pending-failure variant computes wait and capacity from the actual stored record, not the synthetic incremented copy — pending failures change failure projection but not observed wait/capacity history.
5. Display formatter emits all 10 axis labels (entity, commodity, failure, wait_avg, wait_pen, cap, cap_age, cap_sig, pre, post) per FND-29 (debuggability is a product feature).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — 4 new `#[test]` fns + 1 extended existing test per Section 6 of What to Change.
2. `crates/worldwake-ai/src/decision_trace.rs` — if a Display test exists in `#[cfg(test)]`, extend it to assert the new formatter shape; otherwise add a focused doc-test or unit test that constructs a `SourceReliabilityDiscount` and asserts its formatted output.

### Commands

1. `cargo test -p worldwake-ai source_reliability_discount` — all source-reliability discount tests by name filter.
2. `cargo test -p worldwake-ai` — confirms decision-trace and integration tests still pass.
3. `cargo test --workspace` — confirms cross-crate impact on belief views and downstream consumers.
4. `scripts/verify.sh` — full pre-PR gate. This implementation ran the wrapper's live component gates directly rather than invoking the wrapper as one command.

## Outcome

Completed on 2026-05-03.

- Replaced the failure-only source-reliability ranking path with one shared composite helper used by both normal ranking and pending-failure reranking.
- Extended `SourceReliabilityDiscount` and its rendered decision-trace summary with wait/capacity fields: `average_wait_ticks`, `wait_penalty`, `last_observed_capacity`, `capacity_freshness_ticks`, and `capacity_signal`.
- Updated existing ranking/decision-trace fixtures and added focused ranking coverage for wait-only, fresh-capacity, stale-capacity, and all-zero-axis behavior.
- Updated the active S131 spec pseudocode so it no longer describes the rejected `post == motive_score` early-out.

## Deviations

- The draft `post == motive_score` early-out was narrowed to an explicit all-zero-axis early-out. This preserves the existing trace for a real failure signal when a tiny positive motive floors back to `1`.
- The fresh-capacity test uses an equivalent 100-tick freshness window (`Tick(0)` observed, `Tick(100)` current) rather than the drafted `Tick(100)` to `Tick(200)` setup.
- `scripts/verify.sh` was inspected, then its live gates were run directly after `cargo test --workspace` instead of rerunning the wrapper end-to-end.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib source_reliability_discount` (13 tests; the earlier `--exact` selector matched 0 tests and was discarded as non-proof).
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test --workspace`.
- Passed `cargo fmt --all -- --check`.
- Passed `bash scripts/check_active_goal_removed.sh`.
- Passed `cargo clippy --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Passed `git diff --check` after final ticket/spec Markdown edits.
