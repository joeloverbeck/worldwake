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

## New Enum Variant on Cross-Crate Enum

**Trigger**: Spec extends an enum used across multiple crates.

**Verify**:

1. Exhaustive match sites — grep for `match` on the enum across all crates
2. Derive compatibility — new variant fields satisfy existing derives
3. `#[allow(clippy::large_enum_variant)]` — check if new variant is significantly larger than existing ones
