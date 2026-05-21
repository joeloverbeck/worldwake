# S160HTNAUTHHON-001: MethodSubgoalAuthority enum + StageHint labeling

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — HTN method schema (`worldwake-ai`)
**Deps**: None

## Problem

Before this ticket, the HTN method schemas declared rich subgoal templates
(`PerformAction`, `ResolveCoordination`, `ReturnTo`, etc.), but
`search/strategic.rs` converted them into *strategic stages* with no enforcement
that a declared subgoal mapped to a real `ActionDef` leaf. The schema surface
overpromised enforcement the code did not provide (FND-20: planner formalisms may
encode search control, not authority they do not enforce). Nothing in the schema
recorded whether a subgoal was a mere stage hint or an enforced leaf, so a reader
could not tell honest from overpromised behavior.

This ticket introduced an explicit honesty axis: a `MethodSubgoalAuthority` label
on every subgoal. Every landed subgoal is labeled `StageHint` (truthful
classification of present behavior). The `RequiredActionLeaf` variant is defined
but unused, guarded by a negative test — the present-tense consumer that gives the
variant meaning and guards against premature method-required labeling.

## Assumption Reassessment (2026-05-21)

1. Before this ticket, `htn/method_schema.rs::MethodSchema` held
   `subgoals: Vec<SubgoalTemplate>`. `SubgoalTemplate` remains an 8-variant enum:
   `AcquireCommodity`,
   `TravelTo`, `ObserveTarget`, `AskWitness`, `InspectArtifact`, `PerformAction`,
   `ResolveCoordination`, `ReturnTo`. The landed carrier is now
   `Vec<MethodSubgoal>`, with per-subgoal authority stored beside the template.
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
5. Existing focused test affected: `registry.rs`
   `registry_builds_with_11_methods_without_dead_method_ids` — unaffected by this
   ticket (still 11 methods; only the subgoal carrier shape changes). No existing
   test asserted subgoal authority before this ticket.

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

## Verified Layers

1. Every current subgoal carries `StageHint` -> proved by
   `htn::registry::tests::all_current_subgoals_are_stage_hints`.
2. No current method declares `RequiredActionLeaf` -> proved by
   `htn::registry::tests::no_method_declares_required_action_leaf_at_landing`.
3. Single-layer ticket (schema metadata only): no action-trace / event-log mapping
   applied because the label is static schema state, not a runtime mutation.

## Landed Changes

### 1. Defined `MethodSubgoalAuthority` in `htn/method_schema.rs`

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
The landed enum derives `Clone, Copy, Debug, Eq, PartialEq`. `RequiredActionLeaf`
is documented as forward-declared: defined for the honest axis, with strategic
search enforcement deferred to the first method-required method.

### 2. Attached the label per subgoal

`MethodSchema.subgoals` now stores
`Vec<MethodSubgoal>`, where each wrapper has `template` and `authority` fields.
`htn/methods.rs::schema()` maps every current method-body `SubgoalTemplate`
literal through `MethodSubgoal::stage_hint`, so the stored registry schema carries
an explicit `StageHint` label without changing the 11 method bodies' template
lists.

### 3. Updated direct construction sites and consumers

The direct test construction sites in `method_schema.rs` and `search/strategic.rs`
now build `MethodSubgoal::stage_hint(...)` values. Strategic search and method
trace construction now read the wrapped `subgoal.template`. The integration
registry validation test was updated for the wrapper shape.

### 4. Added the positive + negative tests

- Positive: `all_current_subgoals_are_stage_hints` iterates
  `build_method_registry()` and asserts every subgoal's authority is `StageHint`.
- Negative: `no_method_declares_required_action_leaf_at_landing` asserts no method
  declares `RequiredActionLeaf`.

## Landed Files

- `crates/worldwake-ai/src/htn/method_schema.rs` (modify — enum + carrier shape + test site)
- `crates/worldwake-ai/src/htn/methods.rs` (modify — macro, `schema()`, 11 method bodies, tests)
- `crates/worldwake-ai/src/htn/mod.rs` (modify — re-export new wrapper and authority enum)
- `crates/worldwake-ai/src/htn/registry.rs` (modify — positive and negative authority tests)
- `crates/worldwake-ai/src/search/strategic.rs` (modify — wrapper consumer/test construction fallout)
- `crates/worldwake-ai/tests/integration/htn_registry_validation.rs` (modify — wrapper consumer fallout)

## Out of Scope

- The honest stage-hint *trace* surface (`SubgoalAttemptResult.authority`) — ticket 002.
- Any strategic-search enforcement of `RequiredActionLeaf` — deferred to a future
  method-required spec; this ticket only declares the variant.
- The group-hunt method rename — now archived at
  `archive/tickets/S160HTNAUTHHON-003.md`.
- The escort sentinel removal — ticket 004.

## Acceptance Result

### Tests Passed

1. Positive test added: every subgoal across all 11 methods is `StageHint`.
2. Negative test added: no method declares `RequiredActionLeaf`.
3. Existing suite passed: `cargo test -p worldwake-ai`.

### Invariants

1. Every subgoal in every registered method now carries an explicit
   `MethodSubgoalAuthority`; no implicit/defaulted-at-read authority.
2. `RequiredActionLeaf` is declared but unconstructed at landing (forward-declared
   variant), enforced by the negative test.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/htn/method_schema.rs` (or `methods.rs` test module) —
   the landed positive and negative authority tests live in
   `crates/worldwake-ai/src/htn/registry.rs`, where they can iterate the complete
   method registry.
2. `crates/worldwake-ai/tests/integration/htn_registry_validation.rs` — updated
   existing registry validation tests to read wrapped subgoal templates.

### Commands Passed

1. `cargo test -p worldwake-ai htn::`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-21.

- Added `MethodSubgoalAuthority` and `MethodSubgoal`, changing
  `MethodSchema.subgoals` from bare templates to explicit per-subgoal authority
  records.
- Labeled every current registered method subgoal as `StageHint` through the
  central `htn/methods.rs::schema()` construction path.
- Updated strategic-search and registry-validation consumers to read the wrapped
  template while preserving existing HTN behavior.
- Added focused positive and negative registry tests proving the landing state:
  all current subgoals are stage hints, and no current method declares
  `RequiredActionLeaf`.

## Deviations

- The ticket allowed either a tuple pair or a wrapper struct. The implementation
  landed the wrapper struct because named `template` and `authority` fields make
  later trace wiring clearer.
- The 11 method body subgoal template lists stayed terse; the explicit authority
  exists in the stored `MethodSchema` after the central constructor maps every
  current template through `MethodSubgoal::stage_hint`.
- Compile fallout required updating `search/strategic.rs` and
  `tests/integration/htn_registry_validation.rs`, in addition to the originally
  listed HTN schema/method files.

## Verification Result

- Passed `cargo test -p worldwake-ai htn:: -- --nocapture`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
