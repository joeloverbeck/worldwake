# S165EPIVERREP-006: Scenario D golden and test migration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — tests only (`worldwake-ai` tests)
**Deps**: archive/tickets/S165EPIVERREP-001.md, archive/tickets/S165EPIVERREP-002.md, archive/tickets/S165EPIVERREP-003.md, archive/tickets/S165EPIVERREP-004.md, archive/tickets/S165EPIVERREP-005.md

## Problem

S165's closed gap must stay proven. The two `without_s139` tests asserted the now-removed
always-fail behavior and must be migrated to assert real verification repair. The proof
surface must show that a stale belief-backed breach can splice a co-located `ask_witness`
verification, that the authoritative `RepairApplied` event records the witness, and that
the no-witness branch still falls through to a typed `InformationBarrier`.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Tests to migrate: `insert_verification_returns_no_epistemic_substrate_without_s139`
   (`crates/worldwake-ai/src/plan_repair.rs:546`) and
   `stale_belief_breach_attempts_insert_verification_without_s139`
   (`crates/worldwake-ai/tests/scenarios/plan_repair.rs:300`). Both currently assert
   `NoEpistemicSubstrate`; after tickets 001-003 a lawful co-located witness yields
   `Repaired` with an `ask_witness` step.
2. Spec deliverable D8 + Validation/Falsification section
   (`specs/S165-epistemic-verification-repair.md`). Golden inventory is regenerated with
   `python3 scripts/golden_inventory.py --write --check-docs` per `tickets/README.md`.
3. Coverage classification (precision rule 3): this ticket adds **golden/E2E coverage**
   (new `golden_epistemic_verification_repair.rs`) plus migrates two focused/scenario
   tests. The runtime `agent_tick` decision-trace coverage for the seam lives in ticket
   003; this ticket is the end-to-end proof.
4. Live `GoalKind` under test: `GoalKind::AskWitness`; the verification action is
   `ask_witness` toward a co-located witness. The harness requires full action registries
   (the `ask_witness` handler must execute and import belief), not a needs-only harness.
5. Scenario isolation (precision rule 8): (a) intended branch — a belief-backed breach
   triggers a spliced co-located `ask_witness` verification that the agent executes,
   resuming or abandoning with the discrepancy retained until the carrier updates the
   belief; (b) lawful competing affordances the architecture permits — full replan, other
   repair kinds (`RebindTarget`/`ReplaceProvider`), self-care goals; (c) the scenario
   excludes alternative repair candidates and competing high-priority goals so the
   verification branch is the one under test, and includes a sibling no-witness setup that
   must instead fall through to `DowngradeToTypedBarrier`.

## Assumption Reassessment (2026-05-24)

1. Live reassessment found the positive `InsertVerification` unit coverage already exists
   in `crates/worldwake-ai/src/plan_repair.rs` as
   `insert_verification_returns_repaired_plan_for_supplied_candidate`; the stale unit
   fossil was the negative no-candidate test name and panic wording. This ticket therefore
   renames that negative to `insert_verification_returns_no_epistemic_substrate_without_candidate`
   and keeps the existing positive unit proof.
2. The current golden owner for plan-repair scenario metadata is
   `crates/worldwake-ai/tests/scenarios/plan_repair.rs`, compiled by the existing
   `golden_ai` integration binary. Creating a separate `golden_epistemic_verification_repair`
   integration target would duplicate the owning surface rather than strengthen the proof.
   The landed golden work migrates Scenario 409 in the existing owner and adds Scenario 461
   for the no-witness fall-through.
3. Full action execution and `PerceptionSource::Report` import remain owned by the existing
   S139 golden in `crates/worldwake-ai/tests/scenarios/epistemic_sensing.rs`
   (`golden_ask_witness_refreshes_stale_report` plus deterministic replay). This ticket's
   new S165 golden proves the repair seam, selected `AskWitness` step shape, authoritative
   witness anchor, and no-witness typed barrier; it relies on the S139 golden for the
   already-landed action effect sink.

## Architecture Check

1. Migrating (not deleting) the `without_s139` tests keeps the closed gap under active
   proof rather than leaving a coverage hole. The golden proves the authored causal reason
   (Scenario D), satisfying FND-31's "structural activation is not causal proof" bar.
2. The no-witness branch locks the fall-through so a future change cannot silently turn
   every breach into a verification attempt.

## Verified Layers

1. Spliced verification chosen for the right reason (belief-backed breach + co-located
   witness) → decision trace (`RepairAttemptTrace` anchor from ticket 005).
2. `ask_witness` action executes and imports belief with `PerceptionSource::Report`
   provenance → action trace + authoritative belief state.
3. Authoritative `RepairApplied` records `substitute_target = Some(witness)` → event-log
   delta.
4. Belief stays stale until the carrier updates it (no magic correction) → authoritative
   belief state across ticks.
5. No-witness breach → typed `InformationBarrier` → decision trace.
6. Replay/save-load equivalence over the golden → deterministic replay check
   (`SAVE_FORMAT_VERSION` unchanged).

## Implemented Changes

### 1. Migrated the `without_s139` unit test

Renamed `insert_verification_returns_no_epistemic_substrate_without_s139`
(`plan_repair.rs`) to `insert_verification_returns_no_epistemic_substrate_without_candidate`.
The positive supplied-candidate unit proof already existed as
`insert_verification_returns_repaired_plan_for_supplied_candidate`; the migrated negative
now records the still-lawful no-candidate `NoEpistemicSubstrate` path without stale S139
or ticket-007 wording.

### 2. Migrated the `without_s139` scenario test

Replaced `stale_belief_breach_attempts_insert_verification_without_s139` with
`stale_belief_breach_inserts_ask_witness_verification`. The migrated scenario now asserts
that a stale belief-backed breach with a supplied co-located witness verification candidate
selects `RepairKind::InsertVerification`, splices an `AskWitness` step, and round-trips a
`RepairApplied` payload with `substitute_target = Some(witness)`.

### 3. Added no-witness fall-through golden coverage

Added `stale_belief_breach_without_witness_falls_through_to_information_barrier` in the
existing plan-repair golden owner. It asserts the no-candidate branch records
`NoEpistemicSubstrate` for `InsertVerification` and then downgrades to a typed
`InformationBarrier`.

### 4. Regenerated golden docs

Regenerated the golden inventory and generated scenario docs so the old
`without_s139` scenario name is removed and the new S165 plan-repair scenario metadata is
published.

## Files Touched

- `crates/worldwake-ai/src/plan_repair.rs` (modify — migrate inline test)
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs` (modify — migrate scenario test)
- `specs/S165-epistemic-verification-repair.md` (modify — truth-sync D8 and validation
  proof surfaces)
- `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`,
  `docs/generated/golden-scenario-details/`, `docs/generated/golden-coverage-matrix.md`
  (regenerate via
  `python3 scripts/golden_inventory.py --write --check-docs`)

## Out of Scope

- Any production code change (owned by tickets 001-005).
- Broader Scenario G false-rumor/wrongful-accusation chain (spec Non-Goal / S139 deferral).

## Acceptance Criteria

### Tests That Passed

1. Migrated unit test: lawful co-located witness → `Repaired` with `ask_witness` step;
   no-candidate/non-epistemic → `NoEpistemicSubstrate`.
2. Migrated scenario test: stale-belief breach + co-located witness → verification repair;
   no-witness sibling → typed `InformationBarrier`.
3. Existing `golden_ai::scenarios::plan_repair` coverage proves the witness repair branch
   and no-witness branch; existing S139 `golden_ask_witness_refreshes_stale_report`
   continues to prove the action effect sink imports `Report` provenance and replays
   deterministically.
4. No regression: `cargo test -p worldwake-ai` (1440-tick survival goldens unaffected).

### Invariants

1. Belief is never made true without a carrier; the discrepancy persists until the
   `ask_witness` import updates the belief (FND-16).
2. The golden proves the authored causal reason, not mere survival (FND-31).
3. `SAVE_FORMAT_VERSION` unchanged; replay is byte-stable.

## Verification Result

1. Passed `cargo test -p worldwake-ai plan_repair`.
2. Passed `cargo test -p worldwake-ai --test golden_ai golden_ask_witness_refreshes_stale_report`.
3. Passed `python3 scripts/golden_inventory.py --write --check-docs`.
4. Passed `cargo test -p worldwake-ai`.

## Outcome

Completed: 2026-05-24

This ticket migrated the stale S165 `without_s139` tests to the landed epistemic
verification repair behavior. The live proof uses the existing plan-repair golden owner
rather than a new `golden_epistemic_verification_repair` integration target, because that
is the current repository owner for plan-repair scenario metadata. The existing S139
`AskWitness` replay golden remains the proof that the spliced action's effect sink imports
belief through `PerceptionSource::Report`.

Deviations from the original plan:

1. No new `crates/worldwake-ai/tests/golden_epistemic_verification_repair.rs` file was
   added; the proof landed in `crates/worldwake-ai/tests/scenarios/plan_repair.rs`.
2. The full `./scripts/verify.sh` pre-PR gate was not run at ticket closeout; the ticket's
   requested no-regression boundary was covered by `cargo test -p worldwake-ai`, and the
   harness will run the repository pre-PR gate before final branch push.
