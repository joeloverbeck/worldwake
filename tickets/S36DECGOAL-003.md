# S36DECGOAL-003: Declaration-backed static AI dispatch

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai`: migrate static goal dispatch to declaration lookup keyed by the payload-aware S36 declaration key
**Deps**: [tickets/S36DECGOAL-002.md](/home/joeloverbeck/projects/worldwake/tickets/S36DECGOAL-002.md), [specs/S36-declarative-goal-registration.md](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md), [archive/tickets/completed/S36DECGOAL-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S36DECGOAL-001.md)

## Problem

Even after the provenance cleanup, static AI dispatch is still spread across hand-maintained tables and matches. The architecture has the right idea, but not one canonical declaration lookup yet. Static properties such as relevant planner ops, planner-op membership, trace labels, and provenance-family selection should all come from one declaration surface keyed by the payload-aware declaration key, not from parallel tables that can drift.

## Assumption Reassessment (2026-03-28)

1. Live static dispatch is still scattered across multiple `worldwake-ai` files:
   - [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) `GoalKindPlannerExt::relevant_op_kinds()`
   - [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) `goal_ranking_provenance()` via `ranked_goal_provenance_family()`
   - [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) `PlannerOpSemantics.relevant_goal_kinds` plus the `GOALS_*` arrays
   - [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) human-readable goal rendering still defaults to `Debug` formatting rather than a declaration-owned label
2. The completed provenance-family ticket intentionally stopped short of a full declaration table. It added `ranked_goal_provenance_family()` as one canonical family selector, but that is still one concern-specific lookup rather than the broader S36 declaration substrate.
3. `planner_ops.rs` still keys reverse planner-op membership on `GoalKindTag`, not on a payload-aware declaration key. That means S36 still lacks one source of truth for the static relationship “which planner ops are relevant to this dispatch-distinguishing goal shape?”
4. `GoalKind::PunishAccused { punishment }` already proves the need for payload-aware static dispatch in `GoalKindPlannerExt::relevant_op_kinds()`, even though `planner_ops.rs` currently treats both punishments under one coarse tag for reverse membership.
5. `decision_trace.rs` currently uses `Debug` formatting for selected-goal summaries instead of a stable declaration-owned trace label. The live architecture therefore does not yet satisfy the current S36 “trace label” narrative and should be migrated through the declaration surface rather than by adding another string table.
6. `agent_tick/frame.rs::progress_op_kinds()` remains domain-owned. Reassessment shows it should stay out of this ticket. The contract there is `IntentionDomain` progress, not static goal-shape declaration.
7. Coverage gap classification after search:
   - focused tests exist for planner-op semantics in `planner_ops.rs`, including `planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs`
   - focused tests exist for goal-side relevant-op routing in `goal_model.rs`
   - focused tests exist for provenance-family lookup in `goal_model.rs` and `ranking.rs`
   - no active ticket currently owns the full static-dispatch migration to one declaration table
8. This is a structural `worldwake-ai` ticket. No authoritative world behavior or information path changes are in scope. The shared boundary under audit is the AI static-dispatch contract between goal declaration, planner-op semantics, ranking, and human-readable trace labeling.
9. Mismatch + correction: the live code does not yet have a `GoalKindDeclaration` table. This ticket should create a declaration table keyed by the new payload-aware declaration key and migrate only truly static dispatch surfaces onto it. Dynamic strategy computation stays for a later ticket.

## Architecture Check

1. This ticket should move only static dispatch to declarations. That is the clean architectural center: labels, planner-op membership, goal-side relevant-op sets, and provenance-family selection are all static properties of a dispatch-distinguishing goal shape.
2. Reverse planner-op membership should be derived from the same declarations instead of maintained by a second manual `GOALS_*` matrix. That removes a real two-table drift risk without touching authoritative semantics.
3. This is cleaner than trying to force dynamic logic such as invalidation baselines or feasibility heuristics into static structs prematurely. The static table should own static facts; later tickets can layer dynamic strategy enums/functions on top of the same key.

## Verification Layers

1. Goal-side relevant-op lookup resolves through one declaration path -> focused `goal_model` tests
2. Planner-op reverse membership resolves through the same declaration source -> focused `planner_ops` tests
3. Provenance-family selection resolves through declaration metadata rather than a concern-specific side path -> focused `goal_model` and `ranking` tests
4. Human-readable selected-goal labels come from declaration metadata rather than raw `Debug` output -> focused `decision_trace` tests
5. Single-layer `worldwake-ai` structural ticket; no action trace, event-log, or authoritative world-state mapping applies

## What to Change

### 1. Introduce a real declaration table keyed by the payload-aware declaration key

Add a declaration struct in `worldwake-ai` for static goal-dispatch properties. At minimum it should own:

- a stable human-readable trace label
- the structured provenance family, if any
- the goal-side relevant planner-op set

If implementation finds another truly static field already duplicated beside these, migrate it in-scope rather than leaving another manual table behind.

### 2. Migrate goal-side static dispatch to declarations

Replace direct hand-maintained matches for:

- goal-side relevant-op lookup
- provenance-family selection
- any parallel static label lookup introduced during implementation

with declaration lookups keyed by the canonical payload-aware declaration key.

### 3. Derive planner-op reverse membership from the same declarations

Replace `planner_ops.rs` `GOALS_*` arrays / `relevant_goal_kinds`-style duplication with a reverse mapping built from the declaration table. The planner should not maintain a second manually curated matrix for the same static relationship.

If the planner-side type name needs to change from coarse “goal kinds” to payload-aware declaration keys, do that directly instead of keeping a compatibility alias.

### 4. Use declaration-owned labels in decision traces

Replace raw `Debug`-only selected-goal formatting in `decision_trace.rs` where a stable declaration label is the intended contract. Preserve payload information where it materially matters, but the label source itself should come from the declaration.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — declaration struct/table plus goal-side static dispatch lookups)
- `crates/worldwake-ai/src/ranking.rs` (modify — provenance-family lookup should consume declaration metadata)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — derive reverse planner-op membership from declarations)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — declaration-owned labels where the trace contract wants stable labels)
- `crates/worldwake-ai/src/lib.rs` (modify — export declaration types if needed across modules)
- `specs/S36-declarative-goal-registration.md` (modify if the implementation tightens field ownership or naming)

## Out of Scope

- Dynamic invalidation baseline computation
- Dynamic feasibility heuristics
- `IntentionDomain` progress-op ownership
- Candidate generation or authoritative action behavior changes

## Acceptance Criteria

### Tests That Must Pass

1. Goal-side relevant-op lookup no longer depends on a hand-maintained goal match once declaration lookup lands
2. Planner-op reverse membership no longer depends on the manual `GOALS_*` matrix
3. Provenance-family selection resolves from declaration metadata
4. Decision traces use stable declaration-owned goal labels where labels are part of the contract
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. Static goal dispatch has one canonical declaration source
2. No second planner-side reverse-membership table survives as a functional alias
3. Declaration-owned labels do not replace or invent authoritative payload facts; they only label the static dispatch family
4. `IntentionDomain` progress semantics remain domain-owned unless a future ticket changes that design intentionally

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — add focused tests proving declaration-owned relevant-op lookup remains payload-correct
2. `crates/worldwake-ai/src/planner_ops.rs` — add focused tests proving planner-op reverse membership is derived from declarations rather than a manual matrix
3. `crates/worldwake-ai/src/decision_trace.rs` — add focused tests proving selected-goal summaries use stable declaration-owned labels
4. `crates/worldwake-ai/src/ranking.rs` — keep or strengthen provenance-family regression tests against the declaration table

### Commands

1. `cargo test -p worldwake-ai planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs`
2. `cargo test -p worldwake-ai goal_model::tests::ranked_goal_provenance_family_is_payload_aware`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
