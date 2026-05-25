# S169GENLAWVER-003: ConsultRecord provider and golden

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — replaces `consult_record_provider::try_build` placeholder with real implementation
**Deps**: archive/tickets/S169GENLAWVER-002.md

## Problem

S169GENLAWVER-002 landed the registry dispatch and AskWitness provider with `consult_record_provider::try_build` as a placeholder that returns `Err(VerificationRejection::BreachClassMismatch)`. Stale institutional-claim breaches (e.g., `RecordTopic::OfficeRule`, `RecordTopic::BountyExists`) still collapse to `NoEpistemicSubstrate` and fall through to `DowngradeToTypedBarrier`. This ticket replaces the placeholder with a real `try_build` implementation: when the breach classifies as `VerificationNeed::StaleInstitutionalClaim { record_topic }`, the provider searches the actor's co-located entities for a `Record` whose topic matches, constructs a `ConsultRecord` action step, and returns a `RepairPlanCandidate` with `provider_kind = ConsultRecord`.

The implemented proof uses focused provider tests plus an in-crate repair-seam regression in `agent_tick::execution`. During reassessment, the drafted external golden file was rejected because the registry/repair seam that proves provider delegation is private; exporting it only for an external scenario would widen production API surface for test shape rather than simulation behavior.

## Outcome

The ConsultRecord verification provider is implemented for stale institutional-claim breaches. It lawfully searches the actor's belief view for a co-located record matching the broken `RecordTopic`, emits an `InsertVerification` repair candidate with a real `ConsultRecord` step, and records `provider_kind = ConsultRecord` in the repair-applied event when the private repair seam selects it.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ConsultRecord` action def at `crates/worldwake-systems/src/consult_record_actions.rs:25` is a real lawful action with `TargetSpec::EntityAtActorPlace { kind: EntityKind::Record }`, `DurationExpr::ConsultRecord { target_index: 0 }`, `VisibilitySpec::SamePlace`, and effect schema `EffectStep::ConsultRecord`. The existing payload validator `validate_consult_record_payload_override` is registered at line 28 and defined at line 147.
2. `RecordTopic` enum at `crates/worldwake-core/src/causal_link.rs:62` provides the topic discriminator for records. Variants include `OfficeRule`, `BountyExists`, `RouteSafety`, `TestifiedAbout`, `PriceObserved` (per agent 1's report on /reassess-spec). The provider classifies whether a co-located record's recorded topic matches the breach's `record_topic` by reading the actor's belief about that record (not by querying the authoritative record contents directly).
3. Mixed-layer boundary under audit: provider implementation lives in `worldwake-ai/src/verification_provider/consult_record_provider.rs`; consumes `VerificationContext.belief_view` (from `worldwake-sim`) and produces `RepairPlanCandidate` (from `worldwake-ai`); the authoritative `ConsultRecord` action handler lives in `worldwake-systems`. No cross-system direct calls.
4. Planner-driven AI ticket: live `GoalKind` under test remains the original goal that triggered the breach (e.g., a goal whose `CausalLink` depended on a believed institutional fact). The verification step is spliced into the repaired plan as a *step*, not a new goal — no new `GoalKind` variants are added (S169 Non-Goal #2).
5. Heuristic substrate: the provider's same-place legality check is enforced by reading `ctx.belief_view.entities_at(ctx.effective_place)` and filtering for `kind = EntityKind::Record`. The actor's belief about the record's topic relevance (not the authoritative record contents) is the lawful gate — preserves FND-14B (planner-visible inputs must be belief-backed).
6. archive/tickets/S169GENLAWVER-002.md did not add target-scoped repair memory to `VerificationContext`, and live `RepairMemory` is keyed by `BreachSignature` plus `RepairKind`, not by provider target. Per `docs/FOUNDATIONS.md` FND-18 and FND-22A, this ticket does not invent a `recently_failed_at(record)` shortcut. `RecentlyFailedAtTarget` remains reserved for a future explicitly specified target-scoped verification memory substrate, if one is needed.
7. The originally drafted external golden would either miss the private registry seam or require widening API surface solely for test access. Per user-approved option 1 on 2026-05-25, this ticket keeps the stronger in-crate seam regression and records the deviation instead of adding a weaker `verification_consult_record_repair.rs`.

## Architecture Check

1. The provider reads only `ctx.belief_view` and `ctx.effective_place` — no `&World` access, no remote-state queries. Locality enforcement is compile-checked by the `VerificationContext` shape (introduced in 002).
2. Payload synthesis follows the existing `validate_consult_record_payload_override` validator's expectations. The repair-side synthesized payload is structurally identical to an affordance-derived payload (same target index, same record entity); D7's validator-parity unit test verifies this.
3. No new `GoalKind` variant. The verification step is a single `PlannedStep` spliced into a plan tail by the established repair pipeline.
4. Co-located institutional records (e.g., a posted bounty, a stamped office-rule) are real `Record` entities per FND-18. The provider treats them as carriers per FND-15; consulting them is a lawful authoritative action, not a belief-write shortcut.

## Verification Layers Landed

1. ConsultRecord provider produces a candidate for `VerificationNeed::StaleInstitutionalClaim` -> focused inline `#[cfg(test)]` test in `consult_record_provider.rs` constructing a stale institutional-claim breach with a co-located matching record and asserting `Ok(VerificationCandidate { provider_kind: ConsultRecord, .. })`.
2. ConsultRecord provider rejects when no co-located matching record exists -> focused unit test asserting `Err(VerificationRejection::NoLawfulLocalTarget)`.
3. ConsultRecord provider rejects when the breach is a `StaleEntityBelief` (not a `StaleInstitutionalClaim`) -> focused unit test asserting `Err(VerificationRejection::BreachClassMismatch)`.
4. Repair seam delegates to ConsultRecord provider when classifier produces `StaleInstitutionalClaim` -> in-crate `agent_tick::execution` regression asserts `outcome.verification_provider = Some(ConsultRecord)` and ordered registry rejections for `AskWitness` and `SearchPlace`.
5. Authoritative `RepairApplied` event records `provider_kind = ConsultRecord` and `substitute_target = Some(record_id)` -> same seam regression applies the repaired plan and asserts the event-log `RepairAppliedPayload`.
6. Synthesized payload passes `validate_consult_record_payload_override` -> integration test verifying `requested_affordance_matches` accepts the candidate's `payload_override`.

## Implementation Summary

1. Replaced `consult_record_provider::try_build` placeholder with a real `StaleInstitutionalClaim` implementation.
2. Added private helpers that locate a co-located believed `Record`, match its `RecordData` against the broken `RecordTopic`, and synthesize a `ConsultRecord` `PlannedStep` with a payload accepted by the registered validator.
3. Added focused provider tests for happy path, locality rejection, breach-class mismatch, and payload validator parity.
4. Added `record_breach_inserts_consult_record_verification_and_records_provider` in `agent_tick::execution` to prove classifier/registry/provider/plan-repair/event-payload composition at the private seam.
5. Extended `plan_repair::provider_supports_fact` so record-backed institutional causal providers can support their corresponding facts when evaluating inserted verification candidates.

## Files Touched

- `crates/worldwake-ai/src/verification_provider/consult_record_provider.rs` (modify — replace placeholder with real `try_build`)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — seam regression covering registry delegation and `RepairAppliedPayload`)
- `crates/worldwake-ai/src/plan_repair.rs` (modify — record-topic/fact support mapping)
- `crates/worldwake-ai/src/verification_provider/mod.rs` (modify — registry placeholder test now only covers `SearchPlace`)

## Out of Scope

- Real `search_place_provider::try_build` implementation — S169GENLAWVER-004 (placeholder remains).
- Negative omniscience cross-provider E2E golden — S169GENLAWVER-005.
- New `GoalKind::ConsultRecord` agenda companion variant — explicitly Non-Goal'd by S169 (deferred to a follow-up spec).
- Changes to `ConsultRecord` action handler or effect schema in `worldwake-systems` — no authoritative changes needed; the existing action is sufficient.

## Acceptance Criteria

### Proof Passed

1. Passed `cargo test -p worldwake-ai consult_record_provider_produces_candidate_for_stale_institutional_claim` — provider happy-path.
2. Passed `cargo test -p worldwake-ai consult_record_provider_rejects_no_local_record` — locality enforcement.
3. Passed `cargo test -p worldwake-ai consult_record_provider_rejects_entity_belief_breach` — `BreachClassMismatch` for non-institutional breaches.
4. Passed `cargo test -p worldwake-ai record_breach_inserts_consult_record_verification_and_records_provider` — private seam regression.
5. Passed `cargo test -p worldwake-ai verification_provider` — registry/provider lane.
6. Passed `cargo test -p worldwake-ai plan_repair` — repair candidate selection lane.
7. Passed `cargo test -p worldwake-ai golden_ask_witness` — S165 parity still passes.
8. Passed `cargo test -p worldwake-ai` — affected crate suite.

### Invariants

1. ConsultRecord candidate is only emitted when the breach classifies as `StaleInstitutionalClaim` AND a co-located record with a matching belief-known topic exists.
2. No `RepairApplied` event records `provider_kind = ConsultRecord` AND `substitute_target = Some(record_id)` where `record_id` is not at the actor's effective place (negative omniscience invariant — verified in 005's E2E golden, but partially exercised by this ticket's per-provider locality unit test).
3. The synthesized `ConsultRecord` step's payload override passes `validate_consult_record_payload_override`.

## Verification Result

### Landed Tests

1. Passed `crates/worldwake-ai/src/verification_provider/consult_record_provider.rs` inline `#[cfg(test)]` — focused unit tests covering happy path, locality rejection, breach-class mismatch, and payload validator parity.
2. Passed `crates/worldwake-ai/src/agent_tick/execution.rs` inline regression — seam → registry → repair → event-log assertions.

### Command Results

1. Passed `cargo test -p worldwake-ai consult_record_provider`.
2. Passed `cargo test -p worldwake-ai consult_record_provider_produces_candidate_for_stale_institutional_claim`.
3. Passed `cargo test -p worldwake-ai consult_record_provider_rejects_no_local_record`.
4. Passed `cargo test -p worldwake-ai consult_record_provider_rejects_entity_belief_breach`.
5. Passed `cargo test -p worldwake-ai record_breach_inserts_consult_record_verification_and_records_provider`.
6. Passed `cargo test -p worldwake-ai verification_provider`.
7. Passed `cargo test -p worldwake-ai plan_repair`.
8. Passed `cargo test -p worldwake-ai golden_ask_witness`.
9. Passed `cargo test -p worldwake-ai`.
10. Passed `cargo clippy --workspace --all-targets -- -D warnings`.

### Deviations

1. Waived `crates/worldwake-ai/tests/scenarios/verification_consult_record_repair.rs`. The requested seam is private and is better proven by the in-crate regression than by adding a test-only API export or a weaker external scenario.
2. Waived `RecentlyFailedAtTarget`; live repair memory is breach/kind scoped, and target-scoped provider memory needs its own specified substrate before implementation.
3. Waived new belief-view trait methods. Existing `record_data`, `entities_at`, `entity_kind`, and `effective_place` accessors provide the lawful belief-backed record substrate.
