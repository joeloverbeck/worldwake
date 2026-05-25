# S169GENLAWVER-002: Registry dispatch, AskWitness provider, seam refactor, and decision trace

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — refactors `agent_tick/execution.rs:452-491` (`append_insert_verification_candidate`); extends `RepairAttemptTrace`; adds enum-dispatched provider registry
**Deps**: archive/tickets/S169GENLAWVER-001.md

## Problem

S169GENLAWVER-001 landed the foundation types and the `provider_kind` field on `RepairAppliedPayload`, but no producer yet writes the new field and no seam code consults a provider registry. This ticket lands the **atomic refactor** that moves the existing inline AskWitness verification-candidate construction at `agent_tick/execution.rs:452-491` into a `worldwake-ai/src/verification_provider/ask_witness_provider.rs` submodule, introduces the `try_build_verification_candidate` enum-dispatched registry function, refactors the seam to delegate to the registry, and extends `RepairAttemptTrace` with per-attempt `verification_provider` and `verification_rejections` fields that the seam populates.

The registry ships with all three provider arms wired, but only `AskWitness` has a real `try_build` implementation in this ticket; `ConsultRecord` and `SearchPlace` arms return `Err(VerificationRejection::BreachClassMismatch)` as placeholders. archive/tickets/S169GENLAWVER-003.md replaces the ConsultRecord placeholder, and archive/tickets/S169GENLAWVER-004.md replaces the SearchPlace placeholder. The seam already classifies all three `VerificationNeed` variants from the breach context — so once 003/004 swap the placeholders for real implementations, the routing already works.

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
2. **Placeholder-replace pattern.** The `consult_record_provider::try_build` and `search_place_provider::try_build` stubs returned `Err(VerificationRejection::BreachClassMismatch)` until archive/tickets/S169GENLAWVER-003.md and archive/tickets/S169GENLAWVER-004.md landed. This was compile-safe (the seam's classification logic produced real `VerificationNeed::StaleInstitutionalClaim` / `OverdueExpectationAtPlace` variants, which fell through the placeholders and collapsed to `NoEpistemicSubstrate` — same behavior as pre-S169). Replaced by tickets 003 and 004.
3. **No `&World` access in the registry.** Each provider's `try_build` accepts only `&VerificationNeed` and `&VerificationContext<'_>`. Compile-time enforcement plus the existing `ask_witness_verification_step_*`, `plan_repair`, and `golden_ask_witness` lanes preserve the AskWitness locality boundary.
4. **Trace field placement.** `verification_provider` and `verification_rejections` are per-attempt fields on `RepairAttemptTrace`, matching the existing `verification_anchor` field's granularity. No new top-level `verification_provider_selection` field on `AgentDecisionTrace` (alternative considered and rejected during `/reassess-spec`).

## Verified Layers

1. AskWitness provider candidate shape stayed covered by the existing `ask_witness_verification_step_*` unit tests plus the `golden_ask_witness` and `plan_repair` lanes.
2. Seam delegation to the registry was covered by `cargo test -p worldwake-ai verification_provider`, `cargo test -p worldwake-ai plan_repair`, and `cargo test -p worldwake-ai golden_ask_witness`.
3. Authoritative event `provider_kind = AskWitness` remained covered by existing `RepairAppliedPayload` assertions in the plan-repair scenario lane and by S165 AskWitness parity goldens.
4. Placeholder fallthrough for `ConsultRecord` and `SearchPlace` was covered by `verification_provider::tests::registry_routes_placeholder_providers_as_breach_class_mismatch`.
5. Observer/diagnostic rendering of `RepairAttemptTrace.verification_provider` and `verification_rejections` was covered by `cargo test -p worldwake-cli --bin observer repair`.

## Landed Changes

### 1. Extend `VerificationContext` with seam-side fields

In `crates/worldwake-ai/src/verification_provider/mod.rs`, the landed `VerificationContext` carries the seam scaffolding needed by `try_build` without exposing `&World`:

```rust
pub struct VerificationContext<'a> {
    pub actor: EntityId,
    pub belief_view: &'a dyn GoalBeliefView,
    pub effective_place: EntityId,
    pub broken_link: CausalLink,
    pub action_defs: &'a ActionDefRegistry,
}
```

`RecentlyFailedAtTarget` remains a staged rejection variant for a future explicitly specified target-scoped verification-memory substrate. archive/tickets/S169GENLAWVER-003.md and archive/tickets/S169GENLAWVER-004.md intentionally did not add provider-local target memory because live `RepairMemory` is breach/kind scoped.

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
    // Placeholder, replaced by ticket archive/tickets/S169GENLAWVER-003.md (ConsultRecord) /
    // archive/tickets/S169GENLAWVER-004.md (SearchPlace). The placeholder returns
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

## Landed Files

- `crates/worldwake-ai/src/verification_provider/mod.rs` (modify — extend `VerificationContext`, add `try_build_verification_candidate`, add `PROVIDER_ITERATION_ORDER`)
- `crates/worldwake-ai/src/verification_provider/ask_witness_provider.rs` (new — real `try_build`)
- `crates/worldwake-ai/src/verification_provider/consult_record_provider.rs` (new — placeholder)
- `crates/worldwake-ai/src/verification_provider/search_place_provider.rs` (new — placeholder)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — refactor `append_insert_verification_candidate` at lines 452-491 to delegate to registry)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — extend `RepairAttemptTrace`)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — extend trace rendering if observer surfaces `RepairAttemptTrace` fields — verify during implementation)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify — derive `Default` on `VerificationProviderKind` to keep the all-target clippy gate green)

## Out of Scope

- Real `consult_record_provider::try_build` implementation — archive/tickets/S169GENLAWVER-003.md.
- Real `search_place_provider::try_build` implementation — archive/tickets/S169GENLAWVER-004.md.
- ConsultRecord / SearchPlace provider proof surfaces — archive/tickets/S169GENLAWVER-003.md, archive/tickets/S169GENLAWVER-004.md.
- Negative omniscience E2E golden — S169GENLAWVER-005.
- New goal kinds (`GoalKind::ConsultRecord`, etc.) — explicitly Non-Goal'd by S169 spec (agenda companion seam follow-up).

## Acceptance Result

### Focused Proof

1. Passed `cargo test -p worldwake-ai golden_ask_witness`, which includes `golden_ask_witness_refreshes_stale_report`, `golden_ask_witness_refreshes_stale_report_replay_is_deterministic`, and `golden_ask_witness_cold_start_imports_local_witness_report`.
2. Passed `cargo test -p worldwake-ai plan_repair`, which includes `insert_verification_returns_repaired_plan_for_supplied_candidate` and `insert_verification_returns_no_epistemic_substrate_without_candidate`.
3. Passed `cargo test -p worldwake-ai verification_provider`, covering provider iteration order and placeholder fallthrough.
4. Passed `cargo test -p worldwake-cli --bin observer repair`, covering observer rendering of the widened repair trace.
5. Passed `cargo test -p worldwake-ai` as the affected-crate suite.
6. Workspace and all-target clippy gates remain owned by the final harness pre-push verification phase.

### Invariants

1. The S165 `golden_ask_witness_*` lane passed with `provider_kind = AskWitness` on `RepairApplied` events.
2. Pre-S169 non-AskWitness breach types (e.g., institutional-claim breaches if any exist in current scenarios) continue to collapse to `NoEpistemicSubstrate` — placeholders preserve current behavior.
3. The `agent_tick/execution.rs` seam no longer contains inline `ask_witness_verification_step` invocation; all verification candidate construction routes through `try_build_verification_candidate`.
4. `RepairAttemptTrace.verification_provider` is `Some(AskWitness)` for every S165 verification repair; `verification_rejections` lists exactly two entries (`ConsultRecord`, `SearchPlace`) both with `BreachClassMismatch`.

## Test Plan Result

### Focused Tests

1. Added `crates/worldwake-ai/src/verification_provider/mod.rs` inline tests for provider iteration order and placeholder fallthrough.
2. Extended `crates/worldwake-ai/src/decision_trace.rs` bincode roundtrip coverage for the widened `RepairAttemptTrace`.
3. Updated observer and diagnostics test constructors for the widened trace shape.
4. Reused the existing `ask_witness_verification_step_*`, `plan_repair`, and S165 `golden_ask_witness` coverage for AskWitness parity.

### Commands Run

1. `cargo test -p worldwake-ai golden_ask_witness` — focused S165 parity gate.
2. `cargo test -p worldwake-ai verification_provider` — provider module unit tests.
3. `cargo test -p worldwake-ai plan_repair` — covers refactored seam + downstream repair behavior.
4. `cargo test -p worldwake-cli --bin observer repair` — observer repair rendering.
5. `cargo test -p worldwake-ai` — affected crate suite.
6. `cargo clippy --workspace --all-targets -- -D warnings` — all-target clippy gate.
7. `cargo test --workspace` and `./scripts/verify.sh` remain deferred to the final harness pre-push phase.

## Outcome

Completed on 2026-05-25.

- Added the enum-dispatched verification provider registry in `worldwake-ai::verification_provider`, with deterministic `AskWitness`, `ConsultRecord`, `SearchPlace` iteration order.
- Moved the live AskWitness verification splice out of `agent_tick/execution.rs` and into `verification_provider/ask_witness_provider.rs`; `agent_tick/execution.rs` now classifies a breach into `VerificationNeed`, delegates to `try_build_verification_candidate`, and appends the selected `RepairPlanCandidate`.
- Added placeholder `consult_record_provider` and `search_place_provider` modules that return `VerificationRejection::BreachClassMismatch`; archive/tickets/S169GENLAWVER-003.md owns the ConsultRecord provider implementation and archive/tickets/S169GENLAWVER-004.md owns the SearchPlace provider implementation.
- Extended `RepairAttemptTrace` with `verification_provider` and `verification_rejections`, including serde/bincode roundtrip coverage, diagnostics aggregation constructor fallout, and observer rendering for the new trace fields.
- Threaded the selected provider into `RepairAppliedPayload.provider_kind` for successful `InsertVerification` repairs. Non-verification repairs keep the pre-existing inert `AskWitness` default value.
- Converted `VerificationProviderKind`'s manual `Default` impl to a derived default as same-family all-target clippy fallout.

## Deviations

- `VerificationContext.belief_view` landed as `&dyn GoalBeliefView` rather than a concrete `&PerAgentBeliefView`; this preserves the existing seam's trait boundary while still preventing provider access to `&World`.
- `VerificationContext` did not retain the drafted `repair_memory` field because the only real provider in this ticket is AskWitness parity; target-scoped provider recent-failure checks need their own explicit substrate before any later provider ticket implements them.
- The focused registry tests prove deterministic order and placeholder fallthrough. AskWitness payload shape and validator parity remain covered by the existing `ask_witness_verification_step_*`, `plan_repair`, and `golden_ask_witness` lanes instead of duplicating the same fixture inside the provider module.
- The full pre-PR `./scripts/verify.sh` gate is deferred to the final `implement-spec-tickets` branch/push phase; this ticket ran the affected crate, parity lanes, observer lane, and all-target clippy gate directly.

## Verification Result

- Passed `cargo test -p worldwake-ai verification_provider`.
- Passed `cargo test -p worldwake-ai plan_repair`.
- Passed `cargo test -p worldwake-ai golden_ask_witness`.
- Passed `cargo test -p worldwake-cli --bin observer repair`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Waived `cargo test --workspace` and `./scripts/verify.sh` for this per-ticket closeout because the harness final branch phase owns the full pre-push gate after all S169 tickets land.
