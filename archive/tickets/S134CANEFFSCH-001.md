# S134CANEFFSCH-001: Effect schema foundation and ActionDef field

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — adds `worldwake-sim::effect_schema` module, extends `ActionDef` with required `effect_schema: EffectSchema` field
**Deps**: spec `archive/specs/S134-canonical-effect-schema.md`

## Problem

S134 unifies three parallel forward models per action (imperative authoritative handlers, explicit `apply_hypothetical_transition` arms, and per-`GoalKind` `apply_planner_step` fallback) into a single declarative `EffectSchema` attached to each `ActionDef`. This ticket establishes the foundation: the new `effect_schema` module owning the type surface (`EffectSchema`, `EffectMode`, `EffectStep`, `EffectPrecondition`, `EffectFact`, `EffectOutcome`, `EffectSink` trait, `apply_effects` function), and the `ActionDef.effect_schema` field populated with `EffectSchema::empty()` at every construction site. Subsequent tickets populate real schemas per action category (003–009) and switch the planner to consume them (010); this ticket leaves the field present but inert so the workspace builds and existing imperative handlers continue to run unchanged.

## Assumption Reassessment (2026-05-04)

1. Before this ticket, `ActionDef` lived at `crates/worldwake-sim/src/action_def.rs:121–144` with 19 existing fields including `binding_strictness: BindingStrictness`, `guard_template: Option<GuardTemplateSpec>`, and `expectation_template: Vec<ExpectationTemplateSpec>`. All three remain unchanged after this ticket — `effect_schema` is layered alongside them, not absorbing them, per spec Q1=(b) resolution at reassessment.
2. Live constructor fallout was wider than the draft count. A repo-wide sweep plus `cargo test --workspace --no-run` found 127 `effect_schema: EffectSchema::empty()` insertion sites across `worldwake-sim`, `worldwake-systems`, and AI-side test/planner fixtures. The first compile pass exposed 54 `worldwake-ai` `ActionDef` literals not named in the original file list. Zero use spread syntax (`..Default::default()`) was found; every updated site explicitly enumerates fields. `EffectSchema::empty()` constructs the no-op schema (`preconditions: vec![], steps: vec![]`).
3. Shared abstraction boundary under audit: the `ActionDef` registry shape (carried by `ActionDefRegistry` in `crates/worldwake-sim/src/action_def_registry.rs`) is consumed by every action handler in `worldwake-systems/` and by planner/test fixtures in `worldwake-ai`. After this ticket the field exists on every `ActionDef` but is never read at runtime; it becomes load-bearing only when ticket 010 routes the planner through `apply_effects`.
4. Existing focused/unit coverage: `crates/worldwake-sim/src/action_def_registry.rs` carries inline `#[cfg(test)]` tests around line 61 (`sample_action_def`), `interrupt_abort.rs:320`, `action_handler.rs:499`, `action_handler_registry.rs:94`/`316`/`320`. Each of these factory functions is exercised by adjacent tests; adding the empty-schema field to each preserves their behavior.
5. Bitwise-identical event-log invariant: existing goldens in `crates/worldwake-ai/tests/golden_*.rs` must produce identical canonical state hashes (`blake3` over post-replay ECS) before and after this ticket because no runtime code reads `effect_schema` yet. The ignored 1440-tick `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` suites were run explicitly for this closeout.
6. Save/load boundary: `ActionDef.effect_schema` is registry data, not part of `SimulationState` or the saved runtime payload. `SAVE_FORMAT_VERSION` remains `66`.

## Architecture Check

1. The required-field-with-empty-default path keeps the workspace compiling at every step of the migration without introducing an `Option<EffectSchema>` shim that would later need to be removed (FND-28 — no backward-compatibility shims in live authority paths). The empty schema is a placeholder that downstream tickets replace; it is not a fossilized fallback because subsequent tickets unconditionally populate every action.
2. Per spec Q1 resolution, `effect_schema` is layered alongside `guard_template`/`expectation_template` rather than absorbing them — the existing S114 plan-step guard/expectation surface is preserved unchanged. The two surfaces serve different time horizons (cross-step plan validity vs. action-interior forward model) and remain distinct.
3. The `EffectSink` trait is defined in `worldwake-sim` so that `worldwake-sim` never names `worldwake-ai` types (`PlanningState` lives in ai). The trait is the only cross-crate seam — implementations land in subsequent tickets and preserve the workspace layering `core → sim → systems → ai → cli` (FND-26).

## Verification Layers

1. Workspace compilation invariant → focused unit/runtime test (existing tests in `crates/worldwake-sim/src/action_def*.rs` and `action_handler*.rs` pass unchanged).
2. Bitwise-identical event-log invariant → soak: 1440-tick replay of `scenarios/survival-baseline.ron`, `scenarios/survival-scattered.ron`, `scenarios/survival-contested.ron` produce identical canonical state hashes pre- and post-ticket.
3. No runtime read of `effect_schema` invariant → grep verification: `apply_effects` is defined but called from zero runtime sites until ticket 002 lands sinks and ticket 010 switches the planner. Single-layer ticket (introduction-only); the higher-layer mapping (decision trace, action trace) is exercised by downstream tickets.

## What to Change

### 1. New `effect_schema` module in `worldwake-sim`

Create `crates/worldwake-sim/src/effect_schema.rs` with:

- `pub struct EffectSchema { pub preconditions: Vec<EffectPrecondition>, pub steps: Vec<EffectStep> }` plus `EffectSchema::empty()` constructor returning the no-op schema.
- `pub enum EffectMode { Authoritative, Hypothetical }`
- `pub enum EffectPrecondition { … }` — variants enumerated per spec D1 sketch using existing core types (`EntityId` for actor/target/grant, `CommodityKind` for commodities, `BeliefClaimKey` for belief claims, `Quantity` for quantities, `TargetSpec` for slot shape). Include `TargetMatchesSlot { slot_index: usize, shape: TargetSpec }`, `CoLocated { actor: EntityId, target: EntityId }`, `QuantityAvailable { source: EntityId, commodity: CommodityKind, min: Quantity }`, `CapacityFloor { container: EntityId, min_free: Quantity }`, `ContentionGrantHeld { actor: EntityId, affordance: EntityId }`, `BeliefHeld { agent: EntityId, claim: BeliefClaimKey }`. Add additional variants as per-category tickets surface needs.
- `pub enum EffectStep { … }` — variants enumerated per spec D1 sketch using existing core types. Include `Transfer { source: EntityId, dest: EntityId, commodity: CommodityKind, quantity: Quantity }`, `Consume { … }`, `Produce { … }`, `ApplyWound { target: EntityId, cause: WoundCause }`, `EmitEvent { tag: EventTag }`, `AssertExpectationFulfilled { expectation: ExpectationId }`, `ConsumeContentionGrant { grant: EntityId }`, `PartialOnFailure { primary: Vec<EffectStep>, fallback: Vec<EffectStep> }`. Per-category tickets add variants if their actions need effects not yet covered.
- `pub enum EffectFact { … }` — typed outputs threaded back to the caller. Include `CommodityTransfer { … }`, `PartialQuantity { requested: Quantity, delivered: Quantity }`, `WoundApplied { … }`, `ExpectationFulfilled { … }`, `ContentionGrantConsumed { … }`, `EventEmitted { tag: EventTag }`. Note: this is *not* a reuse of `PlanningFact` (which is `pub(super)` in `worldwake-ai/src/search/landmarks.rs:12` and unreachable from sim).
- `pub struct EffectOutcome { pub facts: Vec<EffectFact> }`.
- `pub trait EffectSink { /* one method per EffectStep variant: write_transfer, write_consume, write_produce, write_wound, write_event, assert_expectation_fulfilled, consume_grant */ }` — trait stub with method signatures only; implementations land in ticket 002.
- `pub fn apply_effects(schema: &EffectSchema, actor: EntityId, targets: &[EntityId], sink: &mut dyn EffectSink, mode: EffectMode) -> Result<EffectOutcome, Discrepancy>` — function body stub returning `Ok(EffectOutcome { facts: vec![] })`. Per-step interpretation logic and non-empty-schema failure behavior land as sink methods are implemented in ticket 002 and category tickets exercise non-empty schemas.

Re-export the public types from `crates/worldwake-sim/src/lib.rs`.

### 2. `ActionDef` field addition

In `crates/worldwake-sim/src/action_def.rs`, add `pub effect_schema: EffectSchema` (line ~143–145, immediately after `expectation_template`). The field is required and non-optional per spec Design Goal 1.

### 3. Populate `effect_schema: EffectSchema::empty()` at all live construction sites

Each live `ActionDef { … }` literal receives the literal `effect_schema: EffectSchema::empty(),` added as the final field of the literal (matching the field order in `action_def.rs`). Drafted count was 44; live closeout count is 127 empty-schema construction sites across `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` test/planner fixtures.

- `crates/worldwake-sim/src/action_def_registry.rs:62` (sample_action_def)
- `crates/worldwake-sim/src/interrupt_abort.rs:321`
- `crates/worldwake-sim/src/action_handler.rs:500`
- `crates/worldwake-sim/src/action_handler_registry.rs:95`, `:316`, `:320`
- All `ActionDef {` openings in `crates/worldwake-systems/src/*.rs` (38 sites: `combat.rs`, `needs_actions.rs`, `production_actions.rs`, `stock_actions.rs`, `transport_actions.rs`, `trade_actions.rs`, `facility_queue_actions.rs`, `escort_actions.rs`, `patrol_actions.rs`, `travel_actions.rs`, `bandit_camp_actions.rs`, `tell_actions.rs`, `consult_record_actions.rs`, `ask_about_person_actions.rs`, `epistemic_actions.rs`, `search_actions.rs`, `investigate_actions.rs`, `report_actions.rs`, `justice_actions.rs`, `office_actions.rs`, `artifact_actions.rs`)

Run `rg -n "effect_schema: .*EffectSchema::empty\(\)" crates/` during reassessment/closeout to enumerate the live empty-schema surface.

## Files to Touch

- `crates/worldwake-sim/src/effect_schema.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export new types)
- `crates/worldwake-sim/src/action_def.rs` (modify — add field; touch test factory at line 62)
- `crates/worldwake-sim/src/interrupt_abort.rs` (modify — line 321)
- `crates/worldwake-sim/src/action_handler.rs` (modify — line 500)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — lines 95, 316, 320)
- `crates/worldwake-systems/src/combat.rs` (modify — multiple sites)
- `crates/worldwake-systems/src/needs_actions.rs` (modify)
- `crates/worldwake-systems/src/production_actions.rs` (modify)
- `crates/worldwake-systems/src/stock_actions.rs` (modify)
- `crates/worldwake-systems/src/transport_actions.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `crates/worldwake-systems/src/facility_queue_actions.rs` (modify)
- `crates/worldwake-systems/src/escort_actions.rs` (modify)
- `crates/worldwake-systems/src/patrol_actions.rs` (modify)
- `crates/worldwake-systems/src/travel_actions.rs` (modify)
- `crates/worldwake-systems/src/bandit_camp_actions.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify)
- `crates/worldwake-systems/src/ask_about_person_actions.rs` (modify)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify)
- `crates/worldwake-systems/src/search_actions.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-systems/src/report_actions.rs` (modify)
- `crates/worldwake-systems/src/justice_actions.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — test/helper constructor fallout)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — test/helper constructor fallout)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — test/helper constructor fallout)
- `crates/worldwake-ai/src/goal_model.rs` (modify — test/helper constructor fallout)
- `crates/worldwake-ai/src/plan_guard_build.rs` (modify — test/helper constructor fallout)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — test/helper constructor fallout)
- `crates/worldwake-ai/src/planning_state.rs` (modify — test/helper constructor fallout)

## Out of Scope

- Implementing `EffectSink` trait methods on a real authoritative or hypothetical backing (ticket 002).
- Populating any non-empty `EffectSchema` (replaced by tickets 003–009 per category).
- Reading `effect_schema` from runtime code paths (ticket 010 switches the planner).
- Changing imperative action handler bodies (per-category tickets 003–009 replace them with `apply_effects(..., Authoritative)` calls).
- `guard_template`/`expectation_template` modification (preserved unchanged per spec Non-Goals — Q1=(b) resolution).
- `apply_hypothetical_transition`, `PlannerTransitionKind`, `apply_planner_step` deletion (ticket 010).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim` — existing tests in `action_def*.rs`, `action_handler*.rs`, `interrupt_abort.rs` pass with the new field present.
2. `cargo test -p worldwake-systems` — every `register_*_action` site compiles with the empty-schema field.
3. `cargo test -p worldwake-ai` — all 36 golden tests pass; planner still uses `apply_hypothetical_transition` (unchanged) and produces identical event logs.
4. `cargo clippy --workspace --all-targets -- -D warnings` — no warnings from the new module or field.

### Invariants

1. Bitwise-identical canonical state hash (`blake3` over post-replay ECS) before and after this ticket on `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` over 1440 ticks.
2. `EffectSchema::empty()` is the only construction path used in this ticket; no `EffectSchema` literal with non-empty `preconditions` or `steps` is introduced (those live in per-category tickets).
3. `EffectSink` trait has method signatures only in production — no authoritative or hypothetical sink implementations land in this ticket. A `#[cfg(test)]` no-op sink exists only to prove the empty-schema `apply_effects` return path.
4. `apply_effects` is defined but called from zero runtime sites in this ticket (verified via `rg -n "apply_effects\(" crates/`; only the definition and focused unit test match).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/effect_schema.rs` — small inline `#[cfg(test)]` block with `EffectSchema::empty()` round-trip and `apply_effects` no-op return for empty schema (~3 focused tests).
2. Existing tests in `crates/worldwake-sim/src/action_def_registry.rs`, `action_handler.rs`, `action_handler_registry.rs`, `interrupt_abort.rs` — modified only to include the new field in their `ActionDef` literal factories. No semantic changes.

### Commands

1. `cargo test -p worldwake-sim effect_schema` (after writing the new module's tests)
2. `cargo test -p worldwake-systems` (verify all action registrations compile)
3. `cargo test -p worldwake-ai` (full AI/golden package suite; ignored 1440-tick cases are run separately below)
4. `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
5. `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
6. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-04.

- Added `crates/worldwake-sim/src/effect_schema.rs` with the staged public type surface: `EffectSchema`, `EffectMode`, `EffectPrecondition`, `EffectStep`, `EffectFact`, `EffectOutcome`, `EffectSink`, and inert `apply_effects`.
- Re-exported the new effect-schema surface from `worldwake-sim`.
- Added required `ActionDef.effect_schema: EffectSchema` immediately after `expectation_template`.
- Populated every live explicit `ActionDef` literal with `EffectSchema::empty()`. The live fallout included `worldwake-ai` planner/test fixtures beyond the original draft's `worldwake-sim`/`worldwake-systems` file list.
- Left runtime behavior unchanged: no production code reads `effect_schema` or calls `apply_effects` yet.

## Deviations

- Drafted constructor count was stale (`44`); closeout count is 127 empty-schema insertions across `crates/`.
- `SAVE_FORMAT_VERSION` was not bumped because `ActionDef` registry data is not part of the persisted `SimulationState`/runtime save payload.
- The only `EffectSink` implementation in this ticket is a test-only `NoopSink` used by the focused empty-schema `apply_effects` unit test; no production authoritative or hypothetical sink landed.
- `./scripts/verify.sh` was not run because this was not a PR-prep request. Its required live gates were covered directly by focused tests, affected package tests, ignored survival goldens, and CI-shaped clippy.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-sim effect_schema`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `rg -n "apply_effects\(" crates/` runtime-read scan: only the function definition and focused unit test call matched.
- Passed scoped source diff hygiene: `git diff --check -- crates/worldwake-ai/src crates/worldwake-sim/src crates/worldwake-systems/src`
- Passed untracked-file whitespace checks for `tickets/S134CANEFFSCH-001.md` and `crates/worldwake-sim/src/effect_schema.rs` (`git diff --check --no-index -- /dev/null <path>` emitted no diagnostics).
