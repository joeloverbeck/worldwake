# S112PORPLAN-001: CognitiveProfile slot weights

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` (CognitiveProfile field addition)
**Deps**: None

## Problem

S112's portfolio assembly orders plausible slots by score × slot-weight. The weights must be per-agent (FND-22) and declared on `CognitiveProfile`, but the type currently has no slot-weight fields. Downstream tickets (002 types, 005 integration) need `PortfolioSlotWeights` to exist before they can consume it.

This ticket adds the foundation: the `PortfolioSlotWeights` struct and its `slot_weights: PortfolioSlotWeights` field on `CognitiveProfile`, with a `Default` impl matching the spec's declared defaults (survival=1000, commitment=900, economic=700).

## Assumption Reassessment (2026-04-20)

1. `CognitiveProfile` is defined at `crates/worldwake-core/src/cognitive_profile.rs:6` and serialized with serde; existing `#[serde(default = "...")]` fields (e.g., `stale_belief_backoff_ticks`, `use_ff_heuristic`) establish the pattern for backward-compatible field addition. `SAVE_FORMAT_VERSION = 34` at `crates/worldwake-sim/src/save_load.rs:6` — adding a new field with `#[serde(default)]` does not require a version bump.
2. Spec S112 D5 names `PortfolioSlotWeights` with three `Permille` fields (survival / commitment / economic) and defers the `information` field to S113. Defaults declared in the spec: survival=1000, commitment=900, economic=700.
3. Shared boundary: `CognitiveProfile` serde-deserialized from scenario RON (`crates/worldwake-cli/src/scenario/types.rs:932` style) and from save-state (`save_load.rs`). `#[serde(default)]` on `slot_weights` preserves both paths — existing scenarios and saves without the field deserialize with `PortfolioSlotWeights::default()`.
4. Mismatch + correction: the ticket's constructor sweep overcounted one live edit site. `per_agent_belief_view.rs` uses `..CognitiveProfile::default()` spread and required no change; the real exhaustive literal fallout was 8 files (`decision_runtime.rs`, `agent_tick/tests.rs`, `agent_tick/planning.rs`, `failure_handling.rs`, `cognitive_profile.rs`, `goal_model.rs`, `search/tests.rs`, `delta.rs`).
5. Additional reassessment note: other live `CognitiveProfile { ... }` sites in CLI and golden/conformance tests also use spread-based updates and therefore remained correct without edits.
6. Construction-site touchpoint burden remained Medium — the final exhaustive literal set was 8 files, and every non-test site was a deterministic mechanical update.

## Architecture Check

1. Profile-driven (FND-22) per `docs/spec-drafting-rules.md` section 3: slot weights are stable per-agent parameters, not hardcoded constants. Two agents with identical motives can differ on portfolio shape by carrying different weights.
2. No backwards-compatibility shim: new field is added with `#[serde(default)]` so deserialization compatibility is preserved at the boundary (save/load, RON import) without introducing a second live representation (FND-28).

## Verification Layers

1. Field exists, defaults are correct → focused unit test in `cognitive_profile.rs` (additions to existing `cognitive_profile_default_matches_split_defaults` and `cognitive_profile_roundtrips_through_bincode` tests, plus a new default-when-absent serde test).
2. Single-layer ticket — `PortfolioSlotWeights` introduces a pure-data profile struct with no runtime behavior; no decision/action/event-log surfaces are emitted until later tickets consume it.

## What to Change

### 1. Add `PortfolioSlotWeights` to `cognitive_profile.rs`

Insert above `impl Default for CognitiveProfile`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PortfolioSlotWeights {
    pub survival: Permille,
    pub commitment: Permille,
    pub economic: Permille,
}

impl Default for PortfolioSlotWeights {
    fn default() -> Self {
        Self {
            survival: Permille::new_unchecked(1000),
            commitment: Permille::new_unchecked(900),
            economic: Permille::new_unchecked(700),
        }
    }
}
```

### 2. Add `slot_weights` field to `CognitiveProfile`

Insert after `pub decision_history_alternatives: u8` with `#[serde(default)]`:

```rust
#[serde(default)]
pub slot_weights: PortfolioSlotWeights,
```

And in the `Default::default()` impl, add `slot_weights: PortfolioSlotWeights::default(),`.

### 3. Re-export from `worldwake-core/src/lib.rs`

Add `PortfolioSlotWeights` to the `pub use` list alongside `CognitiveProfile`.

### 4. Extend existing unit tests in `cognitive_profile.rs`

- Add `slot_weights` to `cognitive_profile_default_matches_split_defaults` assertion.
- Add `slot_weights` to the `cognitive_profile_roundtrips_through_bincode` fixture.
- Add a new serde-default test `cognitive_profile_deserialization_defaults_slot_weights` following the pattern of the existing `cognitive_profile_deserialization_defaults_*` tests.

### 5. Update exhaustive-enum construction sites

Add `slot_weights: PortfolioSlotWeights::default()` to every `CognitiveProfile { ... }` literal that enumerates all fields without `..Default::default()` spread. Sites:

- `crates/worldwake-core/src/cognitive_profile.rs` — roundtrip test fixture
- `crates/worldwake-core/src/delta.rs` — delta test fixture
- `crates/worldwake-ai/src/decision_runtime.rs`
- `crates/worldwake-ai/src/failure_handling.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`

Spread-based sites (e.g., `CognitiveProfile { max_plan_depth: 12, ..CognitiveProfile::default() }`) need no change.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)
- `crates/worldwake-core/src/delta.rs` (modify — test fixture)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- `PortfolioSlotWeights::information` field (deferred to S113 follow-up).
- Any consumer of `slot_weights` (D1/D2 categorization, D4 integration, D6 trace). This ticket only *declares* the profile; consumers land in 002–005.
- Scenario RON authoring for `slot_weights` — existing scenarios fall back to default via `#[serde(default)]`. No scenario edits required.
- `max_candidates_to_plan` bypass behavior change (that's D4, ticket 005).

## Acceptance Criteria

### Tests That Must Pass

1. `cognitive_profile_default_matches_split_defaults` — updated to include `slot_weights: PortfolioSlotWeights::default()` assertion.
2. `cognitive_profile_roundtrips_through_bincode` — updated fixture roundtrips with explicit slot weights.
3. `cognitive_profile_deserialization_defaults_slot_weights` (new) — RON without `slot_weights` field deserializes with default weights.
4. Existing suite: `cargo test -p worldwake-core`, `cargo test -p worldwake-ai`, `cargo test -p worldwake-sim`.

### Invariants

1. Existing scenario RON files deserialize without modification.
2. `PortfolioSlotWeights` derives `Copy` (required because `CognitiveProfile` derives `Copy`).
3. No existing `CognitiveProfile` field semantics change; only an additive field is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs` — modify two existing tests (default/roundtrip) and add one new serde-default test.

### Commands

1. `cargo test -p worldwake-core cognitive_profile`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

- Added `PortfolioSlotWeights` to `crates/worldwake-core/src/cognitive_profile.rs` with the spec defaults and wired `slot_weights: PortfolioSlotWeights` onto `CognitiveProfile` behind `#[serde(default)]`.
- Re-exported `PortfolioSlotWeights` from `worldwake-core` and updated every exhaustive `CognitiveProfile` literal that needed the new field.
- Extended the core tests to cover default values, bincode roundtrip, and omitted-field serde defaulting for `slot_weights`.

## Deviations

- Reassessment narrowed the constructor fallout: `crates/worldwake-sim/src/per_agent_belief_view.rs` stayed unchanged because its `CognitiveProfile` literal already uses `..CognitiveProfile::default()`.
- The focused core proof ran as `cargo test -p worldwake-core --lib cognitive_profile_` so the new serde-default test and the adjacent `cognitive_profile` unit suite all executed together on the live test binary.

## Verification Result

- Passed `cargo test -p worldwake-core --lib cognitive_profile_`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
