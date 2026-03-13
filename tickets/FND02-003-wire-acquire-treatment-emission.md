# FND02-003: Wire AcquireCommodity(Treatment) Goal Emission in Candidate Generation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate_generation.rs
**Deps**: Phase 2 complete

## Problem

`GoalKind::AcquireCommodity { commodity, purpose: CommodityPurpose::Treatment }` is defined in `crates/worldwake-core/src/goal.rs` and already ranked in `crates/worldwake-ai/src/ranking.rs` (lines ~111-121, priority derives from pain pressure), but no emission function exists in `crates/worldwake-ai/src/candidate_generation.rs`. Wounded agents cannot autonomously decide to seek medicine/treatment commodities. The Treatment acquisition path is defined but unreachable through candidate generation.

## Assumption Reassessment (2026-03-13)

1. `CommodityPurpose::Treatment` exists in `goal.rs` — confirmed (lines ~7-12).
2. `ranking.rs` handles `AcquireCommodity { purpose: Treatment }` with pain-pressure-derived priority — confirmed.
3. `candidate_generation.rs` has `emit_combat_candidates()` (line ~134) but no treatment/medicine-seeking emission — confirmed.
4. `WoundList` component exists on agents with `wounds: Vec<Wound>` — confirmed via `wounds.rs`.
5. Wound severity uses `Permille` — confirmed, no floats.
6. Need to determine: what commodity kind(s) qualify as "treatment" — likely requires checking recipe registry for healing recipes or a specific `CommodityKind` variant. Check if `CommodityKind` has a medical/treatment variant.

## Architecture Check

1. Follows the same candidate emission pattern as needs-based goals — checks for a condition (wounds exist), then emits acquisition goal. Mirrors `emit_need_candidates()` structure.
2. No backwards-compatibility shims — new emission path with no existing behavior changes.

## What to Change

### 1. Add wound-aware treatment candidate emission function

New function `emit_treatment_candidates()` in `candidate_generation.rs`:

**Emission conditions** (all must be true):
- Agent or co-located entity has active wounds (check `WoundList` component via belief view).
- Agent does not currently hold sufficient treatment commodity.
- A treatment commodity kind exists in the item system.

**Self-treatment priority**: When the wounded entity is the agent itself, the existing pain/danger pressure derivation in `ranking.rs` naturally elevates priority — no special handling needed in emission.

**For co-located wounded entities**: Emit at lower priority (the ranking system handles this through motive scoring).

**Evidence**: The wounded entity (self or other).

### 2. Wire into candidate generation pipeline

Call `emit_treatment_candidates()` from `generate_candidates_with_travel_horizon()`, in the appropriate position alongside existing emission functions (after combat candidates, since wounds result from combat or deprivation).

### 3. Determine treatment commodity mapping

Identify which `CommodityKind` values are treatment-applicable. This may require:
- A check against `RecipeRegistry` for recipes that produce healing effects.
- Or a hardcoded treatment commodity kind if one exists.
- Document the chosen approach clearly in code comments.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `emit_treatment_candidates()` and call site)

## Out of Scope

- Do NOT implement healing action handlers — those exist in combat system.
- Do NOT modify `GoalKind` enum, `CommodityPurpose`, or `ranking.rs`.
- Do NOT modify wound tracking, needs system, or combat system.
- Do NOT add new `CommodityKind` variants — use existing ones.
- Do NOT touch `worldwake-core`, `worldwake-sim`, or `worldwake-systems` crates.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: Agent with active wounds and no treatment commodity emits `AcquireCommodity { commodity, purpose: Treatment }` candidate.
2. Unit test: Unwounded agent with no wounded co-located entities does not emit treatment candidates.
3. Unit test: Agent already holding sufficient treatment commodity does not emit treatment candidates.
4. Existing suite: `cargo test -p worldwake-ai`
5. Full suite: `cargo test --workspace`

### Invariants

1. All evidence uses `BTreeSet<EntityId>` — no `HashSet`.
2. No `f32`/`f64` in any new code — use `Permille` and integer types.
3. Existing combat and needs candidate emission behavior unchanged.
4. Deterministic output — same inputs produce same candidates in same order.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (or test module) — unit tests for treatment emission with/without wounds, with/without treatment commodity, with co-located wounded entities.

### Commands

1. `cargo test -p worldwake-ai -- treatment` — targeted treatment-related tests
2. `cargo test -p worldwake-ai` — full AI crate suite
3. `cargo clippy --workspace` — lint check
4. `cargo test --workspace` — full workspace suite
