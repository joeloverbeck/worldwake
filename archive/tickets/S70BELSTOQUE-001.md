# S70BELSTOQUE-001: Add query and mutation methods to AgentBeliefStore

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`AgentBeliefStore` exposes its internal fields as `pub`, and cross-crate consumers (currently `perception.rs` in `worldwake-systems`) reach directly into `known_entities`, `entity_claims`, `social_observations`, and `institutional_beliefs`. This couples consumers to the struct's field layout across a crate boundary. Adding named accessor and targeted-mutation methods provides a stable API surface that the next ticket (S70BELSTOQUE-002) will migrate callers to.

## Assumption Reassessment (2026-04-07)

1. `AgentBeliefStore` is defined at `crates/worldwake-core/src/belief.rs:44` with fields `entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>`, `known_entities: BTreeMap<EntityId, BelievedEntityState>`, `social_observations: Vec<SocialObservation>`, `institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>`. Confirmed via grep.
2. None of the 7 proposed method names (`iter_known_entities`, `get_entity_claims`, `iter_social_observations`, `get_institutional_beliefs`, `has_institutional_belief`, `update_believed_activity`, `clear_believed_activity`) exist on `AgentBeliefStore` today. Confirmed via grep.
3. `BelievedEntityState` has field `believed_activity: Option<BelievedActivity>` at belief.rs:1009. The `update_believed_activity` and `clear_believed_activity` methods correctly target this field.
4. Existing method `get_entity(&self, id: &EntityId) -> Option<&BelievedEntityState>` at belief.rs:126 is complementary (point lookup vs iteration). No naming conflict.
5. All referenced types (`EntityBeliefClaim`, `SocialObservation`, `InstitutionalBeliefKey`, `BelievedInstitutionalClaim`, `BelievedActivity`) exist in `worldwake-core`. No crate boundary issue — all methods live in the same crate as the types.
6. Single-layer ticket (worldwake-core only, no cross-system interaction). No mixed-layer or cross-system boundary under audit.

## Architecture Check

1. Thin pass-through accessors on the owning struct are the idiomatic Rust encapsulation pattern. No new logic, allocation, or fallibility — each method delegates directly to a single field operation. This is strictly cleaner than the status quo (raw field access across crate boundaries) and introduces no new abstractions.
2. No backward-compatibility shims. The new methods are additive; field visibility remains unchanged (deferred per spec Non-Goals).

## Verification Layers

1. Methods return correct types and compile → `cargo build -p worldwake-core`
2. `update_believed_activity` returns true on change, false on no-op or missing entity → focused unit test
3. `clear_believed_activity` returns true when previously Some, false otherwise → focused unit test
4. Single-layer ticket (worldwake-core only); additional layer mapping is not applicable.

## What to Change

### 1. Add query methods to `AgentBeliefStore`

File: `crates/worldwake-core/src/belief.rs`

Add the following 5 query methods in a new `impl AgentBeliefStore` block (or appended to the existing one):

```rust
/// Iterate over all known entity beliefs.
pub fn iter_known_entities(&self) -> impl Iterator<Item = (&EntityId, &BelievedEntityState)> {
    self.known_entities.iter()
}

/// Get the raw entity claims for a subject.
pub fn get_entity_claims(&self, id: &EntityId) -> Option<&[EntityBeliefClaim]> {
    self.entity_claims.get(id).map(Vec::as_slice)
}

/// Iterate over all social observations.
pub fn iter_social_observations(&self) -> impl Iterator<Item = &SocialObservation> {
    self.social_observations.iter()
}

/// Get raw institutional beliefs for a key.
pub fn get_institutional_beliefs(
    &self,
    key: &InstitutionalBeliefKey,
) -> Option<&[BelievedInstitutionalClaim]> {
    self.institutional_beliefs.get(key).map(Vec::as_slice)
}

/// Check whether any institutional belief exists for a key.
pub fn has_institutional_belief(&self, key: &InstitutionalBeliefKey) -> bool {
    self.institutional_beliefs.contains_key(key)
}
```

### 2. Add targeted mutation methods to `AgentBeliefStore`

File: `crates/worldwake-core/src/belief.rs`

Add the following 2 mutation methods:

```rust
/// Update the believed activity for a known entity.
/// Returns `true` if the belief was actually changed.
/// Returns `false` if the entity is not known or the activity was already equal.
pub fn update_believed_activity(
    &mut self,
    id: &EntityId,
    activity: Option<BelievedActivity>,
) -> bool {
    if let Some(belief) = self.known_entities.get_mut(id) {
        if belief.believed_activity != activity {
            belief.believed_activity = activity;
            return true;
        }
    }
    false
}

/// Clear the believed activity for a known entity (set to None).
/// Returns `true` if it was previously Some.
pub fn clear_believed_activity(&mut self, id: &EntityId) -> bool {
    if let Some(belief) = self.known_entities.get_mut(id) {
        return belief.believed_activity.take().is_some();
    }
    false
}
```

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- Changing field visibility (`pub` → `pub(crate)`)
- Migrating any callers to the new methods (that is S70BELSTOQUE-002)
- Adding methods for `told_beliefs`, `heard_beliefs`, or `asked_witnesses` fields
- Modifying belief semantics or storage layout

## Acceptance Criteria

### Tests That Must Pass

1. `update_believed_activity` returns `true` when changing activity on a known entity, `false` when entity unknown or activity unchanged
2. `clear_believed_activity` returns `true` when clearing a Some activity, `false` when already None or entity unknown
3. Query methods return expected data for populated and empty stores
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No behavioral change to any existing code — methods are purely additive
2. All new methods are thin wrappers with no new allocation, logic, or fallibility beyond the underlying collection operations

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` (inline `#[cfg(test)]` module) — unit tests for `update_believed_activity` and `clear_believed_activity` return values covering: known entity with changed activity, known entity with same activity (no-op), unknown entity, clear when Some, clear when None. Query methods are trivial pass-throughs; exercised transitively by S70BELSTOQUE-002's migration.

### Commands

1. `cargo test -p worldwake-core -- believed_activity`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test -p worldwake-core`
3. `scripts/verify.sh`

## Outcome

Completed on 2026-04-08.

- Added 5 query methods (`iter_known_entities`, `get_entity_claims`, `iter_social_observations`, `get_institutional_beliefs`, `has_institutional_belief`) and 2 mutation methods (`update_believed_activity`, `clear_believed_activity`) to `AgentBeliefStore` in `crates/worldwake-core/src/belief.rs`.
- Added 6 focused unit tests covering mutation return-value contracts.
- No behavioral changes to existing code; all methods are purely additive.

## Verification Result

- Passed `cargo test -p worldwake-core -- believed_activity` (8 tests, including 6 new)
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
- Passed `cargo test -p worldwake-core` (full crate suite)
- Pre-existing clippy failure in `worldwake-ai/src/bin/perf_diag.rs` (cast precision loss) — unrelated to this ticket, file was already modified before this session.
