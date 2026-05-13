# S143STABELVIE-003: Migrate authority methods to `BelievedAuthorityView`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — belief-view trait surface migration; `AgentBeliefStore` read paths re-routed for `believed_owner_of` and `believed_office_holder`; `RuntimeBeliefView` supertrait extended.
**Deps**: archive/tickets/S143STABELVIE-001.md, archive/tickets/S143STABELVIE-002.md, archive/tickets/S143STABELVIE-003A.md

## Problem

Before this ticket, S143's compile-error guarantee ("planner module attempting to read `believed_owner_of` from a trait it did not import would not link") required moving `believed_owner_of` and `believed_office_holder` from their legacy homes (`ControlBeliefView`, `PoliticalBeliefView`) and from the planner's narrow AI-facing surfaces (`GoalBeliefView`, `GoalControlBeliefView`). Without that move, the planner could continue reading these accessors from `GoalBeliefView`, bypassing the `BeliefRead<T>` wall. The migration had to land atomically because removing a method from `ControlBeliefView` breaks every `impl ControlBeliefView for X` site until all consumers are migrated.

## Assumption Reassessment (2026-05-13)

1. `believed_owner_of` is declared on 3 traits: `ControlBeliefView::believed_owner_of` (`belief_view.rs:780`), `GoalControlBeliefView::believed_owner_of` (`belief_view.rs:250`), and `GoalBeliefView::believed_owner_of` (`belief_view.rs:437`). Consumer files (per workspace grep, all current as of 2026-05-13): `worldwake-systems/src/tell_actions.rs`, `investigate_actions.rs`; `worldwake-sim/src/trade_valuation.rs`, `per_agent_belief_view.rs`, `commodity_opportunity.rs`, `belief_view.rs`, `affordance_query.rs`; `worldwake-ai/src/search/tests.rs`, `search/strategic.rs`, `ranking.rs`. Per-trait blanket impls at `belief_view.rs:1463` (`impl<T: ControlBeliefView + ?Sized> GoalControlBeliefView for T`) and `belief_view.rs:1477` (`impl<T> GoalBeliefView for T`) forward `believed_owner_of` from `ControlBeliefView`; both forwarders need updating.
2. `believed_office_holder` is declared on 2 traits: `PoliticalBeliefView::believed_office_holder` (`belief_view.rs:1303`, returns `InstitutionalBeliefRead<Option<EntityId>>`) and `GoalBeliefView::believed_office_holder` (`belief_view.rs:691`, similar). Consumers (current grep): `worldwake-systems/src/office_actions.rs`; `worldwake-sim/src/per_agent_belief_view.rs`, `institutional_knowledge_trace.rs`, `belief_view.rs`; `worldwake-core/src/belief.rs`; `worldwake-cli/src/scenario/mod.rs`; `worldwake-ai/tests/golden_survival_ask_consult.rs`, `golden_offices.rs`; `worldwake-ai/src/search/tests.rs`, `search/strategic.rs`. The stable authority surface maps certain institutional holder knowledge to `BeliefRead<Option<EntityId>>`, preserving both `Some(holder)` and known vacancy `None`; `Unknown` and `Conflicted(Vec<...>)` collapse to `BeliefRead::Unknown`. Callers needing conflict/provenance continue to read underlying institutional claims through `AgentBeliefStore`-backed paths.
3. The 4 BelievedAuthorityView methods that are net-new (`believed_holder_of`, `believed_access_right`, `believed_jurisdiction`) wire to the truthful substrate available here. `PerAgentBeliefView::believed_holder_of` reads holder-belief entries from `AgentBeliefStore::entity_claims` (when present; otherwise `BeliefRead::Unknown`). `believed_access_right` and `believed_jurisdiction` return `BeliefRead::Unknown` until those authority belief carriers are modeled.
4. Adjacent contradiction (was item 13): per Step 2's 1-3-1 (a) approval, this ticket extends the spec's D3 audit table scope to also migrate `GoalBeliefView::believed_owner_of`, `GoalControlBeliefView::believed_owner_of`, and `GoalBeliefView::believed_office_holder`. Classification: required consequence of the spec's compile-error guarantee reaching `GoalBeliefView` (the planner's primary read surface; consumed by 10+ `worldwake-ai/src/` files: `exhaustion.rs`, `effect_sink_hypothetical.rs`, `feasibility.rs`, `lib.rs`, `enterprise.rs`, `theft.rs`, `planning_state.rs`, `agenda_manager.rs`, `pressure.rs`, `route_threat.rs`).
5. Mismatch + correction (was item 14): spec D2 method-origin table claims `believed_owner_of` is migrated "from `ControlBeliefView::believed_owner_of`". Correction: the migration source set is `{ControlBeliefView, GoalControlBeliefView, GoalBeliefView}` — three trait declarations, not one. Documented here and reflected in What to Change.
6. FOUNDATIONS reassessment (2026-05-13): live `AgentBeliefStore::entity_claims` did not contain owner or holder/custody claim aspects, so implementing real `BelievedAuthorityView` reads directly in this ticket would either derive social facts from authoritative world state (violating FND-14A) or add hidden substrate under a trait-migration ticket. The now-archived `archive/tickets/S143STABELVIE-003A.md` owns the prerequisite explicit owner/holder belief claim lanes and save-version bump. This ticket resumes after that substrate exists.

## Architecture Check

1. FND-28-clean: no temporary alias paths or shims. The method's old declarations are removed from all source traits; `BelievedAuthorityView::believed_owner_of` is the single authoritative form.
2. The ~15 `TestBeliefView` mock impls of `RuntimeBeliefView` (across `worldwake-ai/src/**` and `worldwake-ai/tests/**`) gain default impls via the new trait's `BeliefRead::Unknown` defaults — only an empty `impl BelievedAuthorityView for TestBeliefView {}` block is needed at each site. No method-by-method override required.
3. Adding `BelievedAuthorityView` as a supertrait of `RuntimeBeliefView` cascades through every existing impl. Default impls absorb the cascade with zero method overrides required at non-canonical sites.
4. The 1-3-1 (a) extension preserves `GoalBeliefView`'s role as the planner's flat narrow read surface — `believed_owner_of` and `believed_office_holder` are removed from it (not re-declared with the new return type), forcing planner code to import `BelievedAuthorityView` directly. This is a deliberate API-surface tightening, not a regression in ergonomics.

## Verification Layers

1. Compile-time surface: `believed_owner_of` is reachable only via `BelievedAuthorityView` (or `RuntimeBeliefView` via supertrait + `use BelievedAuthorityView`). Verified by workspace test/clippy compilation and ticket 006's `compile_fail` doctest.
2. Return-type contract: owner/holder reads expose `BeliefRead<EntityId>` and office-holder reads expose `BeliefRead<Option<EntityId>>`. Verified by focused tests in this ticket.
3. Institutional holder conversion semantics preserve known vacancy and collapse unknown/conflicted reads to `BeliefRead::Unknown`. Focused tests cover the stable authority surface; planner paths needing conflict detail continue through full institutional claims.
4. FND-14A wall: `BelievedAuthorityView` impl on `PerAgentBeliefView` reads only from `AgentBeliefStore`, never authoritative world state. Verified by code review at this ticket's review pass; ticket 006's golden encodes the regression.
5. Existing golden coverage continues to pass — `golden_offices.rs`, `golden_survival_ask_consult.rs`, all `worldwake-ai` tests.

## Landed Changes

### 1. Trait declaration changes in `crates/worldwake-sim/src/belief_view.rs`

- Remove `fn believed_owner_of(...)` from `ControlBeliefView` (line 780).
- Remove `fn believed_owner_of(...)` from `GoalControlBeliefView` (line 250).
- Remove `fn believed_owner_of(...)` from `GoalBeliefView` (line 437).
- Remove `fn believed_office_holder(...)` from `PoliticalBeliefView` (line 1303).
- Remove `fn believed_office_holder(...)` from `GoalBeliefView` (line 691).
- Add `BelievedAuthorityView` to `RuntimeBeliefView`'s supertrait list, alongside existing supertraits.
- Update the `GoalControlBeliefView` blanket impl to remove the `believed_owner_of` forwarding stanza.
- Update the `GoalBeliefView` blanket impl to remove the `believed_owner_of` and `believed_office_holder` forwarding stanzas.

### 2. Canonical impl updates in `crates/worldwake-sim/src/per_agent_belief_view.rs`

- Remove `believed_owner_of` impl from `impl ControlBeliefView for PerAgentBeliefView`.
- Remove `believed_office_holder` impl from `impl PoliticalBeliefView for PerAgentBeliefView`.
- Update `impl BelievedAuthorityView for PerAgentBeliefView` (added in ticket 002 with `Unknown` defaults) to provide real reads:
  - `believed_owner_of(entity)` — reads from `AgentBeliefStore::entity_claims` for the entity's owner aspect; converts present belief to `BeliefRead::Known(value)`, stale belief to `BeliefRead::Stale(value)`, and missing/ambiguous belief to `BeliefRead::Unknown`.
  - `believed_office_holder(office)` — read from `AgentBeliefStore::institutional_beliefs` for the office's holder claim; convert certain institutional reads to `BeliefRead<Option<EntityId>>` per Assumption Reassessment 2.
  - `believed_holder_of(entity)` — read holder-belief entries from `entity_claims` (or `BeliefRead::Unknown` if absent).
  - `believed_access_right(actor, target)` — return `BeliefRead::Unknown` (access-right belief is not yet modeled).
  - `believed_jurisdiction(place)` — return `BeliefRead::Unknown` until a jurisdiction belief carrier is modeled.

### 3. Consumer call-site migration

For each consumer file, add `use worldwake_sim::BelievedAuthorityView;` and update owner/holder call sites to unwrap `BeliefRead<EntityId>` (typical pattern: `match view.believed_owner_of(entity) { BeliefRead::Known(v) | BeliefRead::Stale(v) => Some(v.value), BeliefRead::Unknown => None }`). Office-holder call sites unwrap `BeliefRead<Option<EntityId>>` where stable vacancy is meaningful, or use full institutional claims when conflict/provenance matters. Files (full list per Assumption Reassessment 1–2):

- `worldwake-sim/src/per_agent_belief_view.rs` (also re-routes internal callers)
- `worldwake-sim/src/commodity_opportunity.rs`
- `worldwake-sim/src/trade_valuation.rs`
- `worldwake-sim/src/affordance_query.rs`
- `worldwake-systems/src/tell_actions.rs`
- `worldwake-systems/src/investigate_actions.rs`
- `worldwake-systems/src/office_actions.rs`
- `worldwake-ai/src/search/strategic.rs`
- `worldwake-ai/src/search/tests.rs`
- `worldwake-ai/src/ranking.rs`

D6 import narrowing (distributed): in each consumer file, evaluate whether the file's reads are entirely covered by `BelievedAuthorityView` + 0–2 other sub-traits; if so, narrow the `RuntimeBeliefView` import. Otherwise keep `RuntimeBeliefView`. The hard goal (per spec D6) is "no belief-view import in `worldwake-ai` reaches `DebugWorldView`" — verified by ticket 005's lint.

### 4. Test-mock cascade

Add an empty `impl BelievedAuthorityView for TestBeliefView {}` (or the locally-named mock type) block at every site that currently `impl RuntimeBeliefView for <MockType> {}`. Per the agent's earlier inventory, ~15 sites across `worldwake-ai/src/**` and `worldwake-ai/tests/**`: `candidate_generation.rs`, `enterprise.rs`, `failure_handling.rs`, `feasibility_probe.rs`, `goal_explanation.rs`, `goal_model.rs`, `plan_revalidation.rs`, `planner_ops.rs`, `planning_snapshot.rs`, `planning_state.rs`, `pressure.rs`, `ranking.rs`, `agent_tick/tests.rs`, `search/strategic.rs`, `search/tests.rs`. The default impls (returning `BeliefRead::Unknown`) absorb the cascade — no method overrides needed at these sites unless the test specifically exercises authority belief.

## Landed Files

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait declarations, supertrait composition)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — impl moves + new method bodies)
- `crates/worldwake-sim/src/commodity_opportunity.rs` (modify)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify)
- `crates/worldwake-sim/src/affordance_query.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `crates/worldwake-ai/src/search/strategic.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/enterprise.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/feasibility.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/pressure.rs` (modify — test-double cascade)
- `crates/worldwake-ai/src/pursuit_belief.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` (modify — explicit owner-belief fixture)
- Test-mock cascade files listed above — add empty `impl BelievedAuthorityView for <MockType> {}` blocks where default unknown authority reads are sufficient.

Likely: additional consumer files may surface during implementation via reassessment grep (`grep -rn "believed_owner_of\b\|believed_office_holder\b" crates/`). The implementer should re-grep at start to confirm the consumer list is current.

## Out of Scope

- `locally_observed_entities_at` migration — ticket 004.
- `LocalPhysicalObservationView` as supertrait of `RuntimeBeliefView` — ticket 004.
- CI lint (D7) — ticket 005.
- Golden coverage (D8, including the belief-wall trap regression) — ticket 006.
- Narrowing of test mocks to specific sub-traits — out of scope per spec D6 ("Test-side mock impls (~15 files) continue to implement `RuntimeBeliefView` directly — narrowing test mocks is out of scope").
- Adding new belief-store fields for `believed_access_right` real reads — Non-Goals.

## Acceptance Result

### Tests Passed

1. Existing test: `golden_offices.rs` passes with updated `believed_office_holder` consumer pattern.
2. Existing test: `golden_survival_ask_consult.rs` passes.
3. Focused test coverage proves `BelievedAuthorityView::believed_owner_of` on `PerAgentBeliefView` returns `BeliefRead::Known(v)` when the agent's belief store has an owner claim and `BeliefRead::Unknown` when no claim exists.
4. Focused test coverage proves stable office-holder conversion preserves `Certain(Some)` and known vacancy while collapsing unknown/conflicted inputs to `BeliefRead::Unknown`.
5. Focused test coverage proves `believed_holder_of`, `believed_access_right`, and `believed_jurisdiction` return `BeliefRead::Unknown` for a fresh agent with no relevant beliefs.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. `believed_owner_of` and `believed_office_holder` are reachable only via `BelievedAuthorityView` (or `RuntimeBeliefView` via its authority supertrait plus explicit `use BelievedAuthorityView`).
2. No `ControlBeliefView`, `PoliticalBeliefView`, `GoalControlBeliefView`, or `GoalBeliefView` declaration contains these two methods after this ticket.
3. The conflicted office-holder case collapses to `BeliefRead::Unknown` for `BelievedAuthorityView::believed_office_holder` callers; callers needing conflict/provenance continue to read full institutional claims directly.
4. `BelievedAuthorityView` canonical impl on `PerAgentBeliefView` reads no authoritative world state — only `AgentBeliefStore`.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — `BelievedAuthorityView` impl tests.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — canonical impl behavior tests for migrated and authority methods.
3. `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` — explicit owner-belief fixture for the opportunity compiler's owned-bread regression.

### Commands Passed

1. `cargo test -p worldwake-sim believed_authority`
2. `cargo test -p worldwake-ai --test golden_offices`
3. `cargo test -p worldwake-ai --test golden_survival_ask_consult`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-13.

- Removed `believed_owner_of` and `believed_office_holder` from the legacy control, political, and goal-facing traits; `BelievedAuthorityView` is now the stable authority read surface.
- Added `BelievedAuthorityView` as a `RuntimeBeliefView` supertrait and migrated AI/sim/systems call sites and test doubles to import or implement the authority view explicitly.
- Wired `PerAgentBeliefView`, planning snapshots, planning state, opportunity compilation, ranking, office, tell, and investigation paths through explicit authority beliefs.
- Preserved known office vacancy on the stable surface as `BeliefRead<Option<EntityId>>`, while keeping contested/provenance-sensitive political candidate generation on the full institutional-claim path.
- Updated S143 spec wording and the prerequisite `S143STABELVIE-003A` ticket to match the FOUNDATIONS-driven authority belief substrate.

## Deviations

- The drafted spec expected `believed_office_holder` to collapse `Certain(None)` to `Unknown`. The landed design keeps known vacancy as concrete stable belief (`Known(None)`) because absence of an office holder is a concrete institutional state, not ignorance.
- `believed_jurisdiction` remains `Unknown` rather than reading an institutional shortcut; no explicit jurisdiction belief carrier exists yet, and FND-14A forbids deriving it indirectly.
- `scripts/verify.sh` was not run as a wrapper; its relevant gates were covered individually by `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`. `cargo fmt --all` was run, but `cargo fmt --all -- --check` was not rerun after final Markdown/spec closeout edits.

## Verification Result

- Passed `cargo test -p worldwake-sim believed_authority`
- Passed `cargo test -p worldwake-sim save`
- Passed `cargo test -p worldwake-ai --test golden_survival_ask_consult`
- Passed `cargo test -p worldwake-ai --test golden_offices`
- Passed `cargo test -p worldwake-ai --test golden_opportunity_compiler`
- Passed `cargo test -p worldwake-systems investigate_actions::tests::owner_investigating_missing`
- Passed `cargo test -p worldwake-ai --lib`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
