# S148PORMOTBAC-005: Core-residing IntentionResumeCondition and IntentionAbandonCondition with ArtifactLegalEffectTag mirror

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new core-resident `IntentionResumeCondition` and `IntentionAbandonCondition` enums; new payload-free `ArtifactLegalEffectTag` discriminant mirror in `social_artifact.rs` per the Core-Side Mirror pattern with single conversion site
**Deps**: `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S148 D6 extends `IntentionFrame` (`crates/worldwake-core/src/intention_frame.rs:138`) with `resume_conditions` and `abandon_conditions` vector fields. The condition types must live in `worldwake-core` because `IntentionFrame` itself is core-resident. The existing `BeliefPredicate` at `crates/worldwake-ai/src/htn/method_schema.rs:72` cannot be reused: it lives in `worldwake-ai` (core can't depend on ai per the `core → sim → systems → ai` graph) and its variants are HTN-domain-specific (`BountyRecordExists`, `WitnessNamesKnown`, `InstitutionalRecordBelievedExtant`, `ResourceSourceKnown`). Spec S148 D7 defines new core-residing condition enums composed from existing core-resident types (`BeliefStatusTag`, `OpportunityAnchor`, `MotiveSourceDiscriminant`, `FrameAssumption`, `EntityId`) plus a newly-added `ArtifactLegalEffectTag` payload-free mirror following the established `BeliefStatusTag` precedent.

## Assumption Reassessment (2026-05-17)

1. Existing core-resident types referenced by the new enums: `BeliefStatusTag` at `crates/worldwake-core/src/decision_event_payload.rs:281` (payload-free mirror of `BeliefStatus` per Core-Side Mirror precedent); `OpportunityAnchor` at `crates/worldwake-core/src/goal.rs:324`; `MotiveSourceDiscriminant` at `crates/worldwake-core/src/motive_source.rs:25`; `FrameAssumption` at `crates/worldwake-core/src/intention_frame.rs:62`; `EntityId` and `Tick` at `crates/worldwake-core/src/ids.rs`. All verified during reassessment.
2. `ArtifactLegalEffect` enum at `crates/worldwake-core/src/social_artifact.rs:103` with payload-bearing variants: `None`, `Active { expires_at }`, `Suspended { reason, suspended_at }`, `Expired { expired_at }`, `Revoked { revoked_at, by, reason }`, `Fulfilled { fulfilled_at, by, evidence }`. A payload-free discriminant mirror `ArtifactLegalEffectTag` does not exist yet — it must be added per spec D7's `BeliefStatusTag`-style precedent.
3. Shared abstraction under audit: the core-side condition predicate surface. After this ticket lands, `IntentionFrame.resume_conditions: Vec<IntentionResumeCondition>` and `IntentionFrame.abandon_conditions: Vec<IntentionAbandonCondition>` (added in ticket 006) compose with these enums; the evaluator (ticket 007) reads them to decide `FrameState` transitions. The HTN `BeliefPredicate` stays in ai untouched — two separate concerns (HTN method preconditions vs. generic intention lifecycle predicates), per spec D7's core-residency rationale.
4. `BeliefStatus` source enum lives at `crates/worldwake-sim/src/belief_view.rs:40`; its `BeliefStatusTag` mirror's conversion site is `crates/worldwake-sim/src/save_load.rs:1368-1372` per the established Core-Side Mirror pattern. `ArtifactLegalEffectTag`'s conversion site lives in `social_artifact.rs` itself since both types are core-resident — no cross-crate conversion needed.

## Architecture Check

1. New types live in core; no upward crate-graph dependency. The decision to define new condition enums rather than relocate `BeliefPredicate` from ai to core respects FND-28 (no parallel authority) and avoids conflating HTN-method preconditions with generic intention lifecycle — two distinct concerns deserve distinct types.
2. `ArtifactLegalEffectTag` follows the existing `BeliefStatusTag` precedent at `decision_event_payload.rs:281` exactly: same derive set (`Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`), 1:1 mechanical mirror of source variants (no renames, no merges, no narrowing), single conversion site per Core-Side Mirror pattern in `references/worldwake-validation-patterns.md`.
3. FND-21 alignment: making intention lifecycle predicates first-class typed values (rather than implicit conditions baked into evaluator code) gives every revision-or-abandon path an inspectable trace surface — the cause of a resume/abandon decision is always nameable via a variant.

## Verification Layers

1. Enum shape and serialization → focused unit tests in each new module's `#[cfg(test)]` block: serde round-trip, `Ord` stability
2. `ArtifactLegalEffect → ArtifactLegalEffectTag` conversion correctness → focused unit test asserting each `ArtifactLegalEffect` variant maps to its corresponding `Tag` variant (mechanical 1:1)
3. Cross-crate visibility → workspace compile (`cargo clippy --workspace --all-targets -- -D warnings`) — re-exports are reachable from sim/systems/ai/cli without import errors

## What to Change

### 1. Define `IntentionResumeCondition` and `IntentionAbandonCondition`

Create `crates/worldwake-core/src/intention_condition.rs`:

```rust
use crate::{
    ArtifactLegalEffectTag, BeliefStatusTag, EntityId, FrameAssumption,
    MotiveSourceDiscriminant, OpportunityAnchor,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IntentionResumeCondition {
    /// Belief about an entity transitioned to a specific status (e.g., to `Active`).
    BeliefStatusChanged { subject: EntityId, target_status: BeliefStatusTag },
    /// A specific opportunity became visible to the agent again.
    OpportunityVisible(OpportunityAnchor),
    /// Agent reached a specific place (e.g., resume on arrival).
    LocationReached(EntityId),
    /// Resume after this many ticks have elapsed since suspension.
    TickElapsed(u32),
    /// Artifact legal effect transitioned to `Active`.
    ArtifactLegalEffectActive(EntityId),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IntentionAbandonCondition {
    /// The motive that produced this intention is no longer present in the ledger.
    MotiveSourceLost(MotiveSourceDiscriminant),
    /// A frame assumption has been broken in a way that cannot recover.
    AssumptionPermanentlyBroken(FrameAssumption),
    /// The opportunity this intention targeted is gone (consumed, expired, destroyed).
    OpportunityForeverGone(OpportunityAnchor),
    /// `stalled_ticks` reached `patience_limit` in `frame.rs` (existing mechanism).
    PatienceExhausted,
    /// An explicit-claim artifact transitioned to `ArtifactExistence::Destroyed`.
    ArtifactDestroyed(EntityId),
    /// An explicit-claim artifact's legal effect transitioned out of `Active`
    /// (to `Suspended`, `Expired`, `Revoked`, or `Fulfilled`).
    ArtifactLegalEffectLost(EntityId),
}
```

Re-export from `crates/worldwake-core/src/lib.rs`: `pub use intention_condition::{IntentionResumeCondition, IntentionAbandonCondition};`.

### 2. Add `ArtifactLegalEffectTag` discriminant mirror

In `crates/worldwake-core/src/social_artifact.rs` (alongside the existing `ArtifactLegalEffect` enum at line 103), add:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactLegalEffectTag {
    None,
    Active,
    Suspended,
    Expired,
    Revoked,
    Fulfilled,
}

impl From<&ArtifactLegalEffect> for ArtifactLegalEffectTag {
    fn from(effect: &ArtifactLegalEffect) -> Self {
        match effect {
            ArtifactLegalEffect::None              => Self::None,
            ArtifactLegalEffect::Active { .. }     => Self::Active,
            ArtifactLegalEffect::Suspended { .. }  => Self::Suspended,
            ArtifactLegalEffect::Expired { .. }    => Self::Expired,
            ArtifactLegalEffect::Revoked { .. }    => Self::Revoked,
            ArtifactLegalEffect::Fulfilled { .. }  => Self::Fulfilled,
        }
    }
}
```

Update the existing `social_artifact.rs` re-exports / lib.rs surface so `ArtifactLegalEffectTag` is available alongside `ArtifactLegalEffect`. The conversion site lives in `social_artifact.rs` itself (both types core-resident — no cross-crate hop needed, unlike `BeliefStatusTag` whose source enum lives in `worldwake-sim`).

## Files to Touch

- `crates/worldwake-core/src/intention_condition.rs` (new)
- `crates/worldwake-core/src/social_artifact.rs` (modify — add `ArtifactLegalEffectTag` enum + `From<&ArtifactLegalEffect>` impl)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types)

## Out of Scope

- Adding the new fields to `IntentionFrame` (ticket 006 — depends on this ticket for the condition enum types)
- Implementing the resume/abandon condition evaluator in `frame.rs` (ticket 007)
- `Discrepancy::AbandonConditionFired` variant + `IntentionAbandonConditionDiscriminant` (ticket 007 — the discrepancy variant carries the discriminant, which is sized to fit `Discrepancy`'s `Copy` derive)
- Authoring `explicit_claims` semantics — `IntentionAbandonCondition::ArtifactDestroyed(EntityId)` and `ArtifactLegalEffectLost(EntityId)` define the predicate shape; the field they fire from (`IntentionFrame.explicit_claims: Vec<EntityId>`) lands in ticket 006

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core intention_condition` — new focused tests: serde round-trip on each variant of both enums; `Ord` stability
2. `cargo test -p worldwake-core social_artifact::tests::artifact_legal_effect_tag_*` — focused tests: every `ArtifactLegalEffect` variant maps to the expected `ArtifactLegalEffectTag` variant
3. Existing suite: `cargo test --workspace`
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `IntentionResumeCondition` and `IntentionAbandonCondition` are defined exactly once each (in `worldwake-core`); no parallel definition anywhere.
2. `ArtifactLegalEffectTag` is mechanically 1:1 with `ArtifactLegalEffect` (same variant names, same arity ignoring payload, same declaration order); no narrowing, no merging, no renames.
3. `ArtifactLegalEffectTag` derives `Copy` (required so consumers like the new `Discrepancy` variant in ticket 007 can carry it without `Box`-ing).
4. The conversion `&ArtifactLegalEffect → ArtifactLegalEffectTag` lives in exactly one file (`social_artifact.rs`); no parallel conversion path elsewhere.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/intention_condition.rs` — inline `#[cfg(test)]` module: `resume_condition_round_trips_through_serde`, `abandon_condition_round_trips_through_serde`, ord-stability tests
2. `crates/worldwake-core/src/social_artifact.rs` — extend existing `#[cfg(test)]` block at line 320+ with `artifact_legal_effect_tag_mirrors_every_variant` (explicit enumeration test using a sample value per variant)

### Commands

1. `cargo test -p worldwake-core intention_condition social_artifact`
2. `./scripts/verify.sh`
