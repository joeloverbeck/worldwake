# E07ACTFRA-003: ActionDef + ActionDefRegistry

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines the action definition model
**Deps**: E07ACTFRA-001 (IDs), E07ACTFRA-002 (semantic types)

## Problem

Every action in the simulation must be defined by a static `ActionDef` that encodes all 10 semantics from spec 3.7. These definitions are registered in a deterministic static order. The registry provides lookup by `ActionDefId` and deterministic iteration.

## Assumption Reassessment (2026-03-09)

1. Spec 3.7 requires 10 semantics: actor constraints, targets, preconditions, reservation requirements, duration, interruptibility, commit conditions, effects (handler), visibility, causal event emission.
2. `VisibilitySpec` exists in `worldwake-core/src/visibility.rs` — use it directly.
3. `EventTag` exists in `worldwake-core/src/event_tag.rs` — use `BTreeSet<EventTag>` for causal tags.
4. `ActionHandlerId` from E07ACTFRA-001 links to effect logic (the handler).
5. Action definitions are static data — they do not change at runtime.
6. `worldwake-sim` already has the six semantic schema types from E07ACTFRA-002, plus the handler registry from E07ACTFRA-005. This ticket should compose those existing building blocks rather than introduce parallel variants.
7. At the time this ticket was implemented, `ActionInstance` from E07ACTFRA-004 still stored `handler_id` in addition to `def_id`. That duplication was identified here as a broader E07 architectural question rather than something to half-solve inside this ticket. A same-day follow-up later removed the redundant field from `ActionInstance`, leaving `ActionDef` as the authoritative handler link.

## Architecture Check

1. `ActionDef` is a plain struct with no optional shortcut fields — all 10 semantics are mandatory. This prevents "default away" mistakes.
2. `ActionDefRegistry` uses a `Vec<ActionDef>` indexed by `ActionDefId(u32)` for O(1) lookup with deterministic insertion-order iteration.
3. No `HashMap` — the registry is an indexed vector.
4. Because `ActionDef` embeds `id`, the registry should validate deterministic IDs rather than inventing or mutating them. Static registration order is the source of truth; `register()` merely enforces it.
5. This ticket is beneficial relative to the surrounding architecture because the action framework had IDs, semantic atoms, handler dispatch, and active-instance state, but no single definition object that bound those pieces into a durable schema. Adding `ActionDef` closed that gap cleanly. The nearby `handler_id` duplication between `ActionDef` and `ActionInstance` was correctly identified here as a separate architectural smell, and a later follow-up removed it across E07 instead of half-fixing it locally.

## What to Change

### 1. Create `worldwake-sim/src/action_def.rs`

Define `ActionDef`:
```rust
pub struct ActionDef {
    pub id: ActionDefId,
    pub name: String,
    pub actor_constraints: Vec<Constraint>,
    pub targets: Vec<TargetSpec>,
    pub preconditions: Vec<Precondition>,
    pub reservation_requirements: Vec<ReservationReq>,
    pub duration: DurationExpr,
    pub interruptibility: Interruptibility,
    pub commit_conditions: Vec<Precondition>,
    pub visibility: VisibilitySpec,
    pub causal_event_tags: BTreeSet<EventTag>,
    pub handler: ActionHandlerId,
}
```

Must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. Create `worldwake-sim/src/action_def_registry.rs`

Define `ActionDefRegistry`:
- `register(def: ActionDef) -> ActionDefId` — appends after validating that `def.id` matches the next sequential index, then returns `def.id`
- `get(id: ActionDefId) -> Option<&ActionDef>` — O(1) lookup
- `iter() -> impl Iterator<Item = &ActionDef>` — deterministic insertion-order iteration
- `len()` / `is_empty()`

The registry validates that `def.id` matches the next sequential index on registration. It does not rewrite IDs.

Must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 3. Update `worldwake-sim/src/lib.rs`

Declare modules, re-export `ActionDef` and `ActionDefRegistry`.

## Files to Touch

- `crates/worldwake-sim/src/action_def.rs` (new)
- `crates/worldwake-sim/src/action_def_registry.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- Evaluating constraints, preconditions, or commit conditions (E07ACTFRA-007/008)
- ActionInstance or active action state (E07ACTFRA-004)
- Handler execution logic (E07ACTFRA-005)
- KnowledgeView (E07ACTFRA-006)
- Populating the registry with concrete game actions (later epics)

## Acceptance Criteria

### Tests That Must Pass

1. `ActionDef` exposes the exact required field set from the E07 corrected spec: `id`, `name`, the 10 semantics, and no optional shortcut fields for omitting a semantic
2. `ActionDef` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
3. `ActionDefRegistry` round-trips through bincode with registered defs preserved
4. `register()` accepts only sequential `ActionDefId` values starting from 0 and returns the validated embedded ID
5. `get()` returns the correct def for each registered ID
6. `iter()` returns defs in registration order
7. Registration panics or errors if `def.id` does not match expected next index
8. All ten semantics are required — no field is `Option` (type model enforces completeness)
9. Existing suite: `cargo test --workspace`

### Invariants

1. No `HashMap` or `HashSet` in registry storage
2. ActionDef fields are not optional — all 10 semantics are mandatory
3. Registry iteration order equals registration order (deterministic)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_def.rs` — trait assertions, required-field construction/destructuring coverage, no-`Option` semantics coverage, bincode round-trip
2. `crates/worldwake-sim/src/action_def_registry.rs` — register/get/iter, sequential embedded ID validation, order preservation, bincode round-trip, mismatched-ID rejection

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `crates/worldwake-sim/src/action_def.rs` with a serializable `ActionDef` that binds the six semantic schema types, visibility, causal event tags, and handler linkage into one durable definition object.
  - Added `crates/worldwake-sim/src/action_def_registry.rs` with deterministic `Vec<ActionDef>` storage, O(1) lookup by `ActionDefId`, insertion-order iteration, and sequential embedded-ID validation.
  - Updated `crates/worldwake-sim/src/lib.rs` to expose the new public modules and re-exports.
  - Narrowly fixed pre-existing test-only Clippy issues in the handler modules so the required workspace lint pass succeeds without changing production signatures.
- Deviations from original plan:
  - Corrected the ticket before implementation to remove the brittle “exact field count” requirement and to make the registry contract consistent: embedded `ActionDefId` values are validated, not assigned or rewritten by the registry.
  - Chose fail-fast validation for mismatched IDs because registration is deterministic bootstrap code, not runtime user input. That keeps the API small and honest.
  - Preserved the existing E07 design where both `ActionDef` and `ActionInstance` reference handler linkage, but documented it as an architectural smell to revisit across the broader action framework instead of half-solving it inside this ticket.
- Verification results:
  - `cargo fmt --all --check` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed

Outcome amended: 2026-03-09
- The follow-up architectural cleanup referenced here was implemented: `ActionInstance` no longer stores redundant handler linkage.
- `ActionDef` is now the sole schema-level owner of handler selection, and active execution state refers only to `def_id`.
