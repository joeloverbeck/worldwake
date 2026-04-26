# S126: Need Projection and Plan Time-Budget Assumptions

## Summary

Give agents a forward horizon for their own physiology so plan adoption can ask "will this plan keep me inside my survival envelope long enough to complete it?" and revise on assumption breach. Today, `HomeostaticNeeds` carries only `current_level` per need, and `MetabolismProfile` carries the rate. The arithmetic that converts those into "tick at which hunger crosses high threshold" exists in no system — every agent reasons reactively from current pressure. This spec adds a derived projection (no new authoritative state — the projection is recomputable from `current_level + base_rate + DriveThresholds`), wires it into the planner's intention-frame as a `FrameAssumption::NeedSafeUntilTick { need, until_tick }`, and routes assumption breach through the post-S109 typed discrepancy chain that S122 already lit up. The result: an agent committing to "Sleep" or "Travel + Harvest" carries an explicit time budget against each at-risk need; if any need's projection collapses below the plan's expected completion tick (computed from the agent's current plan's `total_estimated_ticks`), the assumption fails and the planner gets a clean replan signal — the same mechanism S122 uses for `CommodityAvailableAt`.

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `FrameAssumption::NeedSafeUntilTick { need, until_tick }` variant in `intention_frame.rs`; new `Discrepancy::NeedHorizonExceeded { need, projected_breach_tick }` variant in `discrepancy.rs`; small derived helper on `HomeostaticNeeds` (`needs.rs`) that returns the projected tick at which a given need crosses a target level under base activity; keyed `rate(need)` on `MetabolismProfile` and `high(need)` on `DriveThresholds` mirroring the existing `DriveThresholds::critical(need)` pattern. No new component.
- `worldwake-ai` — `populate_assumptions` extension in `agent_tick/frame.rs` that reads each at-risk need's projection against a `plan_completion_tick: Tick` parameter the caller supplies, and pushes a `NeedSafeUntilTick` assumption per need that would breach. `evaluate_assumptions` arm that recomputes projection at evaluation time (using a new `current_tick: Tick` parameter) and returns `CriticalFailure(FrameAssumption::NeedSafeUntilTick { .. })` when the projection has collapsed. Both signature additions ripple to the ~5 call sites in `agent_tick/mod.rs` and `agent_tick/planning.rs` and the existing assumption tests in `agent_tick/frame.rs`.
- `worldwake-systems` — no new system tick. The projection helper composes existing `HomeostaticNeeds`, `MetabolismProfile`, and `DriveThresholds` reads against the agent's current tick.
- `worldwake-sim` — no new accessor. The existing `ProfileBeliefView` (`belief_view.rs:741-752`) already exposes `homeostatic_needs(agent)`, `drive_thresholds(agent)`, and `metabolism_profile(agent)`, which `RuntimeBeliefView` inherits at line 1226. The AI evaluator reads through these accessors rather than raw world component reads, preserving the FND-14A locality story.

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — **hard**. Reuses the post-correction `record_assumption_failure` path (`agent_tick/frame.rs:496-527`) by extending the flat `Discrepancy` enum with a new `NeedHorizonExceeded` variant. Archived at `archive/specs/S109-typed-discrepancy-taxonomy.md`.
- S122 (Frame Assumption — Commodity Availability) — **hard**. Reuses the `populate_assumptions` per-tick-refresh pattern and the `AssumptionEvalResult::CriticalFailure(FrameAssumption)` carrier already widened to carry the failed assumption (`agent_tick/frame.rs:214`). Archived at `archive/specs/S122-frame-assumption-commodity-availability.md`.
- S110 (Decision History Events) — **soft**. `BlockerRecorded` payload already carries the `Discrepancy`; the new `NeedHorizonExceeded` variant lands in the same observer rendering path with no new `EventTag` variant required. Archived at `archive/specs/S110-decision-history-events.md`.
- S116 (Drive Escalation) — **soft**. `DriveThresholds.critical(need)` already exists per `HomeostaticNeedId` (`drives.rs:92-100`); the new `DriveThresholds.high(need)` keyed accessor (D3 below) mirrors that pattern over the existing `ThresholdBand.high()` per-band reads (`drives.rs:46-48`). Authored escalation behavior remains the only authority on what "high" or "critical" means per need per agent. Archived at `archive/specs/S116-drive-escalation-sustained-critical.md`.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 1: "agents mostly react to current need pressure … shallow … should reason about time-to-trouble, not just need value × weight." The narrative report shows Agent A choosing sleep over apple acquisition when fatigue rises into the 300s — a reactive interruption, not a horizon decision. Three specific behavioral patterns the spec is meant to enable:

- `harvest_before_sleep` when projected hunger would breach during the planned sleep window;
- `wake_early` when another need's projected curve invalidates the sleep commitment;
- `carry_reserve_before_leaving_water` when projected thirst exceeds round-trip duration.

None of these are scriptable; they fall out of agents holding `NeedSafeUntilTick` assumptions tied to plan-step durations.

## Design Goals

1. Need projection is derived, not stored. The arithmetic is `current_level + base_rate × Δticks` against `DriveThresholds.high()` / `.critical()`. No `projected_tick_of_high` field on the component — that would violate FND-3 (concrete state over abstract scores) by promoting a recomputable view to authoritative state.
2. The assumption is per-frame and per-tick-refreshed. `populate_assumptions` derives a `NeedSafeUntilTick { need, until_tick }` for each need whose projected high-threshold crossing falls before the plan's completion tick. The completion tick is computed by the caller as `current_tick + remaining_estimated_ticks`, where `remaining_estimated_ticks` is `runtime.current_plan.total_estimated_ticks` minus the duration of any already-completed steps (the plan struct already carries `total_estimated_ticks: u32` at `crates/worldwake-ai/src/planner_ops.rs:972`). `populate_assumptions` accepts `plan_completion_tick: Tick` as a new parameter rather than reading it from a new field on `IntentionFrame` — this keeps the frame minimal and avoids a parallel authoritative copy of plan timing.
3. The evaluator recomputes projection at evaluation time. Re-derivation is FND-27-compliant (caches recomputable from source state). A perception event that updated `HomeostaticNeeds` (e.g., a `pick_up_then_eat` commit reduced hunger) shifts the projection and the assumption may transition from `CriticalFailure` back to `Holds` on the next tick. `evaluate_assumptions` accepts `current_tick: Tick` as a new parameter.
4. The assumption uses `DriveThresholds.high()` as the baseline, not `.critical()`. Treating critical as the failure threshold gives agents zero margin to choose alternative plans before the world becomes hostile. `high()` is the "comfortable upper band" that agents already use today for goal ranking; using it for assumption breach gives agents one threshold of margin to revise.
5. No new `EventTag`. The discrepancy lands through S109's existing path; observer rendering surfaces the new kind through S110's existing `BlockerRecorded` payload.
6. No CLI authored profile. Projection lives entirely on substrate the agent already carries (`HomeostaticNeeds`, `MetabolismProfile`, `DriveThresholds`, `IntentionFrame.assumptions`). No `NeedProjectionProfile` component — the per-agent variation is already encoded in `MetabolismProfile.*_rate` and `DriveThresholds.high()` per need.
7. Projection uses base rate only; per-tick re-evaluation catches activity-driven drift. Activity multipliers (e.g., `MetabolismProfile.travel_thirst_multiplier` at `needs.rs:143-149`) modify the per-tick rate during travel and labor. Computing a step-composition-weighted rate inside the projection helper would require the helper to inspect plan structure, which contradicts the goal of keeping the helper a pure arithmetic kernel. Instead, projection uses the base rate and the per-tick `evaluate_assumptions` re-run catches the actual depletion as activity advances. Assumption-breach-on-arrival (the agent reaches a node where the actual rate proves higher than the projection assumed) is acceptable feedback consistent with FND-21 — the intention is revisable, the breach is concrete surprise, and the replan path is the same as for any other `CriticalFailure`.
8. Belief-view-mediated reads. The evaluator and populator read physiology through `ProfileBeliefView` accessors (`view.homeostatic_needs(agent)`, `view.drive_thresholds(agent)`, `view.metabolism_profile(agent)`) rather than raw `world.get_component_*` calls. The agent is co-located with its body, so these reads return the same authoritative values an FND-14A-compliant perception pipeline would deliver, and going through the belief-view trait keeps the call site uniform with how every other AI consumer reads agent profiles.

## Non-Goals

- Sleep-quality recovery curve modulation. S128 owns `SleepEpisode` and its place-quality recovery; S126 only provides the time budget that lets `SleepEpisode` declare its own `WakeCondition::ProjectedNeedBreach`.
- Travel-speed or activity-cost modulation. Per-tick `base_rate` from `MetabolismProfile` is the only rate source. Activity multipliers (`travel_thirst_multiplier`, `travel_fatigue_multiplier`, `travel_bladder_multiplier`, `wilderness_relief_dirtiness_penalty`, all at `needs.rs:143-149`) are read by the metabolism system itself when applying tick-by-tick depletion; the projection helper is rate-agnostic by design (Design Goal 7).
- Probabilistic projection (e.g., 70% confidence that hunger will breach by tick T). The projection is a single deterministic value from current state; uncertainty enters through the agent's perception of its own physiology, not through a stochastic projection.
- Reserve carrying or hoarding decisions. S127 owns quantity-aware acquisition; S126 only provides the `until_tick` against which S127 can decide whether one apple suffices or three are needed.
- Predicting other agents' need horizons. FND-14 forbids reading another agent's authoritative `HomeostaticNeeds`; this spec applies only to the planning agent's own physiology.
- Adding a `BeliefStatus`-style envelope to the projection. The agent's belief about its own physiology is FND-14A-equivalent (the body is co-located with the agent, always); same-tick observation is authoritative.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Projection is derived from existing concrete state (`HomeostaticNeeds.value(need)` + `MetabolismProfile.rate(need)`); no new score component. The `until_tick` carried in the assumption is a concrete tick value, not an "urgency score." |
| FND-7 (Locality of Motion, Interaction, and Communication) | The projection reads only the agent's own physiology and per-agent profile, mediated by `ProfileBeliefView`. No global queries. Other agents' need horizons are not visible. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | The plan's time budget consumes the same per-step duration data the planner already uses for scheduling (`PlannedPlan.total_estimated_ticks`, `PlannedStep.estimated_ticks`). The new piece is matching that duration against the agent's own physiology horizon. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | The "projection breach → replan → projection breach again on new plan" loop is dampened by S109's `structural_block_ticks` TTL on the recorded discrepancy (same TTL S122 uses) plus the fact that successful action commits reduce the underlying need and shift the projection back into safe territory. |
| FND-14 (World State Is Not Belief State) | Agent reads its own `HomeostaticNeeds` via `ProfileBeliefView` (FND-14A-equivalent — the body is co-located). It does not read other agents' physiology. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | An agent's body is always co-located with itself. The same-tick belief-equivalence corollary applies to the agent's own physiology fields, and the `ProfileBeliefView` accessor surface implements that equivalence uniformly. Social facts about the body (e.g., "I am known to be sick") still require explicit belief entries. |
| FND-17 (Surprise Comes From Violated Expectation) | The `NeedSafeUntilTick` assumption *is* the agent's expectation about its own envelope. Breach is concrete surprise — the body advanced faster than the agent anticipated when planning. |
| FND-21 (Intentions Are Revisable Commitments) | Closes the gap symmetrically with S122: agents now monitor not only "the apple is at the place" but also "I will arrive in time to eat it before another need breaches." Both assumptions revise plans through the same S109 typed-discrepancy path. Activity-driven projection drift (Design Goal 7) is also revisable through the same re-evaluation cycle. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents with the same hunger weight but different `MetabolismProfile.hunger_rate` produce different projections from the same starting `current_level`. Two agents with the same metabolism but different `DriveThresholds.hunger.high()` produce different breach ticks. Diversity emerges from existing per-agent parameters. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | The recorded discrepancy from a failed projection assumption is concrete per-agent learned state with explicit acquisition (failure event), explicit decay (S109 TTL), and explicit replacement (re-evaluation when belief refreshes). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | The metabolism system writes `HomeostaticNeeds` per tick. The AI evaluator reads `HomeostaticNeeds` via `ProfileBeliefView`. No imperative call between them. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `until_tick` in the assumption is a per-tick recomputation; the assumption is rebuilt on every `populate_assumptions` call. Authoritative source remains `HomeostaticNeeds` + `MetabolismProfile` + `DriveThresholds`. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | No old "reactive-only" planning path is preserved beside the new horizon-aware one. The planner adopts the assumption uniformly; reactive interruption (motive-score-based) continues to work for needs the assumption does not gate — the two coexist by domain, not as parallel authority. |
| FND-29 (Debuggability Is a Product Feature) | Decision-trace summary names which `(need, until_tick)` failed, the projected breach tick, and the plan-completion tick. The "why did the agent abandon Travel + Harvest at tick 312?" question becomes answerable from the trace alone. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Records the discrepancy via `record_assumption_failure` with the new flat `Discrepancy::NeedHorizonExceeded { need, projected_breach_tick }` variant. The S110 `BlockerRecorded` event already names the discrepancy class, so the typed provenance reaches the event log without any new `EventTag`. |

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** All inputs to the projection are agent-local. `HomeostaticNeeds` is updated by the metabolism system (`worldwake-systems/src/needs.rs`) each tick. `MetabolismProfile` is per-agent stored state from scenario load. `DriveThresholds` is per-agent stored state. The AI evaluator reaches all three through `ProfileBeliefView` accessors (`crates/worldwake-sim/src/belief_view.rs:741-752`, inherited by `RuntimeBeliefView` at line 1226). No information needs to travel — the projection is computable from state the agent already holds.
2. **Positive-feedback analysis.** Two theoretical loops: (a) "Projection-driven replan triggers a new plan whose projection also fails immediately." Dampener: S109 `structural_block_ticks` TTL on the discrepancy plus the fact that the most-pressing need will rank highest in the next planning round. (b) "Assumption population on every tick churns the frame." Dampener: assumption is derived from frame state, the caller-supplied `plan_completion_tick`, and current physiology, idempotent across ticks for stable goals — same FND-27/FND-3 framing S122 documented.
3. **Concrete dampeners.** S109's TTL is the dampener for loop (a) and is per-agent profile-driven via `cognitive.structural_block_ticks` — no hidden cap. Loop (b) is dampened by the architecture itself (`populate_assumptions` returns a vec rebuilt each tick from current state).
4. **Stored state vs. derived read-model.** Stored: `IntentionFrame.assumptions` (already authoritative; gains the new variant). Stored (no change): `runtime.current_plan.total_estimated_ticks` (the plan-time-budget source). Derived: the per-tick `until_tick` value carried inside the variant, the per-tick `plan_completion_tick` value computed by the caller before each `populate_assumptions` call, and the boolean evaluation result. No new component or field on `HomeostaticNeeds`, `IntentionFrame`, or any other stored type.

## Deliverables

### D1: `FrameAssumption::NeedSafeUntilTick`

In `crates/worldwake-core/src/intention_frame.rs`, extend the `FrameAssumption` enum:

```rust
pub enum FrameAssumption {
    TargetAlive(EntityId),
    RouteExists { from: EntityId, to: EntityId },
    NoCriticalThreat,
    CommodityAvailableAt {
        commodity: CommodityKind,
        place: EntityId,
    },
    /// The named need is projected to remain below its high threshold
    /// at least until `until_tick`, given the agent's current level
    /// and base metabolism rate. Recomputed each tick by the evaluator.
    NeedSafeUntilTick {
        need: HomeostaticNeedId,
        until_tick: Tick,
    },
}
```

`HomeostaticNeedId` and `Tick` are already in scope from the existing imports. Both are `Copy`, preserving the enum's `Copy` derive (`intention_frame.rs:61`).

### D2: `HomeostaticNeeds::projected_tick_of` derived helper

In `crates/worldwake-core/src/needs.rs`, add a derived helper composing the existing fields:

```rust
impl HomeostaticNeeds {
    /// Projected tick at which `need` reaches `target_level` given the
    /// agent's `base_rate` for that need, starting from `current_tick`.
    /// Returns `None` if the need would never reach the target (rate is
    /// zero or current level already at or above target).
    #[must_use]
    pub fn projected_tick_of(
        &self,
        need: HomeostaticNeedId,
        target_level: Permille,
        base_rate: Permille,
        current_tick: Tick,
    ) -> Option<Tick> {
        let current = self.value(need).value();
        let target = target_level.value();
        if current >= target {
            return Some(current_tick);
        }
        let rate = base_rate.value();
        if rate == 0 {
            return None;
        }
        let delta_ticks = u64::from(target - current).div_ceil(u64::from(rate));
        Some(Tick(current_tick.0.saturating_add(delta_ticks)))
    }
}
```

The arithmetic is integer-only (per the no-floats invariant). `u16::from(target - current)` is safe because the early-return guards `current >= target`. `Permille` is bounded `0..=1000` (`numerics.rs:24`), so the subtraction cannot overflow. `u64::div_ceil` is the conservative tick projection — partial accumulation at `delta_ticks - 1` is below target, so the breach tick is `current_tick + delta_ticks`.

### D3: Keyed `rate(need)` and `high(need)` accessors

In `crates/worldwake-core/src/needs.rs`, add a keyed accessor on `MetabolismProfile`:

```rust
impl MetabolismProfile {
    #[must_use]
    pub const fn rate(&self, need: HomeostaticNeedId) -> Permille {
        match need {
            HomeostaticNeedId::Hunger => self.hunger_rate,
            HomeostaticNeedId::Thirst => self.thirst_rate,
            HomeostaticNeedId::Fatigue => self.fatigue_rate,
            HomeostaticNeedId::Bladder => self.bladder_rate,
            HomeostaticNeedId::Dirtiness => self.dirtiness_rate,
        }
    }
}
```

In `crates/worldwake-core/src/drives.rs`, add a keyed accessor on `DriveThresholds` mirroring the existing `critical(need)` (`drives.rs:92-100`):

```rust
impl DriveThresholds {
    #[must_use]
    pub const fn high(&self, need: HomeostaticNeedId) -> Permille {
        match need {
            HomeostaticNeedId::Hunger => self.hunger.high(),
            HomeostaticNeedId::Thirst => self.thirst.high(),
            HomeostaticNeedId::Fatigue => self.fatigue.high(),
            HomeostaticNeedId::Bladder => self.bladder.high(),
            HomeostaticNeedId::Dirtiness => self.dirtiness.high(),
        }
    }
}
```

Scope matches `critical(need)` — only the 5 `HomeostaticNeedId` variants. The `pain` and `danger` bands on `DriveThresholds` (`drives.rs:62-63`) remain accessible via direct field access; they are not needs in the homeostatic sense and are not subject to projection.

### D4: `populate_assumptions` extension

In `crates/worldwake-ai/src/agent_tick/frame.rs::populate_assumptions` (currently at `frame.rs:280`), add a new parameter and a new arm. Updated signature:

```rust
pub(super) fn populate_assumptions(
    frame: &IntentionFrame,
    agent: EntityId,
    view: &dyn RuntimeBeliefView,
    current_tick: Tick,
    plan_completion_tick: Tick,
) -> Vec<FrameAssumption>
```

After the existing domain-keyed assumption population (Travel/Errand/Care/Escort/Generic), append per-need projection assumptions. For each `need` in `HomeostaticNeedId::ALL`:

1. Read physiology via the belief view: `view.homeostatic_needs(agent)`, `view.metabolism_profile(agent)`, `view.drive_thresholds(agent)`. If any is `None`, skip projection for this agent (the agent is missing required profiles; reactive ranking continues to handle it).
2. Call `needs.projected_tick_of(need, thresholds.high(need), metabolism.rate(need), current_tick)`.
3. If the projection is `Some(breach_tick)` and `breach_tick < plan_completion_tick`, push `FrameAssumption::NeedSafeUntilTick { need, until_tick: plan_completion_tick }`.

The caller computes `plan_completion_tick` as `current_tick + remaining_estimated_ticks`, where `remaining_estimated_ticks` is `runtime.current_plan.total_estimated_ticks` minus the `estimated_ticks` of any already-completed steps (via `runtime.current_step_index`). Both `runtime` and `tick` are in scope at every call site (`agent_tick/mod.rs:1027`, `agent_tick/planning.rs:1680, 2114, 2751, 2847`). When `runtime.current_plan` is `None`, the caller passes `current_tick` for `plan_completion_tick`, which trivially makes every `breach_tick < current_tick` test false and skips need-horizon assumption population — there is no plan to budget against.

Update the existing call sites and the `populate_assumptions` test cases in `agent_tick/frame.rs::tests` to pass the two new arguments.

### D5: `evaluate_assumptions` arm

In the same module, extend `evaluate_assumptions` (currently at `frame.rs:339`). Updated signature adds `current_tick: Tick`:

```rust
pub(super) fn evaluate_assumptions(
    assumptions: &[FrameAssumption],
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    ranked_candidates: Option<&OrderedRanked<'_>>,
    current_tick: Tick,
) -> AssumptionEvalResult
```

Add the new arm:

```rust
FrameAssumption::NeedSafeUntilTick { need, until_tick } => {
    let (Some(metabolism), Some(needs), Some(thresholds)) = (
        view.metabolism_profile(agent),
        view.homeostatic_needs(agent),
        view.drive_thresholds(agent),
    ) else {
        // Missing profile — cannot evaluate; treat as deferred.
        has_deferred = true;
        continue;
    };
    let projected = needs.projected_tick_of(
        need,
        thresholds.high(need),
        metabolism.rate(need),
        current_tick,
    );
    match projected {
        Some(breach_tick) if breach_tick < until_tick => {
            return AssumptionEvalResult::CriticalFailure(*assumption);
        }
        _ => {}
    }
}
```

`*assumption` carries the original `NeedSafeUntilTick { need, until_tick }` forward to D7 trace surfacing — no separate `(need, breach_tick, until_tick)` tuple needs to be reconstructed. Update the existing call sites in `agent_tick/mod.rs:1028, 1213` and the `evaluate_assumptions` test cases in `agent_tick/frame.rs::tests` to pass the new `current_tick` argument.

### D6: `Discrepancy::NeedHorizonExceeded` variant

In `crates/worldwake-core/src/discrepancy.rs` (the actual discrepancy taxonomy module — not `blocker_memory.rs`, which holds `BlockerMemory`), extend the flat `Discrepancy` enum (currently 10 variants at `discrepancy.rs:6-27`):

```rust
pub enum Discrepancy {
    // ... existing 10 variants (BeliefStale, BeliefContradicted, …, PartialExecutionDrift) ...
    /// A `NeedSafeUntilTick` assumption breached: the projected tick at
    /// which the named need crosses its high threshold fell before the
    /// plan-completion tick the assumption was guarding.
    NeedHorizonExceeded {
        need: HomeostaticNeedId,
        projected_breach_tick: Tick,
    },
}
```

Both payload fields are `Copy`, preserving the enum's `Copy` derive (`discrepancy.rs:5`). The `until_tick` from the failed assumption is intentionally NOT duplicated in this payload — the trace consumer (D7) reads it from the `FrameAssumption` carried by `AssumptionEvalResult::CriticalFailure`.

Extend `record_assumption_failure` (`agent_tick/frame.rs:496-527`) so that when the failed assumption is `FrameAssumption::NeedSafeUntilTick { need, until_tick }`, it constructs `Discrepancy::NeedHorizonExceeded { need, projected_breach_tick: until_tick }` instead of falling through to `BeliefContradicted` / `PartialExecutionDrift`. The existing `frame.expected_commodity()`-based `DiscrepancyClearing::CommodityAvailabilityChanged` branch does not apply to need-horizon breaches; use `DiscrepancyClearing::TtlExpiry`. The structural-block-ticks TTL (read by the caller from `CognitiveProfile.structural_block_ticks`) is unchanged.

To wire the assumption identity into `record_assumption_failure`, add a `failed_assumption: FrameAssumption` parameter to the function (it is currently inferred from frame domain). Callers in `agent_tick/mod.rs:1048` already destructure `AssumptionEvalResult::CriticalFailure(assumption)` (`mod.rs:1039-1041`), so passing it down is a small forwarding change.

A future spec may add a richer clearing condition (e.g., `NeedLevelDecreasedBelow`) if the breach should clear when the agent eats; for S126, TTL-only matches the existing structural-block semantics and avoids inventing a new clearing arm.

### D7: Decision-trace surfacing

Extend the existing decision-trace path that S122 widened. The `CriticalFailure(FrameAssumption)` carrier already exists; the new `NeedSafeUntilTick` variant prints as `"NeedSafeUntilTick { need: Hunger, until_tick: 412 } breached: projected breach at tick 387"`. The `until_tick` is read from the assumption, and `projected_breach_tick` is read from the `Discrepancy::NeedHorizonExceeded` payload recorded in D6.

Observer rendering in `crates/worldwake-cli/src/bin/observer.rs` already consumes both surfaces (`PlanInvalidationReason::AssumptionFailed { assumption }` at `observer.rs:509` and the `BlockerRecorded` payload). The new variant lands automatically through the existing `Display`/`Debug` paths; the only additive change is a focused match arm where free-text trace summaries are produced.

### D8: Golden coverage

Add a survival-golden scenario that exercises the path:

- Agent with high hunger rate adopts a multi-step plan whose completion tick exceeds the projected hunger-high crossing.
- Confirm `populate_assumptions` (with the new `plan_completion_tick` argument) adds the `NeedSafeUntilTick` for hunger.
- Confirm `evaluate_assumptions` (with the new `current_tick` argument) returns `CriticalFailure(FrameAssumption::NeedSafeUntilTick { need: Hunger, .. })` after the plan starts.
- Confirm the recorded `Discrepancy::NeedHorizonExceeded { need: Hunger, projected_breach_tick }` suppresses the original goal for `structural_block_ticks`.
- Confirm a shorter-completion alternative plan (e.g., harvest-first then sleep) wins the next ranking round.

Place under `crates/worldwake-ai/tests/golden_need_projection.rs`. Reuse `survival-baseline.ron` topology if the projection breach can be reproduced under a tighter `MetabolismProfile.hunger_rate`; otherwise author a focused `survival-need-projection.ron`. If the existing golden harness lacks helpers for asserting on `evaluate_assumptions` results or `DiscrepancyMemory` contents, add minimal helpers under `crates/worldwake-ai/tests/golden_harness/` (or the equivalent tests-support module) so the new test reads cleanly and the helpers remain reusable for any later assumption-driven golden.

## SystemFn Integration

No new system tick function. The metabolism system (`crates/worldwake-systems/src/needs.rs`) already updates `HomeostaticNeeds` per tick. `populate_assumptions` and `evaluate_assumptions` run inside the existing per-agent AI tick loop — no scheduling change. The two new parameters (`current_tick` on both functions, `plan_completion_tick` on `populate_assumptions`) are computed at call sites that already hold `tick` and `runtime`.

## Component Registration

No new components. The new `FrameAssumption` variant lives inside the existing `IntentionFrame.assumptions: Vec<FrameAssumption>` storage. The new `Discrepancy` variant lives inside the existing `DiscrepancyMemory` storage. Per FND-22 Section 5 of `docs/spec-drafting-rules.md`, no new agent profile is added because per-agent variation already lives in `MetabolismProfile` and `DriveThresholds`.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Metabolism (`worldwake-systems/src/needs.rs`) | Writes `HomeostaticNeeds`; AI projector reads it via `ProfileBeliefView` | State-mediated |
| Drive escalation (S116) | Writes `DriveThresholds`-equivalent escalation state (already integrated with motive scoring); projector reads `DriveThresholds.high(need)` via the new keyed accessor | State-mediated |
| Discrepancy memory (S109, `worldwake-core/src/discrepancy.rs`) | AI evaluator records `Discrepancy::NeedHorizonExceeded { need, projected_breach_tick }` via `record_assumption_failure`; planner reads suppression via `DiscrepancyMemory::is_suppressed` | State-mediated |
| Decision history (S110) | `BlockerRecorded` event payload carries the new discrepancy variant; observer renders it through existing `Display`/`Debug` paths | State-mediated |
| S122 (CommodityAvailableAt) | Sibling `FrameAssumption` variant; share the `populate_assumptions`/`evaluate_assumptions` infrastructure and the `record_assumption_failure` recording path | Co-evolution; no direct call |

## Profile-Driven Parameters

Per FND-22, all parameters that vary per agent are sourced from existing profiles:

- Need rate per tick: `MetabolismProfile.{hunger,thirst,fatigue,bladder,dirtiness}_rate`, accessed via the new `MetabolismProfile::rate(need)` keyed helper.
- High threshold per need: `DriveThresholds.{hunger,thirst,fatigue,bladder,dirtiness}.high()`, accessed via the new `DriveThresholds::high(need)` keyed helper.
- Suppression TTL: `CognitiveProfile.structural_block_ticks` (consumed by S109's discrepancy memory through `record_assumption_failure`).

No magic numbers introduced.
