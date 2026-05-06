# Worldwake Validation Patterns

Project-specific patterns for reassess-spec. When a spec proposes one of the triggers below, verify the corresponding integration points exist in the spec. Flag missing items as HIGH Issues.

## Pattern Triggers Map to Deliverables, Not Prose Only

When any of the patterns below trigger, every named integration point must appear as an itemized deliverable (D-section) in the spec's `## Deliverables` section. Prose-only references in Summary, Design Goals, Cross-System Interactions, or FOUNDATIONS Alignment do not substitute. Flag the missing deliverable as a HIGH Issue with the pattern's integration-point list as the recommendation.

## New GoalKind Variant

**Trigger**: Spec adds a variant to `GoalKind` in `crates/worldwake-core/src/goal.rs`.

**Verify the spec addresses**:

1. `GoalDispatchKey` — new variant + `ALL` constant + `from_goal_kind` match arm (`crates/worldwake-ai/src/goal_dispatch_key.rs`)
2. `GoalDispatchDeclaration` — entry with `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `progress_barrier_ops`, `family_policy` (`crates/worldwake-ai/src/goal_dispatch_decl.rs`)
3. `GoalKindPlannerExt` — implementation of all 12 methods (`crates/worldwake-ai/src/goal_model.rs`)
4. Ranking — `GoalPriorityClass` assignment and `motive_score` formula
5. Candidate generation — emitter function in `crates/worldwake-ai/src/candidate_generation.rs`
6. `GoalKind` derive compatibility — new variant fields must all be `Copy` (GoalKind derives Copy)

## New Component on EntityKind::Agent

**Trigger**: Spec adds a new ECS component registered on `EntityKind::Agent`.

**Verify the spec addresses**:

1. `component_schema.rs` — registration with insert/get accessors
2. `AgentDef` — field in `crates/worldwake-cli/src/scenario/types.rs`
3. `spawn_agent()` — set_component call in `crates/worldwake-cli/src/scenario/mod.rs`
4. Classification — one of (a) scenario-authorable universal, (b) scenario-authorable role-specific, or (c) runtime-only / scenario-exempt (analogous to `ActiveGoal`, `IntentionFrame`, `WoundList`). Per `docs/spec-drafting-rules.md` Section 5. For (c), the component still requires `component_schema.rs` registration and `create_agent()` insertion with a default-empty value, but **no** `AgentDef` field, **no** `*Def` wrapper, and **no** `spawn_agent()` `set_component_*` call.
5. `Default` impl if (a) or (c)
6. `*Def` wrapper type if (a) or (b) and the component contains `EntityId` references
7. **Core-residence constraint**: the component struct itself MUST be defined in `worldwake-core`. The `with_component_schema_entries!` macro at `crates/worldwake-core/src/component_schema.rs:3` references types via `crate::TypeName`, so components defined in `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, or `worldwake-cli` cannot be registered through this path. Specs that propose a new component outside core must either (a) relocate the type to core, (b) reframe it as ai-layer per-agent runtime state (follows `AgentDecisionRuntime` precedent at `crates/worldwake-ai/src/decision_runtime.rs:151`, stored in `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` at `crates/worldwake-ai/src/agent_tick/mod.rs:70`, and explicitly tested as not-a-component at `decision_runtime.rs:438`), or (c) split — define a scenario-authorable profile component in core and keep the runtime-generated state in ai. Flag any crate-list claim that puts the component struct outside core while still asserting `EntityKind::Agent` registration as a CRITICAL Issue.

**Note**: The only items exempt from `component_schema.rs` registration are transient local variables that are never stored as ECS components. If the spec calls it a "component" and proposes `create_agent()` insertion, registration is mandatory regardless of classification (a)/(b)/(c).

## New Component on EntityKind::Place

**Trigger**: Spec adds a new ECS component registered on `EntityKind::Place` (or on the combined `EntityKind::Facility | EntityKind::Place` filter).

**Verify the spec addresses**:

1. `component_schema.rs` — registration with insert/get accessors, kind filter `|kind| kind == EntityKind::Place` (or the facility+place combination at lines 1560-1635). The component struct itself MUST live in `worldwake-core` per the same core-residence constraint as Agent components.
2. `PlaceDef` — field in `crates/worldwake-cli/src/scenario/types.rs` (precedent: `visibility_profile: Option<PlaceVisibilityProfile>` at line 323). If the component contains `EntityId` references, create a `*Def` wrapper type with string names.
3. `spawn_place` loop — set_component call inside the place-iteration block in `crates/worldwake-cli/src/scenario/mod.rs` (precedent: `set_component_place_visibility_profile` at line 276).
4. **Universal vs. optional classification**:
   - **Optional precedent** (only existing pattern as of S128 reassessment): conditional `if let Some(profile) = &place_def.field { txn.set_component_*(place_id, profile.clone())?; }`. Component is absent on places the scenario does not author. Runtime reads use `Option<&Component>` accessors.
   - **Universal pattern** (no Place precedent prior to S128's `SleepQualityProfile` — flag the spec as setting a new convention if it picks this path): `spawn_place` always calls `txn.set_component_*(place_id, place_def.field.map(Into::into).unwrap_or_default())`. Runtime reads on known places use `expect()`. Mirrors the universal-on-Agent pattern (`metabolism_profile.unwrap_or_default()` at `mod.rs:576`).
5. `Default` impl if universal.
6. Sibling components co-residing on the same Place — check whether the new component duplicates or overlaps an existing place property (e.g., would the `recovery_modifier` belong on `PlaceVisibilityProfile`?). Apply 5f semantic-overlap analysis.

**Note**: As of S128's reassessment, no universal-on-Place precedent exists in the codebase. The first such spec sets the convention and inherits responsibility for the precedent — flag this in the audit and note it in the spec's Component Registration table.

## New Component Read by AI Crate

**Trigger**: Spec adds any new derived read the AI crate (candidate generation, ranking, or planning) consumes through `GoalBeliefView` — whether the underlying state is a new component, an existing component, a relation, or a composite read across multiple sources. The trigger fires on the *accessor surface*, not on whether a component is being introduced.

**Verify the spec addresses**:

1. `GoalBeliefView` accessor — new method on trait in `crates/worldwake-sim/src/belief_view.rs`
2. `RuntimeBeliefView` impl — backing implementation
3. `impl_goal_belief_view!` macro or blanket impl — forwarding the new method
4. Crate list — `worldwake-sim` must appear in the spec's Crates section (GoalBeliefView and component traits live there)

## New Action Type

**Trigger**: Spec adds a new action registered in `ActionDefRegistry`.

**Verify the spec addresses**:

1. Action definition — `ActionDomain`, name, `TargetSpec`, `DurationExpr`, interruptibility
2. Handler functions — start, tick, commit, abort
3. `PlannerOpKind` — new variant if action is plannable (not always needed if reusing existing op)
4. `classify_action_def` match arm in `crates/worldwake-ai/src/planner_ops.rs`
5. Affordance query — how the planner discovers this action is available
6. `Authoritative-to-AI Impact Rule` checklist (CLAUDE.md) if modifying preconditions

## New Scenario Design

**Trigger**: Spec creates or redesigns a `.ron` scenario file.

**Verify the spec addresses**:

1. All `WorkstationTag` values exist as enum variants (`crates/worldwake-core/src/production.rs`)
2. All `PlaceTag` values exist as enum variants (`crates/worldwake-core/src/topology.rs`). Note: `PlaceTag` (place-level property) and `WorkstationTag` (facility-level) are distinct — do not conflate them
3. Recipe names match the action registry format: Title Case with spaces (e.g., `"Harvest Grain"`, not `HarvestGrain`)
4. `AgentDef` fields match current definition in `crates/worldwake-cli/src/scenario/types.rs`
5. Commodity names match `CommodityKind` enum variants
6. If the scenario claims survival coverage, all `HomeostaticNeedId` variants (Hunger, Thirst, Fatigue, Bladder, Dirtiness) have satisfiable action paths given the proposed places, facilities, and tags
7. Action preconditions are satisfiable: check that each need-satisfaction action's required facility, tag, or possession constraint is met by at least one reachable place in the scenario

Cross-reference with existing scenarios (`scenarios/*.ron`) for structural conventions.

## Candidate Scoring Architecture

**Trigger**: Spec proposes scoring/utility computation for candidate emission (e.g., drive scores, priority weights, utility factors).

**Verify the spec's scoring model matches the actual emission-vs-ranking architecture**:

1. Emitters call `emit_candidate_with_trace` which produces `GroundedGoal` — a struct with `GoalKey`, `OpportunityAnchor`, and evidence sets. There is **no score field** on `GroundedGoal`.
2. Ranking happens separately in `crates/worldwake-ai/src/ranking.rs` via `motive_score` computation.
3. Specs that embed scoring/utility computation in the emitter (e.g., computing a `drive_score` and attaching it to a candidate struct) are architectural mismatches — flag as Issues.
4. Emitters determine **whether** to emit (gate logic: thresholds, vetoes, cooldowns). Ranking determines **relative priority** among emitted candidates.

If the spec proposes utility gates (emit only if utility > 0), that belongs in the emitter. If the spec proposes priority/ranking formulas, those belong in `ranking.rs` via `motive_score`.

## New Enum Variant on Cross-Crate Enum

**Trigger**: Spec extends an enum used across multiple crates.

**Verify**:

1. Exhaustive match sites — grep for `match` on the enum across all crates
2. Derive compatibility — new variant fields satisfy existing derives
3. `#[allow(clippy::large_enum_variant)]` — check if new variant is significantly larger than existing ones

## Core-Side Mirror Enum Pattern

**Trigger**: Spec proposes a new core-resident type (struct or enum) whose field types reference an enum defined in a higher crate (`worldwake-sim`, `worldwake-systems`, `worldwake-ai`, or `worldwake-cli`). Because `worldwake-core` cannot depend on higher crates, the referenced enum must either be relocated to core or surfaced through a core-side `*Tag` mirror with a single conversion site.

**Precedent**: `BeliefStatus` lives in `crates/worldwake-sim/src/belief_view.rs:40` (sim crate). Its core-side mirror `BeliefStatusTag` is defined at `crates/worldwake-core/src/decision_event_payload.rs:231` with `Copy, Clone, Debug, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize` derives. The conversion `BeliefStatus → BeliefStatusTag` is a single match table at `crates/worldwake-sim/src/save_load.rs:1368-1372`. Other core-resident payloads (e.g., `BeliefSnapshot.status` at `decision_event_payload.rs:225`) reference the `Tag` form, not the sim form.

**Verify the spec addresses**:

1. **Mirror placement**: the proposed `*Tag` enum is defined in `worldwake-core` alongside other historical-record mirrors (typically in `decision_event_payload.rs` or a sibling module), not in the same crate as the source enum.
2. **Mechanical equivalence**: the mirror's variants are 1:1 with the source enum's variants — same names, same arity. The mirror is not allowed to introduce semantic differences (no narrowing, no merging, no renaming) — it is a serialization shim, not a domain abstraction.
3. **Derive requirements**: the mirror derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` (matching `BeliefStatusTag`). If the source enum carries non-`Copy` payload, the spec must explain how the mirror handles it (typically by mirroring only the discriminant and citing the source for the payload).
4. **Single conversion site**: the `Source → Tag` conversion lives in exactly one file (typically `crates/worldwake-sim/src/save_load.rs` parallel to the existing `BeliefStatus` table). The reverse direction (`Tag → Source`) is provided only if a higher-crate consumer needs to lift the historical record back into the live enum — most reassessments never need this, and the spec should not propose it speculatively.
5. **No double-truth**: the spec must not propose using both `Source` and `Tag` forms in the same authoritative state. The mirror is the historical/serialized form; the source enum is the live form. Per FND-28, two live authoritative representations of the same fact may not coexist.

**Flag as Issue**: spec proposes a core-resident type whose field types resolve to a non-core enum without naming the mirror; spec proposes a mirror with semantic differences from the source (variant rename, merge, narrowing); spec proposes the mirror in a non-core crate; spec proposes a `Tag → Source` conversion without naming a consumer that requires it.

**Recommendation framing**: when this pattern triggers and the spec hasn't acknowledged it, the recommendation should cite the `BeliefStatusTag` precedent and propose a parallel mirror (e.g., `RankedGoalComparisonDimensionTag`, `GoalKindTag`) following its derives and conversion-site convention. Do not recommend relocating the source enum to core unless the source enum is also referenced by core-resident *non-historical* state — that is a much larger blast-radius change.

## Existing Variant Payload Widening

**Trigger**: Spec proposes widening an existing enum variant's payload — e.g., turning a unit variant `Foo::Bar` into a payload-bearing `Foo::Bar(NewType)`, or adding a field to an existing tuple/struct variant. The variant already exists; the change is to its shape, not its presence.

**Verify the spec addresses**:

1. Exhaustive match sites that destructure the variant — grep across all crates for the variant by name (e.g., `Foo::Bar`); every site that previously matched a unit variant now needs the new pattern (e.g., `Foo::Bar(payload)` or `Foo::Bar { .. }`). Tests that asserted the bare variant (e.g., `assert_eq!(result, Foo::Bar)`) need updating to the new pattern.
2. Sibling unit variants in the same enum — if other variants in the enum carry analogous information that the new payload represents (e.g., `CriticalFailure` gains the failed assumption while `RecoverableFailure(SuspensionReason)` already carries its reason), consider whether uniform widening across siblings improves ergonomic consistency. Flag as Improvement if mixed.
3. Derive compatibility — `Copy`, `Clone`, `Eq`, `Hash`, `Serialize`, `Deserialize` derives on the enum may break if the new payload type does not satisfy them. Flag as CRITICAL Issue if a non-`Copy` payload is added to a `Copy`-deriving enum.
4. Downstream payload consumers — for each match site updated in (1), trace whether the new payload data is actually consumed (e.g., emitted in traces, recorded in events, used for branching). If the spec adds a payload but no consumer reads it, flag as Issue — payload widening for trace surfacing only is legitimate but should be named explicitly.

## Dual-Use Read-Model Types

**Trigger**: Spec proposes types, extractors, or report models that must be consumable from both `crates/*/tests/` and any non-test crate (e.g., the observer binary in `worldwake-cli`, a diagnostic CLI, or a future replay tool). Common signals: "shared observer code", "golden support or observer rendering", forensic/report/snapshot/diagnostic surfaces.

**Rule**: Dual-use types MUST live in `src/` of the owning crate (typically `worldwake-ai/src/` or `worldwake-sim/src/`). Test modules under `crates/X/tests/` are not importable by sibling crates — placing dual-use types there forces later refactor when the observer, replay tool, or downstream consumer is added.

**Verify the spec addresses**:

1. Report/model types committed to `crates/<owner>/src/<module>.rs` with `pub` visibility and `lib.rs` re-export.
2. Test-facing wrappers (assertion helpers, ignored-reproducer scaffolding) may remain under `crates/<owner>/tests/golden_harness/` composing over the runtime types — runtime types are authoritative, test wrappers are thin.
3. Crate attribution in the spec's Crates section names the runtime module path, not `tests/support`.

**Analog patterns already in repo**: `DecisionTraceSink` (`crates/worldwake-ai/src/decision_trace.rs`), `ActionTraceSink` (`crates/worldwake-sim/src/action_trace.rs`). Both are consumed by goldens AND the observer binary through runtime placement.

**Flag as Issue**: Spec text that leaves placement ambiguous ("test/support or shared observer code") or picks `tests/` when observer reuse is desired. Recommend committing to runtime placement as part of the Issue finding.

## Fabricated Migration Before-Signatures

**Trigger**: Spec contains Before/After code blocks (or prose framing like "currently:" / "today the trait returns…" / "migrates from X to Y") that present deliverables as migrations of existing methods, signatures, or return types.

**Rule**: Every "Before" signature the spec claims as existing MUST be verified by direct grep. Fabricated migrations are a distinct failure mode from "renamed/moved signature": the methods do not exist *anywhere*, and the spec's entire D-section framing is wrong. This is the scenario where the spec was drafted against an imagined API surface rather than against current code.

**Verify the spec addresses**:

1. For each "Before" method name cited in migration framing, grep the workspace (`rg "fn <name>\|<name>\("` across `crates/`). Zero matches outside the spec file itself = fabricated migration.
2. When fabrication is confirmed, check whether equivalent functionality is served under a *different* name — search for the semantic capability (e.g., grep for `last_known_place` if the spec talks about target location) and cite the actual existing surface in the finding.
3. If no equivalent exists, the deliverables are net-new additions, not migrations — the Design Goals, Summary, and D-section framing all need rewrites.

**Analog failure modes already covered**: Code example fidelity (3.3) catches structural mismatches in code snippets; Pseudocode dependency completeness (3.3) catches missing symbols in proposed pseudocode. Fabricated-migration fills the gap between them: it names an *explicit* failure mode where the spec's "before" side is entirely fictional, and it prescribes the grep-every-Before-signature check as the early-warning check.

**Flag as CRITICAL Issue**: The spec's migration framing is false. Recommend reframing affected D-sections as net-new additions, deleting any `*_crisp` / `*_old` / shim discussion (nothing to shim), and rewriting Summary/Design Goals to describe the gap filled by the new methods rather than the migration of non-existent ones. Also reframe any consumer-migration D-section ("emitters currently check X.is_some()…") as net-new consumer integration, because consumers cannot be migrating code that never existed.

## Proposed Visibility Qualifier Audit

**Trigger**: Spec proposes a new type, function, or field with an explicit Rust visibility qualifier (`pub`, `pub(crate)`, `pub(super)`, `pub(in path)`) and the surrounding prose claims a specific reachability ("only from within X", "not reachable outside Y", "external crates cannot construct Z").

**Verify the spec addresses**:

1. The proposed qualifier's effective scope, given the host module's actual placement in the crate tree, matches the prose claim. `pub(super)` in a crate-root module is visible to the entire crate — functionally equivalent to `pub(crate)` for most intents. To restrict construction to the defining module only, the correct qualifier is no qualifier (file-private) or `pub(self)`.
2. If the prose claims "only reachable from X" and the qualifier permits a broader scope, flag as a HIGH Issue. Recommend either tightening the qualifier or rewriting the prose to match the delivered scope.
3. If the spec proposes a constructor that must be invoked from multiple crates (the D3-style "call sites construct the type at each site" pattern), the prose "only reachable from X" is often incompatible with the required call-site pattern. Recommend restructuring the API so the authoritative producer returns the type directly, making external construction unnecessary.

## Read-Only Tooling Consumer

**Trigger**: Spec proposes a tool that consumes public APIs from multiple Worldwake crates (`core`, `sim`, `systems`, `ai`, `cli`) without writing to any system. Common signals: debug visualizers, observer-style binaries, replay viewers, event-log explorers, diagnostic CLIs. Whether the tool lives in an existing crate as a binary or in a brand-new tooling crate is incidental — the trigger is the *read-only consumer* shape.

**Verify the spec addresses each surface it consumes**. Validate signatures and visibility for every named accessor; cite `crates/worldwake-cli/src/bin/observer.rs` as the reference implementation:

1. **Topology read accessors** (`crates/worldwake-core/src/topology.rs`): `Topology::place_ids`, `place(id)`, `outgoing_edges(place)`, `incoming_edges(place)`, `edge(id)`, `neighbors(place)`, `shortest_path(from, to)`. `TravelEdge` exposes `from()`, `to()`, `travel_time_ticks()`, `id()`. The visualizer/observer must reach these via `world.topology()` — there is no `txn.edges_out` or equivalent shortcut on `WorldTxn`.
2. **Agent enumeration**: `World::entities_with_name_and_agent_data() -> impl Iterator<Item = EntityId>` (`crates/worldwake-core/src/world.rs`). There is no `entities_by_kind(EntityKind::Agent)` accessor — use this enumerator.
3. **Active-action read**: `Scheduler::active_actions() -> &BTreeMap<ActionInstanceId, ActionInstance>` (`crates/worldwake-sim/src/scheduler.rs`); filter by `instance.actor`. `ActionInstance.local_state: Option<ActionState>` carries the `Travel { edge_id, origin, destination, departure_tick, arrival_tick }` variant. There is no `txn.active_action_of(agent)`.
4. **Agent decision runtime read**: `AgentTickDriver::runtime(agent: EntityId) -> Option<&AgentDecisionRuntime>` (`crates/worldwake-ai/src/agent_tick/mod.rs`). `AgendaState`, `current_plan: Option<PlannedPlan>`, and other per-agent runtime state live on `AgentDecisionRuntime`, not on the world transaction. The tool must own the persistent `AgentTickDriver` to retain this state across ticks. `AutonomousControllerRuntime` is a per-tick borrow holder constructed fresh — never store it as a struct field.
5. **Trace sink installation**: Sinks (`ActionTraceSink`, `DecisionTraceSink`, `PerceptionTraceSink`, `RequestResolutionTraceSink`, `PoliticalTraceSink`, `InstitutionalKnowledgeTraceSink`) are owned by the tool and borrowed into `TickStepServices` per tick. `TickStepServices` itself is lifetime-bound (`Option<&'a mut ActionTraceSink>`); like `AutonomousControllerRuntime`, it cannot be stored as a struct field.
6. **Scenario loader pair**: `worldwake_cli::scenario::load_scenario_file(path) -> Result<ScenarioDef, ScenarioError>` and `spawn_scenario(def) / spawn_scenario_ignoring_lints(def) -> Result<SpawnedSimulation, ScenarioError>` (`crates/worldwake-cli/src/scenario/mod.rs`). The `--ignore-lints` flag selects between the pair (not a parameter on either function). `SpawnedSimulation` returns `{ state: SimulationState, action_registries: ActionRegistries, dispatch_table: SystemDispatchTable }`; the tool unpacks these as separate persistent fields.
7. **Per-tick step pattern**: Each tick, call `sim.tick_parts_mut() -> (&mut World, &mut EventLog, &mut Scheduler, &mut ControllerState, &mut DeterministicRng, &RecipeRegistry)` (`crates/worldwake-sim/src/simulation_state.rs`); construct `AutonomousControllerRuntime::new(vec![&mut self.driver])` and `TickStepServices { … }` locally; then call `step_tick(...)`. This mirrors `observer.rs:3702-3719` exactly.
8. **Common name accessors**: `World::get_component_name(id) -> Option<&Name>` (macro-generated at `crates/worldwake-core/src/component_schema.rs`), or the higher-level helper `worldwake_cli::display::entity_display_name(world, id) -> String`. There is no `get_component_display_name` accessor.
9. **Location accessor**: `World::effective_place(entity) -> Option<EntityId>` (`crates/worldwake-core/src/world/placement.rs`), inherited by `WorldTxn` via `Deref`. There is no `txn.location_of(agent)` accessor.

**Flag as Issue**: Specs in this class that name nonexistent shortcut accessors (e.g., `entities_by_kind`, `edges_out`, `location_of`, `active_action_of`, `get_component_display_name`, `active_goal_of`), embed lifetime-bound types (`TickStepServices`, `AutonomousControllerRuntime`) as plain struct fields, or reach for `AgendaState` through a transient borrow rather than through the persistent `AgentTickDriver`.

## Multi-Substrate Hook Coverage

**Trigger**: Spec adds a hook on a grant transition, event emission, perception write, or learning point that has parallel substrates in worldwake. Common parallel-substrate pairs:

- `ContentionQueue` (facility-level, `crates/worldwake-systems/src/facility_queue.rs::promote_ready_head`) ↔ `ResourceExtractionQueues` (per-slot on resource sources, `crates/worldwake-systems/src/production_actions.rs::grant_or_signal_full`). Only `ContentionQueue` grants emit `EventTag::QueueGrantPromoted`; resource-extraction grants do not.
- Same-tick co-located observation (FND-14A direct read of authoritative state) ↔ memory-backed belief lookup (perception writes to `AgentBeliefStore`, AI reads from belief view).
- Goal emission in `crates/worldwake-ai/src/candidate_generation.rs` ↔ goal ranking in `crates/worldwake-ai/src/ranking.rs` (gates vs. ordering — see also "Candidate Scoring Architecture" pattern).
- Action precondition checks at `start_*` ↔ `validate_*` ↔ payload revalidation in `plan_revalidation.rs` (the Authoritative-to-AI Impact Rule from CLAUDE.md spans these).

**Verify the spec addresses**:

1. Enumerate the parallel substrates by grep — for each substrate the spec's domain spans, identify the actual handler/site (file + function + line range).
2. Map the spec's motivating examples or scenario classes to their substrates: which entity types and components does each example use, and which substrate carries that state? Specs commonly cite a scenario whose substrate differs from the named hook (e.g., orchard contention runs through `ResourceExtractionQueues`, not the named `ContentionQueue` site).
3. Require the spec to either (a) hook each relevant substrate explicitly with its own deliverable or sub-deliverable, or (b) explicitly Non-Goal the substrates it does not cover, with a written rationale. Silent omission is the failure mode.
4. For the hook ordering: when a substrate's grant/promotion mutates state, identify which fields the hook needs to read BEFORE the mutation (e.g., `queued_at` from a waiter that's about to be removed) and call out the read-before-mutate ordering in the deliverable.

**Flag as HIGH Issue**: Spec hooks one substrate while its motivating scenario or stated coverage spans others. The named hook will never fire for the unhooked scenarios — the spec is internally inconsistent even though every named symbol resolves.

## Discrepancy as Failure-Attribution Surface

**Trigger**: Spec proposes a new failure mode that an action handler, revalidation path, or planner step can encounter and attribute (typed cause for "this didn't work because…").

**Three options for surfacing the failure**:

1. **As a `Discrepancy` enum variant** (`crates/worldwake-core/src/discrepancy.rs:8`) — first-class typed surface. Constraints: (a) `Discrepancy` derives `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`, so any payload type the new variant carries must derive `Copy` and the rest of the bounds; (b) workspace-wide exhaustive-match audit (~145 `Discrepancy` use sites at the time of writing — most are construction `Err(Discrepancy::X)` sites in `effect_sink_hypothetical.rs`, `needs_actions.rs`, `search_actions.rs`; the genuinely-exhaustive `match d { ... }` sites are the subset requiring new arms); (c) `Ord` ordering decision for the new variant.
2. **As a trace-only annotation** on `RootCandidateTrace` (`crates/worldwake-ai/src/decision_trace.rs`), `ActionTraceSink`, `DecisionTraceSink`, or another decision sink — surfaces the cause for debug/observer inspection without extending the typed failure taxonomy. No exhaustive-match cost. Use when the failure does not need to alter handler control flow or replan logic.
3. **Reuse an existing `Discrepancy` variant** — if `MissingObservation`, `BeliefStale`, `BeliefContradicted`, `SourceInvalidated`, etc. already cover the semantic case, the spec should not introduce a new variant. The new failure mode can still surface specifics through trace annotation alongside the reused variant.

**Verify the spec addresses**:

1. Which of (1)/(2)/(3) the spec is choosing — explicitly. A spec that mentions `Discrepancy` in prose without naming the choice has a missing design decision.
2. For option (1): a deliverable section enumerates the variant addition, payload type and its `Copy` derive, and the exhaustive-match audit scope.
3. For option (2): a deliverable section names the trace surface and the field/variant added there.
4. For option (3): the spec acknowledges the reuse and (where applicable) names the existing variant.

**Flag as Issue**: Spec writes failure attribution into prose without committing to one of (1)/(2)/(3), or proposes a new `Discrepancy` variant without enumerating the deliverable per "Pattern Triggers Map to Deliverables, Not Prose Only" (top of file).
