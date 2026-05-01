# INFRARET-001: Generalize direct-observed concrete-opportunity retention

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` belief retention helper (`state_salience_boost`)
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md, archive/tickets/S129CIREM-003-tell-session-vs-self-care.md, tickets/BELASPCOV-001-believed-entity-state-claim-aspect-coverage.md (soft — findings may add aspects worth retaining)

## Problem

S129CIREM-003 added `state_salience_boost`
(`crates/worldwake-core/src/belief.rs`, `~line 2697`) to retain
direct-observed concrete-opportunity infrastructure under need
pressure. The implementation hardcodes two shape matches:

```rust
if state.wash_basin_state.is_some() && state.workstation_tag == Some(WorkstationTag::WashBasin) {
    return boost.value();
}
if state.resource_source.is_some() && state.workstation_tag.is_some() {
    return boost.value();
}
```

The shape match is correct for today's two cases (wash basins with
state, resource-source facilities with workstation tag) but does not
generalize. As more state-rich opportunity infrastructure is added
(crafting stations with material state, market stalls with stock
claims, sleep sites with `SleepQualityProfile`-derived hints, latrine
fullness trackers from S129), each will need a new shape check
appended to this function. The pattern is the same in every case:
"the agent has directly observed an entity that carries an actionable
opportunity claim relevant to a currently-pressuring need; retain
the entity through stale-claim decay so the agent can still act on
it." Encoding this as enumerated shape pairs accumulates technical
debt and is brittle against new aspect variants.

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Live `state_salience_boost`** at
   `crates/worldwake-core/src/belief.rs` (`~line 2697`): two hardcoded
   shape pairs as quoted above; falls through to
   `salience_boost(needs.max_value(), state.believed_kind, ..)`.
2. **Live call site** at `prune_decayed_beliefs`
   (`crates/worldwake-core/src/belief.rs:323+`): boost added to
   `compute_activation` per-entity activation score during decay
   pruning, and (separately) added to direct-observation claim
   confidence during claim retention.
3. **Generalization candidate**: the underlying contract is "if a
   directly-observed entity carries actionable opportunity claims for
   any of the agent's pressuring needs, retain it." The audit
   surfaced by BELASPCOV-001 will name the full set of opportunity
   claim aspects. After that audit, the retention rule can read:
   *for each aspect the agent currently has a need for, if the entity
   carries that aspect via a direct-observation claim, return boost*.
4. **Mismatch + correction**: today's hardcoded matches are correct
   for the wash-basin and resource-source cases. The risk is silent
   regression: when a future spec adds a new opportunity infrastructure
   class, the retention shape match will not include it, and the next
   chronic-stall failure mode will be hidden until a golden surfaces
   it. This ticket eliminates that fragility.
5. **Heuristic Removal Discipline (precision-rules §12)**: the boost
   value remains the same; the substrate change is *what gets
   boosted*, not *how much*. Replacing two shape checks with a
   need-aware aspect query does not weaken the existing retention
   contract.
6. **Coverage gap (precision-rules §3)**: existing focused coverage at
   `test_prune_salience_boost_preserves_observed_wash_basin_infrastructure`
   and the resource-source companion test must continue to pass after
   refactor. New focused coverage for the general predicate (a fake
   aspect that exercises the predicate without coupling to the two
   live shapes) closes the false-confidence gap that the existing
   tests validate the live shapes specifically rather than the
   underlying contract.
7. **Coordination with BELASPCOV-001**: BELASPCOV-001 may surface
   additional opportunity aspects worth adding to the retention
   predicate. This ticket can land before BELASPCOV-001 (with the
   current two cases mapped through the new predicate) or after
   (with newly-discovered aspects added in the same change). Soft
   dependency, not hard.

## Architecture Check

1. **No backwards-compatibility shim**: the hardcoded shape matches
   are removed, not preserved alongside the new predicate.
2. **Concrete state, not abstract score (FND-3)**: the predicate
   reads concrete `BelievedEntityState` claim presence and concrete
   per-need pressure, not a derived "infrastructure score."
3. **Concrete dampener already present (FND-11)**: retention is
   bounded by claim-confidence decay for non-direct claims, by
   activation decay for entity presentation history, and by per-need
   pressure thresholds. This refactor does not weaken any dampener.

## Verification Layers

1. **Wash-basin retention** (existing CIREM-003 contract) -> focused
   unit test
   `test_prune_salience_boost_preserves_observed_wash_basin_infrastructure`
   continues to pass.
2. **Resource-source retention** (existing CIREM-003 contract) ->
   focused unit test
   `test_prune_salience_boost_preserves_observed_resource_source_infrastructure`
   continues to pass.
3. **Generalized predicate** -> new focused unit test verifies the
   predicate fires for a synthetic state carrying a need-relevant
   opportunity claim aspect, and does not fire for an aspect the
   agent has no pressuring need for.
4. **No salience over-retention** -> the existing falsifying test
   asserting that *unrelated* facilities decay normally must
   continue to pass under the new predicate (i.e. the predicate
   does not flatly return boost for every directly-observed entity).
5. **Goldens**: `golden_activation_decay`, `golden_survival_tell`
   (CIREM-003 listener-tell coverage), and the eight-scenario
   `golden_place_dirtiness` suite continue to pass.

## What to Change

### 1. Define the need-relevance map

Introduce a private helper that maps each
`HomeostaticNeedId` to the set of `EntityBeliefAspect` variants that
carry actionable-opportunity information for that need. Initial
population (matching today's behavior + closing the natural gap):

| Need | Opportunity aspects |
|---|---|
| `Dirtiness` | `WashBasinState` |
| `Eat`, `Drink`, `Hunger`, `Thirst` | `ResourceAvailable(commodity)` (gated on `WorkstationPresent` like the current resource-source check) |
| Other needs | (empty until a future spec declares an opportunity aspect) |

The map is a `match` over `HomeostaticNeedId` returning a small
`&'static [EntityBeliefAspect]`-style iterator. No allocation per
call.

### 2. Refactor `state_salience_boost`

Replace the two hardcoded shape checks with:

```rust
fn state_salience_boost(
    needs: &HomeostaticNeeds,
    state: &BelievedEntityState,
    urgency_threshold: Permille,
    boost: Permille,
) -> u16 {
    if state.source == PerceptionSource::DirectObservation
        && carries_pressuring_opportunity(needs, state, urgency_threshold)
    {
        return boost.value();
    }
    salience_boost(needs.max_value(), state.believed_kind, urgency_threshold, boost)
}
```

Where `carries_pressuring_opportunity` iterates the agent's
above-threshold needs and asks: "for any need above threshold, does
the entity expose a non-empty opportunity aspect from the
need-relevance map?"

### 3. Preserve direct-observation gating

CIREM-003's design preserves stale-report decay by limiting the
retention boost to direct-observation claims. The refactor must keep
this discipline: non-direct claim sources still decay normally. The
predicate above includes the
`state.source == PerceptionSource::DirectObservation` guard.

### 4. Update / extend focused unit tests

- Keep the wash-basin and resource-source tests as-is — they remain
  the live contract for the two known opportunity classes.
- Add `state_salience_boost_returns_boost_for_each_pressuring_need_with_opportunity_aspect`
  that drives the predicate against a synthetic entity carrying a
  need-relevant aspect for a per-test-parameterized need.
- Add `state_salience_boost_does_not_boost_unrelated_facility_under_pressure`
  asserting that an entity without any opportunity aspect does not
  receive the boost.
- Add `state_salience_boost_does_not_boost_indirect_observation_claim_even_with_aspect`
  to lock the direct-observation gate.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — `state_salience_boost`,
  new helper, new tests)

## Out of Scope

- Adding new `EntityBeliefAspect` variants. Those are BELASPCOV-001's
  domain.
- Changing the boost magnitude (`Permille` value). This refactor is
  about *what* gets boosted, not *how much*.
- Changing claim-confidence decay arithmetic. Stale-report decay
  governance is preserved.
- Changing perception throttling or observation budgeting (S105).

## Acceptance Criteria

### Tests That Must Pass

1. `test_prune_salience_boost_preserves_observed_wash_basin_infrastructure`
2. `test_prune_salience_boost_preserves_claim_backed_wash_basin_infrastructure`
3. The companion resource-source retention test
4. `state_salience_boost_returns_boost_for_each_pressuring_need_with_opportunity_aspect`
   (new)
5. `state_salience_boost_does_not_boost_unrelated_facility_under_pressure`
   (new)
6. `state_salience_boost_does_not_boost_indirect_observation_claim_even_with_aspect`
   (new)
7. `cargo test -p worldwake-core`
8. `cargo test -p worldwake-ai --test golden_activation_decay`
9. `cargo test --release -p worldwake-ai --test golden_survival_tell -- --ignored --test-threads=1`
10. `./scripts/verify.sh`

### Invariants

1. **Direct observation + pressuring need + opportunity aspect → boost**:
   the predicate fires whenever an agent has a need above
   `urgency_threshold`, the entity is direct-observed, and the entity
   carries any aspect declared as opportunity-relevant for that need.
2. **No silent boost-for-everything**: an entity that does not carry
   any need-relevant opportunity aspect must not receive the boost,
   regardless of `believed_kind`.
3. **Stale-report decay preserved**: claims sourced from anything
   other than direct observation continue to decay by confidence
   threshold.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs::tests::state_salience_boost_returns_boost_for_each_pressuring_need_with_opportunity_aspect`
   — new
2. `crates/worldwake-core/src/belief.rs::tests::state_salience_boost_does_not_boost_unrelated_facility_under_pressure`
   — new
3. `crates/worldwake-core/src/belief.rs::tests::state_salience_boost_does_not_boost_indirect_observation_claim_even_with_aspect`
   — new

### Commands

1. `cargo test -p worldwake-core state_salience_boost`
2. `cargo test -p worldwake-core test_prune_salience_boost_preserves`
3. `cargo test -p worldwake-ai --test golden_activation_decay`
4. `cargo test --release -p worldwake-ai --test golden_survival_tell -- --ignored --test-threads=1`
5. `./scripts/verify.sh`
