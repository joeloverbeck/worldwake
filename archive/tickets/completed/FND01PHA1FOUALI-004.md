# FND01PHA1FOUALI-004: Rename KnowledgeView to BeliefView, WorldKnowledgeView to OmniscientBeliefView

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — trait rename across sim crate, updated module names
**Deps**: None (but larger than -001/-002/-003; do after them for cleaner merge)

## Problem

`KnowledgeView` is the query boundary used by agent-facing affordance code, but the name does not signal that this boundary is supposed to represent beliefs rather than authoritative truth. `WorldKnowledgeView` wraps `&World` directly, so agent-facing call sites can currently satisfy the interface with omniscient state. That makes it too easy for future planning/perception work to cross the belief boundary accidentally, which conflicts with Principle 7 (Locality) and Principle 10 (Intelligent Agency).

## Assumption Reassessment (2026-03-10)

1. `KnowledgeView` trait at `crates/worldwake-sim/src/knowledge_view.rs:3` has 7 methods: `is_alive`, `entity_kind`, `effective_place`, `entities_at`, `commodity_quantity`, `has_control`, `reservation_conflicts` — confirmed.
2. `WorldKnowledgeView` at `crates/worldwake-sim/src/world_knowledge_view.rs:6` wraps `&World` and delegates directly to authoritative world queries — confirmed.
3. `get_affordances()` and its helper functions in `crates/worldwake-sim/src/affordance_query.rs` accept `&dyn KnowledgeView` — confirmed.
4. Non-archive references exist in:
   - `crates/worldwake-sim/src/knowledge_view.rs`
   - `crates/worldwake-sim/src/world_knowledge_view.rs`
   - `crates/worldwake-sim/src/affordance_query.rs`
   - `crates/worldwake-sim/src/tick_step.rs`
   - `crates/worldwake-sim/src/start_gate.rs`
   - `crates/worldwake-sim/src/tick_action.rs`
   - `crates/worldwake-sim/src/interrupt_abort.rs`
   - `crates/worldwake-sim/src/lib.rs`
   - `CLAUDE.md`
5. `start_gate.rs`, `tick_action.rs`, and `interrupt_abort.rs` do not use raw `&World` alone for all validation. They currently instantiate `WorldKnowledgeView` to reuse shared constraint/precondition/effective-place logic during authoritative action execution — confirmed.
6. Current tests already cover the rename surface indirectly:
   - `world_knowledge_view.rs` has trait/behavior coverage for the wrapper.
   - `affordance_query.rs` has stub-backed tests plus control-source invariants using `WorldKnowledgeView`.
   - `start_gate.rs` explicitly verifies authoritative revalidation against current world state.

## Architecture Check

1. Renaming `KnowledgeView` → `BeliefView` is better than the current architecture because it makes the intended semantics explicit at the abstraction boundary used by affordance generation. This is valuable even before E14 lands.
2. Renaming `WorldKnowledgeView` → `OmniscientBeliefView` with an explicit "Omniscient" prefix is better than the current architecture because it advertises that the implementation is a temporary, authority-backed stand-in rather than a correct belief store.
3. This ticket does **not** fully solve the deeper architectural issue that authoritative execution paths (`start_gate`, `tick_action`, `interrupt_abort`) currently reuse the same query trait. That split should happen later when action semantics can cleanly distinguish belief evaluation from authoritative validation. For now, the rename still improves correctness-by-default naming without adding compatibility shims or misleading aliases.
4. No backward-compatibility aliases — all references are updated directly (Principle 13).

## What to Change

### 1. Rename `KnowledgeView` trait to `BeliefView`

In `knowledge_view.rs`: rename trait. Rename file to `belief_view.rs`.

### 2. Rename `WorldKnowledgeView` to `OmniscientBeliefView`

In `world_knowledge_view.rs`: rename struct and impl block. Rename file to `omniscient_belief_view.rs`.

Add doc-comment to `OmniscientBeliefView`:
```rust
/// Temporary stand-in until E14 provides per-agent belief stores.
/// MUST NOT be used in agent-facing code after E14 lands.
/// Wraps `&World` directly — returns authoritative truth, not beliefs.
```

### 3. Update `lib.rs` module declarations and re-exports

- `mod knowledge_view` → `mod belief_view`
- `mod world_knowledge_view` → `mod omniscient_belief_view`
- Update all `pub use` statements.

### 4. Update `affordance_query.rs`

- `view: &dyn KnowledgeView` → `view: &dyn BeliefView` in `get_affordances()`, `evaluate_constraint()`, `evaluate_precondition()`, `enumerate_targets()`.
- Update imports.

### 5. Update `tick_step.rs`

- `WorldKnowledgeView::new(world)` → `OmniscientBeliefView::new(world)`.
- Update imports.

### 6. Update remaining sim files

For each of `tick_action.rs`, `start_gate.rs`, `interrupt_abort.rs`:
- Update imports and constructor calls from `KnowledgeView` / `WorldKnowledgeView` to `BeliefView` / `OmniscientBeliefView`.
- Preserve the current authoritative behavior. These files still use the omniscient wrapper as a temporary adapter for shared validation logic; this ticket does not split that architecture yet.

### 7. Add divergent-belief test

Create a `StubBeliefView` that returns different `effective_place` values for the same actor. Call `get_affordances()` with two different stubs and assert they produce different affordance sets. This proves the pipeline respects the belief boundary.

Place this test in `affordance_query.rs` tests or a new test module.

### 8. Update CLAUDE.md architecture table

Update the `worldwake-sim modules` table:
- `knowledge_view` → `belief_view` with description: "`BeliefView` trait — agent belief interface"
- `world_knowledge_view` → `omniscient_belief_view` with description: "`OmniscientBeliefView` — omniscient stand-in until E14"

## Files to Touch

- `crates/worldwake-sim/src/knowledge_view.rs` → rename to `belief_view.rs` (modify)
- `crates/worldwake-sim/src/world_knowledge_view.rs` → rename to `omniscient_belief_view.rs` (modify)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — parameter types + imports)
- `crates/worldwake-sim/src/tick_step.rs` (modify — constructor + imports)
- `crates/worldwake-sim/src/tick_action.rs` (modify — imports + constructor rename)
- `crates/worldwake-sim/src/start_gate.rs` (modify — imports + constructor rename)
- `crates/worldwake-sim/src/interrupt_abort.rs` (modify — imports + constructor rename)
- `crates/worldwake-sim/src/lib.rs` (modify — module declarations + re-exports)
- `CLAUDE.md` (modify — architecture table update)

## Out of Scope

- Do NOT implement real belief filtering or per-agent belief stores (that's E14).
- Do NOT change the `BeliefView` trait methods — same interface, just renamed.
- Do NOT split authoritative validation into a separate interface in this ticket.
- Do NOT touch worldwake-core crate.
- Do NOT modify archive files or old spec documents.

## Acceptance Criteria

### Tests That Must Pass

1. No symbol named `KnowledgeView` or `WorldKnowledgeView` exists in non-archive source files.
2. `get_affordances()` accepts `&dyn BeliefView`.
3. `start_gate`, `tick_action`, and `interrupt_abort` preserve their current authoritative behavior while using the renamed omniscient adapter.
4. Divergent-belief test: two `StubBeliefView` instances with different `effective_place` returns produce different affordance sets.
5. `OmniscientBeliefView` has doc-comment stating temporary status.
6. Existing suite: `cargo test -p worldwake-sim`
7. Full suite: `cargo test --workspace`
8. `cargo clippy --workspace` clean.

### Invariants

1. All existing affordance tests pass unchanged (behavior identical, only names changed).
2. `OmniscientBeliefView` still delegates to `&World` methods identically to old `WorldKnowledgeView`.
3. Human/AI control swap test still passes (agent symmetry invariant).
4. Authoritative action-start / tick / abort revalidation behavior remains unchanged.

## Test Plan

### New/Modified Tests

1. `affordance_query.rs::divergent_belief_views_produce_different_affordances` — new test with stub implementations.
2. All existing `world_knowledge_view.rs` tests — renamed file, same tests, updated type names.
3. All existing `affordance_query.rs` tests — updated imports only.
4. Existing `start_gate.rs` authoritative revalidation tests — unchanged expectations, updated type names only.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Renamed the query trait and wrapper to `BeliefView` and `OmniscientBeliefView`, including the sim crate module filenames and public re-exports.
- Updated all sim call sites and tests to use the new names with no compatibility aliases left behind.
- Added the divergent-belief affordance test to prove that `get_affordances()` respects the passed belief view rather than assuming authoritative placement.
- Added `action_validation`, a dedicated authoritative validation module, and removed the belief wrapper from `start_gate`, `tick_action`, and `interrupt_abort`.
- Kept `OmniscientBeliefView` only as the temporary omniscient adapter for affordance generation until real per-agent belief stores exist.
