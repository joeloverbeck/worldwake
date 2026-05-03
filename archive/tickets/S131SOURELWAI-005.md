# S131SOURELWAI-005: Golden coverage for wait/capacity learning

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — golden test additions only
**Deps**: archive/tickets/S131SOURELWAI-001.md, archive/tickets/S131SOURELWAI-002.md, archive/tickets/S131SOURELWAI-003.md, archive/tickets/S131SOURELWAI-004.md

## Problem

Tickets 001–004 deliver the per-component changes (field extension, two grant hooks, perception hook, composite ranking) but no end-to-end golden exercises the cross-tick chain "agent waits → observation written → next ranking pass weighs the wait". Without a golden, the spec's headline scenario ("Agent A and B competing at North Orchard") is verified only by the composition of focused tests, none of which exercise the full Tick → grant → observe → re-rank → re-decide loop. This ticket adds `golden_source_reliability.rs` covering five scenarios that prove the learning surface fires correctly under realistic multi-tick agent simulation.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The `crates/worldwake-ai/tests/` directory contains existing golden tests following the `golden_<system>_<scenario>.rs` convention (per Step 2 ls: `golden_experience_preferences.rs`, `golden_quantity_aware_acquisition.rs`, `golden_survival_baseline.rs`, etc.). The new file `golden_source_reliability.rs` follows the same convention. The golden harness is `crates/worldwake-ai/tests/golden_harness/mod.rs` (per Step 2 grep finding `worldwake_core::ResourceExtractionQueues` references at lines 937, 971). Goldens are listed in `docs/generated/golden-e2e-inventory.md` after running `python3 scripts/golden_inventory.py --write --check-docs`.
2. The intended verification layer is **golden E2E** per `docs/precision-rules.md` Rule 3 — the cross-tick chain spans multiple SystemFns (perception, contention, harvest action lifecycle, ranking) and cannot be exercised by a single focused unit test. Per `docs/golden-e2e-testing.md`, golden tests use `BeliefStoreSnapshot` and decision-trace assertions for stable proof surfaces; this ticket follows that convention.
3. Per FND-21 (intentions are revisable commitments), the second golden scenario must let the planner re-rank after wait observations accumulate — this requires a multi-tick run where the agent's `agent_tick` cycles through the planning pipeline at least twice with different `SourceReliability` state.
4. The harvest start path that triggers the resource-extraction wait observation (ticket 002 D2b hook) requires an agent with `PerceptionProfile` to observe post-grant world state — per `CLAUDE.md`'s "Golden production tests require `PerceptionProfile` on agents that need to observe post-production output" — and at least two believed sources for the planner to actually exercise the composite ranking from ticket 004.
5. Test scenarios must use `Permille` for `wait_sensitivity_weight` per `docs/spec-drafting-rules.md` Section 3 (no f32/f64) — the spec D5 baseline is `Permille::new_unchecked(150)`; the high-sensitivity scenario uses `Permille::new_unchecked(800)`.
6. No existing golden currently exercises wait/capacity learning. `golden_experience_preferences.rs` covers the failure-ratio side of source reliability; this ticket adds the wait/capacity side as a separate file rather than extending the existing one — separation keeps the scope of each golden focused per `docs/golden-e2e-testing.md` guidance.
8. The spec's D7 scenario "After 32 wait observations, the EMA replaces the running mean" is intrinsically hard to set up in a multi-tick simulation (would require 32 distinct grant cycles within the test). The focused unit test `observe_wait_switches_to_ema_after_32` from ticket 001 already proves this; the golden version is replaced with a simpler wait-promotion scenario that exercises the resource-extraction grant hook with realistic harness setup. State this scope adjustment explicitly so reviewers understand the EMA and exact running-mean contracts are verified by ticket 001's focused tests, not by this golden.
9. Final live reassessment narrowed the golden layer to cross-layer composition: real resource-extraction queue promotion writes wait memory, real perception writes capacity memory, and AI ranking reads stored wait/capacity fields through `SourceReliabilityDiscount` in decision traces. This avoids duplicating the archived lower-layer tests for exact recurrence math and hook mutation details.

## Architecture Check

1. Golden tests are the canonical proof surface for cross-tick, cross-system invariants per `docs/golden-e2e-testing.md`. A unit test cannot prove "wait observation written by the contention handler is read by the ranking phase on a subsequent tick" because it spans two SystemFn invocations and the agent_tick decision cycle.
2. No backwards-compatibility shim. The new file is purely additive; no existing golden is modified.
3. The golden does not duplicate ticket 002–004's focused tests — those prove component-level correctness; this golden proves cross-tick composition. Per `docs/precision-rules.md` Rule 5, this is the intended layer for the "agent learns over time" invariant.

## Verification Layers

1. Resource-extraction wait observation -> component-state assertion on the queued actor's `SourceReliability` after a real first harvester commits and the queued actor is later promoted through the live extraction queue.
2. Capacity observation -> component-state assertion that a real perception tick writes `last_observed_capacity` and `last_observed_capacity_tick` for the same `(source, commodity)` key ranking later consumes.
3. Fresh capacity contribution -> decision-trace assertion that fresh capacity produces `capacity_signal > 0` and increases the post-composite motive.
4. Capacity-freshness staleness -> decision-trace assertion that an old `last_observed_capacity_tick` produces `capacity_signal == 0` after `current_tick - last_observed_capacity_tick > memory_retention_ticks`.
5. High-`wait_sensitivity_weight` re-ranking -> decision-trace assertion that stored repeated wait observations make the next `AcquireCommodity { Apple }` decision switch from the closer orchard to the farther uncontested orchard.

## What to Change

### 1. Create `golden_source_reliability.rs`

Add `crates/worldwake-ai/tests/golden_source_reliability.rs` with the following test functions, each setting up a focused scenario via the standard golden harness:

Final landed functions:

- `resource_extraction_wait_observation_records_when_promoted`
- `capacity_observation_records_from_perception`
- `capacity_signal_within_retention_window_contributes_to_motive`
- `capacity_freshness_zeros_signal_after_retention_window`
- `high_wait_sensitivity_agent_prefers_alternative_after_three_wait_observations`

The draft `wait_observation_running_mean_accumulates_across_grant_cycles` setup below is superseded by the landed resource-extraction wait-promotion golden plus the archived ticket 001/002 focused proofs. The exact running-mean and EMA arithmetic is not duplicated at the golden layer.

#### `wait_observation_running_mean_accumulates_across_grant_cycles`

- Spawn one well-class facility with `ContentionQueue`, `ContentionPolicy { auto_promote: true, ... }`, `ResourceSource { commodity: Water, available_quantity: Quantity(100), extraction_slots: 1, ... }`, and `ResourceExtractionQueues` with one slot.
- Spawn one acting agent with universal profiles (perception, preference) using defaults.
- Run 5 sequential queue-grant cycles where the agent waits respectively (3, 5, 8, 12, 2) ticks before being granted access. The simplest setup: a "blocker" agent already holds the grant for N ticks, then expires; the test agent (already enqueued) is promoted at Tick(start + N). Repeat 5 times with new blockers.
- Assert after the 5th grant: `agent.SourceReliability.sources[SourceKey { entity: facility, commodity: Water }].average_wait_ticks == 5` (the deterministic integer recurrence over `(3, 5, 8, 12, 2)`) and `wait_observation_count == 5`.

#### `high_wait_sensitivity_agent_prefers_alternative_after_three_wait_observations`

- Spawn two equivalent water-source facilities A and B (same `ResourceSource`, same `ContentionPolicy`, same `ContentionQueue`); facility A is initially preferred (e.g., closer in topology so motive_score is slightly higher).
- Spawn the test agent with `PreferenceProfile { wait_sensitivity_weight: Permille::new_unchecked(800), ..Default::default() }`.
- Spawn 1–2 competing agents that contend with the test agent at facility A but not at facility B, so the test agent records wait observations only at A.
- Run the simulation long enough for the test agent to accumulate 3 wait observations at A while having zero at B.
- Assert via decision trace that on the 4th `AcquireCommodity { commodity: Water }` decision, the chosen source switches from A to B — the `wait_penalty` accumulated at A exceeds the small motive advantage A had.

#### `capacity_freshness_zeros_signal_after_retention_window`

- Spawn one water source, one agent.
- Tick perception once at Tick(100) with the source at `available_quantity: Quantity(18)`.
- Assert `agent.SourceReliability.sources[key].last_observed_capacity == 18` and `last_observed_capacity_tick == Tick(100)`.
- Run the simulation forward to Tick(600) without re-perceiving the source (move the agent away or block perception). With `PreferenceProfile::default().memory_retention_ticks == 400`, capacity_freshness (500) exceeds retention (400).
- Trigger a ranking pass on a new `AcquireCommodity { commodity: Water }` decision at Tick(600); assert via decision trace that the `SourceReliabilityDiscount.capacity_signal == 0` for this source.

#### `capacity_signal_within_retention_window_contributes_to_motive`

- Same setup as above but only run forward to Tick(300) — capacity_freshness (200) is well within retention (400).
- Assert via decision trace that `SourceReliabilityDiscount.capacity_signal > 0` and the `post_discount_motive > pre_discount_motive` (capacity contribution adds to motive).

#### `resource_extraction_wait_observation_records_when_promoted`

- Spawn one orchard with `ResourceSource { commodity: Apple, available_quantity: Quantity(5), extraction_slots: 1 }` and `ResourceExtractionQueues` with one slot.
- Spawn two agents with `PerceptionProfile`. Agent A acquires the slot at Tick(10) and starts harvesting (action duration 10 ticks). Agent B attempts harvest at Tick(11), gets `extraction_slots_full`, and is enqueued via the existing harvest-failure handler.
- Tick the simulation to Tick(20) when agent A's harvest commits, freeing the slot. Agent B's next harvest start at Tick(20) succeeds and gets the grant.
- Assert agent B's `SourceReliability.sources[SourceKey { entity: orchard, commodity: Apple }].average_wait_ticks == 9` (Tick(20) − Tick(11)) and `wait_observation_count == 1`.

### 2. Regenerate golden inventory

Run `python3 scripts/golden_inventory.py --write --check-docs` to register the new file in `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_source_reliability.rs` (new) — 5 golden test functions per Section 1.
- `docs/generated/golden-e2e-inventory.md` (modify, regenerated) — auto-updated by the inventory script.
- `docs/generated/golden-scenario-index.md` (modify, regenerated) — auto-updated.
- `docs/generated/golden-scenario-details/` (modify, regenerated) — auto-updated per-file detail.

## Out of Scope

- Modifying any production code in `worldwake-core`, `worldwake-systems`, or `worldwake-ai/src` — all production changes land in tickets 001–004.
- Verifying the EMA transition at the 33rd wait observation — covered by the focused unit test `observe_wait_switches_to_ema_after_32` in ticket 001 (golden setup of 33 grant cycles is impractical for harness simplicity; the focused test exercises the same code path).
- Cross-agent reliability sharing via `ShareBelief` — explicit Non-Goal in the spec.
- Hostile/non-hostile encounter reliability extension to `RouteExperience` — explicit Non-Goal in the spec.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_source_reliability resource_extraction_wait_observation_records_when_promoted -- --exact`
2. `cargo test -p worldwake-ai --test golden_source_reliability high_wait_sensitivity_agent_prefers_alternative_after_three_wait_observations`
3. `cargo test -p worldwake-ai --test golden_source_reliability capacity_freshness_zeros_signal_after_retention_window`
4. `cargo test -p worldwake-ai --test golden_source_reliability capacity_signal_within_retention_window_contributes_to_motive`
5. `cargo test -p worldwake-ai --test golden_source_reliability capacity_observation_records_from_perception`
6. New golden binary: `cargo test -p worldwake-ai --test golden_source_reliability`.
7. Inventory script runs clean: `python3 scripts/golden_inventory.py --check-docs` reports no diff after regeneration.

### Invariants

1. A real resource-extraction queue promotion writes a positive wait observation through the production hook. Exact running-mean and EMA arithmetic remain the archived ticket 001 focused-test contract.
2. An agent with high `wait_sensitivity_weight` revises its acquisition preference in favor of less-contested alternatives once enough wait observations accumulate — FND-21 (intentions are revisable) verified end-to-end.
3. Capacity signal contributes to motive within the retention window and zeros out beyond it — FND-29A (history is append-only) preserved by *discounting* stale observations rather than overwriting them.
4. Resource-extraction grants produce the same wait observation semantics as facility-queue grants — both substrates are first-class learning surfaces per ticket 002's design.
5. Goldens use only state-mediated reads (component state, decision trace) and never query global authoritative truth on behalf of an agent — FND-7 / FND-15 preserved.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_source_reliability.rs` — 5 new golden test functions per the final landed Section 1 of What to Change.
2. `docs/generated/golden-e2e-inventory.md` and sibling generated files — auto-updated; no manual edit.

### Commands

1. `cargo test -p worldwake-ai --test golden_source_reliability` — narrowest verification while iterating on the new file.
2. `cargo test -p worldwake-ai` — affected-crate broad verification for the new golden integration.
3. `python3 scripts/golden_inventory.py --write --check-docs` — regenerate inventory and confirm clean.
4. `cargo test --workspace` — full workspace gate.
5. `scripts/verify.sh` — full pre-PR gate.

## Outcome

Completed on 2026-05-03.

- Added `crates/worldwake-ai/tests/golden_source_reliability.rs` with five scenarios, numbered 137-141, covering resource-extraction wait writes, perception capacity writes, fresh capacity ranking, stale capacity zeroing, and wait-memory source re-ranking.
- Regenerated golden docs, adding `docs/generated/golden-scenario-details/source-reliability.md` and refreshing the inventory, scenario index, coverage matrix, and line-number-only generated detail updates.

## Deviations

- Replaced the drafted `wait_observation_running_mean_accumulates_across_grant_cycles` autonomous golden with `resource_extraction_wait_observation_records_when_promoted` plus the existing archived focused proofs. This keeps the golden on the cross-system composition seam and avoids duplicating lower-layer running-mean/EMA tests.
- The stale-capacity golden seeds a small wait observation so the composite trace is emitted while still proving `capacity_signal == 0`; the ranking path intentionally returns no `SourceReliabilityDiscount` when every axis is zero.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_source_reliability -- --list`.
- Passed `cargo test -p worldwake-ai --test golden_source_reliability`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo test --workspace`.
