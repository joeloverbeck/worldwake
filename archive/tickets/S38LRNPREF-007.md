# S38LRNPREF-007: Source reliability discount in opportunity ranking

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — ranking pipeline in worldwake-ai
**Deps**: S38LRNPREF-001, S38LRNPREF-003, S38LRNPREF-005

## Problem

All agents rank commodity sources identically regardless of personal trade/harvest history. After this ticket, agents with source failure experience discount unreliable sources' motive scores, making reliable alternatives more attractive. This is a tie-breaking influence within priority class, never a suppression mechanism.

## Assumption Reassessment (2026-04-02)

1. Ranking pipeline at `crates/worldwake-ai/src/ranking.rs:101-111`: sequence is `ranked_motive_score()` → `apply_competition_discount()` → final `RankedGoal`.
2. `ranked_motive_score` at line 142 returns `u32` motive score.
3. `apply_competition_discount` at line 163 returns `Option<CompetitionDiscount>` — adjusts motive post-computation.
4. Source reliability discount inserts between these two steps (or alongside `apply_competition_discount`) as a separate discount step.
5. `RankingContext` at ranking.rs provides `view: &dyn GoalBeliefView` — gives access to `source_reliability()` and `preference_profile()` after S38LRNPREF-003.
6. `GroundedGoal` contains `GoalKind`, `OpportunityAnchor`, and `evidence_entities`. For commodity acquisition goals, the concrete source entity is derived from `evidence_entities`, while the anchor remains place-scoped for routing/planning.
7. `GoalKind` variants like `RestockCommodity` and `AcquireCommodity` contain commodity information. The live source entity path for ranking is the grounded candidate's single concrete evidence entity, not the place anchor.
8. Motive must never drop below 1 after discount (spec requirement).

## Architecture Check

1. A separate `apply_source_reliability_discount` function mirrors the existing `apply_competition_discount` pattern — both are post-motive adjustments with optional trace structs. Clean, parallel structure.
2. Source reliability discount and competition discount are independent adjustments — they compose cleanly (both reduce motive from the same base score).
3. No backward-compatibility shims. Agents without `PreferenceProfile` or `SourceReliability` skip the discount entirely.

## Verification Layers

1. No experience → no discount applied → focused unit test
2. No `PreferenceProfile` → no discount applied → focused unit test
3. Source with failures → proportional motive discount → focused unit test with known failure ratio
4. Motive never drops below 1 → focused unit test with extreme failure ratio
5. Non-commodity goals unaffected → focused unit test
6. Ranking-sensitive ticket: verify discount arithmetic against live ranking substrate. Discount applies after motive score, before final RankedGoal construction. Does not affect priority class.

## What to Change

### 1. New discount function

Add `fn apply_source_reliability_discount(candidate: &GroundedGoal, context: &RankingContext<'_>, motive_score: u32) -> Option<SourceReliabilityDiscount>` to `ranking.rs`:

1. Extract source entity from `candidate.evidence_entities` and commodity from `GoalKind`. Return `None` if not a commodity-acquisition goal or the grounded candidate does not resolve to exactly one concrete source entity.
2. Look up `SourceReliability` and `PreferenceProfile` from `context.view`. Return `None` if either absent.
3. Look up `ReliabilityRecord` for the `SourceKey`. Return `None` if no experience.
4. Compute `failure_ratio_permille`: `failed_attempts as u32 * 1000 / (successful_acquisitions + failed_attempts) as u32`.
5. Compute `adjusted_motive = motive * (1000 - source_trust_weight.value() as u32 * failure_ratio / 1000) / 1000`.
6. Clamp to `max(adjusted_motive, 1)`.
7. Return `SourceReliabilityDiscount` trace struct.

### 2. Trace struct for debuggability

Add `SourceReliabilityDiscount` struct (analogous to `CompetitionDiscount`) with fields: `source_entity`, `commodity`, `failure_ratio_permille`, `pre_discount_motive`, `post_discount_motive`.

### 3. Integrate into ranking pipeline

In the main ranking loop (line ~101), call `apply_source_reliability_discount` and apply the adjusted motive to the `RankedGoal` construction, alongside the existing competition discount.

### 4. Helper function for failure ratio computation

Add `fn failure_ratio_permille(record: &ReliabilityRecord) -> u32` as a pure function for testability. Returns 0 if total attempts is 0.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — new discount function, trace struct, pipeline integration)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `SourceReliabilityDiscount` trace struct, if traces are defined there)
- `crates/worldwake-core/src/experience.rs` (modify — add `failure_ratio_permille` helper)

## Out of Scope

- Route cost penalty (S38LRNPREF-006)
- Experience recording in action handlers (S38LRNPREF-004, 005)
- Golden tests (S38LRNPREF-008)
- Interaction between source reliability discount and competition discount (they compose independently)

## Acceptance Criteria

### Tests That Must Pass

1. Non-commodity goal → no discount applied
2. Commodity goal, no `SourceReliability` → no discount applied
3. Commodity goal, no `PreferenceProfile` → no discount applied
4. Commodity goal with 50% failure ratio → proportional motive discount
5. Commodity goal with 100% failure ratio → maximum discount, motive >= 1
6. Commodity goal with 0% failure ratio → no discount
7. Source reliability discount composes correctly with competition discount
8. Integer arithmetic produces correct results at Permille boundaries
9. Existing suite: `cargo test --workspace`

### Invariants

1. Agents without `PreferenceProfile` behave identically to pre-spec behavior
2. Motive never drops below 1 after source discount
3. All arithmetic is integer — no floats (determinism)
4. Source discount never changes priority class — only adjusts motive within class
5. `SourceReliabilityDiscount` trace struct enables debuggability (P29)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (new focused tests) — source reliability discount with various failure ratios, no-experience passthrough, no-profile passthrough, motive floor at 1, composition with competition discount
2. `crates/worldwake-core/src/experience.rs` (modify) — `failure_ratio_permille` unit tests with boundary values

### Commands

1. `cargo test -p worldwake-ai ranking`
2. `cargo test -p worldwake-core experience`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed: 2026-04-02

- Added the shared `failure_ratio_permille` helper in `crates/worldwake-core/src/experience.rs` and re-exported it from `crates/worldwake-core/src/lib.rs`.
- Added `SourceReliabilityDiscount` and threaded it through `RankedGoal`, `RankedGoalSummary`, decision-trace formatting, and planning summary handoff in `crates/worldwake-ai/src/decision_trace.rs`, `crates/worldwake-ai/src/goal_model.rs`, and `crates/worldwake-ai/src/agent_tick/planning.rs`.
- Implemented `apply_source_reliability_discount` in `crates/worldwake-ai/src/ranking.rs`, using the grounded candidate's single concrete `evidence_entities` source plus the actor's `SourceReliability` and `PreferenceProfile` to discount `AcquireCommodity` and `RestockCommodity` motive scores without changing priority class.
- Added focused coverage for no-experience passthrough, no-profile passthrough, proportional discounting, floor-at-one behavior, zero-failure passthrough, composition with competition discount, helper boundary values, and trace-summary preservation.

Deviations from original plan:

- The ticket's source-entity assumption was corrected before implementation: the live ranking boundary derives the concrete source entity from `GroundedGoal.evidence_entities`, not from `OpportunityAnchor`.
- The implementation also updated sibling AI test constructors and trace literals outside the original `Files to Touch` list so the new optional discount field propagated cleanly through the existing shared ranked-goal and decision-trace surfaces.

Verification results:

- `cargo test -p worldwake-core experience -- --nocapture`
- `cargo test -p worldwake-ai ranking -- --nocapture`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
