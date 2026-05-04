# S134: Canonical Effect Schema for ActionDef

**Status**: Draft

## Summary

Worldwake currently maintains three parallel forward models per action:

1. **Imperative authoritative handlers** (`crates/worldwake-systems/src/*_actions.rs`) — start/tick/commit/abort handler bodies that mutate authoritative ECS through the scheduler.
2. **Explicit per-action hypothetical transitions** — 8 non-fallback arms of `PlannerTransitionKind` dispatched by `apply_hypothetical_transition` (`crates/worldwake-ai/src/planner_ops.rs:311`): `ConsumeMatchingTargetCommodity`, `PickUpGroundLot`, `StealGroundLot`, `PutDownGroundLot`, `StoreStockIntoLocalFacility`, `StageStoredStockForSale`, `CollectFacilityStockToPossession`, `UnstageDisplayedStock`. These mutate `PlanningState` overlays (`crates/worldwake-ai/src/planning_state.rs:46`).
3. **Per-`GoalKind` hypothetical transitions** — `GoalKindPlannerExt::apply_planner_step` (`crates/worldwake-ai/src/goal_model.rs:54` trait method, `:1051` impl block) reached by every action that maps to `PlannerTransitionKind::GoalModelFallback` (currently the majority — combat, travel, trade, queue, patrol, tell, investigate, social actions, etc.). This also mutates `PlanningState` overlays, but keyed on the goal rather than the action.

`crates/worldwake-ai/tests/planner_conformance.rs` guards against drift between (1) and (2)/(3), but the seam is the highest-risk surface in the planner — drift means agents reason into impossible plans or skip plans the world would actually permit.

S134 collapses all three paths into a single canonical effect schema attached to each `ActionDef`. The schema is a declarative statement of preconditions, consumed inputs, produced outputs, mutated relations, and emitted events. A shared evaluator applies the schema in two modes: `EffectMode::Authoritative` writes to the ECS and event log through the scheduler; `EffectMode::Hypothetical` writes to a `PlanningState` overlay. Both `apply_hypothetical_transition` (with its `PlannerTransitionKind` dispatch) and `GoalKindPlannerExt::apply_planner_step` are deleted; the planner calls `apply_effects(schema, …, EffectMode::Hypothetical)` for forward-model evaluation. Action handlers stop carrying their own forward-model code. Conformance tests become precondition-coverage tests over the schema rather than dual-implementation diff tests.

`EffectSchema` is layered alongside the existing `ActionDef.guard_template` / `ActionDef.expectation_template` (landed by S114, `crates/worldwake-sim/src/action_def.rs:141,143`): plan-step guards/expectations remain the cross-step plan-validity surface (assumptions agents commit to and revisit), while `EffectSchema` is the action-interior forward model. The two surfaces serve different time horizons and remain distinct.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Implementation Notes

- Ticket 002 landed the first real `EffectSink` implementations and made sink writes fallible. `apply_effects` emits facts only after the sink accepts each write.
- The authoritative sink lives in `worldwake-systems` over `WorldTxn`; it covers current commodity transfer/consume/produce, event-tag, expectation, and contention-grant surfaces. It does not expose a generic transaction snapshot/restore. Future schemas that require authoritative all-or-nothing multi-step semantics must add an explicit atomic effect shape, preflight discipline, or transaction support before relying on rollback.
- `EffectStep::ApplyWound` remains a staged generic variant. Combat attack now uses `EffectStep::ResolveCombatAttack`, because authoritative wound construction depends on combat profiles, stance, payload weapon, existing wounds, tick, and seeded RNG.
- Combat action commit handlers now call `apply_effects_with_context(...)` through a combat-owned delegation helper. Needs action commit handlers now call the same evaluator through a needs-owned delegation helper. Other runtime handler categories still have not migrated.
- Ticket 003 corrects an important substrate mismatch in the first draft: `ActionDef.effect_schema` is registry-time template data, so schema operands cannot be literal runtime `EntityId`s only. The live schema language now includes `EffectEntityRef::{Actor, Target, Entity}` and `EffectActionRef` so registry templates can lawfully bind actor, target, and payload-derived action identities at evaluation time. The original `apply_effects(...)` wrapper remains for existing callers; action handlers that need payload/current-action context use `apply_effects_with_context(...)`.
- Combat migration adds authoritative combat effect steps for the commit-time world mutations that cannot be represented as commodity transfer alone: contention queue entry, contention membership cleanup, capacity-limited corpse loot, corpse burial, attack wound/evidence resolution, and wound-resolution contention cleanup. These are typed effect steps interpreted by the combat-owned authoritative sink; they are not wrappers around a parallel live commit path.
- Needs migration adds authoritative needs effect steps for branch-specific needs actions: consuming a bound consumable lot from its concrete item profile, ending sleep episodes, using a latrine, relieving in wilderness, and using a wash basin. These are typed effect steps interpreted by a needs-owned authoritative sink because the old handlers carry domain payloads and aftermath, not just a generic need-delta write.
- The hypothetical sink resolves the new runtime entity refs for existing generic effects but still rejects category-specific staged steps until the planner switch ticket implements mode parity for the expanded effect language. This is intentional staged substrate: the planner still uses the old hypothetical path through tickets 003-009, and ticket 010 owns replacing that path only after every category schema has a verified hypothetical interpretation.

## Crates

- `worldwake-sim` — new `effect_schema` module owning `EffectSchema`, `EffectMode`, `EffectStep`, `EffectPrecondition`, `EffectFact`, `EffectOutcome`, the `EffectSink` trait, and the unified `apply_effects(...)` evaluator. Extends `ActionDef` with a required `effect_schema: EffectSchema` field. `binding_strictness`, `guard_template`, and `expectation_template` remain on `ActionDef` (layered roles, not absorbed by `EffectSchema`).
- `worldwake-systems` — every `register_*_action` (~24 registration files, ≥40 individual action definitions including composites like `register_needs_actions`, `register_craft_actions`, `register_harvest_actions`, `register_office_actions`, `register_artifact_actions`) replaces the imperative handler body with a constructed `EffectSchema`. Per-action commit-trace data (`CommitTraceData::Harvest.partial_quantity`, `CommitTraceData::Tell`) becomes a typed output of the schema rather than handler-internal logic. The authoritative `EffectSink` impl lives here over `WorldTxn`.
- `worldwake-ai` — `planner_ops.rs` deletes `apply_hypothetical_transition` and the `PlannerTransitionKind` enum and dispatch. `goal_model.rs` deletes the `apply_planner_step` method on `GoalKindPlannerExt` (and all per-`GoalKind` impls). The planner calls `apply_effects(&action_def.effect_schema, …, EffectMode::Hypothetical)` for forward-model evaluation against a `PlanningState`-backed `EffectSink` impl that lives here. Conformance tests become precondition-and-mode coverage tests.
- `worldwake-core` — no new component. `Discrepancy` (already at `crates/worldwake-core/src/discrepancy.rs:8`) is reused as the schema-evaluation failure type. `EventTag`, `EntityId`, `Quantity`, `Permille`, `CommodityKind`, `BeliefClaimKey`, `WoundCause` are all reused directly by `EffectStep`/`EffectPrecondition` variants — no new core type unless an existing handler used a non-typed primitive that must become typed.
- `worldwake-cli` — no change. Scenarios continue to load `ActionDef` via the existing registry.

## Dependencies

- S114 (Plan Step Guards) — completed (archived at `archive/specs/S114-plan-step-guards.md`). `PlanGuard` and `PlanExpectation` already declare per-step preconditions and expected effects in declarative shape; `EffectSchema` is layered alongside the existing `ActionDef.guard_template`/`expectation_template` rather than replacing them.
- S108 (Per-Action Binding Strictness) — completed (archived at `archive/specs/S108-per-action-binding-strictness.md`). `BindingStrictness` remains on `ActionDef` and is the targeting layer; `EffectSchema` is the post-binding evaluation layer.
- S109 (Typed Discrepancy Taxonomy) — completed (archived at `archive/specs/S109-typed-discrepancy-taxonomy.md`). Schema-precondition failures classify into the existing `Discrepancy` taxonomy (11 variants in `worldwake-core/src/discrepancy.rs`).
- S127 (Quantity-Aware Acquisition) — completed (archived at `archive/specs/S127-quantity-aware-acquisition.md`). `CommitTraceData::Harvest.partial_quantity` (already at `crates/worldwake-sim/src/action_handler.rs:45–47`) becomes one of the schema's typed effect outputs.

## Design Goals

1. **Single forward model per action.** No action ever has both an imperative handler body and a parallel hypothetical-transition path. The schema is the truth; both the authoritative scheduler path and the hypothetical planner path consume it. Both `apply_hypothetical_transition` (per-action `PlannerTransitionKind` dispatch) and `apply_planner_step` (per-`GoalKind` dispatch) are deleted; their work is subsumed.
2. **Declarative preconditions.** Every schema lists `EffectPrecondition`s (target-shape constraints, co-location requirements, minimum quantities, capacity floors, role/office requirements). Failure produces a typed `Discrepancy` already known to S109.
3. **Declarative outputs.** Every schema lists `EffectStep`s. Each step names the world fact it asserts (commodity transfer, wound application, expectation fulfillment, contention-grant consumption, …) using existing core types (`EntityId`, `CommodityKind`, `Quantity`, `EventTag`, `WoundCause`, `BeliefClaimKey`). The evaluator interprets each fact in the active mode.
4. **Mode parity at the evaluator layer.** `EffectMode::Authoritative` and `EffectMode::Hypothetical` differ only in their sinks — the ECS scheduler vs the `PlanningState` overlay. The schema interpretation is identical.
5. **Partial-outcome support.** Schemas can declare `EffectStep::PartialOnFailure { primary, fallback }` so that a contended harvest can yield 3 of the requested 5 units (already present as ad-hoc `partial_quantity` in S127). The fallback is itself an `EffectStep` chain.
6. **Conformance via coverage, not duplication.** The current dual-implementation conformance test is replaced by: (a) a coverage test that every registered `ActionDef` has a non-empty `EffectSchema`; (b) a precondition-completeness test that every `Discrepancy` variant the simulator can record on action failure is reachable via at least one schema's preconditions.
7. **No silent privilege.** Schemas may not skip the contention substrate, override locality, or invoke other systems' privileged behavior. The evaluator only writes to the ECS / overlay through the same write paths action handlers used (FND-26).
8. **Determinism.** Schema interpretation is deterministic over `BTreeMap`-ordered inputs. No floats, no wall-clock time. The evaluator runs entirely on integer math over `Permille` values where applicable.

## Non-Goals

- **Replacing `BindingStrictness`.** The targeting layer (which entity satisfies the action's slot) remains a separate concern carried by S108's `BindingStrictness`.
- **Replacing `guard_template`/`expectation_template`.** The plan-step guard/expectation surface (S114) remains on `ActionDef` and continues to encode cross-step plan-validity assumptions. `EffectSchema` is the action-interior forward model; the two surfaces serve different time horizons and remain distinct.
- **Replacing the scheduler.** The evaluator does not bypass the action scheduler; for `EffectMode::Authoritative`, the scheduler still owns ordering, contention resolution, and event emission. The evaluator produces the same effect facts the scheduler would consume.
- **Live save-format break.** `EffectSchema` lives on the in-memory `ActionDef` registry, not on persisted state. `SAVE_FORMAT_VERSION` (`crates/worldwake-sim/src/save_load.rs:2`) does not change unless a schema's typed output adds a new persistent component (handled per-action, not by S134 itself).
- **A new event tag.** No `EffectApplied` event tag is introduced. Each typed effect output already maps to an existing simulator event tag.
- **HTN methods.** Methods (deferred to Phase 12) are a separate layer above the action registry. S134 only unifies the per-action forward model.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Schema outputs are typed effect facts grounded in concrete world state (commodity transfer, wound application, queue grant), never abstract scores. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | Preconditions and durations are declared in the schema rather than scattered across handler bodies. Cost/occupancy continue to be carried by `ActionDef`'s existing `body_cost_per_tick`, `attention_cost`, and `interruptibility` fields (no new occupancy type introduced). |
| FND-9 (Scheduling, Simultaneity, and Tie-Breaking Are Part of the World Model) | Authoritative mode preserves the scheduler's contention/ordering authority; the schema only declares effects, not the resolution rule. |
| FND-10 (Outcomes Are Granular and Leave Aftermath) | `EffectStep::PartialOnFailure` makes partial outcomes a first-class shape of the schema rather than ad-hoc handler logic. |
| FND-12 (Performance May Compress Computation, Never Causality) | Hypothetical mode runs the same schema as authoritative mode against an overlay — no computation compression that drops effects. The bitwise-identical event-log requirement enforces this. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | The evaluator writes through the same ECS sinks as the scheduler. Schemas do not invoke other systems directly. The `EffectSink` trait abstraction (defined in `worldwake-sim`) is the only cross-crate seam — implemented by sim/systems (authoritative) and ai (hypothetical), keeping `worldwake-sim` ignorant of `PlanningState`. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The old `apply_hypothetical_transition` path, the `PlannerTransitionKind` enum, the `apply_planner_step` per-`GoalKind` method, and the old per-handler imperative bodies are deleted, not preserved as fallbacks. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | This spec is a structural refactor with no new causal hooks; behavior is bitwise-identical (Validation section). Section H is preserved below for documentation completeness, not because new hooks are introduced. |

## Deliverables

### `worldwake-sim::effect_schema` (new module)

Type sketch using existing core types directly (no `*Key` / `*Ref` indirection):

```rust
pub struct EffectSchema {
    pub preconditions: Vec<EffectPrecondition>,
    pub steps: Vec<EffectStep>,
    // duration/occupancy/cost stay on ActionDef itself (existing fields:
    // duration: DurationExpr, body_cost_per_tick, attention_cost, interruptibility)
    // — EffectSchema only carries the effect language.
}

pub enum EffectMode {
    Authoritative,
    Hypothetical,
}

pub enum EffectPrecondition {
    /// Slot constraint references an existing TargetSpec variant.
    TargetMatchesSlot { slot_index: usize, shape: TargetSpec },
    CoLocated { actor: EffectEntityRef, target: EffectEntityRef },
    QuantityAvailable { source: EffectEntityRef, commodity: CommodityKind, min: Quantity },
    CapacityFloor { container: EntityId, min_free: Quantity },
    RoleAuthority { actor: EntityId, role: /* existing role enum from worldwake-core */ },
    ContentionGrantHeld { actor: EffectEntityRef, affordance: EffectEntityRef },
    BeliefHeld { agent: EffectEntityRef, claim: BeliefClaimKey },
    // ... one variant per category of precondition currently checked imperatively
}

pub enum EffectEntityRef {
    Actor,
    Target { index: usize },
    Entity(EntityId),
}

pub enum EffectActionRef {
    CurrentAction,
    PayloadQueueIntendedAction,
    Action(ActionDefId),
}

pub enum EffectStep {
    Transfer { source: EffectEntityRef, dest: EffectEntityRef, commodity: CommodityKind, quantity: Quantity },
    Consume { source: EffectEntityRef, commodity: CommodityKind, quantity: Quantity },
    Produce { sink: EffectEntityRef, commodity: CommodityKind, quantity: Quantity },
    // Currently staged after ticket 002; combat must extend or replace this
    // shape with enough payload for real wound construction.
    ApplyWound { target: EffectEntityRef, cause: WoundCause },
    EmitEvent { tag: EventTag /* payload uses existing per-tag payload types, no EventPayloadKey indirection */ },
    AssertExpectationFulfilled { expectation: ExpectationId },
    ConsumeContentionGrant { grant: EffectEntityRef },
    EnqueueContention { actor: EffectEntityRef, entity: EffectEntityRef, intended_action: EffectActionRef },
    ClearContentionMembership { actor: EffectEntityRef, entity: EffectEntityRef, action: EffectActionRef },
    LootPossessionsWithinCapacity { looter: EffectEntityRef, corpse: EffectEntityRef },
    BuryCorpse { corpse: EffectEntityRef, burial_site: EffectEntityRef },
    ResolveCombatAttack { attacker: EffectEntityRef, target: EffectEntityRef },
    ClearEntityContentionIfNoWounds { entity: EffectEntityRef },
    ConsumeTargetConsumable { target: EffectEntityRef, effect: ConsumableEffect },
    EndSleepEpisode,
    UseToilet,
    RelieveWilderness,
    UseWashBasin { basin: EffectEntityRef },
    PartialOnFailure { primary: Vec<EffectStep>, fallback: Vec<EffectStep> },
    // ... one variant per kind of authoritative effect that handlers currently produce
}

/// Typed outputs threaded back to the caller for trace/event emission.
pub enum EffectFact {
    // EffectFact is NEW in worldwake-sim; the prior draft's claim that it
    // reuses PlanningFact (worldwake-ai/src/search/landmarks.rs:12) is dropped:
    // PlanningFact is pub(super) and carries only 6 generic landmark variants,
    // unsuitable for the typed-effect surface this enum needs.
    CommodityTransfer { source: EntityId, dest: EntityId, commodity: CommodityKind, quantity: Quantity },
    PartialQuantity { requested: Quantity, delivered: Quantity },
    WoundApplied { target: EntityId, cause: WoundCause },
    ExpectationFulfilled { expectation: ExpectationId },
    ContentionGrantConsumed { /* existing grant reference */ },
    EventEmitted { tag: EventTag },
}

pub struct EffectOutcome {
    pub facts: Vec<EffectFact>,
}

/// Sink abstraction so worldwake-sim never names worldwake-ai types.
/// Authoritative impl lives in worldwake-systems (or worldwake-sim if generic).
/// Hypothetical impl over PlanningState lives in worldwake-ai.
pub trait EffectSink {
    fn check_precondition(&self, /* … */) -> Result<(), Discrepancy>;
    fn checkpoint(&mut self) -> usize;
    fn restore(&mut self, checkpoint: usize) -> Result<(), Discrepancy>;
    fn write_transfer(&mut self, /* … */) -> Result<(), Discrepancy>;
    fn write_consume(&mut self, /* … */) -> Result<(), Discrepancy>;
    fn write_produce(&mut self, /* … */) -> Result<(), Discrepancy>;
    fn write_wound(&mut self, /* … */) -> Result<(), Discrepancy>;
    fn write_event(&mut self, /* … */) -> Result<(), Discrepancy>;
    fn consume_grant(&mut self, /* … */) -> Result<(), Discrepancy>;
    // ... one method per EffectStep variant
}

pub fn apply_effects(
    schema: &EffectSchema,
    actor: EntityId,
    targets: &[EntityId],          // matches ActionInstance shape
    sink: &mut dyn EffectSink,
    mode: EffectMode,
) -> Result<EffectOutcome, Discrepancy> { /* … */ }

pub fn apply_effects_with_context(
    schema: &EffectSchema,
    context: EffectEvaluationContext<'_>, // actor, targets, payload, current action id
    sink: &mut dyn EffectSink,
    mode: EffectMode,
) -> Result<EffectOutcome, Discrepancy> { /* … */ }
```

Type-naming notes:

- The prior draft's `CommodityKey`, `ContentionGrantKey`, `ExpectationKey`, `EventPayloadKey`, `AffordanceKey`, `AuthorityKind`, `ActionSlot`, `TargetShape`, `ActionOccupancy`, `QuantityExpr`, `ResolvedBinding`, `WoundSeverity` types do not exist in the codebase and are not introduced. Use existing types: `EffectEntityRef` for actor/target/entity operands in registry templates, `EntityId` for already-resolved entities, `CommodityKind` for commodities, `ExpectationId` for expectations, `Quantity` for quantities, `EventTag` for event tags, `TargetSpec` for slot shape, `WoundCause` (no severity enum exists today), `BindingStrictness` (already on `ActionDef`).
- The ~37 `register_*_action` functions across 24 files in `worldwake-systems/src/` accept actor + target through `ActionInstance` (`worldwake-sim/src/action_instance.rs:6–21`); `apply_effects` matches that shape directly rather than introducing `ResolvedBinding`.

### `ActionDef` extension (in `worldwake-sim`)

```rust
pub struct ActionDef {
    // existing fields preserved (id, name, domain, actor_constraints, targets,
    // preconditions, reservation_requirements, duration, body_cost_per_tick,
    // attention_cost, interruptibility, commit_conditions, visibility,
    // causal_event_tags, payload, handler, binding_strictness)
    pub binding_strictness: BindingStrictness,                    // unchanged
    pub guard_template: Option<GuardTemplateSpec>,                // unchanged (S114)
    pub expectation_template: Vec<ExpectationTemplateSpec>,       // unchanged (S114)
    pub effect_schema: EffectSchema,                              // NEW (required, non-optional)
}
```

`guard_template` and `expectation_template` are not absorbed by `EffectSchema`. They serve a different time horizon (cross-step plan-validity assumptions agents commit to and revisit) than `EffectSchema` (per-action evaluation at commit time).

### `worldwake-ai::planner_ops` deletions

- Delete `apply_hypothetical_transition` (`crates/worldwake-ai/src/planner_ops.rs:311`), the `PlannerTransitionKind` enum (`:69–79`), and the per-arm dispatch in `semantics_for` and adjacent helpers.
- Delete the re-export of `apply_hypothetical_transition` from `crates/worldwake-ai/src/lib.rs:116`.
- Replace the single runtime call site at `crates/worldwake-ai/src/search/transition.rs:154` with `apply_effects(&action_def.effect_schema, actor, targets, &mut overlay_sink, EffectMode::Hypothetical)`.
- Delete the test-only call sites in `planner_ops.rs:2268, 2305, 2343, 2374, 2418, 2480, 2551, 2593` along with the function itself; the conformance harness rewrite (below) replaces them.

(The prior draft's references to `crates/worldwake-ai/src/search/mod.rs` and `crates/worldwake-ai/src/agent_tick/planning.rs` are dropped — neither file calls `apply_hypothetical_transition`.)

### `worldwake-ai::goal_model` deletions

- Delete the `apply_planner_step` method from the `GoalKindPlannerExt` trait (`crates/worldwake-ai/src/goal_model.rs:54`) and all per-`GoalKind` implementations (`:1051` impl block onward).
- Delete every test that exercises `apply_planner_step` (~14+ usages in `goal_model.rs` test module).
- The work currently performed by `apply_planner_step` (per-`GoalKind` overlay mutation reached via `PlannerTransitionKind::GoalModelFallback`) is subsumed by `apply_effects(&action_def.effect_schema, …, EffectMode::Hypothetical)` invoked in the planner search loop. Each action that previously fell through to `GoalModelFallback` now carries a complete `EffectSchema` describing its hypothetical transition explicitly.

### Action handler migration (in `worldwake-systems`)

For each of the ~24 registration files (combat, needs_actions, production_actions, trade_actions, facility_queue_actions, escort_actions, patrol_actions, tell_actions, consult_record_actions, ask_about_person_actions, epistemic_actions, investigate_actions, justice_actions, transport_actions, travel_actions, artifact_actions, office_actions, bandit_camp_actions, stock_actions, report_actions, search_actions, action_registry, perception, needs), each `register_*_action` declaration constructs an `EffectSchema` literal listing preconditions and steps. The handler body shrinks to:

```rust
fn handle(...) -> ActionOutcome {
    let schema = action_def.effect_schema.clone();
    apply_effects(&schema, actor, targets, &mut authoritative_sink, EffectMode::Authoritative)
        .map(ActionOutcome::Completed)
        .unwrap_or_else(|d| ActionOutcome::Failed(d))
}
```

The imperative body is removed. Composite registrations (e.g., `register_needs_actions` registers six actions internally; `register_craft_actions` and `register_harvest_actions` each register multiple commodity-specific definitions) each construct one schema per action definition. Total scope: ≥40 individual action definitions.

### Conformance test rewrite (in `worldwake-ai/tests/planner_conformance.rs`)

- Replace the existing per-action dual-implementation diff tests (currently 21 conformance tests, count drifts as new actions land — do not pin a specific count in spec or tickets) with:
  - `every_actiondef_has_effect_schema()` — registry coverage assertion.
  - `every_discrepancy_variant_reachable_from_some_schema_precondition()` — taxonomic completeness (covers all 11 variants of `Discrepancy` plus future additions).
  - `partial_outcome_steps_emit_typed_facts()` — no ad-hoc handler-internal `partial: bool`.

## FND-01 Section H — Causal Hooks Declaration

S134 is a structural refactor with bitwise-identical behavior; per FOUNDATIONS-alignment guidance, Section H is informational rather than load-bearing for refactor specs. Preserved here for completeness.

1. **Information-path analysis.** S134 introduces no new information-flow path. Schema interpretation reads the same authoritative state action handlers read today (component values, container contents, queue state) and the same overlay state the planner already uses. No system gains a new authoritative read.
2. **Positive-feedback analysis.** No new amplifying loop. The schema is interpretation, not generation — a planner cycle that schedules an action, applies effects hypothetically, and commits is identical in shape to today's cycle.
3. **Concrete dampeners.** Not applicable — no new amplifier.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: `ActionDef.effect_schema` (registry-time constant, not per-tick state). All ECS components and event-log entries the schema produces remain authoritative as today.
   - **Derived read-model**: The `PlanningState` overlay produced during hypothetical evaluation is and remains a per-tick derived view, never persisted (already true pre-S134).

## SystemFn Integration

S134 does not introduce a new `SystemFn`. Action evaluation remains under the scheduler's existing system, and hypothetical evaluation remains a per-planning-cycle synchronous call from the AI planner. The unification is internal to those existing call sites.

## Component Registration

No new components. `ActionDef` is registry data, not an ECS component.

## Cross-System Interactions

- **AI ↔ Sim**: AI calls `apply_effects(..., EffectMode::Hypothetical)` against the same `ActionDef.effect_schema` the simulator interprets in `EffectMode::Authoritative`. The `EffectSink` trait (defined in `worldwake-sim`) is the only seam — `worldwake-sim` never names `PlanningState` (which lives in `worldwake-ai/src/planning_state.rs:46`); the hypothetical sink impl over `PlanningState` lives in `worldwake-ai`. Workspace layering (`core → sim → systems → ai → cli`) is preserved.
- **Systems ↔ Sim**: Each `register_*_action` constructs its `EffectSchema` declaratively at registration time. The scheduler interprets it at execution time. Systems remain decoupled from each other.
- **Sim ↔ Core**: `EffectStep` and `EffectPrecondition` variants reference existing core types (`EntityId`, `CommodityKind`, `Quantity`, `EventTag`, `BeliefClaimKey`, `WoundCause`). `Discrepancy` (`worldwake-core/src/discrepancy.rs:8`) is reused as the schema-evaluation failure type. No new core type is introduced unless an existing handler used a non-typed primitive that must become typed.

## Profile-Driven Parameters

Not applicable — `EffectSchema` is a registry-time per-action constant, not a per-agent profile. Per-agent variation in action *outcomes* (skill-driven success rate, profile-driven duration scaling) remains where it is today: applied to the schema's evaluation by the existing per-agent components, not by the schema itself.

## Validation and Falsification

- **Conformance**: `every_actiondef_has_effect_schema()` and `every_discrepancy_variant_reachable_from_some_schema_precondition()` assert taxonomic completeness.
- **Migration regression**: Every existing golden in `crates/worldwake-ai/tests/golden_*.rs` (currently ~36 files) continues to pass without modification. The schema-driven evaluation must produce bitwise-identical event logs to the pre-S134 imperative path on every committed scenario seed.
- **Planner determinism**: `crates/worldwake-ai/tests/planner_conformance.rs` is updated as above; the existing dual-implementation conformance tests are replaced by precondition-coverage and partial-outcome-typedness tests.
- **Soak**: 1440-tick replay of `scenarios/survival-baseline.ron`, `scenarios/survival-scattered.ron`, `scenarios/survival-contested.ron` produce identical canonical state hashes (`blake3` over the post-replay ECS) before and after S134.

## Risks

- **Migration scope.** Every action registration must be migrated. Scope: ~24 `register_*_action` files in `worldwake-systems/src/` and ≥40 individual action definitions (composites like `register_needs_actions`, `register_craft_actions`, `register_harvest_actions`, `register_office_actions`, `register_artifact_actions` each register multiple action definitions internally). Mitigation: ticket-decompose per action category; each category becomes its own ticket with a focused golden subset.
- **Schema expressiveness.** Some current handlers carry conditional logic (e.g., per-action `BindingStrictness` override at `crates/worldwake-systems/src/needs_actions.rs:183–188`, which assigns different `BindingStrictness` variants per action name). The schema must expose conditional-step or conditional-precondition shapes that match. Mitigation: identify the irreducible conditionality during the first migration ticket (combat); if it cannot be declarative, narrow `EffectStep::Conditional` to the smallest necessary form.
- **`PlanningState` overlay coverage.** The overlay must support every effect the schema can produce through the `EffectSink` impl. Mitigation: ticket-decomposition pairs each handler-migration ticket with a corresponding overlay-extension test. The current 17-field overlay shape (`crates/worldwake-ai/src/planning_state.rs:46`) already covers the substrates the 8 explicit `PlannerTransitionKind` arms touch — gap is in the coverage that previously fell through to `apply_planner_step`.
- **Per-`GoalKind` semantics absorption.** Deleting `apply_planner_step` requires that every action previously routed through `PlannerTransitionKind::GoalModelFallback` now carries a complete `EffectSchema`. Mitigation: the first migration tickets explicitly target the goal-model-fallback substrate; precondition-coverage tests catch any goal-shaped semantics that didn't translate to action-shaped schemas.
