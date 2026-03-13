# FND02-004: Feedback Dampening Audit Across Phase 2 Systems

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible — code fixes if undamped loops found
**Deps**: Phase 2 complete

## Problem

No systematic audit has been performed on Phase 2 systems for amplifying feedback loops (Principle 10: Every Positive Feedback Loop Needs a Physical Dampener). Each system was implemented independently; cross-system amplification patterns may exist undocumented and undamped. Numerical clamps (`min`, `max`, `clamp`) are not acceptable dampeners — only physical world mechanisms qualify.

## Assumption Reassessment (2026-03-13)

1. `docs/dampening-audit-phase2.md` does not exist — confirmed, must be created.
2. Phase 2 systems to audit: needs (`needs.rs`, `needs_actions.rs`), production (`production.rs`, `production_actions.rs`), trade (`trade.rs`, `trade_actions.rs`), combat (`combat.rs`), AI enterprise (`enterprise.rs`) — confirmed all exist.
3. All systems use `Permille` and integer types (no floats) — confirmed.
4. `DemandMemory` has aging mechanism (`TradeDispositionProfile.demand_memory_retention_ticks`) — confirmed, this is a potential dampener for trade loops.
5. `BlockedIntentMemory` has expiration — confirmed, this dampens goal spirals.
6. `PlanningBudget` limits planning depth/width — confirmed via `budget.rs`.

## Architecture Check

1. Analysis-first approach — audit all loops before any code changes. Code fixes only if undamped loops are discovered.
2. No backwards-compatibility shims — any fixes add new dampening mechanisms, not wrappers.

## What to Change

### 1. Audit Needs/Metabolism system

**Files to read**: `crates/worldwake-systems/src/needs.rs`, `crates/worldwake-systems/src/needs_actions.rs`

Investigate:
- Does need satisfaction create conditions that accelerate need growth? (eating -> energy -> activity -> faster hunger)
- Document dampeners: resource depletion (food consumed is gone), action duration (eating takes time), capacity limits, deprivation wound consequences.

### 2. Audit Production system

**Files to read**: `crates/worldwake-systems/src/production.rs`, `crates/worldwake-systems/src/production_actions.rs`

Investigate:
- Does production create conditions accelerating further production? (crafting tools -> faster crafting -> more tools)
- Document dampeners: raw material depletion, workstation occupancy, action duration, storage/load limits (`LoadUnits`, container capacity).

### 3. Audit Trade system

**Files to read**: `crates/worldwake-systems/src/trade.rs`, `crates/worldwake-systems/src/trade_actions.rs`

Investigate:
- Does successful trade create conditions for more trade? (profit -> buy more -> sell more -> more profit)
- Document dampeners: inventory/load limits, travel time between markets, demand saturation (`DemandMemory` aging via `demand_memory_retention_ticks`), commodity conservation.

### 4. Audit Combat system

**Files to read**: `crates/worldwake-systems/src/combat.rs`

Investigate:
- Does combat create conditions for more combat? (wounds -> vulnerability -> more attacks -> more wounds)
- Document dampeners: wound incapacitation, death, bleed rate mechanics, weapon depletion, goal switching away from combat.

### 5. Audit AI Enterprise logic

**Files to read**: `crates/worldwake-ai/src/enterprise.rs`, `crates/worldwake-ai/src/candidate_generation.rs`

Investigate:
- Does enterprise goal generation create runaway goal spirals?
- Document dampeners: `PlanningBudget` limits, `BlockedIntentMemory` with expiration, goal switching margins, restock threshold bounds.

### 6. Document findings

Create `docs/dampening-audit-phase2.md` with:
- Per-system section listing all identified amplifying loops.
- For each loop: the physical dampener mechanism (not numerical clamps).
- Cross-system interactions that could amplify.
- Any undamped loops requiring code fixes.

### 7. Fix undamped loops (if any)

If the audit reveals loops with no physical dampener (only numerical clamps), add concrete dampening mechanisms through physical world processes.

## Files to Touch

- `docs/dampening-audit-phase2.md` (new — audit document)
- `crates/worldwake-systems/src/needs.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-systems/src/needs_actions.rs` (read for audit)
- `crates/worldwake-systems/src/production.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-systems/src/production_actions.rs` (read for audit)
- `crates/worldwake-systems/src/trade.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-systems/src/trade_actions.rs` (read for audit)
- `crates/worldwake-systems/src/combat.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-ai/src/enterprise.rs` (read for audit; modify only if undamped loop found)

## Out of Scope

- Do NOT restructure any Phase 2 systems.
- Do NOT refactor code for style or performance — only add dampening mechanisms if missing.
- Do NOT audit Phase 1 systems (E01-E08) — they are stable and not in scope.
- Do NOT add new systems or components — only document existing behavior and fix gaps.
- Do NOT modify specs — this is a code and documentation audit.

## Acceptance Criteria

### Tests That Must Pass

1. `docs/dampening-audit-phase2.md` exists and covers all five Phase 2 system domains.
2. Each identified amplifying loop has a documented physical dampener (not a numerical clamp).
3. No undamped loops remain after any code fixes.
4. If code changes were made: `cargo test --workspace` passes.
5. If code changes were made: `cargo clippy --workspace` passes.

### Invariants

1. No new numerical-only clamps (`min`, `max`, `clamp`) introduced as dampeners — all dampeners must be physical world mechanisms.
2. Existing system behavior preserved unless explicitly adding a dampener.
3. Determinism maintained — no `HashMap`, `HashSet`, `f32`, `f64` introduced.
4. Conservation invariants remain intact.

## Test Plan

### New/Modified Tests

1. If undamped loops are found and fixed: add regression tests proving the dampener limits amplification.

### Commands

1. `cargo test --workspace` — verify no regressions from any code changes
2. `cargo clippy --workspace` — lint check
