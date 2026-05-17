# S153: Golden Gaps — AI Architecture Scaling

**Status**: Draft

## Summary

PR-15 (Adversarial regression scenarios) from `reports/ai-architecture-improvements.md` lists eight adversarial scenario patterns the architecture should produce but currently lacks golden coverage for. The triage scope-down originally narrowed this spec to four patterns where Phase 12's other accepted specs provide the substrate to make the goldens meaningful: belief-wall trap (regression for S143's trait separation), false rumor justice (regression for S151's testimony reliability), office vacancy → patrol gap (regression for S148's portfolio expansion exercising obligation/duty slots under stress), scaled contention (regression for S150's cross-goal blocker scoping under realistic resource pressure).

Status update (2026-05-13): `archive/tickets/S143STABELVIE-006.md` landed the belief-wall trap regression as `crates/worldwake-ai/tests/golden_belief_wall_trap.rs` with inline harness construction rather than a RON scenario. S153's remaining active scope is therefore the three not-yet-landed adversarial patterns: false rumor justice, office vacancy → patrol gap, and scaled contention.

Four scenarios deferred until substrate ships: 100-goal dense market (needs S144 diagnostics to verify behavior at scale); 20-agent route bottleneck (needs S147 HTN methods for caravan/escort decomposition); long production chain (4+ prereqs) (covered after S146 GoalSchema per-goal budgets land); boundary shock (covered by Phase 7's planned S62 + S64).

Each remaining S153 scenario block follows the project's golden-gaps convention: per-scenario Setup, Assertion, GoalKinds/ActionDomains exercised, emergence justification, and "Why it is not a duplicate." The remaining scenarios are committed RON files (`scenarios/golden-*.ron`) plus golden test files (`crates/worldwake-ai/tests/golden_*.rs`) — same shape as archived `S81-golden-gaps-simulation-remediation.md` and `S76-golden-gaps-simulation-observer.md`.

This spec is the final wave of Phase 12: it validates the other accepted specs by exercising them under adversarial conditions. After S143STABELVIE-006, the remaining S153 goldens diagnose regressions in S148, S150, and S151 directly; the S143 belief-wall regression is covered by the landed S143 golden.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns the remaining new golden test files (`golden_false_rumor_justice.rs`, `golden_office_vacancy.rs`, `golden_scaled_contention.rs`). `golden_belief_wall_trap.rs` already landed under S143STABELVIE-006.
- `worldwake-cli` — owns the remaining supporting RON scenario files (`scenarios/golden-false-rumor-justice.ron`, etc.) and the golden-harness assertions.
- Other crates: no source change.

## Dependencies

- S143 (Static Belief-View Trait Separation, Phase 12, archived at `archive/specs/S143-static-belief-view-trait-separation.md`) — provides the trait fences the belief-wall trap golden exercises; `S143STABELVIE-006` already landed that regression.
- S148 (Portfolio Slot Expansion, Phase 12) — provides the seven-slot portfolio the office-vacancy golden exercises.
- S150 (Cross-Goal Blocker Scoping, archived at `archive/specs/S150-cross-goal-blocker-scoping.md`) — provides the typed scopes the scaled-contention golden checks.
- S151 (Testimony Reliability and Route Preferences, Phase 12) — provides the testimony substrate the false-rumor-justice golden exercises.
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
| FND-31 (Validation and Falsification Are First-Class) | The whole spec exists to keep the remaining S148/S150/S151 scenarios falsifiable through committed scenarios while preserving the already-landed S143 belief-wall regression. |

## Deliverables

### D1: Belief-wall trap covered by S143STABELVIE-006

`archive/tickets/S143STABELVIE-006.md` landed `crates/worldwake-ai/tests/golden_belief_wall_trap.rs` as an inline fixture rather than `scenarios/golden-belief-wall-trap.ron`. The landed Scenario 420 proves the S143/FND-14A wall at the trait/read-surface boundary: an actor has local physical observation of a co-located item lot and facility, has no owner/holder/jurisdiction/office-holder beliefs, emits no theft candidate, commits no steal action, and includes a compile-fail doctest proving `DebugWorldView` remains outside `RuntimeBeliefView`.

The earlier RON-backed sketch and suppression-reason wording are superseded by the stronger live proof seam for S143: candidate absence at generation and decision-trace layers, authoritative no-commit, and a runtime trait-composition compile-fail witness.

**Why not a duplicate**: Prior goldens test legality predicates with belief entries present; the landed S143 golden tests the *absent-belief* path against the trait-fence enforcement.

### D2: `golden_false_rumor_justice.rs` + `scenarios/golden-false-rumor-justice.ron`

**Setup**: Three agents — Witness W (unreliable, has been wrong before in TestimonyReliability), Witness V (reliable), Magistrate M. W tells M that Agent A stole from a stash. V was actually present and saw nothing happen. A is innocent.

**Assertions**:
1. M's `TestimonyReliability` for W has `direct_refutations >= 2` from prior unreliable testimony (pre-seeded).
2. M receives W's claim — the belief enters M's store with low confidence (because trust < threshold).
3. M ranks `Accuse(A)` candidate against W's testimony; the candidate is damped per S151 ranking integration.
4. M asks V for corroborating testimony (`AskWitness` candidate emitted from `SocialEpistemic` slot per S148).
5. V's testimony contradicts W's; M's belief contradiction surfaces via S109 `Discrepancy::BeliefContradicted`.
6. M does *not* commit `Accuse` — the contradiction holds enough weight that the decision payload (S136) records the comparison.
7. W's `TestimonyReliability` `contradicted_claims` increments — M learns W's prior pattern.

**GoalKinds/ActionDomains exercised**: `AskWitness`, `Accuse`, decision-history payload, testimony reliability updates.

**Emergence justification**: M does not have an authored "ignore W" rule. M's prior experience with W (concrete `TestimonyReliability` state) shapes ranking, and contradiction with V's testimony resolves through belief-contradiction discrepancy.

**Why not a duplicate**: Existing goldens test single-source testimony updates. This golden tests *cross-source contradiction with prior reliability state*, which is uniquely enabled by S151.

### D3: `golden_office_vacancy.rs` + `scenarios/golden-office-vacancy.ron`

**Setup**: Town with magistrate office + 2 guards holding patrol obligations issued by the magistrate. Magistrate dies (concrete `DeadAt` event per S81). Office becomes vacant. Patrol obligations have S59 expiration in 200 ticks. A bandit appears on a route.

**Assertions**:
1. After the magistrate dies, the office is observable as vacant (S140 lifecycle `ArtifactLegalEffect::Suspended`).
2. Each guard's `ObligationDuty` slot (S148) still ranks the patrol obligation initially.
3. Within the next 200 ticks, the patrol obligations expire (S59 substrate) — `ExpectationFailure` events emit.
4. With patrols expired, guards' `ObligationDuty` slot empties for that obligation; `EconomicMaintenance` or `OpportunisticLocal` slot wins instead.
5. The bandit traverses an unpatrolled route — visible in event log as a route-traversal event with no guard interception.
6. A traveling merchant observes the bandit; route preferences (S151) record the dangerous traversal.

**GoalKinds/ActionDomains exercised**: `PatrolRoute`, `Obligation`, `ExpectationFailure`, portfolio slot dynamics.

**Emergence justification**: The patrol gap is not authored — it emerges from (a) the magistrate's death lawfully suspending the office's legal effects, (b) obligations expiring per their TTL with no successor renewing them, (c) the portfolio slot system letting other slots win when ObligationDuty has nothing valid. No hidden scenario flag fires.

**Why not a duplicate**: Existing goldens test obligation issuance and patrol behavior. This golden tests the *vacancy → gap* failure mode the assessment specifically calls out.

### D4: `golden_scaled_contention.rs` + `scenarios/golden-scaled-contention.ron`

**Setup**: Six agents (existing project scale cap), two wells (capacity 2 each), one wash basin (capacity 1), at a central hub. All agents have hunger / dirtiness / thirst rising. Single travel route to a remote source. The remote route has been ambushed before — at least one agent has `RoutePreferenceEntry.dangerous_traversals >= 2`.

**Assertions**:
1. Wells issue `ContentionGrant`s up to capacity; waiting agents enter the queue.
2. When wells are full, hungry-not-thirsty agents prefer the orchard over waiting (existing portfolio behavior under S148 with EconomicMaintenance vs Survival weighting).
3. The remote route gets used by agents whose `RoutePreference` is neutral or positive; agents with negative preference (S151) detour or wait.
4. `RouteSegment` blocker (S150) on the remote route is recorded by at least one agent after an ambush event. The blocker persists per TTL.
5. After blocker TTL, the remote route becomes usable again per `BlockerClearingCondition::TtlExpiry`.
6. No agent dies; all hunger/thirst/dirtiness needs are addressed through queue waiting, route choice, or substitution.

**GoalKinds/ActionDomains exercised**: `Eat`, `Drink`, `Wash`, `AcquireCommodity`, `TravelTo`, queue/grant lifecycle, route preference, route-segment blocker.

**Emergence justification**: Six agents share three contended resources; outcomes emerge from queue contention + route preferences + cross-goal blockers without any per-agent script.

**Why not a duplicate**: `survival-contested.ron` exists but doesn't exercise S150 RouteSegment blockers or S151 RoutePreference state.

### D5: Shared golden-harness assertion helpers

Three new helpers in the existing `golden_harness/` directory:

- `expect_belief_wall_compile_fail()` — runs a compile_fail doctest probe to ensure FND-14A widening fails to build.
- `expect_route_blocker_lifecycle(segment, observation_event, ttl)` — asserts blocker recording, persistence, and clearing.
- `expect_testimony_reliability_update(source, topic, before, after, observation_event)` — asserts a single reliability transition.

### D6: Determinism regression

Each of the three remaining S153 scenarios runs with a fixed seed and the golden harness asserts:
1. Event log byte-stable across reruns.
2. Final `ScenarioDiagnosticsReport` (S144) byte-stable across reruns.

### D7: Falsification documentation

Each `golden_*.rs` file carries a `// Falsification:` comment block: what would need to change in the world for the assertion to be wrong. E.g., for belief-wall trap: "If the Steal candidate emits despite missing owner belief, FND-14A widening occurred."

## FND-01 Section H Analysis

### Information-Path Analysis

Each scenario exercises an existing information path:
- Belief-wall trap: already covered by S143STABELVIE-006 through observation via S143's `LocalPhysicalObservationView` and absent authority beliefs through `BelievedAuthorityView`.
- False-rumor justice: testimony through S139's AskWitness; reliability updates through S151's confirmation/refutation hooks.
- Office vacancy: obligation expiration through S59's TTL; portfolio slot dynamics through S148; route-danger propagation through perception.
- Scaled contention: grant/queue lifecycle through S140; route blockers through S150; route preferences through S151.

No new information path introduced.

### Positive-Feedback Analysis

Not applicable. Goldens validate existing dampeners.

### Concrete Dampeners

Tested rather than introduced: queue grant capacity (scaled contention), obligation TTL (office vacancy), testimony minimum_observations (false rumor), route-blocker TTL (scaled contention).

### Stored State vs. Derived Read-Model List

No new stored state. No new derived read-model.

## SystemFn Integration

Not applicable.

## Component Registration

Not applicable.

## Cross-System Interactions

Goldens exercise integration paths between archived S143 and the remaining S148 / S150 / S151 substrate; no new cross-system interaction is introduced.

## Profile-Driven Parameters

Not applicable. Goldens use authored scenario data; no new profile fields.

## Test Plan

- Three remaining S153 golden test files with the per-block assertions above; the belief-wall trap golden is already covered by S143STABELVIE-006.
- D6 determinism regression.
- All goldens passing — `cargo test --workspace`.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
