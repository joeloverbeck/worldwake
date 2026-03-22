# S19INSRECCON-005: Update golden-e2e-coverage.md and golden-e2e-scenarios.md with S19 scenarios

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation only
**Deps**: [`archive/specs/S19-institutional-record-consultation-golden-suites.md`](/home/joeloverbeck/projects/worldwake/archive/specs/S19-institutional-record-consultation-golden-suites.md); [`archive/tickets/completed/S19INSRECCON-002.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S19INSRECCON-002.md); [`archive/tickets/completed/S19INSRECCON-003.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S19INSRECCON-003.md); `docs/generated/golden-scenario-map.md`

## Problem

The live golden docs have drifted behind the delivered S19 office-record tests. The generated inventories already know about Scenario 33 (`golden_remote_record_consultation_political_action`) and Scenario 34 (`golden_knowledge_asymmetry_race_informed_wins_office`), but the human-maintained dashboards do not describe them yet.

This ticket must align the hand-written docs with the live source-declared test surface. It must not document planned-but-unshipped coverage as if it exists.

## Assumption Reassessment (2026-03-22)

1. `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` both exist, but neither currently documents the live S19 Scenario 33 or Scenario 34 coverage. Verified against [`docs/generated/golden-scenario-map.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md), which already lists Scenario 33 at `golden_offices.rs:1140` and Scenario 34 at `golden_offices.rs:1429`.
2. The original ticket assumption that “Scenarios 32–34” are all implemented is false. Live source declares Scenario 16 (`golden_information_locality_for_political_facts`) plus Scenario 33 and Scenario 34. There is no live source-declared Scenario 32 block or `golden_consult_record_prerequisite_political_action` test in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs).
3. The archived S19 spec draft was stale in the same direction: [`archive/specs/S19-institutional-record-consultation-golden-suites.md`](/home/joeloverbeck/projects/worldwake/archive/specs/S19-institutional-record-consultation-golden-suites.md) originally claimed zero golden E2E coverage for ConsultRecord and still treated Scenario 32 as planned. Live code and generated inventories contradicted that for Scenario 33 and 34, so the spec was corrected during archival.
4. This ticket is still documentation-owned, not engine-owned. The live runtime/test surface already exists in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs): `golden_remote_record_consultation_political_action`, `golden_remote_record_consultation_political_action_replays_deterministically`, `golden_knowledge_asymmetry_race_informed_wins_office`, and `golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`.
5. The relevant live `GoalKind` is `ClaimOffice`. The authoritative political action under test is `declare_support`, and the prerequisite/operator surface is planner insertion of `PlannerOpKind::ConsultRecord` when office-holder institutional belief is unknown. That contract is already exercised by the live Scenario 33 and Scenario 34 tests.
6. The relevant ordering contracts are mixed-layer and already implemented: plan-shape / candidate divergence via decision traces, consult-versus-support sequencing via action traces, and office-holder outcome via authoritative world state. The docs need to reflect those distinct proof surfaces rather than collapse them into a generic “scenario covers it” claim.
7. Test-name verification was dry-run checked with `cargo test -p worldwake-ai -- --list`, which confirms the exact live names for the Scenario 16, 33, and 34 office tests.
8. Scenario isolation in the live office-record tests is already encoded in the source: Scenario 33 isolates the remote-record travel/consult/political chain; Scenario 34 isolates knowledge asymmetry rather than travel distance as the deciding variable. This ticket should describe those actual contracts, not the spec’s older hypothetical local-consult scenario.
9. The clean architectural path is to treat [`docs/generated/golden-scenario-map.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md) plus live source as the source of truth for shipped scenarios, and to update the hand-maintained docs around that truth. Inventing Scenario 32 in prose would reintroduce the same doc/spec drift that the generated map was added to prevent.
10. No production code changes are warranted by this reassessment. The discrepancy is documentation/spec state, not an architectural hole in the live runtime.
11. Mismatch + correction: scope is corrected from “document Scenarios 32–34 as delivered” to “document the actually delivered Scenario 33 and 34 coverage, and avoid claiming the spec’s planned Scenario 32 is live.”
12. No new arithmetic-sensitive test surface is introduced here; the docs should simply reflect the live scenario math already encoded in source, including the long-consult timing asymmetry in Scenario 34.

## Architecture Check

1. Aligning the docs to source-declared live scenarios is cleaner than preserving the older “planned S19 inventory” narrative. It keeps the hand-written docs subordinate to executable truth and to the generated scenario map, which is the more robust long-term architecture for golden coverage tracking.
2. No backwards-compatibility aliasing or shadow scenario names should be introduced. If the spec planned Scenario 32 but the repo shipped different coverage, the docs must say what shipped and the archived outcome must record the deviation plainly.

## Verification Layers

1. Exact live scenario ownership / names -> `cargo test -p worldwake-ai -- --list` and [`docs/generated/golden-scenario-map.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md)
2. Scenario 33 plan/action/state contract summary remains truthful -> live test source in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs)
3. Scenario 34 knowledge-asymmetry race summary remains truthful -> live test source in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs)
4. Human-maintained docs remain synchronized with generated inventory/map -> `python3 scripts/golden_inventory.py --write --check-docs`
5. No additional verification layer is needed beyond documentation truthfulness because this ticket does not change runtime behavior.

## What to Change

### 1. Update `docs/golden-e2e-coverage.md`

Document the delivered office-record chains that are actually present in source:

- Scenario 33: remote record travel -> consultation -> return to jurisdiction -> `declare_support` -> succession installation
- Scenario 34: knowledge asymmetry -> informed claimant acts immediately while uninformed claimant pays consult duration -> competitive office outcome

Update any summary counts that change as a result, including the RulersHall topology count and cross-system-chain total.

### 2. Update `docs/golden-e2e-scenarios.md`

Add detailed scenario descriptions for the delivered Scenario 33 and Scenario 34 entries using the existing office-scenario format. If helpful for reader context, distinguish them from the already-live Scenario 16 locality coverage without claiming that Scenario 16 is an S19 scenario.

### 3. Run golden inventory script

Run `python3 scripts/golden_inventory.py --write --check-docs` to verify the generated inventory and the hand-written docs remain aligned after the update.

## Files to Touch

- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)
- `tickets/S19INSRECCON-005.md` (modify for reassessment/finalization)
- `archive/specs/S19-institutional-record-consultation-golden-suites.md` (closed out and archived during finalization)
- `specs/IMPLEMENTATION-ORDER.md` (modify after S19 archival)

## Out of Scope

- No code changes of any kind
- No changes to `golden_offices.rs` or `golden_harness/mod.rs`
- No changes to engine crates
- No invention of a non-existent Scenario 32 test just to satisfy stale prose

## Acceptance Criteria

### Tests That Must Pass

1. `python3 scripts/golden_inventory.py --write --check-docs` — inventory script passes with updated docs
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action` — targeted live Scenario 33 verification
3. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office` — targeted live Scenario 34 verification
4. `cargo test -p worldwake-ai` — full AI crate suite
5. `cargo test --workspace` — workspace suite
6. `cargo clippy --workspace --all-targets -- -D warnings` — lint

### Invariants

1. The docs describe only source-declared live scenarios; they do not claim the planned-but-missing Scenario 32 exists.
2. Scenario 33 and 34 entries accurately reflect the actual test implementations in [`golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs).
3. Existing scenario descriptions outside the touched S19-alignment area remain semantically unchanged.
4. No code files are modified

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs` — golden inventory refresh
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
3. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-22
- What actually changed:
  - corrected the ticket scope before implementation to match live source and generated inventory
  - updated [`docs/golden-e2e-coverage.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) to include the delivered Scenario 33 and 34 office-record coverage, including topology/count updates
  - updated [`docs/golden-e2e-scenarios.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md) with detailed Scenario 33 and 34 descriptions grounded in the live tests
- Deviations from original plan:
  - the original ticket incorrectly assumed Scenario 32, 33, and 34 were all live
  - live source only ships Scenario 33 and 34 for S19; Scenario 32 remains planned in the old spec narrative and was not documented as shipped
  - no code changes or new tests were required because the runtime/test surface already existed
- Verification results:
  - `python3 scripts/golden_inventory.py --write --check-docs` ✅
  - `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action` ✅
  - `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
