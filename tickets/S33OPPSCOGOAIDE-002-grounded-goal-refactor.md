# S33OPPSCOGOAIDE-002: Refactor GroundedGoal to carry OpportunityAnchor and emit per-opportunity

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — GroundedGoal struct change, candidate_generation emission change
**Deps**: S33OPPSCOGOAIDE-001

## Problem

`GroundedGoal` currently has no anchor field. Candidate generation merges all sources for the same `GoalKey` into a single `GroundedGoal` via `BTreeMap<GoalKey, GroundedGoal>` with evidence merging in `emit_candidate()`. This means exhaustion of one source suppresses all sources for the same desire. Each opportunity must be emitted as a separate `GroundedGoal` with its own `OpportunityAnchor`.

## Assumption Reassessment (2026-03-28)

1. `GroundedGoal` is at `crates/worldwake-ai/src/goal_model.rs:1600-1604` with fields `{ key, evidence_entities, evidence_places }`. Confirmed no `anchor` field exists.
2. `emit_candidate()` is at `crates/worldwake-ai/src/candidate_generation.rs:2013-2042`. It takes `BTreeMap<GoalKey, GroundedGoal>` and merges evidence on key collision. Confirmed.
3. `emit_candidate_with_trace()` exists at line 2256 — a traced variant that must also be updated.
4. The `Evidence` struct (internal to candidate_generation.rs, lines 35-64) carries `entities: BTreeSet<EntityId>` and `places: BTreeSet<EntityId>`.
5. Callers of `emit_candidate`: grep shows multiple call sites within `candidate_generation.rs` for acquire, produce, sell, self-care, combat, political, and other goal kinds.
6. `RankedGoal.grounded` is of type `GroundedGoal` (`ranking.rs:1702`). Adding `anchor` to `GroundedGoal` flows through ranking automatically.
7. This is a cross-system ticket (goal_model + candidate_generation). The shared boundary is the `GroundedGoal` struct definition and `emit_candidate()` function signature.
8. No adjacent contradictions found.

## Architecture Check

1. Adding `anchor: OpportunityAnchor` to `GroundedGoal` is the minimal structural change. The alternative — creating a new `OpportunityGoal` wrapper — would duplicate fields and complicate downstream consumers. Direct field addition is cleaner.
2. Changing `BTreeMap<GoalKey, GroundedGoal>` to `Vec<GroundedGoal>` (or `BTreeMap<OpportunityKey, GroundedGoal>`) in candidate collection ensures no evidence merging across different anchors. Using `Vec` is simpler since dedup happens post-rank (S33OPPSCOGOAIDE-005).
3. No backward-compatibility shims — the `Vacant`/`Occupied` evidence-merging logic in `emit_candidate()` is removed entirely (P26).

## Verification Layers

1. `GroundedGoal` has `anchor` field → focused unit test constructing `GroundedGoal` with each anchor variant.
2. Per-opportunity emission (no evidence merging) → focused test: two sources for same `GoalKey` produce two `GroundedGoal` instances with different anchors and disjoint evidence.
3. Self-care goals use `OpportunityAnchor::None` → focused test verifying anchor assignment for eat/drink/sleep.

## What to Change

### 1. Add `anchor` field to `GroundedGoal`

In `crates/worldwake-ai/src/goal_model.rs`, add `pub anchor: OpportunityAnchor` to the struct.

### 2. Rewrite `emit_candidate()` signature and body

Change the candidates collection from `BTreeMap<GoalKey, GroundedGoal>` to `Vec<GroundedGoal>`. The function pushes a new `GroundedGoal` per call instead of merging. Each call site must supply the appropriate `OpportunityAnchor`.

### 3. Update all `emit_candidate()` / `emit_candidate_with_trace()` call sites

- Acquire goals: one call per source place → `OpportunityAnchor::Place(source_place)`.
- Produce goals: one call per workstation place → `OpportunityAnchor::Place(workstation_place)`.
- Sell goals: one call per buyer location → `OpportunityAnchor::Place(buyer_place)`.
- Self-care (eat, drink, sleep, relieve, wash): `OpportunityAnchor::None`.
- Care goals (treat wounds): `OpportunityAnchor::Entity(patient)`.
- Political goals: `OpportunityAnchor::Entity(office)` or `OpportunityAnchor::Place(jurisdiction)`.
- Combat/loot/bury: derive anchor from target entity or target place as appropriate.

### 4. Update all downstream consumers of the candidates collection

Functions that consume `BTreeMap<GoalKey, GroundedGoal>` must accept `Vec<GroundedGoal>` (or `&[GroundedGoal]`). This includes the ranking entry point and the pipeline between generation and ranking.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add `anchor` field to `GroundedGoal`)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — rewrite `emit_candidate`, `emit_candidate_with_trace`, all call sites, change collection type)
- `crates/worldwake-ai/src/ranking.rs` (modify — accept `Vec<GroundedGoal>` or `&[GroundedGoal]` instead of `BTreeMap<GoalKey, GroundedGoal>`)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify — update pipeline between generation and ranking if collection type changes there)

## Out of Scope

- Two-pass blocker filtering (S33OPPSCOGOAIDE-003) — this ticket still passes blocked memory but does NOT restructure the filtering into two passes. The global `is_blocked` check in `emit_candidate` is removed but per-opportunity filtering is added in S33OPPSCOGOAIDE-003.
- Exhaustion re-keying (S33OPPSCOGOAIDE-004)
- Post-rank deduplication (S33OPPSCOGOAIDE-005)
- `PlannedPlan` changes (S33OPPSCOGOAIDE-006)
- Decision trace changes (S33OPPSCOGOAIDE-007)
- Changes to `build_planning_snapshot()` (it already accepts `evidence_entities`/`evidence_places` by reference)

## Acceptance Criteria

### Tests That Must Pass

1. Two sources for `AcquireCommodity(Apple)` produce two `GroundedGoal` instances with `OpportunityAnchor::Place(orchard)` and `OpportunityAnchor::Place(market)` respectively.
2. Each `GroundedGoal` carries evidence scoped to its anchor (not merged).
3. Self-care goals produce `GroundedGoal` with `OpportunityAnchor::None`.
4. `ProduceCommodity` goals produce per-workstation `GroundedGoal` instances.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. No evidence merging across different `OpportunityAnchor` values for the same `GoalKey`.
2. `GroundedGoal.key` still matches the `GoalKind`-derived `GoalKey`.
3. `GroundedGoal.anchor` is always populated (never left as a default).
4. Determinism: candidate emission order is deterministic (BTreeSet-based evidence, deterministic iteration of belief data).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — `test_per_opportunity_emission_acquire` — two sources produce two GroundedGoal instances.
2. `crates/worldwake-ai/src/candidate_generation.rs` — `test_per_opportunity_evidence_isolation` — evidence sets are disjoint per anchor.
3. `crates/worldwake-ai/src/candidate_generation.rs` — `test_self_care_anchor_none` — self-care goals get `OpportunityAnchor::None`.
4. Existing `candidate_generation` tests updated to work with new collection type.

### Commands

1. `cargo test -p worldwake-ai -- candidate_generation`
2. `cargo test -p worldwake-ai -- ranking`
3. `cargo clippy --workspace && cargo test --workspace`
