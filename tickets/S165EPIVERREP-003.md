# S165EPIVERREP-003: Seam-side verification construction and authoritative anchor

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` revalidation seam (`agent_tick/execution.rs`)
**Deps**: S165EPIVERREP-001, S165EPIVERREP-002

## Problem

This is the behavior-changing core of S165: when a plan step's causal link breaks on a
stale/contradicted/missing belief and a lawful co-located witness exists, the agent
should splice an `ask_witness` verification step rather than collapse to a typed barrier
or full replan. Because `plan_repair` cannot search, the verification step must be built
at the revalidation seam (`agent_tick/execution.rs`), where the belief view and place
context are available, and passed in as a `RepairPlanCandidate`. The chosen witness must
also be recorded in the authoritative `RepairApplied` event for FND-29A reconstructability.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The seam builds repair candidates in `repair_candidates_from_reusable_suffix`
   (`crates/worldwake-ai/src/agent_tick/execution.rs:534`) and passes them via
   `PlanRepairContext.replacement_candidates` into `attempt_repair_then_replan`
   (`execution.rs:412-424`). `substitute_target_from_repaired_plan`
   (`execution.rs:655`) derives `RepairAppliedPayload.substitute_target` from the repaired
   step at `step_index`. The breach's `broken_link: CausalLink`
   (`crates/worldwake-core/src/causal_link.rs:8`, `provider: CausalProvider`) and
   `discrepancy_entry` (`DiscrepancyClearing`, `crates/worldwake-core/src/discrepancy.rs:78`)
   are present in `PlanRepairContext`.
2. Spec deliverables D1 (breach-classification predicate), D3 (seam construction), D5
   (authoritative anchor) — `specs/S165-epistemic-verification-repair.md`.
3. Shared boundary under audit: `PlanRepairContext` / `RepairPlanCandidate` (the seam↔
   `plan_repair` data contract from S137) and the `RepairApplied` event payload
   (`RepairAppliedPayload.substitute_target`, `crates/worldwake-core/src/decision_event_payload.rs:440`)
   as the seam↔core authoritative-history contract.
4. Live `GoalKind` under test: `GoalKind::AskWitness`; the verification step is the
   `ask_witness` action toward a co-located witness — a single step (no travel), so no
   multi-step search is invoked inside the repair. Depends on ticket 001's step
   constructor and ticket 002's consuming arm.
5. Adjacent contradictions: `substitute_target_from_repaired_plan` currently has no
   `InsertVerification` arm; adding one is a required consequence of D5, not a separate
   bug. `CausalProvider::Belief { claim_key }` / `Observation` / `Record` carry the
   subject (`BeliefClaimKey.subject`, `observed_entity`, `record_entity`); `PriorStep` /
   `CarriedItem` / `Expectation` are non-epistemic and must yield no verification.

## Architecture Check

1. Building the verification step at the seam (which has belief-view + place context)
   and passing it as a candidate respects the established S137 contract that `plan_repair`
   only composes pre-built steps. The alternative — giving `plan_repair` a planner handle
   — would break that layering and duplicate search.
2. Recording the witness anchor through the **existing** `substitute_target` field reuses
   the S137 mechanism rather than adding a parallel authoritative field (FND-28), and
   keeps the provenance in append-only history rather than only a transient trace
   (FND-29A).

## Verification Layers

1. A verification `RepairPlanCandidate` is built only for a belief-backed breach
   (`Belief`/`Observation`/`Record` provider + `BeliefUpdate`/`ReobservationOf` clearing)
   with a lawful co-located witness → focused runtime test on the seam + decision trace
   (candidate presence).
2. A non-epistemic breach (`PriorStep` provider) or no lawful co-located witness builds
   no candidate → the search falls through to `DowngradeToTypedBarrier` → decision trace
   / focused test.
3. On a successful verification repair, the authoritative `RepairApplied` event carries
   `repair_kind = InsertVerification` and `substitute_target = Some(witness)` → event-log
   delta assertion.

## What to Change

### 1. Epistemic-breach classification predicate (D1)

Add a pure predicate over the breach context that inspects `broken_link.provider`
(`Belief { claim_key }` / `Observation { observed_entity, .. }` / `Record { record_entity, .. }`)
and `discrepancy_entry.clearing_condition` (`BeliefUpdate { claim_key }` /
`ReobservationOf { target }`), yielding the subject `EntityId` or `None`. Non-epistemic
providers yield `None`.

### 2. Seam-side verification-candidate construction (D3)

Where repair candidates are assembled, when the predicate yields a subject AND ticket
001's `ask_witness_verification_step(agent, witness, subject, view)` yields a step for a
lawful co-located witness, append a
`RepairPlanCandidate { kind: RepairKind::InsertVerification, step, .. }` to
`replacement_candidates`. Build no candidate otherwise.

### 3. Authoritative witness-anchor recording (D5)

Add a `RepairKind::InsertVerification` arm to `substitute_target_from_repaired_plan`
(`execution.rs:655`) so the emitted `RepairApplied(RepairAppliedPayload { repair_kind:
InsertVerification, substitute_target: Some(witness), .. })` records the witness via the
existing field. No new `RepairAppliedPayload` field; no `SAVE_FORMAT_VERSION` bump.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify)

## Out of Scope

- The belief-*subject* authoritative record for the applied-but-unexecuted case (spec D5
  deferred sub-decision; would need a new `RepairAppliedPayload` field + save bump).
- `RepairAttemptTrace` diagnostic fields (ticket 005).
- Payload revalidation of the spliced step (ticket 004).
- Place-search / `ExploreLocation` verification (spec Non-Goal).

## Acceptance Criteria

### Tests That Must Pass

1. New: belief-backed breach + lawful co-located witness → `Repaired` with an
   `ask_witness` step toward that witness.
2. New: belief-backed breach + no lawful co-located witness → no verification candidate;
   outcome is `DowngradeToTypedBarrier` (typed `InformationBarrier`).
3. New: non-epistemic breach (`PriorStep` provider) → no verification candidate.
4. New: successful verification repair emits `RepairApplied` with
   `substitute_target = Some(witness)`.
5. Existing suite: `cargo test -p worldwake-ai agent_tick::execution`.

### Invariants

1. The seam reads only the lawful belief view for the subject/witness — no authoritative
   world read for the breach subject (FND-14/FND-14A: witness must be co-located).
2. The verification step is single (no travel) and carries the `ask_witness` action's
   preconditions/duration/cost (FND-8).
3. The witness anchor is in append-only authoritative history, not only the trace
   (FND-29A).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/execution.rs` (inline `#[cfg(test)]`) — predicate,
   candidate construction (witness / no-witness / non-epistemic), and the
   `substitute_target` event assertion.

### Commands

1. `cargo test -p worldwake-ai agent_tick::execution`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `./scripts/verify.sh`
