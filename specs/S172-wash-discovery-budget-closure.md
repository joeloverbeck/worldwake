# S172: Wash Discovery and Budget Closure

## Summary

Close the known `Wash` budget-exhaustion exclusion in `survival-scattered` and `survival-contested` so that Wash obeys the same lawful discovery, travel-search, planner-budget, and traceable-failure accounting as Eat, Drink, Sleep, and Relieve. Before S172 implementation began, `survival-contested` omitted Wash from `required_self_care_families`, and both `survival-scattered` and `survival-contested` omitted Wash from budget-exhaustion assertions because Wash could exhaust planner budget before the agent discovered a `WashBasin`. Ticket `archive/tickets/S172WASDISBUD-001.md` landed the contested contract/test side and fixed the planner active-goal/current-plan retention bug that surfaced there; ticket `archive/tickets/S172WASDISBUD-002.md` landed the scattered budget-check and `WashFacilityUsed` payload proof; ticket `archive/tickets/S172WASDISBUD-003.md` landed the scattered/contested belief-only Wash regression; ticket `archive/tickets/S172WASDISBUD-004.md` landed the player-POV remote WashBasin leak assertion. The simulation has the core Wash substrates (`GoalPlanningBudget::SELF_CARE`, `emit_wash_goal`, `WASH_OPS = [Wash, Travel]`, `MayContainWashBasin`, `WashBasinState`, and the drive-escalation/scattered/contested/CLI-POV belief-only Wash regressions), and this spec pins the lawful collision proof that Wash discovery and budget closure now matches the other four families under scattered, contested, belief-only, and player-POV topologies.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-ai` (candidate generation, goal schema, planner search, decision-trace emission)
- `worldwake-systems` (no behavior changes; the spec confirms `wash` action precondition surface)
- `worldwake-cli` (scenario contract amendments only)

## Dependencies

- `archive/tickets/S116DRIESCSUS-009.md` (archived ticket that recorded the exclusion as a known live planner-budget issue under tracking ID `GOAPTRVLSCAL-001`) — provides the explicit deferred-issue audit
- `archive/specs/S116-drive-escalation-sustained-critical.md` — related sustained-critical drive-escalation work that surfaced the Wash-pruning gap
- `archive/specs/S128-sleep-episode-place-quality.md` — proves the analogous Sleep-discovery/budget closure that Wash now mirrors
- `archive/specs/S129-place-dirtiness-facility-wear.md` — provides `WashBasinState` per-facility clean-water and dirtiness state
- `archive/specs/S158-belief-view-remote-truth-leak-closure.md` — provides the remote-facility belief-view gating Wash candidate enumeration must respect
- `archive/specs/S162-belief-view-source-gate-hardening.md` — provides the belief-backed read-gating for remote facility queue state

## Design Goals

- Wash discovery is belief-backed or same-tick co-located, never authoritative-read of remote `WashBasinState`.
- Wash travel-search consumes the same planner budget as Eat/Drink/Sleep/Relieve travel.
- Wash budget exhaustion is traceable as a distinct decision-event branch, indistinguishable in shape from the other self-care families.
- `survival-scattered` and `survival-contested` contracts include Wash in `required_self_care_families` and in budget-exhaustion assertions.
- Belief-only regression: a remote `WashBasin` invisible to the agent cannot produce a Wash candidate, regardless of authoritative truth.
- Player POV: the CLI cannot surface remote basin clean-water level or contention state unless the controlled agent has lawful local observation or a belief entry.

## Non-Goals

- No basin occupancy or facility-contention substrate. Wash-action mutual exclusion is deferred to S173.
- No new partial-progress state on Wash. Commit-time partial relief (already present when clean water is insufficient) is preserved; duration-based partial Wash is deferred.
- No new homeostatic need, profile field, or metabolism parameter. Existing `MetabolismProfile` controls suffice.
- No new affordance type, action handler, or component. The spec audits and pins the existing surfaces.
- No expansion of `MayContainWashBasin` exploration semantics beyond what the analogous `MayContainSleepSite` and `MayContainLatrine` hypotheses already do.
- No UI overhaul. The player-POV requirement is a single belief-source-class assertion on the existing CLI accessors.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-3 (Concrete state) | Wash uses the existing `HomeostaticNeeds::dirtiness`, `WashBasinState`, and `MetabolismProfile` — no abstract score is introduced |
| FND-7 (Locality) | Wash discovery is local or belief-backed; remote basin truth must travel via belief, witness, or report before becoming a candidate input |
| FND-8 (Action preconditions) | `wash_preconditions` already requires `TargetHasWashBasinClean { min: 1 }`; the spec preserves these and pins the failure-on-disconfirmed-belief branch |
| FND-14 (World ≠ Belief) | Remote `WashBasinState` is authoritative-read forbidden during candidate emission; only belief entries or FND-14A same-tick co-located reads qualify |
| FND-14A (Same-tick co-located observation) | Co-located basin physical state (clean water count, dirtiness) is observable; ownership/queue membership is not (belief-gated) |
| FND-14B (Planner-visible inputs) | The spec enumerates every input the Wash candidate path reads and classifies each as Self, Same-tick local, Belief-backed, or Public topology |
| FND-16 (Ignorance) | A Wash-needing agent without lawful basin knowledge produces no Wash candidate (no candidate ≠ false candidate) |
| FND-19 (Agent symmetry) | Human and AI agents both go through identical candidate enumeration and budget accounting |
| FND-26 (Systems interact through state) | Wash candidate emission reads belief and `WashBasinState`; no system commands the planner directly |
| FND-29A (Causal history is authoritative) | Budget-exhausted and belief-disconfirmed branches each emit distinct decision-event payloads that survive replay |
| FND-31 (Validation) | Scenarios A, B, and C below name negative cases (no Wash candidate without belief; no Wash plan when budget exhausts; no UI leak of remote basin state) |

## Deliverables

### 1. Wash candidate-enumeration source-class audit

For every input read by `emit_wash_goal` (and its support helpers in `crates/worldwake-ai/src/candidate_generation.rs`) the spec records the source class and the stale/unknown behavior. Implementation must match this table; any deviation requires updating the spec, not the audit.

| Input | Source class | Stale/unknown behavior |
|-------|--------------|------------------------|
| Actor `HomeostaticNeeds::dirtiness` | Self | If absent, no Wash candidate |
| Actor effective place | Self | If absent, no Wash candidate |
| Co-located `WashBasin` entity at actor place | Same-tick local physical observation (FND-14A) | If absent, fall through to belief-backed enumeration |
| Co-located `WashBasinState::clean_water` | Same-tick local physical observation | If `< min`, candidate emitted with disconfirmed-water failure branch (see Deliverable 5) |
| Co-located basin occupancy / current user identity | Belief-backed | If unknown, candidate may still emit; runtime contention resolves at action start (S173) |
| Remote `WashBasin` known via belief | Belief-backed | If no belief, no candidate; authoritative remote read forbidden |
| Remote `WashBasinState::clean_water` | Belief-backed (must be in belief entry payload) | If belief lacks payload, candidate emits with revalidation-required flag; runtime confirms at arrival |
| `MayContainWashBasin` hypothesis for unknown place | Belief-backed | If no hypothesis, no exploration candidate |
| Place-graph connectivity between actor and target | Public topology | Same as Eat/Drink/Sleep/Relieve travel |
| Per-agent `DriveThresholds` (`crates/worldwake-core/src/drives.rs:58`) | Self | If absent, default applied |
| `MetabolismProfile::wash_ticks` (Wash action duration, per agent) | Self | If absent, default applied |

**Implementation note on the dual-mode accessor.** Rows 3, 4 (co-located) and rows 6, 7 (remote) reflect the two branches of the same belief-view accessor. The canonical implementation lives at `crates/worldwake-sim/src/per_agent_belief_view.rs:824` (`fn wash_basin_state`): when `has_authoritative_local_visibility(basin)` is true the accessor reads authoritative `WashBasinState` from the world (FND-14A same-tick same-place observation); otherwise it falls back to `BelievedEntityState::wash_basin_state` stored from a prior visit. The code comment at line 835 explicitly cites FND-14A as the rationale. The source-class table classifies each row by the *legal* class for its case, not by the accessor's implementation surface — both branches of the dual-mode accessor are FOUNDATIONS-compliant under their respective classes.

### 2. Wash travel-search and budget-accounting parity

The spec pins three invariants over the existing search machinery in `crates/worldwake-ai/src/search/transition.rs` and `crates/worldwake-ai/src/planner_ops.rs`:

1. Wash travel arcs are charged against `GoalPlanningBudget::SELF_CARE` identically to Sleep, Relieve, Eat, and Drink travel arcs. No conditional discount, no parallel budget.
2. `PlannerOpKind::Wash` is `may_appear_mid_plan = true` and is not a materialization barrier (already true; spec pins this).
3. Belief-disconfirmed `WashBasin` candidates (basin known but `clean_water < min`) emit a candidate with the same shape as a disconfirmed grain-listing candidate today: planner may attempt it, runtime revalidation rejects at action start, decision trace records `Disconfirmed { basin, missing: CleanWater }`.

The implementation deliverable is an audit ticket against `search/transition.rs` confirming Wash terminal ordering matches the other self-care goals. The spec does not redesign the search; it pins the parity contract.

### 3. Scenario contract amendments

| Scenario | Change | Failure mode if regressed |
|----------|--------|---------------------------|
| `scenarios/survival-contested.ron` (line 20–33, `survival_health_contract` block) | `survival_health_contract.required_self_care_families` becomes `[Eat, Drink, Sleep, Relieve, Wash]` | Scenario contract violation if any agent does not exercise Wash |
| `scenarios/survival-contested.ron` (test) | Budget-exhaustion assertion includes Wash in the covered family set | Test failure if Wash exhausts budget without traceable recovery |
| `scenarios/survival-scattered.ron` (test; the `.ron` already includes Wash in `survival_health_contract.required_self_care_families` at line 19) | Budget-exhaustion assertion includes Wash in the covered family set | Test failure as above |
| `crates/worldwake-ai/tests/scenarios/survival_contested.rs` | Landed by `S172WASDISBUD-001`: Wash carve-out comment and exclusion removed; `wash_facility_payloads_record_every_agent` added | Compile/test failure if exclusion is reintroduced |
| `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` | Landed by `S172WASDISBUD-002`: Wash carve-out comment and exclusion removed; `wash_facility_payloads_record_every_agent` added | Compile/test failure if exclusion is reintroduced |

### 4. Belief-only Wash regression generalization

Landed by `archive/tickets/S172WASDISBUD-003.md`: the existing `survival_drive_escalation::build_belief_only_wash_harness` proves Wash is not synthesized for a remote unseen basin in a drive-escalation topology, and ignored golden Scenario 468 / Scenario 477 add the same candidate-emission proof shape to `survival-scattered` and `survival-contested`.

- Each landed sub-scenario preserves the authored remote `WashBasin`, clears the selected agent's belief store, seeds only local beliefs, and disables exploration pressure so the test remains at the no-remote-truth candidate boundary.
- Assertion: no `emit_wash_goal` candidate references the remote basin; no Wash plan is composed or selected; the agent's dirtiness remains unresolved instead of being corrected by remote truth.
- Assertion negative: any planner-visible candidate carrying the remote basin's `EntityId` fails the scenario.

### 5. Lawful Wash failure-attribution surfaces

**Failure-attribution strategy: option (3) Reuse.** Per the project's "Discrepancy as Failure-Attribution Surface" pattern, this spec introduces no new `Discrepancy`, `BlockingFact`, `PlanInvalidationReason`, or trace-event variants. Each Wash failure branch maps to an existing emission surface; the spec's job is to **pin** which existing surface fires for which Wash outcome and to commit to the **goal-key-join inspection convention** — assertions filter on `goal_key == GoalKind::Wash` AND the generic cause. This convention is uniform across all goals: Eat, Drink, Sleep, Relieve, and Wash all inspect through the same `(goal_key, generic_cause)` shape. Per FND-26 + FND-28, goal-specific cause-type widening in generic substrates and parallel typed enums are both rejected.

| Branch | Triggering condition | Existing emission substrate | Inspection key |
|--------|---------------------|---|---|
| Wash completed | Action commits | `DecisionEventPayload::WashFacilityUsed(WashFacilityUsedPayload { user, basin, water_consumed, agent_dirtiness_delta, basin_dirtiness_delta, partial })` — emitted at `crates/worldwake-systems/src/needs_actions.rs::commit_wash` (payload defined at `crates/worldwake-core/src/decision_event_payload.rs:79`) | `WashFacilityUsedPayload` carries `basin` and `user` directly |
| Wash budget exhausted | Planner expansion budget consumed before plan composed | `PlanSearchOutcome::BudgetExhausted { expansions_used }` at `crates/worldwake-ai/src/decision_trace.rs:1393`, nested inside a `SelectionTrace` that references the searched goal; also `Discrepancy::SearchBudgetExhausted` at `crates/worldwake-core/src/discrepancy.rs` for downstream consumers | `SelectionTrace.selected_opportunity` / per-goal-key search trace; filter on `goal_key == GoalKind::Wash` |
| Wash belief disconfirmed | Basin known to belief but runtime revalidation rejects (basin gone, dry, contended) | `RevalidationOutcome::Invalidated { reason: PlanInvalidationReason::ExpectationMismatch { step_index }, expectation_kind, mismatch_detail }` at `crates/worldwake-ai/src/plan_revalidation.rs:17`; paired with `ExpectationMismatchPayload { agent, goal_key: GoalKey, step_index }` at `crates/worldwake-core/src/decision_event_payload.rs:365` which carries the goal-key join field | `ExpectationMismatchPayload.goal_key`; `mismatch_detail` carries the concrete basin precondition that failed |
| Wash no candidate | No basin known and no lawful exploration hypothesis emitted | Absence in `crates/worldwake-ai/src/candidate_generation.rs::emit_wash_goal` (no candidate produced when `wash_access_opportunities` returns empty); observable through `CandidateGenerationDiagnostics` per-tick | Diagnostics keyed on candidate-type; assertion is "no Wash candidate emitted this tick despite drive escalation" |

The spec pins these mappings. No new payload variant is added. Implementation responsibility is limited to:

1. Confirming each substrate above already carries the `goal_key` (or equivalent goal identifier) needed for filtered assertion. Verification confirms `ExpectationMismatchPayload.goal_key` exists at `decision_event_payload.rs:365`; `SelectionTrace` and `RootCandidateTrace` carry goal context.
2. Ensuring scenario goldens assert each branch via the `(goal_key, generic_cause)` filter rather than by inventing Wash-specific variants. If a future audit reveals an existing branch does not actually carry goal context, that gap is a separate spec amendment — not a license to specialize cause types.

### 6. Player POV CLI assertion

The CLI must not display remote `WashBasin::clean_water`, basin dirtiness, or queue/contention state unless the controlled agent has either (a) FND-14A same-tick co-located observation or (b) a belief entry with the matching payload. This deliverable is a single scenario-level assertion that exercises the existing belief-view accessors and confirms no leak. No new accessor is introduced; the assertion exercises the gating already landed in S158/S162/S163.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: The `survival-scattered` and `survival-contested` scenarios are the canonical proof surface for self-care collision under bounded planner budget. Before S172 implementation, Wash was one of the five homeostatic needs but escaped parts of those contracts. As a result, an entire fifth of the survival loop could escape budget pressure without breaking goldens — exactly the seam where a meter becomes decorative instead of systemic. `archive/tickets/S172WASDISBUD-001.md` and `archive/tickets/S172WASDISBUD-002.md` landed the contested and scattered budget-closure tests, `archive/tickets/S172WASDISBUD-003.md` landed the scattered/contested belief-only candidate regression, and `archive/tickets/S172WASDISBUD-004.md` landed the CLI/player-POV remote WashBasin leak assertion.

2. **New entities/relations/records**: None. The spec audits existing surfaces (`emit_wash_goal`, `WASH_OPS`, `GoalPlanningBudget::SELF_CARE`, `MayContainWashBasin`, `WashBasinState`, `MetabolismProfile`) and existing decision-event payloads.

3. **Actions that mutate them**: None. The `wash` action, candidate emission, and search transitions retain their current mutators. Scenario contract files (`.ron` + Rust test files) are updated to include Wash.

4. **Information production and travel**: Wash candidate generation reads only Self, FND-14A same-tick local observation, Belief-backed entries, and Public topology — per the source-class table in Deliverable 1. No information path changes.

5. **Conserved quantities**: None added. `WashBasinState::clean_water` and actor `HomeostaticNeeds::dirtiness` continue to be conserved as today (water consumed at commit; dirtiness reduced).

6. **Scarce capacities and contention**: Not applicable to this spec. Basin occupancy / mutual-exclusion is the subject of S173.

7. **Partial failures and aftermath**: Four lawful failure branches (Deliverable 5) — Completed, BudgetExhausted, BeliefDisconfirmed, NoCandidate. Each leaves traceable decision-event state. No silent rescue, no global truth correction.

8. **Positive feedback loops**: None introduced. Wash budget pressure does not amplify other needs; it competes with them through the shared `GoalPlanningBudget::SELF_CARE` arena. The shared budget IS the dampener for thrashing between competing self-care goals.

9. **Physical dampeners**: Not applicable — no new amplifying loop. Existing dampeners apply: `WashBasinState::clean_water` depletion (Wash consumes water; basin must be refilled by an explicit water-acquisition action); place-graph travel cost (distant basins charge travel duration); planner budget exhaustion (Wash competes with Eat/Drink/Sleep/Relieve in the same self-care budget).

10. **Agent learning**: Not directly. An agent that repeatedly attempts a disconfirmed basin will accumulate failed plan attempts; learning over those is out of scope (P1.3 in the source report, deferred).

11. **How agents can be wrong**: An agent's basin belief can be stale (basin emptied, dirtied, or destroyed since last observation). Revalidation at action start fires the belief-disconfirmed surface from Deliverable 5 (`RevalidationOutcome::Invalidated { reason: ExpectationMismatch, mismatch_detail }` paired with `ExpectationMismatchPayload.goal_key`). Correction comes from the agent's own perception on arrival or from witness reports — never from authoritative-read of remote state.

12. **Lifecycle states**: None added. Wash goal lifecycle uses existing planner failure-attribution states.

13. **Temporal resolution**: Wash candidate enumeration runs once per agent-tick during candidate generation, before search. Same as Eat/Drink/Sleep/Relieve.

14. **Boundary conditions**: Not applicable — Wash is fully inside the simulated region.

15. **Derived views**: None new. The Deliverable 1 source-class table makes explicit that no derived snapshot or cache is read during candidate emission.

16. **Causal records**: The four failure-attribution surfaces pinned in Deliverable 5 — `WashFacilityUsed` (commit), `PlanSearchOutcome::BudgetExhausted` + `Discrepancy::SearchBudgetExhausted` (budget), `RevalidationOutcome::Invalidated { reason: ExpectationMismatch, mismatch_detail }` (belief disconfirmed), and `emit_wash_goal` empty-emit diagnostics (no candidate) — provide the causal record. Scenario goldens assert each branch via the `(goal_key, generic_cause)` filter convention.

17. **Target patterns**: Scenario A (Wash budget closure in scattered survival); Scenario B (Wash under belief-only remote basin); Scenario C (Wash budget exhausted → traceable failure → recovery via Eat/Drink/Sleep/Relieve rotation).

18. **Save/load and replay**: No new state. Existing replay determinism is unaffected. Decision-trace payloads are already replay-safe.

## Stored State vs. Derived Read-Model List

The spec adds no new authoritative state and no new derived view. The Deliverable 1 source-class table is the load-bearing artifact: it enumerates every read the Wash candidate path performs and classifies each.

## Planner-formalism analysis

Wash candidate enumeration is plain GOAP/affordance search over the existing `WASH_OPS = [Wash, Travel]` op set. No HTN method is registered, decomposed, or required. Fallback semantics are the standard GOAP behavior: candidate emission, search expansion, planner-budget exhaustion, runtime revalidation, retry or replan. The spec does not introduce a Wash-specific method, schema contract, or stage builder.

## Systemic-validation analysis (FND-31)

| Check | Negative case | Mechanism |
|-------|---------------|-----------|
| No remote-truth leak | Wash plan composed for unseen remote basin | Belief-only sub-scenario (Deliverable 4); assertion that no candidate references the remote basin id |
| No silent budget escape | Wash exhausts budget without trace | Scenario A/B/C; assertion that the existing budget-exhausted surface (`PlanSearchOutcome::BudgetExhausted` + `Discrepancy::SearchBudgetExhausted` filtered by `goal_key == GoalKind::Wash`) is the only path to budget exhaustion involving Wash |
| Player/AI symmetry | UI displays remote basin clean-water without belief | Deliverable 6 scenario-level CLI assertion |
| No partial relief without lawful precondition | Wash commits when `clean_water == 0` | Existing `wash_preconditions` invariant; spec pins it |
| Replay determinism | Decision trace differs across replays | Existing replay-equivalence golden; spec adds Wash-specific assertions |

## SystemFn Integration

No new SystemFn is introduced. The spec touches:

- `extract_need_candidates` in `crates/worldwake-ai/src/candidate_generation.rs` (audit only)
- The `wash` action handler in `crates/worldwake-systems/src/needs_actions.rs` (no behavior change; trace-field audit only)
- Scenario contract `.ron` files (`survival-scattered.ron`, `survival-contested.ron`) and their Rust test counterparts

Ordering against other systems is unchanged.

## Component Registration

No new components are registered on `EntityKind::Agent` or any other entity kind. All inputs read by Wash candidate generation already exist (`HomeostaticNeeds`, `MetabolismProfile`, `WashBasinState`, `MayContainWashBasin` belief entries).

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Belief view (S158/S162) | Wash candidate emission reads belief entries; cannot read authoritative remote state | State-mediated, read-only from belief |
| Needs system (E09) | Provides `HomeostaticNeeds::dirtiness`, `MetabolismProfile::dirtiness_rate`, and per-agent `DriveThresholds` (drive-emission gating) | State-mediated |
| WashBasin facility (S129) | Provides `WashBasinState::clean_water` for co-located reads; belief carries remote payload | State-mediated |
| Place graph (E02) | Public topology used for Wash travel arcs | State-mediated, public |
| Planner search (existing) | Wash op participates in shared self-care budget | State-mediated via `GoalPlanningBudget::SELF_CARE` |
| CLI POV boundary (S163) | UI accessors respect belief-source class for basin state | State-mediated via belief-view accessors |

## Profile-Driven Parameters

The spec adds no new profile field. It pins the use of existing profile-driven parameters as they appear in current code:

- `DriveThresholds` (`crates/worldwake-core/src/drives.rs:58`) — per-agent thresholds at which homeostatic pressure escalates into drive-emitted candidates, including dirtiness urgency. Verified via `emit_wash_goal` signature `(candidates, diagnostics, ctx, needs, thresholds: DriveThresholds)` at `crates/worldwake-ai/src/candidate_generation.rs:4607`.
- `MetabolismProfile::wash_ticks: NonZeroU32` (`crates/worldwake-core/src/needs.rs:166`) — per-agent Wash action duration.
- `MetabolismProfile::dirtiness_rate: Permille` (`crates/worldwake-core/src/needs.rs`) — per-agent rate at which dirtiness accumulates between Wash actions.
- `WashBasinState::clean_water` (per-basin, scenario-authored via `S129`) — clean-water count consumed by Wash. The Wash precondition minimum is `Precondition::TargetHasWashBasinClean { min: 1 }` per `wash_preconditions` at `crates/worldwake-systems/src/needs_actions.rs:271-289` — a per-action precondition constant, not a per-agent profile field.
- `GoalPlanningBudget::SELF_CARE` (`crates/worldwake-core/src/goal_planning_budget.rs:13`) — planner budget shared by all self-care goals including Wash.

Per `docs/spec-drafting-rules.md`, no Permille or [0,1000] range value is added. Existing fields retain their current types.

## Scenario Validation

### Scenario A — Wash budget closure in scattered survival

`S172WASDISBUD-002` updated the `survival-scattered` Rust golden so the budget-exhaustion assertion includes Wash and the event log must contain `WashFacilityUsed` payloads for every agent. No `.ron` change was required because the scenario already separated food, water, latrine, and washbasin across places under travel pressure and already listed Wash in `required_self_care_families`.

Assertions:
- `required_self_care_families` includes Wash.
- Each agent exercises Wash at least once within the run window OR emits the budget-exhausted surface from Deliverable 5 (filtered by `goal_key == GoalKind::Wash`) with a documented recovery branch.
- No Wash plan references a basin the agent has no belief about.
- Replay-equivalence golden holds.

### Scenario B — Wash under contested topology

`S172WASDISBUD-001` updated `scenarios/survival-contested.ron` so the scenario includes Wash and updated the contested Rust golden so the budget-exhaustion assertion includes Wash.

Assertions: same shape as Scenario A, plus assertion that no Wash plan reads remote `WashBasinState` directly.

### Scenario C — Belief-only Wash regression in scattered/contested topologies

Landed by `archive/tickets/S172WASDISBUD-003.md`: sub-scenarios of A and B in which the selected agent has no belief about a `WashBasin` that exists in authoritative truth. The agent's dirtiness remains unresolved; no Wash candidate, plan, selection, or commit occurs. The landed isolation deliberately disables exploration and non-dirtiness need pressure so this proof stays at the belief-only candidate boundary.

Assertions:
- No `emit_wash_goal` candidate references the remote basin id.
- The no-candidate surface from Deliverable 5 fires for the dirty agent (if dirtiness crosses the urgency threshold) — observable via `CandidateGenerationDiagnostics` showing zero Wash candidates that tick.
- No Wash plan is found or selected, no `wash` action commits, and dirtiness does not drop.

### Scenario D — Player POV self-care UI assertion

Landed by `archive/tickets/S172WASDISBUD-004.md`: `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent` extends the existing belief-only Wash harness. The controlled agent remains at a place without a co-located `WashBasin`, the remote basin carries non-default authoritative `WashBasinState` plus queue/grant state, and `PerAgentBeliefView` surfaces used by CLI consumers return default/none for remote clean-water, dirtiness, queue position, and grant state without a belief entry.

## Risks and Open Questions

1. **`emit_wash_goal` helper-body audit closed.** The S172 implementation verified the source-class table through live helper inspection plus regression coverage: `wash_access_opportunities` enumerates reachable places through public topology, obtains Wash facilities through the belief view, and only admits basins whose `facility_wash_basin_state` is visible through co-location or stored belief; tickets 003 and 004 pin the negative remote-truth/POV cases.
2. **`MetabolismProfile` and `DriveThresholds` field naming.** Field names in Profile-Driven Parameters and the Deliverable 1 source-class table are pinned to current source at reassessment time (`needs.rs:166` for `wash_ticks`, `drives.rs:58` for `DriveThresholds`). If field names drift between reassessment and implementation, the implementation maps to actual field names and surfaces the drift as a spec amendment.
3. **Decision-trace payload variants.** Deliverable 5 commits to pure reuse of existing emission surfaces (option (3) of the project's "Discrepancy as Failure-Attribution Surface" pattern). No new variants are introduced. The implementation must verify that the existing substrates (`SelectionTrace`, `ExpectationMismatchPayload`, `CandidateGenerationDiagnostics`) carry sufficient goal-key context for the `(goal_key, generic_cause)` filter convention. If a verification gap is found, that becomes a documented spec amendment — not a license to introduce Wash-specific cause types.
4. **Sub-scenario vs. parameterized variant.** Whether Scenario C is a separate `.ron` file or a parameterized arm of the existing scenarios is left to implementation taste; the assertion contract is what matters.

## Out of Scope (Tracked Elsewhere)

- Basin occupancy / mutual exclusion — S173.
- Self-care interruption contracts for Eat/Drink/Toilet/Wilderness/Wash — S173.
- Repeated-interruption collapse trace — S173.
- Recovery-memory blockers (avoid retrying a recently-failed basin) — deferred (P1.3 in source report).
- Disease, sanitation economy, etiquette, privacy, social shame — deferred (P2 in source report).
