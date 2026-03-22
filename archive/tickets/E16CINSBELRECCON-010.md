# E16CINSBELRECCON-010: PlanningSnapshot + PlanningState Institutional Belief Fields

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend planning snapshot/state in worldwake-ai
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-003, E16CINSBELRECCON-009

## Problem

The GOAP plan search operates on `PlanningSnapshot` (immutable read-only state) and `PlanningState` (mutable hypothetical state). For the AI to reason about institutional beliefs during planning, both must carry institutional belief data. Without this, the planner cannot evaluate goals that depend on institutional knowledge (e.g., "who holds the office?") or hypothesize the result of consulting a record.

## Assumption Reassessment (2026-03-22)

1. `PlanningSnapshot` (`crates/worldwake-ai/src/planning_snapshot.rs`) currently captures `actor_support_declarations` and `office_support_declarations` through `RuntimeBeliefView::support_declaration()` / `support_declarations_for_office()`. In the live runtime implementation (`crates/worldwake-sim/src/per_agent_belief_view.rs`) those methods still read authoritative world state for public institutional facts, so the current planning substrate is still on the legacy live-helper seam.
2. Mismatch + correction: `PlanningState.support_declaration_overrides` is not an institutional-belief override pattern. It models hypothetical authoritative declaration results inside search (`DeclareSupport`, `Bribe`, `Threaten`) and must stay distinct from new institutional-belief overrides. Reusing that map would collapse belief state and hypothetical world state into one substrate.
3. `AgentBeliefStore` derivation helpers for institutional reads already exist in `crates/worldwake-core/src/belief.rs`: `believed_office_holder()`, `believed_support_declaration()`, and `believed_support_declarations_for_office()`. This ticket should reuse those helpers instead of re-deriving institutional reads inside worldwake-ai.
4. New prerequisite for this ticket: `GoalBeliefView` / `RuntimeBeliefView` must expose belief-backed institutional read methods. Without that trait-boundary extension, `PlanningSnapshot` cannot capture institutional beliefs cleanly and would have to keep depending on the legacy live-helper shape.
5. `PlanningState` institutional-belief overrides are populated by hypothetical consultation/search transitions later in the E16c chain. This ticket owns the storage/query surface now so later tickets can write into it without inventing another seam.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. The live architectural gap is narrower than the original ticket text implied: the core institutional belief model and consult-record action already exist, but the planning substrate still lacks first-class belief-backed institutional read state. That is the correct scope for this ticket.
12. Additional migration note: the old snapshot support fields and the old trait methods still exist after this ticket because ticket `-014` owns final seam removal. This ticket should add the new belief-backed substrate and stop extending the legacy path further.

## Architecture Check

1. The clean architecture is: core derives institutional reads, belief-view traits expose those reads, `PlanningSnapshot` caches belief-backed base values, and `PlanningState` layers explicit institutional-belief overrides on top. That preserves the belief boundary and avoids leaking `AgentBeliefStore` internals into AI call sites.
2. Belief overrides must be stored separately from hypothetical world-state overrides. Mixing them would make search unable to distinguish “the actor believes X” from “the hypothetical world now contains X”.
3. No backward-compatibility shims. The legacy live-helper seam remains only as existing migration debt already owned by ticket `-014`; this ticket must not create any new alias path or fallback abstraction.

## Verification Layers

1. `PerAgentBeliefView` belief-backed institutional queries delegate to `AgentBeliefStore` derivation helpers → worldwake-sim unit test
2. `PlanningSnapshot` captures belief-backed office/support reads at build time → worldwake-ai unit test
3. `PlanningState` institutional-belief overrides take precedence over snapshot → worldwake-ai unit test
4. Read path: overrides → snapshot → Unknown → worldwake-ai unit test chain

## What to Change

### 1. Extend belief-view traits with institutional read methods

Add belief-backed institutional query methods to `GoalBeliefView` / `RuntimeBeliefView` for the reads this ticket needs:
```rust
fn believed_office_holder(&self, office: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
fn believed_support_declaration(
    &self,
    office: EntityId,
    supporter: EntityId,
) -> InstitutionalBeliefRead<Option<EntityId>>;
fn believed_support_declarations_for_office(
    &self,
    office: EntityId,
) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)>;
```

Implement them in `PerAgentBeliefView` by delegating to the existing `AgentBeliefStore` derivation helpers.

### 2. Extend `PlanningSnapshot` in `planning_snapshot.rs`

Add belief-backed institutional caches for the planning reads needed by E16c political search:
```rust
pub(crate) actor_office_holder_beliefs:
    BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
pub(crate) office_support_declaration_beliefs:
    BTreeMap<EntityId, Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)>>,
```

Populate them at snapshot build time through the new belief-view methods.

### 3. Extend `PlanningState` in `planning_state.rs`

Add explicit institutional-belief override maps:
```rust
office_holder_belief_overrides:
    BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
support_declaration_belief_overrides:
    BTreeMap<(EntityId, EntityId), InstitutionalBeliefRead<Option<EntityId>>>,
```

### 4. Add institutional belief query methods on `PlanningState`

Methods that read overrides first, then fall back to snapshot:
```rust
pub fn believed_office_holder(&self, office: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
pub fn believed_support_declaration(&self, office: EntityId, supporter: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
pub fn believed_support_declarations_for_office(
    &self,
    office: EntityId,
) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)>;
```

### 5. Add override mutation methods for hypothetical consultation/search transitions

```rust
pub fn override_office_holder_belief(
    &mut self,
    office: EntityId,
    value: InstitutionalBeliefRead<Option<EntityId>>,
);
pub fn override_support_declaration_belief(
    &mut self,
    office: EntityId,
    supporter: EntityId,
    value: InstitutionalBeliefRead<Option<EntityId>>,
);
```

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add institutional belief read methods to trait boundary)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement belief-backed institutional queries)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add belief-backed office/support caches, populate at build time)
- `crates/worldwake-ai/src/planning_state.rs` (modify — add institutional-belief override maps, query methods, override mutation)

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
5. `PlanningState::believed_support_declaration()` returns snapshot / override / `Unknown` correctly
6. Belief overrides do not interfere with the existing hypothetical support-declaration world-state overrides
7. Existing suites: `cargo test -p worldwake-ai` and `cargo test -p worldwake-sim`

### Invariants

1. Snapshot is immutable after construction — institutional-belief overrides live only in PlanningState
2. Override precedence: PlanningState belief overrides > PlanningSnapshot belief cache > Unknown
3. No new planning-path consumers of the legacy live institutional helper seam
4. Belief-state overrides remain separate from hypothetical authoritative support-declaration state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — belief-backed institutional query tests
2. `crates/worldwake-ai/src/planning_snapshot.rs` — snapshot build captures belief-backed office/support reads
3. `crates/worldwake-ai/src/planning_state.rs` — override precedence, query methods, and separation from support-declaration world-state overrides

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-ai planning_snapshot`
3. `cargo test -p worldwake-ai planning_state`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

Completion date: 2026-03-22

What actually changed:
- Added belief-backed institutional query methods to `GoalBeliefView` and `RuntimeBeliefView` for office-holder and support-declaration reads.
- Implemented those new reads in `PerAgentBeliefView` by delegating to `AgentBeliefStore` derivation helpers in `worldwake-core`.
- Extended `PlanningSnapshot` with belief-backed office-holder and office support-declaration caches while leaving the legacy live-helper fields untouched for later removal in ticket `-014`.
- Extended `PlanningState` with explicit institutional-belief override maps and query helpers, separate from the existing hypothetical authoritative support-declaration override map.
- Added focused tests in `worldwake-sim`, `planning_snapshot`, and `planning_state` covering belief-backed capture, override precedence, and separation of belief overrides from world-state overrides.

Deviations from original plan:
- The original ticket assumed `support_declaration_overrides` provided a reusable override pattern for institutional beliefs. That assumption was wrong in the live code: it models hypothetical authoritative world state, not belief state. The delivered implementation introduced separate belief override storage instead of overloading that map.
- The original ticket also omitted the trait-boundary work required to get belief-backed institutional reads into `PlanningSnapshot` cleanly. The delivered implementation added that trait surface rather than threading `AgentBeliefStore` internals directly into worldwake-ai.
- The delivered snapshot/state substrate uses belief-backed office/support read caches, not a raw heterogenous institutional-belief map. That is a smaller and cleaner cut for the reads current E16c planning actually needs.

Verification results:
- `cargo test -p worldwake-sim per_agent_belief_view` ✅
- `cargo test -p worldwake-ai planning_snapshot` ✅
- `cargo test -p worldwake-ai planning_state` ✅
- `cargo test -p worldwake-ai` ✅
- `cargo clippy --workspace` ✅
- `cargo test --workspace` ✅
