# S131SOURELWAI-001: ReliabilityRecord and PreferenceProfile field extensions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` (ECS components extended; new helper methods; serde defaults)
**Deps**: None

## Problem

Today `ReliabilityRecord { successful_acquisitions, failed_attempts, last_attempt_tick }` captures only success/failure ratio per (entity, commodity). The narrative report shows agents repeatedly contending for the same source with no learning surface for *how long they waited* or *how much the source held* — two operationally critical signals. This ticket extends the in-place type with concrete observation fields and adds the `wait_sensitivity_weight` profile parameter agents will use to weigh those signals. Subsequent tickets (002–004) hook the actual write/read paths.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `pub struct ReliabilityRecord` is at `crates/worldwake-core/src/experience.rs:77` (3 fields, derives `Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize`). `pub struct PreferenceProfile` is at `experience.rs:125` (5 fields, same derive set). `pub struct SourceReliability` at `experience.rs:84` is the host component (universal per-agent, registered in `crates/worldwake-core/src/component_schema.rs:358–380`). `failure_ratio_permille(record: &ReliabilityRecord) -> u32` is a free function at `experience.rs:61`. Existing inline tests in `experience.rs` `#[cfg(test)]` block (line 152+): tests at lines 180, 190, 209, 214, 219, 236, 247, 270, 288, 336, 356, 369, 385, 397, 425 cover `ReliabilityRecord`, `PreferenceProfile`, and `enforce_limits`. Construction-site count: 30 `ReliabilityRecord {` literal sites and 9 `PreferenceProfile {` literal sites across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai` — no `..Default::default()` spread syntax in any current site, so all sites must be updated explicitly.
2. `docs/spec-drafting-rules.md` Section 5 (universal profile pattern) requires a `Default` impl on `PreferenceProfile` and seeding via `World::create_agent` (`crates/worldwake-core/src/world.rs:225`). The seeding line already calls `PreferenceProfile::default()`, so adding a new field with a value in the existing `Default` impl is sufficient — no extra bootstrap edit. No `world_txn.rs` delta assertion is associated with `create_agent` in this codebase (grep returned empty).
3. Cross-crate boundary under audit: `ReliabilityRecord` and `PreferenceProfile` are public types in `worldwake-core` consumed by `worldwake-sim` (belief view and save/load), `worldwake-systems` (experience recording, trade, production), `worldwake-ai` (ranking, decision trace, agent_tick), and authored scenario profiles. Field additions ripple through those surfaces. Reassessment corrected the draft's false old-save premise: bincode encodes structs positionally, and the repo rejects non-current save versions. This ticket therefore bumps `SAVE_FORMAT_VERSION` from 62 to 63 and proves current-format roundtrip of non-default new fields rather than supporting old saves.
4. The runtime construction sites that should migrate to `ReliabilityRecord::new(tick)` are `crates/worldwake-systems/src/experience_recording.rs:15` (the `.or_insert(ReliabilityRecord {...})` in agent learning) and `crates/worldwake-ai/src/ranking.rs:557` (the default record in `apply_source_reliability_discount_with_pending_failures`). Test fixtures in `experience.rs:238–262`, `test_utils.rs:146`, `trade_actions.rs` test sites (lines 3044/3115/3255), `production_actions.rs` test sites (1645/1693/1757), `agent_tick/mod.rs:2114`, `agent_tick/tests.rs` (6333/6448-6459), and `ranking.rs:3846-3847` may keep struct-literal form provided they include the new fields.

## Architecture Check

1. Field extension on existing components is the minimum FND-30-compliant surface. `ReliabilityRecord` already declares "(entity, commodity) reliability memory"; the new fields extend that contract with concrete observable quantities (wait ticks, observed capacity) rather than introducing a new component or abstract score (FND-3). `PreferenceProfile` already declares "per-agent decision weights"; adding `wait_sensitivity_weight` follows the existing `source_trust_weight` / `route_caution_weight` pattern.
2. No backwards-compatibility shim. New fields land directly on the live types; old call sites that only read `failure_ratio_permille` continue to work because `failure_ratio_permille` reads only the original three fields. Current save format advances to 63; saves with older format versions remain rejected by `load_from_bytes`.

## Verification Layers

1. Field addition correctness — focused unit tests in `experience.rs` `#[cfg(test)]` block exercising `observe_wait` running-mean math, EMA transition at the 32nd observation, and `observe_capacity` overwrite-and-tick-update.
2. `Default` impl correctness — focused unit test asserting `PreferenceProfile::default().wait_sensitivity_weight == Permille::new_unchecked(150)` and `ReliabilityRecord::default()` zeroes all observation fields with `last_attempt_tick = Tick(0)`.
3. Serialization/current save correctness — focused unit test that round-trips a `SourceReliability` containing non-default wait/capacity fields through `bincode`, plus a `worldwake-sim` save/load test proving current-format saves preserve the new persisted fields and write `SAVE_FORMAT_VERSION == 63`.
4. Single-layer ticket: this ticket changes type definitions and helper methods only; no decision-trace, action-trace, or event-log delta is exercised here. Ticket 002–004 each map their own behavioral invariants to those layers.

## What to Change

### 1. Extend `ReliabilityRecord` and add helpers

In `crates/worldwake-core/src/experience.rs`, after the existing `pub struct ReliabilityRecord {...}` declaration:

- Add four fields: `average_wait_ticks: u32`, `wait_observation_count: u32`, `last_observed_capacity: u16`, `last_observed_capacity_tick: Tick`.
- Add doc comments per the spec (D1) so the "never observed" vs "observed empty" semantic for `last_observed_capacity == 0` is discoverable.
- Add `impl Default for ReliabilityRecord` returning all-zero observation fields and `last_attempt_tick: Tick(0)`.
- Add `impl ReliabilityRecord` block with three associated functions:
  - `pub fn new(last_attempt_tick: Tick) -> Self` — fresh record with `last_attempt_tick` set, all observation fields zero.
  - `pub fn observe_wait(&mut self, wait_ticks: u32)` — running mean while `wait_observation_count < 32`; switches to EMA with α = 1/32 afterwards. Use `saturating_mul` / `saturating_add` per the spec pseudocode.
  - `pub fn observe_capacity(&mut self, capacity: u16, tick: Tick)` — overwrites `last_observed_capacity` and `last_observed_capacity_tick`.

### 2. Extend `PreferenceProfile` and update Default

In the same file:

- Add `pub wait_sensitivity_weight: Permille` to the struct. Add a doc comment per the spec.
- Update `impl Default for PreferenceProfile` (currently at `experience.rs:138-148`) to include `wait_sensitivity_weight: Permille::new_unchecked(150)`.

### 3. Migrate runtime construction sites

- `crates/worldwake-systems/src/experience_recording.rs:15` — replace the `.or_insert(ReliabilityRecord { successful_acquisitions: 0, failed_attempts: 0, last_attempt_tick: current_tick })` with `.or_insert_with(|| ReliabilityRecord::new(current_tick))`.
- `crates/worldwake-ai/src/ranking.rs:557` — replace the `.unwrap_or(ReliabilityRecord { successful_acquisitions: 0, failed_attempts: 0, last_attempt_tick: context.current_tick })` with `.unwrap_or_else(|| ReliabilityRecord::new(context.current_tick))`.

### 4. Update test-fixture construction sites

For every remaining `ReliabilityRecord { ... }` and `PreferenceProfile { ... }` struct literal across the workspace, either add the new fields explicitly or use struct-update syntax from the appropriate default/constructor when the test is only exercising pre-existing fields. Test fixtures testing pre-existing behavior should default the new fields to zero (which matches the "never observed" semantic). Authored scenario `preference_profile` blocks are full RON struct literals and must include `wait_sensitivity_weight: 150`.

- `crates/worldwake-core/src/experience.rs` (test fixtures around lines 238–262)
- `crates/worldwake-core/src/test_utils.rs:146`
- `crates/worldwake-systems/src/experience_recording.rs` (any test below the runtime site)
- `crates/worldwake-systems/src/trade_actions.rs:3044, 3115, 3255`
- `crates/worldwake-systems/src/production_actions.rs:1645, 1693, 1757`
- `crates/worldwake-ai/src/ranking.rs:3846-3847` (helper) plus any test-only fixtures
- `crates/worldwake-ai/src/agent_tick/mod.rs:2114`
- `crates/worldwake-ai/src/agent_tick/tests.rs:6333, 6448-6459`
- Any `PreferenceProfile { ... }` literal in the same files (grep `PreferenceProfile {` to enumerate; 9 sites total).

### 5. Add focused tests

Append to `experience.rs` `#[cfg(test)]` block (after line 425):

- `observe_wait_running_mean_until_cap`: feed 5 observations `(0, 3, 5, 8, 12)` to a fresh record; assert `average_wait_ticks == 4` and `wait_observation_count == 5`. The stored integer recurrence has no separate accumulated total, so this is the deterministic integer running estimate produced by the documented formula, not the exact arithmetic mean.
- `observe_wait_switches_to_ema_after_32`: feed 32 identical observations of `4`, then a 33rd observation of `100`; assert the running mean before the 33rd is `4` and after is `(31 × 4 + 100) / 32 == 7`.
- `observe_capacity_overwrites_value_and_tick`: write capacity 18 at Tick(100), then capacity 5 at Tick(200); assert both fields reflect the second write.
- `reliability_record_default_zeroes_observation_fields`: assert `Default` produces all-zero observation fields and `last_attempt_tick == Tick(0)`.
- `preference_profile_default_includes_wait_sensitivity_baseline`: assert the new field is `Permille::new_unchecked(150)`.
- `reliability_record_round_trips_observation_fields_through_bincode`: serialize a `SourceReliability` containing non-default new fields, deserialize back into the new struct, and assert equality. Use `bincode` to match the project's component payload encoding.
- `failure_ratio_permille_ignores_observation_fields`: assert the existing pure projection remains unchanged when only wait/capacity fields differ.
- `save_to_bytes_roundtrip_preserves_full_nondefault_state` in `worldwake-sim`: extend the existing current-format save/load test to assert non-default wait/capacity fields and `wait_sensitivity_weight` survive `save_to_bytes` / `load_from_bytes`.

## Files to Touch

- `crates/worldwake-core/src/experience.rs` (modify) — struct field additions, Default impls, new helper methods, new tests.
- `crates/worldwake-core/src/test_utils.rs` (modify) — fixture update.
- `crates/worldwake-systems/src/experience_recording.rs` (modify) — runtime site migration.
- `crates/worldwake-systems/src/trade_actions.rs` (modify) — fixture updates.
- `crates/worldwake-systems/src/production_actions.rs` (modify) — fixture updates.
- `crates/worldwake-ai/src/ranking.rs` (modify) — runtime site migration + fixture updates.
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify) — fixture update.
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — fixture updates.
- `crates/worldwake-sim/src/save_load.rs` (modify) — save format bump to 63 and current-format roundtrip assertions for the new fields.
- `scenarios/survival-preferences.ron`, `scenarios/cli-evaluation.ron`, `scenarios/final-integration.ron` (modify) — authored full `PreferenceProfile` literals gain `wait_sensitivity_weight: 150`.

## Out of Scope

- Wait observation hooks at grant promotion sites — covered by ticket 002.
- Capacity observation hook in perception — covered by ticket 003.
- Composite ranking integration and `SourceReliabilityDiscount` extension — covered by ticket 004.
- Old-save migration or compatibility — intentionally not supported. `SAVE_FORMAT_VERSION` advances to 63 and older versions remain rejected by the live loader.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core --lib experience::tests::observe_wait_running_mean_until_cap -- --exact`
2. `cargo test -p worldwake-core --lib experience::tests::observe_wait_switches_to_ema_after_32 -- --exact`
3. `cargo test -p worldwake-core --lib experience::tests::observe_capacity_overwrites_value_and_tick -- --exact`
4. `cargo test -p worldwake-core --lib experience::tests::reliability_record_default_zeroes_observation_fields -- --exact`
5. `cargo test -p worldwake-core --lib experience::tests::preference_profile_default_includes_wait_sensitivity_baseline -- --exact`
6. `cargo test -p worldwake-core --lib experience::tests::reliability_record_round_trips_observation_fields_through_bincode -- --exact`
7. `cargo test -p worldwake-core --lib experience::tests::failure_ratio_permille_ignores_observation_fields -- --exact`
8. `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`
9. Existing suite: `cargo test --workspace` — all pre-existing tests continue to pass; fixture updates must not change the behavior any existing test asserts.

### Invariants

1. `ReliabilityRecord` remains `Copy` after the field additions (all new fields are `u32` / `u16` / `Tick`, all `Copy`).
2. `PreferenceProfile::default().wait_sensitivity_weight == Permille::new_unchecked(150)` so universal-profile seeding via `World::create_agent` propagates the documented baseline to every spawned agent.
3. Current-format saves preserve the new persisted fields; `SAVE_FORMAT_VERSION` is 63 and old save versions remain rejected.
4. `failure_ratio_permille(record)` returns the same value for any record whose original three fields are unchanged, regardless of the new field values (the function reads only the original fields).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/experience.rs` — append focused tests in the existing `#[cfg(test)]` block per Section 5 of What to Change.
2. `crates/worldwake-sim/src/save_load.rs` — extend current-format save/load coverage for the new persisted fields and version 63.
3. Pre-existing tests in `experience.rs`, `experience_recording.rs`, `trade_actions.rs`, `production_actions.rs`, `ranking.rs`, `agent_tick/mod.rs`, `agent_tick/tests.rs`, `test_utils.rs` — fixture updates only (add new fields or struct-update defaults). No behavioral assertion changes.

### Commands

1. `cargo test -p worldwake-core --lib experience::tests` — narrowest core verification while iterating.
2. `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact` — current-format persisted-shape proof.
3. `cargo test --workspace --no-run` — early shared-shape compile fallout sweep.
4. `cargo test --workspace` — confirms cross-crate fixture updates compile and pass.
5. `cargo clippy --workspace --all-targets -- -D warnings` — lint gate per AGENTS.md.
6. `scripts/verify.sh` — full pre-PR gate (fmt + tests + clippy).

## Outcome

Completed on 2026-05-03.

- Extended `ReliabilityRecord` with wait/capacity observation fields, `Default`, `new`, `observe_wait`, and `observe_capacity`.
- Added `PreferenceProfile.wait_sensitivity_weight` with default baseline `Permille::new_unchecked(150)`.
- Migrated runtime fallback construction sites to `ReliabilityRecord::new` and updated explicit `ReliabilityRecord` / `PreferenceProfile` fixtures across core, systems, AI, travel, and golden test surfaces, plus authored `preference_profile` scenario blocks.
- Bumped `SAVE_FORMAT_VERSION` from 62 to 63 and extended current-format save/load proof for non-default new fields.

## Deviations

- Reassessment rejected the draft old-save compatibility claim. Bincode does not provide named-field omitted-field defaulting for old positional payloads, and the live loader rejects non-current save versions. The landed contract is current-format version 63 only.
- The drafted exact mean expectation for `(0, 3, 5, 8, 12)` was corrected to the documented integer recurrence. With only `average_wait_ticks` and `wait_observation_count` stored, the deterministic value is `4`, not the exact arithmetic mean `5`.

## Verification Result

- Passed `cargo test -p worldwake-core --lib experience::tests -- --list`.
- Passed `cargo test -p worldwake-core --lib experience::tests`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests -- --list`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`.
- Passed `cargo test --workspace --no-run`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh` (`cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`).
