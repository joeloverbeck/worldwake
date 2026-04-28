# S128SLEEPIPLA-002: MetabolismProfile.min_sleep_ticks field

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `min_sleep_ticks: NonZeroU32` field on the universal `MetabolismProfile` agent component, with cascading updates to 24 explicit construction sites across 14 files.
**Deps**: specs/S128-sleep-episode-place-quality.md (D6)

## Problem

S128SLEEPIPLA-004 (sleep handler refactor) needs a per-agent minimum sleep duration so the sleep episode can derive `intended_min_ticks` from agent profile rather than from a magic constant. Today, `MetabolismProfile` (`crates/worldwake-core/src/needs.rs:142`) carries `rest_efficiency` (per-tick recovery rate) and `travel_fatigue_multiplier` but no minimum-sleep-duration field. Adding the field is structurally independent of the new types in S128SLEEPIPLA-001 — it is a localized field addition with a wide construction-site blast radius.

## Assumption Reassessment (2026-04-27)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `MetabolismProfile` (`crates/worldwake-core/src/needs.rs:142-175`) currently has 17 fields, all `Permille` or `NonZeroU32`. `Default` impl at lines 233-254 sets `rest_efficiency: pm(20)` and the duration-based fields use `nz(8)` / `nz(12)` patterns. Constructor at `MetabolismProfile::new` (around line 192 — `pub const fn new`) takes all fields as positional arguments.
2. Construction sites across the workspace (verified via `rg -c "MetabolismProfile\s*\{" crates/`): 24 sites in 14 files. Only 2 sites use `..MetabolismProfile::default()` spread syntax (`crates/worldwake-ai/tests/golden_harness/soak_world.rs:273`, plus one other). The remaining 22 sites enumerate fields explicitly OR call `MetabolismProfile::new(...)` with positional arguments. All 22 must be updated. `MetabolismProfile::new` constructor signature also needs a new positional parameter; every `::new(...)` call site must add the new argument. `crates/worldwake-systems/tests/e09_needs_integration.rs:533` destructures `MetabolismProfile { ... }` — destructuring sites also need the new field name added (or `..` rest pattern, but check current style).
3. Shared boundary under audit: the `MetabolismProfile` struct contract. The new `min_sleep_ticks: NonZeroU32` field uses `#[serde(default = "default_min_sleep_ticks")]` to preserve save/load compatibility without bumping `SAVE_FORMAT_VERSION` again (S128SLEEPIPLA-001 bumps to 54; this ticket adds a serde-defaulted field).
4. `World::create_agent()` (`crates/worldwake-core/src/world.rs:181`) seeds defaults for universal profiles via `MetabolismProfile::default()`. With the new field added to `Default`, `create_agent` automatically picks up the default — no bootstrap-path code change needed beyond confirming the existing test `create_agent_components_queryable` (line 1285) still passes. The `world_txn.rs::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` test at line 2378 also continues to pass (delta assertion is on component identity, not field count).
5. `AgentDef.metabolism_profile: Option<MetabolismProfile>` (`crates/worldwake-cli/src/scenario/types.rs:344`) exposes the full struct in scenario authoring; existing scenarios using `metabolism_profile: None` continue to work because `unwrap_or_default()` (`mod.rs:576`) supplies the new field at default.
6. Existing focused/unit tests exercising `MetabolismProfile`: `e09_needs_integration.rs::metabolism_profile` helper (line 219) and the destructuring test at line 533 — both need the new field added to compile. `worldwake-systems/src/needs.rs:617` returns a `MetabolismProfile` from a test helper — needs update. No tests assert on `min_sleep_ticks` directly today (the field doesn't exist yet); behavioral coverage lands in S128SLEEPIPLA-004 and -007.
7. Construction-site blast radius is wide (22 explicit sites) but the work is mechanical: add `min_sleep_ticks: nz(8)` (or appropriate per-test value) to each site. Per `references/spec-writing-rules.md` Step 2 sub-check (d), construction count above 20 elevates to Medium — load-bearing because most sites lack spread syntax.

## Architecture Check

1. The field addition follows the existing `MetabolismProfile` pattern: per-need timing (`toilet_ticks: NonZeroU32`, `wash_ticks: NonZeroU32`) at lines 162-163 establishes the precedent for per-action duration parameters on the metabolism profile. `min_sleep_ticks` extends this naturally.
2. Universal default (`NonZeroU32::new(8).unwrap()`) keeps existing scenarios working without authoring effort. Per-agent variation comes through scenario `MetabolismProfile.min_sleep_ticks: 12` (or similar) once authors want it.
3. `#[serde(default)]` avoids a SAVE_FORMAT_VERSION bump beyond the one S128SLEEPIPLA-001 already issues — older saves missing the field deserialize with the default `8` ticks. This is a legitimate save-boundary use of serde defaulting, not a backward-compatibility shim in the live authority path (FND-28 distinguishes save/load boundary from runtime authority).

## Verification Layers

1. `MetabolismProfile.min_sleep_ticks` reads from authoritative agent state → focused unit test in `crates/worldwake-core/src/needs.rs` test module asserting `MetabolismProfile::default().min_sleep_ticks == NonZeroU32::new(8).unwrap()`.
2. Save/load round-trip preserves the new field → existing `crates/worldwake-sim/src/save_load.rs` test sweep (extended in S128SLEEPIPLA-001 already has a `MetabolismProfile`-bearing fixture; verify it round-trips with the new field present).
3. Scenario load with `metabolism_profile: None` produces an agent with `min_sleep_ticks == 8` → existing `crates/worldwake-cli/src/scenario/mod.rs` test for `metabolism_profile: None` path (around line 576's tests) — extend assertion to check the new field.
4. Single-layer ticket: this is a struct-extension ticket with no behavioral changes (no action handler logic uses the field yet — that lands in S128SLEEPIPLA-004). Verification stays at focused unit + serialization round-trip surfaces.

## What to Change

### 1. Add field to `MetabolismProfile`

In `crates/worldwake-core/src/needs.rs`:

- Line 142-175 struct definition: add `pub min_sleep_ticks: NonZeroU32` (after the existing duration fields `toilet_ticks`, `wash_ticks` — preserves grouping).
- Annotate the field with `#[serde(default = "default_min_sleep_ticks")]`. Add the helper `fn default_min_sleep_ticks() -> NonZeroU32 { NonZeroU32::new(8).unwrap() }` at module scope.
- Line 233-254 `Default` impl: add `min_sleep_ticks: NonZeroU32::new(8).unwrap()`.
- `MetabolismProfile::new` constructor (around line 192, `pub const fn new`): add `min_sleep_ticks: NonZeroU32` as a new trailing parameter. Update the `#[allow(clippy::too_many_arguments)]` annotation if needed.

### 2. Update construction sites

Add `min_sleep_ticks: <value>` to every explicit struct-literal construction. Choose values per site context:

- Production code (`world.rs:922 sample_metabolism_profile`, `needs.rs:617`): use `NonZeroU32::new(8).unwrap()`.
- Test fixtures: use `NonZeroU32::new(8).unwrap()` unless the test explicitly varies sleep timing (none do today).

Per-file sites confirmed via grep:

- `crates/worldwake-core/src/world.rs:922` (`sample_metabolism_profile`)
- `crates/worldwake-core/src/needs.rs:233-254` (`Default` impl — already covered above)
- `crates/worldwake-systems/src/needs.rs:617`
- `crates/worldwake-systems/tests/e09_needs_integration.rs:219` (helper) and `:533` (destructuring — add `min_sleep_ticks` to the destructuring pattern, or use `..` rest pattern)
- `crates/worldwake-ai/src/candidate_generation.rs:19280`
- `crates/worldwake-ai/src/agent_tick/frame.rs:1262, 1454`
- `crates/worldwake-ai/tests/golden_perception_exposure.rs:88`
- `crates/worldwake-ai/tests/golden_planner_pathology.rs:313`
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs:676-677`
- `crates/worldwake-ai/tests/golden_item_decay.rs:14`
- `crates/worldwake-ai/tests/golden_activation_decay.rs:12`
- `crates/worldwake-ai/tests/golden_exploration.rs:353-354`
- `crates/worldwake-ai/tests/golden_harness/soak_world.rs:143-144, 273` (273 uses spread syntax — no change needed)
- `crates/worldwake-ai/tests/golden_survival_drive_escalation.rs:249, 390`

For every `MetabolismProfile::new(...)` call site, add the new positional argument at the end. Grep for `MetabolismProfile::new(` to find these (`crates/worldwake-systems/tests/e09_needs_integration.rs` is the primary user).

### 3. AgentDef wiring is automatic

`AgentDef.metabolism_profile: Option<MetabolismProfile>` already exposes the full struct (`crates/worldwake-cli/src/scenario/types.rs:344`); scenario authors can set the new field directly. No `AgentDef` change needed.

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (modify — add field, default, constructor parameter, default helper fn)
- `crates/worldwake-core/src/world.rs` (modify — `sample_metabolism_profile` fixture)
- `crates/worldwake-systems/src/needs.rs` (modify — test helper)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify — helper + destructure)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — `metabolism_with_rates` helper)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — `metabolism_with_only` and one inline construction)
- `crates/worldwake-ai/tests/golden_perception_exposure.rs` (modify)
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (modify — `starvation_traceability_metabolism`)
- `crates/worldwake-ai/tests/golden_item_decay.rs` (modify)
- `crates/worldwake-ai/tests/golden_activation_decay.rs` (modify)
- `crates/worldwake-ai/tests/golden_exploration.rs` (modify — `calm_metabolism_profile`)
- `crates/worldwake-ai/tests/golden_harness/soak_world.rs` (modify — `t30_default_metabolism` and one explicit construction; the `..MetabolismProfile::default()` site needs no change)
- `crates/worldwake-ai/tests/golden_survival_drive_escalation.rs` (modify — two construction sites)

## Out of Scope

- Reading `min_sleep_ticks` from `tick_sleep` — handled by S128SLEEPIPLA-004
- Behavioral assertions about minimum sleep duration — S128SLEEPIPLA-004 introduces the consumer; S128SLEEPIPLA-007 adds golden coverage
- Per-agent scenario authoring of `min_sleep_ticks` overrides — out of scope (already exposed via `MetabolismProfile`); follow-up scenarios may use it once handler logic exists
- A second `SAVE_FORMAT_VERSION` bump — explicitly avoided via `#[serde(default)]` per Architecture Check item 3

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core needs` — focused unit test asserts `MetabolismProfile::default().min_sleep_ticks == NonZeroU32::new(8).unwrap()`.
2. `cargo test -p worldwake-sim save_load` — existing round-trip test passes; serialized state with the new field deserializes correctly. A regression test reading a save written without the field (using the `default_min_sleep_ticks` helper path) deserializes to `8`.
3. `cargo test -p worldwake-systems e09_needs_integration` — existing tests pass with the constructor and destructuring updates.
4. `cargo test -p worldwake-ai` — all golden tests pass (the field default doesn't affect any current test's sleep behavior; sleep semantics still per-tick re-commit until S128SLEEPIPLA-004 lands).
5. Existing suite: `cargo test --workspace`.

### Invariants

1. `MetabolismProfile::default().min_sleep_ticks == NonZeroU32::new(8).unwrap()`.
2. `SAVE_FORMAT_VERSION` is unchanged from 54 (set by S128SLEEPIPLA-001) — this ticket does NOT bump again.
3. All explicit `MetabolismProfile { ... }` construction sites are updated; no compile errors remain.
4. `MetabolismProfile::new(...)` accepts the new positional argument at every call site.
5. Scenarios using `metabolism_profile: None` produce agents with `min_sleep_ticks == 8` (via the `Default` path).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/needs.rs` (modify — add a focused unit test in the existing `#[cfg(test)]` module asserting the default value of `min_sleep_ticks`).
2. `crates/worldwake-sim/src/save_load.rs` (modify — extend the existing round-trip fixture from S128SLEEPIPLA-001 to also assert the new field's default deserialization for a save produced by the function under test; if the existing test already covers component round-trips, add only an explicit field-level assertion).
3. `crates/worldwake-cli/src/scenario/mod.rs` (modify — extend the existing `metabolism_profile: None` scenario test, or add a focused unit test, asserting the spawned agent's `min_sleep_ticks` equals `8`).

### Commands

1. `cargo test -p worldwake-core needs`
2. `cargo test -p worldwake-systems e09_needs_integration`
3. `cargo test -p worldwake-ai` (must remain green — sleep behavior unchanged at this ticket)
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`
