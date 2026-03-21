# E16CINSBELRECCON-010: PlanningSnapshot + PlanningState Institutional Belief Fields

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend planning snapshot/state in worldwake-ai
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-003, E16CINSBELRECCON-009

## Problem

The GOAP plan search operates on `PlanningSnapshot` (immutable read-only state) and `PlanningState` (mutable hypothetical state). For the AI to reason about institutional beliefs during planning, both must carry institutional belief data. Without this, the planner cannot evaluate goals that depend on institutional knowledge (e.g., "who holds the office?") or hypothesize the result of consulting a record.

## Assumption Reassessment (2026-03-21)

1. `PlanningSnapshot` (planning_snapshot.rs:193-211) currently has `actor_support_declarations`, `office_support_declarations`. These are live truth reads — the migration will replace them with belief-derived data.
2. `PlanningState` (planning_state.rs:36-52) has `support_declaration_overrides` — the pattern for institutional_belief_overrides follows this.
3. The snapshot is built at the start of planning from the actor's belief view. The new field `actor_institutional_beliefs` is captured from `AgentBeliefStore` derivation helpers (ticket -009).
4. `PlanningState` overrides are populated when hypothetical ConsultRecord transitions fire during search.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. `actor_support_declarations` and `office_support_declarations` currently read live truth. This ticket replaces them with belief-derived data. The old fields remain temporarily until ticket -014 removes the live helper seam.
12. N/A.

## Architecture Check

1. Follows the existing override pattern (snapshot provides base, state provides overrides, read checks overrides first). No novel abstraction needed.
2. No backward-compatibility shims — but the old support_declaration fields remain until ticket -014 completes the migration.

## Verification Layers

1. Snapshot captures actor's institutional beliefs at build time → unit test
2. PlanningState overrides take precedence over snapshot → unit test
3. Hypothetical ConsultRecord populates overrides → unit test
4. Read path: overrides → snapshot → Unknown → unit test chain

## What to Change

### 1. Extend `PlanningSnapshot` in `planning_snapshot.rs`

Add field:
```rust
pub(crate) actor_institutional_beliefs: BTreeMap<InstitutionalBeliefKey, InstitutionalBeliefRead<...>>,
```

Populate at snapshot build time by calling `AgentBeliefStore` derivation helpers for all institutional belief keys the actor has.

### 2. Extend `PlanningState` in `planning_state.rs`

Add field:
```rust
institutional_belief_overrides: BTreeMap<InstitutionalBeliefKey, InstitutionalBeliefRead<...>>,
```

### 3. Add institutional belief query methods on `PlanningState`

Methods that read overrides first, then fall back to snapshot:
```rust
pub fn believed_office_holder(&self, office: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
pub fn believed_support_declaration(&self, office: EntityId, supporter: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
```

### 4. Add override mutation method for hypothetical ConsultRecord

```rust
pub fn override_institutional_belief(&mut self, key: InstitutionalBeliefKey, value: InstitutionalBeliefRead<...>);
```

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add `actor_institutional_beliefs` field, populate at build time)
- `crates/worldwake-ai/src/planning_state.rs` (modify — add `institutional_belief_overrides` field, query methods, override mutation)

## Out of Scope

- Replacing `actor_support_declarations` / `office_support_declarations` with belief-derived data (ticket -014)
- PlannerOpKind::ConsultRecord (ticket -011)
- Candidate generation (ticket -012)
- Ranking changes (ticket -013)
- Failure handling (ticket -013)

## Acceptance Criteria

### Tests That Must Pass

1. `PlanningSnapshot` captures institutional beliefs from actor's belief store at build time
2. `PlanningState::believed_office_holder()` returns snapshot value when no override exists
3. `PlanningState::believed_office_holder()` returns override value when override exists
4. `PlanningState::believed_office_holder()` returns `Unknown` when neither snapshot nor override has data
5. `override_institutional_belief()` correctly populates overrides
6. Multiple overrides can coexist without interference
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Snapshot is immutable after construction — overrides live only in PlanningState
2. Override precedence: PlanningState > PlanningSnapshot > Unknown
3. No live world reads for institutional facts in planning path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — snapshot build captures beliefs
2. `crates/worldwake-ai/src/planning_state.rs` — override precedence, query methods

### Commands

1. `cargo test -p worldwake-ai planning_snapshot`
2. `cargo test -p worldwake-ai planning_state`
3. `cargo clippy --workspace && cargo test --workspace`
