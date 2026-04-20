# S110DECHISEVE-003: CognitiveProfile decision_history_alternatives field

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `CognitiveProfile` component gains one new field; serde default handles save/scenario round-trip
**Deps**: None

## Problem

S110's `GoalCommittedPayload::rejected_alternatives` list must be bounded to keep event size predictable. The bound is per-agent configurable via `CognitiveProfile::decision_history_alternatives: u8` (default 5). This ticket adds the field with a serde default so existing saved state and scenario RON continue to deserialize without modification. Enforcement of the cap at emission time lands in ticket 004.

## Assumption Reassessment (2026-04-20)

1. `CognitiveProfile` is defined at `crates/worldwake-core/src/cognitive_profile.rs:6` and derives `Copy + Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Serialize + Deserialize`. Every existing field with a serde-default uses `#[serde(default = "default_<field>")]` with a dedicated `const fn` constructor — the ticket follows that established convention. Existing tests `cognitive_profile_roundtrips_through_bincode` and `cognitive_profile_deserialization_defaults_*` (at `cognitive_profile.rs:259` onward) verify that missing fields in serialized input deserialize to the default; the new field must be covered by an analogous test.
2. Existing consumers that construct `CognitiveProfile` with explicit literals (not `..CognitiveProfile::default()` spread): ~20 sites across `crates/worldwake-cli/src/scenario/lints.rs`, `crates/worldwake-cli/src/handlers/persistence.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, `crates/worldwake-ai/src/agent_tick/planning.rs` (test fixtures), `crates/worldwake-ai/src/agent_tick/tests.rs`, `crates/worldwake-ai/src/goal_model.rs` (test fixtures), `crates/worldwake-ai/src/search/tests.rs`, `crates/worldwake-ai/tests/golden_exploration.rs`, `crates/worldwake-ai/tests/golden_ai_decisions.rs`, and `crates/worldwake-ai/tests/conformance_execution_budget.rs`. Each needs `decision_history_alternatives: 5` (or the same default value) added. Workspace also has 148 `..CognitiveProfile::default()` spread uses which need no change because the new field inherits its default.
3. Shared abstraction boundary under audit: the `CognitiveProfile` component wire format. Adding a field with `#[serde(default)]` preserves forward-compat deserialization from save-format 33 data — RON scenarios and saved state that predate this field still load correctly. The field is a single `u8`, so wire-size growth is one byte per agent.

## Architecture Check

1. Putting the cap on `CognitiveProfile` rather than a global constant preserves FND-22 (agent diversity through concrete variation) — different agent types can carry different rejection-history depths based on their reasoning profile. This also keeps the parameter out of ticket 002's payload types, where it would be miscategorized as a schema concern rather than a behavior concern.
2. The `#[serde(default = "default_decision_history_alternatives")]` attribute preserves round-trip compatibility without an explicit migration path. This is the established pattern in the file (11 other fields use the same idiom), so no new abstraction is introduced. FND-28 still applies to the overall save format — this ticket coexists with ticket 002's `SAVE_FORMAT_VERSION` bump, which renders old saves undecodable regardless.

## Verification Layers

1. Serde default correctness → new unit test `cognitive_profile_deserialization_defaults_decision_history_alternatives` extending the existing pattern in `cognitive_profile.rs`.
2. Round-trip invariance → existing `cognitive_profile_roundtrips_through_bincode` extended to cover the new field.
6. Single-layer ticket — no runtime logic uses the new field until ticket 004 wires truncation at emission. Additional layer mapping is not applicable until then.

## What to Change

### 1. Add the field and its default constructor

In `crates/worldwake-core/src/cognitive_profile.rs`:

Add to the struct definition (at the end of the field list, following the existing `use_ff_heuristic` pattern):

```rust
#[serde(default = "default_decision_history_alternatives")]
pub decision_history_alternatives: u8,
```

Add the default function alongside the existing `default_*` const fns:

```rust
const fn default_decision_history_alternatives() -> u8 {
    5
}
```

Add the field to the `Default for CognitiveProfile` impl at `cognitive_profile.rs:79` with value `default_decision_history_alternatives()`.

### 2. Update explicit-literal construction sites

Grep the workspace for `CognitiveProfile {` and update every site that enumerates all fields (does not use `..CognitiveProfile::default()`). Each site adds `decision_history_alternatives: 5` (matching the default). Known sites from the reassessment:

- `crates/worldwake-cli/src/scenario/lints.rs:262`
- `crates/worldwake-cli/src/handlers/persistence.rs:183`
- `crates/worldwake-sim/src/per_agent_belief_view.rs:2983`
- `crates/worldwake-ai/src/search/tests.rs:49`
- `crates/worldwake-ai/src/goal_model.rs:2587`
- `crates/worldwake-ai/src/agent_tick/tests.rs:97`
- `crates/worldwake-ai/src/agent_tick/planning.rs:1385`
- `crates/worldwake-ai/tests/golden_exploration.rs:1008` and `:1435`
- `crates/worldwake-ai/tests/golden_ai_decisions.rs:1190`
- `crates/worldwake-ai/tests/conformance_execution_budget.rs:21`, `:29`, `:36`

Implementer must grep-verify the full list — any site missed produces a compile error (the field has no default-on-struct since literals enumerate fields explicitly).

### 3. Add default-deserialization test

In `crates/worldwake-core/src/cognitive_profile.rs` `#[cfg(test)]` block, add `cognitive_profile_deserialization_defaults_decision_history_alternatives` following the pattern of `cognitive_profile_deserialization_defaults_use_ff_heuristic` at line 259 — serialize a profile with a non-default value, strip the field line from the serialized RON, deserialize, and assert the field equals `default_decision_history_alternatives()`.

### 4. Extend the existing round-trip test

In `cognitive_profile_roundtrips_through_bincode` (line 222), add `decision_history_alternatives: 8` (or any non-default `u8`) to the constructed profile so the round-trip actually exercises a non-default value.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify — field, `Default` impl, default fn, two tests)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — literal construction site)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify — literal construction site)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — literal construction site, test fixture)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify — test fixture)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — test fixture)
- `crates/worldwake-ai/tests/golden_exploration.rs` (modify — two construction sites)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (modify — three construction sites)

Other sites may exist — implementer must grep `CognitiveProfile {` and cover every site that does not use `..CognitiveProfile::default()`.

## Out of Scope

- Runtime use of the new field. Ticket 004 applies the cap at `GoalCommittedPayload` emission time; this ticket stores the value and makes it configurable.
- Scenario RON file updates. Existing scenarios that specify `cognitive_profile: (…)` blocks rely on the serde default — no RON file edits needed. If a scenario author wants to override the default per agent, they can add the field after this ticket lands; no pre-existing scenarios require changes.
- Tuning the default value. `5` is the spec's default; any later tuning is a separate concern.
- Any change to `CognitiveProfile`'s derive set. `Copy` is preserved because `u8` is `Copy`.

## Acceptance Criteria

### Tests That Must Pass

1. `cognitive_profile_default_matches_split_defaults` asserts `decision_history_alternatives == 5`.
2. `cognitive_profile_roundtrips_through_bincode` round-trips a non-default value.
3. New test `cognitive_profile_deserialization_defaults_decision_history_alternatives` passes — RON missing the field loads with the default value.
4. Every existing `CognitiveProfile { … }` literal construction compiles without the `missing field` error.
5. `cargo test --workspace` passes.
6. `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. Missing `decision_history_alternatives` in serialized input resolves to `default_decision_history_alternatives() == 5`.
2. `CognitiveProfile` remains `Copy` — `u8` is `Copy`, so no derive widening is needed.
3. No site in the workspace constructs `CognitiveProfile` with `.. fill-missing` syntax in production code that would silently drop the field (test fixtures are explicit literals by convention).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs` — new `cognitive_profile_deserialization_defaults_decision_history_alternatives` test, extension of `cognitive_profile_roundtrips_through_bincode` and `cognitive_profile_default_matches_split_defaults` to assert the new field.

### Commands

1. `cargo test -p worldwake-core cognitive_profile` — targeted, covers the field-level tests.
2. `cargo test --workspace` — confirms every literal construction site compiles.
3. `cargo clippy --workspace --all-targets -- -D warnings`
