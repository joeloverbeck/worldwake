# S130: Survey Records and Frontier Disconfirmation

## Summary

When an agent travels somewhere expecting to find a resource and arrives to find none, the architecture should record "I looked here for X and found nothing" as concrete agent state — not erase the visit because the belief store has nothing to record. Today, `ExploreLocation { target_place, motivating_need }` (from S80) gets the agent to the place and S102's frontier-aware exploration drives the topology coverage. But the agent has no way to express "I checked Hillside Shelter for food and confirmed it is empty" — the absence becomes invisible to future planning, and the same agent can be drawn back to the same fruitless trip. This spec adds two carriers: `SurveyRecord` per (agent, place, hypothesis), and `HypothesisKind` on `ExploreLocation`. When the agent arrives, the perception pipeline checks whether the hypothesis was satisfied; if not, the survey records "checked, did not find" with confidence and tick. Future ranking suppresses re-exploration of confirmed-empty places under the same hypothesis until confidence decays. Folds in PR-7's narrow "arrival-time prediction error for resource sources" piece — when the agent expected `CommodityAvailableAt` (from S122) and arrives to find the source empty or absent, the existing `ExpectationMismatch` event surface carries the diff. Survey records are strictly per-agent in this spec; cross-agent propagation is deferred to a future spec (see Non-Goals).

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft. **Implementation deferred** until S128 lands `SleepQualityProfile` (see Dependencies). `specs/IMPLEMENTATION-ORDER.md` should be updated to move S130 from Wave 1 to Wave 2 (after S128) as part of S128's archival pass.

## Crates

- `worldwake-core` — `SurveyRecord` and `SurveyMemory` components, `HypothesisKind` enum on `ExploreLocation`, `SurveyRecorded` event tag with payload struct, new fields on `CognitiveProfile` and `ExplorationProfile`.
- `worldwake-sim` — new `survey_memory()` accessor on `GoalBeliefView`; `RuntimeBeliefView` impl forwarding to the underlying component read.
- `worldwake-systems` — perception system extension: when the agent arrives at a place with an active `ExploreLocation` intent targeting that place, evaluate hypothesis vs. perceived entities and write `SurveyMemory` entry. Decay rule piggybacks on the existing belief decay pass.
- `worldwake-ai` — `ExploreLocation` candidate generation populates `hypothesis` from the motivating need via a hardcoded need→hypothesis mapping; ranking damps `ExploreLocation` for (place, hypothesis) pairs with a fresh negative survey; decision-trace surfaces the damping reason.
- `worldwake-cli` — no `AgentDef` field for `SurveyMemory` (runtime-generated state, exempt from FND-22 scenario authoring); `spawn_agent` inserts `SurveyMemory::default()` for every agent. New profile fields land on existing `CognitiveProfileDef` and `ExplorationProfileDef`.

## Dependencies

- **S80 (Exploration Drive)** — completed. `ExploreLocation` substrate exists; this spec extends it.
- **S102 (Frontier-Aware Exploration)** — completed. Frontier selection feeds into the candidate generator that populates the new hypothesis.
- **S107 (Proactive Diversification)** — completed. `ExplorationMotivation::Proactive` exists; proactive exploration uses `HypothesisKind::Proactive`.
- **S128 (Sleep Episodes and Place-Quality Recovery)** — **hard**. `MayContainSleepSite` evaluation in D5 requires `SleepQualityProfile.recovery_modifier` introduced by S128. Implementation of S130 is deferred until S128 lands.
- **S122 (Frame Assumption — Commodity Availability)** — soft. Arrival-time mismatch on `CommodityAvailableAt` already surfaces through S109 discrepancy memory; this spec piggybacks the existing `ExpectationMismatch` event for the resource-not-found case rather than introducing a new event type for the same concern.
- **S101 (Activation-Based Belief Decay)** — soft. Survey records decay via the same per-tick maintenance pass that calls `enforce_limits` on other learned-state components.
- **S110 (Decision History Events)** — soft. `SurveyRecorded` event lands in the same `EventTag` / `decision_event_payload` infrastructure S110 already established.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 6: "Arrival should produce an explicit `SurveyRecord` … This lets an agent become disappointed without global correction. If Hillside Shelter contains no food, an agent who explored it should know 'I looked and did not find food,' while another agent who never visited it should remain ignorant." The narrative report shows Hillside Shelter never visited despite being structurally reachable: "no agent ever visited it … the place stayed dormant." Today, even if an agent did visit, the architecture has no way to remember "I checked here and there's no food" — the next exploration cycle can pick the same place again.

The motivating proposal lists five hypothesis values co-equally: `may_contain_food | may_contain_water | may_contain_latrine | may_contain_wash | may_offer_sleep_site`. Sleep-site exploration only becomes meaningful once S128's `SleepQualityProfile` introduces per-place recovery variation; until then, sleep is a uniform-quality action and the hypothesis has no observable to discriminate against. This is the architectural reason S128 is a hard dependency.

## Design Goals

1. `SurveyRecord` is per-(agent, place, hypothesis) state. The agent's `SurveyMemory` component holds a small bounded list (capped at `cognitive.survey_memory_capacity`, default 24).
2. Hypothesis is part of the goal. `ExploreLocation { target_place, motivating_need, hypothesis }` makes the agent's exploration intent inspectable. The hypothesis is what the agent expected to find.
3. Arrival-time evaluation happens in the perception pipeline. When the agent enters the place, the perception system checks "did the agent perceive an entity satisfying the hypothesis?" — if yes, `SurveyRecord { found: true }`; if no, `SurveyRecord { found: false }`.
4. Negative survey suppresses re-exploration. Ranking adds a per-`(place, hypothesis)` damping factor when a fresh negative survey exists. The damping decays with the survey record's freshness, so eventually the agent can re-explore.
5. The negative survey is per-agent learned state. An agent who never visited a place has no survey record and remains in legitimate ignorance about the hypothesis. Two agents may hold contradicting surveys (one stale "had food", one fresh "no food") — both first-class.
6. One new `EventTag::SurveyRecorded` for both positive and negative outcomes. Survey events are authoritative causal records (FND-29A), not optional debug.
7. Resource-arrival mismatches reuse S122's `ExpectationMismatch` surface. No duplicate event for "expected apples, found none" — that case is already a `FrameAssumption::CommodityAvailableAt` breach.

## Non-Goals

- Comprehensive `BeliefObservation` per-observation metadata. PR-7's broader proposal is rejected as already largely addressed by S101 + S114 + S122; this spec scopes only to the survey record for explicit exploration intent.
- "Expected information gain" or "expected survival value" quantification on `ExploreLocation`. Those are ranking inputs (S107 already provides the proactive surface); per-goal authoring would couple ranking to goal identity, violating FND-26.
- "Expected cost" / "abandonment conditions" on `ExploreLocation`. Cost is a planner output; abandonment is the existing `cognitive.transient_block_ticks` machinery applied to exploration goals.
- **Cross-agent survey propagation.** Surveys are strictly per-agent learned state in this spec. There is no extension to `TellTopic`, no `ShareBelief`-mediated survey transfer, and no other propagation primitive. A fellow agent who never visited Hillside Shelter remains ignorant of another agent's negative survey until they visit themselves. A future spec may add an explicit propagation surface (e.g., `TellTopic::SurveyRecord`) when the gameplay need arises; this one stays focused on the per-agent learning loop.
- Per-agent commodity preference for `MayContainCommodity` derivation. The need→hypothesis mapping in D2 is uniform across agents. Agent-specific dietary preferences are out of scope; a follow-on spec can introduce per-agent preferred-food state when consumption diversity matters.
- Searching specific entities ("look for the missing person at the cabin"). Search of specific entities is owned by S59's search/rescue substrate, not this spec.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `SurveyRecord` is a concrete record of (place, hypothesis, found, confidence, tick), not an "exploration score" abstraction. The damping factor in ranking is derived from survey freshness, not stored as a separate score. |
| FND-5 (Carriers of Consequence) | A survey record carries downstream consequence: future ranking suppresses re-exploration; the agent's intent becomes inspectable in the decision trace. |
| FND-7 (Locality of Motion, Interaction, and Communication) | Surveys are per-agent. No system queries another agent's survey memory on behalf of an agent. Cross-agent survey propagation is explicitly out of scope; the principle is satisfied by the absence of any propagation surface. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Arrival-time hypothesis evaluation reads the agent's perception of co-located entities — pure FND-14A. Hypothesis predicates against `ResourceSource.commodity / available_quantity`, `ItemLot.commodity`, `PlaceTag`, `WorkstationTag`, and `SleepQualityProfile.recovery_modifier` are reads of physical properties of co-located entities, eligible for FND-14A's same-tick authoritative-state read. No social or institutional facts are read by hypothesis evaluation. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Survey records carry source (the agent who made them), tick, place, and outcome. Negative knowledge ("I looked, found nothing") is first-class belief-state with provenance. Cross-agent travel of survey knowledge is deferred to a future spec; until then, only direct visit creates the record. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | An agent that never visited a place has no survey record and remains in legitimate ignorance about the hypothesis. Two agents may hold contradicting surveys (one stale "had food", one fresh "no food") — both first-class. |
| FND-17 (Surprise Comes From Violated Expectation) | Negative survey *is* the agent's surprise: "I expected food, I found none." The recorded outcome carries the expectation type so the trace shows what was violated. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents can survey the same place with different hypotheses (one looking for food, one for water) and produce orthogonal records. Per-agent perception fidelity differences yield different survey outcomes for the same place. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Survey records are exactly the kind of concrete learned state FND-22A asks for: explicit acquisition (arrival event), explicit decay (freshness-based pruning), explicit replacement (re-survey on return). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Perception writes `SurveyMemory`; AI reads `SurveyMemory` (via `GoalBeliefView::survey_memory()`) for ranking damping. No imperative cross-system call. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `SurveyMemory` is authoritative per-agent state. The ranking damping factor is derived per-tick from freshness; not stored. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The existing `ExploreLocation` shape (without `hypothesis`) is removed in favor of the new shape; all 60 destructuring/construction sites updated, no shim. |
| FND-29 (Debuggability Is a Product Feature) | `SurveyRecorded` event records the (place, hypothesis, found, confidence) tuple. Decision-trace surfaces the suppression reason as "ExploreLocation suppressed: SurveyMemory has fresh negative for (Hillside Shelter, MayContainCommodity { Apple })." |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | One new `EventTag::SurveyRecorded` lands in the event log; survey records themselves persist in `SurveyMemory` until decayed. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers the four required analyses (information-path, positive-feedback, dampeners, stored-vs-derived) per `docs/spec-drafting-rules.md`. |

## FND-01 Section H — Causal Hooks Declaration

Per `docs/spec-drafting-rules.md`, the four required analyses are:

1. **Information-path analysis.** Survey creation is local: the agent arrives, perceives co-located entities (existing perception path, FND-14A), evaluates the hypothesis, writes the record on its own `SurveyMemory` component. Reading by ranking goes through `GoalBeliefView::survey_memory()` — the standard belief-view-mediated read pattern. No global query, no cross-agent path. Surveys do not propagate cross-agent in this spec.
2. **Positive-feedback analysis.** (a) "Negative survey suppresses re-exploration → less perception of the place → survey decays without refresh → re-exploration eventually allowed → maybe negative again." The natural cycle is itself the dampener. (b) "Positive survey biases ranking toward the place → more visits → more positive surveys." Dampener: the underlying resource depletes (S127's `available_quantity`), creating a negative survey eventually.
3. **Concrete dampeners.** (a) Survey freshness decay (per-tick `enforce_limits` on the per-agent maintenance pass). (b) Resource depletion as observed by repeat surveys. Both are physical world processes, not numeric clamps.
4. **Stored state vs. derived read-model.** Stored: `SurveyMemory.entries`, each with `(place, hypothesis, found, confidence, recorded_tick)`. Derived per-tick: damping multiplier in ranking; eviction decision in `enforce_limits`.

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

`CommodityKind` is `Copy + Eq + Ord + Hash` (verified at `crates/worldwake-core/src/items.rs:8`), so the variant satisfies `GoalKind`'s existing `Copy` derive without widening enum size meaningfully.

### D2: `ExploreLocation` extension and need-to-hypothesis mapping

```rust
GoalKind::ExploreLocation {
    target_place: EntityId,
    motivating_need: ExplorationMotivation,
    hypothesis: HypothesisKind,
}
```

`ExplorationMotivation` is preserved (it captures the *why*); `HypothesisKind` adds the *what*. All ~60 existing destructuring/construction sites of `ExploreLocation` (search: `grep -rn "GoalKind::ExploreLocation {" crates/`) are updated to populate the new field — no shim, per FND-28.

**Hardcoded need-to-hypothesis mapping** (for `motivating_need = ExplorationMotivation::NeedDriven(need_id)`):

| `HomeostaticNeedId` | `HypothesisKind` |
|---------------------|------------------|
| `Hunger`            | `MayContainCommodity { commodity: CommodityKind::Apple }` |
| `Thirst`            | `MayContainCommodity { commodity: CommodityKind::Water }` |
| `Bladder`           | `MayContainLatrine` |
| `Dirtiness`         | `MayContainWashBasin` |
| `Fatigue`           | `MayContainSleepSite` |

For `motivating_need = ExplorationMotivation::Proactive`, `hypothesis = HypothesisKind::Proactive`.

The mapping is implemented as a `const fn need_hypothesis(need: HomeostaticNeedId) -> HypothesisKind` in `crates/worldwake-ai/src/candidate_generation.rs` (the file that owns `emit_*_goal` for needs). Per-agent dietary variation (e.g., one agent prefers Bread, another prefers Apple) is a non-goal — the mapping is uniform across agents in this spec.

**Goal-key identity note**: Adding `hypothesis` to `GoalKind::ExploreLocation` changes `GoalKind`'s `Hash` output. Two `ExploreLocation` goals with the same `(target_place, motivating_need)` but different `hypothesis` therefore have different `GoalKey`s and are distinct goals for commitment, blocker memory, and discrepancy memory purposes. This is intentional: a Hunger-driven and a Thirst-driven exploration of the same place produce orthogonal surveys and should not collide. Binding identity (`matches_binding`) continues to use `target_place` only and is unchanged — Travel ops bind on place, not on hypothesis.

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
    /// Returns the freshest matching record for (place, hypothesis), if any.
    pub fn find(&self, place: EntityId, hypothesis: HypothesisKind) -> Option<&SurveyRecord> {
        self.entries.iter()
            .filter(|r| r.place == place && r.hypothesis == hypothesis)
            .max_by_key(|r| r.recorded_tick)
    }

    /// Replace any existing entry for the same (place, hypothesis); append
    /// otherwise; evict by oldest tick on capacity overflow.
    pub fn record(&mut self, record: SurveyRecord, capacity: usize) {
        if let Some(existing) = self.entries.iter_mut()
            .find(|r| r.place == record.place && r.hypothesis == record.hypothesis)
        {
            *existing = record;
            return;
        }
        self.entries.push(record);
        if self.entries.len() > capacity {
            self.entries.sort_by_key(|r| r.recorded_tick);
            self.entries.remove(0);
        }
    }

    /// Drop entries older than `profile.survey_memory_retention_ticks`.
    /// Called from the per-tick belief maintenance pass alongside the
    /// existing `RouteExperience::enforce_limits` and
    /// `SourceReliability::enforce_limits` calls.
    pub fn enforce_limits(&mut self, current_tick: Tick, profile: &CognitiveProfile) {
        let retention = profile.survey_memory_retention_ticks;
        self.entries.retain(|r| {
            current_tick.0.saturating_sub(r.recorded_tick.0) <= retention
        });
    }
}
```

`Vec<SurveyRecord>` is consistent with the existing project convention for bounded learned-state collections (`WoundList.wounds: Vec<Wound>`, `DemandMemory.observations: Vec<DemandObservation>`). Iteration order is insertion order; determinism is preserved because no observable output depends on traversal order beyond `find`'s `max_by_key`.

### D4: `SurveyMemory` registration and default insertion

In `crates/worldwake-core/src/component_schema.rs`, register `SurveyMemory` via the existing `with_component_schema_entries!` macro on `EntityKind::Agent`, generating `get_component_survey_memory`, `set_component_survey_memory`, and the standard query/iter accessors (mirroring the existing `WoundList` registration at component_schema.rs:83–106).

In `crates/worldwake-cli/src/scenario/mod.rs::spawn_agent()`, insert `SurveyMemory::default()` for every agent unconditionally (universal component, `Default` impl present). No `AgentDef` field is added — `SurveyMemory` is runtime-generated state and exempt from FND-22 scenario authoring per `docs/spec-drafting-rules.md` Section 5.

### D5: `SurveyRecorded` event tag

In `crates/worldwake-core/src/event_tag.rs`, add a new variant to `EventTag`:

```rust
pub enum EventTag {
    // ... existing 39 variants ...
    SurveyRecorded,
}
```

Payload in `crates/worldwake-core/src/decision_event_payload.rs`, alongside `GoalCommittedPayload`, `PlanAdoptedPayload`, etc.:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurveyRecordedPayload {
    pub surveyor: EntityId,
    pub place: EntityId,
    pub hypothesis: HypothesisKind,
    pub found: bool,
    pub confidence: Permille,
}
```

`EventTag` is consumed by tag-add APIs (`txn.add_tag(...)`) and tag-membership tests, not by exhaustive matches in downstream crates (verified: `grep -rn "match.*EventTag::"` returns 0 exhaustive sites in ai/systems/cli), so adding a variant has no downstream blast radius beyond the new emission.

### D6: Perception-time hypothesis evaluation

In `crates/worldwake-systems/src/perception.rs`, after the arrival-perception path (the existing "agent enters new place → record observations" code) and **before** any per-tick goal-satisfaction sweep that might clear the agent's `IntentionFrame`, check the agent's `IntentionFrame.goal` for an active `ExploreLocation` goal targeting the just-entered place. The lifecycle ordering matters because `is_satisfied(ExploreLocation)` returns true on arrival (`effective_place == target_place`) — survey evaluation must run while the goal is still the active intent.

If found, evaluate `hypothesis` against the agent's freshly-perceived entities at the place (FND-14A: same-tick co-located reads of physical properties of `ResourceSource`, `ItemLot`, `PlaceTag`, `WorkstationTag`, `SleepQualityProfile` are belief-equivalent):

- `MayContainCommodity { commodity }` → any perceived `ResourceSource` with `commodity == hypothesis.commodity && available_quantity > Quantity::zero()`, OR any `ItemLot` of that commodity at the place.
- `MayContainLatrine` → place carries `PlaceTag::Latrine` (verified: `PlaceTag` enum at `crates/worldwake-core/src/topology.rs:11–26`; `Latrine` is a place-level tag, not a workstation tag).
- `MayContainWashBasin` → place hosts a workstation tagged `WorkstationTag::WashBasin` (verified: `WorkstationTag` enum at `crates/worldwake-core/src/production.rs:10–34`; `WashBasin` is a facility-level tag).
- `MayContainSleepSite` → place carries `SleepQualityProfile` with `recovery_modifier > Permille::new(1000)` (better-than-universal-default; per S128 — hard dependency).
- `Proactive` → always `found = true` (the act of arriving satisfies proactive intent).

Write the `SurveyMemory` entry via `record(...)` with `confidence = profile.observation_fidelity` (the existing `PerceptionProfile.observation_fidelity` field at `crates/worldwake-core/src/belief.rs:2516`; the field is named `observation_fidelity`, not `fidelity`). Capacity is read from `cognitive.survey_memory_capacity`.

Emit `EventTag::SurveyRecorded` with `SurveyRecordedPayload`.

### D7: AI ranking integration

In `crates/worldwake-ai/src/ranking.rs`, the `ExploreLocation` ranking arm (current call site at `ranking.rs:1096` → `exploration_motive(context, motivating_need) -> u32`) is wrapped with a damping step.

Read the agent's survey memory through `GoalBeliefView::survey_memory()` (new accessor — see D8) for `(target_place, hypothesis)`. Compute the damping factor using **integer/Permille arithmetic** (CLAUDE.md determinism: no floats):

```rust
fn survey_damping_factor(
    survey: Option<&SurveyRecord>,
    current_tick: Tick,
    profile: &ExplorationProfile,
) -> Permille {
    let Some(record) = survey else { return Permille::new(1000).unwrap() };
    if record.found {
        return Permille::new(1000).unwrap(); // positive surveys do not damp
    }
    let age = current_tick.0.saturating_sub(record.recorded_tick.0);
    if age >= profile.negative_survey_damping_window as u64 {
        return Permille::new(1000).unwrap(); // expired; full score
    }
    // multiplier = 1000 - (confidence * damping_strength / 1000), in Permille units
    let attenuation = (record.confidence.value() as u32)
        .saturating_mul(profile.negative_survey_damping_strength.value() as u32)
        / 1000;
    Permille::new(1000u32.saturating_sub(attenuation).min(1000)).unwrap()
}
```

The existing `exploration_motive(...) -> u32` return is multiplied by the Permille factor (using the project's existing Permille-multiply-into-u32 helper, mirroring existing weighted score composition in ranking.rs). A confident, fresh negative survey (confidence 1000, strength 800) attenuates the score to ~200/1000 of its original value.

Damping window and strength come from per-agent fields on the existing `ExplorationProfile` (added in D9).

### D8: `GoalBeliefView::survey_memory()` accessor

In `crates/worldwake-sim/src/belief_view.rs`, extend the `GoalBeliefView` trait with:

```rust
fn survey_memory(&self, agent: EntityId) -> Option<&SurveyMemory> {
    None // default impl — view backends without survey access return None
}
```

Add a backing implementation on `RuntimeBeliefView` that forwards to `world.get_component_survey_memory(agent)`. Update the `impl_goal_belief_view!` macro / blanket impl forwarding to include the new method.

This is the standard pattern used by sibling memory accessors (`discrepancy_memory`, `blocker_memory`, `repair_memory`, `learned_opportunity_memory`) and is required because the AI ranking layer reads beliefs through `GoalBeliefView`, never directly from world components.

### D9: Profile field additions

Extend two existing per-agent profile components in `worldwake-core` and their scenario `*Def` mirrors in `worldwake-cli`:

**`CognitiveProfile`** (`crates/worldwake-core/src/cognitive_profile.rs`):

| Field | Type | Default |
|-------|------|---------|
| `survey_memory_capacity` | `usize` | `24` |
| `survey_memory_retention_ticks` | `u64` | `300` |

**`ExplorationProfile`** (`crates/worldwake-core/src/exploration.rs`):

| Field | Type | Default |
|-------|------|---------|
| `negative_survey_damping_window` | `u32` | `200` (ticks) |
| `negative_survey_damping_strength` | `Permille` | `Permille::new_unchecked(800)` |

Mirror updates in `crates/worldwake-cli/src/scenario/types.rs` `CognitiveProfileDef` and `ExplorationProfileDef`, with serde defaults so existing scenarios deserialize unchanged. Update the corresponding `Default` impls and any unit tests asserting field counts.

### D10: Resource-arrival mismatch reuse

When the agent's arrival evaluates `MayContainCommodity { commodity }` to `found = false`, and the agent had a `FrameAssumption::CommodityAvailableAt { commodity, place }` (S122) on the active `IntentionFrame`, the existing `record_assumption_failure` path (`crates/worldwake-ai/src/agent_tick/frame.rs:596`) fires alongside the survey write. No new event needed — S122/S110 already surface this case via `EventTag::ExpectationMismatch`.

The S130 survey adds the "I checked here and found nothing" record even when no S122 assumption was present (e.g., the agent was exploring proactively and had no committed acquisition goal yet). The two systems are orthogonal and both can fire on the same arrival tick.

### D11: Decision-trace surfacing

In `crates/worldwake-ai/src/decision_trace.rs`, extend the candidate diagnostics to carry an optional damping reason. Add:

```rust
pub enum CandidateDampingReason {
    SurveyMemoryNegative {
        place: EntityId,
        hypothesis: HypothesisKind,
        recorded_tick: Tick,
        confidence: Permille,
    },
}
```

Attach as `damping: Option<CandidateDampingReason>` on the existing per-candidate diagnostic (alongside `CandidateSuppressionDiagnostic`). The trace renderer formats it as: `ExploreLocation { target: Hillside Shelter, hypothesis: MayContainCommodity { Apple } } damped by SurveyMemory: found=false at tick 312, confidence=850.`

### D12: Golden coverage

Add `crates/worldwake-ai/tests/golden_survey_records.rs`:

- **Sub-test 1 — negative survey is written and damps re-exploration.** Agent explores Hillside Shelter under hunger pressure with `hypothesis = MayContainCommodity { Apple }`. Place has no apples. After arrival, confirm `SurveyMemory` contains `SurveyRecord { found: false }`. On the next exploration cycle, confirm `ExploreLocation { Hillside Shelter, ... }` ranking score is damped enough to lose to alternatives.
- **Sub-test 2 — damping fades.** After `negative_survey_damping_window` ticks pass, confirm ranking damping disappears and the agent will re-explore Hillside Shelter when frontier conditions favor it.
- **Sub-test 3 — surveys are per-agent.** Two agents in the same scenario; Agent A surveys Hillside Shelter empty; Agent B (who never visited) still ranks Hillside Shelter for exploration normally. Confirm survey is per-agent, not shared.
- **Sub-test 4 — goal-identity collision is benign.** Same agent emits `ExploreLocation { Hillside Shelter, NeedDriven(Hunger), MayContainCommodity { Apple } }` and `ExploreLocation { Hillside Shelter, NeedDriven(Thirst), MayContainCommodity { Water } }` in the same cycle. Confirm both candidates are valid distinct goals (different `GoalKey`), do not collide in commitment or blocker memory, and produce orthogonal surveys after arrival.

## SystemFn Integration

No new SystemFn. Survey writes happen inside the existing perception system's arrival-perception path. Survey decay piggybacks on the existing per-tick belief maintenance that calls `RouteExperience::enforce_limits` and `SourceReliability::enforce_limits` (`crates/worldwake-core/src/experience.rs:22, 91`); add an analogous `SurveyMemory::enforce_limits(current_tick, &cognitive_profile)` call in the same pass. Per-agent capacity and retention come from `cognitive.survey_memory_capacity` and `cognitive.survey_memory_retention_ticks`.

## Component Registration

| Component | EntityKind | Classification | Default | Registration |
|-----------|-----------|----------------|---------|--------------|
| `SurveyMemory` | Agent | Universal | `Default` (empty) — every agent has one for ranking damping to read | D4 (`component_schema.rs` macro entry + `spawn_agent` insertion) |

`SurveyMemory` is universal per `docs/spec-drafting-rules.md` Section 5 — every agent needs the field even if empty (the `enforce_limits` and `find` calls run unconditionally during ranking and per-tick maintenance). It is runtime-generated state with no scenario-authorable surface, so no `AgentDef` field is added (analogous to `WoundList`'s default insertion path).

`HypothesisKind` is a value type embedded in the goal variant; not a component.

`CognitiveProfile` and `ExplorationProfile` field additions (D9) extend existing universal components and are scenario-authorable through the existing `CognitiveProfileDef` / `ExplorationProfileDef` mirrors.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Perception (`worldwake-systems`) | Writes `SurveyMemory` on arrival; emits `SurveyRecorded` event | State-mediated |
| AI ranking (`worldwake-ai`) | Reads `SurveyMemory` via `GoalBeliefView::survey_memory()` for damping `ExploreLocation` candidates | State-mediated (belief-view-mediated) |
| Frame assumption (S122, `worldwake-ai`) | When `found=false` and `CommodityAvailableAt` was present on the active `IntentionFrame`, S122's existing `record_assumption_failure` path records the discrepancy; S130 adds the survey record orthogonally on the same tick | State-mediated |
| Belief decay (S101, `worldwake-core`) | Survey records decay through `enforce_limits(current_tick, &CognitiveProfile)` on the per-tick maintenance pass | State-mediated |
| Decision history (S110, event log) | `SurveyRecorded` event lands in event log with full payload | State-mediated |

## Profile-Driven Parameters

Per-agent fields added in D9:

- `CognitiveProfile.survey_memory_capacity` (default `24`)
- `CognitiveProfile.survey_memory_retention_ticks` (default `300`)
- `ExplorationProfile.negative_survey_damping_window` (default `200` ticks)
- `ExplorationProfile.negative_survey_damping_strength` (default `Permille::new_unchecked(800)`)

No per-place authoring — survey records are emergent from agent behavior.

No magic numbers in agent-side code — all per-agent values flow through `CognitiveProfile` and `ExplorationProfile`.
