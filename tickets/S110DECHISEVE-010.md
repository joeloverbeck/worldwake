# S110DECHISEVE-010: Richer repair-kind provenance for decision events

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — add authoritative runtime provenance for non-target repair classes before widening `RepairApplied`
**Deps**: archive/tickets/S110DECHISEVE-009.md

## Problem

`S110DECHISEVE-009` lands only the currently provable `RepairKind::AlternateTarget` slice. The broader S110 repair taxonomy (`AlternateRoute`, `AlternateMerchant`, `AlternateRecipe`) is still not represented by an authoritative runtime carrier, so those variants cannot yet be emitted honestly.

## Assumption Reassessment (2026-04-20)

1. The live durable repair substrate is `RepairKey { goal_key, alternate_target }` in `crates/worldwake-core/src/repair_memory.rs`, which proves only alternate-target success.
2. `crates/worldwake-ai/src/failure_handling.rs` classifies failed plans, but does not expose a successful repair-selection result with a stable repair-kind taxonomy.
3. Shared abstraction boundary under audit: the planning/execution/runtime transport needed to carry accepted repair provenance into `agent_tick/mod.rs` before event emission.
4. This is a separate architectural concern from 009, not incidental fallout: emitting the broader repair family now would require new provenance substrate, not just a local event-log write.

## Architecture Check

1. Adding explicit successful repair provenance is cleaner than inferring repair class from later memory or opportunity anchors.
2. The end state should preserve one authoritative carrier from repair acceptance to event emission, rather than parallel guessed classifications in ranking, memory, and the event log.

## Verification Layers

1. Repair acceptance transport -> focused unit/runtime coverage at the seam that accepts the repair.
2. Event emission -> focused `agent_tick` runtime test proving each newly supported `RepairKind`.
3. If no truthful proof seam exists for one drafted repair kind, narrow again instead of emitting a guessed variant.

## What to Change

### 1. Introduce successful repair provenance transport

Carry the accepted repair class and repaired step through the runtime at the point a repair is actually chosen and succeeds.

### 2. Widen `RepairApplied`

Extend the 009 alternate-target slice so `RepairApplied` can honestly emit the additional supported repair kinds.

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify as needed)
- `crates/worldwake-ai/src/agent_tick/*` (modify as needed)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)

## Out of Scope

- Replay coverage
- Observer rendering

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove each newly supported repair class emits the correct `RepairAppliedPayload`.
2. `cargo test -p worldwake-ai`

### Invariants

1. No `RepairApplied` variant is emitted from inferred post-hoc context alone.
2. Every emitted repair class is backed by an explicit successful repair carrier in live runtime state.

## Test Plan

### New/Modified Tests

1. Focused `agent_tick` / repair-seam tests for each newly supported repair kind.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
