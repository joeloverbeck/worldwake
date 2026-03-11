# E13DECARC-008: Priority class assignment, motive scoring, and candidate ranking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — AI-layer logic
**Deps**: E13DECARC-004, E13DECARC-006, E13DECARC-007

## Problem

After candidate generation, goals must be ranked by priority class and motive score. Priority class comes from the drive band, motive score from `weight * pressure`. Enterprise goals use a candidate-specific `opportunity_signal`. Hard cap rules prevent enterprise/loot/burial from outranking critical survival needs.

## Assumption Reassessment (2026-03-11)

1. `GoalPriorityClass` and `RankedGoal` live in `worldwake-ai`; `GroundedGoal` is now evidence-only.
2. `UtilityProfile` with per-drive weights from E13DECARC-002 — dependency.
3. `classify_band()` from E13DECARC-006 — dependency.
4. `DemandMemory`, `MerchandiseProfile` exist in `worldwake-core` — confirmed.
5. `Permille.value()` exists for extracting `u16`.

## Architecture Check

1. Ranking is deterministic: priority class -> motive score -> cheapest ticks -> GoalKind discriminant -> entity/commodity/place ids.
2. No `HashMap`/`HashSet` — `BTreeMap` or sorted `Vec`.
3. `opportunity_signal` is derived per-candidate from stock deficit, demand memory, and reachable paths — never a stored global score.
4. Hard cap rules are explicit: Critical survival/danger blocks enterprise/loot/burial from outranking.

## What to Change

### 1. Implement ranking as a separate pass

In a new `worldwake-ai/src/ranking.rs` module, add:

```rust
pub fn rank_candidates(
    candidates: &[GroundedGoal],
    view: &dyn BeliefView,
    agent: EntityId,
    utility: &UtilityProfile,
) -> Vec<RankedGoal>
```

This pass consumes grounded candidates and returns ranked candidates. Candidate generation must remain evidence-only.

### 2. Implement priority class assignment

Add priority class logic:

- Self-care goals: use `classify_band(drive_value, threshold_band)`
- `ReduceDanger`: use `classify_band(danger_pressure, thresholds.danger)`
- `Heal`: use pain band, promote by one class if danger is also High or Critical
- Enterprise goals (`RestockCommodity`, `ProduceCommodity`, `SellCommodity`, `MoveCargo`): capped at `Medium`
- `LootCorpse`, `BuryCorpse`: capped at `Low`

### 3. Implement hard cap rules

- If any self-care drive or danger is `Critical`, enterprise/loot/burial goals may not outrank it
- If danger is `High`, loot and burial candidates are suppressed entirely
- If any self-care drive is `High`, loot and burial are suppressed entirely

### 4. Implement motive scoring

```rust
// Self-care:
motive_score = relevant_weight.value() as u32 * relevant_pressure.value() as u32

// ReduceDanger:
motive_score = danger_weight.value() as u32 * danger_pressure.value() as u32

// Heal:
motive_score = pain_weight.value() as u32 * pain_pressure.value() as u32
// plus danger contribution if danger > 0

// Enterprise:
motive_score = enterprise_weight.value() as u32 * opportunity_signal.value() as u32
```

### 5. Implement opportunity_signal derivation

Per-candidate, derived from:
- current stock deficit at sale point
- retained `DemandMemory` for that commodity
- known reachable replenishment path
- whether agent already controls needed inputs

Returns `Permille` — no stored global enterprise score.

### 6. Implement deterministic tie-breaking

Sort candidates by:
1. `GoalPriorityClass` (highest first)
2. `motive_score` (highest first)
3. `GoalKind` discriminant (for determinism)
4. commodity/entity/place ids in lexicographic order

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (new — ranking pass over grounded candidates)
- `crates/worldwake-ai/src/lib.rs` (modify — export `RankedGoal` / ranking API if needed)
- `crates/worldwake-ai/src/pressure.rs` (modify only if a reusable helper belongs there)

## Out of Scope

- Plan cost (`total_estimated_ticks`) as a tiebreaker — that requires plan search (E13DECARC-012)
- Plan search itself — E13DECARC-012
- Plan selection logic — E13DECARC-012

## Acceptance Criteria

### Tests That Must Pass

1. Self-care goal with hunger at critical band gets `GoalPriorityClass::Critical`
2. Enterprise goal capped at `Medium` even when opportunity is high
3. `LootCorpse` capped at `Low`
4. Loot/burial suppressed when danger is `High`
5. Loot/burial suppressed when any self-care drive is `High`
6. Enterprise goals cannot outrank `Critical` survival
7. Motive score for hunger = `hunger_weight.value() * hunger_pressure.value()` (u32 product)
8. Two candidates in same priority class sort by motive_score descending
9. Identical motive scores break ties by `GoalKind` discriminant
10. Opportunity signal is `Permille(0)` when no demand memory and no stock deficit
11. Ranking is fully deterministic (same inputs -> same output order)
12. Existing suite: `cargo test --workspace`

### Invariants

1. No stored enterprise/opportunity score — derived per-candidate
2. No `HashMap`/`HashSet` in ranking
3. Hard cap rules are absolute — no exceptions
4. Tie-breaking order is deterministic and documented

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — tests for priority assignment, hard caps, motive scoring, tie-breaking, and opportunity signal

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
