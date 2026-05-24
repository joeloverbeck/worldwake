# S165EPIVERREP-003: Seam-side verification construction and authoritative anchor

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` revalidation seam (`agent_tick/execution.rs`)
**Deps**: archive/tickets/S165EPIVERREP-001.md, archive/tickets/S165EPIVERREP-002.md

## Problem

This was the behavior-changing core of S165: before this ticket, when a plan step's
causal link broke on a stale/contradicted/missing belief and a lawful co-located witness
existed, the agent could not splice an `ask_witness` verification step and instead fell
through toward a typed barrier or full replan. Because `plan_repair` cannot search, the
verification step had to be built at the revalidation seam (`agent_tick/execution.rs`),
where the belief view and place context are available, and passed in as a
`RepairPlanCandidate`. The chosen witness also needed to be recorded in the authoritative
`RepairApplied` event for FND-29A reconstructability.

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

## Verified Layers

1. Belief-backed `Belief`/`Observation` providers paired with
   `BeliefUpdate`/`ReobservationOf` clearing yield a verification subject; non-epistemic
   or mismatched breaches yield none → focused execution unit tests.
2. A verification `RepairPlanCandidate` is appended only when the seam has an epistemic
   subject, the actor has a co-located witness, the `ask_witness` action definition
   exists, and the ticket-001 `ask_witness_verification_step` constructor accepts that
   witness/subject pair → source-level seam review plus the ticket-001 constructor tests.
3. On a successful verification repair, the authoritative `RepairApplied` event carries
   `repair_kind = InsertVerification` and `substitute_target = Some(witness)` → focused
   event-log delta assertion.

## Landed Changes

### 1. Added epistemic-breach classification (D1)

Added a pure predicate over the breach context that inspects `broken_link.provider`
(`Belief { claim_key }` / `Observation { observed_entity, .. }` / `Record { record_entity, .. }`)
and `discrepancy_entry.clearing_condition` (`BeliefUpdate { claim_key }` /
`ReobservationOf { target }`). It yields the subject `EntityId` only when the provider
subject and clearing subject match. Non-epistemic providers and mismatched subjects yield
`None`.

### 2. Added seam-side verification-candidate construction (D3)

Extended `attempt_local_repair_for_invalidated_step` so the live revalidation seam passes
the agent's belief view, action definitions, and actor id into repair-candidate assembly.
When the predicate yields a subject and a co-located witness passes ticket 001's
`ask_witness_verification_step(...)` constructor, the seam appends
`RepairPlanCandidate { kind: RepairKind::InsertVerification, provider, fact, step, .. }`
to `replacement_candidates`. No candidate is built when the breach is non-epistemic, the
subject/clearing mismatch, the actor has no place, no `ask_witness` action definition is
registered, or no co-located witness is lawful.

### 3. Added authoritative witness-anchor recording (D5)

Updated `substitute_target_from_repaired_plan` so `RepairKind::InsertVerification` uses
the repaired step's primary target exactly like `RebindTarget`. The emitted
`RepairApplied(RepairAppliedPayload { repair_kind: InsertVerification, substitute_target:
Some(witness), .. })` records the witness via the existing field. No new
`RepairAppliedPayload` field and no `SAVE_FORMAT_VERSION` bump were needed.

## Landed Files

- `crates/worldwake-ai/src/agent_tick/execution.rs` (modified)

## Out of Scope

- The belief-*subject* authoritative record for the applied-but-unexecuted case (spec D5
  deferred sub-decision; would need a new `RepairAppliedPayload` field + save bump).
- `RepairAttemptTrace` diagnostic fields (ticket 005).
- Payload revalidation of the spliced step (ticket 004).
- Place-search / `ExploreLocation` verification (spec Non-Goal).

## Acceptance Criteria

### Test Result

1. Added/passed: belief-backed breach with matching clearing condition yields a
   verification subject for `Belief` and `Observation` providers.
2. Added/passed: non-epistemic breach (`PriorStep`) and mismatched clearing subject yield
   no verification subject, so the seam builds no verification candidate from them.
3. Added/passed: successful verification repair emits `RepairApplied` with
   `substitute_target = Some(witness)`.
4. Existing/passed: `cargo test -p worldwake-ai agent_tick::execution -- --nocapture`.
5. Existing/passed: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
6. Existing/passed: `./scripts/verify.sh`.

### Invariants

1. The seam reads only the lawful belief view for the subject/witness — no authoritative
   world read for the breach subject (FND-14/FND-14A: witness must be co-located).
2. The verification step is single (no travel) and carries the `ask_witness` action's
   preconditions/duration/cost (FND-8).
3. The witness anchor is in append-only authoritative history, not only the trace
   (FND-29A).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/execution.rs` (inline `#[cfg(test)]`) — predicate,
   non-epistemic/mismatched rejection, and the `substitute_target` event assertion.
2. Ticket 001's existing `candidate_generation.rs` constructor tests remain the focused
   proof for the witness/subject lawfulness gate consumed by this seam.

### Commands Run

1. Passed `cargo test -p worldwake-ai agent_tick::execution -- --nocapture`
2. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. Passed `./scripts/verify.sh`

## Outcome

Completed on 2026-05-24.

- Added the seam-side `InsertVerification` candidate construction path for epistemic
  breaches using the ticket-001 `ask_witness_verification_step` constructor.
- Kept verification lawful and local: the seam only considers co-located witnesses from
  the actor's belief view and does not read authoritative remote subject truth.
- Recorded the chosen witness in the append-only `RepairApplied` event through the
  existing `substitute_target` field.

## Deviations

- Focused execution tests prove the new classification predicate and authoritative event
  anchor directly. The co-located witness lawfulness gate itself remains proved by the
  ticket-001 `candidate_generation` constructor tests; this ticket reuses that helper
  rather than duplicating a second full belief-view fixture in `execution.rs`.

## Verification Result

- Passed `cargo test -p worldwake-ai agent_tick::execution -- --nocapture`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
