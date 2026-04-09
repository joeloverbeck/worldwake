# S78OBSFAIPLADIA-002: Add target-belief column and failure frequency breakdown

**Status**: COMPLETED
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
4. Auto-correction: `PlanningPipelineTrace` in `crates/worldwake-ai/src/decision_trace.rs:227` does not carry a planning-time belief snapshot or `known_entities` inventory. The observer can read the end-of-run `AgentBeliefStore` from world state, but that would not prove “had target beliefs at planning time” for the failed attempt rows and would violate the intended proof boundary.
5. Auto-correction: `CandidateTrace.evidence[*].knowledge_path` is available on the planning trace, but it records candidate-motivation provenance, not a full planning-time entity-summary inventory. Using it as a proxy for “had target beliefs” would over-claim the contract for goals whose legality or emission depends on institutional or self knowledge rather than an entity-summary entry.
6. Correction applied: narrow this ticket to the frequency breakdown plus an explicit deferred note for the target-belief column. This matches the S78 spec’s “omit from the initial implementation and note as a future enhancement” fallback, preserves the CLI-only boundary, and avoids inventing a misleading debugging signal.

## Architecture Check

1. Target extraction from `GoalKind` is a pure pattern match in the observer — no AI crate changes. The frequency breakdown is a mechanical count over table data. Both are derived views over existing trace state, consistent with P27.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Deferred target-belief note appears only when planning-time belief presence cannot be proven from trace data → observer output visual inspection
2. Frequency breakdown counts match table row data → observer output visual inspection + focused unit test
3. Single-layer ticket (CLI output formatting only) — no cross-system verification needed

## What to Change

### 1. Confirm and note target-belief deferral honestly

In `crates/worldwake-cli/src/bin/observer.rs`, do not read the end-of-run `AgentBeliefStore` as a proxy for planning-time belief state. Instead, emit a short note after the failed-plan table explaining that the "Had Target Beliefs" column is deferred because the current trace model does not expose a planning-time belief snapshot for failed attempts.

### 2. Add failure frequency breakdown after table

After the failed-plan table, emit a summary section:

```markdown
### Failed Plan Frequency Breakdown
- frontier-exhausted: N / T
- budget-exhausted: N / T
- Max Depth = 0 (no operators available): N / T
```

Where `T` is total failed attempts shown and `N` is count per category. This is a mechanical count — no interpretive heuristics. Omit any target-belief count while the column is deferred.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Modifying `GoalKind` or any core/AI crate types
- Modifying `PlanSearchOutcome`, `PlanAttemptTrace`, or trace types
- Changing the planner's search algorithm or fallback behavior
- Adding new trace sink types
- Causal interpretation in the frequency breakdown (no "likely perception issue" prose)
- Reconstructing planning-time belief presence from end-of-run world belief state
- Using `knowledge_path` as a substitute for a full planning-time target-belief inventory

## Acceptance Criteria

### Tests That Must Pass

1. Observer binary runs on `scenarios/cli-evaluation.ron` and the failed-plan table remains at 7 columns with an explicit deferred note for "Had Target Beliefs"
2. Frequency breakdown section appears after the table with correct counts matching table rows
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. No AI crate or core crate types are modified
2. Frequency breakdown uses only mechanical counts — no interpretive prose
3. Observer output remains valid markdown format

## Test Plan

### New/Modified Tests

1. Add focused observer helper tests covering frequency-breakdown counts from failed-plan rows.

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test -p worldwake-cli`
3. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
4. `cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 200 --output /tmp/s78-observer-report-002.md`

## Outcome

Completed on 2026-04-09.

- Added a mechanical failed-plan frequency breakdown after each observer failed-plan table, counting frontier-exhausted rows, budget-exhausted rows, and rows with `Max Depth = 0` from the displayed failed attempts.
- Added an explicit deferral note for `Had Target Beliefs` because current planning traces do not carry a planning-time belief snapshot or `known_entities` inventory for failed attempts.
- Added focused observer helper coverage for the breakdown counts without changing any AI or core trace types.

## Deviations

- The original ticket draft assumed the target-belief column might still be implementable from observer-readable trace data. Reassessment showed that only end-of-run belief state and candidate-motivation provenance are available, neither of which truthfully proves planning-time target-belief presence for failed attempts. The ticket was therefore narrowed to the spec-allowed fallback: defer the column and land the frequency breakdown only.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
- Passed `cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 200 --output /tmp/s78-observer-report-002.md`
