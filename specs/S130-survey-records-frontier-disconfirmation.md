# S130: Survey Records and Frontier Disconfirmation

## Summary

When an agent travels somewhere expecting to find a resource and arrives to find none, the architecture should record "I looked here for X and found nothing" as concrete agent state — not erase the visit because the belief store has nothing to record. Today, `ExploreLocation { target_place, motivating_need }` (from S80) gets the agent to the place and S102's frontier-aware exploration drives the topology coverage. But the agent has no way to express "I checked Hillside Shelter for food and confirmed it is empty" — the absence becomes invisible to future planning, and the same agent (or another agent reading shared beliefs) can be drawn back to the same fruitless trip. This spec adds two carriers: `SurveyRecord` per (agent, place, hypothesis), and `HypothesisKind` on `ExploreLocation`. When the agent arrives, the perception pipeline checks whether the hypothesis was satisfied; if not, the survey records "checked, did not find" with confidence and tick. Future ranking suppresses re-exploration of confirmed-empty places under the same hypothesis until confidence decays. Folds in PR-7's narrow "arrival-time prediction error for resource sources" piece — when the agent expected `CommodityAvailableAt` (from S122) and arrives to find the source empty or absent, the existing `ExpectationMismatch` event surface carries the diff.

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `SurveyRecord` and `SurveyMemory` components, `HypothesisKind` enum on `ExploreLocation`, `SurveyRecorded` event tag with payload struct.
- `worldwake-systems` — perception system extension: when the agent arrives at a place with a non-empty exploration history, evaluate hypothesis vs. perceived entities and write `SurveyMemory` entry. Decay rule piggybacks on the existing belief decay pass.
- `worldwake-ai` — `ExploreLocation` candidate generation populates `hypothesis` from the motivating need (Hunger → MayContainFood, Thirst → MayContainWater, etc.); ranking suppresses `ExploreLocation` for (place, hypothesis) pairs with a fresh negative survey.
- `worldwake-cli` — no scenario authoring (runtime-generated state, exempt from FND-22 contract).

## Dependencies

- S80 (Exploration Drive) — **completed**. `ExploreLocation` substrate exists; this spec extends it.
- S102 (Frontier-Aware Exploration) — **completed**. Frontier selection feeds into the candidate generator that populates the new hypothesis.
- S107 (Proactive Diversification) — **completed**. Proactive exploration uses the same hypothesis machinery (`HypothesisKind::Proactive` for diversification candidates).
- S122 (Frame Assumption — Commodity Availability) — **soft**. Arrival-time mismatch on `CommodityAvailableAt` already surfaces through S109 discrepancy memory; this spec piggybacks the existing `ExpectationMismatch` event for the resource-not-found case rather than introducing a new event type for the same concern.
- S101 (Activation-Based Belief Decay) — **soft**. Survey records decay via the same belief-store activation pass; freshly-supported records persist longer.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 6: "Arrival should produce an explicit `SurveyRecord` … This lets an agent become disappointed without global correction. If Hillside Shelter contains no food, an agent who explored it should know 'I looked and did not find food,' while another agent who never visited it should remain ignorant." The narrative report shows Hillside Shelter never visited despite being structurally reachable: "no agent ever visited it … the place stayed dormant." Today, even if an agent did visit, the architecture has no way to remember "I checked here and there's no food" — the next exploration cycle can pick the same place again.

## Design Goals

1. `SurveyRecord` is per-(agent, place, hypothesis) state. The agent's `SurveyMemory` component holds a small bounded list (capped at `cognitive.survey_memory_capacity`, default 24).
2. Hypothesis is part of the goal. `ExploreLocation { target_place, motivating_need, hypothesis }` makes the agent's exploration intent inspectable. The hypothesis is what the agent expected to find.
3. Arrival-time evaluation happens in the perception pipeline. When the agent enters the place, the perception system checks "did the agent perceive an entity satisfying the hypothesis?" — if yes, `SurveyRecord { found: true }`; if no, `SurveyRecord { found: false }`.
4. Negative survey suppresses re-exploration. Ranking adds a per-`(place, hypothesis)` damping factor when a fresh negative survey exists. The damping decays with the survey record's freshness, so eventually the agent can re-explore.
5. The negative survey is per-agent. Other agents who never visited keep believing the place might have food — until they hear about it through `ShareBelief` (FND-15). Per-agent surveys are not authoritative world facts; they are the agent's own learned negative belief about a (place, hypothesis) pair.
6. One new `EventTag::SurveyRecorded` for both positive and negative outcomes. Survey events are authoritative causal records (FND-29A), not optional debug.
7. Resource-arrival mismatches reuse S122's `ExpectationMismatch` surface. No duplicate event for "expected apples, found none" — that case is already a `FrameAssumption::CommodityAvailableAt` breach.

## Non-Goals

- Comprehensive `BeliefObservation` per-observation metadata. PR-7's broader proposal is rejected as already largely addressed by S101 + S114 + S122; this spec scopes only to the survey record for explicit exploration intent.
- "Expected information gain" or "expected survival value" quantification on `ExploreLocation`. Those are ranking inputs (S107 already provides the proactive surface); per-goal authoring would couple ranking to goal identity, violating FND-26.
- "Expected cost" / "abandonment conditions" on `ExploreLocation`. Cost is a planner output; abandonment is the existing `cognitive.transient_block_ticks` machinery applied to exploration goals.
- Cross-agent survey propagation. Surveys are per-agent learned state. A fellow agent learns about an empty Hillside Shelter only by hearing it through the existing `ShareBelief` channel — this spec does not add a new propagation primitive.
- Searching specific entities ("look for the missing person at the cabin"). Search of specific entities is owned by S59's search/rescue substrate, not this spec.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `SurveyRecord` is a concrete record of (place, hypothesis, found, confidence, tick), not an "exploration score" abstraction. The damping factor in ranking is derived from survey freshness, not stored as a separate score. |
| FND-5 (Carriers of Consequence) | A survey record carries downstream consequence: future ranking suppresses re-exploration; the agent's intent becomes inspectable in the decision trace. |
| FND-7 (Locality of Motion, Interaction, and Communication) | Surveys are per-agent. Other agents do not see another agent's survey memory directly; cross-agent propagation runs through `ShareBelief`. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Arrival-time hypothesis evaluation reads the agent's perception of co-located entities — pure FND-14A. The `found` flag reflects what the agent actually perceives, not what world state knows to be true (e.g., a hidden item under perception threshold yields `found: false`, which is the lawful outcome). |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Survey records carry source (the agent who made them), tick, place, and outcome. Negative knowledge ("I looked, found nothing") is first-class belief-state with provenance. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | An agent that never visited a place has no survey record and remains in legitimate ignorance about the hypothesis. Two agents may hold contradicting surveys (one stale "had food", one fresh "no food") — both first-class. |
| FND-17 (Surprise Comes From Violated Expectation) | Negative survey *is* the agent's surprise: "I expected food, I found none." The recorded outcome carries the expectation type so the trace shows what was violated. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents can survey the same place with different hypotheses (one looking for food, one for water) and produce orthogonal records. Per-agent perception fidelity differences yield different survey outcomes for the same place. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Survey records are exactly the kind of concrete learned state FND-22A asks for: explicit acquisition (arrival event), explicit decay (freshness-based pruning), explicit replacement (re-survey on return). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Perception writes `SurveyMemory`; AI reads `SurveyMemory` for ranking damping. No imperative cross-system call. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `SurveyMemory` is authoritative per-agent state. The ranking damping factor is derived per-tick from freshness; not stored. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The existing `ExploreLocation` shape (without `hypothesis`) is removed in favor of the new shape; all call sites updated. |
| FND-29 (Debuggability Is a Product Feature) | `SurveyRecorded` event records the (place, hypothesis, found, confidence) tuple. Decision-trace surfaces the suppression reason as "ExploreLocation suppressed: SurveyMemory has fresh negative for (Hillside Shelter, MayContainFood)." |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | One new `EventTag::SurveyRecorded` lands in the event log; survey records themselves persist in `SurveyMemory` until decayed. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers all 18 declarations. |

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Survey creation is local: the agent arrives, perceives co-located entities (existing perception path), evaluates the hypothesis, writes the record. Other agents learn via `ShareBelief`. No global query.
2. **Positive-feedback analysis.** (a) "Negative survey suppresses re-exploration → less perception of the place → survey decays without refresh → re-exploration eventually allowed → maybe negative again." The natural cycle is itself the dampener. (b) "Positive survey biases ranking toward the place → more visits → more positive surveys." Dampener: the underlying resource depletes (S127's `available_quantity`), creating a negative survey eventually.
3. **Concrete dampeners.** (a) Survey freshness decay (per-tick or per-perception-pass). (b) Resource depletion as observed by repeat surveys.
4. **Stored state vs. derived read-model.** Stored: `SurveyMemory.entries`, each with `(place, hypothesis, found, confidence, recorded_tick)`. Derived: per-tick freshness multiplier in ranking; per-tick eviction decision.

## Deliverables

### D1: `HypothesisKind` enum

In `crates/worldwake-core/src/goal.rs`:

```rust
/// What an exploring agent expects to find at the target place.
/// Drives both ranking input and arrival-time hypothesis evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum HypothesisKind {
    MayContainCommodity { commodity: CommodityKind },
    MayContainLatrine,
    MayContainWashBasin,
    MayContainSleepSite,
    /// Generic proactive exploration with no specific expectation.
    Proactive,
}
```

### D2: `ExploreLocation` extension

```rust
GoalKind::ExploreLocation {
    target_place: EntityId,
    motivating_need: ExplorationMotivation,
    hypothesis: HypothesisKind,
}
```

`ExplorationMotivation` is preserved (it captures the *why*); `HypothesisKind` adds the *what*. Where the existing `motivating_need = NeedDriven(Hunger)` was used, candidate generation derives `hypothesis = MayContainCommodity { commodity: agent's preferred food source }` (the consumable kind that satisfies hunger for this agent — read from existing food-mapping). Where `motivating_need = Proactive`, `hypothesis = Proactive`.

### D3: `SurveyRecord` and `SurveyMemory`

In `crates/worldwake-core/src/survey_memory.rs` (new module):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurveyRecord {
    pub place: EntityId,
    pub hypothesis: HypothesisKind,
    /// Whether the agent perceived an entity satisfying the hypothesis
    /// during the arrival window.
    pub found: bool,
    /// Confidence in the result, derived from perception fidelity at
    /// arrival time.
    pub confidence: Permille,
    pub recorded_tick: Tick,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurveyMemory {
    pub entries: Vec<SurveyRecord>,
}

impl Component for SurveyMemory {}

impl SurveyMemory {
    /// Returns the freshest matching record for (place, hypothesis),
    /// if any. Older matching records are pruned by `enforce_limits`.
    pub fn find(&self, place: EntityId, hypothesis: HypothesisKind) -> Option<&SurveyRecord> {
        self.entries.iter()
            .filter(|r| r.place == place && r.hypothesis == hypothesis)
            .max_by_key(|r| r.recorded_tick)
    }

    pub fn record(&mut self, record: SurveyRecord, capacity: usize, retention_ticks: u64) {
        // Replace any existing entry for the same (place, hypothesis);
        // append otherwise; evict by oldest tick on overflow.
        if let Some(existing) = self.entries.iter_mut()
            .find(|r| r.place == record.place && r.hypothesis == record.hypothesis)
        {
            *existing = record;
            return;
        }
        self.entries.push(record);
        // Bounded capacity per S127's ring-buffer pattern; evict oldest.
        if self.entries.len() > capacity {
            self.entries.sort_by_key(|r| r.recorded_tick);
            self.entries.remove(0);
        }
    }

    pub fn enforce_limits(&mut self, current_tick: Tick, retention_ticks: u64) {
        self.entries.retain(|r| {
            current_tick.0.saturating_sub(r.recorded_tick.0) <= retention_ticks
        });
    }
}
```

### D4: `SurveyRecorded` event tag

In `crates/worldwake-core/src/event_tag.rs`:

```rust
pub enum EventTag {
    // ... existing variants ...
    SurveyRecorded,
}
```

Payload in `crates/worldwake-core/src/decision_event_payload.rs`:

```rust
pub struct SurveyRecordedPayload {
    pub surveyor: EntityId,
    pub place: EntityId,
    pub hypothesis: HypothesisKind,
    pub found: bool,
    pub confidence: Permille,
}
```

### D5: Perception-time hypothesis evaluation

In `crates/worldwake-systems/src/perception.rs`, after the arrival-perception path (the existing "agent enters new place → record observations" code), check the agent's `IntentionFrame` for an active `ExploreLocation` goal targeting the just-entered place:

- If found: evaluate `hypothesis` against the agent's freshly-perceived entities at the place. Hypothesis evaluation:
  - `MayContainCommodity { commodity }`: any perceived `ResourceSource.commodity == commodity` with `available_quantity > 0`, or any `ItemLot` of `commodity` at the place.
  - `MayContainLatrine`: any perceived facility with the `Latrine` tag.
  - `MayContainWashBasin`: any perceived facility with the `WashBasin` workstation tag.
  - `MayContainSleepSite`: any perceived place with `SleepQualityProfile.recovery_modifier > 1000` (better than universal default), per S128.
  - `Proactive`: always `found = true` (the act of arriving satisfies proactive intent).
- Write `SurveyMemory` entry via `record(...)` with confidence derived from `PerceptionProfile.fidelity`.
- Emit `SurveyRecorded` event.

### D6: AI ranking integration

In `crates/worldwake-ai/src/ranking.rs`, the `ExploreLocation` ranking arm reads the agent's `SurveyMemory` for `(target_place, hypothesis)`:

- If a fresh negative survey exists (`found == false` and `current_tick - recorded_tick < negative_survey_damping_window`): multiply ranking score by `(1.0 - confidence × damping_strength) / 1.0`. A confident, fresh negative survey heavily damps the score.
- If a fresh positive survey exists: leave ranking unchanged (or boost slightly — the reliable positive evidence is mostly handled by S131).

Damping strength and window are per-agent fields on the existing `ExplorationProfile` (S80 already authored).

### D7: Resource-arrival mismatch reuse

When the agent's arrival evaluates `MayContainCommodity { commodity }` to `found = false`, and the agent had a `FrameAssumption::CommodityAvailableAt { commodity, place }` (S122), the existing `record_assumption_failure` path fires. No new event needed — S122/S110 already surface this case.

The S130 survey adds the "I checked here and found nothing" record even when no S122 assumption was present (e.g., the agent was exploring proactively and had no committed acquisition goal yet). The two systems are orthogonal.

### D8: Decision-trace surfacing

The decision-trace renders ranking-time damping as: `ExploreLocation { target: Hillside Shelter, hypothesis: MayContainCommodity { Apple } } damped by SurveyMemory: found=false at tick 312, confidence=850.`

### D9: Golden coverage

Add `crates/worldwake-ai/tests/golden_survey_records.rs`:

- Agent explores Hillside Shelter under hunger pressure with `hypothesis = MayContainCommodity { Apple }`. Place has no apples. Confirm `SurveyMemory` contains `SurveyRecord { found: false }` after arrival.
- Confirm next exploration cycle's ranking damps `ExploreLocation { Hillside Shelter, ... }` significantly enough to lose to alternatives.
- After `negative_survey_damping_window` ticks pass, confirm ranking damping fades and the agent will re-explore if frontier conditions change.
- Two-agent scenario: Agent A surveys Hillside Shelter empty; Agent B (who never visited) still ranks Hillside Shelter for exploration. Confirm survey is per-agent, not shared.

## SystemFn Integration

No new SystemFn. Survey writes happen inside the existing perception system's arrival-perception path. Survey decay piggybacks on the existing `enforce_limits` pattern from `experience.rs` (called during the per-tick belief maintenance). Per-agent capacity and retention come from `cognitive.survey_memory_capacity` and `cognitive.survey_memory_retention_ticks`.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `SurveyMemory` | Agent | Universal | `Default` (empty) — every agent has one for ranking damping to read |

`SurveyMemory` is universal per FND-22 Section 5 — every agent needs the field even if empty (the `enforce_limits` and `find` calls run unconditionally during ranking). New agent profile field `cognitive.survey_memory_capacity` (default `24`) and `cognitive.survey_memory_retention_ticks` (default `300`) live on the existing `CognitiveProfile`.

`HypothesisKind` is a value type embedded in the goal variant; not a component.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Perception | Writes `SurveyMemory` on arrival; emits `SurveyRecorded` event | State-mediated |
| AI ranking | Reads `SurveyMemory` for damping `ExploreLocation` candidates | State-mediated |
| Frame assumption (S122) | When `found=false` and `CommodityAvailableAt` was present, S122's existing path records the discrepancy; S130 adds the survey record orthogonally | State-mediated |
| Belief decay (S101) | Survey records decay through `enforce_limits` on the per-tick maintenance pass | State-mediated |
| Decision history (S110) | `SurveyRecorded` event lands in event log | State-mediated |

## Profile-Driven Parameters

Per-agent: `CognitiveProfile.survey_memory_capacity` (universal default 24), `CognitiveProfile.survey_memory_retention_ticks` (universal default 300), `ExplorationProfile.negative_survey_damping_window` (default 200), `ExplorationProfile.negative_survey_damping_strength` (default `Permille::new_unchecked(800)`).

No per-place authoring — survey records are emergent from agent behavior.

No magic numbers in agent-side code — all per-agent values flow through `CognitiveProfile` and `ExplorationProfile`.
