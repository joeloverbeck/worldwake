# S169GENLAWVER-004: SearchPlace provider and seam proof

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — replaces `search_place_provider::try_build` placeholder with real implementation
**Deps**: archive/tickets/S169GENLAWVER-002.md

## Problem

S169GENLAWVER-002 landed the registry dispatch and AskWitness provider with `search_place_provider::try_build` as a placeholder returning `Err(VerificationRejection::BreachClassMismatch)`. Before this ticket, overdue-expectation breaches at the actor's effective place — where the lawful response is "look around" — collapsed to `NoEpistemicSubstrate` and fell through to `DowngradeToTypedBarrier`. This ticket replaced the placeholder with a real `try_build` implementation: when the breach classifies as `VerificationNeed::OverdueExpectationAtPlace { expectation, place }` and the target place equals the actor's effective place, the provider constructs a `SearchPlace` action step and returns a `RepairPlanCandidate` with `provider_kind = SearchPlace`.

The implemented proof uses focused provider tests plus an in-crate repair-seam regression in `agent_tick::execution`. During reassessment, the drafted external golden file was rejected for the same reason as archive/tickets/S169GENLAWVER-003.md: the registry/repair seam that proves provider delegation is private, and exporting it only for an external scenario would widen production API surface for test shape rather than simulation behavior.

## Outcome

The SearchPlace verification provider is implemented for overdue-expectation breaches at the actor's effective place. It lawfully reads the actor's expectation store and violation-disposition profile through `GoalBeliefView`, emits an `InsertVerification` repair candidate with a real `SearchPlace` step, and records `provider_kind = SearchPlace` in the repair-applied event when the private repair seam selects it.

archive/tickets/S169GENLAWVER-003.md (ConsultRecord) and this ticket are parallel-safe — both depend only on -002 and touch independent submodules.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `SearchPlace` action def at `crates/worldwake-systems/src/search_actions.rs:25` is a real lawful action with `TargetSpec::ActorPlace`, `DurationExpr::ActorInvestigationDisposition`, `VisibilitySpec::SamePlace`, effect schema `EffectStep::SearchPlace`. The existing payload validator `validate_search_place_payload_override` is registered at line 29 and defined at line 132.
2. `ExpectationId` exists at `crates/worldwake-core/src/expectation.rs` as `pub struct ExpectationId(pub u64)`. The breach context's `CausalLink.fact` or `discrepancy_entry` may surface an overdue expectation id.
3. Mixed-layer boundary under audit: provider implementation in `worldwake-ai/src/verification_provider/search_place_provider.rs` consumes `VerificationContext` (belief view, effective place, breach context) and produces a `RepairPlanCandidate`. The `SearchPlace` action handler in `worldwake-systems` is unchanged.
4. Planner-driven AI ticket: same as archive/tickets/S169GENLAWVER-003.md — the verification step is a `PlannedStep` spliced into the repaired plan; no new `GoalKind` variants. The motivating semantic is FND-17 (surprise from violated expectation): when an expectation at a place is overdue, the lawful response is to physically inspect the place.
5. Heuristic substrate: the provider's same-place legality is enforced by `need.place == ctx.effective_place`. If the overdue expectation is at a remote place, the provider returns `NoLawfulLocalTarget` — the agent cannot search a place they are not at.
6. archive/tickets/S169GENLAWVER-002.md and archive/tickets/S169GENLAWVER-003.md did not add target-scoped repair memory to `VerificationContext`, and live `RepairMemory` is keyed by `BreachSignature` plus `RepairKind`, not by provider target. Per `docs/FOUNDATIONS.md` FND-18 and FND-22A, this ticket must not invent a `recently_failed_at(place)` shortcut. `RecentlyFailedAtTarget` remains reserved for a future explicitly specified target-scoped verification memory substrate, if one is needed.
7. The originally drafted external golden would either miss the private registry seam or require widening API surface solely for test access. Per the same user-approved option 1 applied to archive/tickets/S169GENLAWVER-003.md, this ticket keeps the stronger in-crate seam regression and records the deviation instead of adding a weaker `verification_search_place_repair.rs`.

## Architecture Check

1. The provider reads only `ctx.belief_view`, `ctx.effective_place`, and the `VerificationNeed::OverdueExpectationAtPlace.place` field. No `&World` access, no remote-state queries.
2. The `SearchPlace` action's `TargetSpec::ActorPlace` means the target index 0 binds to the actor's current place automatically at dispatch; the synthesized payload only needs to encode any payload-validator-required fields beyond the auto-bound target.
3. FND-17 alignment: an overdue expectation is the canonical "surprise from violated expectation" trigger. Searching the place where the expectation was anchored is the lawful evidence-seeking response.

## Verification Layers Landed

1. SearchPlace provider produces a candidate for `VerificationNeed::OverdueExpectationAtPlace` when target place equals actor's place -> focused inline `#[cfg(test)]` test in `search_place_provider.rs`.
2. SearchPlace provider rejects when target place is remote -> focused unit test asserting `Err(VerificationRejection::NoLawfulLocalTarget)`.
3. SearchPlace provider rejects when the breach is not an overdue-expectation breach -> focused unit test asserting `Err(VerificationRejection::BreachClassMismatch)`.
4. Repair seam delegates to SearchPlace provider when classifier produces `OverdueExpectationAtPlace` -> in-crate `agent_tick::execution` regression asserts `outcome.verification_provider = Some(SearchPlace)` and ordered registry rejections for `AskWitness` and `ConsultRecord`.
5. Authoritative `RepairApplied` event records `provider_kind = SearchPlace` and `substitute_target = Some(place_id)` -> same seam regression applies the repaired plan and asserts the event-log `RepairAppliedPayload`.
6. Synthesized payload passes `validate_search_place_payload_override` -> integration test verifying `requested_affordance_matches` accepts the candidate's `payload_override`.

## Implementation Summary

1. Replaced `search_place_provider::try_build` placeholder with a real `OverdueExpectationAtPlace` implementation.
2. Added private helpers that verify local-place legality, find the matching overdue expectation record, require a violation-disposition profile, and synthesize a `SearchPlace` `PlannedStep` with a payload accepted by the registered validator.
3. Added focused provider tests for happy path, locality rejection, breach-class mismatch, and payload validator parity.
4. Added `expectation_breach_inserts_search_place_verification_and_records_provider` in `agent_tick::execution` to prove classifier/registry/provider/plan-repair/event-payload composition at the private seam.
5. Extended `plan_repair::provider_supports_fact` so expectation-backed target-present causal providers can support inserted SearchPlace verification candidates.

## Files Touched

- `crates/worldwake-ai/src/verification_provider/search_place_provider.rs` (modify — replace placeholder with real `try_build`)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — seam regression covering registry delegation and `RepairAppliedPayload`)
- `crates/worldwake-ai/src/plan_repair.rs` (modify — expectation/fact support mapping)

## Out of Scope

- Negative omniscience cross-provider seam proof — archive/tickets/S169GENLAWVER-005.md.
- Real `consult_record_provider::try_build` implementation — archive/tickets/S169GENLAWVER-003.md (independent, parallel-safe).
- New `GoalKind::SearchPlace` agenda companion variant — explicitly Non-Goal'd by S169 (deferred follow-up).
- Changes to `SearchPlace` action handler or effect schema in `worldwake-systems` — none needed.

## Acceptance Criteria

### Proof Passed

1. Passed `cargo test -p worldwake-ai search_place_provider_produces_candidate_for_overdue_expectation_at_actor_place` — provider happy-path.
2. Passed `cargo test -p worldwake-ai search_place_provider_rejects_remote_place` — locality enforcement.
3. Passed `cargo test -p worldwake-ai search_place_provider_rejects_entity_belief_breach` — `BreachClassMismatch` for non-overdue-expectation breaches.
4. Passed `cargo test -p worldwake-ai expectation_breach_inserts_search_place_verification_and_records_provider` — private seam regression.
5. Passed `cargo test -p worldwake-ai verification_provider` — registry/provider lane.
6. Passed `cargo test -p worldwake-ai plan_repair` — repair candidate selection lane.
7. Passed `cargo test -p worldwake-ai golden_ask_witness` — S165 parity still passes.
8. Passed `cargo test -p worldwake-ai` — affected crate suite.

### Invariants

1. SearchPlace candidate is only emitted when breach classifies as `OverdueExpectationAtPlace` AND the target place equals the actor's effective place.
2. No `RepairApplied` event records `provider_kind = SearchPlace` AND `substitute_target = Some(place_id)` where `place_id != actor's effective place at repair tick` (locality invariant).
3. The synthesized `SearchPlace` step's payload override passes `validate_search_place_payload_override`.

## Verification Result

### Landed Tests

1. Passed `crates/worldwake-ai/src/verification_provider/search_place_provider.rs` inline `#[cfg(test)]` — focused unit tests covering happy path, locality rejection, breach-class mismatch, and payload validator parity.
2. Passed `crates/worldwake-ai/src/agent_tick/execution.rs` inline regression — seam → registry → repair → event-log assertions.

### Command Results

1. Passed `cargo test -p worldwake-ai search_place_provider`.
2. Passed `cargo test -p worldwake-ai search_place_provider_produces_candidate_for_overdue_expectation_at_actor_place`.
3. Passed `cargo test -p worldwake-ai search_place_provider_rejects_remote_place`.
4. Passed `cargo test -p worldwake-ai search_place_provider_rejects_entity_belief_breach`.
5. Passed `cargo test -p worldwake-ai expectation_breach_inserts_search_place_verification_and_records_provider`.
6. Passed `cargo test -p worldwake-ai verification_provider`.
7. Passed `cargo test -p worldwake-ai plan_repair`.
8. Passed `cargo test -p worldwake-ai golden_ask_witness`.
9. Passed `cargo test -p worldwake-ai`.
10. Passed `cargo clippy --workspace --all-targets -- -D warnings`.

### Deviations

1. Waived `crates/worldwake-ai/tests/scenarios/verification_search_place_repair.rs`. The requested seam is private and is better proven by the in-crate regression than by adding a test-only API export or a weaker external scenario.
2. Waived `RecentlyFailedAtTarget`; live repair memory is breach/kind scoped, and target-scoped provider memory needs its own specified substrate before implementation.
3. Waived new belief-view trait methods. Existing `expectation_store` and `violation_disposition_profile` accessors provide the lawful belief-backed/action-validator substrate.
