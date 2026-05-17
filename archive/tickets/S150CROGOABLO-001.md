# S150CROGOABLO-001: CognitiveProfile per-scope blocker TTL fields

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `CognitiveProfile` (per-agent reasoning parameters in `worldwake-core`)
**Deps**: None

## Problem

Before this ticket, S150 needed per-scope blocker TTLs for `RouteSegment` and `Counterparty` blockers, but the per-agent `CognitiveProfile` had no fields to carry them. Without per-agent fields, the downstream substrate migration (ticket 002) would either hardcode constants (violating FND-2: no ungrounded triggers) or read TTLs from the wrong-purpose backoff fields (`route_unknown_backoff_ticks` and `counterparty_refusal_backoff_ticks` are per-discrepancy retry TTLs, not blocker generation-suppression TTLs). This ticket added the substrate fields ahead of the migration so 002 reads agent-attributable TTLs from a single profile surface.

## Assumption Reassessment (2026-05-17)

1. `CognitiveProfile` lives at `crates/worldwake-core/src/cognitive_profile.rs:23-121` with 36 existing fields. Every existing TTL field uses `#[serde(default = "default_<field>")]` (e.g., `route_unknown_backoff_ticks` at line 65, `counterparty_refusal_backoff_ticks` at line 62) with const-fn defaults sited alongside the field-helper block at lines 195-253. The Default impl at lines 123-163 enumerates every field explicitly — additions must update it in lockstep.
2. Spec source: `archive/specs/S150-cross-goal-blocker-scoping.md` D5 (defaults 240 and 360 with rationale captured in doc comments) and Profile-Driven Parameters section.
3. Shared abstraction boundary: `CognitiveProfile` is a universal-on-Agent component registered in `crates/worldwake-core/src/component_schema.rs:1008-1018`, bootstrap-seeded with `CognitiveProfile::default()` in `World::create_agent()` at `crates/worldwake-core/src/world.rs:202`, and authored via `AgentDef.cognitive_profile: Option<CognitiveProfile>` at `crates/worldwake-cli/src/scenario/types.rs:593`. The component contract requires the new fields to (a) have `Default` values, (b) carry `#[serde(default = "...")]` so existing authored scenarios continue to deserialize.
4. Existing tests in target module — `crates/worldwake-core/src/cognitive_profile.rs` `#[cfg(test)]` block: `cognitive_profile_default_matches_split_defaults` (line 278) must gain assertions for the two new fields; `cognitive_profile_roundtrips_through_bincode` (line 331) must gain values for the two new fields in its sample; `cognitive_profile_deserialization_defaults_discrepancy_ttls` (line 633) is the precedent for a new `cognitive_profile_deserialization_defaults_blocker_scope_ttls` test.
5. Test fixtures across the workspace that explicitly enumerate `CognitiveProfile` fields without `..CognitiveProfile::default()` spread were updated where required: `crates/worldwake-ai/src/failure_handling.rs::cognitive`, `crates/worldwake-ai/src/decision_runtime.rs::cognitive`, `crates/worldwake-ai/src/agent_tick/planning.rs::cognitive`, `crates/worldwake-ai/src/agent_tick/tests.rs::cognitive`, `crates/worldwake-ai/src/goal_model.rs::cognitive`, and `crates/worldwake-ai/src/search/tests.rs::cognitive`. The `conformance_execution_budget.rs` helpers use default spread and inherited the new fields unchanged.
6. Early compile fallout exposed one additional exhaustive sample literal in `crates/worldwake-core/src/delta.rs`; it was current-ticket shared-field fallout and now includes non-default blocker TTL samples.
7. `docs/profiles/all-profiles.md` is generated from profile doc comments by `scripts/profile_docs.py`; it now lists the two new fields.

## Architecture Check

1. Adding the fields ahead of the consuming migration (002) avoids the "wait — which TTL controls this scope?" question during the migration's recording-path edits. The fields exist; recording-path code reads them; no transient state where blockers exist but TTL provenance is hardcoded.
2. No backwards-compatibility shim: the existing `route_unknown_backoff_ticks` and `counterparty_refusal_backoff_ticks` fields stay (they continue to serve their per-discrepancy retry purpose); the new fields are net-new with distinct semantic intent (per-blocker generation-suppression TTL).

## Verified Layers

1. Default-value provenance — focused unit test (`cognitive_profile_deserialization_defaults_blocker_scope_ttls`) proving the const-fn defaults are picked up when the RON omits the new fields.
2. Default-impl consistency — focused unit test (extension of `cognitive_profile_default_matches_split_defaults`) proving the new fields are 240 and 360 in `Default::default()`.
3. Serialization roundtrip — focused unit test (extension of `cognitive_profile_roundtrips_through_bincode`) proving the new fields survive bincode round-trips with non-default values.
4. Single-layer ticket — the additional layer mapping (action trace, event-log delta) is not applicable because this ticket is a pure profile field addition with no runtime behavior change.

## Landed Changes

### 1. Added two TTL fields to `CognitiveProfile`

Following the existing pattern for per-discrepancy backoff TTLs (lines 46-72 of `cognitive_profile.rs`):

```rust
/// Ticks before a RouteSegment-scoped blocker expires under `TtlOnly` clearing.
#[serde(default = "default_route_segment_blocker_ticks")]
pub route_segment_blocker_ticks: u32,
/// Ticks before a Counterparty-scoped blocker expires under `TtlOnly` clearing.
#[serde(default = "default_counterparty_blocker_ticks")]
pub counterparty_blocker_ticks: u32,
```

### 2. Added const-fn defaults alongside the existing TTL helpers

```rust
const fn default_route_segment_blocker_ticks() -> u32 {
    // Mirrors default_route_unknown_backoff_ticks (200) with slight inflation because
    // blockers suppress goal generation entirely, not just retry. Route conditions change
    // through traversal evidence within a few hundred ticks.
    240
}

const fn default_counterparty_blocker_ticks() -> u32 {
    // Counterparty refusal is more durable than transient unwillingness — once a refusal
    // is observed, the agent should give the counterparty time to revise. The
    // per-discrepancy retry TTL (default_counterparty_refusal_backoff_ticks = 40) is the
    // shorter retry envelope; this is the longer "give them time" suppression envelope.
    360
}
```

### 3. Updated Default impl to include the new fields

Added `route_segment_blocker_ticks: default_route_segment_blocker_ticks()` and `counterparty_blocker_ticks: default_counterparty_blocker_ticks()` to the `impl Default for CognitiveProfile` block.

### 4. Extended existing tests

- `cognitive_profile_default_matches_split_defaults`: add `assert_eq!(profile.route_segment_blocker_ticks, 240); assert_eq!(profile.counterparty_blocker_ticks, 360);`.
- `cognitive_profile_roundtrips_through_bincode`: add non-default values for the two fields in the sample profile.
- Added test `cognitive_profile_deserialization_defaults_blocker_scope_ttls`: mirrors `cognitive_profile_deserialization_defaults_discrepancy_ttls` — serialize a profile with overridden values, strip the two new fields from the serialized RON, deserialize, assert the defaults applied.

### 5. Updated exhaustive `CognitiveProfile` fixtures

Each exhaustive fixture site identified in Assumption Reassessment item 5 now adds the two new fields explicitly using `CognitiveProfile::default()` values, matching the existing explicit-field-enumeration pattern in those AI test harnesses.

## Landed Files

- `crates/worldwake-core/src/cognitive_profile.rs` (modify) — field additions, const-fn defaults, Default impl update, existing-test updates, new defaults-test
- `crates/worldwake-ai/src/failure_handling.rs` (modify) — `cognitive` test fixture
- `crates/worldwake-ai/src/decision_runtime.rs` (modify) — `cognitive` test fixture
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify) — `cognitive` test fixture
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — `cognitive` test fixture
- `crates/worldwake-ai/src/goal_model.rs` (modify) — `cognitive` test fixture
- `crates/worldwake-ai/src/search/tests.rs` (modify) — `cognitive` test fixture
- `crates/worldwake-core/src/delta.rs` (modify) — exhaustive `ComponentValue::CognitiveProfile` sample literal
- `docs/profiles/all-profiles.md` (modify) — regenerated profile docs for the new fields

## Out of Scope

- Recording-path code that actually consumes the new TTLs — that lands in ticket 002 (substrate foundation).
- `BlockerScope` enum and `RouteSegment` newtype — defined in ticket 002.
- `BlockerClearingCondition` new variants — added in ticket 003.
- Authored scenario RON files — none currently set per-scope blocker TTLs explicitly; the `#[serde(default)]` annotation means existing RON continues to deserialize unchanged. No scenario file edits required.

## Acceptance Result

### Tests Passed

1. `cognitive_profile_default_matches_split_defaults` asserts the two new fields equal 240 and 360 in `CognitiveProfile::default()`.
2. `cognitive_profile_roundtrips_through_bincode` round-trips a profile with non-default values for the two new fields.
3. `cognitive_profile_deserialization_defaults_blocker_scope_ttls` proves that omitting the two new fields from serialized RON deserializes to the const-fn defaults.
4. Existing suite: `cargo test -p worldwake-core --lib cognitive_profile` passed.
5. Workspace clippy lint: `cargo clippy --workspace --all-targets -- -D warnings` passed after doc-markdown cleanup.

### Invariants

1. `CognitiveProfile` continues to satisfy `Copy + Eq + Ord + Hash` derive bounds (`u32` field additions preserve these).
2. Authored `scenarios/*.ron` continues to deserialize because both new fields carry `#[serde(default = "...")]`.
3. `World::create_agent()` continues to bootstrap-seed `CognitiveProfile::default()`; no edit at `world.rs` was required because the Default impl carries the new fields automatically.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs` — added `cognitive_profile_deserialization_defaults_blocker_scope_ttls`; extended `cognitive_profile_default_matches_split_defaults` and `cognitive_profile_roundtrips_through_bincode` for the two field assertions.

### Verification Commands

1. `cargo test -p worldwake-core --lib cognitive_profile`
2. `cargo test -p worldwake-ai --lib` — confirms the 6 fixture sites compile with the field additions.
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh` for the full pre-PR gate.

## Outcome

Completed on 2026-05-17.

- Added `route_segment_blocker_ticks` and `counterparty_blocker_ticks` to `CognitiveProfile` with serde defaults, `Default` entries, and const-fn defaults of 240 and 360 ticks.
- Extended focused `CognitiveProfile` tests for default values, omitted-field RON defaulting, and non-default bincode roundtrip.
- Updated exhaustive AI test fixtures and the core `ComponentValue::CognitiveProfile` sample literal so compile and lint surfaces use the new fields explicitly where required.
- Regenerated `docs/profiles/all-profiles.md` with the new profile fields. The generator still reports 15 pre-existing documentation-gap warnings outside this ticket's fields.

## Deviations

- `crates/worldwake-ai/tests/conformance_execution_budget.rs` was listed during drafting, but its `CognitiveProfile` helpers already use `..CognitiveProfile::default()` and needed no edit.
- Early compile fallout added `crates/worldwake-core/src/delta.rs` and generated profile docs to the landed surface; both are shared-profile fallout for the same substrate change.
- The first clippy run failed on missing Markdown backticks around `TtlOnly` in the new doc comments; the comments and generated profile docs were corrected before rerunning clippy and `./scripts/verify.sh`.

## Verification Result

- Passed `cargo test --workspace --no-run`.
- Passed `cargo test -p worldwake-core --lib cognitive_profile`.
- Passed `cargo test -p worldwake-ai --lib`.
- Passed `python3 scripts/profile_docs.py --write` with the expected 15 pre-existing documentation-gap warnings outside this ticket's fields.
- Passed `cargo fmt --all`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings` after doc-comment cleanup.
- Passed `./scripts/verify.sh`.
