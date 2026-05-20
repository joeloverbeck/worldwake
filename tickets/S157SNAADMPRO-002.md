# S157SNAADMPRO-002: Source-restricted strategic scans

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` strategic plan search (`search/strategic.rs`)
**Deps**: S157SNAADMPRO-001

## Problem

Strategic search scans the admitted entity map directly — `state.snapshot().entities.keys()` —
to discover workstations, sellers, resource sources, and acquisition places
(`crates/worldwake-ai/src/search/strategic.rs:902,914,923,956`). That raw scan is only sound if
snapshot admission is airtight: nothing stops a scan from reading, say, a seller's
`item_lot_commodity`/`has_sale_listing` on an entity that was admitted purely as an evidence
carrier or topology place. Now that ticket 001 records why each entity was admitted, this ticket
restricts the strategic scans to entities whose admission source legitimately exposes the field
being read, so the scan cannot consume a field admitted for an unrelated reason (S157 D2; FND-7,
FND-14A, FND-20). This is defense-in-depth: post-S155 the snapshot is built from a belief-correct
view, so the restriction is expected to be behavior-preserving — proven by the golden suite.

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

## Verification Layers

1. A strategic scan does not pick up an entity admitted for an unrelated reason -> focused unit
   test: build a `PlanningState` where an entity carries a seller-relevant field but was admitted
   as `GroundedEvidence`/`PublicTopology`, and assert `seller_places`/`workstation_places` does
   **not** return its place.
2. Legitimate strategic discovery is unchanged -> existing focused tests (`test_single_location_goal_no_travel`,
   `test_multi_location_prerequisite_then_goal`, `test_belief_only_excludes_unknown_locations`)
   continue to pass with no edits to their assertions.
3. End-to-end planning behavior is preserved -> `golden_ai` suite passes with no world-outcome
   change (the behavior-preservation contract for a defense-in-depth restriction). The golden
   layer is the correct surface here because the claim is "no observable planning difference,"
   which only the full agent decision cycle exercises; a decision-trace assertion alone would not
   prove cross-scenario neutrality.

## What to Change

### 1. Add a source-restricted entity accessor

Add an accessor (on `PlanningState` or as a free helper in `search/strategic.rs`, matching where
`SnapshotEntity.admission` is reachable per ticket 001) that yields admitted entity ids filtered
by both a field predicate and the set of admission sources legally allowed to expose that field.
The allowed-source set per field follows FND-14A: physical/economic facility fields
(`workstation_tag`, `item_lot_commodity` + `has_sale_listing`, `resource_source`,
`place_supports_commodity`) are exposable for entities admitted via `SelfAuthoritative`,
`LocalSameTickPhysical`, `BeliefLastSeen`, `GroundedEvidence`, and `PublicTopology` (the carriers
through which a planner could lawfully know a place/facility's physical commodity state).
Determine the exact per-field allowed-source policy during implementation against the FND-14A
split; document the chosen policy inline.

### 2. Route the four scans through the accessor

Replace the raw `state.snapshot().entities.keys()` iteration in `workstation_places` (902),
`seller_places` (914), `resource_source_places` (923), and `acquisition_places_for_commodity`
(956) with the new source-restricted accessor. Preserve the existing per-function field predicates
and the `places_for_entities` place-dedup/sort behavior.

## Files to Touch

- `crates/worldwake-ai/src/search/strategic.rs` (modify — add accessor, route the four scans)
- `Likely: crates/worldwake-ai/src/planning_snapshot.rs` (modify — only if the per-id source
  reader belongs on `PlanningState`/`PlanningSnapshot` rather than in `strategic.rs`; confirm
  against ticket 001's exposed surface — `grep AdmissionSource` consumers)

## Out of Scope

- Recording the admission source (ticket 001 owns the enum and population).
- Trace surfacing of the source (ticket 003).
- Changing any action precondition, `validate_*`, candidate-emission, or goal-satisfaction logic
  — this ticket only narrows which entities the strategic *place scans* consider. No authoritative
  validation surface changes.
- Broadening the scan to new entity kinds or new fields.

## Acceptance Criteria

### Tests That Must Pass

1. A focused test asserts an entity carrying a seller-relevant field but admitted as
   `GroundedEvidence` (or `PublicTopology`) is **not** returned by `seller_places`.
2. `test_single_location_goal_no_travel`, `test_multi_location_prerequisite_then_goal`, and
   `test_belief_only_excludes_unknown_locations` pass unchanged.
3. Existing suite: `cargo test -p worldwake-ai` and the golden suite
   `cargo test -p worldwake-ai --test golden_ai`

### Invariants

1. A strategic place scan reads a field only on entities whose admission source legally exposes
   that field; an entity admitted for an unrelated reason is never returned by that scan.
2. The source restriction is a no-op for correctly-admitted entities — `golden_ai` world outcomes
   are unchanged.
3. The four scans share a single source-aware accessor; no inline per-site admission guard exists.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/strategic.rs` (`#[cfg(test)]` after line 1128) — new focused
   non-leakage test(s) per scan family, building a `PlanningState` with a field-bearing but
   wrong-source entity.

### Commands

1. `cargo test -p worldwake-ai strategic`
2. `cargo test -p worldwake-ai --test golden_ai`
3. `scripts/verify.sh`
