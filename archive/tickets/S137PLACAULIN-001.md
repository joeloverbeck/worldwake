# S137PLACAULIN-001: Core type foundations — CausalLink, CausalProvider, PlanningFact, RecordTopic, BreachSignature

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Core type substrate only; no runtime consumers in this ticket. Existing key types gained `Hash` derives where required by the new shared type bounds.
**Deps**: archive/specs/S137-plan-causal-links-and-repair.md (D2, D3, D9)

## Problem

S137's downstream tickets (causal-links field on `PlanGuard`, `RepairMemory` shape migration, `plan_repair` module, decision-trace) all consume five new core types that do not yet exist anywhere in the codebase. Without these types in place first, downstream tickets cannot compile against each other in dependency order.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Four of the five shared core types (`CausalLink`, `CausalProvider`, `RecordTopic`, `BreachSignature`) did not exist anywhere in `crates/` before this ticket. A private `worldwake-ai::search::landmarks::PlanningFact` helper already existed, but it is not the S137 shared core boundary and has no public/runtime dependency relationship with this ticket's `worldwake_core::PlanningFact`. No existing focused/unit, runtime trace/integration, or golden/E2E coverage referenced the new core substrate.
2. Spec `archive/specs/S137-plan-causal-links-and-repair.md` D2, D3, D9 define the proposed shapes, derives, and intended consumers. Per Pre-Process classification (a)+(b) hybrid, these are the (a)-component net-new types.
3. Shared abstraction boundary: the core/ai crate split. `CausalLink` and `CausalProvider` live in core because (a) `BreachSignature` references `InvalidatorTag` (core, `crates/worldwake-core/src/plan_step_guards.rs:38-44`) and (b) downstream tickets need the types referenced from both ai (`PlanGuard.causal_links`) and core (`RepairMemory.repairs` and `RepairAppliedPayload`).

## Architecture Check

1. **Core-residence**: Per `references/worldwake-validation-patterns.md` Core-Side Mirror Enum Pattern, core-resident types whose field types reference higher-crate enums require either a relocation or a mirror. All five types reference only core types (`EntityId`, `Tick`, `Permille`, `CommodityKind`, `BeliefClaimKey`, `EntityBeliefAspect`, `ExpectationId`, `GoalKey`, `InvalidatorTag`) — no mirror needed.
2. **No back-compat shims**: net-new types; no legacy aliases or wrapper types introduced.

## Verification Layers

1. Type-shape invariant → focused unit tests asserting derive bounds (`Copy`, `Clone`, `Debug`, `Eq`, `Ord`, `PartialEq`, `PartialOrd`, `Hash`, `Serialize`, `Deserialize`) and bincode roundtrip per type.
2. Single-layer ticket (type definitions only with no runtime consumers landing in this ticket); additional layer mapping not applicable per precision-rules.md item 6.

## Implemented Changes

### 1. New `crates/worldwake-core/src/causal_link.rs` module

Defined `CausalLink`, `CausalProvider`, `PlanningFact`, `RecordTopic`:

```rust
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CausalLink {
    pub provider: CausalProvider,
    pub fact: PlanningFact,
    pub consumer_step_index: u16,
    pub source_tick: Tick,
    pub confidence: Permille,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CausalProvider {
    PriorStep { step_index: u16 },
    Belief { claim_key: BeliefClaimKey },
    Observation { observed_entity: EntityId, aspect: EntityBeliefAspect },
    Record { record_entity: EntityId, topic: RecordTopic },
    CarriedItem { item_lot: EntityId },
    Expectation { expectation_id: ExpectationId },
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PlanningFact {
    TargetPresent { target: EntityId, at_place: EntityId },
    CommodityAvailable { place: EntityId, kind: CommodityKind, min_quantity: Quantity },
    RouteKnown { from: EntityId, to: EntityId },
    ResourceAccess { resource: EntityId, agent_holds_permission: bool },
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RecordTopic {
    PriceObserved { commodity: CommodityKind },
    RouteSafety,
    OfficeRule { office: EntityId },
    BountyExists,
    TestifiedAbout { subject: EntityId },
}
```

`OfficeRule` variant is carried by `RecordTopic` (not by `CausalProvider`) per S137 FND-14A clarification: office authority enters via concrete record entities, never as a discriminant.

### 2. Extended `crates/worldwake-core/src/repair_memory.rs` with `BreachSignature`

```rust
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BreachSignature {
    pub goal_key: GoalKey,
    pub invalidator: InvalidatorTag,
    pub step_target: Option<EntityId>,
}
```

Lives adjacent to `RepairMemory`. No consumers landed in this ticket — ticket 005 migrates `RepairMemory.repairs` to key by `BreachSignature`.

### 3. Re-exports in `crates/worldwake-core/src/lib.rs`

Added `pub mod causal_link`, `pub use causal_link::{CausalLink, CausalProvider, PlanningFact, RecordTopic};`, and `pub use repair_memory::BreachSignature;`.

## Files to Touch

- `crates/worldwake-core/src/causal_link.rs` (new)
- `crates/worldwake-core/src/repair_memory.rs` (modify — add `BreachSignature` adjacent to existing `RepairMemory`)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-core/src/belief_claim_key.rs`, `crates/worldwake-core/src/goal.rs`, `crates/worldwake-core/src/social_artifact.rs` (modify — derive `Hash` where needed for the new shared key/type bounds)

## Out of Scope

- `PlanGuard.causal_links` field — ticket 004.
- `RepairMemory.repairs` shape migration to `BTreeMap<BreachSignature, _>` — ticket 005.
- `plan_repair` module consuming `CausalLink` for repair search — ticket 006.
- `RepairAppliedPayload` extension — ticket 003.
- `DiscrepancyClearing` variant extension — subsumed into ticket 006.

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-core causal_link` passed and covered bincode roundtrip per causal-link type plus required derive bounds.
2. `cargo test -p worldwake-core repair_memory` passed and covered `BreachSignature` bincode roundtrip plus `Copy`, `Eq`, `Ord`, `Hash`, `Serialize`, `Deserialize` bounds.
3. Existing suite `cargo test --workspace` passed.

### Invariants

1. All five types are `Copy`. (`CausalLink` payload structure preserves `Copy` because all referenced types are `Copy`.)
2. All five types derive `Ord` and `Hash` for BTreeMap/BTreeSet use (AGENTS.md determinism invariant).
3. No type references a higher-crate enum (worldwake-sim/systems/ai/cli) — verified at compile time.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/causal_link.rs` `#[cfg(test)]` module — new — `causal_link_types_satisfy_required_bounds`, `causal_link_roundtrips_through_bincode`, `causal_provider_variants_roundtrip_through_bincode`, `planning_fact_variants_roundtrip_through_bincode`, `record_topic_variants_roundtrip_through_bincode`.
2. `crates/worldwake-core/src/repair_memory.rs` `#[cfg(test)]` — extended `repair_memory_types_satisfy_required_bounds` to include `BreachSignature` and added `breach_signature_roundtrips_through_bincode`.

### Commands Run

1. `cargo test -p worldwake-core causal_link`
2. `cargo test -p worldwake-core repair_memory`
3. `cargo test -p worldwake-core`
4. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

Completed on 2026-05-13.

- Added the S137 core substrate types in `worldwake-core`: `CausalLink`, `CausalProvider`, `PlanningFact`, `RecordTopic`, and `BreachSignature`.
- Re-exported the new shared types from `worldwake-core`.
- Added focused bincode roundtrip and trait-bound tests for the new substrate.
- Added `Hash` derives to existing key/payload types needed for the new shared `Hash` bounds. This did not change persisted shape and did not require a `SAVE_FORMAT_VERSION` bump.

## Deviations

- The draft claimed no `PlanningFact` existed anywhere under `crates/`. Live reassessment found a private `worldwake-ai::search::landmarks::PlanningFact`; this ticket still landed the distinct shared `worldwake_core::PlanningFact` required by S137 D3.
- The drafted integration-test filename `--test causal_link_types_roundtrip` was narrowed to module-local core unit tests because the owned seam is the new core module and its public re-export surface, not an integration-test binary.
- `scripts/verify.sh` was not run because the ticket explicitly defers that wrapper until ticket 010; the per-ticket required workspace proof was covered by `cargo test --workspace` plus core clippy.

## Verification Result

- Passed `cargo test -p worldwake-core causal_link`
- Passed `cargo test -p worldwake-core repair_memory`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
- Passed `cargo test --workspace`
