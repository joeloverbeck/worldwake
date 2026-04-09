# S78OBSFAIPLADIA-002: Add target-belief column and failure frequency breakdown

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — observer binary output only
**Deps**: S78OBSFAIPLADIA-001

## Problem

After ticket 001 adds diagnostic columns, the observer's failed-plan table still lacks two pieces of information needed for full root-cause triage: (1) whether the agent had beliefs about the goal's target entity (a common cause of frontier-exhausted failures), and (2) a frequency breakdown summarizing dominant failure modes across all failed plans. Without these, developers must manually count table rows and cross-reference belief state to identify systemic issues.

## Assumption Reassessment (2026-04-09)

1. `GoalKind` at `crates/worldwake-core/src/goal.rs:8-125` has 32 variants. Variants with entity target fields include: `EngageHostile { target }`, `RaidTarget { target }`, `TreatWounds { patient }`, `SearchForMissing { subject }`, `ReportMissing { subject }`, `ReportFound { subject }`, `EscortToSafety { subject }`, `LootCorpse { corpse }`, `BuryCorpse { corpse }`, `FulfillBounty { bounty }`, `ShareBelief { listener }`, `ClaimOffice { office }`, `SupportCandidateForOffice { office, candidate }`, `InvestigateViolation { place }`, `Patrol { place }`, `StealItem { target_item }`, `Accuse { accused }`, `PunishAccused { office, accused }`, `PostBounty { posting }`, `PostNotice { posting }`. Variants without entity targets: `ConsumeOwnedCommodity`, `AcquireCommodity`, `Sleep`, `Relieve`, `Wash`, `ReduceDanger`, `RegroupWithFaction`, `EstablishBanditCamp`, `ProduceCommodity`, `SellCommodity`, `RestockCommodity`, `MoveCargo`.
2. The trace hierarchy exposes `PlanAttemptTrace.goal: GoalKey` which contains `kind: GoalKind`. The target entity can be extracted by pattern-matching on the `GoalKind` variant.
3. The shared abstraction boundary is the observer's access to trace data. Whether belief snapshot data is available at observer read time needs verification during implementation. The spec notes: "If the existing trace types do not carry enough belief snapshot data to determine target-belief presence at observer read time, this column should be omitted from the initial implementation and noted as a future enhancement." This ticket follows that guidance — implement if feasible, defer if trace data is insufficient.

## Architecture Check

1. Target extraction from `GoalKind` is a pure pattern match in the observer — no AI crate changes. The frequency breakdown is a mechanical count over table data. Both are derived views over existing trace state, consistent with P27.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. "Had Target Beliefs" column shows `true`/`false`/`n/a` per row → observer output visual inspection
2. Frequency breakdown counts match table row data → observer output visual inspection
3. Single-layer ticket (CLI output formatting only) — no cross-system verification needed

## What to Change

### 1. Extract target entity from GoalKind

In `crates/worldwake-cli/src/bin/observer.rs`, add a helper function:

```rust
fn goal_target_entity(kind: &GoalKind) -> Option<EntityId> {
    match kind {
        GoalKind::EngageHostile { target } => Some(*target),
        GoalKind::RaidTarget { target } => Some(*target),
        GoalKind::TreatWounds { patient } => Some(*patient),
        GoalKind::SearchForMissing { subject, .. } => Some(*subject),
        // ... other variants with entity targets
        _ => None, // Sleep, Relieve, Wash, etc.
    }
}
```

### 2. Determine target-belief presence

Investigate whether the trace hierarchy exposes belief snapshot data at observer read time. Candidate sources:
- `PlanningPipelineTrace` may carry belief-related trace data
- The observer may need to check if belief summaries are available

If belief data is accessible: for each failed attempt, extract the target entity via `goal_target_entity()`, then check the belief snapshot for an `EntitySummary` matching that entity. Render `true`/`false`.

If belief data is NOT accessible at observer read time: render the column as `—` for all rows and add a code comment noting the limitation for a future enhancement.

### 3. Add "Had Target Beliefs" column to table

Extend the table header from ticket 001's 7 columns to 8: append `| Had Target Beliefs |`. Render `true`, `false`, or `n/a` (for goal kinds without entity targets).

### 4. Add failure frequency breakdown after table

After the failed-plan table, emit a summary section:

```markdown
### Failed Plan Frequency Breakdown
- frontier-exhausted: N / T
- budget-exhausted: N / T
- Max Depth = 0 (no operators available): N / T
- Had Target Beliefs = false: N / T   (omit if column is deferred)
```

Where `T` is total failed attempts shown and `N` is count per category. This is a mechanical count — no interpretive heuristics.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Modifying `GoalKind` or any core/AI crate types
- Modifying `PlanSearchOutcome`, `PlanAttemptTrace`, or trace types
- Changing the planner's search algorithm or fallback behavior
- Adding new trace sink types
- Causal interpretation in the frequency breakdown (no "likely perception issue" prose)

## Acceptance Criteria

### Tests That Must Pass

1. Observer binary runs on `scenarios/cli-evaluation.ron` and the failed-plan table shows 8 columns (or 7 + deferred note if belief data unavailable)
2. Frequency breakdown section appears after the table with correct counts matching table rows
3. `goal_target_entity` correctly returns `None` for targetless variants (`Sleep`, `Relieve`, etc.) and `Some(id)` for target variants
4. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. No AI crate or core crate types are modified
2. Frequency breakdown uses only mechanical counts — no interpretive prose
3. Observer output remains valid markdown format

## Test Plan

### New/Modified Tests

1. None — observer output formatting; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo build -p worldwake-cli --bin observer` — confirms compilation
2. `cargo clippy --workspace --all-targets -- -D warnings` — lint clean
3. `cargo test --workspace` — full suite
