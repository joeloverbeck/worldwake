# S165EPIVERREP-004: Payload revalidation for the spliced verification step

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` plan revalidation test coverage; no production fix required
**Deps**: archive/tickets/S165EPIVERREP-003.md

## Problem

The spliced verification step uses S139's planner-synthesized `AskWitnessPayload`. Per
the Authoritative-to-AI Impact Rule (`AGENTS.md`, item 6), an action with a synthesized
(not affordance-derived) payload must have its `with_payload_override_validator`
registration accept the payload at the revalidation seam, or the step silently fails
revalidation. This ticket confirmed `validate_ask_witness_payload_override` accepts the
spliced step and added focused coverage; no rejection fix was required.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The `ask_witness` action registers `.with_payload_override_validator(validate_ask_witness_payload_override)`
   (`crates/worldwake-systems/src/epistemic_actions.rs:27`; validator fn at
   `epistemic_actions.rs:155`). `plan_revalidation.rs` calls `requested_affordance_matches`,
   which delegates to the handler's override validator for untargeted synthesized payloads.
2. Spec deliverable D6 (`archive/specs/S165-epistemic-verification-repair.md`).
3. Shared boundary under audit: the payload-revalidation contract between the spliced
   `RepairPlanCandidate.step` (ticket 003) and `validate_ask_witness_payload_override`.
   The spliced payload is the same `AskWitnessPayload` shape the organic planner produces,
   so the validator is expected to accept it; this ticket proves that rather than
   assuming it (`AGENTS.md` payload-revalidation note).
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

## Verified Layers

1. The spliced verification step passes payload revalidation at the seam → focused
   runtime request-resolution test (`plan_revalidation`) added here.
2. Single-boundary ticket — authoritative start/abort is not the contract here; if the
   step is later rejected at authoritative start, that is a separate (out-of-scope)
   boundary named for follow-up.

## Landed Changes

### 1. Confirmed revalidation acceptance

Added a focused test that runs the revalidation path
(`requested_affordance_matches` / `plan_revalidation.rs`) over a spliced verification step
and asserts acceptance. `validate_ask_witness_payload_override` accepted the synthesized
payload through the live `ask_witness` action registration, so no production validator or
revalidation code change was required.

## Landed Files

- `crates/worldwake-ai/src/plan_revalidation.rs` (modified — focused test fixture plus
  `spliced_ask_witness_payload_revalidates_with_override_validator`)
- `crates/worldwake-systems/src/epistemic_actions.rs` (checked through the live
  registered `ask_witness` action; no code change)

## Out of Scope

- Authoritative start/abort behavior of the `ask_witness` action (unchanged; existing
  handler).
- The construction of the spliced step (ticket 003).

## Acceptance Criteria

### Acceptance Result

1. Passed: a spliced `InsertVerification` verification step passes payload revalidation.
2. Passed: existing `plan_revalidation` tests remained green.

### Invariants

1. The spliced step uses the same `AskWitnessPayload` shape as the organic planner — one
   payload contract, one validator (FND-28).
2. Revalidation reads only lawful belief-view state (FND-14B).

## Test Plan Result

### Focused Tests

1. `crates/worldwake-ai/src/plan_revalidation.rs` (inline `#[cfg(test)]`) — spliced-step
   revalidation acceptance via `spliced_ask_witness_payload_revalidates_with_override_validator`.

### Commands Run

1. `cargo test -p worldwake-ai plan_revalidation`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo fmt --all`

## Outcome

Completed on 2026-05-24.

- Added a focused `plan_revalidation` unit test that constructs the same targeted
  `AskWitnessPayload` shape used by the spliced verification repair step and sends it
  through `revalidate_next_step`.
- Registered the real `ask_witness` action in the test, so the assertion exercises
  `validate_ask_witness_payload_override` through the live handler path.
- Confirmed the live validator accepts the synthesized payload; no production
  `plan_revalidation.rs` or `epistemic_actions.rs` fix was needed.

## Deviations

- The ticket landed as audit-confirmed regression coverage only. The expected validator
  rejection did not occur.
- Replaced stale `CLAUDE.md` references with the current `AGENTS.md` authority.

## Verification Result

- Passed `cargo test -p worldwake-ai plan_revalidation`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `cargo fmt --all`
