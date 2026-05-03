# S133: Source Composite Tiebreaker for Same-Commodity Acquisition

**Status**: DRAFT — proposed 2026-05-03 to recover S131's repeated-game intent without the cross-category goal-rank perturbation introduced by the rolled-back S131SOURELWAI-004 motive-additive composite.

## Summary

S131's architecture and data substrate are correct: agents retain per-source `average_wait_ticks` and `last_observed_capacity` against `(entity, commodity)` keys, fed by the resource-extraction grant path, the facility-queue grant path, and per-tick perception. The S131 spec also correctly identified the consumer surface — the per-candidate source-reliability evaluation in `apply_source_reliability_discount`. Where S131 misfired was D4's *integration shape*: the composite added `capacity_signal` (raw `u16` capacity, ~12–20 in scenarios) and subtracted `wait_penalty` (≈ tens) directly to/from `motive_score` (which lives in the **0…1,000,000** range as `score_product(weight, pressure)`). Two failure modes followed, both observed in CI:

1. **Cross-category bias.** Only `AcquireCommodity` / `RestockCommodity` goals have a single source entity in `evidence_entities`, so only those goal kinds hit the composite. `Wash`, `Sleep`, `Relieve`, `ConsumeOwnedCommodity`, `Patrol`, etc. get nothing. Every co-located harvestable source therefore silently bumped acquisition-goal motive upward by 12–20 vs every other goal kind, with no architectural intent for that bias.
2. **Tiebreaker leak.** The sort comparator at `compare_ranked_goals` orders by `motive_score` directly. When goals are within ±20 of each other (which is exactly the regime several survival scenarios sit in for short stretches each cycle), the additive bonus flipped sorting and starved goals like `Drink` and `Wash`. Four pre-existing goldens (`survival_drive_escalation_lands_row_four`, `survival_offices_proves_force_law_uptake`, `survival_preferences_keeps_proactive_diversification_alive_under_survival`, `survival_tell_lands_row_five`) failed deterministically as a consequence.

The S131SOURELWAI-004 motive integration was rolled back on 2026-05-03 in the same change that drafts this spec: `apply_source_reliability_discount` and the pending-failures variant compute the failure-ratio trust discount only; the wait/capacity fields remain populated on `SourceReliabilityDiscount` but `wait_penalty = 0` and `capacity_signal = 0` (no motive perturbation). The wait observation hooks, perception capacity hook, and `PreferenceProfile.wait_sensitivity_weight` all remain live — only the consumer was removed.

S133 reauthors the consumer as a **same-commodity sub-rank tiebreaker** that operates within `(commodity, purpose)` peer sets and never touches `motive_score` or any cross-category comparison. Per FND-26 (state-mediated, not magnitude-leak) and FND-27 (composite is a derived view, never replaces motive truth), the right place for "this source is better than that source for the same goal" is downstream of the cross-goal rank, not inside it.

## Phase and Status

Phase 10: Survival Mechanic Depth — adjunct (correction of S131's D4). Follows S131's data substrate (which remains live) and supersedes its motive-integration deliverable. No hard external dependency beyond what S131 already established.

## Crates

- `worldwake-ai` — new `source_composite_tiebreaker` module exposing `SourceCompositeRank` (struct with permille factors and the composite product), the per-candidate computation, and the comparator integration. Removes the now-vestigial `wait_penalty` and `capacity_signal` fields from `SourceReliabilityDiscount` (they are zero-filled today by the rollback). Extends `compare_ranked_goals` with a `SourceComposite` ordering dimension that fires only when two entries share `(GoalKind::AcquireCommodity { commodity, purpose, .. }, anchor-place-or-none)` keying — i.e. siblings in the candidate-generation sense. Surfaces the new dimension in `RankedGoalComparisonDimension` so the decision trace can attribute the tiebreaker.
- `worldwake-core` — no new component; no new event tag. (`SourceReliabilityDiscount` lives in `worldwake-ai::decision_trace`, not core.)
- `worldwake-systems` — no change. Wait/capacity write paths from S131 stay as-is.
- `worldwake-cli` — no change. `PreferenceProfile.wait_sensitivity_weight` remains the per-agent dial; default `pm(150)` is preserved.

## Dependencies

- S131 (Source Reliability Wait and Capacity Extension) — **completed** (archived 2026-05-03). Provides `ReliabilityRecord.{average_wait_ticks, wait_observation_count, last_observed_capacity, last_observed_capacity_tick}`, `PreferenceProfile.wait_sensitivity_weight`, the queue/perception write hooks, and `SourceReliabilityDiscount`. S133 consumes those fields; it does not alter the write paths. The S131 archived spec retains its D1, D2, D3, D5 (and most of D6) as accurate descriptions of the live substrate; only the D4 motive integration was rolled back.
- E07 / E14 / E15 — completed. `SourceReliability` and `PreferenceProfile` registration on agents.
- S110 (Decision History Events) — completed (soft). The new `SourceComposite` decision-trace surface piggybacks on the existing trace pipeline; no new event tag.

## Motivating Evidence

The four golden failures observed on 2026-05-03 (post-`6aba83b0` "Implemented S131."):

- `survival_drive_escalation_lands_row_four`: Agent A never commits **Drink** in 1440 ticks (committed: eat/wash/sleep/relieve/harvest:Apples/travel/pick_up — drink and harvest:Water both absent).
- `survival_offices_proves_force_law_uptake`: Claimant Rhea never commits **Drink**.
- `survival_preferences_keeps_proactive_diversification_alive_under_survival`: familiar-orchard depletion never converts into a stored `failed_attempts > 0` reliability record because the agent's path past the depleted source is reordered by the composite.
- `survival_tell_lands_row_five`: Scout Una's dirtiness stays critical for **725 consecutive ticks** (max allowed 700) — wash relief comes too late because acquisition goals are pulled forward.

Bisection confirmed `2b6c34b2` (S131SOURELWAI-004) as the first commit at which all four fail. A targeted experiment that zeroed only `capacity_signal` in the composite restored all four tests *and* the wait-history goldens, while only the focused capacity-signal contract test failed (as expected — that test was the new behavior). This pinpoints the additive integration as the architectural fault, not the data substrate.

The repeated-game intelligence motivation from PR-8 (`reports/proposed-gameplay-mechanic-changes.md` Section 8) is unchanged: agents who repeatedly contend for an orchard or a well should learn to prefer alternatives, and agents who observe a source is depleted should weigh it accordingly. S133 lands that learning *as a sub-rank within the goal* instead of *as a perturbation of the goal-vs-goal rank*.

## Design Goals

1. **No `motive_score` mutation.** The composite never adds to or subtracts from `motive_score`. Cross-category ranking (Wash vs AcquireCommodity vs Sleep) is determined entirely by drive pressure × utility weight × escalation multiplier, exactly as it was before SOURELWAI-004.
2. **Same-commodity tiebreaker.** The composite governs ordering between two `AcquireCommodity { commodity, purpose, quantity_target }` opportunities (or two `RestockCommodity { commodity }` opportunities) that target different sources of the same commodity. Different-commodity comparisons fall through to the existing tiebreaker stack.
3. **Permille-scale factors.** Trust, wait, and capacity all express themselves as permille modifiers in `[0, 1000]`, multiplied together to a composite permille. This is dimensionally consistent and bounded, regardless of raw quantity scale.
4. **Profile-driven scale normalizers.** `wait_sensitivity_weight` (existing) and the new `capacity_observation_weight` (one new field on `PreferenceProfile`) determine how strongly each axis pulls the composite away from its 1000-permille neutral. Per FND-22 these stay per-agent.
5. **Stale capacity contributes neutrally.** Once `current_tick - last_observed_capacity_tick > memory_retention_ticks`, the capacity factor returns to 1000 (no bonus, no penalty). The data is preserved; only its weight in current ranking decays.
6. **Empty-source observation is a real signal.** If `last_observed_capacity == 0` and the observation is fresh, the capacity factor is **below** 1000 — the agent's most recent observation said the source had nothing. (Distinct from "never observed," which produces no record.)
7. **Failure ratio remains a motive discount.** S131's failure-ratio path through `apply_source_reliability_discount` (now `source_reliability_failure_discount`) stays. Repeated failures *should* lower a goal's motive because the goal-vs-goal rank should reflect "this acquisition is unlikely to succeed at this source." S133 layers the composite *on top of* that pre-existing motive discount.
8. **Decision-trace observability.** The composite, its three factors, the chosen source, and the rejected siblings appear in the planning trace under a new `SourceComposite` ranking dimension, so observer reports and goldens can verify the tiebreaker fired with attributable factors.
9. **Determinism preserved.** All factors are integer math on permille values. No floats. `BTreeMap`/`BTreeSet` iteration order continues to govern any tie within the composite itself.

## Non-Goals

- **Cross-commodity comparison.** Apple sources do not get composite-ranked against Water sources. The hunger vs thirst pressure axis already governs that — the composite never enters it.
- **Cross-goal-kind comparison.** Wash vs AcquireCommodity ordering is unchanged. The composite only fires when both compared entries share the same `(commodity, purpose)` key (or `(commodity)` for `RestockCommodity`).
- **New `SurvivalHabit` or `BlockedIntentRecord` substrate.** S131 already rejected those as speculative. S133 does not revive them.
- **Cross-agent reliability sharing.** `ShareBelief` remains the only cross-agent path. Per-agent learning stays per-agent (FND-15).
- **A new event tag.** The composite is a derived per-tick scoring artifact (FND-27); it has no place in the append-only event log beyond the decision-history payload that already records ranked candidates.
- **Pre-rank candidate deduplication.** S133 does not drop sibling candidates from the rank; all opportunities for the chosen `(commodity, purpose)` remain in the ranked vec, ordered by composite. Dropping siblings would deny the planner downstream fallback structure when its first choice fails — and the agent's existing replan / pending-failure path would then have to regenerate them anyway.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | The composite is a derived per-tick read-model over the concrete observable quantities S131 already stores (`average_wait_ticks`, `last_observed_capacity`, `last_observed_capacity_tick`, `successful_acquisitions`, `failed_attempts`). Stored state remains the truth. |
| FND-5 (Carriers of Consequence) | Wait and capacity memories continue to carry consequence — agents who learn the orchard is reliably contested choose the alternative source. The consequence path is intra-commodity rerank instead of cross-category motive bump. |
| FND-7 (Locality of Motion, Interaction, and Communication) | All learning is per-agent. Cross-agent propagation requires `ShareBelief`. Composite is read-only over the agent's own `SourceReliability`. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Inherited from S131. Capacity observation is co-located perception of `ResourceSource.available_quantity`; wait observation is the agent's own activity history at the queue. Both fall inside FND-14A. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Inherited from S131. The composite reads only the local agent's `SourceReliability`. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | An agent with no record for a source contributes the neutral 1000-permille factor on every axis — i.e. uncertainty defaults to "no opinion," not "best" or "worst." Stale capacity decays to neutral, not zero. The composite is a soft re-ranker, not a hard filter. |
| FND-21 (Intentions Are Revisable Commitments) | A goal currently anchored on a contested source can be reranked to its sibling on the next planning tick if wait observations have shifted the composite. The goal-kind doesn't change; only the chosen source. |
| FND-22 (Agent Diversity Through Concrete Variation) | `wait_sensitivity_weight` (existing) and `capacity_observation_weight` (new) make the composite per-agent. Two agents with identical reliability memories rerank differently because they weight wait and capacity differently. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Stored state on `ReliabilityRecord` remains untouched. The composite is a derived view; the preference shift is the agent's choice in the next plan, recorded through the existing decision-trace and event-history paths. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Contention queue and resource-extraction handlers write `SourceReliability`; perception writes `SourceReliability`; AI ranking reads `SourceReliability`. No cross-system call is added. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | The composite is explicitly a per-tick derived score with no persistence. The fix S133 represents is precisely a re-classification of S131's D4 from "motive truth" back to "derived per-tick view." |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The motive-additive composite path is *removed*, not deprecated, before S133 lands its replacement. The vestigial `wait_penalty` and `capacity_signal` fields on `SourceReliabilityDiscount` (zero-filled today) are deleted by D1 below; no shim, no parallel route. The three S131 motive-integration goldens (139/140/141) were deleted, not gated. S133 reauthors them under its own deliverables. |
| FND-29 (Debuggability Is a Product Feature) | New `SourceCompositeRank` decision-trace struct, new `RankedGoalComparisonDimension::SourceComposite`, observer Section 3/4 line per D5. Goldens at D6 cover both the firing path and the no-fire path. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | No new event tag. Composite reads existing causally-traceable data: queue grant promotions (already emit `EventTag::QueueGrantPromoted`) and perception writes (already emit through perception's existing system events). |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | S133 is a *consumer-only* extension on top of S131's existing system. Section H below declares the four points relevant to a consumer-shape change. |

## FND-01 Section H — Causal Hooks Declaration

This is a consumer-side reranker; it adds no new system tick, no new event, no new component. Section H below covers the four declarations relevant to that shape.

1. **Information-path analysis.** Inputs are the agent's own `SourceReliability` (per-agent, written by the existing facility-queue grant hook, the existing resource-extraction grant hook, and the existing perception capacity hook), the agent's own `PreferenceProfile`, and the per-tick `current_tick` for freshness. No global state read. Cross-agent transfer continues through `ShareBelief` per FND-15. The composite output is consumed only by the per-agent planning trace and the per-agent ranked agenda.
2. **Positive-feedback analysis.** "Agent learns source A is contested → chooses source B → A's queue contention drops → A's queue-time observation eventually decays past `memory_retention_ticks` → agent re-evaluates A on later observation." Dampener: `memory_retention_ticks` (already on `PreferenceProfile`); the EMA cap on `wait_observation_count = 32` (already on `ReliabilityRecord::observe_wait`); the neutral-default behavior — a stale or never-observed capacity contributes 1000 permille (no bias either way), so an agent who has been avoiding source A long enough for the record to age out automatically returns to neutral instead of permanently locking out.
3. **Concrete dampeners.** (a) Three independent permille factors clamp(0, 2000) before multiplication, then the product clamp(0, 2000) → no axis can run away, no axis can drive the composite negative. (b) The composite only orders within a `(commodity, purpose)` peer set; cross-category ranking is dampened by construction. (c) The motive-scale failure-ratio discount remains a separate motive perturbation, so the composite never has to encode "I keep failing here" — that's the failure-ratio path's job. (d) `acquisition_failure_threshold` on `ExplorationProfile` continues to drive the agent toward novel sources as a separate exploration dampener.
4. **Stored state vs. derived read-model.** Stored: the existing `ReliabilityRecord` fields (no change). Derived: `SourceCompositeRank { trust_factor_permille, wait_factor_permille, capacity_factor_permille, composite_permille, source_entity, commodity }`, computed per-candidate per-tick by D2 and surfaced through D5. Not stored on the agent; not persisted to event log; lives only in the per-tick `AgendaEntry` and decision-trace payload.

## Deliverables

### D1: Strip vestigial fields from `SourceReliabilityDiscount`

In `crates/worldwake-ai/src/decision_trace.rs`, remove the four fields the rollback reduced to zero-only state:

```rust
pub struct SourceReliabilityDiscount {
    pub source_entity: EntityId,
    pub commodity: CommodityKind,
    pub failure_ratio_permille: u32,
    pub pre_discount_motive: u32,
    pub post_discount_motive: u32,
    // Removed: average_wait_ticks, wait_penalty,
    // last_observed_capacity, capacity_freshness_ticks, capacity_signal.
    // Wait/capacity surface migrates to SourceCompositeRank (D2).
}
```

Update the `Display` formatting at `decision_trace.rs:1952–1961` from

```
source_reliability=entity=_ commodity=_ failure=_ wait_avg=_ wait_pen=_ cap=_ cap_age=_ cap_sig=_ pre=_ post=_
```

back to

```
source_reliability=entity=_ commodity=_ failure=_ pre=_ post=_
```

Update the field initializers at every `SourceReliabilityDiscount {...}` construction site (`ranking.rs` `source_reliability_failure_discount`, the `goal_model.rs` test fixture at `goal_model.rs:2839`, `agent_tick/planning.rs` test fixtures, all the assertion-form fixtures inside `ranking.rs::tests`). The dropped-field count is small and they are all currently zero, so the migration is mechanical.

The save format does not need a bump: `SourceReliabilityDiscount` is part of the per-tick decision trace, which is not persisted across saves (only the underlying `ReliabilityRecord` is, and that is unchanged from S131).

### D2: `SourceCompositeRank` derivation

In `crates/worldwake-ai/src/source_composite.rs` (new module), add:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SourceCompositeRank {
    pub source_entity: EntityId,
    pub commodity: CommodityKind,
    pub trust_factor_permille: u32,
    pub wait_factor_permille: u32,
    pub capacity_factor_permille: u32,
    pub composite_permille: u32,
}

pub(crate) fn source_composite_rank(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
) -> Option<SourceCompositeRank> {
    let (source_entity, commodity) = source_reliability_discount_scope(candidate)?;
    let source_reliability = context.view.source_reliability(context.agent)?;
    let profile = context.view.preference_profile(context.agent)?;
    let key = SourceKey { entity: source_entity, commodity };
    let record = source_reliability.sources.get(&key)?;

    let trust_factor_permille = trust_factor_permille(record, profile);
    let wait_factor_permille = wait_factor_permille(record, profile);
    let capacity_factor_permille = capacity_factor_permille(record, profile, context.current_tick);
    let composite_permille = compose_factors(
        trust_factor_permille,
        wait_factor_permille,
        capacity_factor_permille,
    );

    Some(SourceCompositeRank {
        source_entity,
        commodity,
        trust_factor_permille,
        wait_factor_permille,
        capacity_factor_permille,
        composite_permille,
    })
}
```

Factor functions (each returns a permille in `[0, 2000]`, neutral = 1000):

- **trust factor** = `1000 - (failure_ratio_permille × source_trust_weight) / 1000`. Neutral 1000 when no failures recorded; floor 0 when every attempt failed.
- **wait factor** = `1000 - capped_wait_penalty_permille`, where `capped_wait_penalty_permille = min(800, average_wait_ticks × wait_sensitivity_weight / wait_normalizer_ticks)`. The cap of 800 keeps the floor at 200 — even an extremely contested source contributes 20% of neutral, never 0%, because contention is recoverable in principle.
- **capacity factor** = neutral 1000 when stale (`capacity_freshness_ticks > memory_retention_ticks`) or when `wait_observation_count == 0 && last_observed_capacity == 0` (never observed). Otherwise:
  - `freshness_factor_permille = 1000 - capacity_freshness_ticks × 1000 / memory_retention_ticks` (clamped at `[0, 1000]`)
  - `capacity_signal_permille` = a permille-scale view of `last_observed_capacity / capacity_observation_weight` (where `capacity_observation_weight` is the new `PreferenceProfile` field — see D3); clamped to [0, 1000]
  - `capacity_bonus = capacity_signal_permille × freshness_factor_permille / 1000` — bonus added to neutral
  - For empty-but-fresh observations (`last_observed_capacity == 0` && fresh): factor returns `1000 - freshness_factor_permille / 2`. Floor 500 — depletion is real signal but recoverable.

`compose_factors` multiplies the three permilles: `(t × w × c) / (1000 × 1000)`, then `clamp(0, 2000)`. The composite represents a permille modifier where 1000 is "no opinion," <1000 is "this source is worse than baseline for this commodity," >1000 is "this source is better than baseline."

`wait_normalizer_ticks` is a constant `worldwake-ai` private static derived from FND-22's "concrete profile values" guidance. The starting choice is `60` ticks (one in-sim hour at the existing tick scale). This is exposed only as a code-level constant (not a designer dial) per FND-3 — it's a structural unit-conversion factor, not a magnitude knob.

### D3: Add `PreferenceProfile.capacity_observation_weight`

In `crates/worldwake-core/src/experience.rs`:

```rust
pub struct PreferenceProfile {
    pub route_caution_weight: Permille,
    pub source_trust_weight: Permille,
    pub route_memory_capacity: u32,
    pub source_memory_capacity: u32,
    pub memory_retention_ticks: u64,
    pub wait_sensitivity_weight: Permille,
    /// How much weight the agent gives to its last observed capacity at a
    /// source when comparing same-commodity sources. The value is a permille
    /// "expected useful capacity" — `capacity_observation_weight = 20` means
    /// the agent treats 20 units as a fully-saturated source signal; sources
    /// with `last_observed_capacity = 20` contribute a maximum capacity
    /// bonus, anything above saturates at the cap.
    pub capacity_observation_weight: Permille,
}

impl Default for PreferenceProfile {
    fn default() -> Self {
        Self {
            route_caution_weight: Permille::new_unchecked(300),
            source_trust_weight: Permille::new_unchecked(200),
            route_memory_capacity: 24,
            source_memory_capacity: 18,
            memory_retention_ticks: 400,
            wait_sensitivity_weight: Permille::new_unchecked(150),
            capacity_observation_weight: Permille::new_unchecked(20),
        }
    }
}
```

Universal default `pm(20)` matches the `extraction_slots × extraction_duration_ticks` median of the survival scenarios — a 20-unit orchard capacity is a "normal full source." Per-agent tuning via existing `AgentDef.preference_profile` block. Save format bumps from current to next version since `PreferenceProfile` is a persisted component.

### D4: Sort comparator extension

In `crates/worldwake-ai/src/ranking.rs`, populate a new `source_composite: Option<SourceCompositeRank>` field on `AgendaEntry` during the per-candidate ranking pass:

```rust
let source_composite = source_composite_rank(candidate, &context);
```

…computed alongside `source_reliability_discount` and `competition_discount`. Threading through `AgendaEntry::pending` constructor and any plan-summary downstream consumers.

In `ranked_goal_ordering`, insert a new ordering dimension *immediately after* `MotiveScore` and *before* `Feasibility`:

```rust
// After the existing motive_score comparison …
if let Some((left_key, right_key)) = source_composite_peer_keys(&left.offer, &right.offer)
    && left_key == right_key
{
    let ordering = right.source_composite.as_ref().map_or(0, |c| c.composite_permille)
        .cmp(&left.source_composite.as_ref().map_or(0, |c| c.composite_permille));
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::SourceComposite));
    }
}
// existing Feasibility comparison …
```

`source_composite_peer_keys(left, right)` returns `Some((shared_key, shared_key))` only when both candidates are siblings in the candidate-generation sense — same `GoalKind::AcquireCommodity { commodity, purpose, quantity }` (with `quantity` compared by its `desired_target` so `acquire_commodity_quantity_bonus` differences don't gate the tiebreaker), or same `GoalKind::RestockCommodity { commodity }`. Other goal kinds return `None` and the comparator falls through unchanged. Add `SourceComposite` to the `RankedGoalComparisonDimension` enum and to the dimension's `Display`/`Debug` path so the trace can attribute it.

### D5: Decision-trace surfacing

Add `SourceCompositeRank` to the per-candidate planning trace at `decision_trace.rs`:

```
source_composite=entity=_ commodity=_ trust=_ wait=_ cap=_ composite=_
```

Surfaces under the same per-candidate trace block that already carries `source_reliability=…`. Both lines may appear together — the failure-ratio path (S131) handles motive-discount, the composite (S133) handles intra-commodity tiebreaker. Observer Section 3 / Section 4 then render readable lines like:

```
Source choice (Apple): Far Orchard composite=1080 (trust=1000 wait=1000 cap=1080)
                       Close Orchard composite=920 (trust=1000 wait=600 cap=900)
                       → tiebreaker SourceComposite picked Far Orchard
```

`AgendaEntry` summary structs (`agent_tick/planning.rs`, `goal_model.rs::RankedGoal`) gain matching `source_composite: Option<SourceCompositeRank>` fields so the summary path preserves the data.

### D6: Goldens

Reauthor the three goldens that S131SOURELWAI-004 introduced to test the rolled-back motive-additive path. New file `crates/worldwake-ai/tests/golden_source_composite.rs` covers:

- **Same-commodity wait reranking.** Two orchards, same agent, identical hunger. Close orchard has three `observe_wait(30)` events recorded; far orchard has none. With `wait_sensitivity_weight = 800`, the agent picks the FAR orchard. Trace shows `RankedGoalComparisonDimension::SourceComposite` fired and lists the close orchard's wait_factor below 1000.
- **Cross-category neutrality.** Hungry agent at a place with a fresh-capacity orchard *and* a critical wash basin. The orchard's high capacity factor must NOT pull AcquireCommodity above Wash when Wash has higher motive. Trace shows the comparator stops at `MotiveScore` (Wash wins on motive) and never reaches `SourceComposite`. Direct regression for the 2026-05-03 four-golden failure mode.
- **Fresh capacity bonus.** Hungry agent with two orchards at equal travel cost, same successful_acquisitions, different `last_observed_capacity` (18 vs 4, both fresh). Higher-capacity source wins via `SourceComposite`.
- **Stale capacity neutrality.** As above but the high-capacity observation is stale (older than `memory_retention_ticks`). The agent now picks based on whatever else differs, and the trace records `capacity_factor_permille = 1000` for the stale source — neutral, not zero.
- **Empty-but-fresh observation penalty.** Two orchards at equal cost, both with observed capacity, but one's most recent observation is `last_observed_capacity = 0` and fresh. The empty-observed source is reranked below.
- **No-record neutrality.** New agent with no `SourceReliability` data for either source. Both sources get `composite_permille = 1000`, the comparator falls through to lower tiebreakers (place-key etc.), and the rank is deterministic.

Existing S131 data-path goldens (`golden_source_reliability::resource_extraction_wait_observation_records_when_promoted`, `…::capacity_observation_records_from_perception`) remain in place and continue to verify the write paths.

The four originally-failing goldens (`survival_drive_escalation_lands_row_four`, `survival_offices_proves_force_law_uptake`, `survival_preferences_keeps_proactive_diversification_alive_under_survival`, `survival_tell_lands_row_five`) are not modified — they re-assert their original survival contracts. The S133 implementation must keep them green; they are the regression contract for the cross-category-neutrality goal.

## SystemFn Integration

No new SystemFn. The composite computation runs inside the existing `rank_goals` pass on the AI's planning tick. The sort comparator runs on the existing sort. No new tick scheduling, no system manifest change.

## Component Registration

No new components. `PreferenceProfile.capacity_observation_weight` lands in the existing universal-component schema (`component_schema.rs:383–405`), and `AgentDef.preference_profile.unwrap_or_default()` already populates the new field via `Default`. Save format bumps once.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Generalized contention substrate (S44) / `ContentionQueue` grants | Writes wait observation through S131's existing hook in `facility_queue.rs::promote_ready_head` | State-mediated (unchanged) |
| Resource extraction queues / `ResourceExtractionQueues` grants | Writes wait observation through S131's existing hook in `production_actions.rs::grant_or_signal_full` | State-mediated (unchanged) |
| Perception | Writes capacity observation through S131's existing hook | State-mediated (unchanged) |
| AI ranking — failure-ratio motive discount | S131 (now `source_reliability_failure_discount`) — separate path, motive-affecting, governs cross-category goal motive | State-mediated |
| AI ranking — same-commodity composite tiebreaker | S133 (this spec) — `source_composite_rank` — sort-comparator-only, intra-commodity | State-mediated |
| `ShareBelief` | Cross-agent reliability propagation continues as today | State-mediated (unchanged) |

## Profile-Driven Parameters

Per-agent (universal `PreferenceProfile`):

- `source_trust_weight` — existing. Inputs the trust factor.
- `wait_sensitivity_weight` — existing (S131). Inputs the wait factor.
- `capacity_observation_weight` — **new**, default `pm(20)`. Inputs the capacity factor's saturation point.
- `memory_retention_ticks` — existing. Bounds the capacity-freshness window AND the eviction window for the underlying record.
- `source_memory_capacity` — existing. Bounds total reliability records.

No magic numbers in agent-side code beyond the structural unit-conversion constants documented inside the factor functions (the 800 wait-cap and 60-tick wait normalizer in D2). These are FND-3 structural choices, not designer dials.

## Implementation Order

1. **D3** (add `capacity_observation_weight`) — touches core, cli scenario loader, save format. Smallest blast radius.
2. **D2** (new `source_composite.rs` module) — pure ai-crate addition.
3. **D4** (sort comparator extension) — add `source_composite` to `AgendaEntry`, extend `RankedGoalComparisonDimension`, plumb through plan-summary structs.
4. **D5** (decision-trace surfacing) — Display + observer renderer.
5. **D1** (strip vestigial fields) — only after D2-D5 land; D2's `SourceCompositeRank` becomes the canonical surface for wait/capacity.
6. **D6** (goldens) — new file; run alongside the four pre-existing survival goldens that must stay green.

A single ticket chain is sufficient (likely 4–5 tickets, mirroring S131's chain in shape). No phase-gate impact: Phase 10 is already gated on S131 completion; S133 is post-gate cleanup.

## Outcome (post-implementation)

After S133 lands, all four originally-failing goldens stay green; the wait/capacity learning surface is a real consumer of agent behavior again (now restricted to its proper architectural niche); and the decision trace clearly attributes any source-choice flip to the new `SourceComposite` ordering dimension. No motive-additive composite remains in the codebase; FND-26 / FND-27 / FND-28 are satisfied without backward-compatibility shims.
