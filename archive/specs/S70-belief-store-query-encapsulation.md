# S70: Belief Store Query Encapsulation

## Summary

Add missing read and targeted-mutation methods to `AgentBeliefStore` so that `perception.rs` (and future cross-crate consumers) access beliefs through the struct's public API rather than reaching into internal fields. Currently `perception.rs` bypasses the API in ~24 locations across production and test code, coupling it to the struct's field layout across a crate boundary.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (new accessor and mutation methods on `AgentBeliefStore`)
- `worldwake-systems` (update `perception.rs` to use the new API)

## Dependencies

None. `AgentBeliefStore` and `perception.rs` exist and are stable.

## Design Goals

- Every cross-crate access to `AgentBeliefStore` goes through a named method, not direct field access
- No behavioral change: all existing tests pass without modification
- New methods are thin wrappers — no new logic, no new allocation, no new fallibility
- Targeted scope: only `perception.rs` is updated in this spec; other consumers are out of scope

## Non-Goals

- Changing field visibility (`pub` to `pub(crate)`) — would cascade to other crates; deferred
- Adding new belief storage capabilities or changing belief semantics
- Refactoring the `RuntimeBeliefView` trait surface (see assessment report for that discussion)
- Cleaning up other crates' direct field accesses — this spec targets `perception.rs` only

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P14 (World State != Belief State) | Unchanged — methods return belief state, not world state |
| P26 (Systems Through State) | Strengthened — cleaner API boundary between core belief storage and systems that write beliefs |
| P28 (No Backward Compat) | Aligned — no compatibility wrappers; direct field accesses are replaced, not wrapped |

## Deliverables

### 1. New Query Methods on `AgentBeliefStore`

File: `crates/worldwake-core/src/belief.rs`

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

### 2. New Targeted Mutation Methods on `AgentBeliefStore`

File: `crates/worldwake-core/src/belief.rs`

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

### 3. Updated Production Code in `perception.rs`

File: `crates/worldwake-systems/src/perception.rs`

Replace 4 production-code field accesses:

| Line | Current | Replacement |
|------|---------|-------------|
| 339 | `store.institutional_beliefs.get(&key).cloned()` | `store.get_institutional_beliefs(&key).map(\|s\| s.to_vec())` |
| 388-390 | `store.known_entities.get_mut(subject)` + manual activity set | `store.update_believed_activity(subject, next_activity)` |
| 396-398 | `store.known_entities.get_mut(subject)` + `.take()` | `store.clear_believed_activity(subject)` |
| 463 | `for (subject, belief) in &store.known_entities` | `for (subject, belief) in store.iter_known_entities()` |
| 603-605 | `store.institutional_beliefs.get(&key).is_some_and(...)` | `store.get_institutional_beliefs(&key).is_some_and(...)` |

### 4. Updated Test Code in `perception.rs`

Replace ~19 test-code field accesses with the new query methods:

- `store.entity_claims.get(&id)` -> `store.get_entity_claims(&id)`
- `store.social_observations.iter().any(...)` -> `store.iter_social_observations().any(...)`
- `store.institutional_beliefs.get(&key)` -> `store.get_institutional_beliefs(&key)`
- `store.known_entities.values().find(...)` -> `store.iter_known_entities().find_map(...)`
- `store.known_entities.is_empty()` -> `store.iter_known_entities().next().is_none()`

## Stored State vs. Derived Read-Model

- **Stored state**: `AgentBeliefStore` fields (unchanged by this spec)
- **Derived read-model**: None added. All new methods are direct pass-through accessors.

## FND-01 Section H

### Information-path analysis
Not applicable — no new information paths. This spec changes how code accesses existing belief state, not how beliefs are acquired or travel.

### Positive-feedback analysis
No amplifying loops introduced.

### Concrete dampeners
Not applicable.

## SystemFn Integration

No new system functions. `perception.rs` continues to run at its existing tick position.

## Component Registration

No new components.

## Cross-System Interactions

Unchanged. The API methods expose the same data that was previously accessed via fields. No new cross-system coupling.
