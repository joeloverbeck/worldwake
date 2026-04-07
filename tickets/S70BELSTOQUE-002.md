# S70BELSTOQUE-002: Migrate perception.rs to AgentBeliefStore API

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S70BELSTOQUE-001

## Problem

`perception.rs` in `worldwake-systems` directly accesses `AgentBeliefStore` internal fields in ~24 locations (5 production, ~19 test). This couples the perception system to the belief store's field layout across a crate boundary. After S70BELSTOQUE-001 adds the named API methods, this ticket mechanically replaces every direct field access with the corresponding method call. No behavioral change.

## Assumption Reassessment (2026-04-07)

1. Production field accesses confirmed at exact line numbers in `crates/worldwake-systems/src/perception.rs`: line 339 (`institutional_beliefs.get`), lines 388-390 (`known_entities.get_mut` + activity set), lines 396-398 (`known_entities.get_mut` + `.take()`), line 463 (`&store.known_entities` iteration), lines 603-605 (`institutional_beliefs.get` + `is_some_and`). All 5 verified via read.
2. Test-code field accesses confirmed: ~19 occurrences across `entity_claims` (2), `social_observations` (7), `institutional_beliefs` (8), `known_entities` (2). Verified via grep.
3. Total: 24 direct field accesses in perception.rs. Matches spec's corrected "~24" count.
4. No other files in `worldwake-systems/src/perception.rs` are in scope. Other crates' field accesses (worldwake-ai, worldwake-cli, other worldwake-systems files) are explicitly out of scope per spec Non-Goals.
5. The replacement API methods (`iter_known_entities`, `get_entity_claims`, `iter_social_observations`, `get_institutional_beliefs`, `update_believed_activity`, `clear_believed_activity`) will exist after S70BELSTOQUE-001 is implemented.
6. Single-layer ticket (worldwake-systems only, mechanical replacements). No cross-system or mixed-layer boundary under audit.

## Architecture Check

1. Replacing direct field access with method calls is strictly mechanical — each replacement maps one-to-one from a field operation to the equivalent API method. The methods return the same types (or borrowed equivalents like `&[T]` instead of `&Vec<T>`), so downstream code remains unchanged except for minor type adaptations (e.g., `.cloned()` → `.map(|s| s.to_vec())` where the return type changes from `Option<&Vec<T>>` to `Option<&[T]>`).
2. No backward-compatibility shims. Direct field accesses are replaced in-place, not wrapped.

## Verification Layers

1. All ~24 field accesses replaced → no remaining `store.known_entities`, `store.entity_claims`, `store.social_observations`, `store.institutional_beliefs` in perception.rs (grep verification)
2. No behavioral change → all existing perception tests pass without modification to assertions
3. Single-layer ticket (mechanical migration in one file); additional layer mapping is not applicable.

## What to Change

### 1. Replace production-code field accesses

File: `crates/worldwake-systems/src/perception.rs`

| Line | Current | Replacement |
|------|---------|-------------|
| 339 | `store.institutional_beliefs.get(&key).cloned()` | `store.get_institutional_beliefs(&key).map(\|s\| s.to_vec())` |
| 388-390 | `store.known_entities.get_mut(subject)` + manual activity set | `store.update_believed_activity(subject, next_activity)` — use return value for `changed` flag |
| 396-398 | `store.known_entities.get_mut(subject)` + `.take()` | `store.clear_believed_activity(subject)` — use return value for `changed` flag |
| 463 | `for (subject, belief) in &store.known_entities` | `for (subject, belief) in store.iter_known_entities()` |
| 603-605 | `store.institutional_beliefs.get(&key).is_some_and(...)` | `store.get_institutional_beliefs(&key).is_some_and(...)` |

Note: Lines 388-398 involve a block that reads `current_activity` via `store.get_entity(subject)` then conditionally mutates. The `update_believed_activity` method encapsulates the comparison-and-set pattern. The `clear_believed_activity` method encapsulates the take-and-check pattern. Both return `bool` matching the existing `changed` flag logic.

### 2. Replace test-code field accesses

File: `crates/worldwake-systems/src/perception.rs`

Apply these mechanical replacements across all `#[cfg(test)]` code:

- `store.entity_claims.get(&id)` → `store.get_entity_claims(&id)` (note: returns `Option<&[EntityBeliefClaim]>` instead of `Option<&Vec<EntityBeliefClaim>>` — downstream `.iter()` calls work on slices, but if any test calls `.len()` on the inner Vec, it also works on slices)
- `store.social_observations.iter().any(...)` → `store.iter_social_observations().any(...)`
- `store.institutional_beliefs.get(&key)` → `store.get_institutional_beliefs(&key)` (same slice adaptation as entity_claims)
- `store.known_entities.values().find(...)` → `store.iter_known_entities().find_map(|(_, v)| ...)` (adapts from values iterator to key-value iterator)
- `store.known_entities.is_empty()` → `store.iter_known_entities().next().is_none()`

For each replacement, verify the downstream chain compiles with the new return type. Slices (`&[T]`) support `.iter()`, `.len()`, `.is_empty()`, indexing, and pattern matching the same as `&Vec<T>` references in nearly all contexts.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)

## Out of Scope

- Migrating field accesses in other files (`display.rs`, `observer.rs`, `investigate_actions.rs`, `tell_actions.rs`, etc.)
- Changing field visibility on `AgentBeliefStore`
- Modifying any test assertions — only the access pattern changes, not the expected values
- Adding new tests beyond what already exists

## Acceptance Criteria

### Tests That Must Pass

1. All existing perception tests pass unchanged: `cargo test -p worldwake-systems -- perception`
2. No direct field accesses to `store.known_entities`, `store.entity_claims`, `store.social_observations`, or `store.institutional_beliefs` remain in `perception.rs` (verified by grep)
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Zero behavioral change — every replacement is semantically equivalent to the original field access
2. All existing test assertions pass without modification (tests validate behavior, not access patterns)

## Test Plan

### New/Modified Tests

1. None — mechanical migration ticket; all existing perception tests serve as the regression suite. No assertion changes needed.

### Commands

1. `cargo test -p worldwake-systems -- perception`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test -p worldwake-systems`
3. `scripts/verify.sh`
