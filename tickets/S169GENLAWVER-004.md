# S169GENLAWVER-004: SearchPlace provider and golden

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — replaces `search_place_provider::try_build` placeholder with real implementation
**Deps**: S169GENLAWVER-002

## Problem

S169GENLAWVER-002 landed the registry dispatch and AskWitness provider with `search_place_provider::try_build` as a placeholder returning `Err(VerificationRejection::BreachClassMismatch)`. Overdue-expectation breaches at the actor's current place — where the lawful response is "look around" — still collapse to `NoEpistemicSubstrate` and fall through to `DowngradeToTypedBarrier`. This ticket replaces the placeholder with a real `try_build` implementation: when the breach classifies as `VerificationNeed::OverdueExpectationAtPlace { expectation, place }` and the target place equals the actor's effective place, the provider constructs a `SearchPlace` action step and returns a `RepairPlanCandidate` with `provider_kind = SearchPlace`.

The ticket also adds the second new golden scenario `verification_search_place_repair.rs` proving the end-to-end flow.

S169GENLAWVER-003 (ConsultRecord) and this ticket are parallel-safe — both depend only on -002 and touch independent submodules.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `SearchPlace` action def at `crates/worldwake-systems/src/search_actions.rs:25` is a real lawful action with `TargetSpec::ActorPlace`, `DurationExpr::ActorInvestigationDisposition`, `VisibilitySpec::SamePlace`, effect schema `EffectStep::SearchPlace`. The existing payload validator `validate_search_place_payload_override` is registered at line 29 and defined at line 132.
2. `ExpectationId` exists at `crates/worldwake-core/src/expectation.rs` as `pub struct ExpectationId(pub u64)`. The breach context's `CausalLink.fact` or `discrepancy_entry` may surface an overdue expectation id.
3. Mixed-layer boundary under audit: provider implementation in `worldwake-ai/src/verification_provider/search_place_provider.rs` consumes `VerificationContext` (belief view, effective place, breach context) and produces a `RepairPlanCandidate`. The `SearchPlace` action handler in `worldwake-systems` is unchanged.
4. Planner-driven AI ticket: same as S169GENLAWVER-003 — the verification step is a `PlannedStep` spliced into the repaired plan; no new `GoalKind` variants. The motivating semantic is FND-17 (surprise from violated expectation): when an expectation at a place is overdue, the lawful response is to physically inspect the place.
5. Heuristic substrate: the provider's same-place legality is enforced by `need.place == ctx.effective_place`. If the overdue expectation is at a remote place, the provider returns `NoLawfulLocalTarget` — the agent cannot search a place they are not at.

## Architecture Check

1. The provider reads only `ctx.belief_view`, `ctx.effective_place`, and the `VerificationNeed::OverdueExpectationAtPlace.place` field. No `&World` access, no remote-state queries.
2. The `SearchPlace` action's `TargetSpec::ActorPlace` means the target index 0 binds to the actor's current place automatically at dispatch; the synthesized payload only needs to encode any payload-validator-required fields beyond the auto-bound target.
3. FND-17 alignment: an overdue expectation is the canonical "surprise from violated expectation" trigger. Searching the place where the expectation was anchored is the lawful evidence-seeking response.

## Verification Layers

1. SearchPlace provider produces a candidate for `VerificationNeed::OverdueExpectationAtPlace` when target place equals actor's place -> focused inline `#[cfg(test)]` test in `search_place_provider.rs`.
2. SearchPlace provider rejects when target place is remote -> focused unit test asserting `Err(VerificationRejection::NoLawfulLocalTarget)`.
3. SearchPlace provider rejects when the breach is not an overdue-expectation breach -> focused unit test asserting `Err(VerificationRejection::BreachClassMismatch)`.
4. Repair seam delegates to SearchPlace provider when classifier produces `OverdueExpectationAtPlace` -> decision-trace assertion in golden (`AgentDecisionTrace.repair_attempts[*].verification_provider = Some(SearchPlace)`).
5. Authoritative `RepairApplied` event records `provider_kind = SearchPlace` and `substitute_target = Some(place_id)` -> event-log delta assertion in `verification_search_place_repair.rs` golden.
6. Synthesized payload passes `validate_search_place_payload_override` -> integration test verifying `requested_affordance_matches` accepts the candidate's `payload_override`.

## What to Change

### 1. Replace `search_place_provider::try_build` placeholder

In `crates/worldwake-ai/src/verification_provider/search_place_provider.rs`:

```rust
pub fn try_build(
    need: &VerificationNeed,
    ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    let (expectation, place) = match need {
        VerificationNeed::OverdueExpectationAtPlace { expectation, place } => (*expectation, *place),
        _ => return Err(VerificationRejection::BreachClassMismatch),
    };

    // Locality: target place must equal actor's effective place.
    if place != ctx.effective_place {
        return Err(VerificationRejection::NoLawfulLocalTarget);
    }

    // Recently-failed check
    if ctx.repair_memory.recently_failed_at(place) {
        return Err(VerificationRejection::RecentlyFailedAtTarget);
    }

    // Build the SearchPlace PlannedStep.
    let step = build_search_place_step(ctx.action_defs, place, expectation)
        .ok_or(VerificationRejection::PayloadValidationFailed)?;

    Ok(VerificationCandidate {
        provider_kind: VerificationProviderKind::SearchPlace,
        target: VerificationTarget::Place(place),
        repair_candidate: RepairPlanCandidate {
            kind: RepairKind::InsertVerification,
            step,
            ..
        },
        source_belief: ctx.belief_view.expectation_belief(expectation).cloned(),
    })
}
```

`belief_view.expectation_belief` accessor — validate against the `GoalBeliefView` trait surface during implementation. If no accessor exists, propose a new belief-view method (5h trait accessor propagation) and update spec if needed.

### 2. Add `build_search_place_step` helper

Private helper in the same submodule. Constructs `PlannedStep` with target index 0 = actor's place (per `TargetSpec::ActorPlace`'s auto-binding), payload override matching `validate_search_place_payload_override` expectations.

### 3. Add SearchPlace golden scenario

New file `crates/worldwake-ai/tests/scenarios/verification_search_place_repair.rs`:

- Setup: agent has an overdue expectation about an entity at the agent's current place (e.g., expected to find resource X here by tick T, T has passed, no observation). Plan has a causal link depending on the expected presence.
- Trigger: plan executes; revalidation discovers the broken link; seam classifies as `OverdueExpectationAtPlace` with `place = actor's current place`; `search_place_provider::try_build` returns a candidate; repaired plan splices `SearchPlace` step.
- Assertions:
  - `AgentDecisionTrace.repair_attempts[*].verification_provider = Some(SearchPlace)`
  - `AgentDecisionTrace.repair_attempts[*].verification_rejections` contains `(AskWitness, BreachClassMismatch)` and `(ConsultRecord, BreachClassMismatch)` (both iterated before SearchPlace, both rejecting the overdue-expectation breach class)
  - `RepairApplied` event emitted with `provider_kind = SearchPlace`, `substitute_target = Some(place_id)`, `repair_kind = InsertVerification`
  - Belief is updated via the `SearchPlace` action's effect schema (not by direct write)
  - The `SearchPlace` action runs to completion; its perception output updates the agent's discovery-related belief

## Files to Touch

- `crates/worldwake-ai/src/verification_provider/search_place_provider.rs` (modify — replace placeholder with real `try_build`)
- `crates/worldwake-ai/tests/scenarios/verification_search_place_repair.rs` (new — golden scenario)
- Likely: `crates/worldwake-sim/src/belief_view.rs` and `crates/worldwake-sim/src/per_agent_belief_view.rs` — if a new accessor like `expectation_belief` is needed (verify during reassessment per 5h trait accessor propagation rule)

## Out of Scope

- Negative omniscience cross-provider E2E golden — S169GENLAWVER-005.
- Real `consult_record_provider::try_build` implementation — S169GENLAWVER-003 (independent, parallel-safe).
- New `GoalKind::SearchPlace` agenda companion variant — explicitly Non-Goal'd by S169 (deferred follow-up).
- Changes to `SearchPlace` action handler or effect schema in `worldwake-systems` — none needed.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai search_place_provider_produces_candidate_for_overdue_expectation_at_actor_place` — provider happy-path.
2. `cargo test -p worldwake-ai search_place_provider_rejects_remote_place` — locality enforcement.
3. `cargo test -p worldwake-ai search_place_provider_rejects_entity_belief_breach` — `BreachClassMismatch` for non-overdue-expectation breaches.
4. `cargo test -p worldwake-ai golden_verification_search_place_repair` — new golden.
5. `cargo test -p worldwake-ai golden_ask_witness_refreshes_stale_report` — S165 parity still passes.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. SearchPlace candidate is only emitted when breach classifies as `OverdueExpectationAtPlace` AND the target place equals the actor's effective place.
2. No `RepairApplied` event records `provider_kind = SearchPlace` AND `substitute_target = Some(place_id)` where `place_id != actor's effective place at repair tick` (locality invariant).
3. The synthesized `SearchPlace` step's payload override passes `validate_search_place_payload_override`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/verification_provider/search_place_provider.rs` inline `#[cfg(test)]` — ~5 focused unit tests covering happy path, locality rejection, breach-class mismatch, payload validator parity, recently-failed-at-target.
2. `crates/worldwake-ai/tests/scenarios/verification_search_place_repair.rs` — full E2E golden.

### Commands

1. `cargo test -p worldwake-ai search_place_provider` — focused unit tests.
2. `cargo test -p worldwake-ai golden_verification_search_place_repair` — new golden.
3. `cargo test -p worldwake-ai golden_ask_witness` — S165 parity regression check.
4. `cargo test --workspace` — full suite.
5. `./scripts/verify.sh` — pre-PR gate.
