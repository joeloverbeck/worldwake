# S169GENLAWVER-003: ConsultRecord provider and golden

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — replaces `consult_record_provider::try_build` placeholder with real implementation
**Deps**: archive/tickets/S169GENLAWVER-002.md

## Problem

S169GENLAWVER-002 landed the registry dispatch and AskWitness provider with `consult_record_provider::try_build` as a placeholder that returns `Err(VerificationRejection::BreachClassMismatch)`. Stale institutional-claim breaches (e.g., `RecordTopic::OfficeRule`, `RecordTopic::BountyExists`) still collapse to `NoEpistemicSubstrate` and fall through to `DowngradeToTypedBarrier`. This ticket replaces the placeholder with a real `try_build` implementation: when the breach classifies as `VerificationNeed::StaleInstitutionalClaim { record_topic }`, the provider searches the actor's co-located entities for a `Record` whose topic matches, constructs a `ConsultRecord` action step, and returns a `RepairPlanCandidate` with `provider_kind = ConsultRecord`.

The ticket also adds the first new golden scenario `verification_consult_record_repair.rs` proving that a stale institutional-claim belief routes through the registry to a real `ConsultRecord` repair step and produces the authoritative `RepairApplied` event with `provider_kind = ConsultRecord`.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ConsultRecord` action def at `crates/worldwake-systems/src/consult_record_actions.rs:25` is a real lawful action with `TargetSpec::EntityAtActorPlace { kind: EntityKind::Record }`, `DurationExpr::ConsultRecord { target_index: 0 }`, `VisibilitySpec::SamePlace`, and effect schema `EffectStep::ConsultRecord`. The existing payload validator `validate_consult_record_payload_override` is registered at line 28 and defined at line 147.
2. `RecordTopic` enum at `crates/worldwake-core/src/causal_link.rs:62` provides the topic discriminator for records. Variants include `OfficeRule`, `BountyExists`, `RouteSafety`, `TestifiedAbout`, `PriceObserved` (per agent 1's report on /reassess-spec). The provider classifies whether a co-located record's recorded topic matches the breach's `record_topic` by reading the actor's belief about that record (not by querying the authoritative record contents directly).
3. Mixed-layer boundary under audit: provider implementation lives in `worldwake-ai/src/verification_provider/consult_record_provider.rs`; consumes `VerificationContext.belief_view` (from `worldwake-sim`) and produces `RepairPlanCandidate` (from `worldwake-ai`); the authoritative `ConsultRecord` action handler lives in `worldwake-systems`. No cross-system direct calls.
4. Planner-driven AI ticket: live `GoalKind` under test remains the original goal that triggered the breach (e.g., a goal whose `CausalLink` depended on a believed institutional fact). The verification step is spliced into the repaired plan as a *step*, not a new goal — no new `GoalKind` variants are added (S169 Non-Goal #2).
5. Heuristic substrate: the provider's same-place legality check is enforced by reading `ctx.belief_view.entities_at(ctx.effective_place)` and filtering for `kind = EntityKind::Record`. The actor's belief about the record's topic relevance (not the authoritative record contents) is the lawful gate — preserves FND-14B (planner-visible inputs must be belief-backed).
6. archive/tickets/S169GENLAWVER-002.md did not add `repair_memory` to `VerificationContext`; if this ticket retains the `RecentlyFailedAtTarget` behavior below, it must first extend the context or perform an equivalent seam-side recent-failure check before constructing the provider candidate.

## Architecture Check

1. The provider reads only `ctx.belief_view` and `ctx.effective_place` — no `&World` access, no remote-state queries. Locality enforcement is compile-checked by the `VerificationContext` shape (introduced in 002).
2. Payload synthesis follows the existing `validate_consult_record_payload_override` validator's expectations. The repair-side synthesized payload is structurally identical to an affordance-derived payload (same target index, same record entity); D7's validator-parity unit test verifies this.
3. No new `GoalKind` variant. The verification step is a single `PlannedStep` spliced into a plan tail by the established repair pipeline.
4. Co-located institutional records (e.g., a posted bounty, a stamped office-rule) are real `Record` entities per FND-18. The provider treats them as carriers per FND-15; consulting them is a lawful authoritative action, not a belief-write shortcut.

## Verification Layers

1. ConsultRecord provider produces a candidate for `VerificationNeed::StaleInstitutionalClaim` -> focused inline `#[cfg(test)]` test in `consult_record_provider.rs` constructing a stale institutional-claim breach with a co-located matching record and asserting `Ok(VerificationCandidate { provider_kind: ConsultRecord, .. })`.
2. ConsultRecord provider rejects when no co-located matching record exists -> focused unit test asserting `Err(VerificationRejection::NoLawfulLocalTarget)`.
3. ConsultRecord provider rejects when the breach is a `StaleEntityBelief` (not a `StaleInstitutionalClaim`) -> focused unit test asserting `Err(VerificationRejection::BreachClassMismatch)`.
4. Repair seam delegates to ConsultRecord provider when classifier produces `StaleInstitutionalClaim` -> decision-trace assertion in golden scenario (`AgentDecisionTrace.repair_attempts[*].verification_provider = Some(ConsultRecord)`).
5. Authoritative `RepairApplied` event records `provider_kind = ConsultRecord` and `substitute_target = Some(record_id)` -> event-log delta assertion in `verification_consult_record_repair.rs` golden.
6. Synthesized payload passes `validate_consult_record_payload_override` -> integration test verifying `requested_affordance_matches` accepts the candidate's `payload_override`.

## What to Change

### 1. Replace `consult_record_provider::try_build` placeholder

In `crates/worldwake-ai/src/verification_provider/consult_record_provider.rs`:

```rust
pub fn try_build(
    need: &VerificationNeed,
    ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    let record_topic = match need {
        VerificationNeed::StaleInstitutionalClaim { record_topic } => *record_topic,
        _ => return Err(VerificationRejection::BreachClassMismatch),
    };

    // Find a co-located record whose belief-recorded topic is relevant to record_topic.
    let local_record = ctx
        .belief_view
        .entities_at(ctx.effective_place)
        .into_iter()
        .filter(|e| ctx.belief_view.entity_kind(*e) == Some(EntityKind::Record))
        .find(|record| ctx.belief_view.record_topic_matches(*record, record_topic))
        .ok_or(VerificationRejection::NoLawfulLocalTarget)?;

    // Recently-failed check. If this remains provider-local, add the needed
    // repair-memory surface to VerificationContext in this ticket.
    if ctx.repair_memory.recently_failed_at(local_record) {
        return Err(VerificationRejection::RecentlyFailedAtTarget);
    }

    // Build the ConsultRecord PlannedStep.
    let step = build_consult_record_step(ctx.action_defs, local_record)
        .ok_or(VerificationRejection::PayloadValidationFailed)?;

    Ok(VerificationCandidate {
        provider_kind: VerificationProviderKind::ConsultRecord,
        target: VerificationTarget::Record(local_record),
        repair_candidate: RepairPlanCandidate {
            kind: RepairKind::InsertVerification,
            // ... fact, provider, step, reusable_suffix_index per existing pattern
            step,
            ..
        },
        source_belief: ctx.belief_view.record_belief(local_record).cloned(),
    })
}
```

Exact `belief_view.record_topic_matches` and `belief_view.record_belief` accessor names need to be validated against the `GoalBeliefView` trait surface during implementation; if no accessor exists for "actor's belief about a record's topic," a new belief-view method may be needed (per Step 3.5 5h "Trait accessor propagation" — flag during reassessment).

### 2. Add `build_consult_record_step` helper

In `crates/worldwake-ai/src/verification_provider/consult_record_provider.rs`, a private helper that constructs the `PlannedStep` with target index 0 set to the record `EntityId` and payload override matching `validate_consult_record_payload_override`'s expectations.

### 3. Add ConsultRecord golden scenario

New file `crates/worldwake-ai/tests/scenarios/verification_consult_record_repair.rs`:

- Setup: agent holds a stale `RecordTopic::OfficeRule`-class belief; a co-located record exists with the matching topic; the agent's plan has a causal link that depends on the stale belief.
- Trigger: the plan executes; revalidation discovers the broken link; the repair seam classifies the breach as `StaleInstitutionalClaim`; the registry iterates and `consult_record_provider::try_build` returns a candidate; the repaired plan splices a `ConsultRecord` step.
- Assertions:
  - `AgentDecisionTrace.repair_attempts[*].verification_provider = Some(ConsultRecord)`
  - `AgentDecisionTrace.repair_attempts[*].verification_rejections` contains `(AskWitness, BreachClassMismatch)` (AskWitness was iterated first and rejected the institutional-claim breach class)
  - Authoritative `RepairApplied` event emitted with `provider_kind = ConsultRecord`, `substitute_target = Some(record_id)`, `repair_kind = InsertVerification`
  - Belief is updated via the `ConsultRecord` action's effect schema (not by direct write)
  - No `RepairFailure::NoEpistemicSubstrate` recorded for this breach signature

## Files to Touch

- `crates/worldwake-ai/src/verification_provider/consult_record_provider.rs` (modify — replace placeholder with real `try_build`)
- `crates/worldwake-ai/tests/scenarios/verification_consult_record_repair.rs` (new — golden scenario)
- Likely: `crates/worldwake-ai/src/verification_provider/mod.rs` — if this ticket keeps `RecentlyFailedAtTarget` as a provider-local rejection, extend `VerificationContext` with the repair-memory surface required by the snippet above.
- Likely: `crates/worldwake-sim/src/belief_view.rs` and `crates/worldwake-sim/src/per_agent_belief_view.rs` — if a new accessor like `record_topic_matches` is needed (verify during reassessment per 5h trait accessor propagation rule)

## Out of Scope

- Real `search_place_provider::try_build` implementation — S169GENLAWVER-004 (placeholder remains).
- Negative omniscience cross-provider E2E golden — S169GENLAWVER-005.
- New `GoalKind::ConsultRecord` agenda companion variant — explicitly Non-Goal'd by S169 (deferred to a follow-up spec).
- Changes to `ConsultRecord` action handler or effect schema in `worldwake-systems` — no authoritative changes needed; the existing action is sufficient.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai consult_record_provider_produces_candidate_for_stale_institutional_claim` — provider happy-path.
2. `cargo test -p worldwake-ai consult_record_provider_rejects_no_local_record` — locality enforcement.
3. `cargo test -p worldwake-ai consult_record_provider_rejects_entity_belief_breach` — `BreachClassMismatch` for non-institutional breaches.
4. `cargo test -p worldwake-ai golden_verification_consult_record_repair` — new golden.
5. `cargo test -p worldwake-ai golden_ask_witness_refreshes_stale_report` — S165 parity still passes (the registry's AskWitness arm unchanged).
6. Existing suite: `cargo test --workspace`.

### Invariants

1. ConsultRecord candidate is only emitted when the breach classifies as `StaleInstitutionalClaim` AND a co-located record with a matching belief-known topic exists.
2. No `RepairApplied` event records `provider_kind = ConsultRecord` AND `substitute_target = Some(record_id)` where `record_id` is not at the actor's effective place (negative omniscience invariant — verified in 005's E2E golden, but partially exercised by this ticket's per-provider locality unit test).
3. The synthesized `ConsultRecord` step's payload override passes `validate_consult_record_payload_override`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/verification_provider/consult_record_provider.rs` inline `#[cfg(test)]` — ~5 focused unit tests covering happy path, locality rejection, breach-class mismatch, payload validator parity, and recently-failed-at-target.
2. `crates/worldwake-ai/tests/scenarios/verification_consult_record_repair.rs` — full E2E golden with seam → registry → repair → event-log assertions.

### Commands

1. `cargo test -p worldwake-ai consult_record_provider` — focused unit tests.
2. `cargo test -p worldwake-ai golden_verification_consult_record_repair` — new golden.
3. `cargo test -p worldwake-ai golden_ask_witness` — S165 parity regression check.
4. `cargo test --workspace` — full suite.
5. `./scripts/verify.sh` — pre-PR gate.
