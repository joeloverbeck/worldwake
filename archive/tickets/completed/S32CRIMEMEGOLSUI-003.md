# S32CRIMEMEGOLSUI-003: Scenario 43 — Dual Discovery Converges Without Double Accusation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — accusation duplicate suppression in AI + justice case validation, plus golden coverage
**Deps**: E17 (crime/theft/justice), E16c (institutional beliefs), E14 (perception), E15 (social Tell), S27 (expectation-violation goals), `specs/S32-crime-emergence-golden-suites.md`, `docs/golden-e2e-testing.md`

## Problem

Scenario 43 was drafted as a missing golden-only proof that witness discovery and owner-local discovery converge on one institutional accusation. Reassessment shows the live architecture is weaker than the ticket assumed: the two paths can produce distinct authority-side `ViolationId`s for the same concrete `TheftFacts`, while accusation duplicate suppression is still keyed to `(accused, violation_id)`. That means dual discovery is not only untested; it exposes a real case-identity contradiction. The ticket must cover the architectural fix and then prove it end to end.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is crime-case identity at the accusation boundary:
   `crates/worldwake-ai/src/candidate_generation.rs::emit_accusation_candidates()`
   decides whether to surface `GoalKind::Accuse`, and
   `crates/worldwake-systems/src/justice_actions.rs::{start_accuse, commit_accuse}`
   decide whether authoritative filing is still lawful.
2. The draft ticket’s core assumption is stale. `emit_accusation_candidates()` does not currently deduplicate by concrete theft case. It suppresses only when an existing institutional claim matches the same `(accused, violation_id)` pair.
3. That identity is too weak for Scenario 43. The live owner-local path records `ViolationKind::SuspectedTheft { theft, suspect: None }` in `crates/worldwake-systems/src/investigate_actions.rs`, while the witness-report path can record `ViolationKind::SuspectedTheft { theft, suspect: Some(thief) }` in `crates/worldwake-systems/src/tell_actions.rs`. Because `ViolationMemory::record()` deduplicates by exact `ViolationKind`, the authority can lawfully hold two unresolved theft violations for the same `TheftFacts`.
4. The live accusation record already stores the stronger concrete identity the draft ignored: `InstitutionalClaim::Accusation { accused, violation_id, theft, .. }` in `crates/worldwake-core/src/institutional.rs`. That means the clean fix is to deduplicate accusation filing against the concrete recorded theft facts already present in the canonical institutional record, not to add an alias layer or scripted scenario workaround.
5. The authoritative duplicate check in `crates/worldwake-systems/src/justice_actions.rs::crime_case_already_recorded()` is also currently keyed to `(accused, violation_id)`. So even if golden coverage were added alone, Scenario 43 could still lawfully produce two accusations for the same theft through two different violation lanes.
6. Existing focused coverage is narrower than the original ticket claimed. Current tests only prove duplicate suppression for the same `ViolationId`:
   `crates/worldwake-ai/src/candidate_generation.rs::justice_candidates_suppress_duplicate_accusation_when_case_already_known`
   and
   `crates/worldwake-systems/src/justice_actions.rs::duplicate_unresolved_accusation_rejects_at_start`.
   They do not cover “same theft facts, different `ViolationId`”.
7. Existing golden coverage remains partial, not absent. Scenario 37 proves the owner-local discovery path in isolation, and Scenario 38 proves the witness-to-accusation path in isolation. Scenario 43 is still missing from `cargo test -p worldwake-ai --test golden_emergent -- --list`.
8. The live `GoalKind` under test remains `GoalKind::Accuse { crime_register, accused, violation_id }`, but the canonical case identity after this ticket should be “same accused + same concrete theft facts already recorded in the crime register,” not “same accused + same local evidence record id.”
9. Scenario isolation must be corrected. This ticket should isolate accusation convergence, not punishment. The authority setup should intentionally avoid a lawful punishment branch after the first accusation so the duplicate-accusation proof is not blurred by verdict supersession or punishment-specific behavior.
10. The original “both paths travel physically to the Magistrate” story is stronger than current nearby goldens require. The required invariant for this ticket is dual discovery plus single institutional accusation. If the cleanest scenario uses minimal lawful setup moves around the Tell step, the ticket should describe that honestly instead of preserving a stale narrative.
11. Architecture note beyond immediate scope: institutional belief keying still uses `InstitutionalBeliefKey::CrimeCase { accused, violation_id }`. This ticket does not need to redesign all institutional memory lanes, but it must stop record-level duplicate suppression from depending on that weaker identity.
12. Mismatch + correction: `Engine Changes: None` was wrong. The live contradiction is in production candidate generation and authoritative accuse validation, so the ticket scope is corrected to include those engine changes plus focused regression tests and the new golden.

## Architecture Check

1. The cleaner architecture is to treat the append-only `CrimeRegister` as the canonical case boundary and deduplicate accusations against the concrete theft facts already stored there. That keeps institutional identity grounded in world state and avoids adding a second parallel case-id system.
2. Reusing `ViolationId` as the sole institutional case identity across independent discovery paths is not robust, because `ViolationId` is local evidence-memory bookkeeping, not a durable social artifact. Scenario 43 exposes that mismatch directly.
3. The fix should stay narrow: no backwards-compatibility aliases, no special “dual discovery” branch, and no golden-only workaround that papers over a production contradiction.
4. No backwards-compatibility aliasing or shims should be introduced.

## Verification Layers

1. AI duplicate suppression for same theft facts across different `ViolationId`s -> focused candidate-generation test
2. Authoritative accuse start rejects duplicate filing for same theft facts across different `ViolationId`s -> focused justice-action test
3. Witness path still produces first accusation lawfully -> action trace + authoritative `CrimeRegister` entry in the golden
4. Owner-local path still develops independent theft evidence and reaches the authority -> authoritative belief/violation-memory reads in the golden
5. Second path does not create a second accusation after the first case is recorded -> authoritative `RecordData` count + decision-trace absence of the second `GoalKind::Accuse`
6. Determinism of the full convergence scenario -> replay companion `(StateHash, StateHash)` equality

## What to Change

### 1. Strengthen duplicate-case identity at the accusation boundary

Update AI candidate generation and authoritative accuse validation so duplicate suppression checks whether the same accused already has a recorded accusation case for the same concrete `TheftFacts` in the relevant `CrimeRegister`, even when the incoming evidence sits under a different `ViolationId`.

This should use the existing `InstitutionalClaim::Accusation { theft, .. }` data already persisted in the register instead of inventing a new alias type or a special-case Scenario 43 exception.

### 2. Add focused regression coverage

Add focused tests that fail under the old `(accused, violation_id)` identity and pass once the concrete-theft duplicate boundary is in place:
- candidate generation: no second `GoalKind::Accuse` when the authority holds a different `ViolationId` for the same theft facts and the register already records that case
- justice action start: starting `accuse` for a different `ViolationId` but the same accused/theft pair is rejected as already recorded

### 3. Add Scenario 43 golden coverage

Add `run_dual_discovery_converges_without_double_accusation` and deterministic replay coverage in `crates/worldwake-ai/tests/golden_emergent.rs`.

The scenario should:
- prove both discovery paths activate independently
- allow the witness path to file the first accusation
- then deliver the owner-local path to the same authority
- keep punishment out of scope so the proof stays about accusation convergence
- assert the register still contains exactly one accusation case for that theft
- assert the second `GoalKind::Accuse` lane never becomes generated after the duplicate case is already recorded

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-systems/src/justice_actions.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `tickets/S32CRIMEMEGOLSUI-003.md` (modify, then archive on completion)

## Out of Scope

- Broad redesign of institutional belief-key identity beyond what is needed to make the record boundary robust
- Punishment selection or punishment action behavior
- Scenario 41, Scenario 42, and golden-doc inventory refresh work covered by other tickets
- Adding a backwards-compatibility alias layer between `ViolationId` and a new case-id type

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation`
2. `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation_replays_deterministically`
3. Focused regression tests for candidate generation and justice accuse validation covering “same theft facts, different `ViolationId`”
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The same concrete theft case cannot produce two accusation records for the same accused just because it arrived through two different authority-side `ViolationId`s.
2. Dual discovery remains lawful: the witness path and owner-local path both still produce independent theft evidence.
3. The `CrimeRegister` remains the canonical social artifact boundary for duplicate-case suppression.
4. Replay with the same seed yields identical `(world_hash, event_log_hash)`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — add focused regression for duplicate suppression by concrete theft facts rather than only by `ViolationId`
2. `crates/worldwake-systems/src/justice_actions.rs` — add focused regression for authoritative accuse rejection by concrete theft facts rather than only by `ViolationId`
3. `crates/worldwake-ai/tests/golden_emergent.rs::golden_dual_discovery_converges_without_double_accusation` — prove dual-path convergence yields one accusation
4. `crates/worldwake-ai/tests/golden_emergent.rs::golden_dual_discovery_converges_without_double_accusation_replays_deterministically` — prove the new golden is deterministic

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - Reassessed the ticket and corrected its scope from “golden only” to a real engine contradiction at the accusation case-identity boundary.
  - Added a shared record-level duplicate boundary using the concrete theft facts already stored in `InstitutionalClaim::Accusation`, so duplicate suppression no longer depends only on matching `ViolationId`.
  - Added focused regressions for candidate generation and authoritative accuse start when the same theft arrives under a different `ViolationId`.
  - Added Scenario 43 and its deterministic replay companion to `crates/worldwake-ai/tests/golden_emergent.rs`.
- Deviations from original plan:
  - The final change was not test-only. Production AI and justice code both required correction because the original duplicate check was architecturally weaker than the scenario assumed.
  - The golden isolates accusation convergence and intentionally leaves punishment out of scope so the proof stays on the record boundary under audit.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
