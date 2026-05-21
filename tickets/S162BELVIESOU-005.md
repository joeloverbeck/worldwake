# S162BELVIESOU-005: Adversarial belief-wall goldens

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None (golden/E2E tests only) — `worldwake-ai`
**Deps**: `archive/tickets/S162BELVIESOU-001.md`, `archive/tickets/S162BELVIESOU-002.md`, `archive/tickets/S162BELVIESOU-003.md`, `archive/tickets/S162BELVIESOU-006.md` (office/record carrier-positive cases need the lawful believed snapshot), Spec `specs/S162-belief-view-source-gate-hardening.md` (D7)

## Problem

The belief-view gate fixes (001/002/003) close FND-14/14A leaks at the accessor
level, but FND-31 requires inspectable evidence that the *planner consequence* is
correct: a remote authoritative change with no lawful carrier must change no
affordance, candidate, ranking, or HTN method for a distant actor. No adversarial
belief-wall goldens exist today. This ticket adds them, proving absence-for-the-right-
reason rather than "looked plausible."

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Verified (2026-05-21): the gate fixes land in 001/002/003 (`per_agent_belief_view.rs`);
   this ticket consumes their post-fix behavior. The golden harness lives under
   `crates/worldwake-ai/tests/` (post-S154 form: `golden_ai` plus a substring filter
   against the scenario module path, per CLAUDE.md). Confirm the exact current harness
   entry and naming with `cargo test -p worldwake-ai -- --list` during reassessment
   before writing command lines (README check 7).
2. Spec D7 and FOUNDATIONS canonical regressions I (planner belief barrier around
   remote affordances) and J (HTN method rejection/fallback) define the required
   scenario classes. `docs/golden-e2e-testing.md` is the canonical guide for
   proof-surface and ordering choices.
3. Shared boundary under audit: the belief view → candidate generation / ranking /
   HTN selection / affordance enumeration pipeline. The goldens assert at the
   planner-visible output layer (candidate/affordance presence) with decision-trace
   provenance, not at the accessor layer (001/002/003 own that).
4. Intended invariant (restated before trusting any scenario narrative): updating
   authoritative truth alone — remote owner, control source, office vacancy, record
   entry, extraction slot, reservation, or loyalty — must NOT add or change a
   planner-visible candidate/affordance for a non-co-located actor with no carrier;
   the same candidate may appear only after a lawful carrier (consult/testimony/
   perception/travel) updates belief.
5. Planner/golden surfaces: name the live `GoalKind`s exercised per scenario during
   reassessment (e.g., bounty/claim families for office/record; acquisition/production
   for queue/reservation; political families for loyalty). Verify each scenario's
   candidate actually depends on the gated read in current code before asserting its
   absence — a candidate that never depended on the leaked fact would pass vacuously.
12. Isolation: each adversarial golden isolates one belief-wall class and excludes
    unrelated lawful competing affordances so the assertion (candidate absent until
    carrier) is unambiguous. Name the excluded branches per scenario.
13. Adjacent contradiction: existing goldens that seed broad world beliefs for
    convenience remain valid only if their assertions do not claim ignorance/stale
    behavior (third-iteration report §13). If any existing golden silently relied on a
    now-closed leak, it belongs to the owning gate ticket (001/002/003) to fix, not
    here; flag it as a separate finding if discovered.
14. Post-S162BELVIESOU-006 update: the lawful whole-record/office carrier now exists
    as `BelievedRecordDataSnapshot` / `BelievedOfficeDataSnapshot`, with
    `consult_record` as the first lawful acquisition path. The office/record
    positive-carrier half of this golden ticket should use that substrate instead of
    faking a carrier by seeding authoritative world truth. Owner/control, contention,
    and loyalty no-carrier absence cases may still be reassessed independently.
15. Post-[S162BELVIESOU-004](../archive/tickets/S162BELVIESOU-004.md) handoff:
    `cargo test -p worldwake-ai` is currently blocked in package-level
    golden/scenario surfaces after the snapshot guard and library fixture repair
    landed. The observed failures include office goldens
    (`scenarios::offices::*`) plus adjacent scenario fallout
    (`ai_decisions::golden_consume_pipeline_rebinds_pick_up_after_remote_lot_change`
    and
    `planner_pathology::obligation_satiation_allows_survival_needs_to_override_posting`).
    This ticket owns reassessing those golden/scenario contracts against the lawful
    believed office/record snapshot substrate before adding the new adversarial
    belief-wall matrix.

## Architecture Check

1. End-to-end goldens at the candidate/affordance layer with decision-trace
   provenance are the FND-31-mandated proof that the gates produce the right *planner*
   behavior, not just the right accessor return value. Asserting candidate/affordance
   absence (with the trace showing the missing belief as the cause) proves
   absence-for-the-right-reason. This complements — does not duplicate — the focused
   accessor tests in 001/002/003.
2. No production code change; no backwards-compatibility concern.

## Verification Layers

1. Remote owner/control change → no new control/rights affordance -> golden +
   decision-trace assertion (candidate absent; trace shows belief unchanged).
2. Remote office vacancy / record entry change → no claim/support/investigation
   candidate or HTN method-selection change until consult/testimony -> golden +
   decision/HTN-method trace.
3. Remote extraction slot filled / reservation created → distant actor's candidate
   and ranking unchanged; authoritative start may still fail lawfully -> golden +
   decision trace (and action trace if a start-abort path is exercised).
4. Remote loyalty change → no political/economic candidate shift -> golden + decision
   trace.
   Each invariant maps to a distinct candidate/affordance + trace surface; no layer is
   collapsed into a generic "scenario passed" assertion (FND-31; README check 11).

## What to Change

### 1. Add adversarial belief-wall goldens

First repair or truth-sync the existing package-level golden/scenario surfaces that
now depend on S162's lawful believed office/record snapshot substrate, so
`cargo test -p worldwake-ai` has a stable baseline before adding new belief-wall
coverage.

Add golden scenarios under the current `worldwake-ai` golden harness covering, at
minimum: remote owner/control change, remote office vacancy + remote record entry
change, remote extraction slot fill + remote reservation, and remote loyalty change.
Each scenario: establish a distant actor with a stale/absent belief, mutate
authoritative truth with no carrier, advance, and assert the dependent candidate/
affordance is absent or unchanged via the decision trace; then (where the scenario
class supports it) deliver a lawful carrier and assert the candidate appears with
knowledge provenance. Reuse existing scenario-construction helpers; isolate each
belief-wall class per Assumption Reassessment 12.

### 2. Regenerate golden inventories

If the harness requires it, regenerate the golden inventory docs
(`python3 scripts/golden_inventory.py --write --check-docs`) so
`docs/generated/golden-e2e-inventory.md` and siblings stay consistent.

## Files to Touch

- `Likely: crates/worldwake-ai/tests/scenarios/` (new — adversarial belief-wall scenario module(s); confirm exact path/naming with `cargo test -p worldwake-ai -- --list` during reassessment)
- `Likely: crates/worldwake-ai/tests/` golden harness registration (modify — per current post-S154 harness layout)
- `docs/generated/golden-e2e-inventory.md` and siblings (modify — regenerated, if the harness tracks new goldens)

## Out of Scope

- The accessor gate fixes themselves (001/002/003).
- The snapshot-through-view guard
  (`archive/tickets/S162BELVIESOU-004.md`).
- The believed record/office snapshot substrate (`archive/tickets/S162BELVIESOU-006.md`).

## Acceptance Criteria

### Tests That Must Pass

1. New golden: remote owner/control change produces no new control/rights affordance for the distant actor.
2. New golden: remote office vacancy / record change produces no claim/support/investigation candidate or HTN method-selection change until a carrier arrives.
3. New golden: remote extraction-slot fill / reservation leaves the distant actor's candidate set and ranking unchanged.
4. New golden: remote loyalty change produces no political/economic candidate shift.
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No planner-visible candidate/affordance/ranking/HTN-method changes from a remote authoritative fact absent a lawful carrier (FND-14B test clause).
2. Each golden proves absence-for-the-right-reason via decision-trace provenance, not structural activation alone (FND-31).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/<belief_wall>.rs` (new) — one scenario per belief-wall class; rationale: FND-31 evidence that the gates produce correct planner behavior.
2. `docs/generated/golden-e2e-inventory.md` (regenerated) — keep the canonical golden inventory consistent.

### Commands

1. `cargo test -p worldwake-ai -- --list` (first, to pin the current harness entry and naming)
2. `cargo test -p worldwake-ai golden_ai <belief_wall substring>`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `./scripts/verify.sh` (before PR)
