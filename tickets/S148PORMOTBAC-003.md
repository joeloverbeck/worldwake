# S148PORMOTBAC-003: OperatingMode enum and per-tick derivation on AgentDecisionRuntime

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new core `OperatingMode` enum; new `operating_mode: OperatingMode` field on `AgentDecisionRuntime` (per-tick cache, not authoritative state); new `derive_operating_mode` helper in `agent_tick/portfolio.rs`
**Deps**: `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S148's five-slot portfolio degrades slot weights under safety pressure: emergency mode zeroes `EconomicOpportunity` and `SocialMotive` so agents under critical motive pressure stop spreading planner budget across opportunistic and social goals. Idle mode (no above-Background motive) keeps full breadth so agents still explore. The mode must be derived per-tick from the current motive severity *before* `assemble_portfolio` runs so the slot assembly can read it. Per FND-27 the mode is derived state, not authoritative — it lives on the per-tick runtime struct, not as an ECS component.

## Assumption Reassessment (2026-05-17)

1. `AgentDecisionRuntime` exists at `crates/worldwake-ai/src/decision_runtime.rs:153` (per-agent runtime struct stored in `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` per the agent-tick precedent). No existing `OperatingMode`-like field; the closest analog is `FrameRuntimeSnapshot` at `decision_runtime.rs:22-32` (frame-state snapshot, not per-tick decision mode). Spec confirms: "The only `AgentSnapshot` in the codebase is a test profiler at `tests/soak_profiler.rs:37`" — there is no runtime `AgentSnapshot` to attach `OperatingMode` to; `AgentDecisionRuntime` is the correct host.
2. Spec S148 D3 specifies three variants: `Emergency` (Pain or NeedPressure motives at Critical priority), `Normal` (default), `Idle` (no motive above Background). Derivation reads `AgendaEntry.motive_source_contributions: Vec<(MotiveSourceRef, u32)>` (confirmed at `crates/worldwake-ai/src/agenda_types.rs:34`) combined with the priority class produced by `compare_ranked_goals` (`crates/worldwake-ai/src/ranking.rs:3067`).
3. Shared abstraction under audit: the per-tick decision pipeline in `agent_tick/`. The derivation must run after `OrderedRanked` is produced (so motive contributions are visible) and before `assemble_portfolio` runs (so the mode is cached on the runtime when slot assembly reads it). Ticket 004 wires the call site; this ticket only adds the function and the field.
4. No save/load impact: `AgentDecisionRuntime` is per-tick runtime state; `OperatingMode` is *not* persisted across ticks (re-derived each tick). `SAVE_FORMAT_VERSION` at `crates/worldwake-sim/src/save_load.rs:7` (= 90) stays unchanged.

## Architecture Check

1. Per-tick derivation on `AgentDecisionRuntime` follows the existing "per-agent per-tick runtime state" precedent (`AgendaState`, `current_plan: Option<PlannedPlan>`); no new ECS component is introduced for derived state per FND-27 (caches are not authoritative).
2. The derivation is a pure function over `(belief_view, agent, OrderedRanked)` — no global state queries (FND-7 / FND-14).
3. Threshold encoding uses the established `GoalPriorityClass` enum (read through `compare_ranked_goals`'s composite ordering) rather than introducing a new magic-number cutoff per FND-3.

## Verification Layers

1. `OperatingMode` derivation correctness → focused unit tests in `crates/worldwake-ai/src/agent_tick/portfolio.rs::tests` constructing fixture `OrderedRanked` inputs for each branch (Emergency / Normal / Idle) and asserting `derive_operating_mode` returns the expected variant
2. `AgentDecisionRuntime.operating_mode` cache lifecycle → focused unit test asserting the field defaults to `Normal` and is overwritten per-tick when the helper is invoked (the per-tick overwrite contract is wired in ticket 004; this ticket verifies the field is mutable and the default is stable)

## What to Change

### 1. Define `OperatingMode`

Create `crates/worldwake-core/src/operating_mode.rs`:

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

Re-export from `crates/worldwake-core/src/lib.rs`: `pub use operating_mode::OperatingMode;`. The `Default` is `Normal` so a freshly-constructed `AgentDecisionRuntime` carries a benign initial value before the first derivation runs.

### 2. Add `operating_mode` field to `AgentDecisionRuntime`

In `crates/worldwake-ai/src/decision_runtime.rs:153`, add a field:

```rust
pub operating_mode: OperatingMode,
```

Update any `AgentDecisionRuntime::new()` / `Default` construction sites to initialize the field to `OperatingMode::default()` (which is `Normal`).

### 3. Add `derive_operating_mode` helper

In `crates/worldwake-ai/src/agent_tick/portfolio.rs`, add (outside the existing `#[cfg(test)]` block):

```rust
pub(crate) fn derive_operating_mode<V: GoalBeliefView>(
    belief: &V,
    agent: EntityId,
    ranked: &OrderedRanked<'_>,
) -> OperatingMode {
    let mut has_critical_pain_or_need = false;
    let mut highest_priority = GoalPriorityClass::Background;
    for entry in ranked.entries() {
        if entry.priority_class > highest_priority {
            highest_priority = entry.priority_class;
        }
        if entry.priority_class == GoalPriorityClass::Critical {
            for (motive_ref, _weight) in &entry.motive_source_contributions {
                let discriminant = MotiveSourceDiscriminant::from(&motive_ref.source);
                if matches!(
                    discriminant,
                    MotiveSourceDiscriminant::Pain | MotiveSourceDiscriminant::NeedPressure
                ) {
                    has_critical_pain_or_need = true;
                    break;
                }
            }
        }
    }
    if has_critical_pain_or_need {
        OperatingMode::Emergency
    } else if highest_priority <= GoalPriorityClass::Background {
        OperatingMode::Idle
    } else {
        OperatingMode::Normal
    }
}
```

(The exact accessor names — `entries()`, `priority_class`, `motive_source_contributions` — are validated against `agenda_types.rs` and `ranking.rs` during implementation; the structure above mirrors the spec's intent.)

The function is `pub(crate)` rather than `pub` — it's an internal helper for `agent_tick/`; only the cached `operating_mode` field on `AgentDecisionRuntime` is the cross-module surface.

## Files to Touch

- `crates/worldwake-core/src/operating_mode.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `operating_mode` field; update construction sites)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — add `derive_operating_mode` helper)

## Out of Scope

- Calling `derive_operating_mode` during the per-tick decision pipeline (ticket 004 — slot assembly extension wires the call site and writes the cache)
- Consumption of `operating_mode` by `assemble_portfolio` (ticket 004)
- Removal of `max_candidates_to_plan` and replacement with `max_plans_for_mode(mode)` reads (ticket 008)
- Observer rendering of operating mode (ticket 009)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core operating_mode` — `Default` is `Normal`; serde round-trips all 3 variants
2. `cargo test -p worldwake-ai agent_tick::portfolio::tests::derive_operating_mode_*` — 3+ focused tests covering: Emergency branch (Critical Pain or NeedPressure present), Idle branch (all candidates Background or below), Normal branch (above-Background candidates without Critical Pain/NeedPressure)
3. Existing suite: `cargo test --workspace`
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `OperatingMode` is derived state — not stored as an ECS component, not persisted across ticks, not part of save state.
2. `derive_operating_mode` is a pure function over its inputs (belief view, agent, ranked entries); no world mutation.
3. `AgentDecisionRuntime.operating_mode` defaults to `Normal` for a freshly-constructed runtime (before the first derivation runs).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/operating_mode.rs` — inline `#[cfg(test)]` module: serde round-trip on all 3 variants; `Default` assertion
2. `crates/worldwake-ai/src/agent_tick/portfolio.rs` — extend the existing `#[cfg(test)]` block at line 221+ with 3+ tests on `derive_operating_mode`: each constructs a minimal `OrderedRanked` fixture with controlled motive contributions and priority classes, asserts the expected `OperatingMode` variant
3. `crates/worldwake-ai/src/decision_runtime.rs` — extend existing tests (if present) or add a focused one asserting `AgentDecisionRuntime::default().operating_mode == OperatingMode::Normal`

### Commands

1. `cargo test -p worldwake-core operating_mode`
2. `cargo test -p worldwake-ai agent_tick::portfolio decision_runtime`
3. `./scripts/verify.sh`
