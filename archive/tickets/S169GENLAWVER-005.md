# S169GENLAWVER-005: Negative omniscience seam proof

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No production engine changes — in-crate seam tests only
**Deps**: archive/tickets/S169GENLAWVER-002.md, archive/tickets/S169GENLAWVER-003.md, archive/tickets/S169GENLAWVER-004.md

## Problem

archive/tickets/S169GENLAWVER-002.md preserved AskWitness locality through the existing `ask_witness_verification_step` and AskWitness parity lanes; archive/tickets/S169GENLAWVER-003.md and archive/tickets/S169GENLAWVER-004.md added focused provider-local checks for the ConsultRecord and SearchPlace providers. This ticket adds the cross-provider negative-omniscience capstone that exercises the private seam -> registry -> all-three-providers -> repair fallback path with remote breaches.

This is the FND-14B and FND-31 proof for S169: planner-visible verification candidates are backed by a lawful local carrier, and forbidden remote-truth paths are absent. The approved proof boundary is the private in-crate repair seam, because the provider-selection seam is not public API and is not widened solely to satisfy an external golden file shape.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. All three providers have real `try_build` implementations: `AskWitness` (archive/tickets/S169GENLAWVER-002.md), `ConsultRecord` (archive/tickets/S169GENLAWVER-003.md), and `SearchPlace` (archive/tickets/S169GENLAWVER-004.md).
2. Test layer corrected: the strongest available proof surface is an in-crate seam regression in `agent_tick::execution`, not an external `crates/worldwake-ai/tests/scenarios/verification_no_remote_truth.rs` golden. The external golden would either need test-only API widening or would assert a weaker downstream absence.
3. Cross-provider boundary under audit: a breach whose lawful carrier is remote must produce no verification candidate. The test proves seam classification, deterministic registry iteration, per-provider rejection reasons, `InsertVerification` collapse to `NoEpistemicSubstrate`, and fallback `RepairApplied` events that are not verification repairs.
4. Provider rejection precision corrected: only the matching provider returns `NoLawfulLocalTarget` for a remote carrier. Non-matching providers return `BreachClassMismatch`.

## Architecture Check

1. **FND-14B locality.** The actor's remote witness, remote record, or remote place cannot become a planner-visible verification candidate simply because it exists in authoritative world state.
2. **FND-31 falsification.** The tests assert the forbidden path is absent: no provider is selected, no `InsertVerification` repair is applied, and the repair axis records `NoEpistemicSubstrate`.
3. **No public seam widening.** The provider-selection seam remains private. The test lives beside the private seam it validates instead of adding a production or test-only API export.

## Verification Layers Landed

1. Remote witness / stale entity belief: `AskWitness` rejects with `NoLawfulLocalTarget`; `ConsultRecord` and `SearchPlace` reject with `BreachClassMismatch`.
2. Remote record / stale institutional claim: `ConsultRecord` rejects with `NoLawfulLocalTarget`; `AskWitness` and `SearchPlace` reject with `BreachClassMismatch`.
3. Remote place / overdue expectation: `SearchPlace` rejects with `NoLawfulLocalTarget`; `AskWitness` and `ConsultRecord` reject with `BreachClassMismatch`.
4. Each case records `verification_provider = None`, exactly three provider rejections, and rejected repair kind `(InsertVerification, NoEpistemicSubstrate)`.
5. Each case emits a fallback `RepairApplied` event whose `repair_kind` is not `InsertVerification`.

## What Changed

- Added an all-provider action-definition fixture for verification seam tests.
- Added a remote-record fixture for stale institutional claim rejection.
- Added three `agent_tick::execution` tests:
  - `remote_witness_breach_records_all_provider_rejections_without_insert_verification`
  - `remote_record_breach_records_all_provider_rejections_without_insert_verification`
  - `remote_expectation_breach_records_all_provider_rejections_without_insert_verification`

## Files Touched

- `crates/worldwake-ai/src/agent_tick/execution.rs`

## Deviations From Original Plan

- Did not add `crates/worldwake-ai/tests/scenarios/verification_no_remote_truth.rs`. The provider-selection seam is private, and the approved option was to prove the invariant at that private seam rather than widen production/test APIs for an external golden.
- Did not assert a separate event-log belief update absence. At the seam under proof, no verification action is selected or started; the stronger assertion is that `InsertVerification` is rejected with `NoEpistemicSubstrate` and the emitted fallback repair is not a verification repair.
- No production code changed.

## Outcome

Completed: 2026-05-25

S169 now has a negative omniscience capstone over the generalized lawful verification substrate. The test coverage proves that remote witness, remote record, and remote-place expectation breaches iterate all three providers, select no verification provider, collapse the verification axis to `NoEpistemicSubstrate`, and emit only non-verification fallback repair events.

## Verification Result

Verification results:

1. Passed — `cargo test -p worldwake-ai remote_witness_breach_records_all_provider_rejections_without_insert_verification`.
2. Passed — `cargo test -p worldwake-ai remote_record_breach_records_all_provider_rejections_without_insert_verification`.
3. Passed — `cargo test -p worldwake-ai remote_expectation_breach_records_all_provider_rejections_without_insert_verification`.
4. Passed — `cargo test -p worldwake-ai remote_`.
5. Passed — `cargo test -p worldwake-ai golden_ask_witness`.
6. Passed — `cargo test -p worldwake-ai`.
7. Passed — `cargo clippy --workspace --all-targets -- -D warnings`.
