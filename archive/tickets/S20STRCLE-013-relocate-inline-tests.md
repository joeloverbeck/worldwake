# S20STRCLE-013: Relocate inline tests to dedicated test sub-modules

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-002, S20STRCLE-003, S20STRCLE-004, S20STRCLE-005, S20STRCLE-006, S20STRCLE-007, S20STRCLE-009, S20STRCLE-010, S20STRCLE-011, S20STRCLE-012

## Problem

After all sub-module extractions, `agent_tick/mod.rs` still contains 59 inline tests (starting at line 454) and `search/mod.rs` contains 65 inline tests (starting at line 283). These should be moved to dedicated `tests.rs` sub-modules for each directory module, keeping test code near but separate from production code.

## Assumption Reassessment (2026-03-22)

1. `agent_tick/mod.rs` has `#[cfg(test)] mod tests { ... }` starting at line 454 with 59 `#[test]` functions — verified via `grep -c '#\[test\]'`.
2. `search/mod.rs` has `#[cfg(test)] mod tests { ... }` starting at line 283 with 65 `#[test]` functions — verified via `grep -c '#\[test\]'`.
3. Tests use `super::*` imports to access private functions. After sub-module extraction, tests that reference functions now in sub-modules may need adjusted import paths (e.g., `super::observation::*` or `use crate::agent_tick::observation::*`). However, if `mod.rs` re-exports via `use observation::*;`, then `super::*` still works.
4. N/A — not an AI regression.
5–12. N/A — pure structural refactor.

## Architecture Check

1. Dedicated `tests.rs` modules are idiomatic Rust for large modules. They keep test code discoverable without cluttering production modules.
2. No backward-compatibility shims. Test function names, assertions, and behavior remain identical.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai` — exact same test count before and after.
2. Single-layer ticket: test relocation only.

## What to Change

### 1. Create `agent_tick/tests.rs`

Move the entire `#[cfg(test)] mod tests { ... }` block (lines 454–EOF) from `agent_tick/mod.rs` into `agent_tick/tests.rs`. In `mod.rs`, add:
```rust
#[cfg(test)]
mod tests;
```

### 2. Create `search/tests.rs`

Move the entire `#[cfg(test)] mod tests { ... }` block (lines 283–EOF) from `search/mod.rs` into `search/tests.rs`. In `mod.rs`, add:
```rust
#[cfg(test)]
mod tests;
```

### 3. Fix imports in test files

Update `use super::*` and specific `use super::function_name` imports to resolve correctly against the new module structure. Since `mod.rs` re-exports sub-module contents, most `super::*` imports should work unchanged.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/tests.rs` (new)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — remove test block, add `#[cfg(test)] mod tests;`)
- `crates/worldwake-ai/src/search/tests.rs` (new)
- `crates/worldwake-ai/src/search/mod.rs` (modify — remove test block, add `#[cfg(test)] mod tests;`)

## Out of Scope

- Splitting tests into per-sub-module test files (that would be a follow-up if desired)
- Modifying any test logic, assertions, or test names
- Adding new tests
- Any changes outside `worldwake-ai`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all tests pass unchanged
2. `cargo test -p worldwake-ai -- --list 2>&1 | grep -c ':: test$'` — exact same count as before this ticket
3. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Zero behavioral change — test relocation only
2. Every test function name preserved exactly
3. Every assertion preserved exactly
4. Test count before == test count after

## Test Plan

### New/Modified Tests

1. None — test relocation only; all existing tests are preserved.

### Commands

1. `cargo test -p worldwake-ai -- --list 2>&1 | grep -c ':: test$'` (run before AND after — counts must match)
2. `cargo test -p worldwake-ai`
3. `cargo clippy -p worldwake-ai`

## Outcome

- **Completion date**: 2026-03-23
- **What changed**:
  - Created `crates/worldwake-ai/src/agent_tick/tests.rs` (59 tests extracted from `agent_tick/mod.rs` lines 454–4495)
  - Created `crates/worldwake-ai/src/search/tests.rs` (65 tests extracted from `search/mod.rs` lines 283–5208)
  - Both `mod.rs` files trimmed to production code + `#[cfg(test)] mod tests;` declaration
- **Deviations**: Ticket line numbers were stale (2092/955 from pre-extraction state); corrected to actual positions (454/283). No other deviations.
- **Verification**: Test count 830 before = 830 after. All tests pass. Zero clippy warnings.
