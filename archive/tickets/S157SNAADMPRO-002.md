# S157SNAADMPRO-002: Source-restricted strategic scans

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` strategic plan search (`search/strategic.rs`)
**Deps**: `archive/tickets/S157SNAADMPRO-001.md`

## Problem

Before this ticket, strategic search scanned the admitted entity map directly —
`state.snapshot().entities.keys()` — to discover workstations, sellers, resource sources, and
acquisition places (`crates/worldwake-ai/src/search/strategic.rs:902,914,923,956`). That raw scan
was only sound if snapshot admission was airtight: nothing stopped a scan from reading, say, a
seller's `item_lot_commodity`/`has_sale_listing` on an entity that was admitted purely as a
topology place. Ticket 001 records why each entity was admitted; this ticket restricts the
strategic scans to entities whose admission source legitimately exposes the field being read, so
the scan cannot consume a field admitted for an unrelated reason (S157 D2; FND-7, FND-14A,
FND-20). This is defense-in-depth: post-S155 the snapshot is built from a belief-correct view, so
the restriction is behavior-preserving — proven by the golden suite.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `search/strategic.rs` has four place-scan functions over the entity map (verified 2026-05-20):
   `workstation_places` (line 902, filters `state.workstation_tag(*entity)`), `seller_places`
   (line 914, `item_lot_commodity` + `has_sale_listing`), `resource_source_places` (line 923,
   `resource_source(...).commodity`), and `acquisition_places_for_commodity` (line 956,
   `place_supports_commodity`). The first three route through `places_for_entities` (line 934);
   the fourth iterates `entities.keys()` directly. All four read `state.snapshot().entities.keys()`.
2. This ticket depends on ticket 001 having added `AdmissionSource` to `SnapshotEntity` and a way
   to read it per id from `PlanningState`/`PlanningSnapshot`. Confirm the accessor surface ticket
   001 exposes (a public/`pub(crate)` getter or direct field read on `SnapshotEntity`) before
   wiring; if 001 did not expose a `PlanningState`-level reader, add one in this ticket scoped to
   the strategic-search need.
3. Shared boundary under audit: the strategic-search place-discovery layer (`search/strategic.rs`)
   reading the `SnapshotEntity` read-model. This is **plan-search-internal** affordance discovery,
   distinct from authoritative validation — no action precondition, `validate_*`, candidate
   emission, or goal-satisfaction surface is touched, so the Authoritative-to-AI Impact Rule
   7-point checklist does not apply (confirmed against S157's reassessment, which found Step 4.4
   does not fire).
4. Existing focused coverage exercising these scans (must continue to pass): `test_single_location_goal_no_travel`
   (`strategic.rs:1650`), `test_multi_location_prerequisite_then_goal` (1677),
   `test_belief_only_excludes_unknown_locations` (1744). These build a `PlanningState` and assert
   strategic place selection; the source restriction must not change their outcomes.
5. Heuristic-removal discipline (precision rule 12): this ticket does not remove a heuristic — it
   *tightens* an existing scan with the provenance substrate ticket 001 adds. The restriction
   stands in for the previously-implicit assumption that admission was airtight; it does not
   weaken any existing filter. Because post-S155 the belief-view feeding the snapshot is correct,
   restricting by source should exclude no entity the scan legitimately needed — the golden
   no-regression guard is the proof this does not reopen unrelated scenarios.

## Architecture Check

1. The restriction is expressed once as a source-aware accessor (e.g.,
   `visible_entities_for_field(predicate, allowed_sources)` or per-field helpers like
   `seller_candidate_entities()`), not duplicated as an inline `admission == ...` guard at each of
   the four scan sites. A single accessor keeps the source→legal-field policy in one place (DRY)
   and makes the legality contract auditable.
2. No backward-compat path: the raw `entities.keys()` scans are replaced in place, not wrapped or
   aliased. The four call sites move to the new accessor in this ticket; the old direct-scan form
   is removed (FND-28).

## Verified Layers

1. A strategic scan does not pick up an entity admitted for topology-only visibility -> focused
   unit tests: build a `PlanningState` where entities carry seller/workstation/resource fields but
   are admitted as `PublicTopology`, and assert the strategic scan helpers do **not** return their
   place.
2. Legitimate strategic discovery is unchanged -> existing focused tests (`test_single_location_goal_no_travel`,
   `test_multi_location_prerequisite_then_goal`, `test_belief_only_excludes_unknown_locations`)
   continue to pass with no edits to their assertions.
3. End-to-end planning behavior is preserved -> `golden_ai` suite passes with no world-outcome
   change (the behavior-preservation contract for a defense-in-depth restriction). The golden
   layer is the correct surface here because the claim is "no observable planning difference,"
   which only the full agent decision cycle exercises; a decision-trace assertion alone would not
   prove cross-scenario neutrality.

## Landed Changes

### 1. Add a source-restricted entity accessor

Added `entities_admitted_for_physical_fields()` and `admission_exposes_physical_fields()` in
`search/strategic.rs`. The accessor yields admitted entity ids filtered by the set of admission
sources legally allowed to expose physical/economic facility fields.
The allowed-source set per field follows FND-14A and the live S157 evidence-backed planner
contract: physical/economic facility fields (`workstation_tag`, `item_lot_commodity` +
`has_sale_listing`, `resource_source`, `place_supports_commodity`) are exposable for entities
admitted via `SelfAuthoritative`, `LocalSameTickPhysical`, `GroundedEvidence`, and
`BeliefLastSeen`. `GroundedEvidence` remains allowed because existing exact evidence-backed
production planning uses evidence-carried facilities/items as the lawful target of the field read.
`PublicTopology` and `PossessionContainmentFrontier` do not expose these place/facility commodity
fields for strategic place discovery.

### 2. Route the four scans through the accessor

Replaced the raw `state.snapshot().entities.keys()` iteration in `workstation_places`,
`seller_places`, `resource_source_places`, and `acquisition_places_for_commodity` with the
source-restricted accessor. `place_supports_commodity()` now checks commodity support only through
entities admitted by that same source policy, preserving existing field predicates and
place-dedup/sort behavior.

## Landed Files

- `crates/worldwake-ai/src/search/strategic.rs` (modified — added the accessor, routed the four
  scan families, and added focused tests)
- No change: `crates/worldwake-ai/src/planning_snapshot.rs` (ticket 001's direct
  `SnapshotEntity.admission` field was sufficient)

## Out of Scope

- Recording the admission source (ticket 001 owns the enum and population).
- Trace surfacing of the source (now completed by `archive/tickets/S157SNAADMPRO-003.md`).
- Changing any action precondition, `validate_*`, candidate-emission, or goal-satisfaction logic
  — this ticket only narrows which entities the strategic *place scans* consider. No authoritative
  validation surface changes.
- Broadening the scan to new entity kinds or new fields.

## Acceptance Result

### Tests Passed

1. Focused tests assert entities carrying seller/workstation/resource fields but admitted as
   `PublicTopology` are **not** returned by the strategic place scans.
2. `test_single_location_goal_no_travel`, `test_multi_location_prerequisite_then_goal`, and
   `test_belief_only_excludes_unknown_locations` pass unchanged.
3. Existing suite passed: `cargo test -p worldwake-ai` and the golden suite
   `cargo test -p worldwake-ai --test golden_ai`

### Invariants

1. A strategic place scan reads a field only on entities whose admission source legally exposes
   that field; a topology-only entity is never returned by that scan.
2. The source restriction is a no-op for correctly-admitted entities — `golden_ai` world outcomes
   are unchanged.
3. The four scans share a single source-aware accessor; no inline per-site admission guard exists.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` (`#[cfg(test)]`) — focused non-leakage tests for
   seller, workstation, and acquisition scan families, building a `PlanningState` with a
   field-bearing but topology-only entity.

### Commands Run

1. `cargo test -p worldwake-ai strategic`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-ai --test golden_ai`

## Outcome

Completed on 2026-05-20.

- Strategic place discovery now uses one source-aware physical-field accessor instead of raw
  `PlanningSnapshot.entities.keys()` scans.
- `GroundedEvidence` remains a legal physical/economic field source because live exact
  evidence-backed production planning uses evidence-carried facilities/items as the target of the
  read.
- `PublicTopology` and `PossessionContainmentFrontier` do not expose these commodity/facility
  fields for strategic place discovery.

## Deviations

- The original negative example named `GroundedEvidence` as an unrelated source. Focused proof
  showed that excluding `GroundedEvidence` breaks existing exact evidence-backed production
  planning, so the wrong-source regression tests use `PublicTopology` instead.
- `scripts/verify.sh` was not run in this ticket iteration; the `implement-spec-tickets` harness
  owns that full pre-push gate after the S157 ticket queue finishes.

## Verification Result

- Passed `cargo test -p worldwake-ai strategic`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-ai --test golden_ai`
- Waived `scripts/verify.sh` for this ticket iteration because the harness finalization step owns
  the full pre-push gate after all S157 tickets land.
