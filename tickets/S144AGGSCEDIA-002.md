# S144AGGSCEDIA-002: SlotKind visibility promotion and serde derives

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None — foundation ticket (S144 D3)

## Problem

S144's `GoalPressureMetrics.candidates_emitted_by_slot` keys a `BTreeMap` on `SlotKind`. `SlotKind` is currently `pub(crate)` in `crates/worldwake-ai/src/agent_tick/portfolio.rs` and derives no serde traits. `ScenarioDiagnosticsReport` is a `pub` type consumed by the `worldwake-cli` observer and must JSON-serialize; the report cannot name or serialize `SlotKind` as-is.

## Assumption Reassessment (2026-05-14)

1. `SlotKind` is defined at `crates/worldwake-ai/src/agent_tick/portfolio.rs:11` as `pub(crate) enum SlotKind { Survival, Commitment, Economic }` with derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash` (no `Serialize`/`Deserialize`). Consumers are in-crate only: `portfolio.rs` itself and `decision_trace.rs:23,79` (`use crate::agent_tick::portfolio::SlotKind` and `slots: BTreeMap<SlotKind, PortfolioSlotTrace>`). No cross-crate consumers — this is a visibility *widening*, not a rename, so there is no consumer breakage and no rename blast radius.
2. S144 spec D3 (`specs/S144-aggregate-scenario-diagnostics.md`) specifies: promote `pub(crate)` → `pub`, re-export from `crates/worldwake-ai/src/lib.rs`, add `Serialize, Deserialize` derives. No variant or semantic change.
3. Shared abstraction boundary: `SlotKind` becomes a public type of `worldwake-ai`, consumed as a `BTreeMap` key in `scenario_diagnostics` (ticket 004). Data contract under audit: the variant set and `Ord` derive (BTreeMap key requirement) stay unchanged; only visibility and serde derives widen.

## Architecture Check

1. Widening visibility and adding serde derives is the minimal change that lets the diagnostics report key on the real portfolio-slot type rather than duplicating a parallel enum. A duplicate enum would violate FND-28 (two representations of the same concept).
2. No backwards-compatibility aliasing/shims — the `pub(crate)` form is widened in place; existing in-crate consumers continue to compile unchanged with no shim.

## Verification Layers

1. `SlotKind` serde round-trip (serialize → deserialize → equal) -> focused unit test in `portfolio.rs`.
2. Single-layer ticket: a derive/visibility change has no decision-trace, action-trace, or event-log surface — additional layer mapping is not applicable.

## What to Change

### 1. Widen `SlotKind` visibility and derives

In `crates/worldwake-ai/src/agent_tick/portfolio.rs`, change `pub(crate) enum SlotKind` to `pub enum SlotKind` and add `Serialize, Deserialize` to its derive list.

### 2. Re-export from crate root

Add a `pub use` re-export of `SlotKind` from `crates/worldwake-ai/src/lib.rs` so external crates (the `worldwake-cli` observer) can name it.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — visibility + derives)
- `crates/worldwake-ai/src/lib.rs` (modify — `pub use` re-export)

## Out of Scope

- Any change to `SlotKind` variants or their semantics.
- `PortfolioSlotTrace` or `Portfolio` visibility — only `SlotKind` is widened.
- Diagnostics-report field usage (ticket 004).

## Acceptance Criteria

### Tests That Must Pass

1. `SlotKind` serializes and deserializes through serde to an equal value for every variant.
2. Existing in-crate consumers (`decision_trace.rs` portfolio-slot traces) compile unchanged.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `SlotKind`'s variant set and `Ord` ordering are unchanged — only visibility and serde derives widen.
2. No new `SlotKind` consumers are added in this ticket — the change is purely an enabling widening.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/portfolio.rs` (inline `#[cfg(test)]`) — `SlotKind` serde round-trip across all three variants.

### Commands

1. `cargo test -p worldwake-ai portfolio`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo test -p worldwake-ai` (narrow boundary — this ticket touches only `worldwake-ai`)
