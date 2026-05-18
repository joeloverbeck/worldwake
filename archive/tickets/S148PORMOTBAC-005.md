# S148PORMOTBAC-005: Core-residing IntentionResumeCondition and IntentionAbandonCondition with ArtifactLegalEffectTag mirror

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - added core-resident `IntentionResumeCondition` and `IntentionAbandonCondition` enums; added payload-free `ArtifactLegalEffectTag` in `social_artifact.rs`; exported the new types from `worldwake-core`
**Deps**: `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S148 D6 extends `IntentionFrame` with `resume_conditions` and `abandon_conditions`. Those condition types must live in `worldwake-core` because `IntentionFrame` is core-resident. The existing HTN `BeliefPredicate` in `worldwake-ai` is the wrong crate and the wrong abstraction, so this ticket added dedicated core lifecycle predicate enums plus a payload-free `ArtifactLegalEffectTag` mirror for artifact legal-effect transitions.

## Assumption Reassessment (2026-05-18)

1. The live core types needed by the condition enums exist at the expected boundaries: `BeliefStatusTag` in `decision_event_payload.rs`, `OpportunityAnchor` in `goal.rs`, `MotiveSourceDiscriminant` in `motive_source.rs`, `FrameAssumption` in `intention_frame.rs`, and `EntityId` in `ids.rs`.
2. `ArtifactLegalEffect` in `social_artifact.rs` has six variants: `None`, `Active`, `Suspended`, `Expired`, `Revoked`, and `Fulfilled`. No payload-free mirror existed before this ticket.
3. `OpportunityAnchor` has only `Place`, `Entity`, and `None`; the condition tests use those live variants rather than inventing a goal-anchor variant.
4. Shared abstraction under audit: the core-side lifecycle predicate surface. Ticket 006 consumes these types as fields on `IntentionFrame`; ticket 007 consumes them in the evaluator. The HTN predicate surface remains separate.

## Architecture Check

1. The new lifecycle predicate enums live in `worldwake-core`, so no upward dependency is introduced.
2. `ArtifactLegalEffectTag` is a mechanical 1:1 mirror of `ArtifactLegalEffect`, with the same declaration order and no merged or renamed variants.
3. The conversion from `&ArtifactLegalEffect` to `ArtifactLegalEffectTag` has a single implementation beside the source enum in `social_artifact.rs`.
4. The types are first-class serializable values, preserving deterministic replay and making future resume/abandon trace surfaces nameable.

## Verified Layers

1. Enum shape and serialization: focused `intention_condition` tests round-trip every resume and abandon variant through bincode and exercise derived ordering.
2. Legal-effect tag conversion: focused `social_artifact` test enumerates each `ArtifactLegalEffect` variant and asserts the corresponding tag.
3. Cross-crate visibility and trait shape: `worldwake-core` tests plus workspace clippy compile all targets with the new public exports.

## Landed Changes

1. `crates/worldwake-core/src/intention_condition.rs`
   - Added `IntentionResumeCondition`.
   - Added `IntentionAbandonCondition`.
   - Added bincode round-trip tests and deterministic ordering tests.
2. `crates/worldwake-core/src/social_artifact.rs`
   - Added `ArtifactLegalEffectTag`.
   - Added `impl From<&ArtifactLegalEffect> for ArtifactLegalEffectTag`.
   - Extended existing social-artifact trait coverage and added variant-mirror coverage.
3. `crates/worldwake-core/src/lib.rs`
   - Added the `intention_condition` module.
   - Re-exported `IntentionResumeCondition`, `IntentionAbandonCondition`, and `ArtifactLegalEffectTag`.

## Out of Scope

- Adding the new condition vectors to `IntentionFrame` remains owned by ticket 006.
- Implementing resume/abandon evaluation remains owned by ticket 007.
- Adding `Discrepancy::AbandonConditionFired` or an abandon-condition discriminant remains owned by ticket 007.
- Authoring `explicit_claims` semantics remains owned by ticket 006 and ticket 007.

## Acceptance Result

1. `IntentionResumeCondition` and `IntentionAbandonCondition` are defined exactly once in `worldwake-core`.
2. `ArtifactLegalEffectTag` mirrors `ArtifactLegalEffect` exactly and derives `Copy`.
3. The only conversion from legal-effect payload to tag is the `From<&ArtifactLegalEffect>` impl in `social_artifact.rs`.
4. The new public exports compile across the workspace.

## Test Plan Result

1. Added focused condition round-trip and ordering tests in `intention_condition.rs`.
2. Added focused legal-effect tag mirror coverage in `social_artifact.rs`.
3. Ran focused core tests, crate-wide core tests, workspace tests, and workspace all-target clippy.

## Outcome

Completed on 2026-05-18.

- Added core-resident lifecycle condition enums for ticket 006's `IntentionFrame` fields and ticket 007's evaluator.
- Added `ArtifactLegalEffectTag` and a single source-adjacent conversion from `ArtifactLegalEffect`.
- Exported all new public types from `worldwake-core`.

## Deviations

- The drafted combined command `cargo test -p worldwake-core intention_condition social_artifact` was not used as proof because Cargo accepts a single test-name filter. Verification used separate focused selectors plus `cargo test -p worldwake-core`.
- The live `OpportunityAnchor` shape has no goal-anchor variant; tests use the existing `Place` anchor for `OpportunityForeverGone`.
- The canonical broad verification was run as explicit workspace test and clippy commands rather than `./scripts/verify.sh`, matching the required proof surfaces without rerunning `cargo fmt --all -- --check`.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-core intention_condition`.
- Passed `cargo test -p worldwake-core social_artifact::tests::artifact_legal_effect_tag_mirrors_every_variant`.
- Passed `cargo test -p worldwake-core`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo test --workspace`.
