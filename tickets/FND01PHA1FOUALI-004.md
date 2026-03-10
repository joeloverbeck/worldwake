# FND01PHA1FOUALI-004: Rename KnowledgeView to BeliefView, WorldKnowledgeView to OmniscientBeliefView

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — trait rename across sim crate, updated module names
**Deps**: None (but larger than -001/-002/-003; do after them for cleaner merge)

## Problem

`KnowledgeView` is used both for agent-facing affordance queries and for authoritative legality checks. The name does not signal that agent-facing code must operate on beliefs, not world truth. `WorldKnowledgeView` wraps `&World` directly, making it trivially easy for future agent-facing code to read authoritative state. This violates Principle 7 (Locality) and Principle 10 (Intelligent Agency).

## Assumption Reassessment (2026-03-10)

1. `KnowledgeView` trait at `knowledge_view.rs:3-11` has 7 methods: `is_alive`, `entity_kind`, `effective_place`, `entities_at`, `commodity_quantity`, `has_control`, `reservation_conflicts` — confirmed.
2. `WorldKnowledgeView` struct at `world_knowledge_view.rs:6-14` wraps `&'w World` — confirmed.
3. `get_affordances()` at `affordance_query.rs:5` takes `view: &dyn KnowledgeView` — confirmed.
4. Source files referencing `KnowledgeView` or `WorldKnowledgeView` (non-archive):
   - `crates/worldwake-sim/src/knowledge_view.rs` (trait definition)
   - `crates/worldwake-sim/src/world_knowledge_view.rs` (impl)
   - `crates/worldwake-sim/src/affordance_query.rs` (parameter type + helpers)
   - `crates/worldwake-sim/src/tick_step.rs` (constructs WorldKnowledgeView)
   - `crates/worldwake-sim/src/tick_action.rs` (may reference)
   - `crates/worldwake-sim/src/start_gate.rs` (may reference)
   - `crates/worldwake-sim/src/interrupt_abort.rs` (may reference)
   - `crates/worldwake-sim/src/lib.rs` (re-exports)
5. `start_gate.rs` and `tick_action.rs` use `&World` directly for authoritative legality checks — confirmed, this is correct and stays.

## Architecture Check

1. Renaming `KnowledgeView` → `BeliefView` signals that anything consuming `&dyn BeliefView` must tolerate stale/partial data. This is a semantic signal for E14 (perception/beliefs).
2. Renaming `WorldKnowledgeView` → `OmniscientBeliefView` with "Omniscient" prefix makes the temporary shortcut explicit and grep-able for future removal.
3. No backward-compatibility aliases — all references are updated directly (Principle 13).

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
- Update any imports of `KnowledgeView` or `WorldKnowledgeView`.
- If these files only use `&World` directly (for legality), they may only need import cleanup.

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
- `crates/worldwake-sim/src/tick_action.rs` (modify — imports if needed)
- `crates/worldwake-sim/src/start_gate.rs` (modify — imports if needed)
- `crates/worldwake-sim/src/interrupt_abort.rs` (modify — imports if needed)
- `crates/worldwake-sim/src/lib.rs` (modify — module declarations + re-exports)
- `CLAUDE.md` (modify — architecture table update)

## Out of Scope

- Do NOT implement real belief filtering or per-agent belief stores (that's E14).
- Do NOT change the `BeliefView` trait methods — same interface, just renamed.
- Do NOT modify `start_gate.rs` or `tick_action.rs` legality logic that correctly uses `&World`.
- Do NOT touch worldwake-core crate.
- Do NOT modify archive files or old spec documents.

## Acceptance Criteria

### Tests That Must Pass

1. No symbol named `KnowledgeView` or `WorldKnowledgeView` exists in non-archive source files.
2. `get_affordances()` accepts `&dyn BeliefView`.
3. `start_gate` and `tick_action` use `&World` for authoritative checks (unchanged).
4. Divergent-belief test: two `StubBeliefView` instances with different `effective_place` returns produce different affordance sets.
5. `OmniscientBeliefView` has doc-comment stating temporary status.
6. Existing suite: `cargo test -p worldwake-sim`
7. Full suite: `cargo test --workspace`
8. `cargo clippy --workspace` clean.

### Invariants

1. All existing affordance tests pass unchanged (behavior identical, only names changed).
2. `OmniscientBeliefView` still delegates to `&World` methods identically to old `WorldKnowledgeView`.
3. Human/AI control swap test still passes (agent symmetry invariant).

## Test Plan

### New/Modified Tests

1. `affordance_query.rs::divergent_belief_views_produce_different_affordances` — new test with stub implementations.
2. All existing `world_knowledge_view.rs` tests — renamed file, same tests, updated type names.
3. All existing `affordance_query.rs` tests — updated imports only.

### Commands

1. `cargo test -p worldwake-sim -- belief_view`
2. `cargo test -p worldwake-sim -- omniscient`
3. `cargo test -p worldwake-sim -- affordance`
4. `cargo test --workspace && cargo clippy --workspace`
