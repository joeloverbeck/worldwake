# S160HTNAUTHHON-001: MethodSubgoalAuthority enum + StageHint labeling

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — HTN method schema (`worldwake-ai`)
**Deps**: None

## Problem

The HTN method schemas declare rich subgoal templates (`PerformAction`,
`ResolveCoordination`, `ReturnTo`, etc.), but `search/strategic.rs` converts them
into *strategic stages* with no enforcement that a declared subgoal maps to a real
planned `ActionDef` leaf. The schema surface overpromises enforcement the code does
not provide (FND-20: planner formalisms may encode search control, not authority
they do not enforce). Today nothing in the schema records whether a subgoal is a
mere stage hint or an enforced leaf, so a reader cannot tell honest from
overpromised behavior.

This ticket introduces an explicit honesty axis: a `MethodSubgoalAuthority` label
on every subgoal. Every current subgoal is labeled `StageHint` (truthful
classification of present behavior). The `RequiredActionLeaf` variant is defined
but unused, guarded by a negative test — the present-tense consumer that gives the
variant meaning and guards against premature method-required labeling.

## Assumption Reassessment (2026-05-21)

1. `htn/method_schema.rs::MethodSchema` (line 8) holds `subgoals: Vec<SubgoalTemplate>`
   (line 12). `SubgoalTemplate` (line 26) is an 8-variant enum: `AcquireCommodity`,
   `TravelTo`, `ObserveTarget`, `AskWitness`, `InspectArtifact`, `PerformAction`,
   `ResolveCoordination`, `ReturnTo`. No authority label exists today.
2. All 11 method schemas are constructed through `htn/methods.rs::schema(parts:
   MethodParts)` (line 35) and the `method_schema!` macro (line 47); the 11 method
   bodies each list `subgoals` as `vec![SubgoalTemplate::...]` literals. One
   additional construction site exists in a test at `method_schema.rs:276`. The
   construction surface is centralized through the macro + `schema()` helper plus
   the 11 method-body subgoal literals.
3. Shared boundary under audit: the `MethodSchema.subgoals` shape and the
   `method_schema!` macro that all method definitions route through. Per-subgoal
   authority granularity is required because `RequiredActionLeaf` is a per-subgoal
   concept (a future method may mix enforced leaves with stage hints).
4. Precedent: `goal_schema.rs:1173` `test_relevant_ops_authority_is_hint_only_at_landing`
   is the established "define the honest classification axis with both variants,
   assert no method uses the enforced variant yet" pattern. This ticket mirrors it
   for subgoal authority, satisfying `docs/spec-drafting-rules.md` rule 5 ("enforced
   declarations only") via a negative test as the live consumer.
5. Existing focused test affected: `registry.rs:73`
   `registry_builds_with_11_methods_without_dead_method_ids` — unaffected by this
   ticket (still 11 methods; only the subgoal carrier shape changes). No existing
   test asserts subgoal authority (the axis is new).

## Architecture Check

1. The label makes the *unenforced* status of every subgoal explicit in the schema
   itself rather than buried in `search/strategic.rs` behavior — honest by
   construction. Defining both variants now (rather than `StageHint` only) is
   required so the downstream trace distinction (ticket 002) is non-vacuous; a
   single-variant enum would convey no distinction.
2. No backward-compatibility shim: the subgoal carrier shape changes outright; all
   construction sites route through the macro + `schema()` and are updated in this
   ticket. The strategic-search enforcement for `RequiredActionLeaf` is deferred to
   the first real method-required method (a future spec) — the variant is a
   forward-declared enum variant (a declaration, not a dead code path), guarded by
   the negative test.

## Verification Layers

1. Every current subgoal carries `StageHint` -> focused unit test over
   `build_method_registry()` iterating all methods' subgoals.
2. No current method declares `RequiredActionLeaf` -> negative focused unit test
   (mirrors `test_relevant_ops_authority_is_hint_only_at_landing`).
3. Single-layer ticket (schema metadata only): no action-trace / event-log mapping
   applies — the label is static schema state, not a runtime mutation.

## What to Change

### 1. Define `MethodSubgoalAuthority` in `htn/method_schema.rs`

```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodSubgoalAuthority {
    /// Subgoal contributes strategic destinations, prerequisite commodities,
    /// or trace context. Not enforced as an ordinary ActionDef leaf.
    StageHint,
    /// Subgoal must correspond to at least one ordinary ActionDef-backed
    /// planned step, and the trace must prove selected/skipped/failed status.
    RequiredActionLeaf,
}
```
Match the derive set used by sibling schema enums (`MethodPrecondition` derives
`Clone, Debug, Eq, PartialEq`; add `Copy` since the label is a payload-free
discriminant). Document `RequiredActionLeaf` as forward-declared: defined for the
honest axis, with strategic-search enforcement deferred to the first method-required
method.

### 2. Attach the label per subgoal

Change `MethodSchema.subgoals` to carry an authority label per subgoal. Carrier
choice (ticket-time detail, pick the cleaner): either
`subgoals: Vec<(SubgoalTemplate, MethodSubgoalAuthority)>` or a small wrapper
struct `MethodSubgoal { template: SubgoalTemplate, authority: MethodSubgoalAuthority }`.
Update `htn/methods.rs::schema()` and the `method_schema!` macro so each subgoal
literal is paired with an authority. Default all current subgoals to `StageHint`
(the macro may default to `StageHint` to keep the 11 method bodies terse, as long
as the label is explicit in the stored schema).

### 3. Update all 11 method bodies + the test construction site

Ensure every `vec![SubgoalTemplate::...]` literal in `htn/methods.rs` (the 11
methods) and the test construction at `method_schema.rs:276` produce subgoals
labeled `StageHint`.

### 4. Add the positive + negative tests

- Positive: iterate `build_method_registry()` and assert every subgoal's authority
  is `StageHint`.
- Negative: assert no method declares `RequiredActionLeaf` (mirroring
  `goal_schema.rs:1173`).

## Files to Touch

- `crates/worldwake-ai/src/htn/method_schema.rs` (modify — enum + carrier shape + test site)
- `crates/worldwake-ai/src/htn/methods.rs` (modify — macro, `schema()`, 11 method bodies, tests)

## Out of Scope

- The honest stage-hint *trace* surface (`SubgoalAttemptResult.authority`) — ticket 002.
- Any strategic-search enforcement of `RequiredActionLeaf` — deferred to a future
  method-required spec; this ticket only declares the variant.
- The group-hunt method rename — ticket 003.
- The escort sentinel removal — ticket 004.

## Acceptance Criteria

### Tests That Must Pass

1. New positive test: every subgoal across all 11 methods is `StageHint`.
2. New negative test: no method declares `RequiredActionLeaf`.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Every subgoal in every registered method carries an explicit
   `MethodSubgoalAuthority`; no implicit/defaulted-at-read authority.
2. `RequiredActionLeaf` is declared but unconstructed at landing (forward-declared
   variant), enforced by the negative test.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/htn/method_schema.rs` (or `methods.rs` test module) —
   positive `all_current_subgoals_are_stage_hints` and negative
   `no_method_declares_required_action_leaf_at_landing`, mirroring the
   `relevant_ops` hint-only precedent.

### Commands

1. `cargo test -p worldwake-ai htn::`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `./scripts/verify.sh`
