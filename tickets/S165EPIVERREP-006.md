# S165EPIVERREP-006: Scenario D golden and test migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — tests only (`worldwake-ai` tests)
**Deps**: S165EPIVERREP-001, S165EPIVERREP-002, S165EPIVERREP-003, S165EPIVERREP-004, S165EPIVERREP-005

## Problem

S165's closed gap must stay proven. The two `without_s139` tests assert the now-removed
always-fail behavior and must be migrated to assert real verification repair. A new golden
must prove FOUNDATIONS Scenario D end-to-end: an agent acts on a stale belief, the link
breaks, a co-located `ask_witness` verification is spliced and executed, evidence imports
with `Report` provenance, and the authoritative `RepairApplied` event records the witness
— plus a no-witness branch that falls through to a typed `InformationBarrier`.

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

## Architecture Check

1. Migrating (not deleting) the `without_s139` tests keeps the closed gap under active
   proof rather than leaving a coverage hole. The golden proves the authored causal reason
   (Scenario D), satisfying FND-31's "structural activation is not causal proof" bar.
2. The no-witness branch locks the fall-through so a future change cannot silently turn
   every breach into a verification attempt.

## Verification Layers

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

## What to Change

### 1. Migrate the `without_s139` unit test

Rewrite `insert_verification_returns_no_epistemic_substrate_without_s139`
(`plan_repair.rs:546`) into: (a) belief-backed breach + supplied verification candidate →
`Repaired` with the `ask_witness` step; (b) non-epistemic / no-candidate breach →
`NoEpistemicSubstrate`. Rename to drop the `without_s139` suffix.

### 2. Migrate the `without_s139` scenario test

Rewrite `stale_belief_breach_attempts_insert_verification_without_s139`
(`tests/scenarios/plan_repair.rs:300`) to assert a stale-belief breach with a lawful
co-located witness yields a verification repair (and the no-witness sibling falls through).

### 3. New Scenario D golden

Add `crates/worldwake-ai/tests/golden_epistemic_verification_repair.rs` covering the
witness branch (end-to-end, with the event-log + belief-provenance assertions above) and
the no-witness branch, plus a deterministic replay check. Regenerate the golden inventory.

## Files to Touch

- `crates/worldwake-ai/src/plan_repair.rs` (modify — migrate inline test)
- `crates/worldwake-ai/tests/scenarios/plan_repair.rs` (modify — migrate scenario test)
- `crates/worldwake-ai/tests/golden_epistemic_verification_repair.rs` (new)
- `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`,
  `docs/generated/golden-scenario-details/` (regenerate via
  `python3 scripts/golden_inventory.py --write --check-docs`)

## Out of Scope

- Any production code change (owned by tickets 001-005).
- Broader Scenario G false-rumor/wrongful-accusation chain (spec Non-Goal / S139 deferral).

## Acceptance Criteria

### Tests That Must Pass

1. Migrated unit test: lawful co-located witness → `Repaired` with `ask_witness` step;
   no-candidate/non-epistemic → `NoEpistemicSubstrate`.
2. Migrated scenario test: stale-belief breach + co-located witness → verification repair;
   no-witness sibling → typed `InformationBarrier`.
3. New golden `golden_epistemic_verification_repair`: end-to-end witness branch (event +
   belief-provenance assertions), no-witness branch, deterministic replay.
4. No regression: `cargo test -p worldwake-ai` (1440-tick survival goldens unaffected).

### Invariants

1. Belief is never made true without a carrier; the discrepancy persists until the
   `ask_witness` import updates the belief (FND-16).
2. The golden proves the authored causal reason, not mere survival (FND-31).
3. `SAVE_FORMAT_VERSION` unchanged; replay is byte-stable.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_repair.rs` — migrated inline unit test.
2. `crates/worldwake-ai/tests/scenarios/plan_repair.rs` — migrated scenario test.
3. `crates/worldwake-ai/tests/golden_epistemic_verification_repair.rs` — new Scenario D
   golden (witness + no-witness + replay).

### Commands

1. `cargo test -p worldwake-ai --test golden_epistemic_verification_repair`
2. `cargo test -p worldwake-ai plan_repair`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `./scripts/verify.sh`
