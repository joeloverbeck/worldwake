# S122: Frame Assumption — Commodity Availability

## Summary

Close the FND-21 / FND-15 / FND-17 feedback gap that prevents agents from revising commodity-acquisition intentions when local observation contradicts the belief that justified them. `FrameAssumption::CommodityAvailableAt { commodity, place }` is already declared in `crates/worldwake-core/src/intention_frame.rs:69`, but `populate_assumptions` never adds it to any frame and `evaluate_assumptions` stubs it as always-true ("future work" comment at `crates/worldwake-ai/src/agent_tick/frame.rs:278`). With the assumption inert, an agent whose plan is `Travel(P) → pick_up(L)` for an apple lot `L` at place `P` has no architectural path to discover that `L` is empty, missing, or inaccessible: the plan completes vacuously, the next tick re-plans the same broken plan, and the agent oscillates indefinitely. This spec implements `CommodityAvailableAt` end to end — population from the goal context at frame establishment, evaluation against the agent's belief store with FND-14A same-tick co-located perception, integration with the existing assumption-failure routing landed by S109's TYPDISTAX-004 correction, and validation through unit, integration, and survival-golden coverage.

## Phase and Status

Phase 8 Adjunct: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-ai` — `populate_assumptions` extension to add `CommodityAvailableAt` from `IntentionDomain::Travel` frames whose committed goal is `AcquireCommodity`; `evaluate_assumptions` implementation reading the agent's belief store and FND-14A co-located perception
- `worldwake-sim` — read-only belief-view accessor that answers "does the agent believe there is at least one accessible source of `commodity` at `place`?" without leaking ownership/access semantics that FND-14A explicitly excludes
- `worldwake-core` — no schema change (`FrameAssumption::CommodityAvailableAt` already exists). One added derived helper for `IntentionFrame::expected_commodity` that surfaces the goal-derived `(commodity, place)` pair when the frame's domain is `Travel { destination }` and the committed goal is an `AcquireCommodity` variant. No new component.

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — **landed** in this PR. The post-correction `record_assumption_failure` (TTL=`structural_block_ticks`, `TtlExpiry` clearing) is the suppression path this spec relies on. Hard.
- S110 (Decision History Events) — soft. When `EventTag::AssumptionFailed { kind: CommodityAvailableAt, ... }` lands in the authoritative event log, S122's failures are inspectable through that channel. S122 records the discrepancy with or without S110's event variant; the event variant is additive.
- S113 (Belief Envelope) — soft. If `BeliefValue<T>` envelope lands first, the evaluator can read confidence directly; if not, the evaluator falls back to existing crisp-value belief queries. The two variants land identical FOUNDATIONS-aligned semantics.
- S114 (Plan Step Guards) — independent. S114 introduces step-level guards; S122 introduces frame-level assumptions. They share the discrepancy-routing surface but do not collide on the assumption type.

## Motivating Evidence

Confirmed by the in-PR diagnostic (`golden_survival_baseline.rs::diagnostic_agent_b_hunger_trajectory`, removed before merge) under `survival-baseline.ron` seed 104004:

- Agent B last successfully eats at tick **578**. After tick 600 the agent enters a Camp ↔ Fertile Fields oscillation: every 3 ticks `Committed(travel)` fires alternately at slot 0 and slot 1, and the next-step `pick_up` never commits across a 591-tick window.
- The selected plan is consistently `Travel(travel) → MoveCargo(pick_up), terminal=GoalSatisfied` for `AcquireCommodity { commodity: Apple, purpose: SelfConsume }` at place slot 1, score 616500, `feasibility=Uncertain`, `expansions=2`.
- The plan completes vacuously: terminal-kind `GoalSatisfied` fires when the plan reaches its end regardless of whether the agent actually acquired anything. `apple_inv = 0` throughout the failure window.
- Re-planning every 5–10 ticks regenerates the same goal, the search finds the same cheap broken plan, and the cycle continues until world drift (other agents harvesting, an unrelated belief change) eventually shifts the ranking enough that a different plan wins. In `survival-baseline.ron` this took ~190 ticks even with the corrected post-S109 suppression — too long for the authored 100-tick critical-run contract.

The architectural gap: the agent's `IntentionFrame` for "Travel to Fertile Fields and pick up apple" carries only `FrameAssumption::RouteExists`, which holds throughout the failure (the route is fine — the apple isn't there). No assumption captures "I expect an accessible apple source at the destination." Without that assumption, no `evaluate_assumptions` call can return `CriticalFailure`, no `record_assumption_failure` fires, and the post-S109 200-tick suppression that would let the agent consider Harvest+pick_up+eat instead is never created.

This violates FND-21 directly: *"Agents must monitor the assumptions beneath an active intention and suspend, revise, or replace that intention when new local evidence invalidates it."* The assumption *type* exists; the assumption *instance* is never present on any frame.

## Design Goals

1. Agents whose plan depends on a commodity being accessible at a place must hold that as an explicit assumption, evaluable from the agent's own belief state.
2. Local observation that contradicts the assumption (the agent arrived at the place and perceived no accessible source of the commodity) must transition the assumption to `CriticalFailure`, triggering the post-S109 typed discrepancy with `structural_block_ticks` suppression.
3. The evaluator reads only the agent's belief store and FND-14A same-tick co-located perception of physical properties (item-lot commodity, resource-source presence). It does not read social/relational facts (ownership, effective rights, jurisdiction, possessor identity) — those require explicit belief entries per FND-14A.
4. Suppression is the dampener (FND-11). The `structural_block_ticks` TTL is the physical timeout; clearing happens through TTL expiry or through new authoritative belief evidence about the place. No invisible cap, no cliff.
5. The assumption is established when the frame is established and persists across the frame's lifetime. It is not re-derived on every tick from the goal — that would fail FND-3 ("derived summaries are caches, never truth").
6. The evaluator generalizes across the existing acquisition surfaces: pickup of open lots, harvest from resource sources. Container-bound goods and seller-mediated commodities are out of scope (deferred to a follow-up when seller-mediated frames exercise the path).
7. No new authoritative state. `FrameAssumption::CommodityAvailableAt` is already in the type system; this spec wires it to its source (population) and sink (evaluation).

## Non-Goals

- Container-stored or seller-listed commodities. Those acquisition paths involve ownership, access rights, and trade preconditions that go beyond FND-14A's same-tick co-located perception. A follow-up spec extends `CommodityAvailableAt` to those cases when concrete scenarios exercise them.
- Probabilistic acceptance-of-failure (e.g., "the apple might be there, give it 70% confidence"). The assumption is binary against the agent's current belief snapshot; uncertainty surfaces through `BeliefStatus::Stale` / `Disputed` upstream of evaluation, not through the assumption itself.
- Per-action belief invalidation hooks on the action handler side (e.g., a `pick_up` handler that emits "this lot was empty" on commit-with-zero-quantity). The assumption-evaluation loop already runs at the start of every tick the frame is active; if the agent's belief about the place is updated by perception on arrival, the assumption fails on the next tick. A handler-side hook is additive optimization, not part of the architectural fix.
- Refactoring `FrameAssumption` to a richer ADT. The existing variant set covers the four cases that exist; this spec adds wiring, not types.
- Replacing the `FrameAssumption::CommodityAvailableAt`-related "future work" stub for cases other than pure local perception. Only the FND-14A-allowed surface is implemented here.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-1 (Maximal Emergence Through Local Causality) | Agent's plan revision arises from local observation contradicting prior belief — not from an authored override. The assumption-failure → suppression → alternative-plan-selection chain is the same generic mechanism for every acquisition goal. |
| FND-3 (Concrete State Over Abstract Scores) | The assumption is a concrete enum variant with concrete `(commodity, place)` fields. Evaluation reads concrete belief entries (item-lot commodity tags, resource-source presence). No "availability score" abstraction. |
| FND-7 (Locality of Motion, Interaction, and Communication) | Evaluation is per-agent against per-agent belief. No global queries. The agent only knows what its perception pipeline + testimony has put in its belief store. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | The "agent revisits broken plan → records assumption failure → suppression times out → revisits" loop is dampened by the post-S109 `structural_block_ticks` TTL on the recorded discrepancy and by the authoritative belief refresh that occurs when the agent re-perceives the place. Not a numeric cap on retries. |
| FND-14 (World State Is Not Belief State) | Evaluator reads the agent's `AgentBeliefStore`, never authoritative world state for entities outside the agent's current place. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | When the agent is co-located with the place, the evaluator may read item-lot commodity/quantity and resource-source presence directly through the FND-14A-allowed surface. Social facts (ownership, jurisdiction) remain belief-backed. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | The "no apple here" knowledge enters the agent through the existing perception system on arrival at the place. The assumption-evaluator consumes that knowledge; it does not create knowledge. |
| FND-16 (Ignorance / Uncertainty / Contradiction First-Class) | An agent that has *never* perceived the place returns `Deferred` (cannot evaluate); an agent whose belief is `Stale` returns `Deferred` until perception refreshes. An agent whose fresh belief contradicts the assumption returns `CriticalFailure`. Three first-class outcomes. |
| FND-17 (Surprise Comes From Violated Expectation) | The assumption *is* the agent's expectation. The mismatch event is the surprise. The recorded `Discrepancy::BeliefContradicted` is the audit trail. |
| FND-21 (Intentions Are Revisable Commitments) | Closes the gap directly: agents now monitor a concrete commodity-availability assumption and suspend the intention when local evidence invalidates it. The post-S109 `record_assumption_failure` records the typed discrepancy that suppresses the goal long enough for alternatives to win ranking. |
| FND-22 (Agent Diversity Through Concrete Variation) | The evaluator reads each agent's own belief store and (S113-aware) confidence threshold. Agents with different perception fidelity, observation buffer capacity, or staleness penalties evaluate the same place differently. No homogenization. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | The recorded discrepancy from a failed assumption is concrete per-agent learned state with explicit acquisition (failure event), explicit decay (TTL), and explicit replacement (re-evaluation on next belief refresh). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Perception writes belief state. AI reads belief state. No imperative call from perception to AI. The assumption evaluator is a pure read-side computation over belief state plus FND-14A perception. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | The `bool` returned by the evaluator is a per-tick computation. Nothing is cached across ticks. The authoritative state is the belief store. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The current "stub returns always-true" code path is removed, not preserved alongside the real implementation. |
| FND-29 (Debuggability Is a Product Feature) | The decision trace records why an assumption failed (which `(commodity, place)`, what the belief said, and which discrepancy class was recorded). The "why did the agent stop pursuing the apple at Fertile Fields?" question becomes answerable from the trace alone. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Recording the discrepancy emits `EventTag::BlockerRecorded` (S110) with the assumption identity and contradicting belief. The assumption failure is a historical event, not a transient log line. |

## FND-01 Section H: Causal Hooks

1. **Information-path analysis.** The "apple is not at place P" knowledge arises only when the agent perceives P. Perception runs in the existing `perception_system` (`crates/worldwake-systems/src/perception.rs`) and writes the agent's `AgentBeliefStore`. The S122 evaluator reads the belief store. There is no out-of-band channel; if the agent has not perceived P recently enough, the evaluator returns `Deferred` and the assumption neither fails nor passes. Cross-agent propagation of "the orchard is empty" requires the existing `ShareBelief` / report path — S122 does not add a new propagation channel.
2. **Positive-feedback analysis.** Two loops are theoretically possible. (a) "Failed assumption → suppressed goal → goal expires from suppression → re-attempted → fails again." Dampener: the `structural_block_ticks` TTL on the recorded discrepancy, plus the perception refresh that occurs when the agent re-visits P. (b) "Assumption establishment → frame churn." The assumption is established once at frame creation and persists for the frame's lifetime; re-deriving on every tick would couple to FND-3 violations. Dampener: assumption populated from goal context at frame establishment only.
3. **Concrete dampeners.** The TTL is the dampener for loop (a). It is per-agent profile-driven via the existing `cognitive.structural_block_ticks` field — no hidden cap. Loop (b) is dampened by the architecture itself (assumptions are stored on the frame, not re-derived).
4. **Stored state vs. derived read-model.** Stored state: `IntentionFrame.assumptions` (already authoritative; `FrameAssumption::CommodityAvailableAt` is added to it on frame creation). The `AgentBeliefStore` continues to be the authoritative belief surface; this spec adds no new component. Derived read-model: the `bool`/`AssumptionEvalResult` returned by `evaluate_assumptions` is a pure read-side computation per tick, not stored.

## Deliverables

### D1: `IntentionFrame::expected_commodity` derived helper

In `crates/worldwake-core/src/intention_frame.rs`, add a derived (non-stored) accessor:

```rust
impl IntentionFrame {
    /// If this frame's committed goal expects to acquire a commodity at a
    /// known destination, surface the `(commodity, place)` pair the
    /// downstream `populate_assumptions` should turn into a
    /// `CommodityAvailableAt` assumption. Returns `None` for goals or
    /// domains where commodity availability is not the relevant assumption
    /// (Care, Escort, non-acquisition Travel, Generic).
    pub fn expected_commodity(&self) -> Option<(CommodityKind, EntityId)> {
        let destination = match self.domain {
            IntentionDomain::Travel { destination } => destination,
            _ => return None,
        };
        match self.goal.kind {
            GoalKind::AcquireCommodity { commodity, .. } => Some((commodity, destination)),
            _ => None,
        }
    }
}
```

This is a pure derived view over already-stored fields (FND-27). It is *not* called per tick in evaluation; `populate_assumptions` calls it once at frame establishment.

### D2: `populate_assumptions` extension

In `crates/worldwake-ai/src/agent_tick/frame.rs::populate_assumptions`, the `IntentionDomain::Travel { destination }` arm gains a `CommodityAvailableAt` assumption when the committed goal expects a commodity:

```rust
IntentionDomain::Travel { destination } | IntentionDomain::Errand { destination } => {
    let mut assumptions = Vec::new();
    if let Some(from) = current_place {
        assumptions.push(FrameAssumption::RouteExists { from, to: destination });
    }
    // S122: add commodity-availability assumption when the frame serves an
    // acquisition goal. The frame parameter is added below; the populate
    // signature is widened minimally to accept the active goal context.
    if let Some((commodity, place)) = expected_commodity_for_domain(domain, active_goal) {
        assumptions.push(FrameAssumption::CommodityAvailableAt { commodity, place });
    }
    assumptions
}
```

The current signature `populate_assumptions(domain, agent, view)` is widened to also accept the active `GoalKey` (or, equivalently, an `&IntentionFrame` from the caller post-D1). The Care / Escort / Generic arms are unchanged.

### D3: `evaluate_assumptions` implementation for `CommodityAvailableAt`

Replace the "stubbed as always-true (future work)" arm in `evaluate_assumptions` with an actual evaluator:

```rust
FrameAssumption::CommodityAvailableAt { commodity, place } => {
    match assess_commodity_availability(view, agent, commodity, place) {
        AvailabilityVerdict::Believed => continue,
        AvailabilityVerdict::Refuted => return AssumptionEvalResult::CriticalFailure,
        AvailabilityVerdict::UnknownOrStale => has_deferred = true,
    }
}
```

`assess_commodity_availability` is a private helper that:

1. If the agent is co-located with `place` (FND-14A applies), reads authoritative-but-perception-equivalent state for the place's item lots and resource sources of `commodity`. If at least one is present and accessible-by-perception (open ground lot, viable resource source not blocked), returns `Believed`. If the place was perceived this tick AND no source of `commodity` is present, returns `Refuted`.
2. If the agent is not co-located, reads the agent's `AgentBeliefStore` for `place`. If the belief store records at least one `BelievedEntityState` whose `last_known_place == Some(place)` and whose `resource_source.commodity == Some(commodity)` OR `commodity == Some(commodity)` for an item lot, with confidence above the agent's `claim_confidence_threshold`, returns `Believed`.
3. If the place is in the belief store but no entry contradicts or supports the assumption, returns `UnknownOrStale` (will defer until next perception updates).
4. If the agent has no belief about `place` at all, returns `UnknownOrStale`.

`UnknownOrStale` deferring is correct per FND-16 — the agent is allowed to retain an intention based on stale belief; the assumption fails only when the agent has *fresh, refuting* evidence.

### D4: Belief-view accessor for "accessible commodity source at place"

In `crates/worldwake-sim/src/belief_view.rs`, add a read-only accessor on the appropriate sub-trait (likely `EconomicBeliefView`):

```rust
/// Returns true if the agent believes there is at least one accessible
/// source of `commodity` at `place` — an item lot directly believed to be
/// at `place` (open ground or container the agent can perceive lawfully)
/// or a viable resource source for `commodity` at `place`. FND-14A applies
/// when the agent is co-located with `place`: same-tick perception of
/// item-lot commodity, resource-source presence, and workstation tags is
/// belief-equivalent. Social/relational facts (ownership, custody, access
/// rights) are excluded; this accessor cannot answer "is the agent
/// allowed to take it" — that requires explicit social belief entries.
fn believes_commodity_accessible_at(
    &self,
    agent: EntityId,
    commodity: CommodityKind,
    place: EntityId,
) -> bool;
```

The implementation lives in `PerAgentBeliefView` / `RuntimeBeliefView` and consumes the existing `entities_at`, `item_lot_commodity`, and `resource_sources_at` accessors. The default trait implementation returns `false` so test mocks need not implement it.

### D5: Wire D1–D4 through `evaluate_assumptions`

The existing `evaluate_assumptions` signature is widened to accept the agent identity (it already has `view`). The new arm calls `assess_commodity_availability` which calls `view.believes_commodity_accessible_at(agent, commodity, place)` for the not-co-located case and reads the same accessor for the co-located case (where FND-14A already collapses world-state and belief-equivalent perception).

`AvailabilityVerdict` is a private enum local to `frame.rs`:

```rust
enum AvailabilityVerdict {
    Believed,
    Refuted,
    UnknownOrStale,
}
```

### D6: Decision trace surface

The existing `FrameTransitionTrace` and `PlanningPipelineTrace::frame_transition` already carry assumption-failure transitions. S122 adds the `(commodity, place)` payload to the trace summary so `decision_outcome.summary()` answers "the agent abandoned this plan because it now believes there is no Apple at Fertile Fields." This is purely a trace-string extension; no new field on `IntentionFrame`.

### D7: Removal of "future work" stub

The `// CommodityAvailableAt is stubbed as always-true (future work).` comment and the corresponding always-true arm in `evaluate_assumptions` are deleted, not aliased. FND-28: dead paths leave with the live path's arrival.

## SystemFn Integration

No new tick-phase SystemFn. `evaluate_assumptions` already runs once per tick per agent during the agent-tick decision phase (in `process_agent`, after observation/reconciliation, before plan selection). S122 only widens the body of one match arm in that already-running function and adds one trait method on the read-side.

## Component Registration

No new ECS component. `FrameAssumption::CommodityAvailableAt` is a value variant inside the existing `IntentionFrame::assumptions: Vec<FrameAssumption>`. `IntentionFrame` is already registered as a runtime-generated agent component (it emerges from simulation, not configuration — exempt from the §5 scenario contract per `docs/spec-drafting-rules.md`).

`CognitiveProfile` gains no new field. The TTL for the recorded discrepancy uses the existing `structural_block_ticks` (the post-S109 correction routes assumption failures through that bucket).

`PerceptionProfile`, `AgentBeliefStore` — unchanged. The `claim_confidence_threshold` field on `PerceptionProfile` is already used by other belief-store readers; this spec consumes it for the not-co-located confidence gate.

## Cross-System Interactions

- **Perception ↔ assumption evaluation**: Perception writes the agent's `AgentBeliefStore` from local observation. The S122 evaluator reads that store. State-mediated; no direct call. (FND-26.)
- **Frame establishment ↔ assumption population**: `update_frame_for_adopted_plan` (in `frame.rs`) creates the new `IntentionFrame`. `populate_assumptions` is called during establishment. The `expected_commodity` derived helper is the pure-read bridge between goal and assumption.
- **Assumption failure ↔ discrepancy memory**: `record_assumption_failure` (post-S109 correction) records the typed `Discrepancy::BeliefContradicted` (target present) or `PartialExecutionDrift` (target absent) with TTL=`structural_block_ticks` and `TtlExpiry` clearing. State-mediated through `DiscrepancyMemory`.
- **Discrepancy memory ↔ candidate generation**: Existing post-S109 path. `find_matching_suppression` and `goal_is_suppressed` already consult `DiscrepancyMemory` when filtering candidates. No new wiring.
- **S110 event log ↔ assumption failure**: When `EventTag::AssumptionFailed` lands (S110 follow-up), the assumption identity and refuting belief are recorded. S122 emits the discrepancy regardless; the event variant is additive.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `structural_block_ticks` | `CognitiveProfile` | `u32` | 200 | Unchanged. Consumed by `record_assumption_failure` (post-S109) for failed-assumption suppression duration. |
| `claim_confidence_threshold` | `PerceptionProfile` | `Permille` | varies | Unchanged. Consumed by `assess_commodity_availability` for not-co-located belief queries. |

No new profile fields. No magic numbers.

## Validation and Falsification

### Unit tests (`crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]`)

1. **Population happy path.** Construct an `IntentionFrame` with `domain: Travel { destination: P }` and `goal: AcquireCommodity { commodity: Apple, ... }`. Call `populate_assumptions`. Assert the returned `Vec<FrameAssumption>` contains both `RouteExists { from, to: P }` and `CommodityAvailableAt { commodity: Apple, place: P }`.
2. **Population skips non-acquisition Travel.** Construct an `IntentionFrame` with `domain: Travel { destination: P }` and `goal: ExploreLocation { ... }`. Assert the returned vec contains only `RouteExists`.
3. **Population skips non-Travel domains.** Construct frames with `Care`, `Escort`, `Generic`. Assert no `CommodityAvailableAt` present.
4. **Evaluator returns `CriticalFailure` when refuted.** Mock view: agent co-located with `P`; no item lots of `commodity` at `P`; no resource sources of `commodity` at `P`. Assumption fails.
5. **Evaluator returns `AllPass` when believed.** Mock view: agent co-located with `P`; one accessible item lot of `commodity` at `P`. Assumption passes.
6. **Evaluator returns `Deferred` when unknown.** Mock view: agent not co-located with `P`; belief store has no entry for `P`. Assumption defers.
7. **Evaluator returns `Deferred` when stale.** Mock view: agent not co-located; belief store has stale entry below `claim_confidence_threshold`. Assumption defers.
8. **Co-location resource-source case.** Mock view: agent co-located with `P`; `resource_sources_at(P, commodity)` returns one viable source (orchard). Assumption passes.

### Integration tests (`crates/worldwake-ai/src/agent_tick/tests.rs`)

9. **Failure-to-suppression path.** Establish an agent with belief that an apple lot exists at `P`. Spawn the agent at a different place. Adopt a `Travel(P) → pick_up` plan. After establishment, mutate world state to remove the apple lot from `P`. Step the agent until it arrives at `P`. Assert that within one tick of arrival, `evaluate_assumptions` returns `CriticalFailure`, `record_assumption_failure` records a `BeliefContradicted` discrepancy at `(goal, P, lot, None)` with TTL=`structural_block_ticks`, and the agent's frame is cleared with `FrameClearReason::AssumptionFailed`.
10. **Suppression prevents immediate re-adoption.** Continue the run from #9. Assert that for the next `structural_block_ticks` ticks the agent does not re-adopt the same `(goal, P)` opportunity. Assert the agent considers and adopts an alternative plan (Harvest at the same place if a resource source exists, or Travel to a different place with a known source).
11. **Stale belief defers, fresh refutation fails.** Establish an agent with a stale belief about an apple lot at `P` (recorded long enough ago that the `BeliefStatus` would be `Stale` under the agent's confidence policy). Adopt a `Travel(P) → pick_up` plan. Assert assumption defers (intention persists). Step the agent until co-location at `P`. Assert assumption fails on the first co-located tick.

### Migration tests

12. **Existing assumption coverage unchanged.** All pre-S122 unit tests for `populate_assumptions` and `evaluate_assumptions` (Care domain, RouteExists, NoCriticalThreat, TargetAlive) continue to pass without modification.
13. **Stub removal.** Compile-time check: the `// CommodityAvailableAt is stubbed as always-true (future work).` comment is gone. No alias function. (Verified by absence at the line cited; a doc-test is not necessary.)

### Golden-test extension

14. **Survival baseline / contested / scattered all pass within their authored contracts.** With S109's TYPDISTAX-004 corrections (already landed) plus S122's assumption wiring, the survival baselines stay within their `max_authored_critical_run_ticks` bounds. The Camp ↔ Fertile Fields oscillation observed in the diagnostic test is resolved because the agent's frame for "AcquireCommodity Apple at Fertile Fields" gains the assumption that fails on first arrival when the apple lot has been depleted (or never existed). The `structural_block_ticks` suppression then forces the agent to consider the Harvest plan at the same place's orchard resource source, which is the intended chain.
15. **Explorer-discovers-food-source remains green.** The `explorer_discovers_food_source` test in `golden_survival_baseline.rs` already passes after the S109 corrections; S122 must not regress it. Coverage already in place.

### Falsification probes

16. **No-assumption-loss invariant.** Add an assertion harness that sweeps every plan adoption in the survival-baseline run and checks: every `IntentionFrame` whose committed goal is `AcquireCommodity` has a `CommodityAvailableAt` assumption present. Failure = silently dropped assumption. (Lives in the survival-golden harness as an opt-in validator.)
17. **No-spurious-failure invariant.** Sweep every `record_assumption_failure` call in the run and check: each call corresponds to a frame that genuinely had a refuting belief or co-located perception at the failure tick. Failure = the evaluator over-eagerly refutes a plausible assumption. Probe runs in the integration-test layer with a tracing harness.
18. **No-deferred-frozen-frame invariant.** A frame that holds `CommodityAvailableAt` and never co-locates with its `place` (because something else interrupts the travel) must not block forever. Verify that the existing `IntentionFrame::patience_limit` / `stalled_ticks` mechanism still tears down such frames; S122 does not introduce a new stuck-frame failure mode.

## Outcome (target)

What changes:

- `FrameAssumption::CommodityAvailableAt` becomes a live, evaluable, failable assumption rather than a declared-but-inert variant.
- Agents whose plan depends on commodity availability at a specific place revise the intention when local perception refutes the belief. The post-S109 typed-discrepancy suppression (TTL=`structural_block_ticks`, `TtlExpiry` clearing) gives the agent enough breathing room to consider alternative plans.
- The Camp ↔ Fertile Fields oscillation observed in the survival baseline (reported in S109's post-merge correction notes) resolves architecturally rather than through TTL band-aids or contract widening.

What does *not* change:

- The typed-taxonomy design from S109 stays as-is.
- `CognitiveProfile`, `PerceptionProfile`, `IntentionFrame` schemas stay as-is. No new fields, no new components.
- Container-bound and seller-mediated commodities remain on the prior path (unchanged); a follow-up spec extends `CommodityAvailableAt` to those when concrete scenarios surface them.

Verification target:

- `cargo test --workspace` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1` green.
- `cargo test --release -p worldwake-ai --test golden_survival_contested -- --ignored --test-threads=1` green.
- `cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1` green.
- All 18 validation items above covered.

## Decomposition Hint (for `/spec-to-tickets`)

A reasonable ticket split, in implementation order:

- **S122FRMASMCAVL-001**: D1 (`IntentionFrame::expected_commodity`) + D4 (belief-view accessor `believes_commodity_accessible_at`). Pure-read substrate, no wiring. Unit coverage for accessor across co-located / not-co-located / belief-only paths.
- **S122FRMASMCAVL-002**: D2 (population in `populate_assumptions`) + D7 (stub removal). Unit coverage for population happy path, non-acquisition skip, non-Travel skip.
- **S122FRMASMCAVL-003**: D3, D5 (evaluation arm with `AvailabilityVerdict`). Unit coverage for the four verdict cases (Believed / Refuted / UnknownOrStale / co-location resource-source). Integration test #9 (failure-to-suppression).
- **S122FRMASMCAVL-004**: D6 (decision-trace summary extension) + integration tests #10, #11. Survival-baseline / contested / scattered re-run; if green, the golden CI gate clears.
- **S122FRMASMCAVL-005**: Falsification probes #16, #17, #18. The no-assumption-loss and no-spurious-failure invariants land as opt-in validators in the survival-golden harness.

The implementing agent must reassess the spec and these ticket boundaries against the live codebase before starting (per `docs/precision-rules.md` and the per-ticket reassessment rule), and may rebalance the split if the actual code surface differs from the spec assumptions.
