# T01DEBVIS-008: Beliefs + Plan tabs

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: [T01DEBVIS-007](T01DEBVIS-007.md)

## Problem

Spec T01 §D7 sub-tabs Beliefs and Plan are the deepest inspection surfaces in the visualizer. Beliefs tab reads four distinct components (`AgentBeliefStore` plus the separate `LastSeenMemory`, `ExpectationStore`, `SourceReliability` components) and renders each as its own collapsible section — *not* as nested fields of a single store. Plan tab renders the active `IntentionFrame`, the `PlannedPlan` from `AgentTickDriver`, each step's `PlanGuard`/`PlanExpectation` (delivered by archived spec S114), and the last `ReplanReason` derived from the scoped decision-trace ring buffer.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `AgentBeliefStore` at `crates/worldwake-core/src/belief.rs:45` exposes top-level fields `entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>`, `known_entities`, `social_observations`, `told_beliefs`, `heard_beliefs`, `asked_witnesses`, `place_visits: BTreeMap<EntityId, PlaceVisitRecord>`, `institutional_beliefs`. Reassessment 2026-04-25 confirmed `LastSeenMemory` (`expectation.rs:136`), `ExpectationStore` (`expectation.rs:80`), and `SourceReliability` (`experience.rs:84`) are *separate components*, not sub-fields of the store.
2. `EntityBeliefClaim` at `crates/worldwake-core/src/entity_belief_claim.rs:47` carries `aspect: EntityBeliefAspect`, `confidence: Permille`, `acquired_tick: Tick`. Freshness is computed (`current_tick - acquired_tick`) — there is no direct `freshness` field.
3. `IntentionFrame` is a component on agents, accessible via `world.get_component_intention_frame(agent)` (`crates/worldwake-core/src/component_schema.rs:1777`).
4. `AgentTickDriver::runtime(agent) -> Option<&AgentDecisionRuntime>` at `crates/worldwake-ai/src/agent_tick/mod.rs:121`. `AgentDecisionRuntime.current_plan: Option<PlannedPlan>` at `crates/worldwake-ai/src/decision_runtime.rs:152`.
5. `PlannedStep.guard: Option<PlanGuard>` and `PlannedStep.expectations: Vec<PlanExpectation>` at `crates/worldwake-ai/src/planner_ops.rs:826-827`. Types defined at `crates/worldwake-ai/src/plan_guard.rs:8,53`. Archived dep at `archive/specs/S114-plan-step-guards.md`.
6. `ReplanReason` and `PlanInvalidationReason` live in `crates/worldwake-core/src/decision_event_payload.rs` (separate enums per /reassess-spec verification). The "last replan reason" surfaces from the visualizer's scoped decision-trace ring buffer (T01DEBVIS-009 owns the buffer; this ticket queries it).
7. Tooling-only ticket — all reads via public component accessors; no engine state mutation.

## Architecture Check

1. Beliefs tab declares four distinct read sources (one ECS component each) and renders each as its own collapsible section. This matches the corrected spec §D7.3 which untangled the previous misimpression that `LastSeenMemory`/`ExpectationStore`/`SourceReliability` lived inside `AgentBeliefStore`.
2. Plan tab reads `current_plan` through the persistent `AgentTickDriver` owned by `VisualizerApp` (FND-19 carve-out for debug tools surfaces ground-truth plan state). The path matches the corrected §D7.5 — there is no `runtime.active_goal_of` shortcut.
3. "Last replan reason" comes from the visualizer's own scoped decision-trace buffer (T01DEBVIS-009), not from the engine's authoritative event log — keeping the visualizer fully observer-shaped.

## Verification Layers

1. Beliefs tab read correctness → focused unit test loading a scenario, advancing until an agent has at least one `entity_claim`, asserting the rendered claim list matches the agent's actual `entity_claims.len()`.
2. Plan tab IntentionFrame read correctness → focused unit test asserting the rendered IntentionFrame matches `world.get_component_intention_frame(agent)`.
3. Plan tab PlanGuard/PlanExpectation visibility → focused unit test asserting that for an agent with a `current_plan` containing at least one `PlannedStep` with `guard.is_some()`, the guard's `required_facts.len()` is rendered.
4. Per template item 6: action-trace and event-log layers are not relevant — both tabs are read-only views.

## What to Change

### 1. Beliefs tab — `tabs/beliefs.rs`

Create `crates/worldwake-visualizer/src/tabs/beliefs.rs`:

- Read `world.get_component_agent_belief_store(agent)`, `get_component_last_seen_memory(agent)`, `get_component_expectation_store(agent)`, `get_component_source_reliability(agent)` — each may be `None` (the section then renders an "absent" stub).
- Each component renders as a separate `egui::CollapsingHeader` section.
- Within `AgentBeliefStore`: sub-sections for `entity_claims`, `known_entities`, `social_observations`, `told_beliefs`, `heard_beliefs`, `asked_witnesses`, `place_visits`, `institutional_beliefs`.
- `entity_claims` rows: `aspect | confidence | acquired_tick | freshness (= current_tick - acquired_tick)`. Sort by freshness ascending (freshest first).
- `place_visits` rows: `place name | last_arrival_tick | visit_count | ticks_present`. Sort by `last_arrival_tick` descending.

### 2. Plan tab — `tabs/plan.rs`

Create `crates/worldwake-visualizer/src/tabs/plan.rs`:

- Read `world.get_component_intention_frame(agent)` — render the active intention domain or `"no active intention"`.
- Read `app.driver.runtime(agent).and_then(|r| r.current_plan.as_ref())` — render plan step list. Use `AgentDecisionRuntime.current_step_index` (or analogous pointer) to highlight the current step. If a current-step pointer is not exposed, derive it from the active `ActionInstance` matching the plan; defer to implementation phase to confirm the exact field name.
- For each `PlannedStep`: render `op_kind`, `target_place` (if any), `estimated_ticks`, `guard: Option<PlanGuard>` (collapsible — show `required_facts`, `min_confidence`, `invalidators`), `expectations: Vec<PlanExpectation>` (collapsible — show `kind`, `observe_by`).
- Last replan reason: query the scoped `decision_trace` ring buffer for the most recent entry whose outcome encodes a `ReplanReason` or `PlanInvalidationReason`. The buffer is owned and populated by T01DEBVIS-009; if T01DEBVIS-009 has not yet wired the buffer at integration time, render "no replan recorded" as a placeholder.

### 3. Wire tabs into router

Modify `crates/worldwake-visualizer/src/tabs/mod.rs` from T01DEBVIS-007 — replace the placeholder branches for `DetailTab::Beliefs` and `DetailTab::Plan` with actual dispatches.

## Files to Touch

- `crates/worldwake-visualizer/src/tabs/beliefs.rs` (new)
- `crates/worldwake-visualizer/src/tabs/plan.rs` (new)
- `crates/worldwake-visualizer/src/tabs/mod.rs` (modify — register Beliefs and Plan tabs)

## Out of Scope

- Traces tab (T01DEBVIS-009 — it depends on the ring buffer wiring landed in -009).
- Belief-state diffing across ticks.
- Plan-execution playback.
- Cross-agent belief comparison.

## Acceptance Criteria

### Tests That Must Pass

1. `beliefs_tab_renders_each_source_section` — each of the four source components has its own collapsible header in the rendered output.
2. `beliefs_tab_entity_claims_render_aspect_and_confidence` — for an agent with at least one `EntityBeliefClaim`, the rendered row shows `aspect` and `confidence` values matching the underlying claim.
3. `plan_tab_renders_active_intention_when_present` — for an agent with an `IntentionFrame` component, the tab renders the intention domain; for an agent without one, the tab renders the `"no active intention"` placeholder.
4. `plan_tab_step_guards_visible` — for an agent with a `current_plan` containing a `PlannedStep` with `guard.is_some()`, the guard's `required_facts.len()` is rendered.
5. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. Each belief surface is rendered from its own component accessor — no surface is read through a wrong store.
2. Plan-step guards/expectations come from `PlannedStep` fields, not reconstructed from event-log scans.
3. The visualizer never writes to belief or plan state.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/tabs/beliefs.rs` and `plan.rs` (`#[cfg(test)] mod tests`) — four focused tests above, using a baseline scenario advanced enough ticks for beliefs/plan state to populate.

### Commands

1. `cargo test -p worldwake-visualizer tabs::beliefs::`
2. `cargo test -p worldwake-visualizer tabs::plan::`
3. `cargo test -p worldwake-visualizer`
4. `cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron` (manual click + tab smoke)
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Added `crates/worldwake-visualizer/src/tabs/beliefs.rs`, rendering `AgentBeliefStore`, `LastSeenMemory`, `ExpectationStore`, and `SourceReliability` from separate component accessors with absent-state stubs.
- Added `AgentBeliefStore` subsections for the live top-level fields, including entity-claim rows sorted by computed freshness and place-visit rows sorted by last arrival.
- Added `crates/worldwake-visualizer/src/tabs/plan.rs`, rendering `IntentionFrame`, `AgentDecisionRuntime.current_plan`, `AgentDecisionRuntime.current_step_index`, per-step `PlanGuard`, and per-step `PlanExpectation` fields from the live runtime shapes.
- Wired `DetailTab::Beliefs` and `DetailTab::Plan` in `crates/worldwake-visualizer/src/tabs/mod.rs`; `DetailTab::Traces` remains the T01DEBVIS-009 placeholder.

## Deviations

- The Plan tab's last-replan section intentionally renders `"no replan recorded"` because T01DEBVIS-009 is still pending and owns the scoped decision-trace ring buffer. This matches the ticket's integration-time fallback rather than inventing a parallel buffer.
- Focused tests assert helper-level render inputs and live read boundaries rather than scraping egui output text. The rendered UI consumes the same helper rows and component/runtime accessors.
- The manual GUI click smoke was not run in this headless session; automated tab and crate tests were run instead.

## Verification Result

- Passed `cargo test -p worldwake-visualizer --lib -- --list`.
- Passed `cargo test -p worldwake-visualizer --lib tabs::beliefs::tests::beliefs_tab_renders_each_source_section -- --exact`.
- Passed `cargo test -p worldwake-visualizer --lib tabs::beliefs::tests::beliefs_tab_entity_claims_render_aspect_and_confidence -- --exact`.
- Passed `cargo test -p worldwake-visualizer --lib tabs::plan::tests::plan_tab_renders_active_intention_when_present -- --exact`.
- Passed `cargo test -p worldwake-visualizer --lib tabs::plan::tests::plan_tab_step_guards_visible -- --exact`.
- Passed `cargo test -p worldwake-visualizer tabs::beliefs::`.
- Passed `cargo test -p worldwake-visualizer tabs::plan::`.
- Passed `cargo test -p worldwake-visualizer`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`; live gates are `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
