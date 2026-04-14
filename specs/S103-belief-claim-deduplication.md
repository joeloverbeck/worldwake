# S103: Belief Claim Deduplication and Amortized Pruning

## Summary

Fix the unbounded growth of entity belief claims that causes soak test performance regression (4.2x slowdown over 500 ticks, extrapolated to 108 minutes for full 10,080-tick soak vs. 5-6 minute target). The root cause is that `record_entity_snapshot_claims` appends a new claim for every observed aspect on every perception tick, and the staleness-based pruning window (75 ticks) creates a steady-state accumulation of ~375 claims per entity in busy locations. Each tick, `derive_entity_summary` iterates every claim for every entity, making per-tick cost scale linearly with claim count.

The fix applies three targeted changes, all architecturally grounded in existing FOUNDATIONS principles:

1. **Aspect-source deduplication**: When a new claim arrives for an aspect that already has a claim from the same source type, replace the older claim instead of appending. This is not a capacity cap — it is the recognition that a fresh direct observation of an entity's location supersedes a stale direct observation of the same aspect (FND-16: newer evidence overwrites weaker).
2. **Canonicalize semantic belief transport before amortized pruning**: remove production paths that mutate `known_entities` semantics outside `entity_claims`, then skip `derive_entity_summary` during `prune_decayed_beliefs` when no claims were actually removed for an entity. This work is split into `S103BELCLADED-004` (boundary cleanup) followed by `S103BELCLADED-002` (optimization).
3. **Social observation deduplication**: When a new social observation matches an existing one by `detail`, replace rather than append — a repeated sighting of the same social event is an update, not a new independent record.

## Phase

Core infrastructure (performance fix for S101 belief system)

## Status

Draft

## Crates

- `worldwake-core` (belief claim storage, pruning logic, summary re-derivation)

## Dependencies

- S101 (activation-based belief decay) — completed

## Problem Statement

### Evidence

Profiling report (`reports/soak-performance-profile.md`) on the T30 soak world (20 agents, 10 places, seed 0):

| Metric | Tick 0 | Tick 250 | Tick 500 | Growth factor |
|--------|--------|----------|----------|---------------|
| Total entity claims (all agents) | 186 | 61,828 | 66,553 | 358x |
| Total known entities (all agents) | 89 | 407 | 606 | 7x |
| Total social observations (all agents) | 0 | 4,197 | 5,228 | unbounded |
| ms per tick | 12.5 | 64.6 | 80-161 | 4.2x (first 50 vs last 50) |

Linear regression: `ms/tick = 31.56 + 0.12 * tick`. Extrapolated full soak (10,080 ticks): 108 minutes.

Individual agents at busy locations (Farm, Market) accumulate 3,000-4,500 claim records across ~30 known entities — averaging **100-150 claims per entity**. The `derive_entity_summary` function iterates all claims per entity on every observation and every prune call.

### Why claims accumulate

S101 replaced the `entity_claim_capacity: 12` hard cap with confidence-threshold pruning. Claim confidence starts at 950 (direct observation) and decays at 12 per tick, reaching the threshold of 50 after 75 ticks. During those 75 ticks, each co-located entity generates ~5 new claims per observation tick (Location + Inventory + Activity + other aspects). An agent at a busy location (8 co-located entities) accumulates `8 × 5 × 75 = 3,000` claims before the oldest ones expire.

The critical insight: **most of these claims are redundant**. When an agent observes entity X's location at tick 10, then again at tick 11, the tick-11 Location claim supersedes the tick-10 Location claim from the same source. Both are DirectObservation claims for the same aspect of the same entity. Keeping the old one serves no informational purpose — `derive_entity_summary` will always pick the fresher one as the winner. The old claim exists only to be iterated over and rejected.

### Architectural framing

This is not a capacity problem requiring a cap. This is a **redundant evidence** problem. The S101 spec's claim confidence threshold is the correct mechanism for handling genuinely distinct evidence (a direct observation vs. a report vs. a rumor about the same aspect). But when the same source type observes the same aspect repeatedly, each new observation is strictly stronger evidence than the last. Storing all of them violates the spirit of FND-27 (derived summaries are caches, never truth) — the older claims contribute nothing to the derived summary but consume iteration budget.

## Design Goals

- Eliminate redundant same-source, same-aspect claim accumulation without introducing arbitrary capacity numbers
- Maintain the ability to hold competing claims from different sources (direct observation vs. report vs. rumor) for the same aspect — this is core to FND-16 (uncertainty and contradiction)
- Reduce per-tick cost of `derive_entity_summary` from O(total_claims) to O(distinct_aspects × source_types)
- Achieve constant-time claim storage per entity at steady state, bounded by the number of distinct (aspect, source_kind) pairs rather than by observation frequency
- Fix social observation growth through the same principle: newer observations from the same source about the same social event supersede older ones

## Non-Goals

- Changing activation-based entity decay (S101) — that system works correctly; known entity counts are modest (7x growth)
- Introducing hard capacity caps on claims, entities, or social observations
- Changing the confidence-threshold pruning model — it correctly handles age-based evidence decay
- Optimizing the GOAP planner or candidate generation — those scale with known entity count, which is under control

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | No change to what agents learn or forget — only how fast redundant records accumulate |
| FND-3 (Concrete State) | Claims remain concrete stored state. Deduplication removes records that are provably dominated by fresher records of the same kind |
| FND-11 (Physical Dampeners) | The dampener on claim growth becomes the physical world's aspect diversity (finite number of observable aspects per entity), not an artificial number |
| FND-12 (Performance Compresses Computation, Not Causality) | Deduplication is already valid because it removes only dominated claims. Amortized pruning becomes valid only after semantic belief updates are claim-backed, so unchanged claims imply unchanged summaries |
| FND-15 (Knowledge Travels Physically) | No change — provenance, source, confidence, and event time are all preserved |
| FND-16 (Uncertainty and Contradiction) | Multi-source claims are preserved. An agent can still hold a DirectObservation claim AND a Report claim for the same aspect, enabling contradiction detection. Only same-source-same-aspect duplicates are eliminated |
| FND-22 (Agent Diversity) | No profile parameter changes. Per-agent `claim_confidence_threshold` and `staleness_penalty_per_tick` continue to control when evidence ages out |
| FND-27 (Derived Summaries Are Caches) | `derive_entity_summary` produces the same result whether called on 375 claims or on the ~15 non-redundant ones |

## Design

### Change 1: Aspect-source claim deduplication in `record_entity_claim`

Currently `record_entity_claim` unconditionally appends:
```rust
pub fn record_entity_claim(&mut self, claim: EntityBeliefClaim) {
    self.next_claim_id = ClaimId(claim.claim_id.0.saturating_add(1).max(self.next_claim_id.0));
    self.entity_claims.entry(claim.subject).or_default().push(claim);
}
```

Change to: when a new claim arrives for `(subject, aspect, source_kind)`, compact only same-key claims that are dominated by the newcomer. Drop the newcomer if an existing same-key claim already dominates it.

Source kind is derived from `PerceptionSource`:
- `DirectObservation` → `SourceKind::Direct`
- `Report { from, .. }` → `SourceKind::Report(from)` (distinct per informant)
- `Rumor { .. }` → `SourceKind::Rumor`
- `Inference` → `SourceKind::Inference`

The deduplication key is `(subject, aspect, source_kind)`. Dominance must compare stored confidence, staleness anchor (`claimed_event_tick.unwrap_or(acquired_tick)`), and acquisition order so compaction removes only claims that can never win `derive_entity_summary`. This ensures:
- Fresh direct observations replace stale direct observations (common case)
- A report from agent A and a report from agent B remain as separate claims (different informants)
- A direct observation and a rumor about the same aspect coexist (different source kinds)
- A same-informant report about an older event does not incorrectly evict a fresher report merely because it was acquired later

The direct-observation hot path collapses to roughly one claim per aspect, which removes the dominant soak-time growth driver. Same-source report/rumor variants may still coexist when neither dominates the other because their event-time freshness and stored confidence disagree. This keeps storage aligned with the summary-selection contract instead of imposing an artificial cap.

Note: `record_entity_snapshot_claims` also calls `refresh_entity_summary_from_claims` after every observation batch (`belief.rs:107`). With deduplication, this existing call benefits automatically — the per-entity claim count it iterates is dramatically reduced.

### Change 2: Canonicalize semantic belief transport, then skip summary re-derivation when no claims were pruned

The naive optimization was tested on 2026-04-14 and failed `cargo test -p worldwake-ai` in `guard_theron_water_at_thornwall_finds_harvest_plan`: skipping unconditional refresh changed planner-visible behavior. That means `known_entities` is not currently a pure derived cache.

The architectural issue is that production code still transports semantic belief facts through both:

1. `entity_claims` + `refresh_entity_summary_from_claims`
2. direct `known_entities` writes such as `update_entity`, `update_believed_activity`, `update_departure_projection`, and evidence mutation in investigation code

This violates the intended derived-cache contract for `known_entities`. Before amortized pruning ships, semantic fields already represented by `EntityBeliefAspect` must be updated through claims with explicit provenance and timing. After that cleanup, `prune_decayed_beliefs` may safely skip `refresh_entity_summary_from_claims` for entities whose claim vectors did not change.

The final pruning optimization remains:

```rust
let affected_entities = self.entity_claims.keys().copied().collect::<Vec<_>>();
// ... claim pruning ...
for entity in affected_entities {
    self.refresh_entity_summary_from_claims(entity, current_tick, &profile.confidence_policy);
}
```

Change to: track which entities actually had claims removed during the retain pass, and only re-derive summaries for those entities. This avoids iterating claims for entities whose claim sets didn't change.

```rust
for entity in &affected_entities {
    let Some(claims) = self.entity_claims.get_mut(entity) else { continue; };
    let len_before = claims.len();
    claims.retain(|claim| {
        effective_claim_confidence(claim, current_tick, &profile.confidence_policy)
            >= claim_confidence_threshold
    });
    if claims.len() < len_before {
        changed_entities.push(*entity);
    }
}
// Only re-derive for entities that actually changed:
for entity in changed_entities {
    self.refresh_entity_summary_from_claims(entity, current_tick, &profile.confidence_policy);
}
```

### Change 3: Deduplicate social observations

Social observations currently accumulate unbounded (5,228 by tick 500). Apply the same principle: when a new social observation arrives with the same `detail` value as an existing one, replace rather than append. This matches the real-world semantics: seeing the same agent doing the same activity again is an update, not a new independent observation.

The deduplication key for `SocialObservation` is the full `detail: SocialObservationDetail` value — this type derives `Eq`, `Ord`, and `Hash`, so it naturally serves as a composite key encoding both the kind of social event and the specific entities involved (e.g., `WitnessedCooperation { actor: A, counterpart: B }` is distinct from `WitnessedCooperation { actor: C, counterpart: D }`). When a match is found, keep the newer observation (higher `observed_tick`). Note that `SocialObservation` has no `subject` field — the observer is the agent whose `AgentBeliefStore` contains the observation, and the observed entities are embedded in the `SocialObservationDetail` variants.

## FND-01 Section H Analysis

### Information-path analysis

Change 1 removes redundant same-source claim accumulation without changing information paths. Change 2 tightens the information path contract: semantic entity beliefs must travel through `entity_claims`, not through duplicate summary mutation paths. This is a cleanup of competing lawful transports, not a new path.

### Positive-feedback analysis

**Existing loop (unchanged)**: Observation → claim stored → entity retained → more observations → more claims. With deduplication, the claim count for a frequently-observed entity is bounded by `|aspects| × |source_kinds|`, which is determined by the physical world's diversity of observable properties and information sources. This is a concrete physical dampener (FND-11), not an arbitrary number.

No new feedback loops introduced.

### Concrete dampeners

The claim accumulation rate is now dampened by:
- **Aspect diversity**: An entity has a finite number of observable aspects (~12). This is a property of the entity itself, not an artificial limit.
- **Source diversity**: An agent encounters a finite number of information sources per entity (direct observation + a few reports). This is determined by who the agent co-locates with and talks to.
- **Confidence decay**: Claims from stale sources still expire via `staleness_penalty_per_tick` after ~75 ticks of non-renewal.

### Stored state vs. derived read-model list

| Item | Classification | Change |
|------|---------------|--------|
| `entity_claims` (per entity) | Authoritative stored state | Deduplication removes only provably dominated records; semantic entity belief updates must also become canonical here before amortized pruning |
| `derive_entity_summary` result | Derived | Must become the sole semantic producer of `known_entities` for claim-backed fields |
| `social_observations` | Authoritative stored state | Deduplication removes only superseded observations |
| `prune_decayed_beliefs` changed-entity tracking | Transient computation | New (not stored) |

## Verification

1. `cargo clippy --workspace --all-targets -- -D warnings`
2. `cargo test -p worldwake-core` — existing belief system tests pass; new tests for deduplication behavior
3. `cargo test -p worldwake-ai` — golden tests pass (including S101 activation decay tests and S102 exploration tests)
4. Re-run soak profiler (`cargo test --release -p worldwake-ai --features soak --test soak_profiler -- --nocapture`):
   - Growth ratio should be < 1.5x (vs. current 4.23x)
   - Extrapolated full soak time should be < 10 minutes (vs. current 108 min)
   - Steady-state claims per agent should be < 1,000 (vs. current 3,000-4,500)
5. `SOAK_SEED_START=0 SOAK_SEEDS=2 cargo test --release -p worldwake-ai --features soak --test golden_soak` completes in < 6 minutes
