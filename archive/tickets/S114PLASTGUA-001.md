# S114PLASTGUA-001: Plan-guard and expectation core types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new serializable enums in `worldwake-core`; new runtime-only types in `worldwake-ai`.
**Deps**: S114 spec (`specs/S114-plan-step-guards.md`)

## Problem

S114 deliverable D1 introduces the shared type vocabulary that every subsequent plan-guard ticket (003–010) depends on. Splitting the types into a pure additive foundation ticket lets the AI-crate `PlannedStep` extension (003), `ExpectationBasis` variant (004), payload widening (005), and `ActionDef` templates (006) all land in isolation against a stable shared surface.

## Assumption Reassessment (2026-04-21)

1. `ExpectationBasis` lives at `crates/worldwake-core/src/expectation.rs:22` and derives `Copy`; the new `ExpectationKindTag` must remain `Copy` + `Hash` + `Ord` so it can sit inside a future `ExpectationBasis::PlanStepCompletion` variant without regressing existing derives. `StatePredicate` / `ObservationPredicate` carry only `EntityId` / `CommodityKind` / `Quantity` / `BeliefClaimKey` fields — all `Copy` primitives already in core.
2. S114 spec D1 at `specs/S114-plan-step-guards.md:54-139` is the authoritative type listing. Derive propagation rules in the spec: `ExpectationKind` is **not** `Copy`; `PlanGuard` / `PlanExpectation` are `Clone + Debug + Eq + PartialEq` only (runtime-only, never serialized).
3. Shared boundary under audit: the core↔ai seam. Core owns serializable tag/predicate enums (used by `ExpectationMismatchPayload` in ticket 005 and `ExpectationBasis` in ticket 004); ai owns the richer runtime types attached to `PlannedStep` (ticket 003).

## Architecture Check

1. Pure additive — no existing types modified, no existing construction sites touched. Follow-on tickets (003, 004, 005, 006) consume these types but do not modify them further.
2. The core↔ai split mirrors how `MaterializationTag` (core, serializable) and `ExpectedMaterialization` (ai, runtime-only with `HypotheticalEntityId`) are already factored. Same pattern, no novel layering.

## Verification Layers

1. Serialization contract (core-side tag types round-trip through `bincode`) → focused unit test in `plan_step_guards.rs` tests module.
2. Trait bounds (`Copy` on `ExpectationKindTag` / `InvalidatorTag`, non-`Copy` on runtime `ExpectationKind`) → `assert_copy_bounds<T>()` / `assert_value_bounds<T>()` compile-time test helpers matching the pattern in `expectation.rs:193-195`.
3. Single-layer ticket: downstream mapping (event-log delta, action trace) is not applicable — no behavior change, only new symbol definitions.

## What to Change

### 1. Core-side types — `crates/worldwake-core/src/plan_step_guards.rs` (new file)

```rust
use crate::{BeliefClaimKey, CommodityKind, EntityId, EvidenceKind, Quantity};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExpectationKindTag {
    Immediate,
    State,
    Informed,
    Regression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StatePredicate {
    CommodityAtPlaceAtLeast { place: EntityId, kind: CommodityKind, quantity: Quantity },
    EntityAtPlace { entity: EntityId, place: EntityId },
    ActorHoldsCommodity { kind: CommodityKind, min_quantity: Quantity },
    ClaimEstablished { claim: BeliefClaimKey },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ObservationPredicate {
    EntityPerceivedAtPlace { entity: EntityId, place: EntityId },
    EvidencePerceived { kind: EvidenceKind, place: EntityId },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum InvalidatorTag {
    BeliefStatusChange,
    TargetMoved,
    CommodityDepleted,
    NewBlockerRecorded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MismatchDetail {
    GuardInvalidator(InvalidatorTag),
    StateUnmet { predicate: StatePredicate },
    ObservationMissing { predicate: ObservationPredicate },
}
```

Declare and re-export the module in `crates/worldwake-core/src/lib.rs`.

### 2. AI-crate runtime-only types — `crates/worldwake-ai/src/plan_guard.rs` (new file)

```rust
use worldwake_core::{
    BeliefClaimKey, CommodityKind, EntityId, EventTag, ObservationPredicate, Permille, Quantity,
    StatePredicate, Tick,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanGuard {
    pub required_facts: Vec<RequiredFact>,
    pub min_confidence: Permille,
    pub invalidators: Vec<Invalidator>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequiredFact {
    TargetPresent { target: EntityId, at_place: EntityId },
    CommodityAvailable { place: EntityId, kind: CommodityKind, min_quantity: Quantity },
    /// No-op (`false` short-circuit) until a follow-up spec adds
    /// `believed_route_known` on the belief envelope. See S114 Non-Goals.
    RouteKnown { from: EntityId, to: EntityId },
    ResourceAccess { resource: EntityId, agent_holds_permission: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Invalidator {
    BeliefStatusChange { claim: BeliefClaimKey },
    TargetMoved { target: EntityId },
    CommodityDepleted { place: EntityId, kind: CommodityKind },
    NewBlockerRecorded { baseline_tick: Tick },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanExpectation {
    pub kind: ExpectationKind,
    pub observe_by: Option<Tick>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpectationKind {
    Immediate { event_tag: EventTag },
    State { predicate: StatePredicate },
    Informed { observation: ObservationPredicate },
    Regression { predicate: StatePredicate },
}
```

Declare and re-export the module in `crates/worldwake-ai/src/lib.rs`.

## Files to Touch

- `crates/worldwake-core/src/plan_step_guards.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — `pub mod plan_step_guards;` + re-exports)
- `crates/worldwake-ai/src/plan_guard.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — `pub mod plan_guard;` + re-exports)

## Out of Scope

- Attaching `PlanGuard` / `PlanExpectation` to `PlannedStep` (ticket 003).
- `ExpectationBasis::PlanStepCompletion` variant addition (ticket 004).
- Widening `ExpectationMismatchPayload` to carry `MismatchDetail` (ticket 005).
- `ActionDef` template specs and `build_plan_guard` / `build_plan_expectations` (ticket 006).
- Any guard evaluation or expectation emission logic (tickets 007, 009).

## Acceptance Criteria

### Tests That Must Pass

1. Core-side tag types (`ExpectationKindTag`, `InvalidatorTag`, `StatePredicate`, `ObservationPredicate`, `MismatchDetail`) round-trip through `bincode::serialize` / `bincode::deserialize` byte-for-byte.
2. AI-crate runtime types (`PlanGuard`, `PlanExpectation`, `RequiredFact`, `Invalidator`, `ExpectationKind`) compile with the stated derives; compile-time `assert_value_bounds<T: Clone + Debug + Eq + PartialEq>()` helpers pass for all five.
3. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai` stay green.

### Invariants

1. No `Copy` derive on `ExpectationKind` / `PlanGuard` / `PlanExpectation` (they hold `Vec<_>` and may accept richer payloads in future phases).
2. Every core-side tag enum derives `Copy + Hash + Ord` so it stays eligible for use inside a future `ExpectationBasis` variant or `BTreeSet`-keyed ranking arm.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/plan_step_guards.rs` tests module — bincode round-trip for `ExpectationKindTag`, `StatePredicate`, `ObservationPredicate`, `InvalidatorTag`, `MismatchDetail`.
2. `crates/worldwake-ai/src/plan_guard.rs` tests module — compile-time trait-bounds assertions for all runtime types.

### Commands

1. `cargo test -p worldwake-core plan_step_guards`
2. `cargo test -p worldwake-ai plan_guard`
3. `cargo clippy -p worldwake-core -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-21.

- Added the new shared core module at `crates/worldwake-core/src/plan_step_guards.rs` with `ExpectationKindTag`, `StatePredicate`, `ObservationPredicate`, `InvalidatorTag`, and `MismatchDetail`, plus focused bincode/trait-bounds tests.
- Added the new runtime AI module at `crates/worldwake-ai/src/plan_guard.rs` with `PlanGuard`, `RequiredFact`, `Invalidator`, `PlanExpectation`, and `ExpectationKind`, plus focused derive-surface tests.
- Declared and re-exported both modules from the corresponding crate roots so follow-on S114 tickets can consume the shared surface directly.

## Verification Result

- Passed `cargo test -p worldwake-core --lib plan_step_guards::tests::plan_step_guard_core_types_roundtrip_through_bincode -- --exact`
- Passed `cargo test -p worldwake-core --lib plan_step_guards::tests::plan_step_guard_core_types_satisfy_required_bounds -- --exact`
- Passed `cargo test -p worldwake-ai --lib plan_guard::tests::plan_guard_runtime_types_satisfy_required_bounds -- --exact`
- Passed `cargo test -p worldwake-ai --lib plan_guard::tests::plan_guard_runtime_types_support_stated_derives -- --exact`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy -p worldwake-core -p worldwake-ai --all-targets -- -D warnings`

Archival tracking note: the source ticket draft was untracked before archival, so moving it preserved that state. `archive/tickets/S114PLASTGUA-001.md` remains untracked in this worktree, the original `tickets/S114PLASTGUA-001.md` path is gone, and `specs/S114-plan-step-guards.md` was already modified before this implementation and was not edited here.
