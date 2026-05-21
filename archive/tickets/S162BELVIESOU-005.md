# S162BELVIESOU-005: Adversarial belief-wall goldens

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None (golden/E2E tests only) — `worldwake-ai`
**Deps**: `S162BELVIESOU-001.md`, `S162BELVIESOU-002.md`, `S162BELVIESOU-003.md`, `S162BELVIESOU-006.md` (office/record carrier-positive cases need the lawful believed snapshot), Spec `../specs/S162-belief-view-source-gate-hardening.md` (D7)

## Problem

The belief-view gate fixes (001/002/003) close FND-14/14A leaks at the accessor
level, but FND-31 requires inspectable evidence that the *planner consequence* is
correct: a remote authoritative change with no lawful carrier must change no
affordance, candidate, ranking, or HTN method for a distant actor. Live
reassessment found that the adversarial matrix already exists in the active
golden suite; this ticket repairs and truth-syncs that proof surface after the
S162 snapshot guard so it proves absence-for-the-right-reason rather than
"looked plausible."

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
15. Post-[S162BELVIESOU-004](S162BELVIESOU-004.md) handoff:
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
16. Live reassessment result (2026-05-21): the draft assumption that no
    adversarial belief-wall goldens existed was stale. The current
    `scenarios::belief_wall_trap` module already carries the owner/control,
    remote sale/production/load, remote queue/grant, and remote pursuit
    belief-wall matrix at the planner-visible layer, and
    `scenarios::offices::golden_information_locality_for_political_facts`
    covers the remote office no-carrier/explicit-carrier path. This ticket's
    implementation therefore repaired and truth-synced those live proof
    surfaces instead of adding duplicate scenario modules.
17. The stale package failure
    `ai_decisions::golden_consume_pipeline_rebinds_pick_up_after_remote_lot_change`
    was renamed and narrowed to
    `ai_decisions::golden_consume_pipeline_records_start_failure_after_remote_lot_change`.
    After S162, a forced external stale `pick_up` request may still reach
    authoritative start, but it must fail lawfully and must not imply an
    immediate AI rebind/eat without a fresh lawful replacement carrier.
18. The loyalty clause was reassessed against live architecture. Current
    `PoliticalBeliefView::loyalty_to` treats the actor's own loyalty row as
    actor-internal state once the target entity is believed, so there is no
    separate remote public loyalty carrier in scope for this ticket. The
    completed proof therefore keeps loyalty-dependent office goldens lawful by
    pairing target knowledge with explicit office/holder/snapshot belief, rather
    than inventing a non-existent loyalty snapshot carrier.

## Architecture Check

1. End-to-end goldens at the candidate/affordance layer with decision-trace
   provenance are the FND-31-mandated proof that the gates produce the right *planner*
   behavior, not just the right accessor return value. Asserting candidate/affordance
   absence (with the trace showing the missing belief as the cause) proves
   absence-for-the-right-reason. This complements — does not duplicate — the focused
   accessor tests in 001/002/003.
2. No production code change; no backwards-compatibility concern.

## Verified Layers

1. Remote owner/control change -> no new control/rights affordance -> golden +
   decision-trace assertion (candidate absent; trace shows belief unchanged).
2. Remote office vacancy / record entry change → no claim/support/investigation
   candidate or HTN method-selection change until consult/testimony -> golden +
   decision/HTN-method trace.
3. Remote extraction slot filled / reservation created → distant actor's candidate
   and ranking unchanged; authoritative start may still fail lawfully -> golden +
   decision trace (and action trace if a start-abort path is exercised).
4. Loyalty-dependent political behavior → no invented public loyalty snapshot
   carrier; current actor-internal loyalty rows remain paired with explicit
   target, office, holder, and office-data beliefs in office goldens.
   Each invariant maps to a distinct candidate/affordance + trace surface; no layer is
   collapsed into a generic "scenario passed" assertion (FND-31; README check 11).

## Landed Changes

### 1. Adversarial belief-wall proof surface

Repaired and truth-synced the existing package-level golden/scenario surfaces that
now depend on S162's lawful believed office/record snapshot substrate, restoring
`cargo test -p worldwake-ai` to a stable baseline.

Used the live adversarial matrix already present in
`crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` and the office locality
goldens instead of duplicating scenario modules. Assertions remain at the
candidate/affordance/action-trace layer and preserve explicit positive carriers
where the domain supports them.

### 2. Golden inventory regeneration

Regenerated the golden inventory docs
(`python3 scripts/golden_inventory.py --write --check-docs`) so
`docs/generated/golden-e2e-inventory.md` and siblings stay consistent.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modified — lawful owner and office-data belief seeding helpers)
- `crates/worldwake-ai/tests/scenarios/offices.rs` (modified — office goldens now seed explicit `BelievedOfficeDataSnapshot` carriers)
- `crates/worldwake-ai/tests/scenarios/ai_decisions.rs` (modified — stale remote-lot request now proves lawful `StartFailed` instead of synthetic rebind/eat)
- `crates/worldwake-ai/tests/planner_pathology_harness/mod.rs` (modified — obligation fixture uses possessed stock so the scenario stays about satiation rather than ownerless-pickup legality)
- `docs/generated/golden-e2e-inventory.md` and siblings (modified — regenerated after the test rename/comment changes)

## Out of Scope

- The accessor gate fixes themselves (001/002/003).
- The snapshot-through-view guard
  (`S162BELVIESOU-004.md`).
- The believed record/office snapshot substrate (`S162BELVIESOU-006.md`).

## Acceptance Criteria

### Tests That Must Pass

1. Existing golden coverage: `scenarios::belief_wall_trap` proves owner/control, remote contention/extraction grant, and adjacent remote source-gate no-carrier behavior at the candidate/affordance/action-trace layer.
2. Modified office goldens: remote office vacancy/record knowledge remains absent until a carrier arrives, then appears through explicit holder belief plus `BelievedOfficeDataSnapshot`.
3. Modified stale-lot golden: remote lot movement without a replacement carrier records a lawful `pick_up` `StartFailed` and leaves hunger unresolved.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No planner-visible candidate/affordance/ranking/HTN-method changes from a remote authoritative fact absent a lawful carrier (FND-14B test clause).
2. Each golden proves absence-for-the-right-reason via decision-trace provenance, not structural activation alone (FND-31).

## Test Plan

### Modified Tests

1. `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (existing) — live adversarial belief-wall matrix reused as the canonical D7 proof surface.
2. `crates/worldwake-ai/tests/scenarios/offices.rs` (modified) — office carrier-positive cases now use `BelievedOfficeDataSnapshot`.
3. `crates/worldwake-ai/tests/scenarios/ai_decisions.rs` (modified) — stale external `pick_up` proves lawful start failure.
4. `docs/generated/golden-e2e-inventory.md` and sibling generated docs (regenerated) — keep the canonical golden inventory consistent.

## Implementation Result

- Added harness helpers for explicit owner and believed office-data seeding.
- Updated office goldens to use explicit `BelievedOfficeDataSnapshot` carriers instead of relying on broad authoritative world seeding.
- Renamed and narrowed the stale remote-lot consume golden to assert lawful start failure with no immediate hunger relief absent a fresh carrier.
- Kept the obligation-satiation golden focused on notice-satiation/self-care by giving the guard possessed food and water.

## Outcome

Completed. The active S162 D7 proof surface is stable again, generated golden
docs are synchronized, and the ticket records the truthful boundary that loyalty
is actor-internal state in the current architecture rather than a separate public
snapshot carrier.

## Verification Result

- Passed: `cargo test -p worldwake-ai -- --list`.
- Passed: `cargo test -p worldwake-ai scenarios::belief_wall_trap`.
- Passed: `cargo test -p worldwake-ai scenarios::offices`.
- Passed: `cargo test -p worldwake-ai golden_consume_pipeline_records_start_failure_after_remote_lot_change`.
- Passed: `cargo test -p worldwake-ai obligation_satiation_allows_survival_needs_to_override_posting`.
- Passed: `cargo test -p worldwake-ai`.
- Passed: `python3 scripts/golden_inventory.py --write --check-docs`.
- Waived: full pre-PR verification script; this package-scoped ticket used the package test plus regenerated golden docs as the owned proof lane.
