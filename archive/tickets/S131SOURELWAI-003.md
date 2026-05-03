# S131SOURELWAI-003: Capacity observation hook in perception

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-systems` (perception system gains a per-actor capacity observation write alongside existing belief writes)
**Deps**: archive/tickets/S131SOURELWAI-001.md

## Problem

Today an agent who perceives a `ResourceSource` writes the source's `available_quantity` into its belief store but does not record a per-source learning trace for "how much was available the last time I looked." Without that signal, the planner cannot weigh "this orchard usually has plenty" vs. "this well has been empty since Tick 200" when ranking acquisition candidates. This ticket adds a co-located perception hook that updates `ReliabilityRecord.last_observed_capacity` and `last_observed_capacity_tick` per (source, commodity) key whenever the agent perceives a `ResourceSource`.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-systems/src/perception.rs` defines `pub fn perception_system(ctx: SystemExecutionContext<'_>)`. The live fresh-perception seam for co-located resource sources is `collect_direct_local_observation_batch(...)` building `BelievedEntityState` snapshots through `build_believed_entity_state(...)`, followed by `apply_direct_local_observation_batch(...)` recording those snapshots into the observer's `AgentBeliefStore`. The earlier grep hits at former lines 1797 / 4217 / 4250 are now test-fixture literals, not the production write site. The existing test `passive_observation_emits_discovery_for_resource_source_mismatch` exercises the resource-source direct-observation path. `ResourceSource { commodity: CommodityKind, available_quantity: Quantity, ... }` lives in `crates/worldwake-core/src/production.rs`. `SourceKey { entity, commodity }` is the existing key convention used in `apply_source_reliability_discount` (`ranking.rs`) and `experience_recording.rs`.
2. Per FND-14A (same-tick co-located observation is belief-equivalent), the perception hook may read authoritative `ResourceSource.available_quantity` for entities the agent is co-located with — this is the same fact a correct perception pipeline would deliver to the agent's beliefs on the same tick. The hook is therefore lawful regardless of belief-store latency.
3. Cross-system boundary under audit: perception (`worldwake-systems`) writes the agent's `SourceReliability` (ECS component owned by the agent in `worldwake-core`). Per FND-26 this is a state-mediated write within the existing perception walk — no new SystemFn is introduced. The boundary is the perception SystemFn → `SourceReliability` component, identical in shape to the existing perception → `AgentBeliefStore` writes.
4. The single caller location for the new hook needs to be the direct-local observation batch over co-located entities. The hook should consume `batch.observed_snapshots` after the batch is collected, so it fires only for entities that passed the observer's perception check and avoids witness/event replay paths.
6. Intended verification layer is focused/unit (`#[cfg(test)]` block of `perception.rs`) — the capacity-observation write is a state mutation observable by reading the actor's `SourceReliability` after `perception_system` ticks. Cross-tick freshness decay is verified at the ranking layer in ticket 004 (the discount computation reads `last_observed_capacity_tick` and discounts older observations). Golden coverage of capacity learning + ranking lands in ticket 005.

## Architecture Check

1. Piggy-backing on the existing perception walk avoids introducing a new SystemFn and keeps the observation tied to the actor's actual perception event. A separate "capacity observation system" would duplicate the co-location traversal for no architectural benefit.
2. No backwards-compatibility shim. The new write is a fresh state mutation on `SourceReliability`; no parallel "old capacity memory" path is introduced.
3. The hook uses `ReliabilityRecord::new(current_tick)` from ticket 001 to insert when no record exists for the (source, commodity) key. This is the same pattern as `experience_recording.rs:15` after that file's migration in ticket 001 — consistent construction conventions across the workspace.

## Verification Layers

1. Capacity-observation correctness on fresh perception → focused/unit test in `perception.rs` `#[cfg(test)]` block: an actor co-located with a `ResourceSource { commodity: Apple, available_quantity: 18 }` at Tick(100) ticks `perception_system`; assert the actor's `SourceReliability.sources[SourceKey { entity: source, commodity: Apple }].last_observed_capacity == 18` and `last_observed_capacity_tick == Tick(100)`.
2. Repeated observation overwrites prior capacity → focused/unit test: same actor and source, observe at Tick(100) capacity 18, then at Tick(200) capacity 5 (after a draw); assert `last_observed_capacity == 5` and `last_observed_capacity_tick == Tick(200)`.
3. Single-layer ticket: this ticket changes only the perception-system write surface and the agent's `SourceReliability` component. Ranking-side discount on stale capacity is verified in ticket 004; cross-tick golden in ticket 005.

## What to Change

### 1. Add capacity-observation hook in `perception.rs`

In `crates/worldwake-systems/src/perception.rs`, attach capacity observation to the direct-local observation batch produced for each observer. The active production seam is the `DirectLocalObservationBatch.observed_snapshots` map, not the stale grep-hit fixture literals.

After the chosen belief write fires for a perceived `ResourceSource`-bearing entity:

- Quantity-to-u16 conversion: read `source.available_quantity` (currently `Quantity` per `production.rs:77`). Convert via `let observed_capacity: u16 = u16::try_from(source.available_quantity.0).unwrap_or(u16::MAX);` (saturating cast — `Quantity` may be wider than `u16` in principle; a saturated value still preserves the "this source has lots" signal).
- Build `let key = SourceKey { entity: source_entity, commodity: source.commodity };`.
- Fetch the actor's `SourceReliability` via the SystemFn's world handle (matching the pattern used by the surrounding belief write); use `or_insert_with(|| ReliabilityRecord::new(current_tick))` on `reliability.sources.entry(key)`.
- Call `record.observe_capacity(observed_capacity, current_tick)`.
- Write the updated `SourceReliability` back through the same world handle as the existing belief write.

This must be a side-effect of the existing perception walk, not a new independent scan over all nearby sources. The write may be staged into a per-agent `SourceReliability` update map and committed in the same perception-system transaction that writes updated belief stores.

### 2. Add focused tests

In the `perception.rs` `#[cfg(test)]` block (after `passive_observation_emits_discovery_for_resource_source_mismatch:4200`):

- `perception_writes_capacity_observation_for_co_located_resource_source`: spawn an actor in a place containing a `ResourceSource { commodity: Apple, available_quantity: Quantity(18), ... }`; tick `perception_system` at Tick(100); assert `SourceReliability.sources[SourceKey { entity: source, commodity: Apple }] == ReliabilityRecord { last_observed_capacity: 18, last_observed_capacity_tick: Tick(100), ..rest zero }`.
- `perception_overwrites_capacity_observation_on_subsequent_tick`: same setup; tick at Tick(100) (capacity 18), mutate the source to `available_quantity: Quantity(5)`, tick at Tick(200); assert the record now shows `last_observed_capacity == 5` and `last_observed_capacity_tick == Tick(200)`. Assert the original three reliability fields (`successful_acquisitions`, `failed_attempts`, `last_attempt_tick`) are untouched by capacity observation.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify) — perception walk gets capacity-observation side-effect + 2 new tests.

## Out of Scope

- Wait observation hooks at grant-promotion sites — covered by ticket 002.
- Ranking-side discount of stale capacity observations — covered by ticket 004.
- Golden cross-tick verification of capacity-driven ranking changes — covered by ticket 005.
- Off-place or memory-backed capacity observation — Non-Goal per spec FND-14A reasoning (capacity observation is co-located perception only).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems perception::tests::perception_writes_capacity_observation_for_co_located_resource_source`
2. `cargo test -p worldwake-systems perception::tests::perception_overwrites_capacity_observation_on_subsequent_tick`
3. Existing `passive_observation_emits_discovery_for_resource_source_mismatch:4200` continues to pass — the new capacity write must not perturb the discovery path's existing assertions.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. Capacity observation never decreases `last_observed_capacity` *and then increases it back* without a corresponding `last_observed_capacity_tick` advance — `observe_capacity` always overwrites both fields together (FND-29A: append-only history for the agent's perception, but the stored representation is a single latest-observation slot; the freshness discount in ranking provides the temporal weighting).
2. The capacity observation does not touch `successful_acquisitions`, `failed_attempts`, or `last_attempt_tick` — these are written only by `experience_recording.rs` on actual harvest outcomes.
3. The hook fires only for co-located perception (FND-14A) — agents observing through reports/witnesses do not write to `last_observed_capacity` (those paths flow through belief store updates, not through `SourceReliability`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — 2 new `#[test]` fns in the existing `#[cfg(test)]` block per Section 2 of What to Change.

### Commands

1. `cargo test -p worldwake-systems --lib perception::tests::perception_writes_capacity_observation_for_co_located_resource_source -- --exact` — focused fresh-observation proof.
2. `cargo test -p worldwake-systems --lib perception::tests::perception_overwrites_capacity_observation_on_subsequent_tick -- --exact` — focused overwrite proof.
3. `cargo test -p worldwake-systems --lib perception::tests::passive_observation_emits_discovery_for_resource_source_mismatch -- --exact` — existing resource-source discovery regression.
4. `cargo test -p worldwake-systems` — affected crate proof.
5. `./scripts/verify.sh` — full live wrapper gate.

## Outcome

Completed on 2026-05-03.

- Added the perception-system capacity observation hook in `crates/worldwake-systems/src/perception.rs`. The hook consumes `DirectLocalObservationBatch.observed_snapshots`, so it writes `SourceReliability.sources[SourceKey { entity, commodity }].last_observed_capacity` only for resource sources actually perceived through the direct co-located observation path.
- Added focused unit coverage for first observation and subsequent overwrite behavior. The overwrite test also verifies that capacity observation leaves `successful_acquisitions`, `failed_attempts`, `last_attempt_tick`, `average_wait_ticks`, and `wait_observation_count` untouched.
- Reassessed the stale grep-line premise: the cited line hits were test fixture literals on the live branch. The landed production seam is the direct-local observation batch rather than a separate scan or a witness/report path.

## Deviations

- The implementation stages `SourceReliability` updates into a per-agent map and commits them in the existing perception-system mutation transaction, instead of mutating inline at the belief-store helper call. This keeps the side effect tied to the same direct-local batch while preserving the current read-then-commit shape of `perception_system`.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib perception::tests::perception_writes_capacity_observation_for_co_located_resource_source -- --exact`.
- Passed `cargo test -p worldwake-systems --lib perception::tests::perception_overwrites_capacity_observation_on_subsequent_tick -- --exact`.
- Passed `cargo test -p worldwake-systems --lib perception::tests::passive_observation_emits_discovery_for_resource_source_mismatch -- --exact`.
- Passed `cargo test -p worldwake-systems`.
- Passed `./scripts/verify.sh`, which ran `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
