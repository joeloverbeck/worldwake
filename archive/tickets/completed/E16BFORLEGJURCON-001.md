# E16BFORLEGJURCON-001: Add OfficeForceProfile and OfficeForceState components

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — core component types, component tables, component schema
**Deps**: E16 (offices exist), E16c (institutional infra exists)

## Problem

Force-succession offices lack explicit per-office timing parameters and temporal continuity tracking. The spec requires `OfficeForceProfile` (policy) and `OfficeForceState` (mutable continuity) as separate components on `EntityKind::Office`.

## Assumption Reassessment (2026-03-22)

1. `OfficeData` and `SuccessionLaw::Force` exist in [`crates/worldwake-core/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/offices.rs). `OfficeForceProfile` and `OfficeForceState` do not exist anywhere in the codebase yet.
2. Authoritative component registration is macro-driven from [`crates/worldwake-core/src/component_schema.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_schema.rs) into [`crates/worldwake-core/src/component_tables.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs), [`crates/worldwake-core/src/world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs), [`crates/worldwake-core/src/world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs), and [`crates/worldwake-core/src/delta.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs). This ticket must update the shared schema, not just add structs.
3. This is still a `worldwake-core` substrate ticket, but it is not architecturally isolated from higher layers. The live force path is the provisional shortcut in [`resolve_force_succession()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs), and current AI candidate generation in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) explicitly omits `GoalKind::ClaimOffice` for `SuccessionLaw::Force`.
4. Not an AI regression ticket, but reassessment confirmed live AI and golden coverage currently encode the old architecture. Focused examples: [`political_candidates_skip_force_law_offices()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) and Scenario 19 in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) both assume force offices do not expose political action surfaces yet.
5. No ordering contract in scope for this ticket. The timing fields introduced here are authoritative data only; they do not yet change `succession_system` ordering or installation semantics.
6. The current hidden heuristic is the single-contender shortcut in [`resolve_force_succession()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs). This ticket does not remove or weaken that heuristic; it adds the force-specific state substrate that later tickets will use to replace it cleanly.
7. N/A — not a stale-request, contested-affordance, or start-failure ticket.
8. Closure boundary here is narrower than the original wording implied: this ticket does not change support declaration, visible-vacancy loss, succession resolution, or `office_holder` mutation. It only adds force-specific authoritative components needed by later force-control tickets.
9. N/A — no `ControlSource`, queue, or runtime-driver manipulation.
10. Current golden isolation for force succession intentionally excludes force-claim actions and planner surfaces; the only live force branch is system-level vacancy resolution after `succession_period_ticks`. This ticket must not silently invalidate that behavior without the later E16b tickets that replace it.
11. Mismatch corrected: the original ticket called this a "pure data-layer addition" and claimed no mismatches. In reality, the live architecture still has system, AI, and golden assumptions around the provisional force shortcut. Scope is therefore corrected to "add the authoritative force-state components and registrations only; do not change force succession behavior, planner surfaces, or institutional claims in this ticket."
12. No cumulative arithmetic is introduced beyond storing tick continuity data. Existing force timing remains driven by `OfficeData.succession_period_ticks` until E16BFORLEGJURCON-005 replaces the shortcut.

## Architecture Check

1. Two separate components are better than further growing `OfficeData`: `OfficeData` remains cross-law office metadata, `OfficeForceProfile` becomes force-only policy, and `OfficeForceState` becomes force-only mutable continuity. That is cleaner than overloading generic office state with force-specific timers and avoids baking the provisional shortcut deeper into the core schema.
2. Controller identity must stay out of `OfficeForceState`. The durable architecture is relation-based control (`office_controller`) plus temporal continuity state, not a duplicated holder/controller field in multiple authoritative locations.
3. No backward-compatibility shims or alias paths. These are net-new force-specific components that prepare later removal of the old shortcut rather than wrapping it.

## Verification Layers

1. `OfficeForceProfile` attached only to `EntityKind::Office` -> focused `worldwake-core` world/schema tests
2. `OfficeForceState` attached only to `EntityKind::Office` -> focused `worldwake-core` world/schema tests
3. New types serialize, deserialize, and participate in typed component/delta plumbing -> focused `worldwake-core` unit tests
4. Shared macro registration reaches `ComponentTables`, `World`, and `WorldTxn` surfaces -> focused `worldwake-core` compile-backed/unit coverage
5. Single-layer implementation ticket. AI, action, and office-resolution behavior verification is intentionally not part of this ticket.

## What to Change

### 1. Define types in `offices.rs`

Add `OfficeForceProfile` and `OfficeForceState` structs as specified:

```rust
pub struct OfficeForceProfile {
    pub uncontested_hold_ticks: NonZeroU32,
    pub vacancy_claim_grace_ticks: NonZeroU32,
    pub challenger_presence_grace_ticks: NonZeroU32,
}

pub struct OfficeForceState {
    pub control_since: Option<Tick>,
    pub contested_since: Option<Tick>,
    pub last_uncontested_tick: Option<Tick>,
}
```

Both must derive `Clone, Debug, Serialize, Deserialize` and implement the `Component` trait.
They should also satisfy the repo's normal authoritative value bounds (`Eq`, `PartialEq`) so they can participate in component values and deltas.

### 2. Register in component tables

Add `office_force_profile` and `office_force_state` storage fields to `ComponentTables` in `component_tables.rs`, following the existing pattern for `OfficeData`.

### 3. Register in component schema

Add entries to `with_component_schema_entries!` with predicate `kind == EntityKind::Office`.

### 4. Expose via World

Add getter/setter methods on `World` for the new components, following the existing `office_data()` / `set_office_data()` pattern.

### 5. Re-export and delta plumbing

Update any shared exports or typed delta/value enums that depend on the authoritative component manifest so the new component kinds are first-class everywhere the schema is projected.

## Files to Touch

- `crates/worldwake-core/src/offices.rs` (modify — add types)
- `crates/worldwake-core/src/component_tables.rs` (modify — add storage fields)
- `crates/worldwake-core/src/component_schema.rs` (modify — register components)
- `crates/worldwake-core/src/world.rs` or relevant world submodule (modify — add accessors)

## Out of Scope

- Relations (`contests_office`, `office_controller`) — that's E16BFORLEGJURCON-002
- WorldTxn helpers — that's E16BFORLEGJURCON-002
- Action payloads/handlers — later tickets
- AI integration — later tickets
- Replacing the provisional force shortcut in `resolve_force_succession()` — that's E16BFORLEGJURCON-005
- Removing `resolve_force_succession` — that's E16BFORLEGJURCON-005
- Migrating force offices off `OfficeData.succession_period_ticks` — that architectural cleanup belongs with the real force-control system once the old shortcut is removed

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `OfficeForceProfile` can be attached to an `EntityKind::Office` entity and read back
2. Unit test: `OfficeForceState` can be attached to an `EntityKind::Office` entity and read back
3. Unit test: attempting to attach either component to a non-Office entity is rejected by schema
4. Unit test: component tables and world transaction typed surfaces recognize both new component kinds
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `OfficeForceProfile` and `OfficeForceState` are only attachable to `EntityKind::Office`
2. All values use integer/newtype types (`NonZeroU32`, `Tick`) — no floats
3. `OfficeForceState` contains only temporal continuity data, never controller identity (Principle 26)
4. No existing tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/offices.rs` — round-trip and trait-bound coverage for `OfficeForceProfile` and `OfficeForceState`
2. `crates/worldwake-core/src/world.rs` — office-only schema enforcement and round-trip access for both new components
3. `crates/worldwake-core/src/component_tables.rs` — typed storage CRUD coverage for both new components
4. `crates/worldwake-core/src/world_txn.rs` — typed component-delta coverage for set/clear mutation helpers generated from the shared schema
5. `crates/worldwake-core/src/delta.rs` — manifest and sample coverage proving the new component kinds participate in typed delta/value enums

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- Actual changes:
  - added `OfficeForceProfile` and `OfficeForceState` in `worldwake-core`
  - registered both components through the shared authoritative component schema so they now flow through `ComponentTables`, `World`, `WorldTxn`, `ComponentKind`, and `ComponentValue`
  - added focused core tests for serialization, office-only schema enforcement, typed storage access, and `WorldTxn` component-delta plumbing
- Deviations from original plan:
  - the ticket was corrected before implementation to acknowledge that current AI and golden coverage still assume the provisional force shortcut; this ticket intentionally did not change force succession behavior
  - verification expanded beyond the original single test module to include `delta.rs` and `world_txn.rs`, because the component manifest fans out through shared macro projections
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
