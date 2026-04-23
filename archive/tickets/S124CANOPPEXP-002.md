# S124CANOPPEXP-002: Normalize AI-layer expectation-failure incidents and evolve writer

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — AI-layer detection sites, single shared writer signature, source-reliability attribution rules
**Deps**: `specs/S124-canonical-opportunity-expectation-failure.md`, `archive/tickets/S124CANOPPEXP-001.md`

## Problem

Three AI-layer detection sites currently feed a single AI-layer writer, but they exchange only `BTreeSet<SourceKey>`:

- [`pending_local_source_reliability_failures(...)`](../../crates/worldwake-ai/src/agent_tick/observation.rs) at `observation.rs:329` (retained-plan local depletion)
- [`emit_expectation_violation_candidates(...)`](../../crates/worldwake-ai/src/candidate_generation.rs) at `candidate_generation.rs:4172` (the `SupplyDepleted` violation at `candidate_generation.rs:4237-4250`)
- [`same_goal_search_failed_source_keys(...)`](../../crates/worldwake-ai/src/agent_tick/planning.rs) at `planning.rs:341` (same-goal sibling success)

All three feed [`apply_source_reliability_failure_observations(...)`](../../crates/worldwake-ai/src/agent_tick/mod.rs) at `agent_tick/mod.rs:1904` (the single AI-layer writer), called from `agent_tick/mod.rs:1045`, `planning.rs:1608`, and `planning.rs:1966`. With the primitive `SourceKey`-set shape, the writer cannot distinguish observation-stage absence from same-goal-sibling success, attribution collapses to "was a `SourceKey` present?", decision traces can't surface the canonical contradiction, and the reconsideration path cannot distinguish "source invalidated" from "goal invalidated."

This ticket introduces the normalized `OpportunityExpectationFailureIncident` runtime type (plus phase and cause enums), rewrites the three detection sites to emit `Vec<OpportunityExpectationFailureIncident>` instead of `BTreeSet<SourceKey>`, and evolves the writer to consume incidents and apply the five attribution rules from the spec.

## Assumption Reassessment (2026-04-23)

1. Detection sites confirmed at:
   - [`crates/worldwake-ai/src/agent_tick/observation.rs:329`](../../crates/worldwake-ai/src/agent_tick/observation.rs) — `pending_local_source_reliability_failures(view, agent, current_plan)` reads `plan.committed_source` (line 346) and returns `BTreeSet::from([source_key])` when the observed quantity at the committed source is zero.
   - [`crates/worldwake-ai/src/candidate_generation.rs:4172`](../../crates/worldwake-ai/src/candidate_generation.rs) — `emit_expectation_violation_candidates(candidates, diagnostics, ctx)` returns `(Vec<PendingViolationRecord>, BTreeSet<SourceKey>)`; the `SupplyDepleted` case at lines 4237-4250 inserts `SourceKey { entity: *entity_id, commodity: resource_source.commodity }`.
   - [`crates/worldwake-ai/src/agent_tick/planning.rs:341`](../../crates/worldwake-ai/src/agent_tick/planning.rs) — `same_goal_search_failed_source_keys(current_plan, selected_plan, selection_plans, current_place) -> BTreeSet<SourceKey>`, called at `planning.rs:1600` and `planning.rs:1958`.
2. Writer confirmed at [`crates/worldwake-ai/src/agent_tick/mod.rs:1904`](../../crates/worldwake-ai/src/agent_tick/mod.rs) — `apply_source_reliability_failure_observations(world, event_log, agent, tick, failed_sources: &BTreeSet<SourceKey>)`. It enters `SourceReliability` via `world.get_component_source_reliability(agent)`, inserts or updates each `SourceKey` entry, increments `failed_attempts` (line 1932), calls `enforce_limits(tick, &profile)` (line 1935), and commits via `txn.set_component_source_reliability(agent, reliability)` (line 1946). Callers: `agent_tick/mod.rs:1045`, `planning.rs:1608`, `planning.rs:1966`.
3. Shared abstraction boundary under audit: the detection→writer exchange shape for AI-layer source-expectation contradictions. After this ticket, the canonical shape is `Vec<OpportunityExpectationFailureIncident>`; no detection site may retain a `BTreeSet<SourceKey>`-flavored output, and no writer may accept `BTreeSet<SourceKey>`. Systems-layer helpers at `crates/worldwake-systems/src/experience_recording.rs` (`record_failed_source_attempt`, `record_successful_source_acquisition`, called from `production_actions.rs:611, 732` and `trade_actions.rs:394, 421`) remain a separate lawful path under FND-26 and are explicitly out of scope.
4. Existing regression coverage: `pending_source_reliability_failure_reorders_candidates_before_persistence` at [`ranking.rs:5267`](../../crates/worldwake-ai/src/ranking.rs) (focused unit), `refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection` at [`agent_tick/tests.rs:6180`](../../crates/worldwake-ai/src/agent_tick/tests.rs) (focused runtime), and `survival_preferences_keeps_proactive_diversification_alive_under_survival` at [`tests/golden_survival_preferences.rs`](../../crates/worldwake-ai/tests/golden_survival_preferences.rs) (golden E2E). All three must continue to pass after the incident-shape migration.
5. Heuristic/filter discipline (precision rule 12): this ticket does not remove any existing heuristic. The `SupplyDepleted` detection at `candidate_generation.rs:4237-4250` is preserved as-is; only its output shape changes from `SourceKey` insertion to incident emission. The attribution rule in D4 that distinguishes observation-proven local absence from unrelated precondition mismatch is the codification of existing behavior, not a new heuristic.
6. Information-path refactor (precision rule 16): the same fact ("the source was trusted and reality contradicted that trust") currently travels through one AI-layer writer already; after this ticket the canonical payload is richer (`OpportunityExpectationFailureIncident`) but still one path. The primitive `BTreeSet<SourceKey>` exchange is removed in-scope. The systems-layer authoritative-action write path via `experience_recording.rs` is a DIFFERENT fact (authoritative action outcome) and remains unchanged — not an alias to be removed.
7. Mismatch + correction: the current writer increments `failed_attempts` for every `SourceKey` in the set. Attribution rule #5 from D4 requires coalescing duplicate incidents per `(opportunity, source, phase, tick)` before persistence. Because the old input was a set keyed only by `SourceKey`, the pre-coalescing step is new; the canonical implementation is to dedupe the incident slice before the mutation loop.

## Architecture Check

1. Evolving the existing single writer in place is cleaner than introducing a parallel helper: it preserves the architectural property that there is exactly one AI-layer path writing source-reliability aftermath, upholds FND-28 by avoiding a shim, and keeps the three existing caller sites routing to the same function.
2. No backwards-compatibility shim is introduced. The writer's old signature (`&BTreeSet<SourceKey>`) is removed; every caller migrates to building `Vec<OpportunityExpectationFailureIncident>` in the same ticket.
3. Systems-layer writers in `experience_recording.rs` are explicitly preserved as a separate lawful path under FND-26. This ticket does not touch them, and the evolved AI-layer writer does not call them — both paths mutate the same `SourceReliability` component through non-overlapping triggering events (AI-layer on belief/search contradictions; systems-layer on authoritative action outcomes).
4. Runtime-only residency: the new incident types live in `worldwake-ai` because they are transient reasoning artifacts consumed within a single agent tick, never persisted. `worldwake-core` retains all durable building blocks (`SourceKey`, `OpportunityKey`, `OpportunityAnchor`, `Tick`, `EntityId`) unchanged.

## Verification Layers

1. Canonical detection→writer exchange shape (incident carries `opportunity`, `source`, `expectation_kind`, `phase`, `cause`, `detected_at_tick`) -> focused unit coverage per detection site, asserting emission for representative scenarios.
2. Writer attribution rules (concrete source required, sibling success credits source not goal kind, observation-proven absence vs. precondition mismatch, coalescing of duplicates) -> focused unit coverage on the evolved `apply_source_reliability_failure_observations` exercising each rule.
3. Source-reliability mutation ordering and `SourceReliability` component state after a multi-incident batch -> authoritative world state assertion in focused runtime test (read `world.get_component_source_reliability(agent)` after the writer commits).
4. Retained-plan observation path continues to produce the correct concrete `SourceKey` via the new incident shape -> existing `refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection` at `agent_tick/tests.rs:6180` must pass with assertions updated from set-shaped output to incident-shaped output.
5. Ranking discount path on next tick continues to discount the failed source -> existing `pending_source_reliability_failure_reorders_candidates_before_persistence` at `ranking.rs:5267` must pass; its fixture constructs pending-failure input and the migration from `BTreeSet<SourceKey>` to `Vec<OpportunityExpectationFailureIncident>` must preserve behavioral equivalence.
6. Survival-scenario E2E still proves same-goal diversification under source failure -> existing `survival_preferences_keeps_proactive_diversification_alive_under_survival` at `tests/golden_survival_preferences.rs`. This stays as a guardrail, not a primary proof; the primary proofs are at the focused runtime and decision-trace layers.

## What to Change

### 1. Define the incident runtime types

Add a new module `crates/worldwake-ai/src/opportunity_expectation_failure.rs` (or colocate in `planner_ops.rs`, the existing home for `OpportunityExpectationKind` per ticket 001). Define:

```rust
pub struct OpportunityExpectationFailureIncident {
    pub opportunity: worldwake_core::OpportunityKey,
    pub source: worldwake_core::SourceKey,
    pub expectation_kind: OpportunityExpectationKind,
    pub detected_at_tick: worldwake_core::Tick,
    pub phase: ExpectationFailurePhase,
    pub cause: ExpectationFailureCause,
}

pub enum ExpectationFailurePhase {
    Observation,
    CandidateGeneration,
    Search,
}

pub enum ExpectationFailureCause {
    SourceAbsentLocally,
    SourceDepletedLocally,
    SameGoalSearchInfeasibleWhileSiblingSucceeded,
}
```

Per the spec's D2 note, `source` is `SourceKey` (not `Option<SourceKey>`): incidents are only constructed when the detection site has a concrete source identity. Place-only opportunities without a resolved source never produce incidents. Per D4 rule 1, this type shape trivially guarantees attribution rule 1.

Derives: `Clone`, `Debug`, `Eq`, `PartialEq`. `Serialize`/`Deserialize` are NOT required — these types are runtime-only. `Hash` + `Ord` on the enums and incident are helpful for the coalescing step (rule 5).

Re-export from `crates/worldwake-ai/src/lib.rs`.

### 2. Rewrite `pending_local_source_reliability_failures`

Change the signature at [`observation.rs:329`](../../crates/worldwake-ai/src/agent_tick/observation.rs) from `BTreeSet<SourceKey>` to `Vec<OpportunityExpectationFailureIncident>`. Extend the function's inputs so it can read `plan.opportunity`, `plan.committed_source`, `plan.expectation_kind`, and the current tick:

```rust
fn pending_local_source_reliability_failures(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    current_plan: Option<&crate::PlannedPlan>,
    tick: Tick,
) -> Vec<OpportunityExpectationFailureIncident>
```

When the locally observed quantity is zero at the committed source, emit one incident with `phase = Observation`, `cause = SourceDepletedLocally` (the plan believed supply existed; reality is zero). A separate `SourceAbsentLocally` branch only fires when the belief store reports the source as absent altogether; the current detection site proves depletion, so the default cause is `SourceDepletedLocally`. (If the planner has reason to emit both, split branches; otherwise stick to one cause.)

Update the `candidates.pending_source_reliability_failures` struct field type (currently `BTreeSet<SourceKey>` at [`observation.rs:95`](../../crates/worldwake-ai/src/agent_tick/observation.rs) and related literal constructions at lines 1003, 1048, 1092) to `Vec<OpportunityExpectationFailureIncident>`. Rename the field if helpful (e.g., `pending_source_expectation_failures`) but not required.

Update the ranking helper consumer [`ranking.rs:442`](../../crates/worldwake-ai/src/ranking.rs) `apply_pending_source_reliability_failures` to accept the new shape. If the ranking helper only needs the `SourceKey` set for discount computation, extract it locally via `incidents.iter().map(|i| i.source).collect::<BTreeSet<_>>()` — do not store the reduced set on the read-result struct.

### 3. Rewrite `emit_expectation_violation_candidates`

Change the return type at [`candidate_generation.rs:4172`](../../crates/worldwake-ai/src/candidate_generation.rs) from `(Vec<PendingViolationRecord>, BTreeSet<SourceKey>)` to `(Vec<PendingViolationRecord>, Vec<OpportunityExpectationFailureIncident>)`. At the `SupplyDepleted` insertion (lines 4237-4250), replace `pending_source_reliability_failures.insert(SourceKey { entity, commodity })` with construction of an incident tagged `phase = CandidateGeneration`, `cause = SourceDepletedLocally`.

The incident requires `opportunity` and `expectation_kind`. These are not currently available at the `SupplyDepleted` site — the loop iterates over `believed_state` entries, not committed plans. Two options:

- **Option A**: Thread the agent's current committed plan through `GenerationContext` so the `SupplyDepleted` site can correlate the detected depletion against the current plan's `committed_source` and `expectation_kind`. If the detected `SourceKey` matches `plan.committed_source`, emit the incident with `plan.opportunity` and `plan.expectation_kind`. If no match, skip emission — the detection at that site was not the committed expectation.
- **Option B**: Emit a synthetic `opportunity`/`expectation_kind` derived from `believed_state.commodity` + detected `SourceKey` + goal kind inferred from `AcquireCommodity`/`RestockCommodity` priorities.

Option A preserves the spec's semantics ("this committed opportunity failed") and is preferred. The implementation must thread `runtime.current_plan` through `GenerationContext` (grep `GenerationContext` in `candidate_generation.rs` to find the construction site and add a field).

Update the caller at [`candidate_generation.rs:343`](../../crates/worldwake-ai/src/candidate_generation.rs) to propagate the new return shape.

### 4. Rewrite `same_goal_search_failed_source_keys`

Rename and change the return type at [`planning.rs:341`](../../crates/worldwake-ai/src/agent_tick/planning.rs) from `BTreeSet<SourceKey>` to `Vec<OpportunityExpectationFailureIncident>`. Accept `tick` as an input. Emit one incident per detected failed source with `phase = Search`, `cause = SameGoalSearchInfeasibleWhileSiblingSucceeded`. Read `current_plan.opportunity`, `current_plan.committed_source`, `current_plan.expectation_kind` from the passed-in `PlannedPlan`. Update the two call sites at `planning.rs:1600` and `planning.rs:1958` to thread `tick` and consume the new return type.

### 5. Evolve `apply_source_reliability_failure_observations`

Change the signature at [`agent_tick/mod.rs:1904`](../../crates/worldwake-ai/src/agent_tick/mod.rs) from `failed_sources: &BTreeSet<SourceKey>` to `incidents: &[OpportunityExpectationFailureIncident]`. Update the three call sites at `agent_tick/mod.rs:1045`, `planning.rs:1608`, `planning.rs:1966` to pass the new shape.

Inside the function, apply the five attribution rules from the spec's D4:

1. (Type-enforced) Every incident carries a concrete `SourceKey`; no guard needed.
2. For `cause = SameGoalSearchInfeasibleWhileSiblingSucceeded`, the reliability update is charged to the incident's concrete `source`, not to the goal kind. The existing loop already keys by `SourceKey`, so this rule holds by construction — just ensure no later code path uses `incident.expectation_kind` to infer goal-kind invalidation.
3. For `phase = Observation` or `CandidateGeneration` with `cause = SourceAbsentLocally` or `SourceDepletedLocally`, update source reliability. (The current code already mutates for every source; after this change, only these cases reach here because incident construction is gated at the detection site.)
4. `SupplyDepleted` from `candidate_generation.rs:4237-4250` maps to `phase = CandidateGeneration`, `cause = SourceDepletedLocally`, which is lawfully treated as a source-reliability update per rule 3. This preserves current behavior, codified at the type level. The spec's M1 improvement has now been encoded as an incident phase/cause pairing rather than a prose clarification.
5. Coalesce duplicate incidents before persistence: before the mutation loop, collect unique `(opportunity, source, phase, tick)` tuples and count the unique `source` set; mutate `SourceReliability` once per distinct source. Use `BTreeSet<SourceKey>` for deterministic iteration.

Preserve the existing `enforce_limits(tick, &profile)` call and the `txn.set_component_source_reliability(agent, reliability)` commit at line 1946.

Return a summary structure (`Vec<(SourceKey, ExpectationFailureCause)>` or similar) so ticket `S124CANOPPEXP-003` (SourceInvalidated reconsideration) can consume the attribution outcome. This is the forward-link surface. If the return type is not clear at this ticket's implementation time, use `()` and document the downstream requirement via an Out of Scope reference to ticket 003.

### 6. Update existing tests

Migrate assertions in existing focused tests to the new incident shape:

- `agent_tick/tests.rs:6180` `refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection` — expected output changes from `BTreeSet<SourceKey>` to `Vec<OpportunityExpectationFailureIncident>`; assert incident `phase = Observation`, `cause = SourceDepletedLocally`, and that `incident.source == SourceKey { entity, commodity }` and `incident.expectation_kind == Some(AcquireCommodityFromConcreteSource)` (or whatever kind the fixture's plan commits).
- `ranking.rs:5267` `pending_source_reliability_failure_reorders_candidates_before_persistence` — if the fixture currently constructs `BTreeSet<SourceKey>` input, rewrite to construct `Vec<OpportunityExpectationFailureIncident>` with the minimum fields the discount helper needs.
- Any other test that reads `pending_source_reliability_failures` as a `BTreeSet<SourceKey>` — migrate to incident shape.

## Files to Touch

- `crates/worldwake-ai/src/opportunity_expectation_failure.rs` (new — or colocate in `planner_ops.rs`)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export new types)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — detection site + struct field shape)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — detection site + return type)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — detection site + call sites threading tick)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — writer signature + attribution rules + coalescing)
- `crates/worldwake-ai/src/ranking.rs` (modify — `apply_pending_source_reliability_failures` consumer signature)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — migrate assertions)

## Out of Scope

- `OpportunityExpectationKind` enum definition and `PlannedPlan.expectation_kind` field — delivered by ticket `S124CANOPPEXP-001`.
- Systems-layer writers at `crates/worldwake-systems/src/experience_recording.rs` — they remain a separate lawful path for authoritative action outcomes (spec Non-Goal).
- Reconsideration policy extensions (`Discrepancy::SourceInvalidated` plus committed-plan invalidation hook) — delivered by ticket `S124CANOPPEXP-003`.
- Decision-trace surface via `DecisionEventPayload` — delivered by ticket `S124CANOPPEXP-004`.
- Authoritative-start-rejected or authoritative-outcome-contradicted causes — intentionally excluded from the incident enum per spec D2.
- Unifying the AI-layer attribution function with systems-layer writers — explicit Non-Goal.
- Generalization to non-acquisition goal kinds — future work.

## Acceptance Criteria

### Tests That Must Pass

1. A new focused unit test in `agent_tick/observation.rs` (tests module) asserts `pending_local_source_reliability_failures(...)` emits one `OpportunityExpectationFailureIncident` with `phase = Observation`, `cause = SourceDepletedLocally`, the committed `SourceKey`, and the plan's `expectation_kind` when the locally observed quantity at the committed source is zero.
2. A new focused unit test in `candidate_generation.rs` (tests module) asserts `emit_expectation_violation_candidates(...)` emits an incident with `phase = CandidateGeneration`, `cause = SourceDepletedLocally` for the `SupplyDepleted` path when the detected source matches the committed plan's `committed_source`. A sibling test asserts no incident is emitted when the detected source does not match the committed plan.
3. A new focused unit test in `agent_tick/planning.rs` (tests module) asserts the renamed same-goal search function emits one incident per failed source with `phase = Search`, `cause = SameGoalSearchInfeasibleWhileSiblingSucceeded` and the current plan's opportunity/expectation kind.
4. A new focused unit test in `agent_tick/tests.rs` (or a new sibling) exercises `apply_source_reliability_failure_observations` with a multi-incident batch that includes duplicates and asserts (a) `SourceReliability.failed_attempts` increments once per distinct `SourceKey` (coalescing rule 5), (b) `last_attempt_tick` is set to the incident tick, (c) `enforce_limits` was invoked.
5. Existing regression (migrated assertion shape): `cargo test -p worldwake-ai --lib agent_tick::tests::refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection -- --exact`
6. Existing regression (migrated fixture shape): `cargo test -p worldwake-ai --lib ranking::tests::pending_source_reliability_failure_reorders_candidates_before_persistence -- --exact`
7. Existing regression: `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Zero occurrences of `BTreeSet<SourceKey>` as a function return type, struct field type, or parameter type for AI-layer source-reliability pending-failure exchange after this ticket. (Grep: `BTreeSet<SourceKey>` in `crates/worldwake-ai/src/` should return zero production matches related to pending-failure plumbing; test fixtures may construct transient sets but not for cross-module exchange.)
2. There is exactly one AI-layer function (`apply_source_reliability_failure_observations`) that mutates `SourceReliability` from expectation contradiction. No parallel helper is introduced.
3. Every `OpportunityExpectationFailureIncident` constructed at a detection site carries a concrete `SourceKey` (type-enforced by `source: SourceKey` — not `Option<_>`).
4. Systems-layer writers at `crates/worldwake-systems/src/experience_recording.rs` remain unchanged by this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/observation.rs` (tests module) — focused coverage for the rewritten `pending_local_source_reliability_failures`.
2. `crates/worldwake-ai/src/candidate_generation.rs` (tests module) — focused coverage for the `SupplyDepleted` incident emission + non-matching-source skip.
3. `crates/worldwake-ai/src/agent_tick/planning.rs` (tests module) — focused coverage for the renamed same-goal search function.
4. `crates/worldwake-ai/src/agent_tick/tests.rs` — new test for the evolved writer exercising multi-incident coalescing + rule 3 attribution.
5. `crates/worldwake-ai/src/ranking.rs` (tests module) — migrate `pending_source_reliability_failure_reorders_candidates_before_persistence` fixture to the incident shape.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::observation::tests -- --exact`
2. `cargo test -p worldwake-ai --lib candidate_generation::tests -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::planning::tests -- --exact`
4. `cargo test -p worldwake-ai --lib agent_tick::tests -- --exact`
5. `cargo test -p worldwake-ai --lib ranking::tests::pending_source_reliability_failure_reorders_candidates_before_persistence -- --exact`
6. `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
7. `cargo test -p worldwake-ai`
8. `scripts/verify.sh`

## Outcome

Completed on 2026-04-23.

- Added the runtime-only `OpportunityExpectationFailureIncident` carrier plus `ExpectationFailurePhase` and `ExpectationFailureCause`, re-exported from `worldwake-ai`.
- Migrated the three AI-layer detection seams from `BTreeSet<SourceKey>` exchange to incident vectors:
  observation read-phase local contradiction,
  candidate-generation `SupplyDepleted` when it matches the committed plan,
  and same-goal sibling-search failure.
- Evolved `apply_source_reliability_failure_observations(...)` into the single AI-layer incident writer, with deterministic per-incident coalescing, explicit phase/cause filtering, and a forward summary keyed by concrete `SourceKey`.
- Updated pending ranking discount plumbing and focused regression fixtures to consume the incident shape without keeping a cross-module `BTreeSet<SourceKey>` exchange boundary.
- `crates/worldwake-ai/tests/golden_survival_preferences.rs` remained unchanged; the golden guardrail passed against the landed runtime path.

## Deviations

- The truthful existing read-phase regression in `agent_tick/tests.rs` proves the retained-plan observation seam only; it does not seed the committed-plan belief state needed for the candidate-generation incident. The migrated assertion was narrowed to the honest observation-only incident emitted on that harness.
- The final forward summary from `apply_source_reliability_failure_observations(...)` landed as `BTreeMap<SourceKey, BTreeSet<ExpectationFailureCause>>`, which satisfies ticket 003's need for source-attribution aftermath without introducing an extra public summary type at this ticket boundary.
- `./scripts/verify.sh` was not run in this implementation-only pass.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-ai --lib agent_tick::observation::tests::pending_local_source_reliability_failures_emits_observation_incident -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::violation_supply_depleted_emits_matching_committed_plan_incident -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::violation_supply_depleted_skips_incident_when_source_does_not_match_committed_plan -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::same_goal_search_failure_incidents_emit_search_incident_for_committed_source -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::apply_source_reliability_failure_observations_coalesces_duplicates_and_enforces_limits -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::pending_source_reliability_failure_reorders_candidates_before_persistence -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai`
