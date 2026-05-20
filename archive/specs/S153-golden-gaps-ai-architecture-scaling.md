# S153: Golden Gaps — AI Architecture Scaling

**Status**: COMPLETED

## Summary

PR-15 (Adversarial regression scenarios) from `reports/ai-architecture-improvements.md` lists eight adversarial scenario patterns the architecture should produce but currently lacks golden coverage for. The triage scope-down originally narrowed this spec to four patterns where Phase 12's other accepted specs provide the substrate to make the goldens meaningful: belief-wall trap (regression for S143's trait separation), false rumor justice (regression for S151's testimony reliability), office vacancy → patrol gap (regression for S148's portfolio expansion exercising obligation/duty slots under stress), scaled contention (regression for S150's cross-goal blocker scoping under realistic resource pressure).

Status update (2026-05-13): `archive/tickets/S143STABELVIE-006.md` landed the belief-wall trap regression as `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (a module registered in `crates/worldwake-ai/tests/scenarios/mod.rs` and run through the `golden_ai` harness binary) with inline harness construction rather than a RON scenario. S153's remaining active scope was therefore the three not-yet-landed adversarial patterns: false rumor justice, office vacancy → patrol gap, and scaled contention. Status update (2026-05-20): `archive/tickets/S153GOLDGAPSCALE-001.md` landed false-rumor justice at helper/event-payload level; `archive/tickets/S153GOLDGAPSCALE-004.md` landed the office-backed patrol duty substrate; `archive/tickets/S153GOLDGAPSCALE-005.md` landed the office-vacancy patrol-gap golden as Scenario 444; `archive/tickets/S153GOLDGAPSCALE-003.md` landed scaled contention as Scenario 445.

Four scenarios deferred until substrate ships: 100-goal dense market (needs S144 diagnostics to verify behavior at scale); 20-agent route bottleneck (needs S147 HTN methods for caravan/escort decomposition); long production chain (4+ prereqs) (covered after S146 GoalSchema per-goal budgets land); boundary shock (covered by Phase 7's planned S62 + S64).

Each landed S153 scenario block follows the project's golden-gaps convention: per-scenario Setup, Assertion, GoalKinds/ActionDomains exercised, emergence justification, and "Why it is not a duplicate." New scenario families land as golden test modules under `crates/worldwake-ai/tests/scenarios/` (registered in `tests/scenarios/mod.rs`, run via `cargo test -p worldwake-ai --test golden_ai <name>` per the post-S154 harness consolidation); when a live golden owner already exists, the S153 slice extends that owner instead of creating a duplicate module. Whether each scenario's world is built inline (the precedent set by the landed `belief_wall_trap.rs`) or backed by a committed `scenarios/*.ron` file is a per-scenario implementation choice — there is no `scenarios/golden-*.ron` naming convention in the repo today, and the landed belief-wall regression used an inline fixture rather than RON. The substance matches archived `S81-golden-gaps-simulation-remediation.md` and `S76-golden-gaps-simulation-observer.md`.

This spec is the final wave of Phase 12: it validates the other accepted specs by exercising them under adversarial conditions. After S143STABELVIE-006, the S153 goldens diagnose regressions in archived S148, archived S150, and archived S151 directly; the S143 belief-wall regression is covered by the landed S143 golden.

## Phase and Status

Phase 12: AI Architecture Evolution — Completed

## Crates

- `worldwake-ai` — owns the landed golden coverage under `crates/worldwake-ai/tests/scenarios/`: false-rumor justice extended the existing `testimony_reliability.rs` owner; office vacancy landed as `office_vacancy.rs` Scenario 444 under `archive/tickets/S153GOLDGAPSCALE-005.md`; scaled contention landed as `scaled_contention.rs` Scenario 445 under `archive/tickets/S153GOLDGAPSCALE-003.md`, plus the new `golden_harness/route_blocker_assertions.rs` helper. `tests/scenarios/belief_wall_trap.rs` already landed under S143STABELVIE-006.
- `worldwake-cli` — owns any committed `scenarios/*.ron` files a scenario chooses to use (RON is optional per the inline-fixture precedent) and the scenario loader path those files exercise.
- Other crates: no source change.

## Dependencies

- S143 (Static Belief-View Trait Separation, Phase 12, archived at `archive/specs/S143-static-belief-view-trait-separation.md`) — provides the trait fences the belief-wall trap golden exercises; `S143STABELVIE-006` already landed that regression.
- S148 (Portfolio Slot Expansion, archived at `archive/specs/S148-portfolio-and-motive-backed-intentions.md`) — provides the five-slot portfolio the office-vacancy golden exercises.
- S150 (Cross-Goal Blocker Scoping, archived at `archive/specs/S150-cross-goal-blocker-scoping.md`) — provides the typed scopes the scaled-contention golden checks.
- S151 (Testimony Reliability and Route Preferences, archived at `archive/specs/S151-testimony-reliability-and-route-preferences.md`) — provides the testimony substrate the false-rumor-justice golden exercises.
- S125 (Institutional Treasuries and Bounty Funding, archived) — used by office-vacancy scenario for institutional bounty issuance.
- S119 / S121 (Authored Survival Health Contracts, archived) — golden-harness contract assertion helpers.

## Design Goals

1. **Each golden block proves one architectural claim.** Belief-wall trap proves trait separation and is already covered by S143STABELVIE-006. False-rumor justice proves testimony reliability. Office-vacancy proves portfolio breadth under obligation pressure. Scaled-contention proves cross-goal blocker scoping.
2. **Scenarios use only authored profile sets exercised by Phase 12 specs.** No `DiversificationProfile`-only behaviors, no `ExplorationProfile`-only fallbacks. The proof is about the architectural pieces, not feature stacking.
3. **Deterministic replay.** Each scenario has a fixed seed; the golden harness asserts replay produces identical event sequences.
4. **No false negatives.** Each scenario's assertions check the desired *positive* behavior, not the absence of pathologies.
5. **Per FND-31.** Each golden carries a falsification check: what would have to change in the world for the assertion to be wrong?

## Non-Goals

- **No 100-goal dense market golden.** Deferred — needs S144 to verify scaling without artifacts.
- **No 20-agent contention golden.** Deferred — needs more agent-mass than current scenarios exercise.
- **No long-production-chain golden.** Deferred — covered post-S146 per-goal budgets.
- **No boundary shock golden.** Phase 7's S62 territory.
- **No new engine functionality.** Pure scenario + golden coverage.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent; Social Facts Are Not) | Belief-wall trap golden directly tests the FND-14A boundary against the new S143 trait separation. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | False-rumor justice golden traces rumor propagation through testimony carriers and reliability updates. |
| FND-21 (Intentions Are Revisable Commitments) | Office-vacancy golden requires agents to suspend / abandon office-dependent commitments when succession fails. |
| FND-25 (Social Artifacts Are First-Class) | Scaled-contention golden exercises queue tickets and grants as world artifacts. |
| FND-31 (Validation and Falsification Are First-Class) | The whole spec exists to keep the remaining archived S148/S150/S151 scenarios falsifiable through committed scenarios while preserving the already-landed S143 belief-wall regression. |

## Deliverables

### D1: Belief-wall trap covered by S143STABELVIE-006

`archive/tickets/S143STABELVIE-006.md` landed `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (registered in `tests/scenarios/mod.rs`) as an inline fixture rather than a committed RON scenario. The landed Scenario 420 proves the S143/FND-14A wall at the trait/read-surface boundary: an actor has local physical observation of a co-located item lot and facility, has no owner/holder/jurisdiction/office-holder beliefs, emits no theft candidate, commits no steal action. The `DebugWorldView`-remains-outside-`RuntimeBeliefView` compile-fail doctest lives alongside the trait definitions in `crates/worldwake-sim/src/belief_view.rs` (preceding `DebugWorldView` at `belief_view.rs:945`), not in the test module.

The earlier RON-backed sketch and suppression-reason wording are superseded by the stronger live proof seam for S143: candidate absence at generation and decision-trace layers, authoritative no-commit, and a runtime trait-composition compile-fail witness.

**Why not a duplicate**: Prior goldens test legality predicates with belief entries present; the landed S143 golden tests the *absent-belief* path against the trait-fence enforcement.

### D2: `crates/worldwake-ai/tests/scenarios/testimony_reliability.rs` Scenario 443

Status update (2026-05-20): `archive/tickets/S153GOLDGAPSCALE-001.md` landed the false-rumor justice slice as Scenario 443 in the existing `crates/worldwake-ai/tests/scenarios/testimony_reliability.rs` owner rather than creating a duplicate `false_rumor_justice.rs` module. The live proof seam is helper-level testimony reliability and decision-payload coverage: W has prior accusation refutations, V remains a distinct corroborating source, V's contradiction advances W's `contradicted_claims`, W remains below trust threshold, and the suppressed-goal payload carries W's low-trust summary.

**Setup**: Witness W is unreliable for `TopicScope::AccusationCredibility`; Witness V is a distinct corroborating source with no negative reliability entry for that topic. W has two prior refutations, then V contradicts W's accusation claim.

**Assertions**:
1. M's `TestimonyReliability` entry for the `(source: W, topic)` key has `direct_refutations >= 2` from prior unreliable testimony (pre-seeded). (`TestimonyReliability` is keyed by `TestimonyReliabilityKey { source, topic }`; the per-key `TestimonyReliabilityEntry` carries `direct_refutations` and `contradicted_claims` among its counters — `crates/worldwake-core/src/testimony_reliability.rs`.)
2. V's corroborating role does not inherit W's negative history because reliability is keyed by source and topic.
3. V's contradiction advances W's `TestimonyReliability` `contradicted_claims`.
4. W remains below the trust threshold after the contradiction.
5. The decision payload records the suppressed unreliable testimony context (`DecisionEventPayload::GoalSuppressed` with `SuppressedByUnreliableTestimony`).
6. The helper-level deterministic replay repeats the same reliability entries, trust summary, payload, and corroborating-entry absence result.

**GoalKinds/ActionDomains exercised**: `AskWitness`, `Accuse`, decision-history payload, testimony reliability updates.

**Emergence justification**: There is no authored "ignore W" rule. W's prior experience is concrete `TestimonyReliability` state, and contradiction with V changes W's source/topic reliability state rather than global truth.

**Why not a duplicate**: Existing goldens test single-source testimony updates. This golden tests *cross-source contradiction with prior reliability state*, which is uniquely enabled by S151.

### D3: Office-vacancy → patrol-gap substrate and golden

Status update (2026-05-20): `archive/tickets/S153GOLDGAPSCALE-002.md` was rejected during live reassessment because its test-only premise was false. `GoalKind::Patrol` was driven by `PatrolRoute` / `PatrolProfile` and vacancy-aware patrol motive; `ExpectationStore` overdue state drives missing-person search/report motives, not patrol-duty validity. `archive/tickets/S153GOLDGAPSCALE-004.md` landed office-backed patrol duty assignments as first-class institutional world state plus focused lifecycle and AI suppression proof. `archive/tickets/S153GOLDGAPSCALE-005.md` landed the remaining office-vacancy golden as `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` Scenario 444.

**Landed setup in `archive/tickets/S153GOLDGAPSCALE-005.md`**: Town with a vacant road-magistrate office + 2 guards holding concrete office-backed patrol duty assignments on the live Village Square -> South Gate route. Each guard's patrol duty has issuer, assignee, route places, lifecycle state, renewal/deadline policy, actionability, and causal provenance. With the office vacant and no successor/delegate renewing the duties, the duty assignments lapse. A merchant later traverses the route and observes a hostile event.

**Assertions**:
1. The office is observable as vacant through the authored vacant-office setup.
2. Each guard's `ObligationDuty` slot (`SlotKind::ObligationDuty`, S148) still ranks the patrol duty initially.
3. Within the authored renewal/deadline window, each office-backed patrol duty transitions from active/actionable to lapsed through explicit duty maintenance. The transition surfaces as append-only causal history and inspectable duty state.
4. With the patrol duties no longer active/actionable and no successor/delegate renewing them, the guards have no valid patrol duty candidate on that route.
5. A merchant traverses the unpatrolled route through the ordinary travel action path while neither guard commits patrol.
6. The merchant observes hostile route danger; successor route-danger state records the hostile traversal as `RouteExperience`.

**GoalKinds/ActionDomains exercised**: `GoalKind::Patrol`, office-backed patrol duty lifecycle, `SlotKind::ObligationDuty` portfolio slot dynamics.

**Emergence justification**: The patrol gap is not authored — it emerges from (a) the magistrate's death lawfully suspending the office's legal effects, (b) office-backed patrol duties degrading or lapsing with no successor/delegate renewing them, (c) the portfolio slot system letting other slots win when `ObligationDuty` has nothing valid. No hidden scenario flag fires.

**Why not a duplicate**: Existing goldens test obligation issuance and patrol behavior. This golden tests the *vacancy → gap* failure mode the assessment specifically calls out.

### D4: `crates/worldwake-ai/tests/scenarios/scaled_contention.rs`

Status update (2026-05-20): `archive/tickets/S153GOLDGAPSCALE-003.md` landed scaled contention as `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` Scenario 445 with an inline `golden_ai` fixture and deterministic replay. The live well proof uses authoritative per-slot `ResourceExtractionQueues` state, not facility-level `QueueGrantPromoted` events, because per-slot extraction queues do not emit that facility-level event.

**Landed setup**: Six agents (deliberately above `survival-contested.ron`'s four, to intensify rivalry at capacity-bounded resources), two wells (capacity 2 each), one wash basin (capacity 1), at a central hub. Agents have hunger / dirtiness / thirst state. A direct route carries prior dangerous traversal state — at least one agent has `RoutePreferenceEntry.dangerous_traversals >= 2` — plus an agent-carried `RouteSegment` blocker with `TtlOnly` clearing.

**Assertions**:
1. Wells issue `ContentionGrant`s up to capacity through `ResourceExtractionQueues`; waiting agents enter per-slot queues.
2. A hungry actor has local apple substitution while water slots are full.
3. `RoutePreference` state records a below-neutral direct route after two dangerous traversals for at least one agent.
4. `RouteSegment` blocker (S150) on the direct route is recorded in the first agent's `BlockerMemory`. The helper proves the blocker persists through the TTL window and clears via `BlockerClearingCondition::TtlOnly` (`crates/worldwake-core/src/blocker_memory.rs:176`).
5. The alternate route segment remains available while the direct segment is blocked.
6. No agent dies under the authored contention envelope.

**GoalKinds/ActionDomains exercised**: `GoalKind::ConsumeOwnedCommodity` (eat/drink against owned food/water), `GoalKind::Wash`, `GoalKind::AcquireCommodity`, travel as a prerequisite `PlannerOp` / `TravelEdge` traversal (travel is a planner subchain, not a standalone `GoalKind`), queue/grant lifecycle, route preference, route-segment blocker (`BlockerScope::RouteSegment`).

**Emergence justification**: Six agents share three contended resources; outcomes emerge from queue contention + route preferences + cross-goal blockers without any per-agent script.

**Why not a duplicate**: `survival-contested.ron` exists but doesn't exercise S150 RouteSegment blockers or S151 RoutePreference state.

### D5: Shared golden-harness assertion helpers

Two helpers in the existing `golden_harness/` directory:

- `expect_route_blocker_lifecycle(event_log, segment, observation_event, observed_tick, ttl)` — landed by `archive/tickets/S153GOLDGAPSCALE-003.md`; asserts blocker recording, source-event presence, persistence, and `TtlOnly` clearing.
- `expect_testimony_reliability_update(source, topic, before, after, observation_event)` — landed by `archive/tickets/S153GOLDGAPSCALE-001.md`; asserts a single reliability transition.

(The belief-wall compile-fail proof needs no harness helper — it already lives as a `compile_fail` doctest alongside `DebugWorldView` in `crates/worldwake-sim/src/belief_view.rs`, and the landed `belief_wall_trap.rs` regression does not invoke a helper for it.)

### D6: Determinism regression

Each S153 scenario has deterministic coverage. For runtime-style scenarios, the golden harness asserts:
1. Event log byte-stable across reruns.
2. Final `ScenarioDiagnosticsReport` (S144) byte-stable across reruns.

For helper-level testimony reliability coverage, Scenario 443 asserts equality of the repeated before/after reliability entries, trust summary, suppressed payload, and corroborating-entry absence result.

### D7: Falsification documentation

Each landed S153 scenario block carries a `// Falsification:` comment block: what would need to change in the world for the assertion to be wrong. For false-rumor justice, the landed block says W remaining above threshold or failing to advance `contradicted_claims` after V contradicts the claim would falsify the S151 reliability grounding. (The already-landed `belief_wall_trap.rs` predates this convention and does not carry the comment; retrofitting it is out of scope for this spec.)

## FND-01 Section H Analysis

### Information-Path Analysis

Each scenario exercises an existing information path:
- Belief-wall trap: already covered by S143STABELVIE-006 through observation via S143's `LocalPhysicalObservationView` and absent authority beliefs through `BelievedAuthorityView`.
- False-rumor justice: testimony through S139's AskWitness; reliability updates through S151's confirmation/refutation hooks.
- Office vacancy: office legal-effect suspension through S140 (`ArtifactLegalEffect::Suspended`); office-backed patrol duty lifecycle through `archive/tickets/S153GOLDGAPSCALE-004.md`; end-to-end golden proof through `archive/tickets/S153GOLDGAPSCALE-005.md`; portfolio slot dynamics through S148; route-danger propagation through perception.
- Scaled contention: grant/queue lifecycle through S140; route blockers through S150; route preferences through S151. `archive/tickets/S153GOLDGAPSCALE-003.md` proved the final inline golden seam using per-slot extraction queues plus agent-carried blocker memory.

False-rumor justice and scaled contention introduce no new information path. Office vacancy now has an office-backed patrol duty substrate from `archive/tickets/S153GOLDGAPSCALE-004.md`, and `archive/tickets/S153GOLDGAPSCALE-005.md` proved the authored golden path through concrete duty assignment, vacancy lifecycle, candidate suppression, ordinary route traversal, and route-danger memory rather than global truth.

### Positive-Feedback Analysis

Not applicable. Goldens validate existing dampeners.

### Concrete Dampeners

Tested rather than introduced: queue grant capacity (scaled contention), office-backed duty renewal/deadline or lapse dampener (office vacancy substrate landed in `archive/tickets/S153GOLDGAPSCALE-004.md`, golden proof landed in `archive/tickets/S153GOLDGAPSCALE-005.md`), testimony trust threshold (`TestimonyTrustProfile.minimum_observations`, false rumor), route-blocker TTL — `BlockerClearingCondition::TtlOnly` (scaled contention).

### Stored State vs. Derived Read-Model List

False-rumor justice and scaled contention add no new stored state. Office vacancy now uses `OfficePatrolDuty` state landed by `archive/tickets/S153GOLDGAPSCALE-004.md`; `archive/tickets/S153GOLDGAPSCALE-005.md` landed the remaining golden proof.

## SystemFn Integration

False-rumor justice and scaled contention do not add SystemFn integration. Office vacancy duty lifecycle maintenance landed in the patrol system slot under `archive/tickets/S153GOLDGAPSCALE-004.md`.

## Component Registration

False-rumor justice and scaled contention do not add components. Office vacancy uses `OfficePatrolDuty`, registered on guard agents by `archive/tickets/S153GOLDGAPSCALE-004.md`.

## Cross-System Interactions

False-rumor justice and scaled contention exercise integration paths between archived S143, archived S148, archived S150, and archived S151 substrate without introducing new cross-system interaction. Office vacancy requires state-mediated interaction between office lifecycle, patrol duty lifecycle, AI portfolio selection, and route-danger observation; `archive/tickets/S153GOLDGAPSCALE-004.md` landed the duty lifecycle and AI suppression substrate, and `archive/tickets/S153GOLDGAPSCALE-005.md` landed the route-outcome golden.

## Profile-Driven Parameters

Not applicable. Goldens use authored scenario data; no new profile fields.

## Test Plan

- S153 golden coverage with the per-block assertions above; false-rumor justice extends `testimony_reliability.rs`, office vacancy landed in `office_vacancy.rs` under `archive/tickets/S153GOLDGAPSCALE-005.md`, scaled contention landed in `scaled_contention.rs` under `archive/tickets/S153GOLDGAPSCALE-003.md`, and the belief-wall trap golden is already covered by S143STABELVIE-006.
- D6 determinism regression.
- All goldens passing — `cargo test --workspace`.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Outcome

Completed: 2026-05-20.

S153 landed the four highest-impact PR-15 adversarial regression patterns:

- Belief-wall trap coverage landed under `archive/tickets/S143STABELVIE-006.md` as Scenario 420 in `belief_wall_trap.rs`.
- False-rumor justice coverage landed under `archive/tickets/S153GOLDGAPSCALE-001.md` as Scenario 443 in `testimony_reliability.rs`.
- Office vacancy -> patrol gap coverage landed through `archive/tickets/S153GOLDGAPSCALE-004.md` and `archive/tickets/S153GOLDGAPSCALE-005.md` as Scenario 444 in `office_vacancy.rs`.
- Scaled contention coverage landed under `archive/tickets/S153GOLDGAPSCALE-003.md` as Scenario 445 in `scaled_contention.rs`, with `expect_route_blocker_lifecycle` in `golden_harness/route_blocker_assertions.rs`.

Deviations from the original plan:

- False-rumor justice landed at helper/event-payload level in the existing testimony reliability owner rather than as a duplicate autonomous `false_rumor_justice.rs` module.
- Office vacancy required a production substrate split because the original test-only premise was false; office-backed patrol duty state landed before the golden.
- Scaled contention uses an inline fixture and authoritative per-slot `ResourceExtractionQueues` assertions rather than a RON scenario or facility-level queue-promotion event assertions.
- The four lower-readiness PR-15 patterns remain deferred as documented non-goals.

Verification:

- Passed `cargo test -p worldwake-ai --test golden_ai scaled_contention`.
- Passed `cargo test -p worldwake-ai --test golden_ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `./scripts/verify.sh`.
