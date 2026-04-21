# S114PLASTGUA-006: ActionDef template specs + build_plan_guard / build_plan_expectations

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — two new fields on `ActionDef` (widespread construction-site impact); new `plan_guard_build.rs` module; `trade` action's guard_template populated; `SAVE_FORMAT_VERSION` bump.
**Deps**: `archive/tickets/S114PLASTGUA-001.md`, `archive/tickets/S114PLASTGUA-003.md`

## Problem

S114 D3 establishes declarative, serializable guard + expectation authoring on `ActionDef`. Closures are explicitly rejected — `ActionDef` derives `Serialize + Deserialize` and must continue to round-trip through save/load (FND-28 + existing `ActionDef` round-trip test at `action_def.rs:114`). The pure-function `build_plan_guard` / `build_plan_expectations` in the AI crate translate the specs into runtime `PlanGuard` / `Vec<PlanExpectation>` at plan-build time. Populating the `trade` action's guard_template is required here so tickets 007 / 009 / 010 can exercise the guard-check path on a real action.

## Assumption Reassessment (2026-04-21)

1. `ActionDef` at `crates/worldwake-sim/src/action_def.rs:27` has 17 existing fields and derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. No `Default` impl. Adding two new fields forces every literal-enumerating construction site to explicitly initialize them.
2. `rg -c 'ActionDef \{' crates` returns 170 occurrences across 41 files — the construction-site count includes the struct definition line and some type-reference lines, but the majority are literal enumerations inside test helpers and action-registration functions (`register_*_action`). All literal-enumerating sites must be updated. This is the dominant effort driver — the change itself is mechanical (add two lines per literal).
3. S114 spec D3 at `specs/S114-plan-step-guards.md:172-233` defines the template types (`GuardTemplateSpec`, `ExpectationTemplateSpec`, `RequiredFactSpec`, `InvalidatorSpec`, `StatePredicateSpec`, `ObservationPredicateSpec`) and the two pure build functions.
4. The `trade` action is registered at `crates/worldwake-systems/src/trade_actions.rs:23` (`register_trade_action`) with the literal `ActionDef` at `trade_actions.rs:38` (name `"trade"`, `ActionDomain::Trade`). This is the action ticket 010's golden test will exercise, so its `guard_template` must carry `TargetPresent` required fact + `TargetMoved` invalidator.
5. Shared boundary under audit: `ActionDef`'s `Serialize + Deserialize` surface + the ~170 literal-enumeration sites. `SAVE_FORMAT_VERSION = 36` at `crates/worldwake-sim/src/save_load.rs:6` (current baseline) — this ticket bumps it by 1.
6. Existing `ActionDef` tests at `action_def.rs:48-280`:
   - `action_def_satisfies_required_traits` (line 114) — no change needed; trait bounds identical.
   - `action_def_requires_all_expected_fields_with_concrete_non_optional_semantics` (line 119) — assertion-by-field; both new fields must be added to the assertion list.
   - `sample_action_def` helper (line 65) — must populate both new fields.

## Architecture Check

1. Declarative specs (not closures) preserve `ActionDef`'s `Serialize + Deserialize` round-trip. `BindingSourceTag`-style enums inside `StatePredicateSpec` / `ObservationPredicateSpec` (per spec D3 line 211-213) let the pure `build_plan_guard` / `build_plan_expectations` resolve concrete `EntityId` / `CommodityKind` / `Quantity` values against the live `PlannedStep` without storing any state on `ActionDef`.
2. The AI-crate owns the build functions, not sim — `ActionDef` stays agnostic of `PlannedStep`. `build_plan_guard` takes `&ActionDef`, `&PlannedStep`, `Tick` and returns `Option<PlanGuard>`. This matches the existing pattern where sim owns `ActionDef` as serializable data and AI owns plan-building.

## Verification Layers

1. Serialization contract (`ActionDef` with / without `guard_template = Some(_)` round-trips through bincode) → new focused unit tests in `action_def.rs` tests module.
2. Pure build-function contract (`build_plan_guard(ActionDef, PlannedStep, Tick)` translates `GuardTemplateSpec::TargetPresent` into concrete `RequiredFact::TargetPresent` with `step.primary_target()` + `step.target_place()`; returns `None` when `guard_template = None`) → focused unit tests in `plan_guard_build.rs`.
3. Workspace compile after construction-site updates → `cargo build --workspace` succeeds.
4. Existing `ActionDef` semantics tests (all 17 fields still required non-optionally, plus the two new optional-semantic fields) → updated `action_def_requires_all_expected_fields_with_concrete_non_optional_semantics` test.
5. `trade` action registration continues to exercise every pre-S114 scenario (no regression in `cargo test -p worldwake-systems trade_actions`) — guard_template population adds a new optional field, does not replace any existing field.
6. Single-layer (sim-crate struct layout + AI-crate pure fns + widespread construction-site updates); downstream consumers arrive in tickets 008 / 009 / 010.

## What to Change

### 1. Add template-spec types to `action_def.rs`

In `crates/worldwake-sim/src/action_def.rs`, introduce:

- `GuardTemplateSpec` with `required_facts: Vec<RequiredFactSpec>`, `min_confidence: Permille`, `invalidators: Vec<InvalidatorSpec>`.
- `RequiredFactSpec` with variants `TargetPresent`, `CommodityAvailable { min_quantity: Quantity }`, `RouteKnown`, `ResourceAccess`.
- `InvalidatorSpec` with variants `TargetMoved`, `BeliefStatusChange`, `CommodityDepleted { min_quantity: Quantity }`, `NewBlockerRecorded`.
- `ExpectationTemplateSpec` with `kind_tag: ExpectationKindTag`, `observe_by_offset: Option<u32>`, `event_tag: Option<EventTag>`, `state_predicate_spec: Option<StatePredicateSpec>`, `observation_predicate_spec: Option<ObservationPredicateSpec>`.
- `StatePredicateSpec` / `ObservationPredicateSpec` — binding-source-tagged mirrors of their core counterparts, variants per spec line 211-213. Source-tag enums: `PlaceSource { StepTargetPlace, ActorPlace }`, `KindSource { PayloadCommodity, LiteralCommodity(CommodityKind) }`, `QuantitySource { Literal(Quantity), PayloadCommodity }`.

All types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. Extend `ActionDef`

Append to the struct at `action_def.rs:27`:

```rust
#[serde(default)]
pub guard_template: Option<GuardTemplateSpec>,

#[serde(default)]
pub expectation_template: Vec<ExpectationTemplateSpec>,
```

(`#[serde(default)]` is purely defensive — per FND-28, save files pre-S114 are rejected via the `SAVE_FORMAT_VERSION` bump, but the annotation avoids regressions on any future in-memory RON use case.)

### 3. Update every `ActionDef { ... }` literal construction site

Every literal-enumerating site adds:

```rust
guard_template: None,
expectation_template: vec![],
```

This touches ~170 occurrences across 41 files spanning `worldwake-sim`, `worldwake-systems`, and test modules in `worldwake-ai`. The change is mechanical but review-heavy. At implementation time, re-run `rg -n 'ActionDef \{' crates` to generate the exact touch list before editing.

### 4. Populate the `trade` action's guard_template

In `crates/worldwake-systems/src/trade_actions.rs:38`, replace the new default `guard_template: None` with:

```rust
guard_template: Some(GuardTemplateSpec {
    required_facts: vec![RequiredFactSpec::TargetPresent],
    min_confidence: Permille::new(500).unwrap(),
    invalidators: vec![
        InvalidatorSpec::TargetMoved,
        InvalidatorSpec::BeliefStatusChange,
    ],
}),
```

This is the minimum needed for ticket 010's golden test; other action registrations stay at `None` for this ticket's scope.

### 5. New file `crates/worldwake-ai/src/plan_guard_build.rs`

```rust
pub fn build_plan_guard(
    def: &ActionDef,
    step: &PlannedStep,
    adoption_tick: Tick,
) -> Option<PlanGuard> {
    let spec = def.guard_template.as_ref()?;
    // Resolve each RequiredFactSpec against step accessors and payload.
    // Skip facts whose binding source resolves to None (e.g. TargetPresent
    // on an untargeted action emits no fact; invalidators follow suit).
    // min_confidence is copied from spec; ceiling enforcement happens at
    // evaluation time (ticket 007), not build time.
}

pub fn build_plan_expectations(
    def: &ActionDef,
    step: &PlannedStep,
    adoption_tick: Tick,
) -> Vec<PlanExpectation> {
    def.expectation_template
        .iter()
        .map(|tmpl| build_one_expectation(tmpl, step, adoption_tick))
        .collect()
}
```

Export the module from `crates/worldwake-ai/src/lib.rs`.

### 6. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, increment by 1 relative to the current landed value.

## Files to Touch

- `crates/worldwake-sim/src/action_def.rs` (modify — types + struct + existing tests)
- ~40 files across `crates/worldwake-sim`, `crates/worldwake-systems`, `crates/worldwake-ai` (tests), `crates/worldwake-cli` (tests) that construct `ActionDef {` literals (generate exact list via `rg -l 'ActionDef \{' crates` at implementation time)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — populate guard_template at line 38)
- `crates/worldwake-ai/src/plan_guard_build.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — module declaration)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` bump)

## Out of Scope

- Evaluating guards at revalidation time (ticket 007).
- Writing `ExpectationRecord`s from `build_plan_expectations` output (ticket 008).
- Mismatch emission + discrepancy classification (ticket 009).
- Populating guard_templates on actions beyond `trade` — later phases of S114 or Phase 10 scenarios will widen coverage.

## Acceptance Criteria

### Tests That Must Pass

1. New test: `action_def_with_guard_template_round_trips_through_bincode` — constructs an `ActionDef` with `guard_template: Some(...)` and `expectation_template: vec![...]`, asserts bincode round-trip.
2. New test: `action_def_without_guard_template_round_trips_through_bincode` — asserts `guard_template: None` + empty `expectation_template: vec![]` round-trip.
3. New test (in `plan_guard_build.rs`): `build_plan_guard_translates_target_present_binding` — given an `ActionDef` with `RequiredFactSpec::TargetPresent` and a `PlannedStep` whose `primary_target()` and `target_place()` both return `Some(_)`, `build_plan_guard` returns `Some(PlanGuard { required_facts: vec![RequiredFact::TargetPresent { target, at_place }], ... })`.
4. New test (in `plan_guard_build.rs`): `build_plan_guard_returns_none_when_template_absent`.
5. Existing `action_def_requires_all_expected_fields_with_concrete_non_optional_semantics` (line 119) updated to include the two new fields' semantics.
6. Existing suite: `cargo test --workspace` stays green.

### Invariants

1. `ActionDef` continues to derive `Serialize + Deserialize`; no closure field introduced.
2. `build_plan_guard` / `build_plan_expectations` are pure functions with no global state or `&mut self` access.
3. Every `ActionDef { ... }` literal construction site in the workspace explicitly initializes both new fields (no `..Default::default()` spread; no `Default` impl).
4. `RouteKnown` required-fact binding resolves to `RequiredFact::RouteKnown { from, to }` and is evaluated through the existing `RuntimeBeliefView::route_exists(from, to)` seam. No follow-up route-known spec is required for the boolean reachability case.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_def.rs` tests module — `action_def_with_guard_template_round_trips_through_bincode`, `action_def_without_guard_template_round_trips_through_bincode`, updated `action_def_requires_all_expected_fields_with_concrete_non_optional_semantics`.
2. `crates/worldwake-ai/src/plan_guard_build.rs` tests module (new) — `build_plan_guard_translates_target_present_binding`, `build_plan_guard_returns_none_when_template_absent`, `build_plan_expectations_maps_every_spec`.
3. `crates/worldwake-systems/src/trade_actions.rs` — existing `trade_action_*` tests (if any) must stay green; add one new test asserting the `trade` action's guard_template is `Some(_)` post-registration.

### Commands

1. `cargo test -p worldwake-sim action_def`
2. `cargo test -p worldwake-ai plan_guard_build`
3. `cargo test -p worldwake-systems trade_actions`
4. `cargo build --workspace` (ensures all 170 construction sites are covered)
5. `scripts/verify.sh`
