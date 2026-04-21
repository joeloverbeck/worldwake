# Worldwake Validation Patterns

Project-specific patterns for reassess-spec. When a spec proposes one of the triggers below, verify the corresponding integration points exist in the spec. Flag missing items as HIGH Issues.

## New GoalKind Variant

**Trigger**: Spec adds a variant to `GoalKind` in `crates/worldwake-core/src/goal.rs`.

**Verify the spec addresses**:

1. `GoalDispatchKey` — new variant + `ALL` constant + `from_goal_kind` match arm (`crates/worldwake-ai/src/goal_dispatch_key.rs`)
2. `GoalDispatchDeclaration` — entry with `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `progress_barrier_ops`, `family_policy` (`crates/worldwake-ai/src/goal_dispatch_decl.rs`)
3. `GoalKindPlannerExt` — implementation of all 11 methods (`crates/worldwake-ai/src/goal_model.rs`)
4. Ranking — `GoalPriorityClass` assignment and `motive_score` formula
5. Candidate generation — emitter function in `crates/worldwake-ai/src/candidate_generation.rs`
6. `GoalKind` derive compatibility — new variant fields must all be `Copy` (GoalKind derives Copy)

## New Component on EntityKind::Agent

**Trigger**: Spec adds a new ECS component registered on `EntityKind::Agent`.

**Verify the spec addresses**:

1. `component_schema.rs` — registration with insert/get accessors
2. `AgentDef` — field in `crates/worldwake-cli/src/scenario/types.rs`
3. `spawn_agent()` — set_component call in `crates/worldwake-cli/src/scenario/mod.rs`
4. Universal vs. role-specific classification (per `docs/spec-drafting-rules.md` section 5)
5. `Default` impl if universal
6. `*Def` wrapper type if component contains `EntityId` references

**Note**: Runtime-only components (not scenario-definable, always start at defaults) still require `component_schema.rs` registration and `create_agent()` insertion. The only exempt items are transient local variables that are never stored as ECS components. If the spec calls it a "component" and proposes `create_agent()` insertion, registration is mandatory.

## New Component Read by AI Crate

**Trigger**: Spec adds a component that candidate generation, ranking, or planning needs to read.

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
