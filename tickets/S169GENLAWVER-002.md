# S169GENLAWVER-002: Registry dispatch, AskWitness provider, seam refactor, and decision trace

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — refactors `agent_tick/execution.rs:452-491` (`append_insert_verification_candidate`); extends `RepairAttemptTrace`; adds enum-dispatched provider registry
**Deps**: archive/tickets/S169GENLAWVER-001.md

## Problem

S169GENLAWVER-001 landed the foundation types and the `provider_kind` field on `RepairAppliedPayload`, but no producer yet writes the new field and no seam code consults a provider registry. This ticket lands the **atomic refactor** that moves the existing inline AskWitness verification-candidate construction at `agent_tick/execution.rs:452-491` into a `worldwake-ai/src/verification_provider/ask_witness_provider.rs` submodule, introduces the `try_build_verification_candidate` enum-dispatched registry function, refactors the seam to delegate to the registry, and extends `RepairAttemptTrace` with per-attempt `verification_provider` and `verification_rejections` fields that the seam populates.

The registry ships with all three provider arms wired, but only `AskWitness` has a real `try_build` implementation; `ConsultRecord` and `SearchPlace` arms return `Err(VerificationRejection::BreachClassMismatch)` as placeholders. S169GENLAWVER-003 and -004 replace those placeholders. The seam already classifies all three `VerificationNeed` variants from the breach context — so once 003/004 swap the placeholders for real implementations, the routing already works.

This is the parity gate ticket: the S165 AskWitness goldens (`golden_ask_witness_refreshes_stale_report`, `golden_ask_witness_refreshes_stale_report_replay_is_deterministic`, `golden_ask_witness_cold_start_imports_local_witness_report`) must continue to pass with byte-identical authoritative event sequences modulo the new `provider_kind = AskWitness` field.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `append_insert_verification_candidate` is at `crates/worldwake-ai/src/agent_tick/execution.rs:452-491` (NOT `plan_repair.rs` despite earlier spec-citation drift — confirmed by `/reassess-spec` 2026-05-25). It calls `ask_witness_verification_step` (at `candidate_generation.rs:3280`). Both run at runtime (not `#[cfg(test)]`). The function builds an `AskWitness`-shaped `RepairPlanCandidate` and pushes it into the candidate vec.
2. `RepairAttemptTrace` at `crates/worldwake-ai/src/decision_trace.rs:199-206` currently has fields `breach: BreachSignature`, `chosen_kind: Option<RepairKind>`, `verification_anchor: Option<EntityId>` (added by S165), `rejected: Vec<(RepairKind, RepairFailure)>`, `budget_consumed: u16`, `budget_total: u16`. The new `verification_provider` and `verification_rejections` fields are added per-attempt, matching the granularity of the existing `verification_anchor` field.
3. Mixed-layer boundary under audit: the seam at `agent_tick/execution.rs:452` is the upstream candidate-construction layer; `plan_repair::attempt_repair_then_replan` is a pure composition engine that only selects from pre-built candidates. The provider registry sits in `worldwake-ai/src/verification_provider/` (created by S169GENLAWVER-001) and is invoked from the seam, not from `plan_repair`.
4. Existing S165 inline tests in `crates/worldwake-ai/src/plan_repair.rs` `#[cfg(test)]` block: `insert_verification_returns_no_epistemic_substrate_without_candidate` (line 549) and `insert_verification_returns_repaired_plan_for_supplied_candidate` (line 571). Both must continue to pass — they exercise the `plan_repair` arm directly with synthesized `RepairPlanCandidate`s and do not depend on the upstream seam's construction logic.
5. Existing S165 goldens in `crates/worldwake-ai/tests/scenarios/epistemic_sensing.rs`: `golden_ask_witness_refreshes_stale_report` (line 343), `golden_ask_witness_refreshes_stale_report_replay_is_deterministic` (line 353), `golden_ask_witness_cold_start_imports_local_witness_report` (line 372). These exercise the full seam → registry → plan_repair → event-log path. Must pass with byte-identical events modulo the new `provider_kind = AskWitness` field.
6. AI regression layer: this ticket spans candidate construction (seam) and decision trace. The seam refactor is upstream of `attempt_repair_then_replan`, so failures surface at the construction layer. Verification uses (a) inline `#[cfg(test)]` unit tests for `ask_witness_provider::try_build` parity, (b) decision-trace assertions for `verification_provider` field population, (c) S165 goldens for end-to-end parity.
7. Authoritative-to-AI Impact: this ticket modifies the candidate-construction surface that feeds `plan_repair`. Per CLAUDE.md, payload revalidation (`plan_revalidation.rs::requested_affordance_matches`) must continue to accept the registry-produced `RepairPlanCandidate` — D7 (this ticket's AskWitness portion) verifies the synthesized payload passes the existing `validate_ask_witness_payload_override` validator (`epistemic_actions.rs:155`).

## Architecture Check

1. **Enum dispatch over trait objects.** With exactly three known providers and FND-28 forbidding extensibility-for-its-own-sake, the registry is an `enum` + `match`, not a `Box<dyn VerificationCandidateProvider>` registry. No vtable, no heap allocation per provider, exhaustive-match enforcement at compile time. Each provider's `try_build` is a free function in its own submodule.
2. **Placeholder-replace pattern.** The `consult_record_provider::try_build` and `search_place_provider::try_build` stubs return `Err(VerificationRejection::BreachClassMismatch)` until S169GENLAWVER-003 and -004 land. This is compile-safe (the seam's classification logic produces real `VerificationNeed::StaleInstitutionalClaim` / `OverdueExpectationAtPlace` variants, which fall through the placeholders and collapse to `NoEpistemicSubstrate` — same behavior as pre-S169). Replaced by tickets 003 and 004.
3. **No `&World` access in the registry.** Each provider's `try_build` accepts only `&VerificationNeed` and `&VerificationContext<'_>`. Compile-time enforcement plus a locality unit test that constructs a witness outside the actor's place and asserts `VerificationRejection::NoLawfulLocalTarget`.
4. **Trace field placement.** `verification_provider` and `verification_rejections` are per-attempt fields on `RepairAttemptTrace`, matching the existing `verification_anchor` field's granularity. No new top-level `verification_provider_selection` field on `AgentDecisionTrace` (alternative considered and rejected during `/reassess-spec`).

## Verification Layers

1. AskWitness provider produces byte-identical candidates to the prior inline construction -> focused inline `#[cfg(test)]` parity test in `verification_provider/ask_witness_provider.rs` comparing a snapshot of pre-refactor candidate output against the new provider's output for matched inputs.
2. Seam delegates to registry for all three `VerificationNeed` classes -> decision-trace assertion (`AgentDecisionTrace.repair_attempts[*].verification_provider` is `Some(AskWitness)` for S165's stale-entity-belief breach; `verification_rejections` lists `ConsultRecord` and `SearchPlace` placeholder rejections).
3. Authoritative event surfaces new `provider_kind` field -> event-log delta assertion in S165 parity golden (event recorded with `provider_kind = AskWitness`, all other fields byte-identical to pre-refactor).
4. Locality enforcement -> focused unit test per provider asserting `VerificationRejection::NoLawfulLocalTarget` when target entity is at a remote place.
5. Payload revalidation -> integration test verifying the registry-produced `RepairPlanCandidate.step` passes `requested_affordance_matches` with `validate_ask_witness_payload_override` as the synthesized-payload validator.

## What to Change

### 1. Extend `VerificationContext` with seam-side fields

In `crates/worldwake-ai/src/verification_provider/mod.rs`, add the breach/seam scaffolding fields needed by `try_build`:

```rust
pub struct VerificationContext<'a> {
    pub actor: EntityId,
    pub belief_view: &'a PerAgentBeliefView<'a>,
    pub effective_place: EntityId,
    pub broken_link: &'a CausalLink,
    pub discrepancy_entry: &'a DiscrepancyEntry,
    pub action_defs: &'a ActionDefRegistry,
    pub repair_memory: &'a RepairMemory,
}
```

The `repair_memory` field lets each provider check `RepairMemory::recently_failed` to short-circuit on `RecentlyFailedAtTarget`.

### 2. Add `try_build_verification_candidate` registry function

```rust
pub fn try_build_verification_candidate(
    provider: VerificationProviderKind,
    need: &VerificationNeed,
    ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    match provider {
        VerificationProviderKind::AskWitness    => ask_witness_provider::try_build(need, ctx),
        VerificationProviderKind::ConsultRecord => consult_record_provider::try_build(need, ctx),
        VerificationProviderKind::SearchPlace   => search_place_provider::try_build(need, ctx),
    }
}

pub const PROVIDER_ITERATION_ORDER: [VerificationProviderKind; 3] = [
    VerificationProviderKind::AskWitness,
    VerificationProviderKind::ConsultRecord,
    VerificationProviderKind::SearchPlace,
];
```

### 3. Create `verification_provider/ask_witness_provider.rs` with `try_build`

Move the body of `append_insert_verification_candidate` (`agent_tick/execution.rs:452-491`) into `ask_witness_provider::try_build`. The function consumes `VerificationContext`, classifies whether the `VerificationNeed` is `StaleEntityBelief`, finds the local witness via `ctx.belief_view.entities_at(ctx.effective_place)`, builds the `AskWitness` step via the existing `ask_witness_verification_step` helper (still at `candidate_generation.rs:3280`), and returns `Ok(VerificationCandidate { provider_kind: AskWitness, target: Witness(witness), repair_candidate, source_belief })` or `Err(NoLawfulLocalTarget)`.

### 4. Create placeholder submodules

`crates/worldwake-ai/src/verification_provider/consult_record_provider.rs` and `search_place_provider.rs`, each with:

```rust
pub fn try_build(
    _need: &VerificationNeed,
    _ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    // Placeholder, replaced by ticket S169GENLAWVER-003 (ConsultRecord) /
    // S169GENLAWVER-004 (SearchPlace). The placeholder returns
    // BreachClassMismatch so the seam falls through to NoEpistemicSubstrate
    // — same behavior as pre-S169 for non-AskWitness breach classes.
    Err(VerificationRejection::BreachClassMismatch)
}
```

### 5. Refactor `agent_tick/execution.rs:452-491`

Replace the inline `append_insert_verification_candidate` body with:
(a) classify the breach into `Option<VerificationNeed>` by reading `broken_link.provider`, `broken_link.fact`, and `discrepancy_entry`,
(b) when `Some(need)`, iterate `PROVIDER_ITERATION_ORDER`, call `try_build_verification_candidate` for each, collect the first `Ok` candidate and all rejections,
(c) push the chosen `RepairPlanCandidate` into the candidates vec,
(d) record the selected provider and rejections into the in-flight `RepairAttemptTrace` builder.

### 6. Extend `RepairAttemptTrace` with new fields

In `crates/worldwake-ai/src/decision_trace.rs:199-206`:

```rust
pub struct RepairAttemptTrace {
    pub breach: worldwake_core::BreachSignature,
    pub chosen_kind: Option<RepairKind>,
    pub verification_anchor: Option<EntityId>,
    pub verification_provider: Option<VerificationProviderKind>,
    pub verification_rejections: Vec<(VerificationProviderKind, VerificationRejection)>,
    pub rejected: Vec<(RepairKind, RepairFailure)>,
    pub budget_consumed: u16,
    pub budget_total: u16,
}
```

`VerificationRejection` is re-exported from `worldwake-ai/src/verification_provider/mod.rs`.

### 7. Wire the seam to populate the trace fields

When the seam runs the registry, the chosen provider populates `verification_provider`; each rejection populates `verification_rejections`. When no provider succeeds, `verification_provider` is `None` and `verification_rejections` captures all attempted providers' failure reasons.

## Files to Touch

- `crates/worldwake-ai/src/verification_provider/mod.rs` (modify — extend `VerificationContext`, add `try_build_verification_candidate`, add `PROVIDER_ITERATION_ORDER`)
- `crates/worldwake-ai/src/verification_provider/ask_witness_provider.rs` (new — real `try_build`)
- `crates/worldwake-ai/src/verification_provider/consult_record_provider.rs` (new — placeholder)
- `crates/worldwake-ai/src/verification_provider/search_place_provider.rs` (new — placeholder)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — refactor `append_insert_verification_candidate` at lines 452-491 to delegate to registry)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — extend `RepairAttemptTrace`)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — extend trace rendering if observer surfaces `RepairAttemptTrace` fields — verify during implementation)

## Out of Scope

- Real `consult_record_provider::try_build` implementation — S169GENLAWVER-003 (placeholder remains until then).
- Real `search_place_provider::try_build` implementation — S169GENLAWVER-004 (placeholder remains until then).
- ConsultRecord / SearchPlace golden scenarios — S169GENLAWVER-003, -004.
- Negative omniscience E2E golden — S169GENLAWVER-005.
- New goal kinds (`GoalKind::ConsultRecord`, etc.) — explicitly Non-Goal'd by S169 spec (agenda companion seam follow-up).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_ask_witness_refreshes_stale_report` — S165 parity (event sequence byte-identical modulo `provider_kind = AskWitness` field).
2. `cargo test -p worldwake-ai golden_ask_witness_refreshes_stale_report_replay_is_deterministic` — S165 replay parity.
3. `cargo test -p worldwake-ai golden_ask_witness_cold_start_imports_local_witness_report` — S165 cold-start parity.
4. `cargo test -p worldwake-ai insert_verification_returns_repaired_plan_for_supplied_candidate` — S165 inline test passes with new provider field populated.
5. `cargo test -p worldwake-ai insert_verification_returns_no_epistemic_substrate_without_candidate` — S165 inline test continues to pass.
6. New focused test `ask_witness_provider_produces_candidate_for_stale_entity_belief` — provider-level happy path.
7. New focused test `ask_witness_provider_rejects_remote_witness` — locality enforcement.
8. New focused test `registry_falls_through_when_all_providers_return_breach_class_mismatch` — seam delegation correctness.
9. New decision-trace test `repair_attempt_trace_records_selected_verification_provider` — trace population correctness.
10. Existing suite: `cargo test --workspace`.

### Invariants

1. The S165 `golden_ask_witness_*` event sequences are byte-identical to pre-S169 modulo the new `provider_kind` field on `RepairApplied` events.
2. Pre-S169 non-AskWitness breach types (e.g., institutional-claim breaches if any exist in current scenarios) continue to collapse to `NoEpistemicSubstrate` — placeholders preserve current behavior.
3. The `agent_tick/execution.rs` seam no longer contains inline `ask_witness_verification_step` invocation; all verification candidate construction routes through `try_build_verification_candidate`.
4. `RepairAttemptTrace.verification_provider` is `Some(AskWitness)` for every S165 verification repair; `verification_rejections` lists exactly two entries (`ConsultRecord`, `SearchPlace`) both with `BreachClassMismatch`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/verification_provider/ask_witness_provider.rs` inline `#[cfg(test)]` — happy-path candidate construction, locality rejection, payload-validator parity. ~4 focused tests.
2. `crates/worldwake-ai/src/verification_provider/mod.rs` inline `#[cfg(test)]` — registry fall-through behavior; provider iteration order determinism. ~2 focused tests.
3. `crates/worldwake-ai/src/decision_trace.rs` — extend any existing `RepairAttemptTrace` roundtrip / construction tests to cover the two new fields.
4. `crates/worldwake-ai/tests/scenarios/plan_repair.rs:272, :355, :734` — update `RepairAppliedPayload` construction sites already touched in S169GENLAWVER-001; this ticket's behavioral changes may require additional assertions on `provider_kind` value within these scenarios.

### Commands

1. `cargo test -p worldwake-ai golden_ask_witness` — focused S165 parity gate.
2. `cargo test -p worldwake-ai verification_provider` — new module's unit tests.
3. `cargo test -p worldwake-ai plan_repair` — covers refactored seam + downstream repair behavior.
4. `cargo test --workspace` — full suite.
5. `cargo clippy --workspace --all-targets -- -D warnings` — catch any test-target lints from new modules.
6. `./scripts/verify.sh` — pre-PR gate.
