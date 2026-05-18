# S148PORMOTBAC-003: OperatingMode enum and per-tick derivation on AgentDecisionRuntime

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new core `OperatingMode` enum; new `operating_mode: OperatingMode` field on `AgentDecisionRuntime` (per-tick cache, not authoritative state); new `derive_operating_mode` helper in `agent_tick/portfolio.rs`
**Deps**: `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S148's five-slot portfolio degrades slot weights under safety pressure: emergency mode zeroes `EconomicOpportunity` and `SocialMotive` so agents under critical motive pressure stop spreading planner budget across opportunistic and social goals. Idle mode (no above-Background motive) keeps full breadth so agents still explore. This ticket added the per-tick derived mode surface that later slot assembly can read before `assemble_portfolio` consumes operating-mode-adjusted weights. Per FND-27 the mode is derived state, not authoritative — it lives on the per-tick runtime struct, not as an ECS component.

## Assumption Reassessment (2026-05-17)

1. Before implementation, `AgentDecisionRuntime` existed at `crates/worldwake-ai/src/decision_runtime.rs:153` (per-agent runtime struct stored in `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` per the agent-tick precedent). No existing `OperatingMode`-like field was present; the closest analog was `FrameRuntimeSnapshot` at `decision_runtime.rs:22-32` (frame-state snapshot, not per-tick decision mode). Spec confirms: "The only `AgentSnapshot` in the codebase is a test profiler at `tests/soak_profiler.rs:37`" — there is no runtime `AgentSnapshot` to attach `OperatingMode` to; `AgentDecisionRuntime` was the correct host.
2. Spec S148 D3 specifies three variants: `Emergency` (Pain or NeedPressure motives at Critical priority), `Normal` (default), `Idle` (no motive above Background). Derivation reads `AgendaEntry.motive_source_contributions: Vec<(MotiveSourceRef, u32)>` (confirmed at `crates/worldwake-ai/src/agenda_types.rs:34`) combined with the priority class produced by `compare_ranked_goals` (`crates/worldwake-ai/src/ranking.rs:3067`).
3. Shared abstraction under audit: the per-tick decision pipeline in `agent_tick/`. The derivation belongs after `OrderedRanked` is produced (so motive contributions are visible) and before `assemble_portfolio` runs (so the mode is cached on the runtime when slot assembly reads it). Ticket 004 wires the call site; this ticket added the function and the field only.
4. No save/load impact: `AgentDecisionRuntime` is per-tick runtime state; `OperatingMode` is *not* persisted across ticks (re-derived each tick). `SAVE_FORMAT_VERSION` at `crates/worldwake-sim/src/save_load.rs:7` (= 90) stays unchanged.

## Architecture Check

1. Per-tick derivation on `AgentDecisionRuntime` follows the existing "per-agent per-tick runtime state" precedent (`AgendaState`, `current_plan: Option<PlannedPlan>`); no new ECS component is introduced for derived state per FND-27 (caches are not authoritative).
2. The derivation is a pure function over `(belief_view, agent, OrderedRanked)` — no global state queries (FND-7 / FND-14).
3. Threshold encoding uses the established `GoalPriorityClass` enum (read through `compare_ranked_goals`'s composite ordering) rather than introducing a new magic-number cutoff per FND-3.

## Verified Layers

1. `OperatingMode` derivation correctness → focused unit tests in `crates/worldwake-ai/src/agent_tick/portfolio.rs::tests` constructing fixture `OrderedRanked` inputs for each branch (Emergency / Normal / Idle) and asserting `derive_operating_mode` returns the expected variant
2. `AgentDecisionRuntime.operating_mode` cache lifecycle → focused unit test asserting the field defaults to `Normal` and is overwritten per-tick when the helper is invoked (the per-tick overwrite contract is wired in ticket 004; this ticket verifies the field is mutable and the default is stable)

## Landed Changes

### 1. Defined `OperatingMode`

Created `crates/worldwake-core/src/operating_mode.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Serialize, Deserialize)]
pub enum OperatingMode {
    Emergency,
    #[default]
    Normal,
    Idle,
}
```

Re-exported from `crates/worldwake-core/src/lib.rs`: `pub use operating_mode::OperatingMode;`. The `Default` is `Normal` so a freshly-constructed `AgentDecisionRuntime` carries a benign initial value before the first derivation runs.

### 2. Added `operating_mode` field to `AgentDecisionRuntime`

In `crates/worldwake-ai/src/decision_runtime.rs:153`, added a field:

```rust
pub operating_mode: OperatingMode,
```

The derived `Default` initializes the field to `OperatingMode::default()` (which is `Normal`). The bincode round-trip test now verifies the field is preserved when non-default.

### 3. Added `derive_operating_mode` helper

In `crates/worldwake-ai/src/agent_tick/portfolio.rs`, added `derive_operating_mode` with the S148 signature and a private `derive_operating_mode_from_ranked` implementation used by focused tests. The live `OrderedRanked` API uses `iter()` / `IntoIterator`, not the drafted `entries()` accessor. The landed helper scans ranked entries, records the highest observed `GoalPriorityClass`, and returns `Emergency` only when a Critical entry carries a Pain or NeedPressure motive contribution.

The function is `pub(crate)` rather than `pub` — it's an internal helper for `agent_tick/`; only the cached `operating_mode` field on `AgentDecisionRuntime` is the cross-module surface.

## Landed Files

- `crates/worldwake-core/src/operating_mode.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `operating_mode` field; update construction sites)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — add `derive_operating_mode` helper)

## Out of Scope

- Calling `derive_operating_mode` during the per-tick decision pipeline (ticket 004 — slot assembly extension wires the call site and writes the cache)
- Consumption of `operating_mode` by `assemble_portfolio` (ticket 004)
- Removal of `max_candidates_to_plan` and replacement with `max_plans_for_mode(mode)` reads (ticket 008)
- Observer rendering of operating mode (ticket 009)

## Completed Acceptance Criteria

### Tests Passed

1. `cargo test -p worldwake-core operating_mode` — `Default` is `Normal`; serde round-trips all 3 variants
2. `cargo test -p worldwake-ai agent_tick::portfolio::tests::derive_operating_mode_*` — 3+ focused tests covering: Emergency branch (Critical Pain or NeedPressure present), Idle branch (all candidates Background or below), Normal branch (above-Background candidates without Critical Pain/NeedPressure)
3. Existing suite: `cargo test --workspace`
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `OperatingMode` is derived state — not stored as an ECS component, not persisted across ticks, not part of save state.
2. `derive_operating_mode` is a pure function over its inputs (belief view, agent, ranked entries); no world mutation.
3. `AgentDecisionRuntime.operating_mode` defaults to `Normal` for a freshly-constructed runtime (before the first derivation runs).

## Test Plan Result

### New/Modified Tests Result

1. `crates/worldwake-core/src/operating_mode.rs` — inline `#[cfg(test)]` module verifies bincode round-trip on all 3 variants and `Default == Normal`.
2. `crates/worldwake-ai/src/agent_tick/portfolio.rs` — existing `#[cfg(test)]` block now covers Emergency, Idle, and Normal derivation branches with controlled `OrderedRanked` fixtures.
3. `crates/worldwake-ai/src/decision_runtime.rs` — existing default and bincode tests now assert `AgentDecisionRuntime::default().operating_mode == OperatingMode::Normal` and preservation of a non-default `OperatingMode::Emergency`.

### Commands Run

1. `cargo test -p worldwake-core operating_mode`
2. `cargo test -p worldwake-ai agent_tick::portfolio`
3. `cargo test -p worldwake-ai decision_runtime::tests::agent_decision_runtime`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Verification Result

1. Passed `cargo test -p worldwake-core operating_mode` — 2 core `OperatingMode` tests passed.
2. Passed `cargo test -p worldwake-ai agent_tick::portfolio::tests::derive_operating_mode` — the three new derivation branch tests passed.
3. Passed `cargo test -p worldwake-ai decision_runtime::tests::agent_decision_runtime` — existing runtime default, non-component, and bincode tests passed with the new mode field.
4. Passed `cargo test -p worldwake-ai agent_tick::portfolio` — all 11 portfolio tests passed.
5. Passed `cargo test --workspace` — workspace unit, integration, and doctest suite passed.
6. Passed `cargo clippy --workspace --all-targets -- -D warnings` — CI-matching all-target clippy passed.
7. Waived `./scripts/verify.sh` — covered by the exact constituent proof required for this ticket: `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all` was run before proof and produced the final formatted source.

## Outcome

Completed on 2026-05-18. Added the core `OperatingMode` enum, exported it from `worldwake-core`, cached `operating_mode` on `AgentDecisionRuntime` with serde defaulting, and added the portfolio derivation helper that classifies Critical Pain/NeedPressure as Emergency, all-Background ranked inputs as Idle, and other above-Background ranked inputs as Normal.

The implementation kept ticket 004 as the owner of pipeline call-site wiring and slot-weight consumption. The drafted combined command `cargo test -p worldwake-ai agent_tick::portfolio decision_runtime` is not valid Cargo syntax, so focused AI proof was split into separate portfolio and decision-runtime selectors.
