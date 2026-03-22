# E16CINSBELRECCON-009: Add InstitutionalBeliefRead Derivation Helpers to AgentBeliefStore

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new `AgentBeliefStore` query helpers in `worldwake-core`
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-003

## Problem

`InstitutionalBeliefRead<T>` already exists in `worldwake-core`, and `AgentBeliefStore` already stores raw `Vec<BelievedInstitutionalClaim>` per `InstitutionalBeliefKey`. What is still missing is the derivation layer that turns those raw claim vectors into `Unknown` / `Certain` / `Conflicted` reads without duplicating conflict logic in every consumer.

The original ticket overstated the missing architecture and incorrectly bundled place-scoped record discovery into `AgentBeliefStore`. `consultable_records_at(place)` depends on runtime place/entity visibility, not just stored institutional claims, so it should stay outside this ticket.

## Assumption Reassessment (2026-03-22)

1. `InstitutionalBeliefRead<T>` already exists in [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs), and `AgentBeliefStore.institutional_beliefs` already exists in [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs). The missing work is derivation helpers on `AgentBeliefStore`, not introduction of the enum or storage field.
2. The live spec surface in §9 of [specs/E16c-institutional-beliefs-and-record-consultation.md](/home/joeloverbeck/projects/worldwake/specs/E16c-institutional-beliefs-and-record-consultation.md) describes a broader `worldwake-sim` query seam, but this ticket should only deliver the core read-derivation substrate that those runtime helpers will call.
3. `consultable_records_at(place)` is not a valid `AgentBeliefStore` responsibility. `BelievedEntityState` in [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) does not carry entity kind or `RecordData`, and place-scoped consultability currently belongs to the runtime belief view surface (`record_data`, `entity_kind`, `entities_at`) in [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) and [crates/worldwake-sim/src/per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs).
4. Existing focused coverage confirms storage and record infrastructure already landed: `belief::tests::record_institutional_belief_enforces_capacity_deterministically`, `belief::tests::agent_belief_store_roundtrips_through_bincode_with_institutional_beliefs`, and multiple `institutional::tests::*` record tests are present in `worldwake-core`. No existing focused test covers `Unknown` / `Certain` / `Conflicted` derivation because those helpers do not exist yet. I verified with `cargo test -p worldwake-core -- --list | rg "belief|institution|record|support|office|faction"`.
5. The current runtime architecture still leaks live institutional truth through `office_holder`, `factions_of`, `support_declaration`, and `support_declarations_for_office` on the belief-view seam in [crates/worldwake-sim/src/per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) and snapshot capture in [crates/worldwake-ai/src/planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs). That replacement remains desirable, but it is a larger cross-crate seam change than the original ticket effort and is not required to land the core derivation substrate cleanly.
6. This is not a planner or ordering ticket. No `GoalKind`, action lifecycle ordering, or stale-request boundary is under test here.
7. Mismatch corrected: remove `consultable_records_at(place)` from this ticket, keep runtime seam replacement out of scope, and focus this ticket on pure `AgentBeliefStore` derivation methods only.

## Architecture Check

1. Putting derivation on `AgentBeliefStore` is cleaner than teaching every consumer to interpret raw claim vectors. The store owns the data model, so it should also own normalization of agreement versus contradiction.
2. Keeping `consultable_records_at(place)` out of `AgentBeliefStore` is the cleaner boundary. Record discovery depends on visible place-local world artifacts, not just stored institutional claims, so moving it into the store would blur the line between pure belief data and runtime world access.
3. This ticket deliberately does not add backwards-compatibility aliases or duplicate helper paths. It adds the core substrate that later `worldwake-sim` helpers can call directly.

## Verification Layers

1. No institutional claims stored for a queried subject -> focused unit test on `AgentBeliefStore`
2. Matching stored claims collapse to `Certain(...)` regardless of source or tick -> focused unit test on `AgentBeliefStore`
3. Disagreeing stored claims become `Conflicted(...)` with deterministic value ordering -> focused unit test on `AgentBeliefStore`
4. Membership derivation filters by `(faction, member)` rather than treating unrelated claims under the same faction key as conflicts -> focused unit test on `AgentBeliefStore`
5. Single-layer ticket: no action trace, decision trace, or event-log mapping is required because the contract is pure read derivation with no runtime mutation.

## What to Change

### 1. Add institutional read helpers to `AgentBeliefStore`

Add these pure helpers in [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs):

```rust
pub fn believed_office_holder(&self, office: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
pub fn believed_membership(&self, faction: EntityId, member: EntityId) -> InstitutionalBeliefRead<bool>;
pub fn believed_support_declaration(
    &self,
    office: EntityId,
    supporter: EntityId,
) -> InstitutionalBeliefRead<Option<EntityId>>;
pub fn believed_support_declarations_for_office(
    &self,
    office: EntityId,
) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)>;
```

Rules:

- absent matching claims -> `Unknown`
- one or more matching claims with the same effective value -> `Certain(value)`
- two or more matching claims with distinct effective values -> `Conflicted(values)`
- agreement/conflict compares effective values only, not source or learned tick
- malformed claim/key mismatches should be ignored rather than panicking

### 2. Keep place-scoped record discovery out of scope

Do not add `consultable_records_at(place)` to `AgentBeliefStore` in this ticket. That helper belongs with the runtime belief view once the broader `E16c` seam replacement is tackled.

### 3. Add focused derivation coverage

Extend the `belief.rs` test module with focused cases for office holder, faction membership, support declaration, and per-office support aggregation.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- `worldwake-sim` trait additions for the full institutional belief query surface
- replacing live institutional reads in `PerAgentBeliefView` / `PlanningSnapshot`
- `consultable_records_at(place)` runtime discovery
- planner behavior, candidate generation, and contradiction-tolerance policy

## Acceptance Criteria

### Tests That Must Pass

1. `believed_office_holder` returns `Unknown`, `Certain(Some(holder))`, `Certain(None)`, and `Conflicted(...)` correctly
2. `believed_membership` derives membership for one `(faction, member)` pair without treating other members in the same faction key as conflicts
3. `believed_support_declaration` and `believed_support_declarations_for_office` derive deterministic reads from stored support claims
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Derivation helpers are pure reads and do not mutate `AgentBeliefStore`
2. Agreement and conflict are determined by effective institutional value only
3. `Unknown` means no matching claims for the queried subject, not merely stale or low-confidence claims

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — add focused derivation tests for office holder unknown/certain/conflicted cases
2. `crates/worldwake-core/src/belief.rs` — add focused membership filtering tests so unrelated faction-member claims do not pollute one member read
3. `crates/worldwake-core/src/belief.rs` — add support declaration aggregation tests for per-supporter reads and deterministic office-wide grouping

### Commands

1. `cargo test -p worldwake-core belief::tests::believed_support_declarations_for_office_groups_reads_by_supporter`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-22
- Actual changes:
  - added `AgentBeliefStore` institutional read helpers for office holder, membership, support declaration, and per-office support aggregation in `worldwake-core`
  - added focused derivation tests covering unknown, certain, conflicted, member filtering, malformed-claim tolerance, and deterministic support aggregation
  - corrected the ticket scope to keep `consultable_records_at(place)` out of `AgentBeliefStore`; that runtime/place-discovery seam remains follow-up work
  - applied two minimal non-semantic lint fixes outside the ticket’s functional scope so `cargo clippy --workspace --all-targets -- -D warnings` could pass
- Deviations from original plan:
  - removed the proposed `consultable_records_at(place)` helper from this ticket because it is not an `AgentBeliefStore` responsibility under the current architecture
  - did not replace the broader `worldwake-sim` live institutional helper seam in this ticket; this ticket now delivers only the core derivation substrate
- Verification results:
  - `cargo test -p worldwake-core belief::tests::believed_support_declarations_for_office_groups_reads_by_supporter` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
