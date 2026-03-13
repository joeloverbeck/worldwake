# FND02-002: Wire SellCommodity Goal Emission in Candidate Generation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate_generation.rs, enterprise.rs
**Deps**: Phase 2 complete

## Problem

`GoalKind::SellCommodity { commodity }` is defined in `crates/worldwake-core/src/goal.rs` (line ~36) and already ranked in `crates/worldwake-ai/src/ranking.rs` (line ~128, `PriorityClass::Medium`), but no emission function exists in `crates/worldwake-ai/src/candidate_generation.rs`. Agents with merchandise and surplus commodity can never autonomously decide to sell. This is a dead GoalKind variant — defined but unreachable through the candidate generation pipeline.

## Assumption Reassessment (2026-03-13)

1. `GoalKind::SellCommodity { commodity: CommodityKind }` exists in `goal.rs` — confirmed.
2. `ranking.rs` handles SellCommodity with `PriorityClass::Medium` — confirmed (line ~128).
3. `candidate_generation.rs` has `emit_enterprise_candidates()` (line ~126) which calls `emit_restock_goals()` and `emit_move_cargo_goals()` but has no `emit_sell_goals()` — confirmed.
4. `enterprise.rs` has `analyze_candidate_enterprise()` returning `EnterpriseSignals` with `restock_gaps: BTreeMap<CommodityKind, Quantity>` — confirmed. Need to determine surplus detection mechanism.
5. `MerchandiseProfile` has `sale_kinds: BTreeSet<CommodityKind>` and `home_market: Option<EntityId>` — confirmed.
6. Evidence tracking uses `BTreeSet<EntityId>` for entities and places — confirmed, must follow same pattern.

## Architecture Check

1. Mirrors existing `emit_restock_goals()` pattern — adds a symmetric `emit_sell_goals()` that checks for surplus above restock threshold. Follows existing enterprise candidate emission structure.
2. No backwards-compatibility shims — new emission path, no existing behavior changes.

## What to Change

### 1. Add surplus detection to `enterprise.rs`

Extend `EnterpriseSignals` (or add a parallel function) to compute sell surpluses: for each `sale_kind` in the agent's `MerchandiseProfile`, if the agent's current held quantity exceeds the restock threshold, the difference is surplus available for sale.

### 2. Add `emit_sell_goals()` in `candidate_generation.rs`

New function following the pattern of existing `emit_restock_goals()`:

**Emission conditions** (all must be true):
- Agent has `MerchandiseProfile` component.
- Agent holds commodity quantity exceeding restock threshold for at least one `sale_kind` (surplus exists).
- Agent is at a place (not in transit) — checked via placement relation.

**For each surplus commodity**:
- Emit `GoalKind::SellCommodity { commodity }` candidate.
- Evidence: the commodity entity/lot and the current place.

### 3. Wire `emit_sell_goals()` into `emit_enterprise_candidates()`

Call the new function from within `emit_enterprise_candidates()`, parallel to existing `emit_restock_goals()` and `emit_move_cargo_goals()` calls.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `emit_sell_goals()` and call site)
- `crates/worldwake-ai/src/enterprise.rs` (modify — add surplus detection if needed)

## Out of Scope

- Do NOT implement the actual sell action handler — that exists or will be in S04.
- Do NOT modify `GoalKind` enum or `ranking.rs` — SellCommodity is already defined and ranked.
- Do NOT modify `MerchandiseProfile` or trade system code.
- Do NOT change any action definitions or handlers.
- Do NOT touch `worldwake-core`, `worldwake-sim`, or `worldwake-systems` crates.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: Agent with `MerchandiseProfile` and surplus commodity at a place emits `SellCommodity` candidate for each surplus commodity kind.
2. Unit test: Agent without `MerchandiseProfile` does not emit sell candidates.
3. Unit test: Agent with `MerchandiseProfile` but no surplus (quantity <= restock threshold) does not emit sell candidates.
4. Unit test: Agent in transit (no place) does not emit sell candidates.
5. Existing suite: `cargo test -p worldwake-ai`
6. Full suite: `cargo test --workspace`

### Invariants

1. All evidence uses `BTreeSet<EntityId>` — no `HashSet`.
2. No `f32`/`f64` in any new code — use `Permille` and integer types.
3. Existing restock and move-cargo emission behavior unchanged.
4. Deterministic output — same inputs produce same candidates in same order.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (or test module) — unit tests for sell emission with/without merchandise profile, with/without surplus, with/without place placement.

### Commands

1. `cargo test -p worldwake-ai -- sell` — targeted sell-related tests
2. `cargo test -p worldwake-ai` — full AI crate suite
3. `cargo clippy --workspace` — lint check
4. `cargo test --workspace` — full workspace suite
