# S143STABELVIE-003: Migrate authority methods to `BelievedAuthorityView`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — belief-view trait surface migration; `AgentBeliefStore` read paths re-routed for `believed_owner_of` and `believed_office_holder`; `RuntimeBeliefView` supertrait extended.
**Deps**: archive/tickets/S143STABELVIE-001.md, S143STABELVIE-002

## Problem

S143's compile-error guarantee ("planner module attempting to read `believed_owner_of` from a trait it didn't import will not link") requires moving `believed_owner_of` and `believed_office_holder` from their current homes (`ControlBeliefView`, `PoliticalBeliefView`) AND from the planner's narrow AI-facing surfaces (`GoalBeliefView`, `GoalControlBeliefView`) — otherwise the planner could continue reading these accessors with `Option<EntityId>` / `InstitutionalBeliefRead<...>` shape from `GoalBeliefView`, bypassing the spec's `BeliefRead<T>` wall. The migration is workspace-wide and must land atomically — partial states do not compile because removing a method from `ControlBeliefView` breaks every `impl ControlBeliefView for X` site.

## Assumption Reassessment (2026-05-13)

1. `believed_owner_of` is declared on 3 traits: `ControlBeliefView::believed_owner_of` (`belief_view.rs:780`), `GoalControlBeliefView::believed_owner_of` (`belief_view.rs:250`), and `GoalBeliefView::believed_owner_of` (`belief_view.rs:437`). Consumer files (per workspace grep, all current as of 2026-05-13): `worldwake-systems/src/tell_actions.rs`, `investigate_actions.rs`; `worldwake-sim/src/trade_valuation.rs`, `per_agent_belief_view.rs`, `commodity_opportunity.rs`, `belief_view.rs`, `affordance_query.rs`; `worldwake-ai/src/search/tests.rs`, `search/strategic.rs`, `ranking.rs`. Per-trait blanket impls at `belief_view.rs:1463` (`impl<T: ControlBeliefView + ?Sized> GoalControlBeliefView for T`) and `belief_view.rs:1477` (`impl<T> GoalBeliefView for T`) forward `believed_owner_of` from `ControlBeliefView`; both forwarders need updating.
2. `believed_office_holder` is declared on 2 traits: `PoliticalBeliefView::believed_office_holder` (`belief_view.rs:1303`, returns `InstitutionalBeliefRead<Option<EntityId>>`) and `GoalBeliefView::believed_office_holder` (`belief_view.rs:691`, similar). Consumers (current grep): `worldwake-systems/src/office_actions.rs`; `worldwake-sim/src/per_agent_belief_view.rs`, `institutional_knowledge_trace.rs`, `belief_view.rs`; `worldwake-core/src/belief.rs`; `worldwake-cli/src/scenario/mod.rs`; `worldwake-ai/tests/golden_survival_ask_consult.rs`, `golden_offices.rs`; `worldwake-ai/src/search/tests.rs`, `search/strategic.rs`. The current return type `InstitutionalBeliefRead<Option<EntityId>>` is mapped to `BeliefRead<EntityId>` per spec D2: `Certain(Some(id)) → BeliefRead::Known(BeliefValue{value: id, …})`, `Certain(None) | Unknown → BeliefRead::Unknown`, `Conflicted(Vec<…>) → BeliefRead::Unknown` (callers needing the conflict surface continue to read via `PoliticalBeliefView` directly).
3. The 4 BelievedAuthorityView methods that are net-new (`believed_holder_of`, `believed_access_right`, `believed_jurisdiction`) wire to real belief-store reads here. `PerAgentBeliefView::believed_holder_of` reads holder-belief entries from `AgentBeliefStore::entity_claims` (when present; otherwise `BeliefRead::Unknown`). `believed_access_right` returns `BeliefRead::Unknown` until access-right belief is modeled (Non-Goals: no new belief-store fields). `believed_jurisdiction` reads jurisdiction belief from `institutional_beliefs` when present; otherwise `Unknown`.
4. Adjacent contradiction (was item 13): per Step 2's 1-3-1 (a) approval, this ticket extends the spec's D3 audit table scope to also migrate `GoalBeliefView::believed_owner_of`, `GoalControlBeliefView::believed_owner_of`, and `GoalBeliefView::believed_office_holder`. Classification: required consequence of the spec's compile-error guarantee reaching `GoalBeliefView` (the planner's primary read surface; consumed by 10+ `worldwake-ai/src/` files: `exhaustion.rs`, `effect_sink_hypothetical.rs`, `feasibility.rs`, `lib.rs`, `enterprise.rs`, `theft.rs`, `planning_state.rs`, `agenda_manager.rs`, `pressure.rs`, `route_threat.rs`).
5. Mismatch + correction (was item 14): spec D2 method-origin table claims `believed_owner_of` is migrated "from `ControlBeliefView::believed_owner_of`". Correction: the migration source set is `{ControlBeliefView, GoalControlBeliefView, GoalBeliefView}` — three trait declarations, not one. Documented here and reflected in What to Change.

## Architecture Check

1. FND-28-clean: no temporary alias paths or shims. The method's old declarations are removed from all source traits; `BelievedAuthorityView::believed_owner_of` is the single authoritative form.
2. The ~15 `TestBeliefView` mock impls of `RuntimeBeliefView` (across `worldwake-ai/src/**` and `worldwake-ai/tests/**`) gain default impls via the new trait's `BeliefRead::Unknown` defaults — only an empty `impl BelievedAuthorityView for TestBeliefView {}` block is needed at each site. No method-by-method override required.
3. Adding `BelievedAuthorityView` as a supertrait of `RuntimeBeliefView` cascades through every existing impl. Default impls absorb the cascade with zero method overrides required at non-canonical sites.
4. The 1-3-1 (a) extension preserves `GoalBeliefView`'s role as the planner's flat narrow read surface — `believed_owner_of` and `believed_office_holder` are removed from it (not re-declared with the new return type), forcing planner code to import `BelievedAuthorityView` directly. This is a deliberate API-surface tightening, not a regression in ergonomics.

## Verification Layers

1. Compile-time surface: `believed_owner_of` is reachable only via `BelievedAuthorityView` (or `RuntimeBeliefView` via supertrait + `use BelievedAuthorityView`). Verified by `cargo build --workspace` and ticket 006's `compile_fail` doctest.
2. Return-type contract: `BeliefRead<EntityId>` exposes `Unknown | Known(BeliefValue<EntityId>) | Stale(BeliefValue<EntityId>)`. Verified by focused tests in this ticket.
3. `InstitutionalBeliefRead → BeliefRead<EntityId>` conversion semantics (Certain, Unknown, Conflicted cases) — focused unit test in this ticket.
4. FND-14A wall: `BelievedAuthorityView` impl on `PerAgentBeliefView` reads only from `AgentBeliefStore`, never authoritative world state. Verified by code review at this ticket's review pass; ticket 006's golden encodes the regression.
5. Existing golden coverage continues to pass — `golden_offices.rs`, `golden_survival_ask_consult.rs`, all `worldwake-ai` tests.

## What to Change

### 1. Trait declaration changes in `crates/worldwake-sim/src/belief_view.rs`

- Remove `fn believed_owner_of(...)` from `ControlBeliefView` (line 780).
- Remove `fn believed_owner_of(...)` from `GoalControlBeliefView` (line 250).
- Remove `fn believed_owner_of(...)` from `GoalBeliefView` (line 437).
- Remove `fn believed_office_holder(...)` from `PoliticalBeliefView` (line 1303).
- Remove `fn believed_office_holder(...)` from `GoalBeliefView` (line 691).
- Add `BelievedAuthorityView` to `RuntimeBeliefView`'s supertrait list at line 1403 — insert `+ BelievedAuthorityView` immediately before the `{}` body, alongside existing supertraits.
- Update `GoalControlBeliefView` blanket impl (around line 1463) — remove the `believed_owner_of` forwarding stanza.
- Update `GoalBeliefView` blanket impl (around line 1477) — remove the `believed_owner_of` and `believed_office_holder` forwarding stanzas.

### 2. Canonical impl updates in `crates/worldwake-sim/src/per_agent_belief_view.rs`

- Remove `believed_owner_of` impl from `impl ControlBeliefView for PerAgentBeliefView`.
- Remove `believed_office_holder` impl from `impl PoliticalBeliefView for PerAgentBeliefView`.
- Update `impl BelievedAuthorityView for PerAgentBeliefView` (added in ticket 002 with `Unknown` defaults) to provide real reads:
  - `believed_owner_of(entity)` — read from `AgentBeliefStore::entity_claims` for the entity's `Ownership` aspect; convert the `BeliefValue<EntityId>` (when present and current) to `BeliefRead::Known(value)`; stale belief → `BeliefRead::Stale(value)`; no belief → `BeliefRead::Unknown`. Mirror the data path the removed `ControlBeliefView::believed_owner_of` impl used.
  - `believed_office_holder(office)` — read from `AgentBeliefStore::institutional_beliefs` for the office's holder claim; convert `InstitutionalBeliefRead<Option<EntityId>>` per Assumption Reassessment 2.
  - `believed_holder_of(entity)` — read holder-belief entries from `entity_claims` (or `BeliefRead::Unknown` if absent).
  - `believed_access_right(actor, target)` — return `BeliefRead::Unknown` (access-right belief is not yet modeled).
  - `believed_jurisdiction(place)` — read jurisdiction belief from `institutional_beliefs` for the place; `BeliefRead::Unknown` if absent.

### 3. Consumer call-site migration

For each consumer file, add `use worldwake_sim::BelievedAuthorityView;` and update call sites to unwrap `BeliefRead<EntityId>` (typical pattern: `match view.believed_owner_of(entity) { BeliefRead::Known(v) | BeliefRead::Stale(v) => Some(v.value), BeliefRead::Unknown => None }`). Files (full list per Assumption Reassessment 1–2):

- `worldwake-sim/src/per_agent_belief_view.rs` (also re-routes internal callers)
- `worldwake-sim/src/commodity_opportunity.rs`
- `worldwake-sim/src/trade_valuation.rs`
- `worldwake-sim/src/affordance_query.rs`
- `worldwake-sim/src/institutional_knowledge_trace.rs`
- `worldwake-systems/src/tell_actions.rs`
- `worldwake-systems/src/investigate_actions.rs`
- `worldwake-systems/src/office_actions.rs`
- `worldwake-ai/src/search/strategic.rs`
- `worldwake-ai/src/search/tests.rs`
- `worldwake-ai/src/ranking.rs`
- `worldwake-cli/src/scenario/mod.rs`
- `worldwake-core/src/belief.rs` (call-site at `believed_office_holder` consumer — likely a helper or impl block; reassessment-time grep will confirm shape)
- `worldwake-ai/tests/golden_offices.rs`
- `worldwake-ai/tests/golden_survival_ask_consult.rs`

D6 import narrowing (distributed): in each consumer file, evaluate whether the file's reads are entirely covered by `BelievedAuthorityView` + 0–2 other sub-traits; if so, narrow the `RuntimeBeliefView` import. Otherwise keep `RuntimeBeliefView`. The hard goal (per spec D6) is "no belief-view import in `worldwake-ai` reaches `DebugWorldView`" — verified by ticket 005's lint.

### 4. Test-mock cascade

Add an empty `impl BelievedAuthorityView for TestBeliefView {}` (or the locally-named mock type) block at every site that currently `impl RuntimeBeliefView for <MockType> {}`. Per the agent's earlier inventory, ~15 sites across `worldwake-ai/src/**` and `worldwake-ai/tests/**`: `candidate_generation.rs`, `enterprise.rs`, `failure_handling.rs`, `feasibility_probe.rs`, `goal_explanation.rs`, `goal_model.rs`, `plan_revalidation.rs`, `planner_ops.rs`, `planning_snapshot.rs`, `planning_state.rs`, `pressure.rs`, `ranking.rs`, `agent_tick/tests.rs`, `search/strategic.rs`, `search/tests.rs`. The default impls (returning `BeliefRead::Unknown`) absorb the cascade — no method overrides needed at these sites unless the test specifically exercises authority belief.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait declarations, supertrait composition)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — impl moves + new method bodies)
- `crates/worldwake-sim/src/commodity_opportunity.rs` (modify)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify)
- `crates/worldwake-sim/src/affordance_query.rs` (modify)
- `crates/worldwake-sim/src/institutional_knowledge_trace.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `crates/worldwake-ai/src/search/strategic.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify — `believed_office_holder` call-site)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_survival_ask_consult.rs` (modify)
- Test-mock cascade (~15 files, listed in What to Change §4) — add empty `impl BelievedAuthorityView for <MockType> {}` blocks

Likely: additional consumer files may surface during implementation via reassessment grep (`grep -rn "believed_owner_of\b\|believed_office_holder\b" crates/`). The implementer should re-grep at start to confirm the consumer list is current.

## Out of Scope

- `locally_observed_entities_at` migration — ticket 004.
- `LocalPhysicalObservationView` as supertrait of `RuntimeBeliefView` — ticket 004.
- CI lint (D7) — ticket 005.
- Golden coverage (D8, including the belief-wall trap regression) — ticket 006.
- Narrowing of test mocks to specific sub-traits — out of scope per spec D6 ("Test-side mock impls (~15 files) continue to implement `RuntimeBeliefView` directly — narrowing test mocks is out of scope").
- Adding new belief-store fields for `believed_access_right` real reads — Non-Goals.

## Acceptance Criteria

### Tests That Must Pass

1. Existing test: `golden_offices.rs` passes with updated `believed_office_holder` consumer pattern.
2. Existing test: `golden_survival_ask_consult.rs` passes.
3. New focused test: `BelievedAuthorityView::believed_owner_of` on `PerAgentBeliefView` returns `BeliefRead::Known(v)` when the agent's belief store has an owner claim; `BeliefRead::Unknown` when no claim exists.
4. New focused test: `InstitutionalBeliefRead → BeliefRead<EntityId>` conversion for `believed_office_holder` — covers `Certain(Some)`, `Certain(None)`, `Unknown`, `Conflicted(Vec)` input cases.
5. New focused test: `believed_holder_of`, `believed_access_right`, `believed_jurisdiction` return `BeliefRead::Unknown` for a fresh agent with no relevant beliefs.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. `believed_owner_of` and `believed_office_holder` are reachable only via `BelievedAuthorityView` (or `RuntimeBeliefView` via the new supertrait + explicit `use BelievedAuthorityView`).
2. No `ControlBeliefView`, `PoliticalBeliefView`, `GoalControlBeliefView`, or `GoalBeliefView` declaration contains these two methods after this ticket.
3. The `InstitutionalBeliefRead::Conflicted` case collapses to `BeliefRead::Unknown` for `BelievedAuthorityView::believed_office_holder` callers (per spec D2 method-origin table); callers needing the conflict surface continue to read via `PoliticalBeliefView` directly.
4. `BelievedAuthorityView` canonical impl on `PerAgentBeliefView` reads no authoritative world state — only `AgentBeliefStore`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — `BelievedAuthorityView` impl tests (~5 tests covering the 5 methods).
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — canonical impl behavior tests for migrated and net-new methods.
3. `crates/worldwake-ai/tests/golden_offices.rs` — updated consumer pattern (no new test, just call-site migration with the new `BeliefRead<EntityId>` unwrapping).
4. `crates/worldwake-ai/tests/golden_survival_ask_consult.rs` — same as above.

### Commands

1. `cargo test -p worldwake-sim believed_authority`
2. `cargo test -p worldwake-ai golden_offices`
3. `cargo test -p worldwake-ai golden_survival_ask_consult`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh`
