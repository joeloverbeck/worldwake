# S147HTNMETDEC-006: First-ship methods and MethodRegistry with validation tests

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds explicit HTN binding-template types, 13 method definitions, and the `MethodRegistry` to the htn module. No planner integration yet (ticket 008).
**Deps**: `archive/tickets/S147HTNMETDEC-004.md` (MethodSchema + supporting types), `archive/tickets/S147HTNMETDEC-005.md` (GoalSchema.methods)

## Problem

S147 D2 ships 13 methods covering the first-ship scope: `FulfillBounty` × 3 (Direct, Investigation, GroupHunt), `ProduceCommodity` × 3 (FromOwnedStock, WithGather, WithPurchase), `RestockCommodity` × 2 (FromHarvest, FromMarket), `InvestigateViolation` × 3 (OnScene, ByWitness, ByLedger), `EscortToSafety` × 2 (ToHome, ToOffice). D8 ships the `MethodRegistry` (keyed by `GoalKindDiscriminant`) and `build_method_registry()` constructor plus 6 validation tests. Both deliverables are bundled because the registry's content *is* the methods — splitting them would require either an empty-registry intermediate state or backwards references between the two files.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `MethodSchema` and supporting types exist after `archive/tickets/S147HTNMETDEC-004.md` landed at `crates/worldwake-ai/src/htn/method_schema.rs`. `GoalSchema.methods` field exists after `archive/tickets/S147HTNMETDEC-005.md` landed at `crates/worldwake-ai/src/goal_schema.rs`. `GoalKindDiscriminant::ALL` constant exists after `archive/tickets/S147HTNMETDEC-001.md` lands at `crates/worldwake-core/src/goal.rs`.
2. `PlannerOpKind` lives at `crates/worldwake-ai/src/planner_ops.rs:13` with 32+ variants. The methods reference real variants (verified during S147 reassessment): `Attack` (not `Subdue/Kill`), `Trade` (not `BuyCommodity`), `Craft` (not `Bake` or `ProduceCommodity`-action), `DeclareSupport` (substitute for `RecruitAlly` semantics), `ClaimBounty`, `EscortToSafety`, `Investigate`. None of the spec's fictional action names (Subdue, Kill, RecruitAlly, BuyCommodity, HandOffToOffice, Bake) exist — verified during reassessment.
3. `RecruitAlly` does not exist as a `PlannerOpKind` variant. `FulfillBountyGroupHunt`'s subgoal must use a different mechanism — either `PerformAction(DeclareSupport, …)` to signal recruitment (most semantically aligned with existing variants), or defer the `GroupHunt` method's recruitment subgoal as a future-scope variant addition. Pick the `DeclareSupport` substitution to keep first-ship scope intact; document the semantic stretch in the method definition's comment.
4. Shared boundary: the `MethodRegistry` is the data contract between the method definitions (this ticket) and the `MethodSelector` (ticket 007). The registry's lookup API (`methods_for(goal_kind) -> &[MethodSchemaId]` plus `get(id) -> Option<&MethodSchema>`) is the selector-facing surface.
5. The 6 validation tests (D8) catch divergence at compile/test time: (a) every method's `goal_kind` resolves to a real `GoalKindDiscriminant`, (b) every `SubgoalTemplate::PerformAction(op, _)` references a real `PlannerOpKind`, (c) `MethodSchemaId` uniqueness, (d) every method declares at least one `failure_modes` entry, (e) every `MotiveBias.weight` is in `Permille` bounds, and (f) every method declares at least one subgoal. These tests are the regression guard against future method-author drift.
6. Live implementation exposed one required schema correction: build-time methods cannot carry runtime-specific `EntityId`, `CommodityKind`, or recipe values as concrete fields without introducing hidden sentinel bindings. This ticket absorbs the correction by adding explicit `EntityTemplate`, `CommodityTemplate`, and `RecipeTemplate` surfaces in `crates/worldwake-ai/src/htn/method_schema.rs`, then defining method entries against those symbolic bindings. This keeps the selector/planner boundary explicit for tickets 007 and 008 and avoids a FND-20/FND-28 violation.

## Architecture Check

1. Bundling D2 and D8 in one ticket is correct because the registry's `BTreeMap` content is built from the method definitions. Splitting would either leave an intermediate empty-registry state (which has no test value) or require backwards file references between `htn/methods.rs` and `htn/registry.rs`. The current bundling keeps the registry constructor and the methods it constructs in the same review surface.
2. Each method definition uses real `PlannerOpKind` variants only — no new variants introduced. The `FulfillBountyGroupHunt` recruitment subgoal stretches `DeclareSupport` semantics to cover ally-recruitment signaling; this is documented in the method's comment. If first-ship play surfaces a need for a true `RecruitAlly` variant, that's a future spec addition.
3. Explicit binding-template enums are cleaner than sentinel `EntityId(0)`-style placeholders because every runtime-dependent value is named as a symbolic binding that later selector/planner code must resolve against the goal and belief view.
4. No backwards-compatibility shims. All code is net-new in the `htn/` module except the required correction to the already-staged `method_schema.rs` type surface.

## Verification Layers

1. Every method's `goal_kind` is a real discriminant → validation test (a) iterates registry entries and asserts membership in `GoalKindDiscriminant::ALL`.
2. Every `SubgoalTemplate::PerformAction(op, _)` references a real `PlannerOpKind` → validation test (b) iterates method subgoals and matches `op` against a sentinel iteration of all `PlannerOpKind` variants.
3. `MethodSchemaId` uniqueness → validation test (c) collects IDs into a `BTreeSet` and asserts cardinality matches insertion count.
4. Runtime binding values are explicit templates, not hidden sentinels → `method_schema.rs` compile surface plus package tests prove all current constructors use typed templates.
5. Single-layer ticket (data definition) — runtime invariants verified by ticket 008 (planner integration) and ticket 011 (goldens).

## Landed Changes

### 1. Added explicit method binding templates

Modified `crates/worldwake-ai/src/htn/method_schema.rs` so method predicates, subgoals, payloads, artifact expectations, and claim requirements use:

- `EntityTemplate`
- `CommodityTemplate`
- `RecipeTemplate`

This replaced concrete runtime IDs/values in method-schema fields with explicit symbolic bindings such as `GoalPrimaryEntity`, `BountyTarget`, `GoalCommodity`, `RecipeInput { recipe: GoalRecipe, ordinal: 0 }`, and `GoalRecipe`.

### 2. Defined 13 first-ship method constructors

New file `crates/worldwake-ai/src/htn/methods.rs`:

```rust
pub fn fulfill_bounty_direct() -> MethodSchema { /* per spec D2 */ }
pub fn fulfill_bounty_investigation() -> MethodSchema { /* per spec D2 */ }
pub fn fulfill_bounty_group_hunt() -> MethodSchema { /* per spec D2; uses DeclareSupport for recruitment */ }
pub fn produce_from_owned_stock() -> MethodSchema { /* per spec D2 */ }
pub fn produce_with_gather() -> MethodSchema { /* per spec D2 */ }
pub fn produce_with_purchase() -> MethodSchema { /* per spec D2 */ }
pub fn restock_from_harvest() -> MethodSchema { /* per spec D2 */ }
pub fn restock_from_market() -> MethodSchema { /* per spec D2 */ }
pub fn investigate_on_scene() -> MethodSchema { /* per spec D2 */ }
pub fn investigate_by_witness() -> MethodSchema { /* per spec D2 */ }
pub fn investigate_by_ledger() -> MethodSchema { /* per spec D2 */ }
pub fn escort_to_home() -> MethodSchema { /* per spec D2 */ }
pub fn escort_to_office() -> MethodSchema { /* per spec D2 */ }
```

Each method assigns a stable `MethodSchemaId` (1..=13) and uses `PlannerOpKind` variants verified at `crates/worldwake-ai/src/planner_ops.rs:13`:
- `FulfillBountyDirect`: `Attack`, `ClaimBounty`
- `FulfillBountyGroupHunt`: `DeclareSupport` (recruitment substitute), `Attack`
- `Produce*`: `Trade`, `Craft`
- `Restock*`: `Trade` for market variants; harvest doesn't need `PerformAction` (AcquireCommodity subgoal handles it)
- `Investigate*`: `Investigate`
- `Escort*`: `EscortToSafety`

### 3. Defined `MethodRegistry` and `build_method_registry()`

New file `crates/worldwake-ai/src/htn/registry.rs`:

```rust
#[derive(Default)]
pub struct MethodRegistry {
    methods: BTreeMap<MethodSchemaId, MethodSchema>,
    by_goal_kind: BTreeMap<GoalKindDiscriminant, Vec<MethodSchemaId>>,
}

impl MethodRegistry {
    pub fn insert(&mut self, schema: MethodSchema) { /* … */ }
    pub fn get(&self, id: MethodSchemaId) -> Option<&MethodSchema> { /* … */ }
    pub fn methods_for(&self, goal_kind: GoalKindDiscriminant) -> &[MethodSchemaId] { /* … */ }
    pub fn all_method_ids(&self) -> impl Iterator<Item = MethodSchemaId> + '_ { /* … */ }
}

pub fn build_method_registry() -> MethodRegistry {
    let mut registry = MethodRegistry::default();
    registry.insert(methods::fulfill_bounty_direct());
    registry.insert(methods::fulfill_bounty_investigation());
    registry.insert(methods::fulfill_bounty_group_hunt());
    registry.insert(methods::produce_from_owned_stock());
    registry.insert(methods::produce_with_gather());
    registry.insert(methods::produce_with_purchase());
    registry.insert(methods::restock_from_harvest());
    registry.insert(methods::restock_from_market());
    registry.insert(methods::investigate_on_scene());
    registry.insert(methods::investigate_by_witness());
    registry.insert(methods::investigate_by_ledger());
    registry.insert(methods::escort_to_home());
    registry.insert(methods::escort_to_office());
    registry
}
```

`insert()` populates both maps: stores the schema by id AND appends the id to the per-goal-kind list.

### 4. Updated `htn/mod.rs` to re-export

```rust
pub mod method_schema;
pub mod methods;
pub mod registry;

pub use method_schema::*;
pub use registry::{MethodRegistry, build_method_registry};
```

### 5. Added validation tests

New file `crates/worldwake-ai/tests/htn_registry_validation.rs`:
- `every_method_goal_kind_resolves` — iterates the registry and asserts every `method.goal_kind` is in `GoalKindDiscriminant::ALL`.
- `every_subgoal_action_op_resolves` — iterates subgoals and matches `PerformAction(op, _)` against the full `PlannerOpKind` variant set.
- `method_schema_ids_are_unique` — `BTreeSet` collection of IDs equals insertion count.
- `every_method_declares_at_least_one_failure_mode` — `assert!(!method.failure_modes.is_empty())` per entry.
- `motive_bias_weights_are_in_permille_bounds` — every `MotiveBias.weight.value() <= 1000`.
- `every_method_has_at_least_one_action_or_non_action_subgoal` — each method declares at least one decomposed subgoal.

## Landed Files

- `crates/worldwake-ai/src/htn/methods.rs` (new)
- `crates/worldwake-ai/src/htn/registry.rs` (new)
- `crates/worldwake-ai/src/htn/method_schema.rs` (modify — add explicit binding templates and retarget dynamic fields to them)
- `crates/worldwake-ai/src/htn/mod.rs` (modify — add `pub mod methods; pub mod registry;` and re-exports)
- `crates/worldwake-ai/tests/htn_registry_validation.rs` (new)
- `archive/specs/S147-htn-method-decomposition.md` (modified — truth-sync D1/D2 binding-template contract)

## Out of Scope

- `MethodSelector` consumption of the registry (ticket 007).
- Planner integration of method selection in `build_stages` (ticket 008).
- `RecruitAlly` as a new `PlannerOpKind` variant — `FulfillBountyGroupHunt` uses `DeclareSupport` as a recruitment-signal substitute. A true `RecruitAlly` variant is future scope.
- Pre-populating `GoalSchema.methods` static slices with method IDs — left empty by ticket 005; the runtime registry is the authoritative lookup.

## Acceptance Result

### Tests Passed

1. Passed all 6 validation tests in `htn_registry_validation.rs`.
2. Passed `htn::registry::tests::registry_builds_with_13_methods` — `build_method_registry()` returns exactly 13 methods.
3. Passed `htn::registry::tests::methods_for_returns_correct_ids` — `methods_for(GoalKindDiscriminant::FulfillBounty)` returns exactly 3 method IDs.
4. Passed existing package suite: `cargo test -p worldwake-ai`.
5. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.

### Invariants

1. Every method in the registry has a unique `MethodSchemaId`.
2. Every `SubgoalTemplate::PerformAction(op, _)` in any method references a real `PlannerOpKind` variant (compile-time + validation test).
3. `methods_for(goal_kind)` returns IDs in insertion order, which is the deterministic tie-break order documented in ticket 007's `MethodSelector` ranking step 4.
4. No method declares `methods.is_empty() failure_modes` — every method has at least one failure mode.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/tests/htn_registry_validation.rs` — new — 6 invariants per spec D8 plus non-empty subgoal coverage.
2. `crates/worldwake-ai/src/htn/registry.rs` inline tests — registry shape and lookup behavior.

### Commands Run

1. `cargo test -p worldwake-ai --test htn_registry_validation`
2. `cargo test -p worldwake-ai --lib htn::registry`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-17.

- Added explicit `EntityTemplate`, `CommodityTemplate`, and `RecipeTemplate` binding surfaces so build-time methods do not hide runtime goal bindings behind concrete sentinel IDs.
- Added 13 first-ship method constructors covering `FulfillBounty`, `ProduceCommodity`, `RestockCommodity`, `InvestigateViolation`, and `EscortToSafety`.
- Added `MethodRegistry`, `build_method_registry()`, registry re-exports, and focused registry validation tests.
- Truth-synced S147 D1/D2 wording to the landed binding-template contract.

## Deviations

- The original D1/D2 sketch used concrete `EntityId`, `CommodityKind`, and `recipe_id` fields inside build-time method schemas. The landed version uses explicit template bindings because static methods must be resolved against runtime goals and belief views by later selector/planner tickets.
- Validation coverage includes 6 tests rather than the drafted 5; the added test asserts every method has at least one subgoal.

## Verification Result

- Passed `cargo test -p worldwake-ai --test htn_registry_validation`.
- Passed `cargo test -p worldwake-ai --lib htn::registry`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
