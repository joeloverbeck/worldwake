# S144AGGSCEDIA-002: SlotKind visibility promotion and serde derives

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None — foundation ticket (S144 D3)

## Problem

S144's `GoalPressureMetrics.candidates_emitted_by_slot` keys a `BTreeMap` on `SlotKind`. Before this ticket, `SlotKind` was `pub(crate)` in `crates/worldwake-ai/src/agent_tick/portfolio.rs` and derived no serde traits. `ScenarioDiagnosticsReport` is a `pub` type consumed by the `worldwake-cli` observer and must JSON-serialize, so the report needed `SlotKind` to be nameable and serializable outside the private portfolio module.

## Assumption Reassessment (2026-05-14)

1. Before implementation, `SlotKind` was defined at `crates/worldwake-ai/src/agent_tick/portfolio.rs` as `pub(crate) enum SlotKind { Survival, Commitment, Economic }` with derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash` and no `Serialize`/`Deserialize`. Consumers were in-crate only: `portfolio.rs` itself and `decision_trace.rs` (`use crate::agent_tick::portfolio::SlotKind` and `slots: BTreeMap<SlotKind, PortfolioSlotTrace>`). No cross-crate consumers existed, so this was a visibility *widening*, not a rename, with no rename blast radius.
2. S144 spec D3 (`archive/specs/S144-aggregate-scenario-diagnostics.md`) specifies: promote `pub(crate)` → `pub`, re-export from `crates/worldwake-ai/src/lib.rs`, add `Serialize, Deserialize` derives. No variant or semantic change.
3. Shared abstraction boundary: `SlotKind` becomes a public type of `worldwake-ai`, consumed as a `BTreeMap` key in `scenario_diagnostics` (ticket 004). Data contract under audit: the variant set and `Ord` derive (BTreeMap key requirement) stay unchanged; only visibility and serde derives widen.

## Architecture Check

1. Widening visibility and adding serde derives is the minimal change that lets the diagnostics report key on the real portfolio-slot type rather than duplicating a parallel enum. A duplicate enum would violate FND-28 (two representations of the same concept).
2. No backwards-compatibility aliasing/shims — the `pub(crate)` form is widened in place; existing in-crate consumers continue to compile unchanged with no shim.

## Verified Layers

1. `SlotKind` serde round-trip (serialize -> deserialize -> equal) is covered by `agent_tick::portfolio::tests::slot_kind_round_trips_through_serde`.
2. Single-layer ticket: the derive/visibility change has no decision-trace, action-trace, or event-log surface, so no additional runtime proof layer applies.

## Landed Changes

### 1. Widened `SlotKind` visibility and derives

In `crates/worldwake-ai/src/agent_tick/portfolio.rs`, `SlotKind` is now a `pub enum` and derives `Serialize, Deserialize` alongside its existing copy/order/hash traits.

### 2. Re-exported from crate root

`crates/worldwake-ai/src/lib.rs` re-exports `SlotKind` so external crates can name the portfolio-slot key without exposing the rest of the private portfolio module.

## Landed Files

- `crates/worldwake-ai/src/agent_tick/portfolio.rs` — visibility + serde derives, focused serde round-trip test.
- `crates/worldwake-ai/src/lib.rs` — `SlotKind` crate-root re-export.

## Out of Scope

- Any change to `SlotKind` variants or their semantics.
- `PortfolioSlotTrace` or `Portfolio` visibility — only `SlotKind` is widened.
- Diagnostics-report field usage (ticket 004).

## Acceptance Result

### Tests Passed

1. `SlotKind` serializes and deserializes through serde to an equal value for every variant.
2. Existing in-crate consumers (`decision_trace.rs` portfolio-slot traces) compile unchanged.
3. Existing `worldwake-ai` suite passes.

### Invariants

1. `SlotKind`'s variant set and `Ord` ordering are unchanged; only visibility and serde derives widened.
2. No new runtime `SlotKind` consumers were added in this ticket; the change is purely an enabling widening for later S144 report work.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/portfolio.rs` (inline `#[cfg(test)]`) — `SlotKind` serde round-trip across all three variants.

### Commands Run

1. `cargo test -p worldwake-ai slot_kind_round_trips_through_serde`
2. `cargo fmt --all`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-14.

- `SlotKind` is public and serde-ready for the later `ScenarioDiagnosticsReport.candidates_emitted_by_slot` map.
- `worldwake-ai` re-exports `SlotKind`, so downstream observer/report code can name the real portfolio-slot type instead of defining a duplicate aggregation key.
- The implementation stayed within S144 D3; no variant, ordering, planner, trace, or runtime behavior changed.

## Deviations

- The focused serde proof uses `bincode` because it is already a `worldwake-ai` dependency and exercises the serde `Serialize`/`Deserialize` contract without adding a JSON-only dev dependency.

## Verification Result

- Passed `cargo test -p worldwake-ai slot_kind_round_trips_through_serde`.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
