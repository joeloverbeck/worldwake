# S169GENLAWVER-005: Negative omniscience E2E golden

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — test-only ticket
**Deps**: archive/tickets/S169GENLAWVER-002.md, archive/tickets/S169GENLAWVER-003.md, S169GENLAWVER-004

## Problem

archive/tickets/S169GENLAWVER-002.md preserved AskWitness locality through the existing `ask_witness_verification_step` and AskWitness parity lanes; archive/tickets/S169GENLAWVER-003.md and S169GENLAWVER-004 add focused provider-local remote-target checks for the new ConsultRecord and SearchPlace providers. This ticket adds the cross-provider E2E negative-omniscience golden scenario that exercises the full seam → registry → all-three-providers path with a remote breach, asserting that **no provider** emits a candidate and the repair collapses to the lawful `NoEpistemicSubstrate` outcome — same behavior as pre-S169 for breaches the substrate cannot lawfully repair.

This is the FND-14B (planner-visible inputs must be belief-backed or local) and FND-31 (validation/falsification: forbidden causal paths absent) capstone for S169. Without it, a future provider regression could silently allow remote-truth verification through one of the three providers, and the per-provider unit tests would not catch the cross-provider integration drift.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. By this ticket's prerequisites landing, all three providers have real `try_build` implementations: `ask_witness_provider` (archive/tickets/S169GENLAWVER-002.md), `consult_record_provider` (archive/tickets/S169GENLAWVER-003.md), `search_place_provider` (S169GENLAWVER-004). All three are expected to reject remote-target candidates at the locality check.
2. Test layer: this is an AI golden E2E test exercising the full agent decision cycle (seam → registry → repair → event log). Goldens of this shape live under `crates/worldwake-ai/tests/scenarios/`.
3. Mixed-layer cross-system boundary under audit: the assertion is "for a breach whose carrier (witness/record/place) is at a remote place, the registry produces no candidate." This invariant spans candidate construction (seam), provider dispatch (registry), repair outcome (plan_repair), and authoritative event emission (event log) — all four layers must agree no verification candidate exists.
4. Adjacent contradictions classified: none. This ticket is a pure proof-of-invariant addition; if it fails, it surfaces a regression in 002/003/004's locality enforcement.

## Architecture Check

1. **Cross-provider scope.** Provider-local proof in archive/tickets/S169GENLAWVER-002.md, archive/tickets/S169GENLAWVER-003.md, and S169GENLAWVER-004 verifies each provider's own locality gate at its narrow layer. This ticket's golden verifies the *integration*: that the seam classifies the breach, the registry iterates all three providers, each rejects, the seam emits no candidate, and the repair collapses to `NoEpistemicSubstrate`. Without this, independent provider regressions could conspire to admit remote-truth verification through the registry path.
2. **Falsification surface (FND-31).** The golden asserts a *negative*: no `RepairApplied` event with verification-provider semantics is produced. This is harder to assert than positive coverage and is exactly what FND-31's "negative cases that prove forbidden causal or knowledge paths are absent" calls for.
3. **No new code paths.** This ticket adds no production code. The proof is the test plus the assertions it makes about event-log content.

## Verification Layers

1. Registry iterates all three providers and all return `NoLawfulLocalTarget` for a remote breach -> decision-trace assertion (`AgentDecisionTrace.repair_attempts[*].verification_provider = None`; `verification_rejections` contains all three providers each with `NoLawfulLocalTarget`).
2. Repair collapses to `NoEpistemicSubstrate` and falls through to `DowngradeToTypedBarrier` (or `Abandon`) -> repair-outcome assertion (no `RepairApplied` event with `repair_kind = InsertVerification` for this breach signature).
3. No authoritative belief update referencing the remote carrier -> event-log delta assertion (no perception event recording belief content the agent did not lawfully observe).
4. Single-layer claim N/A — this is a deliberately multi-layer cross-system test; the layer-coverage rationale above is the point of the ticket.

## What to Change

### 1. New golden scenario `verification_no_remote_truth.rs`

In `crates/worldwake-ai/tests/scenarios/verification_no_remote_truth.rs`:

- **Setup case A — stale entity belief about a remote witness**: agent at Place A has a stale belief about an entity B; the only witness with relevant testimony is at Place C (remote). The breach classifies as `StaleEntityBelief { subject: B }`. AskWitness provider iterates `belief_view.entities_at(actor_place_A)` and finds no witness — rejects with `NoLawfulLocalTarget`. ConsultRecord and SearchPlace reject with `BreachClassMismatch`.
- **Setup case B — stale institutional claim with remote record**: agent at Place A has a stale belief about an institutional fact (e.g., `RecordTopic::OfficeRule`); the only record carrying that topic is at Place C. The breach classifies as `StaleInstitutionalClaim`. AskWitness rejects with `BreachClassMismatch`; ConsultRecord iterates `belief_view.entities_at(actor_place_A)` and finds no record with matching topic — rejects with `NoLawfulLocalTarget`. SearchPlace rejects with `BreachClassMismatch`.
- **Setup case C — overdue expectation at remote place**: agent at Place A has an overdue expectation at Place C. The breach classifies as `OverdueExpectationAtPlace { place: C }`. AskWitness and ConsultRecord reject with `BreachClassMismatch`. SearchPlace's locality check (`need.place == ctx.effective_place`) fails — rejects with `NoLawfulLocalTarget`.

For each case, assert:
- `AgentDecisionTrace.repair_attempts[*].verification_provider = None`
- `AgentDecisionTrace.repair_attempts[*].verification_rejections` has exactly 3 entries (one per provider) with the expected rejection reasons
- `RepairOutcome::Failed { tried }` where `tried` includes `(InsertVerification, NoEpistemicSubstrate)` — confirming the verification axis collapsed lawfully
- No `RepairApplied` event with `repair_kind = InsertVerification` for this breach signature
- No belief update in the event log that reflects content from the remote witness / record / place

Three separate `#[test]` functions per case, sharing common scenario setup helpers.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/verification_no_remote_truth.rs` (new)

## Out of Scope

- Any production code changes — this ticket is test-only.
- Coverage of partial-failure modes such as `PayloadValidationFailed` — these are covered by per-provider unit tests in the provider implementation tickets that add those behaviors. `RecentlyFailedAtTarget` is reserved until a target-scoped verification-memory substrate exists.
- Replay determinism assertions beyond what `golden_*_replay_is_deterministic` style tests typically include — if needed, add a sibling `_replay_is_deterministic` variant for one of the three cases.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_verification_no_remote_truth_stale_entity_belief` — case A.
2. `cargo test -p worldwake-ai golden_verification_no_remote_truth_remote_institutional_record` — case B.
3. `cargo test -p worldwake-ai golden_verification_no_remote_truth_overdue_remote_expectation` — case C.
4. `cargo test -p worldwake-ai golden_ask_witness` — S165 parity still passes.
5. `cargo test -p worldwake-ai record_breach_inserts_consult_record_verification_and_records_provider` — archive/tickets/S169GENLAWVER-003.md seam proof still passes.
6. `cargo test -p worldwake-ai golden_verification_search_place_repair` — S169GENLAWVER-004 happy path still passes if that ticket lands an external golden; otherwise use its recorded seam-proof command.
7. Existing suite: `cargo test --workspace`.

### Invariants

1. For each of the three breach classes, when the carrier (witness/record/place) is remote, no `RepairApplied` event with `repair_kind = InsertVerification` is produced.
2. `AgentDecisionTrace.repair_attempts[*].verification_rejections` always contains exactly 3 entries (one per provider) when no provider succeeds — the registry iteration is complete and deterministic.
3. No belief update in the event log reflects content the agent could not lawfully observe at their current place.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/verification_no_remote_truth.rs` — three `#[test]` functions, one per breach class, exercising the negative-omniscience cross-provider invariant.

### Commands

1. `cargo test -p worldwake-ai golden_verification_no_remote_truth` — new goldens.
2. `cargo test -p worldwake-ai golden_verification` — full S169 golden lane (consult_record, search_place, no_remote_truth).
3. `cargo test -p worldwake-ai golden_ask_witness` — S165 parity regression check.
4. `cargo test --workspace` — full suite.
5. `./scripts/verify.sh` — pre-PR gate.
