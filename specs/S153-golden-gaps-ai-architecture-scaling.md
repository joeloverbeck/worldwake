# S153: Golden Gaps — AI Architecture Scaling

**Status**: Draft

## Summary

PR-15 (Adversarial regression scenarios) from `reports/ai-architecture-improvements.md` lists eight adversarial scenario patterns the architecture should produce but currently lacks golden coverage for. The triage scope-down originally narrowed this spec to four patterns where Phase 12's other accepted specs provide the substrate to make the goldens meaningful: belief-wall trap (regression for S143's trait separation), false rumor justice (regression for S151's testimony reliability), office vacancy → patrol gap (regression for S148's portfolio expansion exercising obligation/duty slots under stress), scaled contention (regression for S150's cross-goal blocker scoping under realistic resource pressure).

Status update (2026-05-13): `archive/tickets/S143STABELVIE-006.md` landed the belief-wall trap regression as `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (a module registered in `crates/worldwake-ai/tests/scenarios/mod.rs` and run through the `golden_ai` harness binary) with inline harness construction rather than a RON scenario. S153's remaining active scope is therefore the three not-yet-landed adversarial patterns: false rumor justice, office vacancy → patrol gap, and scaled contention.

Four scenarios deferred until substrate ships: 100-goal dense market (needs S144 diagnostics to verify behavior at scale); 20-agent route bottleneck (needs S147 HTN methods for caravan/escort decomposition); long production chain (4+ prereqs) (covered after S146 GoalSchema per-goal budgets land); boundary shock (covered by Phase 7's planned S62 + S64).

Each remaining S153 scenario block follows the project's golden-gaps convention: per-scenario Setup, Assertion, GoalKinds/ActionDomains exercised, emergence justification, and "Why it is not a duplicate." Each remaining scenario lands as a golden test module under `crates/worldwake-ai/tests/scenarios/` (registered in `tests/scenarios/mod.rs`, run via `cargo test -p worldwake-ai --test golden_ai <name>` per the post-S154 harness consolidation). Whether each scenario's world is built inline (the precedent set by the landed `belief_wall_trap.rs`) or backed by a committed `scenarios/*.ron` file is a per-scenario implementation choice — there is no `scenarios/golden-*.ron` naming convention in the repo today, and the landed belief-wall regression used an inline fixture rather than RON. The substance matches archived `S81-golden-gaps-simulation-remediation.md` and `S76-golden-gaps-simulation-observer.md`.

This spec is the final wave of Phase 12: it validates the other accepted specs by exercising them under adversarial conditions. After S143STABELVIE-006, the remaining S153 goldens diagnose regressions in archived S148, archived S150, and archived S151 directly; the S143 belief-wall regression is covered by the landed S143 golden.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns the remaining new golden test modules under `crates/worldwake-ai/tests/scenarios/` (`false_rumor_justice.rs`, `office_vacancy.rs`, `scaled_contention.rs`), each registered in `tests/scenarios/mod.rs` and exercised through the `golden_ai` harness, plus the new `golden_harness/` assertion helpers (D5). `tests/scenarios/belief_wall_trap.rs` already landed under S143STABELVIE-006.
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

### D2: `crates/worldwake-ai/tests/scenarios/false_rumor_justice.rs` (+ optional `scenarios/*.ron`)

**Setup**: Three agents — Witness W (unreliable, has been wrong before in TestimonyReliability), Witness V (reliable), Magistrate M. W tells M that Agent A stole from a stash. V was actually present and saw nothing happen. A is innocent.

**Assertions**:
1. M's `TestimonyReliability` entry for the `(source: W, topic)` key has `direct_refutations >= 2` from prior unreliable testimony (pre-seeded). (`TestimonyReliability` is keyed by `TestimonyReliabilityKey { source, topic }`; the per-key `TestimonyReliabilityEntry` carries `direct_refutations` and `contradicted_claims` among its counters — `crates/worldwake-core/src/testimony_reliability.rs`.)
2. M receives W's claim — the belief enters M's store with low confidence (because computed trust falls below M's `TestimonyTrustProfile.minimum_observations` / threshold parameter; the threshold is a profile field, not an entry counter).
3. M ranks `Accuse(A)` candidate against W's testimony; the candidate is damped per S151 ranking integration.
4. M asks V for corroborating testimony (`AskWitness` candidate emitted from the `SocialMotive` slot per archived S148).
5. V's testimony contradicts W's; M's belief contradiction surfaces via S109 `Discrepancy::BeliefContradicted`.
6. M does *not* commit `Accuse` — the contradiction holds enough weight that the decision payload (S136) records the comparison.
7. W's `TestimonyReliability` `contradicted_claims` increments — M learns W's prior pattern.

**GoalKinds/ActionDomains exercised**: `AskWitness`, `Accuse`, decision-history payload, testimony reliability updates.

**Emergence justification**: M does not have an authored "ignore W" rule. M's prior experience with W (concrete `TestimonyReliability` state) shapes ranking, and contradiction with V's testimony resolves through belief-contradiction discrepancy.

**Why not a duplicate**: Existing goldens test single-source testimony updates. This golden tests *cross-source contradiction with prior reliability state*, which is uniquely enabled by S151.

### D3: `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` (+ optional `scenarios/*.ron`)

**Setup**: Town with a magistrate office + 2 guards holding patrol duties issued by the magistrate. The magistrate dies (concrete `DeadAt { tick }` component set on the magistrate — death substrate at `crates/worldwake-core/src/combat.rs:77`, not an event tag). The office becomes vacant. Each guard's patrol duty is backed by an S59 `Expectation` record ("guard should patrol route X by tick Z") carrying an authored `deadline_tick` (~200 ticks out) plus `grace_ticks` (`crates/worldwake-core/src/expectation.rs`); with the magistrate dead, no successor renews those records. A bandit appears on a route.

**Assertions**:
1. After the magistrate dies, the office is observable as vacant (S140 lifecycle `ArtifactLegalEffect::Suspended`).
2. Each guard's `ObligationDuty` slot (`SlotKind::ObligationDuty`, S148) still ranks the patrol duty initially.
3. Within the next ~200 ticks (`deadline_tick + grace_ticks`), each patrol `Expectation` transitions `Active → Overdue` via the `check_overdue_expectations` system (`crates/worldwake-systems/src/expectation_check.rs:7`, transition at `:62`). The transition surfaces as a `WorldMutation`-tagged event (also tagged `EventTag::System`) carrying the updated `ExpectationStore` component delta — there is **no** dedicated `ExpectationFailure` event tag; the golden asserts on the `ExpectationState::Overdue` transition via that `ExpectationStore` delta.
4. With the patrol expectations overdue (no longer `Active`) and no successor renewing them, the guards' `ObligationDuty` slot has no valid patrol duty to rank; `EconomicOpportunity` or `SocialMotive` can win instead.
5. The bandit traverses an unpatrolled route — visible in event log as a route-traversal event with no guard interception.
6. A traveling merchant observes the bandit; route preferences (S151) record the dangerous traversal.

**GoalKinds/ActionDomains exercised**: `GoalKind::Patrol`, S59 `Expectation` overdue transition (`ExpectationStore` delta), `SlotKind::ObligationDuty` portfolio slot dynamics.

**Emergence justification**: The patrol gap is not authored — it emerges from (a) the magistrate's death lawfully suspending the office's legal effects, (b) obligations expiring per their TTL with no successor renewing them, (c) the portfolio slot system letting other slots win when ObligationDuty has nothing valid. No hidden scenario flag fires.

**Why not a duplicate**: Existing goldens test obligation issuance and patrol behavior. This golden tests the *vacancy → gap* failure mode the assessment specifically calls out.

### D4: `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` (+ optional `scenarios/*.ron`)

**Setup**: Six agents (deliberately above `survival-contested.ron`'s four, to intensify rivalry at capacity-bounded resources), two wells (capacity 2 each), one wash basin (capacity 1), at a central hub. All agents have hunger / dirtiness / thirst rising. Single travel route to a remote source. The remote route has been ambushed before — at least one agent has `RoutePreferenceEntry.dangerous_traversals >= 2`.

**Assertions**:
1. Wells issue `ContentionGrant`s up to capacity; waiting agents enter the queue. (Wells are modeled as a `ResourceSource` for commodity `Water` at a `Well` workstation facility — see `survival-contested.ron`. The golden author must confirm which queue substrate the well uses before asserting on grant events: facility-level `ContentionQueue` emits `EventTag::QueueGrantPromoted`, whereas per-slot `ResourceExtractionQueues` does not.)
2. When wells are full, hungry-not-thirsty agents prefer the orchard over waiting (existing portfolio behavior under archived S148 with `EconomicOpportunity` vs `NeedSurvival` weighting).
3. The remote route gets used by agents whose `RoutePreference` is neutral or positive; agents with negative preference (S151) detour or wait.
4. `RouteSegment` blocker (S150) on the remote route is recorded by at least one agent after an ambush event. The blocker persists per TTL.
5. After blocker TTL, the remote route becomes usable again per `BlockerClearingCondition::TtlOnly` (`crates/worldwake-core/src/blocker_memory.rs:176`).
6. No agent dies; all hunger/thirst/dirtiness needs are addressed through queue waiting, route choice, or substitution.

**GoalKinds/ActionDomains exercised**: `GoalKind::ConsumeOwnedCommodity` (eat/drink against owned food/water), `GoalKind::Wash`, `GoalKind::AcquireCommodity`, travel as a prerequisite `PlannerOp` / `TravelEdge` traversal (travel is a planner subchain, not a standalone `GoalKind`), queue/grant lifecycle, route preference, route-segment blocker (`BlockerScope::RouteSegment`).

**Emergence justification**: Six agents share three contended resources; outcomes emerge from queue contention + route preferences + cross-goal blockers without any per-agent script.

**Why not a duplicate**: `survival-contested.ron` exists but doesn't exercise S150 RouteSegment blockers or S151 RoutePreference state.

### D5: Shared golden-harness assertion helpers

Two new helpers in the existing `golden_harness/` directory:

- `expect_route_blocker_lifecycle(segment, observation_event, ttl)` — asserts blocker recording, persistence, and clearing.
- `expect_testimony_reliability_update(source, topic, before, after, observation_event)` — asserts a single reliability transition.

(The belief-wall compile-fail proof needs no harness helper — it already lives as a `compile_fail` doctest alongside `DebugWorldView` in `crates/worldwake-sim/src/belief_view.rs`, and the landed `belief_wall_trap.rs` regression does not invoke a helper for it.)

### D6: Determinism regression

Each of the three remaining S153 scenarios runs with a fixed seed and the golden harness asserts:
1. Event log byte-stable across reruns.
2. Final `ScenarioDiagnosticsReport` (S144) byte-stable across reruns.

### D7: Falsification documentation

Each of the three remaining `tests/scenarios/*.rs` golden modules carries a `// Falsification:` comment block: what would need to change in the world for the assertion to be wrong. E.g., for false-rumor justice: "If M commits `Accuse(A)` despite W's low reliability and V's contradicting testimony, the S151 reliability damping failed." (The already-landed `belief_wall_trap.rs` predates this convention and does not carry the comment; retrofitting it is out of scope for this spec.)

## FND-01 Section H Analysis

### Information-Path Analysis

Each scenario exercises an existing information path:
- Belief-wall trap: already covered by S143STABELVIE-006 through observation via S143's `LocalPhysicalObservationView` and absent authority beliefs through `BelievedAuthorityView`.
- False-rumor justice: testimony through S139's AskWitness; reliability updates through S151's confirmation/refutation hooks.
- Office vacancy: office legal-effect suspension through S140 (`ArtifactLegalEffect::Suspended`); patrol-expectation `Active → Overdue` transition through S59 (`check_overdue_expectations`, `deadline_tick + grace_ticks`); portfolio slot dynamics through S148; route-danger propagation through perception.
- Scaled contention: grant/queue lifecycle through S140; route blockers through S150; route preferences through S151.

No new information path introduced.

### Positive-Feedback Analysis

Not applicable. Goldens validate existing dampeners.

### Concrete Dampeners

Tested rather than introduced: queue grant capacity (scaled contention), expectation `deadline_tick + grace_ticks` (office vacancy), testimony trust threshold (`TestimonyTrustProfile.minimum_observations`, false rumor), route-blocker TTL — `BlockerClearingCondition::TtlOnly` (scaled contention).

### Stored State vs. Derived Read-Model List

No new stored state. No new derived read-model.

## SystemFn Integration

Not applicable.

## Component Registration

Not applicable.

## Cross-System Interactions

Goldens exercise integration paths between archived S143, archived S148, archived S150, and archived S151 substrate; no new cross-system interaction is introduced.

## Profile-Driven Parameters

Not applicable. Goldens use authored scenario data; no new profile fields.

## Test Plan

- Three remaining S153 golden test files with the per-block assertions above; the belief-wall trap golden is already covered by S143STABELVIE-006.
- D6 determinism regression.
- All goldens passing — `cargo test --workspace`.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
