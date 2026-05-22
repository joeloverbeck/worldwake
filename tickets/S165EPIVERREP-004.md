# S165EPIVERREP-004: Payload revalidation for the spliced verification step

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` plan revalidation (`plan_revalidation.rs`), if a fix is required
**Deps**: S165EPIVERREP-003

## Problem

The spliced verification step uses S139's planner-synthesized `AskWitnessPayload`. Per
the Authoritative-to-AI Impact Rule (CLAUDE.md, item 6), an action with a synthesized
(not affordance-derived) payload must have its `with_payload_override_validator`
registration accept the payload at the revalidation seam, or the step silently fails
revalidation. This ticket confirms `validate_ask_witness_payload_override` accepts the
spliced step and adds focused coverage; if revalidation rejects it, the fix lands here.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The `ask_witness` action registers `.with_payload_override_validator(validate_ask_witness_payload_override)`
   (`crates/worldwake-systems/src/epistemic_actions.rs:27`; validator fn at
   `epistemic_actions.rs:155`). `plan_revalidation.rs` calls `requested_affordance_matches`,
   which delegates to the handler's override validator for untargeted synthesized payloads.
2. Spec deliverable D6 (`specs/S165-epistemic-verification-repair.md`).
3. Shared boundary under audit: the payload-revalidation contract between the spliced
   `RepairPlanCandidate.step` (ticket 003) and `validate_ask_witness_payload_override`.
   The spliced payload is the same `AskWitnessPayload` shape the organic planner produces,
   so the validator is expected to accept it; this ticket proves that rather than
   assuming it (CLAUDE.md payload-revalidation note).
4. Stale-request / start-failure boundary (precision rule 9): the first failure boundary
   under test is **request resolution / affordance reproduction at revalidation**, not
   authoritative start — `requested_affordance_matches` in `plan_revalidation.rs` is the
   shared symbol checked. If revalidation already accepts, this is audit-only with a test;
   if it rejects, the validator or its call path is corrected in-scope.

## Architecture Check

1. Proving the spliced step survives revalidation at the earliest boundary (request
   resolution) rather than discovering a silent revalidation failure downstream keeps the
   verification-repair contract honest and debuggable.
2. No new validator: the existing `validate_ask_witness_payload_override` is reused; a fix,
   if needed, adjusts the existing path rather than adding a parallel one (FND-28).

## Verification Layers

1. The spliced verification step passes payload revalidation at the seam → focused
   runtime request-resolution test (`plan_revalidation`).
2. Single-boundary ticket — authoritative start/abort is not the contract here; if the
   step is later rejected at authoritative start, that is a separate (out-of-scope)
   boundary named for follow-up.

## What to Change

### 1. Confirm (and, if needed, fix) revalidation acceptance

Add a focused test that runs the revalidation path
(`requested_affordance_matches` / `plan_revalidation.rs`) over a spliced verification step
and asserts acceptance. If `validate_ask_witness_payload_override` rejects the synthesized
payload, correct the validator or the revalidation call path so the lawful spliced step is
accepted; document the correction here. If acceptance already holds, record this ticket as
audit-confirmed with the test as the regression guard.

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — test; code change only if a
  rejection is found)
- Likely: `crates/worldwake-systems/src/epistemic_actions.rs` (modify — only if the
  validator must be corrected; confirm via the revalidation test before editing)

## Out of Scope

- Authoritative start/abort behavior of the `ask_witness` action (unchanged; existing
  handler).
- The construction of the spliced step (ticket 003).

## Acceptance Criteria

### Tests That Must Pass

1. New: a spliced `InsertVerification` verification step passes payload revalidation.
2. Existing suite: `cargo test -p worldwake-ai plan_revalidation`.

### Invariants

1. The spliced step uses the same `AskWitnessPayload` shape as the organic planner — one
   payload contract, one validator (FND-28).
2. Revalidation reads only lawful belief-view state (FND-14B).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_revalidation.rs` (inline `#[cfg(test)]`) — spliced-step
   revalidation acceptance.

### Commands

1. `cargo test -p worldwake-ai plan_revalidation`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `./scripts/verify.sh`
