# S128: Sleep Episodes and Place-Quality Recovery

## Summary

Replace the per-tick re-commit pattern that produces 143–146 separate `sleep` actions per agent across a 1440-tick run with a duration-bearing sleep episode that holds an explicit recovery curve, place-quality inputs, and wake conditions integrating with S126's need-projection assumptions. Today, `tick_sleep` runs every tick and the planner re-selects sleep again; the narrative report flags this as a benign `sleep → sleep` loop artifact, but the deeper issue is that sleep cannot have intent: an agent cannot decide "sleep until fatigue is below comfort" or "sleep until thirst projection breaches" because there is no episode-level state to carry that intent. This spec adds `SleepEpisode` as a per-agent runtime component populated when sleep starts and torn down on a `WakeCondition` firing. Place tags drive a per-place sleep-quality modifier on the recovery curve (giving Hillside Shelter, Forest Clearing, and Riverside Camp authored differentiation without new place kinds), and per-place sleep candidates let agents prefer the best believed sleep site rather than collapsing to the current location. Interrupted sleep — woke early because hunger projection breached — produces partial-recovery aftermath (PR-11) instead of failing. New `EventTag::SleepEpisodeStarted` / `EventTag::SleepEpisodeEnded` make the episode a first-class causal record (PR-12 sleep events folded in here, not a standalone PR-12 spec).

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `SleepEpisode` runtime component (per-agent, populated on sleep start, removed on wake), `WakeCondition` enum, `SleepQualityProfile` per-place component (universal default applied at `spawn_place` via `unwrap_or_default()`), `SleepRecoveryCurve` derived helper, two new `EventTag` variants, new `MetabolismProfile.min_sleep_ticks` field, new `DurationExpr::Variable { min, max }` variant in `worldwake-sim`'s shared semantics import.
- `worldwake-sim` — new `DurationExpr::Variable { min, max }` variant in `action_semantics.rs`; new `GoalBeliefView::place_sleep_quality_profile(agent: EntityId, place: EntityId) -> SleepQualityProfile` accessor in `belief_view.rs` with `RuntimeBeliefView` impl and `impl_goal_belief_view!` forwarding.
- `worldwake-systems` — `sleep` action handler refactor: start populates `SleepEpisode` with the bound place's `SleepQualityProfile`, tick consumes the recovery curve (modulated by place quality), commit removes `SleepEpisode` and emits the end event. Wake-condition evaluation runs once per tick within the sleep tick handler.
- `worldwake-ai` — sleep candidate emission becomes per-place: one candidate per believed sleep-eligible place, each carrying the place anchor. Ranking reads the anchored place's `SleepQualityProfile` through `GoalBeliefView` and incorporates `recovery_modifier` so higher-quality places outrank lower-quality ones. Wake-condition synthesis reads S126's `NeedSafeUntilTick` projection to populate `WakeCondition::ProjectedNeedBreach`.
- `worldwake-cli` — `PlaceDef.sleep_quality` optional field; `spawn_place` always calls `set_component_sleep_quality_profile(place_id, sleep_quality.map(Into::into).unwrap_or_default())` so every place carries the component.

## Dependencies

- S126 (Need Projection and Plan Time-Budget Assumptions) — **soft**. `WakeCondition::ProjectedNeedBreach` queries the `NeedSafeUntilTick` evaluator. S126 has landed (`✅ COMPLETED`); `FrameAssumption::NeedSafeUntilTick` is in production at `crates/worldwake-ai/src/decision_trace.rs:2072`, so wake-on-projection is unconditionally available. The dep stays soft because S128 does not modify S126's surface — it only consumes it.
- S109 (Typed Discrepancy Taxonomy) — **soft**. Interrupted-sleep partial recovery does not record a discrepancy by itself; if the wake was triggered by an S126 projection breach, that breach already records its own discrepancy and S128 piggybacks.
- S110 (Decision History Events) — **hard**. The new `EventTag::SleepEpisodeStarted` and `EventTag::SleepEpisodeEnded` follow the S110 decision-history event surface (`worldwake-core/src/decision_event_payload.rs`).
- S116 (Drive Escalation) — **soft**. Drive escalation already modulates motive scoring for sleep selection; this spec does not change that surface, only what happens once sleep is selected.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 3: "Sleep is the clearest mechanical smell in the report. Agents committed sleep 143–146 times each, and Agent A produced repeated `sleep → sleep` loop flags." The narrative report confirms: "Sleep dominated every agent's tick budget … Agent A committed sleep 143 times, Agent B 144 times, Agent C 146 times — together 433 of the 1440 ticks were spent in committed sleeps." The narrative report also notes Hillside Shelter "remained dormant" — there is no in-world reason to prefer it as a sleep site over Riverside Camp.

PR-9's topology depth ask resolves naturally if `SleepQualityProfile` differentiates places: Hillside Shelter authored as `shelter: Shelter, ground_comfort: Soft` becomes preferred for sleep over Fertile Fields' open-air orchard configuration, which gives the dormant place a survival-relevant reason to be visited.

## Design Goals

1. Sleep is one duration-bearing action instance, not a sequence of single-tick re-commits. The action's `DurationExpr` becomes `Variable { min: intended_min_ticks, max: intended_max_ticks }`. The tick handler runs the recovery curve and evaluates wake conditions; commit fires once at episode end.
2. `SleepEpisode` is runtime state, not scenario authoring. Created at action start, removed at commit. Per FND-22 Section 5 of `docs/spec-drafting-rules.md`, runtime-generated state is exempt from the agent-profile scenario contract.
3. Wake conditions are first-class. The episode carries `wake_conditions: Vec<WakeCondition>` populated at start. The tick handler evaluates each condition every tick and exits the episode when any fires.
4. Place quality modulates recovery rate, not whether sleep is allowed. A bad sleep site still produces fatigue reduction — just less per tick. A great site produces more per tick. No "you cannot sleep here" gating.
5. Place quality also drives site preference. The candidate emitter produces one sleep candidate per believed sleep-eligible place; ranked sleep motive scoring weighs `recovery_modifier` so a well-authored shelter ranks above an open-air orchard. This addresses the "Hillside Shelter remained dormant" smell from the gameplay report.
6. Interrupted sleep is partial success. Recovery accumulated up to the wake tick is preserved (the agent's `fatigue` is reduced by the integrated curve up to the wake point). This is the canonical FND-10 partial-aftermath case for sleep.
7. The episode is interruptible only through local or internal causes: projected need breach, scheduled commitment, and external interruption recorded as local disturbance. Not through global scheduling magic. Per the assessment: "Do not add danger or social interruptions if those systems are inactive." Place-safety wake conditions are deferred to S60 (see Non-Goals).
8. Two new event tags. Per FND-30, every spec declares its own causal records. PR-12's standalone "sleep event tags" proposal is folded here because the events are intrinsic to this spec's deliverables, not a separate concern.

## Non-Goals

- Bedding items, blanket entities, or mattress-grade simulation. The assessment explicitly rejects this: "Make place matter without over-simulating bedding."
- `WakeCondition::PlaceNoLongerSafe` (place safety as a wake trigger). Deferred until S60 (Persistent Site Occupancy) lands and `OccupancyClaim`/`OccupancyPosture` exist in code. The remaining live wake conditions (duration, target recovery, projected need breach, scheduled commitment, plus local-disturbance abort recording) are sufficient for Phase 10.
- Crowding effects on sleep quality. Deferred until S129 lands `PlaceDirtiness` (which will give the substrate for "this place has too many sleepers"); PR-9's crowding ask is deferred to a follow-up.
- Weather exposure modulation. No weather system exists yet (FND-5 YAGNI).
- Dream content, REM cycles, sleep debt rollover beyond the existing `fatigue` and `DeprivationExposure` surfaces. The body's `fatigue` field already carries integrated debt.
- Multi-agent shared sleep (squad rest). No bonding/social co-sleep mechanic; each agent has its own `SleepEpisode`.
- Sleep-induced perception suppression. Today, perception runs identically whether the agent is sleeping or not. PR-7's "sleeping reduces perception" is out of scope; if relevant, S130 (survey records) covers belief-side gaps.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `SleepEpisode` carries concrete per-tick fields (`start_tick`, `intended_min_ticks`, `intended_max_ticks`, `accumulated_recovery`); `SleepQualityProfile` carries concrete tags (`shelter`, `ground_comfort`); the recovery curve is a derived view over those fields, not a stored `sleep_score`. |
| FND-5 (Carriers of Consequence) | `SleepEpisode` and `SleepQualityProfile` propagate consequences: a poorly-rested agent becomes a less-effective traveler, a well-authored shelter site becomes a destination worth traveling to. |
| FND-7 (Locality of Motion, Interaction, and Communication) | All sleep state is per-agent (`SleepEpisode`) or per-place (`SleepQualityProfile`). No global "all agents are sleeping" query. Per-place sleep candidates are emitted only for places the agent's belief store knows about. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | Sleep now has explicit `intended_min_ticks`, `intended_max_ticks`, occupies the agent for the duration, and remains interruptible. Duration-routing uses the new `DurationExpr::Variable { min, max }`. |
| FND-10 (Outcomes Are Granular and Leave Aftermath) | Interrupted sleep carries `accumulated_recovery: Permille` — the partial aftermath. The end event records `end_reason: WakeReason` so the trace shows whether sleep completed normally or was cut short. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | "More sleep → less fatigue → more time for other things → eventually more activity → more fatigue." Dampener: physical recovery curve has diminishing returns (per-tick recovery slows as fatigue approaches zero); `intended_max_ticks` bounds the episode; competing need projections trigger wake. |
| FND-14 (World State Is Not Belief State) | Sleep ranking reads each anchored candidate place's `SleepQualityProfile` through `GoalBeliefView::place_sleep_quality_profile` (belief-mediated). The action handler reads authoritative `SleepQualityProfile` at execution time (correct — actions execute against world state). |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Co-located agents perceive `SleepQualityProfile` directly (it is a physical property of the place). Other agents' `SleepEpisode` (whether they are sleeping right now) is also co-located perception — knowing whether the person next to you is asleep is FND-14A. |
| FND-17 (Surprise Comes From Violated Expectation) | An agent who started sleep with `WakeCondition::ProjectedNeedBreach { need: Hunger }` and wakes earlier than the projected breach has a violated expectation about its own envelope. The wake event carries the projection that fired. |
| FND-21 (Intentions Are Revisable Commitments) | Sleep is now revisable in flight — wake conditions provide the lawful interruption surface. The committed intention is "rest until the conditions break"; the conditions break, the intention is revised. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents with the same fatigue at the same place wake at different ticks because `MetabolismProfile.rest_efficiency` differs. Two agents at different places with the same metabolism wake at different ticks because `SleepQualityProfile` differs. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Wake events become inputs to S131's source-reliability extension when applied to "sites this agent has slept at" — though that extension is out of scope for S128 itself. The hook exists. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Metabolism writes `HomeostaticNeeds`; sleep tick handler reads them and `SleepQualityProfile`; sleep tick handler writes `SleepEpisode.accumulated_recovery` and updates `HomeostaticNeeds.fatigue`; on wake, sleep handler emits the end event. No imperative cross-system call. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | The recovery curve is per-tick derived from `SleepQualityProfile` + `MetabolismProfile.rest_efficiency` + current `accumulated_recovery`. No "sleep score" cached as truth. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The old per-tick re-commit sleep path is removed, not preserved beside the new episode path. Existing scenarios continue to load — sleep just becomes one long action instead of many short ones, with no scenario-side change required. |
| FND-29 (Debuggability Is a Product Feature) | `SleepEpisodeStarted` event carries place, intended duration, wake conditions, recovery curve parameters. `SleepEpisodeEnded` carries actual duration, wake reason, accumulated recovery, final fatigue. "Why did Agent A sleep for only 12 ticks at Hillside Shelter?" is answerable from event log alone. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Two new `EventTag` variants land in the existing append-only event log. No erasure on episode end — both start and end events persist for replay and inspection. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers the four required analyses (information-path, positive-feedback, dampeners, stored-vs-derived). Consequence/contention/aftermath/lifecycle items are addressed across Deliverables, Cross-System Interactions, and Profile-Driven Parameters. |

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** `SleepQualityProfile` is per-place authoritative state, perceivable by co-located agents (FND-14A) and surfaced to non-co-located agents through their belief store via `GoalBeliefView::place_sleep_quality_profile`. `SleepEpisode` is per-agent runtime state, perceivable by co-located agents through the standard "is this person asleep?" surface (existing today via `ActionInstance.local_state`). Wake-condition synthesis reads only the agent's own state. No global queries.
2. **Positive-feedback analysis.** (a) "Better-rested agents travel more, perceive more sleep sites, sleep at better sites." Dampener: travel itself accumulates fatigue at the existing `MetabolismProfile.travel_fatigue_multiplier` rate; equilibrium emerges from rate vs. recovery. (b) "Wake-on-need-breach → re-plan → another sleep adoption → wake again." Dampener: between sleeps, the underlying need is addressed (or fails to be, in which case the agent dies — the canonical FND-11 hard dampener), shifting the projection.
3. **Concrete dampeners.** (a) `MetabolismProfile.travel_fatigue_multiplier`, `intended_max_ticks` cap on episodes. (b) `structural_block_ticks` from S109 if the wake-on-projection records a discrepancy via S126. (c) `accumulated_recovery` saturates at `current_fatigue` — recovery cannot drive fatigue below zero.
4. **Stored state vs. derived read-model.** Stored: `SleepEpisode`, `SleepQualityProfile`. Derived: per-tick recovery rate (`MetabolismProfile.rest_efficiency × SleepQualityProfile.recovery_modifier`), wake-condition evaluation result, per-place sleep candidate `motive_score` (recovery-modifier-weighted).

## Deliverables

### D1: `WakeCondition` enum

In `crates/worldwake-core/src/sleep_episode.rs` (new module):

```rust
/// A condition that ends an active sleep episode. Evaluated each tick
/// inside the sleep tick handler. The first matching condition fires
/// and the episode ends with that wake reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WakeCondition {
    /// Episode reached `intended_max_ticks` regardless of recovery.
    IntendedDurationReached,
    /// Recovery accumulated to `target_recovery` before max duration.
    TargetRecoveryReached,
    /// S126's projection for the named need would breach before
    /// `until_tick`. Read fresh each tick from the agent's own state.
    ProjectedNeedBreach { need: HomeostaticNeedId },
    /// A scheduled commitment (existing expectation-store obligation) becomes
    /// due at the named tick.
    ScheduledCommitmentDue { tick: Tick },
    /// External interruption or a future local-disturbance perception carrier.
    LocalDisturbance,
}
```

`WakeCondition::PlaceNoLongerSafe` is intentionally absent. It is deferred until S60 (Persistent Site Occupancy) lands and `OccupancyClaim`/`OccupancyPosture` exist in code. See Non-Goals.

### D2: `SleepEpisode` runtime component

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SleepEpisode {
    pub place: EntityId,
    pub start_tick: Tick,
    pub intended_min_ticks: NonZeroU32,
    pub intended_max_ticks: NonZeroU32,
    pub target_recovery: Permille,
    pub accumulated_recovery: Permille,
    pub recovery_modifier: Permille,
    pub wake_conditions: Vec<WakeCondition>,
}

impl Component for SleepEpisode {}
```

Runtime-only — populated by `sleep` action start, removed by commit. Exempt from the FND-22 scenario contract per the runtime-generated-state carve-out (analogous to `AgendaEntry`, `IntentionFrame`, `WoundList`). `recovery_modifier` is cached at episode start from the bound place's `SleepQualityProfile` so the tick handler does not re-read place state mid-episode.

### D3: `SleepQualityProfile` per-place component

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SleepQualityProfile {
    pub shelter: ShelterTag,
    pub ground_comfort: GroundComfortTag,
    /// Per-tick recovery multiplier, applied as
    /// `MetabolismProfile.rest_efficiency × recovery_modifier`.
    pub recovery_modifier: Permille,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ShelterTag {
    Open,        // Forest Clearing, Fertile Fields
    PartialCover, // Forest Clearing if partially canopied
    Roofed,      // Riverside Camp tents
    Shelter,     // Hillside Shelter
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GroundComfortTag {
    Hard,        // bare stone
    Earth,       // packed dirt
    Soft,        // grass, padded bedding floor
}

impl Component for SleepQualityProfile {}

impl Default for SleepQualityProfile {
    fn default() -> Self {
        Self {
            shelter: ShelterTag::Open,
            ground_comfort: GroundComfortTag::Earth,
            recovery_modifier: Permille::new_unchecked(1000),
        }
    }
}
```

`recovery_modifier = 1000` is "no modulation" — matches existing per-tick recovery exactly. Because this is a `Permille`, authored values must stay in `0..=1000`: Hillside Shelter is authored as `(Shelter, Soft, 1000)`; Riverside Camp as `(Roofed, Earth, 900)`; Forest Clearing as `(PartialCover, Earth, 800)`; Fertile Fields as `(Open, Earth, 700)`. Authored numbers are tunable per scenario within the `Permille` range.

`SleepQualityProfile` is a **universal place component** — every place has one (see Component Registration and D9).

### D4: `EventTag` extensions

In `crates/worldwake-core/src/event_tag.rs`:

```rust
pub enum EventTag {
    // ... existing variants ...
    SleepEpisodeStarted,
    SleepEpisodeEnded,
}
```

Payload structs in `crates/worldwake-core/src/decision_event_payload.rs` per S110 conventions:

```rust
pub struct SleepEpisodeStartedPayload {
    pub sleeper: EntityId,
    pub place: EntityId,
    pub intended_min_ticks: NonZeroU32,
    pub intended_max_ticks: NonZeroU32,
    pub target_recovery: Permille,
    pub wake_conditions: Vec<WakeCondition>,
    pub recovery_modifier: Permille,
}

pub struct SleepEpisodeEndedPayload {
    pub sleeper: EntityId,
    pub place: EntityId,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub end_reason: WakeReason,
    pub accumulated_recovery: Permille,
    pub final_fatigue: Permille,
}

pub enum WakeReason {
    IntendedDuration,
    TargetRecovery,
    ProjectedNeedBreach { need: HomeostaticNeedId, projected_breach_tick: Tick },
    ScheduledCommitment,
    LocalDisturbance,
}
```

### D5: `DurationExpr::Variable { min, max }` variant

In `crates/worldwake-sim/src/action_semantics.rs`, extend `DurationExpr`:

```rust
pub enum DurationExpr {
    // ... existing variants ...
    Variable { min: NonZeroU32, max: NonZeroU32 },
}
```

`fixed_ticks()` returns `None` for `Variable` (consistent with other non-fixed variants like `ActorMetabolism`). Duration-routing match arms in the scheduler/action framework receive a new arm that passes `max` as the scheduling upper bound; tick-handler-driven early termination commits before the upper bound is reached. The existing test sweep `ALL_DURATION_EXPRS` (`action_semantics.rs` test module around line 485) gains the new variant.

### D6: `MetabolismProfile.min_sleep_ticks` field

In `crates/worldwake-core/src/needs.rs`, extend `MetabolismProfile`:

```rust
pub struct MetabolismProfile {
    // ... existing fields ...
    pub min_sleep_ticks: NonZeroU32,
}
```

`Default` impl (around `needs.rs:233-254`) extends with `min_sleep_ticks: NonZeroU32::new(8).unwrap()`. `AgentDef.metabolism_profile` already exposes the full `MetabolismProfile` struct in scenario authoring (`crates/worldwake-cli/src/scenario/types.rs`), so no separate `AgentDef` field is needed; existing scenarios using `metabolism_profile: None` continue to work because the field defaults to `8` ticks. Because `MetabolismProfile` is persisted in the current bincode save payload, the D6 implementation bumps `SAVE_FORMAT_VERSION` from `54` to `55`; older saves remain rejected by the existing version gate instead of being loaded through a compatibility shim.

### D7: `sleep` action handler refactor

In `crates/worldwake-systems/src/needs_actions.rs`, the `sleep` action:

- Registration: `DurationExpr::Variable { min: NonZeroU32::new(1).unwrap(), max: <large default like 64> }` (placeholder bounds; per-episode actual bounds come from `SleepEpisode`). Sleep stays untargeted at the action-registration level (`ActionPayload::None`); per-place selection is handled in the AI candidate emitter (D8) by emitting one candidate per believed sleep place. The action's `Interruptibility` becomes `FreelyInterruptible` (no penalty for wake-condition-driven exit; the wake machinery is internal to the tick handler).
- `start`: derives `intended_min_ticks` from `MetabolismProfile.min_sleep_ticks`, `intended_max_ticks` from agent fatigue and `rest_efficiency` (long enough to fully recover under typical conditions, capped). Reads the agent's current place's `SleepQualityProfile` to capture `recovery_modifier` on the episode. Derives `wake_conditions` from S126's per-need projections plus any scheduled commitments in the agent's `ExpectationStore` (see D10). Inserts `SleepEpisode` component. Emits `SleepEpisodeStarted` event.
- `tick`: each tick, applies recovery `MetabolismProfile.rest_efficiency × SleepEpisode.recovery_modifier × per-tick-step`. Updates `SleepEpisode.accumulated_recovery` and reduces `HomeostaticNeeds.fatigue`. Evaluates each `WakeCondition`. If any fires, the action transitions to commit on the next step.
- `commit`: removes `SleepEpisode` component, emits `SleepEpisodeEnded` event with `end_reason`. Returns `CommitOutcome` with the recovery delta.

The wake-condition machinery sits inside the tick handler; no new sim-side scheduling primitive is needed. Sleep's `Precondition` list (`Precondition::ActorAlive`) is unchanged.

### D8: Per-place sleep candidate emission and ranking

In the AI candidate-emission pipeline (`crates/worldwake-ai/src/candidate_generation.rs` or the equivalent emitter for `Sleep`), change sleep emission from a single untargeted candidate to one candidate per believed sleep-eligible place. For each place in the agent's belief store that the agent could lawfully sleep at (alive, in graph, reachable), emit a `Sleep` candidate carrying that place as the `OpportunityAnchor`'s site reference. Ranking reads each candidate place's `SleepQualityProfile` via `GoalBeliefView::place_sleep_quality_profile` (D9) from that anchored place; the candidate trace exposes the concrete place anchor and fatigue evidence rather than duplicating the modifier into a separate evidence-summary field.

In `crates/worldwake-ai/src/ranking.rs`, extend ranked sleep motive scoring to weigh `recovery_modifier`: a place with higher `recovery_modifier` produces a higher ranked motive score for sleep at that place, so a well-authored shelter ranks above an open-air orchard at the same fatigue level. Per the candidate-scoring architecture pattern, this is a ranking concern (not an emission gate) — every reachable place still emits a candidate; ranking decides which one wins.

This delivers the spec's goal of differentiated sleep sites: at equal fatigue, an agent with belief of both Hillside Shelter (`1000`) and Riverside Camp (`900`) prefers Hillside Shelter.

### D9: `GoalBeliefView::place_sleep_quality_profile` accessor

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait (or the appropriate sub-trait such as `ProfileBeliefView` if place-state accessors live there):

```rust
fn place_sleep_quality_profile(&self, agent: EntityId, place: EntityId) -> SleepQualityProfile;
```

Implement on `RuntimeBeliefView` by reading the place's component (universal — always present per Component Registration) only when the named agent knows the place or is currently there. Forward through `impl_goal_belief_view!` macro / blanket impl. Returns `SleepQualityProfile::default()` if the place is unknown to the named agent's belief store (i.e., the agent has no belief about it) — this prevents the AI from constructing site preference for places it has never observed.

### D10: Wake-condition synthesis

In `crates/worldwake-systems/src/sleep_synthesis.rs`:

For an agent adopting `Sleep`, build the wake-condition vec:

1. `WakeCondition::IntendedDurationReached` — always present.
2. For each `HomeostaticNeedId` in `HomeostaticNeedId::ALL` except `Fatigue`: if S126 projection returns `Some(breach_tick)` with `breach_tick < current_tick + intended_max_ticks`, push `WakeCondition::ProjectedNeedBreach { need }`.
3. Read the agent's `ExpectationStore` for active or overdue agent-owned commitment records, excluding internal `PlanStepCompletion` records; if any deadline falls by the sleep horizon, push `WakeCondition::ScheduledCommitmentDue { tick }` in chronological order.
4. `WakeCondition::LocalDisturbance` — always present.

`WakeCondition::TargetRecoveryReached` is added if `target_recovery < 1000` (the agent has explicit "wake when fatigue is below X" intent).

`LocalDisturbance` is live as the `abort_sleep_episode` end reason. Normal tick evaluation does not fire it until a clean systems-readable local-disturbance carrier exists.

### D11: Decision-trace surfacing

The S110 decision-trace path renders `SleepEpisodeStarted` and `SleepEpisodeEnded` payloads alongside other decision-history events. Existing observer Section 3 rendering already iterates the event tags; the two new variants land naturally.

### D12: Scenario authoring

In `crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct PlaceDef {
    // ... existing fields ...
    #[serde(default)]
    pub sleep_quality: Option<SleepQualityProfileDef>,
}

pub struct SleepQualityProfileDef {
    pub shelter: ShelterTag,
    pub ground_comfort: GroundComfortTag,
    pub recovery_modifier: u16, // permille value
}
```

In `crates/worldwake-cli/src/scenario/mod.rs`, the place-spawning loop unconditionally applies the universal default:

```rust
for place_def in &def.places {
    let place_id = resolve_name(names, &place_def.name, "place sleep_quality")?;
    let profile = place_def
        .sleep_quality
        .as_ref()
        .map(|def| def.clone().into())
        .unwrap_or_default();
    txn.set_component_sleep_quality_profile(place_id, profile)?;
}
```

This establishes a new precedent for **universal place components** (parallel to the agent universal pattern at `mod.rs:566-624`). Existing `scenarios/*.ron` require no change — places without `sleep_quality` get `SleepQualityProfile::default()` automatically. Survival-baseline rebalance is a follow-up ticket: author `sleep_quality` on the four named places to match the assessment's intent (Hillside Shelter best, Fertile Fields worst).

### D13: Golden coverage

Add `crates/worldwake-ai/tests/golden_sleep_episode.rs`:

- **Test 1 — episode lifecycle.** Agent with high fatigue adopts sleep at default place; confirm one `SleepEpisode` runtime component, one `SleepEpisodeStarted` event, one `SleepEpisodeEnded` event after `intended_max_ticks`, no `sleep → sleep` loop in the action log.
- **Test 2 — projected need breach wake.** With S126 enabled, agent with rising hunger adopts sleep; confirm wake fires on `WakeCondition::ProjectedNeedBreach { Hunger }` before `intended_max_ticks` and the `SleepEpisodeEnded.end_reason` records the breach.
- **Test 3 — place-quality recovery differentiation.** Two agents, agent A spawned at Riverside Camp (`recovery_modifier: 900`), agent B spawned at Fertile Fields (`recovery_modifier: 700`); same starting fatigue and metabolism. Agent A wakes at fewer ticks than agent B because place quality differs. Asserts the recovery-rate path independent of site selection.
- **Test 4 — interrupted-sleep partial recovery.** Wake at tick `T < intended_max_ticks`; confirm `accumulated_recovery` reflects integrated curve up to T and `HomeostaticNeeds.fatigue` is reduced by exactly `accumulated_recovery`.
- **Test 5 — site preference via candidate ranking.** Agent with belief of two reachable sleep sites (Hillside Shelter `1000`, Riverside Camp `900`) and no other distinguishing factors (same fatigue, same metabolism, same travel cost). Confirm the agent commits to a Sleep goal anchored at Hillside Shelter — exercises D8's per-place emission and `motive_score` integration.
- **Test 6 — decision-trace integration.** Confirm `SleepEpisodeStarted` and `SleepEpisodeEnded` events appear in the event log at the expected ticks, payload fields populated, and renderable through the observer's decision-history surface.

## SystemFn Integration

No new system tick. The sleep action's tick handler runs inside the existing action-tick loop. Wake-condition evaluation is a per-tick read inside that handler. `SleepQualityProfile` is read once at episode start (cached on the `SleepEpisode` runtime component as `recovery_modifier`).

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `SleepEpisode` | Agent | Runtime-generated | Removed when not sleeping; exempt from FND-22 Section 5 scenario contract |
| `SleepQualityProfile` | Place | Universal | `Default` (`Open`, `Earth`, `1000`) — every place is given the component at `spawn_place`; runtime reads use `expect()` |

`SleepEpisode` follows the same runtime-generated exemption as `AgendaEntry`, `IntentionFrame`, `WoundList`. Per `docs/spec-drafting-rules.md` Section 5: components that "are purely runtime-generated state … are exempt because they emerge from simulation, not configuration."

`SleepQualityProfile` is a **universal place component** — establishes a new precedent (no prior universal-on-Place component exists; `PlaceVisibilityProfile` is optional via `if let Some(...)`). Per FND-22 Section 5's universal pattern as adapted for places: `spawn_place` always calls `set_component_sleep_quality_profile(place_id, def.sleep_quality.map(Into::into).unwrap_or_default())`; runtime reads on known places use `expect()`, deterministic fallback at runtime.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Metabolism (`needs.rs`) | Reads `HomeostaticNeeds.fatigue`; writes reduced fatigue per recovery curve | State-mediated |
| Need projection (S126) | Wake-condition synthesis reads `NeedSafeUntilTick` projections | State-mediated |
| Decision history (S110) | New `EventTag::SleepEpisodeStarted` / `SleepEpisodeEnded` land in event log | State-mediated |
| Perception | Co-located agents perceive `SleepEpisode` (the agent is asleep) and `SleepQualityProfile` (place property) | State-mediated |
| Belief store | `GoalBeliefView::place_sleep_quality_profile` exposes per-place sleep quality to the AI candidate emitter | State-mediated |
| Drive escalation (S116) | No interaction — drive escalation modulates pre-sleep selection ranking, not in-episode behavior | None |

## Profile-Driven Parameters

Per-agent variation:

- `MetabolismProfile.rest_efficiency` — base recovery rate per tick.
- `MetabolismProfile.min_sleep_ticks` — minimum episode duration (new field per D6; default `NonZeroU32::new(8).unwrap()`). Universal `MetabolismProfile` already has a `Default` impl per existing convention; this field defaults to the same `8` value.

Per-place variation:

- `SleepQualityProfile.{shelter, ground_comfort, recovery_modifier}` — authored per place in scenario RON; defaults to `(Open, Earth, 1000)` when absent.

No magic numbers introduced. All `Permille` for [0,1000] range values; no `f32`/`f64`.
