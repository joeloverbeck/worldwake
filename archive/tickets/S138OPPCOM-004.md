# S138OPPCOM-004: Authority enum and relevant_ops_authority method on goal dispatch

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None at landing — the new method returns `HintOnly` for all goal kinds but no consumer reads it yet. Authority semantics activate when ticket 006 lands.
**Deps**: None

## Problem

S138's architectural shift makes `GoalDispatchDeclaration.relevant_ops` a ranking hint rather than the authoritative gate over which planner operators a goal kind can use. The effect-schema index (ticket 005) becomes the broader authority for "which actions produce this effect". To make the semantic shift explicit and inspectable, this ticket introduces an `Authority` enum and a `relevant_ops_authority()` method that returns `HintOnly` for every goal kind at landing. The existing conformance test that asserts `relevant_ops` matches the live `GoalKindPlannerExt::relevant_op_kinds()` is preserved unchanged — the test's assertion is about correctness of the hint, not about whether the hint is authoritative.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-ai/src/goal_dispatch_decl.rs` has focused declaration tests, including `test_declaration_relevant_ops_match_live_goal_model`, which asserts `relevant_ops` equals `relevant_op_kinds()` per goal kind. This ticket preserves the conformance assertion verbatim.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "`relevant_ops` reclassification (in `goal_dispatch_decl.rs`)".
3. Shared abstraction boundary: `GoalDispatchDeclaration` in `goal_dispatch_decl.rs` — the new method is a property of the declaration, sibling to existing per-declaration metadata (`invalidation_strategy`, `feasibility_strategy`, etc.).
4. Planner-modifying ticket: technically yes (the method's return value will be consumed by ticket 006 to decide when to query the effect-schema index), but at this ticket's landing the method has no consumer, so the Authoritative-to-AI Impact Rule 7-point checklist is satisfied trivially — the existing planner behavior is unchanged.

## Architecture Check

1. Returning `HintOnly` uniformly at landing avoids per-goal-kind judgment calls — the conformance test still gates correctness of the hint set, and ticket 006 will read the authority value to decide when to extend candidate generation through the effect-schema index. Future tuning can return `Gate` per goal kind if a specific goal class should not be extended.
2. The conformance test framing is preserved exactly: `relevant_ops` continues to equal `relevant_op_kinds()` — the test name `test_declaration_relevant_ops_match_live_goal_model` reads naturally as "the hint is accurate", which is the post-S138 semantic.
3. No backward-compatibility shim: the `Authority` enum is new and unused at landing; no code path is duplicated or deprecated.

## Verification Layers

1. Method returns `HintOnly` for every goal kind — focused unit test iterating `GoalDispatchKey::ALL` and asserting the method's return value
2. Conformance test `test_declaration_relevant_ops_match_live_goal_model` continues to pass unchanged — runtime test execution
3. `Authority` enum derives are sufficient for use as a value type (`Copy`, `Eq`, `Hash`) — compiler-enforced via the focused unit test

## What to Change

### 1. Define `Authority` enum

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`, add adjacent to `GoalDispatchDeclaration`:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Authority {
    Gate,
    HintOnly,
}
```

### 2. Add `relevant_ops_authority` method

Add a free function or method on `GoalDispatchDeclaration` (preferred — keeps the property attached to the declaration):

```rust
impl GoalDispatchDeclaration {
    pub fn relevant_ops_authority(&self) -> Authority {
        Authority::HintOnly
    }
}
```

The uniform `HintOnly` return is intentional — ticket 006 will consume this value, and per-goal-kind variation is future work (out of scope for S138).

### 3. Preserve existing conformance test

`test_declaration_relevant_ops_match_live_goal_model` stays unchanged. No prose around the test is altered; it continues to assert the hint accuracy.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — add Authority enum + method)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `Authority` for downstream S138 consumers)

## Outcome

Completion date: 2026-05-11.

Implemented the staged authority surface in `goal_dispatch_decl.rs`: `Authority { Gate, HintOnly }` now derives value-type and serde traits, and `GoalDispatchDeclaration::relevant_ops_authority()` returns `Authority::HintOnly` for every declaration. The existing `relevant_ops` slices and conformance assertion were not changed. `worldwake-ai::Authority` is re-exported from `lib.rs` for ticket 006 consumption.

## Deviations

- Added the `Authority` re-export in `crates/worldwake-ai/src/lib.rs`; the draft file list omitted it, but later S138 consumers need the public value type.
- Updated `specs/S138-opportunity-compiler.md` to describe the landed method form on `GoalDispatchDeclaration` instead of a free function.

## Out of Scope

- Consuming `relevant_ops_authority()` in candidate generation — lands in ticket 006
- `EffectSchemaIndex` (which becomes the broader authority when the hint is exhausted) — lands in ticket 005
- Per-goal-kind authority variation — future tuning, not part of S138

## Acceptance Criteria

### Tests That Must Pass

1. New test in `goal_dispatch_decl.rs`: `relevant_ops_authority()` returns `Authority::HintOnly` for every goal kind in `GoalDispatchKey::ALL`
2. Existing test `test_declaration_relevant_ops_match_live_goal_model` continues to pass unchanged
3. Existing neighboring declaration tests continue to pass
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `relevant_ops` static slice remains identical to today's content — no entries added, removed, or reordered as part of this ticket
2. Adding the `Authority` enum does not change the planner's runtime behavior — no consumer reads the new method yet
3. The conformance test's assertion is unchanged: `relevant_ops` equals `relevant_op_kinds()` per goal kind

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs` (inline `#[cfg(test)]`) — `test_relevant_ops_authority_is_hint_only_at_landing` iterating `GoalDispatchKey::ALL`

### Commands

1. `cargo test -p worldwake-ai goal_dispatch_decl`
2. `cargo test -p worldwake-ai test_declaration_relevant_ops_match_live_goal_model`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Verification Result

- Passed: `cargo test -p worldwake-ai goal_dispatch_decl`
- Passed: `cargo test -p worldwake-ai test_declaration_relevant_ops_match_live_goal_model`
- Passed: `cargo test -p worldwake-ai`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
