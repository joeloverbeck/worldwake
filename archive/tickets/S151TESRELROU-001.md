# S151TESRELROU-001: TopicScope enum + belief-topic mapping function

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new core type (`TopicScope` enum) and pure helper (`belief_topic_to_topic_scope`)
**Deps**: None

## Problem

S151 needs a payload-free topic categorization key for per-witness, per-topic testimony reliability tracking. The existing `TellTopic` at `crates/worldwake-core/src/belief.rs:1737` is payload-bearing (carries `EntityId`, `SocialObservation`, `InstitutionalClaim`), so keying a `BTreeMap` reliability histogram by `TellTopic` would fragment by inner subject/observation/claim identity. `TopicScope` is the coarser 8-variant categorization the spec needs.

## Assumption Reassessment (2026-05-17)

1. `TellTopic` exists at `crates/worldwake-core/src/belief.rs:1737` with 3 payload-bearing variants. `EntityBeliefAspect` exists at `crates/worldwake-core/src/entity_belief_claim.rs:17-32` with 14 variants. `SocialObservation` and `InstitutionalClaim` (`crates/worldwake-core/src/institutional.rs:26`) are the other two `TellTopic` payload types. No existing `TopicScope` or overlapping core abstraction.
2. Spec D1+D2 at `archive/specs/S151-testimony-reliability-and-route-preferences.md:71-122` defines the 8-variant enum and the mapping function structure. Derive set matches core conventions (`EntityId` at `crates/worldwake-core/src/ids.rs:44-47`, `Permille` at `crates/worldwake-core/src/numerics.rs:25`).
3. Shared boundary under audit: this ticket establishes the topic-categorization key dimension consumed by tickets 002 (TestimonyReliability store key), 003 (per-topic weights on TestimonyTrustProfile), 005 (decision-history payload field `topic`), and 009 (`BTreeMap<TopicScope, u64>` diagnostics field).
4. Live-code correction: the current `InstitutionalClaim` enum has no bounty-validity or price-level variants. `TopicScope::BountyValidity` and `TopicScope::PriceLevel` still land as reserved categories for later testimony topics, but this ticket's exhaustive mapping and tests only assert live upstream variants. The active spec D2 text was corrected in the same pass.

## Architecture Check

1. Coarse payload-free enum is the correct shape for histogram keys per the Aggregation-key-fidelity rule; using a `TellTopicDiscriminant` (3 variants) would lose the per-topic-category granularity the spec requires for "officialist"/"gullible"/"empiricist" agent variation.
2. Mapping function is a pure, deterministic, total function over each upstream enum's variant set — no global state, no I/O. Per FND-22A: "agent-local learned summaries are legal even when abstract — they are not world truth."
3. Closed-enum exhaustive matches mean adding any new upstream `EntityBeliefAspect`/`SocialObservation`/`InstitutionalClaim` variant produces a compile error here, forcing explicit categorization.

## Verified Layers

1. Mapping correctness per variant → focused unit tests in `crates/worldwake-core/src/topic_scope.rs#[cfg(test)]` covering each upstream enum variant's mapping target.
2. Derive coherence (`Copy + Hash + Ord + Serialize + Deserialize`) → compile-time generic bound assertion in `topic_scope_satisfies_required_bounds`.
3. Single-layer ticket — no cross-layer mapping required; this is foundation type substrate.

## Landed Changes

### 1. Added `crates/worldwake-core/src/topic_scope.rs`

```rust
use serde::{Deserialize, Serialize};

use crate::belief::{SocialObservation, TellTopic};
use crate::entity_belief_claim::EntityBeliefAspect;
use crate::institutional::InstitutionalClaim;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum TopicScope {
    RouteHazard,
    ResourceAvailability,
    OfficeHolder,
    AccusationCredibility,
    BountyValidity,
    PriceLevel,
    EntityWhereabouts,
    GeneralFact,
}

pub fn belief_topic_to_topic_scope(topic: &TellTopic) -> TopicScope {
    match topic {
        TellTopic::EntityBelief { .. } => TopicScope::GeneralFact,
        TellTopic::SocialObservation { observation } => social_observation_to_topic_scope(observation),
        TellTopic::InstitutionalClaim { claim } => institutional_claim_to_topic_scope(claim),
    }
}

pub fn entity_aspect_to_topic_scope(aspect: &EntityBeliefAspect) -> TopicScope {
    match aspect {
        EntityBeliefAspect::Location | EntityBeliefAspect::Holder | EntityBeliefAspect::Activity
        | EntityBeliefAspect::Alive | EntityBeliefAspect::Wounded | EntityBeliefAspect::Courage
            => TopicScope::EntityWhereabouts,
        EntityBeliefAspect::Inventory(_) | EntityBeliefAspect::ResourceAvailable(_)
        | EntityBeliefAspect::WorkstationPresent | EntityBeliefAspect::ContentionState
        | EntityBeliefAspect::WashBasinState
            => TopicScope::ResourceAvailability,
        EntityBeliefAspect::Owner | EntityBeliefAspect::Artifact
            => TopicScope::GeneralFact,
        EntityBeliefAspect::Evidence
            => TopicScope::AccusationCredibility,
    }
}

fn social_observation_to_topic_scope(observation: &SocialObservation) -> TopicScope {
    /* exhaustive match — WitnessedConflict → RouteHazard;
       WitnessedAbsence/SuspectedTheft → AccusationCredibility;
       other current details → GeneralFact */
}

fn institutional_claim_to_topic_scope(claim: &InstitutionalClaim) -> TopicScope {
    /* exhaustive match — office/control/support claims → OfficeHolder;
       accusation/verdict/artifact-refutation claims → AccusationCredibility;
       missing-person status → EntityWhereabouts; faction claims → GeneralFact.
       The current enum has no bounty-validity or price-level arms. */
}
```

The two `_to_topic_scope` helpers use exhaustive `match` over `SocialObservation` and `InstitutionalClaim` variant sets. Read those enums during implementation to enumerate every variant; do not use `_` catch-alls (the compile-failure-on-new-variant property is what makes the design robust).

`belief_topic_to_topic_scope` routes `EntityBelief` to `GeneralFact` as a default because the topic carries only `subject: EntityId`, not the claim aspect; callers with access to the aspect (typically through the belief store's `EntityBeliefClaim`) should call `entity_aspect_to_topic_scope` directly for finer-grained mapping.

### 2. Re-exported from `crates/worldwake-core/src/lib.rs`

Added `pub mod topic_scope;` and `pub use topic_scope::{TopicScope, belief_topic_to_topic_scope, entity_aspect_to_topic_scope};` next to the other core re-exports.

### 3. Unit tests in `topic_scope.rs` `#[cfg(test)]`

- One test per `EntityBeliefAspect` variant asserting the expected `TopicScope`.
- Assert every current `SocialObservationDetail` and `InstitutionalClaim` variant maps to its documented `TopicScope`. `BountyValidity` and `PriceLevel` are reserved enum categories with no current upstream mapping arm.
- Generic bound assertion for `TopicScope: Copy + Hash + Ord + Serialize + Deserialize`.

## Landed Files

- `crates/worldwake-core/src/topic_scope.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — module declaration + re-exports)

## Out of Scope

- `TestimonyReliability` store keyed by `TopicScope` (ticket 002)
- Universal profile types with TopicScope-keyed weights (ticket 003)
- Decision-history payload embedding (ticket 005)
- Diagnostics `BTreeMap<TopicScope, u64>` field (ticket 009)
- Golden E2E scenarios (ticket 011) — D14's "exhaustive mapping unit tests" live with the function here, not in `golden_testimony_reliability.rs`

## Acceptance Result

### Tests Passed

1. Every `EntityBeliefAspect` variant maps to the documented `TopicScope` per the aspect table in Landed Changes §1.
2. `TopicScope` satisfies `Copy + Hash + Ord + Serialize + Deserialize` at compile time.
3. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `TopicScope` is `Copy + Hash + Ord` so it can key authoritative `BTreeMap`s deterministically (per `AGENTS.md` determinism invariant).
2. `belief_topic_to_topic_scope`, `entity_aspect_to_topic_scope`, `social_observation_to_topic_scope`, and `institutional_claim_to_topic_scope` are total functions over their input enums — no `_` catch-alls; new upstream variants cause compile failure.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/topic_scope.rs#[cfg(test)]` — per-variant mapping tests for the 14 `EntityBeliefAspect` variants, every current `SocialObservationDetail`, every current `InstitutionalClaim`, and trait-bound assertion.

### Commands Run

1. `cargo test -p worldwake-core topic_scope`
2. `cargo test -p worldwake-core`
3. `./scripts/verify.sh` (includes `cargo clippy --workspace --all-targets -- -D warnings`)

## Outcome

Completed on 2026-05-17.

- Added the `TopicScope` enum and pure mapping helpers in `crates/worldwake-core/src/topic_scope.rs`.
- Re-exported the new module surface from `crates/worldwake-core/src/lib.rs`.
- Added focused unit coverage for every current upstream enum variant that can map into a topic scope.
- Corrected the active S151 D2 prose to match the live `InstitutionalClaim` surface: `BountyValidity` and `PriceLevel` are reserved `TopicScope` categories with no current upstream mapping arm.

## Deviations

- The drafted spot-check expectation for bounty-validity and price-level mappings was narrowed because the live `InstitutionalClaim` enum has no corresponding variants. The enum categories still landed for later S151 consumers, but this ticket only proves exhaustive mappings for live upstream variants.
- The trait-bound proof landed as a generic compile-time assertion in a unit test rather than the drafted `const _: fn()` sketch.

## Verification Result

- Passed `cargo test -p worldwake-core topic_scope`
- Passed `cargo test -p worldwake-core`
- Passed `./scripts/verify.sh`
