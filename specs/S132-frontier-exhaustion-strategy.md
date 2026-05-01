# S132: Frontier-Exhaustion Strategy as Goal-Kind Property

## Summary

Replace the per-`GoalKind` enumeration in `frontier_exhaustion_entry`
(`crates/worldwake-ai/src/agent_tick/planning.rs:873`) with a goal-kind
property `frontier_exhaustion_strategy`. The current code permanently
suppresses goals whose plan search returned `FrontierExhausted` *unless*
the goal kind appears in a hand-maintained allow-list (today: `Sleep`,
`AcquireCommodity { purpose: SelfConsume, .. }`, `Patrol`). Two of those
three entries were added reactively as CIREM-002 and CIREM-004 fixes,
each treating "this recurring duty must not be permanently stranded" as
a one-off correction. As more recurring goal classes land
(production duties, social commitments, exploration cycles), the
allow-list will keep growing case-by-case, and the default — permanent
suppression — silently fails the next class to be added.

The substrate should be a property declared at goal-kind registration:
each `GoalKind` (or its dispatch declaration) declares whether
frontier exhaustion is `PermanentUntilInvalidator` or
`CooldownRetry`. The dispatch in `frontier_exhaustion_entry` becomes a
property read, not a variant match. Adding a new goal kind requires
declaring the strategy, not patching a switch.

## Phase and Status

Phase 10 adjunct (post-S129 architectural audit). Status: PENDING.

## Crates

- `worldwake-ai` (planner-internal types and dispatch)

No `worldwake-core`, `worldwake-sim`, or `worldwake-systems` changes are
expected: this spec does not introduce world state, components, events,
actions, or systems.

## Dependencies

- Soft: builds on the lifecycle introduced by S109 (typed discrepancy /
  blocker memory) and S115 (agenda manager) — frontier exhaustion is
  one of the planner outcomes those substrates classify, but no
  contract from those specs is altered.
- No hard cross-spec dependency.

## Design Goals

1. Frontier-exhaustion behavior for a goal kind is declared *with* the
   goal kind, not in a downstream switch.
2. Adding a new `GoalKind` variant fails to compile (or fails a
   coverage test) until the variant declares its strategy.
3. The set of strategies is closed and explainable from the goal-kind
   side: a reader inspecting `GoalKind::Sleep` learns that its frontier
   exhaustion is `CooldownRetry` without reading planner internals.
4. Default-on-unfamiliar-variants is biased toward revisable
   commitments (FND-21): the new substrate makes "permanent suppression
   until concrete invalidation" an explicit, justified declaration, not
   the silent fallthrough.

## Non-Goals

- Changing the cooldown decay arithmetic, baseline computation, or
  invalidation-condition semantics. The retry budget and concrete-state
  invalidators on `ExhaustionEntry::budget_retry_pending` and
  `ExhaustionEntry::frontier_exhausted` keep their current behavior.
- Refactoring `BlockerMemory` / `DiscrepancyMemory` TTL or the agenda
  manager's lifecycle classification. Those are separate substrates and
  are not under audit here.
- Adding new goal-kind classes. This is a refactor of the dispatch for
  the existing classes.

## FOUNDATIONS Alignment

| Principle | Alignment |
|---|---|
| FND-21 (Intentions Are Revisable Commitments) | Today's silent default of permanent suppression strands recurring duties (patrol, self-consume acquisition) until external state changes. The new property surfaces the choice between `PermanentUntilInvalidator` and `CooldownRetry` at declaration time so the agent retains revisable commitments by construction, not by reactive patching. |
| FND-22A (Learning, Habits, Preference Shifts Are Concrete State) | Frontier exhaustion is an agent-local learned blocker record. The strategy property names its scope (when it decays, what invalidates it) at the point where the agent's response to exhaustion is declared. No silent global suppression. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | The dispatch becomes a state read on a goal-kind property, not a switch hidden inside the planning layer. The exhaustion cache continues to be the read state. |
| FND-28 (No Backward Compatibility) | The current `_ => ExhaustionEntry::frontier_exhausted(..)` fallthrough is the live authoritative path. The refactor replaces it; no shim is preserved. |

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: `ExhaustionEntry` is per-agent
   runtime planner state in `AgentDecisionRuntime.exhaustion_cache`.
   It does not propagate to other agents through perception, witness,
   report, or rumor. Aligned with FND-7 — frontier exhaustion is
   strictly the observing agent's own learned blocker record.
2. **Positive-feedback analysis**: None. The strategy declaration
   classifies how a single agent's planner reacts to its own
   exhaustion outcome. There is no amplifying loop where exhaustion
   causes more exhaustion.
3. **Concrete dampeners**: Not applicable (no positive feedback). The
   `BudgetRetryPending` cooldown decay (`ExhaustionEntry::budget_retry_pending`) and the
   `FrontierExhausted` concrete-state invalidators
   (`ExhaustionInvalidationCondition`) remain the governing limits;
   this spec does not add new dampeners.
4. **Stored state vs. derived read-model**: The strategy property is a
   **compile-time constant** declared per `GoalKind` variant — not
   stored runtime state, not persisted. The `ExhaustionEntry` runtime
   state continues to live in `AgentDecisionRuntime.exhaustion_cache`
   (already authoritative planner state, save-format-bumped per
   existing rules when it changes). No new derived view is introduced.

## Deliverables

### D1: `FrontierExhaustionStrategy` enum

```rust
// crates/worldwake-ai/src/exhaustion.rs (or new module under agent_tick/)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierExhaustionStrategy {
    /// On `FrontierExhausted`, install `ExhaustionEntry::frontier_exhausted`
    /// (permanent suppression until a concrete invalidation condition
    /// fires).
    PermanentUntilInvalidator,
    /// On `FrontierExhausted`, install `ExhaustionEntry::budget_retry_pending`
    /// (cooldown-backed retry with the existing decay arithmetic).
    CooldownRetry,
}
```

### D2: Goal dispatch declaration property

Add a field to `GoalDispatchDeclaration`:

```rust
pub frontier_exhaustion_strategy: FrontierExhaustionStrategy,
```

The strategy is read through
`GoalDispatchKey::from_goal_kind(goal).declaration().frontier_exhaustion_strategy`.
`GoalDispatchKey::from_goal_kind` already uses an exhaustive match over the
closed `GoalKind` variants without a `_ => ...` arm, and
`GoalDispatchKey::declaration()` already exhaustively maps every dispatch key
to a declaration. Adding a new variant or payload split must therefore add the
dispatch key and its declaration before the code compiles cleanly. Declarations:

| `GoalKind` variant | Strategy | Rationale |
|---|---|---|
| `Sleep` | `CooldownRetry` | Direct local self-care; permanent suppression strands inside authored critical band (existing decision per `frontier_exhaustion_entry` comment). |
| `AcquireCommodity { purpose: SelfConsume, .. }` | `CooldownRetry` | Recurring self-care substrate; CIREM-002 finding — self-consume frontier exhaustion stranded scattered Agent A at saturated thirst. |
| `AcquireCommodity { purpose: RecipeInput/Restock, .. }` | `PermanentUntilInvalidator` (preserve current default) | Non-self-care acquisitions retry on concrete invalidation (e.g. new source observation) rather than time decay. |
| `Patrol { .. }` | `CooldownRetry` | Recurring route duty; CIREM-004 finding — position-only invalidation stranded the guard at Watch Post. |
| `Wash` | (declare; preserve current default `PermanentUntilInvalidator`) | Wash exhaustion is corrected by belief currency on basin state (S129 substrate, CIREM-003 retention work); cooldown retry would mask stale-belief gaps. |
| `Eat`, `Drink`, `Relieve` | (declare per existing default) | Existing default is `PermanentUntilInvalidator`; preserve unless validation against goldens shows otherwise. |
| All other variants | (declare per existing default) | Existing default is `PermanentUntilInvalidator`; preserve. |

The exhaustive dispatch key match and declaration mapping guarantee that
future variant authors must either justify and declare
`PermanentUntilInvalidator` or pick `CooldownRetry`.

### D3: Refactor `frontier_exhaustion_entry`

`crates/worldwake-ai/src/agent_tick/planning.rs:873` becomes:

```rust
fn frontier_exhaustion_entry(
    goal_kind: &GoalKind,
    invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    baseline: ExhaustionBaseline,
    tick: Tick,
    cognitive: &CognitiveProfile,
) -> ExhaustionEntry {
    match GoalDispatchKey::from_goal_kind(goal_kind)
        .declaration()
        .frontier_exhaustion_strategy
    {
        FrontierExhaustionStrategy::CooldownRetry => ExhaustionEntry::budget_retry_pending(
            invalidation_conditions, baseline, tick, cognitive,
        ),
        FrontierExhaustionStrategy::PermanentUntilInvalidator => {
            ExhaustionEntry::frontier_exhausted(invalidation_conditions, baseline)
        }
    }
}
```

The hand-maintained `match { Sleep | AcquireCommodity { SelfConsume } | Patrol => budget; _ => frontier }`
arm is deleted.

### D4: Coverage test

Add unit tests on the declaration module that assert every
`GoalDispatchKey::all()` entry exposes a strategy and that the three recurring
retry declarations are `CooldownRetry` while representative preserved-default
declarations remain `PermanentUntilInvalidator`. The compile-time
exhaustiveness of `GoalDispatchKey::from_goal_kind` and
`GoalDispatchKey::declaration()` is the primary guard; these tests are runtime
backstops for the table.

## Validation and Falsification

### Unit tests

1. `frontier_exhaustion_strategy_marks_recurring_retry_goals`
2. `frontier_exhaustion_strategy_preserves_default_suppression_goals`
3. `frontier_exhaustion_strategy_covers_all_dispatch_declarations`
4. `frontier_exhaustion_entry_uses_strategy_dispatch` — drives
   `frontier_exhaustion_entry` through both arms with synthetic inputs
   and asserts the resulting `ExhaustionEntry` shape matches the
   strategy.

### Existing regressions

1. `record_exhausted_goals_records_self_consume_acquire_frontier_exhaustion_as_retry`
   (CIREM-002) continues to pass.
2. `record_exhausted_goals_records_patrol_frontier_exhaustion_as_budget_retry`
   (CIREM-004) continues to pass.
3. Survival-baseline / scattered / contested / patrol release goldens
   continue to pass under existing budgets.

### Falsification

If a future `GoalKind` variant is added without declaring its
strategy, `cargo check -p worldwake-ai` fails with a non-exhaustive
match error. That is the intended falsification surface.

## SystemFn Integration

Not applicable. This spec does not introduce a new SystemFn; it
refactors an internal helper inside the existing AI planning path
that runs as part of the existing AI agent-tick frame.

## Component Registration

Not applicable. This spec does not introduce a new component.

## Cross-System Interactions

Not applicable. This spec touches only the `worldwake-ai` planning
substrate and reads no state from other crates beyond what
`frontier_exhaustion_entry` already reads
(`ExhaustionInvalidationCondition`, `ExhaustionBaseline`, `Tick`,
`CognitiveProfile`).

## Profile-Driven Parameters

Not applicable. The strategy classification is a property of the goal
kind, not a per-agent variation. Per-agent variation in retry cadence
is governed by `CognitiveProfile` and the existing
`ExhaustionEntry::budget_retry_pending` decay arithmetic, neither of
which this spec changes.

## Save Format

No save format change. `ExhaustionEntry` and
`AgentDecisionRuntime.exhaustion_cache` are unchanged on disk; only
the dispatch that decides which `ExhaustionEntry` constructor to call
is refactored.

## Out of Scope

- New goal kinds.
- Changes to `ExhaustionEntry::budget_retry_pending` decay arithmetic.
- Changes to `ExhaustionInvalidationCondition` taxonomy.
- Persisting the strategy as runtime state — it is a compile-time
  per-variant property by design.
- Per-agent customization of the strategy — agents do not differ in
  whether patrol exhaustion is recurring vs. permanent; the goal-kind
  shape determines that.

## Open Questions

1. Should `Wash` move from `PermanentUntilInvalidator` to
   `CooldownRetry`? Today its exhaustion is corrected by basin-state
   belief currency (CIREM-003 retention work). Cooldown retry would
   mask stale-belief gaps; permanent suppression forces the agent to
   wait for new perception. Default is preserve.
2. Resolved by S132FROEXHSTR-001: the strategy lives on
   `GoalDispatchDeclaration`, the substrate S69 introduced. S69's
   declaration already centralizes goal-kind metadata, so placing the
   strategy there keeps goal-kind metadata co-located.
