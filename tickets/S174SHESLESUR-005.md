# S174SHESLESUR-005: Sleep goal schema — FeasibilityStrategy::CandidateBacked + two-path sleep_rest_opportunities

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `FeasibilityStrategy::CandidateBacked` variant on existing AI-crate enum; rewritten `DECL_SLEEP` goal schema; existing `emit_sleep_goal` replaced with two-path `sleep_rest_opportunities`
**Deps**: 001 (RestCapacity/RestOccupancy types), 002 (rough_sleep_recovery_floor — read by candidate marker), 003 (belief-view accessors)

## Problem

The current Sleep goal schema uses `FeasibilityStrategy::AlwaysLikely` (`crates/worldwake-ai/src/goal_schema.rs:345`) and emits a single sleep candidate per tick via `emit_sleep_goal` (`candidate_generation.rs:4488`). This collapses "sleep at a known rest site" and "sleep rough" into one undifferentiated candidate, so the planner cannot rank shelter quality against rough-sleep fallback. S174 D4 requires splitting Sleep into a two-path enumerator: `KnownRestSite` (belief-backed, higher recovery, possibly contested) and `RoughSleep` (always-available fallback, capped recovery, more interruptible).

The two-path schema requires a new `FeasibilityStrategy::CandidateBacked` variant (the goal is feasible iff at least one lawful candidate exists). The current enum has 10 variants but none with that semantic.

## Assumption Reassessment (2026-05-26)

1. Verified current code: `FeasibilityStrategy` enum at `crates/worldwake-ai/src/goal_schema.rs:38-49` has variants `OwnedCommodityCheck, EvidencePlaceLocal, AlwaysLikely, CommodityPresenceCheck, ColocationOrDead, NoOpinion, SellCheck, CargoDestinationCheck, CorpseBurialCheck, PlaceMatch` — no `CandidateBacked`. `DECL_SLEEP` at lines 340-351 is a `GoalSchema` static (not `GoalDecl`) with `feasibility_strategy: FeasibilityStrategy::AlwaysLikely` on line 345 and `relevant_ops: SLEEP_OPS` where `SLEEP_OPS = &[PlannerOpKind::Sleep]` at line 99. Existing emitter `emit_sleep_goal` at `candidate_generation.rs:4488-4524` is called from line 1282 (`emit_self_consume_candidates(...)` dispatch block). `PlannerOpKind::QueueForFacilityUse` exists at `planner_ops.rs:23` and is reachable.
2. Spec assumption verified against S174 D4 (rewritten during reassessment to introduce `CandidateBacked` as a prerequisite paragraph). The rewrite confirmed `GoalSchema` (not the spec's earlier `GoalDecl` framing) is the actual type and `emit_sleep_goal` is the function being replaced.
3. Shared abstraction boundary under audit: goal schema declaration (`DECL_SLEEP`) + candidate emission (`sleep_rest_opportunities` replacing `emit_sleep_goal`) + feasibility strategy enum extension (`CandidateBacked`). All three changes land together because (a) `CandidateBacked` is consumed by `DECL_SLEEP`'s new declaration, (b) the new emitter requires `CandidateBacked` semantics to function (otherwise an empty candidate set would still mark the goal feasible), (c) splitting them leaves the workspace either broken (DECL_SLEEP referencing a nonexistent variant) or dead-code (CandidateBacked variant unused).
4. Existing inline tests on the affected functions: `candidate_generation.rs` inline tests at `fatigue_and_bladder_emit_sleep_and_relieve:13002`, `sleep_candidate_emission_at_current_place_only:13182`, `action_specific_place_blocker_with_support_target_suppresses_matching_sleep_candidate:11805`. Each will need updates to match the new two-path emission shape — likely splitting "sleep_candidates" expectations into separate KnownRestSite and RoughSleep buckets where the test's intent requires.
5. Live `GoalKind` under test: `GoalKind::Sleep` (the existing variant, unchanged). The exact current operator surface is `SLEEP_OPS` containing only `PlannerOpKind::Sleep`; this ticket extends to `[PlannerOpKind::Sleep, PlannerOpKind::QueueForFacilityUse]` to enable rest-site queueing via the existing S44 substrate.
6. `FeasibilityStrategy` derive analysis: the enum's current derives need checking before adding `CandidateBacked`. The variant has no payload (unit), so derive compatibility is trivial (it inherits all derives from the enum). Verify at ticket-implementation time.
7. Heuristic removal: `FeasibilityStrategy::AlwaysLikely` is no longer used by `DECL_SLEEP` after this ticket. Search for other consumers of `AlwaysLikely`; if no other goal uses it, the variant could potentially be removed per FND-28. However, removal is out of scope for this ticket — verify first whether any other goal schema uses `AlwaysLikely` and, if not, propose removal as a follow-up cleanup ticket.

## Architecture Check

1. The two-path emitter (KnownRestSite + RoughSleep) preserves flat GOAP — no HTN method is registered. Per S174's Planner-formalism analysis, the two-path split lives in the goal schema and candidate enumerator, not in method decomposition. HTN would over-formalize a two-candidate branch.
2. `FeasibilityStrategy::CandidateBacked` is a generally reusable strategy ("feasibility = at least one lawful candidate exists"). No other current goal needs it, but introducing it cleanly handles any future goal whose feasibility is "anything to do" rather than belief-checked or place-checked. This matches FND-28 — a new strategy variant rather than a special-case branch inside `DECL_SLEEP`'s evaluator.
3. The RoughSleep marker (carried via `ActionState::Sleep { rough: true, ... }` or equivalent, per ticket 004's design choice) flows from the emitter to the handler through authoritative action state, not through planner-side metadata. This matches FND-26 — systems interact through state, not direct calls.

## Verification Layers

1. `FeasibilityStrategy::CandidateBacked` constructs and pattern-matches correctly -> focused unit test on the enum
2. `DECL_SLEEP` declares `feasibility_strategy: FeasibilityStrategy::CandidateBacked` and `relevant_ops: SLEEP_OPS` with `SLEEP_OPS = &[PlannerOpKind::Sleep, PlannerOpKind::QueueForFacilityUse]` -> focused unit test on `goal_schema.rs`
3. `sleep_rest_opportunities` KnownRestSite pass emits one candidate per believed-available rest-site place -> focused unit test on `candidate_generation.rs`
4. `sleep_rest_opportunities` RoughSleep pass emits exactly one candidate targeting the actor's current effective place -> focused unit test
5. When KnownRestSite emits zero candidates (no believed rest sites available), RoughSleep candidate is the only output -> focused unit test
6. Sleep candidates carry the rough vs known marker so the handler (ticket 004) can read it -> action-trace assertion via the integration test in ticket 004 (cross-ticket verification)
7. Existing sleep-related candidate-generation tests still pass after replacement (with assertion updates for the new two-path shape) -> existing test regression

## What to Change

### 1. Add `FeasibilityStrategy::CandidateBacked` variant

In `crates/worldwake-ai/src/goal_schema.rs:38-49`, add a new variant to the `FeasibilityStrategy` enum:

```rust
pub enum FeasibilityStrategy {
    OwnedCommodityCheck,
    EvidencePlaceLocal,
    AlwaysLikely,
    CommodityPresenceCheck,
    ColocationOrDead,
    NoOpinion,
    SellCheck,
    CargoDestinationCheck,
    CorpseBurialCheck,
    PlaceMatch,
    CandidateBacked,        // new
}
```

Document the variant: "Goal is feasible iff the dispatch loop produces at least one candidate from any registered emitter (no separate pre-flight feasibility check)."

Grep for exhaustive-match sites on `FeasibilityStrategy` across `worldwake-ai/` (likely 1-3 sites — the feasibility evaluator and possibly a debug renderer). Add a `CandidateBacked` arm to each — semantics: return true iff candidates is non-empty after the dispatch loop completes.

### 2. Extend `SLEEP_OPS`

In `goal_schema.rs:99`, replace:

```rust
const SLEEP_OPS: &[PlannerOpKind] = &[PlannerOpKind::Sleep];
```

with:

```rust
const SLEEP_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Sleep,
    PlannerOpKind::QueueForFacilityUse,
];
```

### 3. Rewrite `DECL_SLEEP`

In `goal_schema.rs:340-351`, replace the existing declaration:

```rust
pub static DECL_SLEEP: GoalSchema = GoalSchema {
    // ... existing fields ...
    feasibility_strategy: FeasibilityStrategy::CandidateBacked,  // was AlwaysLikely
    relevant_ops: SLEEP_OPS,
    // ... preserve other fields unchanged ...
};
```

Preserve all other fields (trace_label, provenance_family, invalidation_strategy, frontier_exhaustion_strategy, family_policy, progress_barrier_ops, candidate_extractors, planning_budget) unchanged.

### 4. Replace `emit_sleep_goal` with `sleep_rest_opportunities`

In `crates/worldwake-ai/src/candidate_generation.rs:4488-4524`, delete the existing `emit_sleep_goal` function. Replace with a new two-pass `sleep_rest_opportunities`:

- **KnownRestSite pass**: for each Place in the actor's belief view with `rest_site_capacity(place).is_some()` AND `rest_site_occupant_count(place) < rest_site_capacity(place)`, emit a Sleep candidate with `target_place = place` and rough-sleep marker `false`. Use the existing `emit_candidate_with_trace` infrastructure to produce `GroundedGoal { kind: GoalKind::Sleep, anchor: OpportunityAnchor::Place(place), ... }`.
- **RoughSleep pass**: emit exactly one Sleep candidate with `target_place = actor.effective_place()` and rough-sleep marker `true`. This candidate is always emitted (regardless of KnownRestSite results) so an actor always has at least one sleep option.

Update the call site at `candidate_generation.rs:1282` — replace `emit_sleep_goal(...)` with `sleep_rest_opportunities(...)`. Preserve the same parameter list (candidates, diagnostics, ctx, needs, thresholds) and side-effect contract (mutating `candidates` and `diagnostics`).

### 5. Update inline tests

The 3 inline tests touching sleep emission (`fatigue_and_bladder_emit_sleep_and_relieve:13002`, `sleep_candidate_emission_at_current_place_only:13182`, `action_specific_place_blocker_with_support_target_suppresses_matching_sleep_candidate:11805`) need updates:

- `sleep_candidate_emission_at_current_place_only` likely needs reframing — under the new schema, sleep at the current place is the RoughSleep branch; if the test's intent is to assert "only one sleep candidate emitted when no other rest site is known," the assertion remains valid but the candidate's marker should be `rough = true`.
- `fatigue_and_bladder_emit_sleep_and_relieve` likely just needs to expect one sleep candidate (the RoughSleep fallback at the agent's current place) rather than the previous single-emission shape.
- `action_specific_place_blocker_with_support_target_suppresses_matching_sleep_candidate` may need a `RestCapacity` annotation on the targeted place to exercise the KnownRestSite suppression path.

Verify each test's intent at ticket-implementation time and update assertions accordingly. Do not adapt tests to bugs — if a test's intent has shifted because of the schema change, document the shift in the test's comment block.

## Files to Touch

- `crates/worldwake-ai/src/goal_schema.rs` (modify — add `FeasibilityStrategy::CandidateBacked` variant, extend `SLEEP_OPS`, rewrite `DECL_SLEEP`)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — replace `emit_sleep_goal` with `sleep_rest_opportunities`, update call site at line 1282, update 3 inline tests at lines 11805, 13002, 13182)
- Likely: exhaustive-match sites on `FeasibilityStrategy` — locate via `grep -rn "match.*FeasibilityStrategy\|FeasibilityStrategy::" crates/worldwake-ai/src/ | grep -v ":use\|: \\*" | head -20`. Typically 1-3 sites; add `CandidateBacked` arm to each.

## Out of Scope

- No `RestCapacity` / `RestOccupancy` component definitions (ticket 001)
- No belief-view accessor implementations (ticket 003)
- No `RestOccupancy` writes at sleep action start (ticket 004)
- No `rough_sleep_recovery_floor` application (ticket 004 reads the floor at sleep-tick)
- No `FailedRestOpportunity` records (ticket 006)
- No `ActionTraceDetail::SleepInterrupted` population (ticket 006)
- No removal of `FeasibilityStrategy::AlwaysLikely` even if it becomes unused after this ticket — proposed as a follow-up cleanup if grep confirms zero other consumers
- No HTN method registration for sleep — flat GOAP per spec Planner-formalism analysis

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test: `FeasibilityStrategy::CandidateBacked` constructs, pattern-matches, and is emitted by the dispatch loop when at least one candidate exists
2. New focused unit test: `DECL_SLEEP` declares `feasibility_strategy: FeasibilityStrategy::CandidateBacked`
3. New focused unit test: `DECL_SLEEP.relevant_ops` contains both `PlannerOpKind::Sleep` and `PlannerOpKind::QueueForFacilityUse`
4. New focused unit test: `sleep_rest_opportunities` KnownRestSite pass emits one candidate per believed-available rest-site Place
5. New focused unit test: `sleep_rest_opportunities` RoughSleep pass always emits one candidate at the actor's current Place, regardless of KnownRestSite output
6. New focused unit test: when no rest sites are believed available, RoughSleep is the only candidate
7. Existing inline tests at `candidate_generation.rs:11805, 13002, 13182` pass with updated assertions
8. Existing suite: `cargo test -p worldwake-ai goal_schema candidate_generation` passes
9. Existing suite: `cargo test --workspace` (full regression — primary risk is sleep_episode goldens and any test depending on single-candidate sleep emission)

### Invariants

1. `DECL_SLEEP` does not use `FeasibilityStrategy::AlwaysLikely` after this ticket
2. Sleep candidates always include a RoughSleep option (or another emission, but never zero candidates when the agent is alive and not in transit)
3. KnownRestSite candidates filter out places where `rest_site_occupant_count >= rest_site_capacity` per the belief view
4. The two-path emitter is flat GOAP; no HTN method is registered for Sleep

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_schema.rs` (extend inline tests) — `CandidateBacked` variant + `DECL_SLEEP` declaration coverage
2. `crates/worldwake-ai/src/candidate_generation.rs` (extend inline tests + modify 3 existing tests at lines 11805, 13002, 13182) — `sleep_rest_opportunities` two-path coverage
3. Likely: `crates/worldwake-ai/src/<feasibility evaluator>.rs` (extend tests) — `CandidateBacked` semantics

### Commands

1. `cargo test -p worldwake-ai goal_schema::tests` (schema coverage)
2. `cargo test -p worldwake-ai candidate_generation::tests` (emitter coverage)
3. `cargo test --workspace` (full regression)
4. `./scripts/verify.sh` (final pre-PR gate)
