# S128: Sleep Episodes and Place-Quality Recovery

## Summary

Replace the per-tick re-commit pattern that produces 143–146 separate `sleep` actions per agent across a 1440-tick run with a duration-bearing sleep episode that holds an explicit recovery curve, place-quality inputs, and wake conditions integrating with S126's need-projection assumptions. Today, `tick_sleep` runs every tick and the planner re-selects sleep again; the narrative report flags this as a benign `sleep → sleep` loop artifact, but the deeper issue is that sleep cannot have intent: an agent cannot decide "sleep until fatigue is below comfort" or "sleep until thirst projection breaches" because there is no episode-level state to carry that intent. This spec adds `SleepEpisode` as a per-agent runtime component populated when sleep starts and torn down on a `WakeCondition` firing. Place tags drive a per-place sleep-quality modifier on the recovery curve (giving Hillside Shelter, Forest Clearing, and Riverside Camp authored differentiation without new place kinds). Interrupted sleep — woke early because hunger projection breached — produces partial-recovery aftermath (PR-11) instead of failing. New `EventTag::SleepEpisodeStarted` / `EventTag::SleepEpisodeEnded` make the episode a first-class causal record (PR-12 sleep events folded in here, not a standalone PR-12 spec).

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `SleepEpisode` runtime component (per-agent, populated on sleep start, removed on wake), `WakeCondition` enum, `SleepQualityProfile` per-place component (universal default if absent), `SleepRecoveryCurve` derived helper, two new `EventTag` variants.
- `worldwake-systems` — `sleep` action handler refactor: start populates `SleepEpisode`, tick consumes the recovery curve (modulated by place quality), commit removes `SleepEpisode` and emits the end event. Wake-condition evaluation runs once per tick within the sleep tick handler.
- `worldwake-ai` — sleep candidate generation reads `SleepQualityProfile` of believed places when ranking sleep at multiple candidate sites. Wake-condition synthesis reads S126's `NeedSafeUntilTick` projection to populate `WakeCondition::ProjectedNeedBreach`.
- `worldwake-cli` — `PlaceDef.sleep_quality` optional field; defaults to `SleepQualityProfile::default()` when absent.

## Dependencies

- S126 (Need Projection and Plan Time-Budget Assumptions) — **soft**. `WakeCondition::ProjectedNeedBreach` queries the `NeedSafeUntilTick` evaluator. Without S126, sleep wakes only on `IntendedDurationReached` or `LocalDisturbance`; with S126, wake-on-projection is available.
- S109 (Typed Discrepancy Taxonomy) — **soft**. Interrupted-sleep partial recovery does not record a discrepancy by itself; if the wake was triggered by an S126 projection breach, that breach already records its own discrepancy and S128 piggybacks.
- S110 (Decision History Events) — **hard**. The new `EventTag::SleepEpisodeStarted` and `EventTag::SleepEpisodeEnded` follow the S110 decision-history event surface (`worldwake-core/src/decision_event_payload.rs`).
- S116 (Drive Escalation) — **soft**. Drive escalation already modulates motive scoring for sleep selection; this spec does not change that surface, only what happens once sleep is selected.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 3: "Sleep is the clearest mechanical smell in the report. Agents committed sleep 143–146 times each, and Agent A produced repeated `sleep → sleep` loop flags." The narrative report confirms: "Sleep dominated every agent's tick budget … Agent A committed sleep 143 times, Agent B 144 times, Agent C 146 times — together 433 of the 1440 ticks were spent in committed sleeps." The narrative report also notes Hillside Shelter "remained dormant" — there is no in-world reason to prefer it as a sleep site over Riverside Camp.

PR-9's topology depth ask resolves naturally if `SleepQualityProfile` differentiates places: Hillside Shelter authored as `shelter: HighQuality, ground_comfort: Soft` becomes preferred for sleep over Fertile Fields' open-air orchard configuration, which gives the dormant place a survival-relevant reason to be visited.

## Design Goals

1. Sleep is one duration-bearing action instance, not a sequence of single-tick re-commits. The action's `DurationExpr` becomes `Variable` bounded by `intended_min_ticks` and `intended_max_ticks`. The tick handler runs the recovery curve and evaluates wake conditions; commit fires once at episode end.
2. `SleepEpisode` is runtime state, not scenario authoring. Created at action start, removed at commit. Per FND-22 Section 5 of `docs/spec-drafting-rules.md`, runtime-generated state is exempt from the agent-profile scenario contract.
3. Wake conditions are first-class. The episode carries `wake_conditions: Vec<WakeCondition>` populated at start. The tick handler evaluates each condition every tick and exits the episode when any fires.
4. Place quality modulates recovery rate, not whether sleep is allowed. A bad sleep site still produces fatigue reduction — just less per tick. A great site produces more per tick. No "you cannot sleep here" gating.
5. Interrupted sleep is partial success. Recovery accumulated up to the wake tick is preserved (the agent's `fatigue` is reduced by the integrated curve up to the wake point). This is the canonical FND-10 partial-aftermath case for sleep.
6. The episode is interruptible only through local or internal causes: projected need breach, scheduled commitment, local disturbance, place-no-longer-safe. Not through global scheduling magic. Per the assessment: "Do not add danger or social interruptions if those systems are inactive."
7. Two new event tags. Per FND-30, every spec declares its own causal records. PR-12's standalone "sleep event tags" proposal is folded here because the events are intrinsic to this spec's deliverables, not a separate concern.

## Non-Goals

- Bedding items, blanket entities, or mattress-grade simulation. The assessment explicitly rejects this: "Make place matter without over-simulating bedding."
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
| FND-7 (Locality of Motion, Interaction, and Communication) | All sleep state is per-agent (`SleepEpisode`) or per-place (`SleepQualityProfile`). No global "all agents are sleeping" query. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | Sleep now has explicit `intended_min_ticks`, `intended_max_ticks`, occupies the agent for the duration, and remains interruptible. The `sleep` action's `Interruptibility` becomes `Interruptible { wake_conditions }`. |
| FND-10 (Outcomes Are Granular and Leave Aftermath) | Interrupted sleep carries `accumulated_recovery: Permille` — the partial aftermath. The end event records `end_reason: WakeReason` so the trace shows whether sleep completed normally or was cut short. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | "More sleep → less fatigue → more time for other things → eventually more activity → more fatigue." Dampener: physical recovery curve has diminishing returns (per-tick recovery slows as fatigue approaches zero); `intended_max_ticks` bounds the episode; competing need projections trigger wake. |
| FND-14 (World State Is Not Belief State) | The candidate generator reads the agent's belief about which places it could sleep at; the action handler reads authoritative `SleepQualityProfile` at execution time (correct — actions execute against world state). |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Co-located agents perceive `SleepQualityProfile` directly (it is a physical property of the place). Other agents' `SleepEpisode` (whether they are sleeping right now) is also co-located perception — knowing whether the person next to you is asleep is FND-14A. |
| FND-17 (Surprise Comes From Violated Expectation) | An agent who started sleep with `WakeCondition::ProjectedNeedBreach { need: Hunger, until_tick: T }` and wakes at `tick < T` has a violated expectation about its own envelope. The wake event carries this. |
| FND-21 (Intentions Are Revisable Commitments) | Sleep is now revisable in flight — wake conditions provide the lawful interruption surface. The committed intention is "rest until the conditions break"; the conditions break, the intention is revised. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents with the same fatigue at the same place wake at different ticks because `MetabolismProfile.rest_efficiency` differs. Two agents at different places with the same metabolism wake at different ticks because `SleepQualityProfile` differs. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Wake events become inputs to S131's source-reliability extension when applied to "sites this agent has slept at" — though that extension is out of scope for S128 itself. The hook exists. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Metabolism writes `HomeostaticNeeds`; sleep tick handler reads them and `SleepQualityProfile`; sleep tick handler writes `SleepEpisode.accumulated_recovery` and updates `HomeostaticNeeds.fatigue`; on wake, sleep handler emits the end event. No imperative cross-system call. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | The recovery curve is per-tick derived from `SleepQualityProfile` + `MetabolismProfile.rest_efficiency` + current `accumulated_recovery`. No "sleep score" cached as truth. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The old per-tick re-commit sleep path is removed, not preserved beside the new episode path. Existing scenarios continue to load — sleep just becomes one long action instead of many short ones, with no scenario-side change required. |
| FND-29 (Debuggability Is a Product Feature) | `SleepEpisodeStarted` event carries place, intended duration, wake conditions, recovery curve parameters. `SleepEpisodeEnded` carries actual duration, wake reason, accumulated recovery, final fatigue. "Why did Agent A sleep for only 12 ticks at Hillside Shelter?" is answerable from event log alone. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Two new `EventTag` variants land in the existing append-only event log. No erasure on episode end — both start and end events persist for replay and inspection. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers all 18 declarations. |

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** `SleepQualityProfile` is per-place authoritative state, perceivable by co-located agents (FND-14A). `SleepEpisode` is per-agent runtime state, perceivable by co-located agents through the standard "is this person asleep?" surface (existing today via `ActionState`). Wake-condition synthesis reads only the agent's own state. No global queries.
2. **Positive-feedback analysis.** (a) "Better-rested agents travel more, perceive more sleep sites, sleep at better sites." Dampener: travel itself accumulates fatigue at the existing `MetabolismProfile.travel_fatigue_multiplier` rate; equilibrium emerges from rate vs. recovery. (b) "Wake-on-need-breach → re-plan → another sleep adoption → wake again." Dampener: between sleeps, the underlying need is addressed (or fails to be, in which case the agent dies — the canonical FND-11 hard dampener), shifting the projection.
3. **Concrete dampeners.** (a) `MetabolismProfile.travel_fatigue_multiplier`, `intended_max_ticks` cap on episodes. (b) `structural_block_ticks` from S109 if the wake-on-projection records a discrepancy via S126. (c) `accumulated_recovery` saturates at `current_fatigue` — recovery cannot drive fatigue below zero.
4. **Stored state vs. derived read-model.** Stored: `SleepEpisode`, `SleepQualityProfile`. Derived: per-tick recovery rate (`MetabolismProfile.rest_efficiency × SleepQualityProfile.recovery_modifier`), wake-condition evaluation result.

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
    /// A scheduled commitment (existing intention frame queue) becomes
    /// due at the named tick.
    ScheduledCommitmentDue { tick: Tick },
    /// Local disturbance perceived (per existing perception channel).
    LocalDisturbance,
    /// The place's safety/validity changed (e.g., site occupancy
    /// flipped to hostile per S60). Reuses S60's site occupancy state.
    PlaceNoLongerSafe,
}
```

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
    pub wake_conditions: Vec<WakeCondition>,
}

impl Component for SleepEpisode {}
```

Runtime-only — populated by `sleep` action start, removed by commit. Exempt from the FND-22 scenario contract per the runtime-generated-state carve-out.

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

`recovery_modifier = 1000` is "no modulation" — matches existing per-tick recovery exactly. Riverside Camp authored as `(Roofed, Earth, 1100)`; Hillside Shelter as `(Shelter, Soft, 1300)`; Fertile Fields as `(Open, Earth, 900)`; Forest Clearing as `(PartialCover, Earth, 1000)`. Authored numbers tunable per scenario.

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
    PlaceNoLongerSafe,
}
```

### D5: `sleep` action handler refactor

In `crates/worldwake-systems/src/needs_actions.rs`, the `sleep` action:

- `start`: derives `intended_min_ticks` from `MetabolismProfile.min_sleep_ticks`, `intended_max_ticks` from agent fatigue and `rest_efficiency` (long enough to fully recover under typical conditions, capped). Derives `wake_conditions` from S126's per-need projections plus any scheduled commitments in the agent's intention queue. Inserts `SleepEpisode` component. Emits `SleepEpisodeStarted` event.
- `tick`: each tick, applies recovery `MetabolismProfile.rest_efficiency × SleepQualityProfile.recovery_modifier × per-tick-step`. Updates `SleepEpisode.accumulated_recovery` and reduces `HomeostaticNeeds.fatigue`. Evaluates each `WakeCondition`. If any fires, the action transitions to commit on the next step.
- `commit`: removes `SleepEpisode` component, emits `SleepEpisodeEnded` event with `end_reason`. Returns `CommitOutcome` with the recovery delta.

The action's `DurationExpr` becomes `Variable { min: intended_min_ticks, max: intended_max_ticks }`. `Interruptibility` becomes `Interruptible` (existing variant); the wake-condition machinery sits inside the tick handler, no new sim-side scheduling primitive needed.

### D6: Wake-condition synthesis

In `crates/worldwake-ai/src/agent_tick/sleep_synthesis.rs` (new):

For an agent adopting `Sleep`, build the wake-condition vec:

1. `WakeCondition::IntendedDurationReached` — always present.
2. For each `HomeostaticNeedId` in `HomeostaticNeedId::ALL` except `Fatigue`: if S126 projection returns `Some(breach_tick)` with `breach_tick < current_tick + intended_max_ticks`, push `WakeCondition::ProjectedNeedBreach { need }`.
3. Read the agent's intention frame queue for `ScheduledCommitmentDue`; if any tick falls within the sleep window, push `WakeCondition::ScheduledCommitmentDue { tick }`.
4. `WakeCondition::LocalDisturbance` — always present.
5. `WakeCondition::PlaceNoLongerSafe` — always present (cheap evaluation).

`WakeCondition::TargetRecoveryReached` is added if `target_recovery < 1000` (the agent has explicit "wake when fatigue is below X" intent).

### D7: Decision-trace surfacing

The S110 decision-trace path renders `SleepEpisodeStarted` and `SleepEpisodeEnded` payloads alongside other decision-history events. Existing observer Section 3 rendering already iterates the event tags; the two new variants land naturally.

### D8: Scenario authoring

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

Existing `scenarios/*.ron` require no change — places without `sleep_quality` use `SleepQualityProfile::default()`. Survival-baseline rebalance is a follow-up ticket: author `sleep_quality` on the four places to match the assessment's intent (Hillside Shelter best, Fertile Fields worst).

### D9: Golden coverage

Add `crates/worldwake-ai/tests/golden_sleep_episode.rs`:

- Agent with high fatigue adopts sleep at default place; confirm one `SleepEpisode` runtime component, one `SleepEpisodeStarted` event, one `SleepEpisodeEnded` event after `intended_max_ticks`, no `sleep → sleep` loop in the action log.
- With S126 enabled, agent with rising hunger adopts sleep; confirm wake fires on `WakeCondition::ProjectedNeedBreach { Hunger }` before `intended_max_ticks` and the `SleepEpisodeEnded.end_reason` records the breach.
- Two agents, one at Riverside Camp (recovery_modifier 1100), one at Fertile Fields (900); same starting fatigue and metabolism — confirm they wake at different ticks because place quality differs.
- Interrupted-sleep partial: wake at tick T < `intended_max_ticks`; confirm `accumulated_recovery` reflects integrated curve up to T and `HomeostaticNeeds.fatigue` is reduced by exactly `accumulated_recovery`.

## SystemFn Integration

No new system tick. The sleep action's tick handler runs inside the existing action-tick loop. Wake-condition evaluation is a per-tick read inside that handler. `SleepQualityProfile` is read once at episode start (cached on the `SleepEpisode` runtime component as `recovery_modifier`).

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `SleepEpisode` | Agent | Runtime-generated | Removed when not sleeping; exempt from FND-22 Section 5 scenario contract |
| `SleepQualityProfile` | Place | Universal | `Default` (`Open`, `Earth`, `1000`) — every place implicitly has one |

`SleepEpisode` follows the same runtime-generated exemption as `ActiveGoal`, `IntentionFrame`, `WoundList`. Per `docs/spec-drafting-rules.md` Section 5: components that "are purely runtime-generated state … are exempt because they emerge from simulation, not configuration."

`SleepQualityProfile` is a universal place component (every place has one, with default `recovery_modifier = 1000` for "no modulation"). Per FND-22 Section 5: the universal pattern applies to places too — `unwrap_or_default()` in scenario load, deterministic fallback at runtime.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Metabolism (`needs.rs`) | Reads `HomeostaticNeeds.fatigue`; writes reduced fatigue per recovery curve | State-mediated |
| Need projection (S126) | Wake-condition synthesis reads `NeedSafeUntilTick` projections | State-mediated |
| Decision history (S110) | New `EventTag::SleepEpisodeStarted` / `SleepEpisodeEnded` land in event log | State-mediated |
| Site occupancy (S60) | `WakeCondition::PlaceNoLongerSafe` reads `OccupancyClaim.posture` for the agent's place | State-mediated |
| Perception | Co-located agents perceive `SleepEpisode` (the agent is asleep) and `SleepQualityProfile` (place property) | State-mediated |
| Drive escalation (S116) | No interaction — drive escalation modulates pre-sleep selection ranking, not in-episode behavior | None |

## Profile-Driven Parameters

Per-agent variation:

- `MetabolismProfile.rest_efficiency` — base recovery rate per tick.
- `MetabolismProfile.min_sleep_ticks` — minimum episode duration (new field; default `NonZeroU32::new(8).unwrap()`). Universal `MetabolismProfile` already has a `Default` impl per existing convention; this field defaults to the same `8` value.

Per-place variation:

- `SleepQualityProfile.{shelter, ground_comfort, recovery_modifier}` — authored per place in scenario RON.

No magic numbers introduced. All `Permille` for [0,1000] range values; no `f32`/`f64`.
