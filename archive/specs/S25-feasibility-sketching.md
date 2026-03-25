**Status**: ✅ COMPLETED

# S25: Feasibility Sketching

## Summary

Add a cheap feasibility pre-check to the candidate ranking pipeline that estimates whether a goal is locally actionable before committing full GOAP search budget. Goals with low feasibility are demoted within their priority class, not excluded — preserving the principle that goals are desired world conditions, not privileged solutions, while preventing wasted search effort on provably unreachable goals.

## Phase

Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 2)

## Crate

`worldwake-ai`

## Dependencies

- S20 (AI pipeline structural cleanup — cleaner module boundaries simplify insertion of the feasibility stage) — **COMPLETED**
- S23 (Refined blocked intents — compound-keyed `BTreeMap<BlockerKey, BlockedIntent>` with `PatienceExhausted`/`AssumptionFailed` facts) — **COMPLETED**
- S22 (Generalized intention frames — `IntentionFrame` with `FrameState::Exhausted` provides frame-exhaustion signals) — **COMPLETED**

All dependencies are met.

## FOUNDATIONS Alignment

- **P18** (Resource-Bounded Practical Reasoning Over Scripts): Agents should reason tractably. Wasting all 4 planning slots on infeasible high-motive goals while obvious actions go unplanned is not bounded reasoning — it is a planner architecture weakness. Feasibility sketching is a bounded-rational heuristic: spend cheap computation to avoid expensive dead ends.
- **P20** (Agent Diversity Through Concrete Variation): Different agents may have different feasibility landscapes. An agent who knows a route sees `Uncertain`; one who does not sees `Unlikely`. The sketch respects per-agent beliefs, not global truth.
- **P12** (World State Is Not Belief State): Feasibility checks use only the agent's `GoalBeliefView`, never authoritative world state. An agent with false beliefs about a route gets `Likely` or `Uncertain` based on what they believe, not what is true.

## Motivation

The current pipeline generates candidates, ranks them by `(GoalPriorityClass, motive_score)`, and searches the top `max_candidates_to_plan` (default 4) with full GOAP. If the highest-motive goal requires traveling to an unknown location, or the target is known-dead, or blocker memory says "this is blocked for N more ticks," the search expands many nodes finding nothing while a directly-actionable lower-motive goal sits unsearched.

**Example**: Agent has critical hunger (motive 900) for food at a place they cannot reach (no adjacent path known) AND critical hunger (motive 600) for food at their current location. Currently, the unreachable goal takes planning slot 1 and wastes the GOAP search budget. With feasibility sketching, the local food goal is searched first because it is `Likely` while the unreachable one is `Unlikely` — both within the same `Critical` priority class.

**Non-goal**: This spec does not exclude any goal from search. An `Unlikely` goal is still searched if budget permits. The only change is ordering within priority classes.

## Design

### FeasibilityHint Enum

```rust
/// Cheap pre-GOAP estimate of whether a goal is locally actionable.
/// Used to reorder candidates within the same `GoalPriorityClass` —
/// never to exclude goals from search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum FeasibilityHint {
    /// Direct affordance exists at current location, or one-step plan is obvious.
    Likely,
    /// Cannot determine feasibility cheaply — needs full GOAP search.
    Uncertain,
    /// Blocker memory, exhausted frame, or missing prerequisites strongly suggest infeasibility.
    Unlikely,
}
```

The `Ord` derivation gives `Likely < Uncertain < Unlikely` (enum variant order). The sorting comparator uses natural order so `Likely` sorts first within a priority class.

### Feasibility Sketch Function

```rust
/// Derive a cheap feasibility estimate for a ranked goal using only the
/// agent's beliefs, blocker memory, and intention frame state.
/// Never touches authoritative world state.
pub fn feasibility_hint(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    goal: &RankedGoal,
    blocked_memory: &BlockedIntentMemory,
    current_frame: Option<&IntentionFrame>,
    current_tick: Tick,
) -> FeasibilityHint
```

The function takes `GoalBeliefView` (the same trait used by `rank_candidates`), not `RuntimeBeliefView` or `Topology`. It additionally takes the agent's current `IntentionFrame` (read-only, already available in `process_agent()`). This ensures it operates strictly within the agent's belief boundary.

### Two-Phase Check Architecture

Feasibility evaluation uses a two-phase architecture: **shared checks** that apply to all `GoalKind` variants, followed by a **per-GoalKind dispatch table** that returns `Option<FeasibilityHint>`.

```rust
fn feasibility_hint(...) -> FeasibilityHint {
    // Phase 1: Shared checks (short-circuit on first conclusive result)
    if let Some(hint) = check_exhausted_frame(goal, current_frame) { return hint; }
    if let Some(hint) = check_blocker_memory(goal, blocked_memory, current_tick) { return hint; }

    // Phase 2: Per-GoalKind dispatch
    if let Some(hint) = goal_specific_feasibility(view, agent, goal) { return hint; }

    // Phase 3: Default
    FeasibilityHint::Uncertain
}
```

#### Phase 1: Shared Checks

**Check 1 — Exhausted IntentionFrame**: If `current_frame` has `state == FrameState::Exhausted` and `frame.goal == goal.grounded.key` → `Unlikely`. An exhausted frame means the agent already committed patience budget to this exact goal and the frame's assumptions broke or patience ran out. Suspended frames are NOT treated as Unlikely — suspension indicates interruption by higher-priority needs, not infeasibility.

**Check 2 — Blocker memory scan**: Iterate `blocked_memory.intents.values()` for any entry where `blocker_key.goal_key == goal.grounded.key` and `expires_tick > current_tick`. Any live blocker → `Unlikely`. This catches all `BlockingFact` variants:
- Hard blockers (`blocks_goal_generation() == true`): `NoKnownPath`, `NoKnownSeller`, `TargetGone`, `CombatTooRisky`, `DangerTooHigh`, `TooExpensive`, `SellerOutOfStock`, `MissingTool`, `MissingInput`, `WorkstationBusy`, `ReservationConflict`, `Unknown`, `PatienceExhausted`, `AssumptionFailed`
- Soft blockers (`blocks_goal_generation() == false`): `SourceDepleted`, `ExclusiveFacilityUnavailable`

Note: Goals fully suppressed by hard blockers at candidate generation will not reach feasibility checking. However, goals re-generated through different evidence (e.g., a new seller appeared at a different place while the old place is still blocked) will reach here, and place-scoped blockers signal risk even for a different evidence path.

#### Phase 2: Per-GoalKind Dispatch Table

```rust
fn goal_specific_feasibility(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    goal: &RankedGoal,
) -> Option<FeasibilityHint>
```

This function dispatches on `goal.grounded.key.kind` via a `match` expression. Each arm returns `Option<FeasibilityHint>` — `None` means "no goal-specific opinion, fall through to `Uncertain`." New `GoalKind` variants added by future specs automatically get `None` (Uncertain) until a feasibility check is written.

| GoalKind | Check | Result |
|----------|-------|--------|
| `ConsumeOwnedCommodity { commodity }` | `view.commodity_quantity(agent, commodity) > Quantity::ZERO` | `Likely` |
| `AcquireCommodity { .. }` | `evidence_places` contains agent's `effective_place` | `Likely` |
| `Sleep` | Always local (no target entity or place required) | `Likely` |
| `Relieve` | Always local | `Likely` |
| `Wash` | `view.commodity_quantity(agent, CommodityKind::Water) > Quantity::ZERO` | `Likely` |
| `EngageHostile { target }` | Target co-located → `Likely`; target believed dead → `Unlikely` | |
| `ReduceDanger` | No specific check (general intent) | `None` |
| `TreatWounds { patient }` | Patient co-located → `Likely`; patient believed dead → `Unlikely` | |
| `ProduceCommodity { .. }` | `evidence_places` contains agent's `effective_place` (workstation is here) | `Likely` |
| `SellCommodity { commodity }` | Agent possesses commodity AND `evidence_places` local → `Likely`; no commodity → `Unlikely` | |
| `RestockCommodity { .. }` | `evidence_places` contains agent's `effective_place` (supplier is here) | `Likely` |
| `MoveCargo { commodity, destination }` | Agent possesses commodity → `Likely` if destination is adjacent; no commodity → `None` | |
| `LootCorpse { corpse }` | `evidence_places` contains agent's `effective_place` (corpse is here) | `Likely` |
| `BuryCorpse { corpse, burial_site }` | Corpse co-located AND burial_site is current place or adjacent → `Likely` | |
| `ShareBelief { listener, .. }` | Listener co-located → `Likely`; listener believed dead → `Unlikely` | |
| `ClaimOffice { .. }` | `evidence_places` contains agent's `effective_place` | `Likely` |
| `SupportCandidateForOffice { candidate, .. }` | Candidate co-located → `Likely`; candidate believed dead → `Unlikely` | |

**Co-location check pattern**: For goal kinds requiring co-location with a specific entity (EngageHostile, TreatWounds, ShareBelief, SupportCandidateForOffice), the check is:
```rust
let agent_place = view.effective_place(agent)?;
let target_place = view.effective_place(target)?;
if agent_place == target_place { return Some(FeasibilityHint::Likely); }
if view.is_dead(target) { return Some(FeasibilityHint::Unlikely); }
None
```
This is O(1) — two lookup calls.

**Evidence-place check pattern**: For goal kinds where the relevant resource/entity is tracked via `evidence_places` (AcquireCommodity, ProduceCommodity, RestockCommodity, LootCorpse, ClaimOffice), the check is:
```rust
let agent_place = view.effective_place(agent)?;
if goal.grounded.evidence_places.contains(&agent_place) {
    return Some(FeasibilityHint::Likely);
}
None
```

**Cost**: Each call performs at most one `effective_place` lookup, one `commodity_quantity` lookup, one `is_dead` call, and one linear scan of `BlockedIntentMemory.intents`. All are O(1) or O(small) relative to the GOAP search budget of 512 node expansions.

### Integration with Ranking

After `rank_candidates()` produces a `RankingOutcome`, apply `feasibility_hint()` to each `RankedGoal` and re-sort. The new comparator becomes:

```
Within the same GoalPriorityClass:
  1. Likely goals   (sorted by motive_score descending)
  2. Uncertain goals (sorted by motive_score descending)
  3. Unlikely goals  (sorted by motive_score descending)
```

Goals do NOT cross priority class boundaries. A `Critical + Unlikely` goal still outranks a `Low + Likely` goal. Feasibility only reorders within the same priority class.

**Implementation approach**: Add a `feasibility: FeasibilityHint` field to `RankedGoal`. Compute it after `rank_candidates()` returns, then re-sort using an updated `compare_ranked_goals()` that inserts the feasibility comparison between `priority_class` and `motive_score`:

```rust
fn compare_ranked_goals(left: &RankedGoal, right: &RankedGoal) -> Ordering {
    right.priority_class.cmp(&left.priority_class)
        .then_with(|| left.feasibility.cmp(&right.feasibility))  // Likely < Uncertain < Unlikely
        .then_with(|| right.motive_score.cmp(&left.motive_score))
        .then_with(|| goal_kind_discriminant(left.grounded.key.kind)
            .cmp(&goal_kind_discriminant(right.grounded.key.kind)))
        .then_with(|| left.grounded.key.commodity.cmp(&right.grounded.key.commodity))
        .then_with(|| left.grounded.key.entity.cmp(&right.grounded.key.entity))
        .then_with(|| left.grounded.key.place.cmp(&right.grounded.key.place))
}
```

Since `rank_candidates()` also calls `sort_unstable_by(compare_ranked_goals)` and feasibility is initialized to `Uncertain`, the initial sort is unaffected (all equal on feasibility). The re-sort after annotation is the one that reorders.

### Integration Point in agent_tick

The feasibility annotation happens in `process_agent()` (`agent_tick/mod.rs`), **after** the deferred `NoCriticalThreat` assumption evaluation (line ~439) and **before** the active-action phase that consumes `ranked_candidates` (line ~441). This placement ensures:

1. `rank_candidates()` has already produced the initial ranking (line ~386-403)
2. The deferred `NoCriticalThreat` evaluation has updated `current_frame` state if needed (lines ~407-439), so exhausted frames are visible to the feasibility check
3. The annotated and re-sorted `ranked_candidates` is consumed by `build_candidate_plans()` which does `.take(max_candidates_to_plan)`

```rust
// ── Feasibility annotation and re-sort ──
let mut ranked_candidates = ranked_candidates;
{
    let view = runtime_belief_view(agent, ctx.world, ctx.scheduler, action_defs);
    for ranked in &mut ranked_candidates {
        ranked.feasibility = feasibility_hint(
            &view, agent, ranked, &blocked_memory,
            current_frame.as_ref(), tick,
        );
    }
    ranked_candidates.sort_by(compare_ranked_goals);
}
```

The `runtime_belief_view()` call is cheap (constructs references, no allocation). A `PerAgentBeliefView` is used, which implements `GoalBeliefView`.

### Decision Trace Integration

Add a `feasibility: FeasibilityHint` field to `RankedGoalSummary` in `decision_trace.rs`. The `dump_agent()` output will show feasibility alongside priority class and motive for each candidate, making it visible why ordering changed. The `summary()` output mentions feasibility only when it is not `Uncertain` (to avoid noise in the common case).

### Budget Allocation (Future Extension, Not in Scope)

A future spec could give `Unlikely` goals a reduced `max_node_expansions` or `beam_width`. This spec only reorders candidates. The budget knob is noted here for design continuity but is explicitly out of scope.

## Tickets

### S25-001: Add FeasibilityHint enum, dispatch table, and feasibility_hint() function

- Create `feasibility.rs` module in `worldwake-ai/src/`
- Define `FeasibilityHint` enum with `Likely`, `Uncertain`, `Unlikely` (derive `Serialize`, `Deserialize`, `Ord`, etc.)
- Implement `feasibility_hint()` with two-phase architecture:
  - Phase 1: shared checks (exhausted frame, blocker memory scan)
  - Phase 2: `goal_specific_feasibility()` match dispatch on all 17 `GoalKind` variants
- All checks use `GoalBeliefView` + `BlockedIntentMemory` + `IntentionFrame`, never authoritative world state
- Add `feasibility: FeasibilityHint` field to `RankedGoal` in `goal_model.rs` (default `Uncertain` for backward compat in tests that construct `RankedGoal` directly)
- Re-export `FeasibilityHint` from `lib.rs`
- **Verify**: Focused unit tests with mock `GoalBeliefView` covering each check path:
  - Exhausted frame match → Unlikely
  - Active blocker for goal → Unlikely
  - ConsumeOwnedCommodity with possessed commodity → Likely
  - Sleep / Relieve → Likely
  - Wash with Water → Likely; without → Uncertain
  - EngageHostile co-located → Likely; target dead → Unlikely
  - TreatWounds co-located → Likely; patient dead → Unlikely
  - ShareBelief co-located → Likely; listener dead → Unlikely
  - ClaimOffice with evidence at current place → Likely
  - SellCommodity without commodity → Unlikely
  - Default (no opinion) → Uncertain
- **Verify**: `cargo test -p worldwake-ai` — existing tests compile and pass (new field initialized to `Uncertain` where needed)

### S25-002: Integrate feasibility into candidate ordering

- In `process_agent()` in `agent_tick/mod.rs`, after the deferred NoCriticalThreat evaluation (~line 439), annotate each ranked candidate with `feasibility_hint()` and re-sort
- Update `compare_ranked_goals()` in `ranking.rs` to include feasibility between priority_class and motive_score
- Re-sort the ranked list with the updated comparator
- **Verify**: `cargo test -p worldwake-ai` — all golden tests pass. Some may show changed tick counts if agents now find food faster; verify new behavior is strictly better (agent acts more sensibly)

### S25-003: Add feasibility to decision traces

- Add `feasibility: FeasibilityHint` field to `RankedGoalSummary` in `decision_trace.rs`
- Populate during trace construction in `summarize_ranked_goal()` in `agent_tick/planning.rs`
- Update `dump_agent()` format string to include feasibility hint per candidate
- Update `summary()` output to mention feasibility if non-`Uncertain`
- **Verify**: Enable tracing in one golden test, confirm feasibility hints appear in trace output

### S25-004: Golden test verification and documentation

- Run all golden tests: `cargo test -p worldwake-ai --test golden_*`
- For any test where behavior changes (different tick counts, different action sequences), verify the new behavior is an improvement and document the change in a brief comment in the test
- If any test regresses (agent does something worse), investigate — the feasibility check may have a false positive/negative that needs correction
- **Verify**: All golden tests pass, no regressions

### S25-005: Workspace verification

- `cargo test --workspace` — all pass
- `cargo clippy --workspace` — no new warnings
- **Verify**: Clean CI

## FND-01 Section H Analysis

### Information-path analysis

Feasibility sketching introduces no new information paths. It reads the agent's existing `GoalBeliefView` (effective place, commodity quantities, entity liveness), `BlockedIntentMemory` (compound-keyed intents from S23), and `IntentionFrame` (frame state from S22) — all of which are already populated by the perception, failure-handling, and frame-evaluation systems. No agent gains information it would not otherwise have.

### Positive-feedback analysis

None. Feasibility hints are stateless derived computations computed fresh each tick. They do not create new state, do not feed back into candidate generation, do not alter blocker memory, and do not modify intention frames. A goal demoted by `Unlikely` this tick may be `Likely` next tick if the agent moves, a blocker expires, or a frame clears. There are no amplifying loops.

### Concrete dampeners

N/A — no positive-feedback loops to dampen.

### Stored state vs. derived read-model list

- **Stored**: None. `FeasibilityHint` is a transient annotation computed fresh each tick from existing beliefs, blocker memory, and frame state. It is not persisted in any component or serialized to save files.
- **Derived**: `FeasibilityHint` per `RankedGoal` (derived from `GoalBeliefView` reads + `BlockedIntentMemory` scan + `IntentionFrame` state check).

## Verification

1. `cargo test --workspace` — all pass
2. `cargo clippy --workspace` — no new warnings
3. Decision traces show feasibility hints per candidate when tracing is enabled
4. Agents no longer waste all planning slots on unreachable goals in scenarios where local alternatives exist
5. No goal is permanently excluded — `Unlikely` goals are still searched if budget permits (they appear after `Likely` and `Uncertain` goals within the same priority class)

## Outcome

- **Completion date**: 2026-03-25
- **What changed**: Added `feasibility.rs` module in `worldwake-ai` with `FeasibilityHint` enum (Likely/Uncertain/Unlikely), two-phase `feasibility_hint()` function (shared checks + per-GoalKind dispatch), `feasibility` field on `RankedGoal`, updated `compare_ranked_goals()` ordering, integration in `process_agent()` after frame evaluation, and `feasibility` field in `RankedGoalSummary` for decision traces.
- **Deviations from plan**: None — all 5 tickets (S25-001 through S25-005) implemented as specified.
- **Verification**: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace` all pass clean.
