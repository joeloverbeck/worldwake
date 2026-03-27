# CRIMECASEARCH-001: Introduce First-Class Institutional Crime Case Artifacts

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core`, `worldwake-systems`, `worldwake-ai`
**Deps**: E17 (crime/theft/justice), E16c (institutional beliefs), `archive/tickets/completed/S32CRIMEMEGOLSUI-003.md`, `docs/FOUNDATIONS.md`

## Problem

The current architecture still conflates two different things:

- agent-local evidence bookkeeping (`ViolationId` inside `ViolationMemory`)
- institutional case identity (accusations and verdicts inside `CrimeRegister`)

That violates the spirit of `docs/FOUNDATIONS.md` Principles 4, 16, 23, and 24. A local evidence record is not a durable social artifact. A crime case is. Even after the S32 duplicate fix, institutional claims, belief keys, and topic grouping still treat a local `ViolationId` as if it were the same thing as a first-class case. The recommended change is to model the institutional case itself as world state with stable identity.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is the identity handoff between local evidence and institutional justice:
   `crates/worldwake-systems/src/justice_actions.rs::{start_accuse, commit_accuse, accuse_claim_from_entry}`
   and
   `crates/worldwake-core/src/institutional.rs::{InstitutionalClaim, InstitutionalBeliefKey, RecordData}`.
2. Live code still uses `ViolationId` inside institutional claims:
   `InstitutionalClaim::Accusation { accused, violation_id, theft, .. }`
   and
   `InstitutionalClaim::Verdict { accused, violation_id, .. }`
   in `crates/worldwake-core/src/institutional.rs`.
3. Live institutional knowledge grouping also still uses `(accused, violation_id)`:
   `InstitutionalBeliefKey::CrimeCase` in `crates/worldwake-core/src/institutional.rs`,
   `InstitutionalTellTopicKey::CrimeCase` in `crates/worldwake-core/src/belief.rs`,
   plus relay/grouping helpers in
   `crates/worldwake-systems/src/{tell_actions.rs,consult_record_actions.rs}`.
4. `ViolationId` is not a suitable long-term institutional key. It is allocated inside `ViolationMemory` in `crates/worldwake-core/src/violation.rs` and is intentionally local, ephemeral evidence bookkeeping governed by expiry and per-agent discovery lanes.
5. The current production fix in S32 makes duplicate filing robust at the `CrimeRegister` boundary by comparing `TheftFacts`, but that still leaves the deeper architectural mismatch in place: the world has a durable case, while the type system still presents a local evidence id as its nominal identity.
6. The cleanest first-class artifact under the live architecture is not “a new scalar alias for violation id.” It is a concrete world object or component-backed entity with stable identity and explicit relation to:
   register / issuer / accused / theft facts / case status.
   That aligns with Principles 4, 16, and 23 better than replacing one id with another hidden id.
7. Information-path analysis: today the same fact has two lawful transport paths.
   Path A: local evidence in `ViolationMemory` drives `GoalKind::Accuse`.
   Path B: institutional record consultation / tell traffic carries `InstitutionalClaim::Accusation` and `InstitutionalClaim::Verdict`.
   After this ticket, the canonical institutional path should be the first-class crime-case artifact plus record entries referencing it. Local `ViolationId` remains only as evidence-binding input for the accuser’s subjective proof boundary.
8. The intended invariant is not “all evidence records collapse into one violation id.” It is:
   many evidence records may support one institutional case, but one institutional case has one stable world identity.
9. This is a mixed-layer ticket. The exact data contract to make explicit is:
   local evidence record -> accuse action binds evidence -> accuse action opens or reuses institutional case artifact -> crime-register entries reference that artifact -> later verdicts supersede within that artifact’s lane.
10. Adjacent contradiction exposed during reassessment: `GoalKind::Accuse` and action payloads will likely continue to bind local evidence via `ViolationId` even after institutional claims stop using it. That is not a contradiction; it is the correct layer split. The contradiction is only when institutional state itself is keyed by that local id.
11. This ticket should not preserve any backwards-compatibility alias such as “case_id but still mirror violation_id everywhere.” The end state should remove `violation_id` from institutional case identity surfaces in-scope.

## Architecture Check

1. A first-class crime-case artifact is cleaner than continuing to overload `ViolationId` because it makes the durable social artifact explicit in world state, which directly matches Principles 16 and 23.
2. This is more robust than a scalar-id substitution. A new opaque `CrimeCaseId` without a world artifact would still leave the case hidden behind bookkeeping. A concrete case entity or component-backed artifact is composable: later evidence attachment, reopening, appeal, jurisdiction transfer, and multi-step procedure can all hang off the same stable thing.
3. This is more extensible than keeping case identity implicit in `CrimeRegister` entry matching. The register remains the append-only ledger, while the case artifact becomes the stable reference target across accusations, verdicts, consultation, and future systems.
4. No backwards-compatibility aliasing or shims should be introduced.

## Verification Layers

1. Institutional case identity is stable and explicit in world state -> focused `worldwake-core` component / serialization tests
2. Accuse opens or reuses one institutional case artifact for same accused + theft facts -> focused `worldwake-systems` justice-action tests
3. CrimeRegister entries reference the case artifact instead of raw `violation_id` identity -> focused `worldwake-core` + `worldwake-systems` tests
4. AI accusation generation still surfaces lawful accusation candidates from local evidence -> focused `worldwake-ai` candidate-generation tests
5. Existing convergence golden still passes with case-artifact identity -> golden Scenario 43 plus existing Scenarios 38 and 41
6. Determinism and append-only register behavior remain intact -> workspace tests plus targeted record-history assertions

## What to Change

### 1. Add a first-class crime-case artifact to authoritative world state

Introduce a concrete world artifact for institutional crime cases. The artifact should have stable identity and explicit authoritative data such as:

- issuing office or register
- accused
- concrete theft facts
- opened tick
- current procedural status needed by justice logic

Prefer an entity-backed component over a hidden scalar alias so the case itself is world state.

### 2. Rekey institutional claims to the case artifact

Update `InstitutionalClaim::Accusation` and `InstitutionalClaim::Verdict` so their canonical case reference is the first-class case artifact, not `ViolationId`.

`ViolationId` may remain in the accuse action payload and other evidence-binding surfaces where the actor still needs to point at a local subjective evidence record, but institutional state should no longer use it as case identity.

### 3. Update record append / supersede behavior

Update justice actions and record helpers so:

- first lawful accusation opens or binds a case artifact
- duplicate accusation for the same concrete case reuses that artifact instead of creating a parallel institutional lane
- verdict supersession operates within the case artifact’s lane

### 4. Update focused tests and existing goldens

Strengthen focused coverage around accusation opening, duplicate reuse, verdict supersession, and record serialization; then update goldens that currently inspect `violation_id` inside institutional claims.

## Files to Touch

- `crates/worldwake-core/src/institutional.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-systems/src/justice_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `tickets/CRIMECASEARCH-001.md` (new)

## Out of Scope

- Broad redesign of non-crime institutional artifacts such as support or force-control records
- Appeal, reopening, confiscation, prison, or multi-office judicial workflows
- New UI/CLI inspection features for cases
- Partial compatibility layers that keep both case-artifact identity and old `violation_id` institutional identity alive in parallel

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove one institutional case artifact is reused for the same accused + theft facts
2. Focused tests prove accusations and verdicts no longer use institutional `violation_id` identity
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Institutional crime cases are first-class world artifacts with stable identity.
2. `ViolationId` remains a local evidence-binding tool, not the canonical institutional case identity.
3. `CrimeRegister` remains append-only ledger state; the new case artifact does not replace the ledger.
4. Duplicate accusation suppression and verdict supersession operate on the stable institutional case, not on parallel evidence lanes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/institutional.rs` — add case-artifact serialization and record-linking tests
2. `crates/worldwake-systems/src/justice_actions.rs` — add accuse open-or-reuse and verdict supersession tests keyed by case artifact
3. `crates/worldwake-ai/src/candidate_generation.rs` — update accusation-generation tests to assert candidate legality against case-artifact-backed institutional state
4. `crates/worldwake-ai/tests/golden_emergent.rs` — update Scenarios 38, 41, and 43 to assert case-artifact-backed institutional identity

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
