# S162BELVIESOU-003: Institutional & social belief-gating

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-sim` belief-view institutional/social accessors (`per_agent_belief_view.rs`) plus focused lower-layer reward-source expectations in `belief_view.rs`. No believed-institutional snapshot type was introduced.
**Deps**: Spec `specs/S162-belief-view-source-gate-hardening.md` (D2, D4)

## Problem

Before this ticket, `record_data` and `office_data` returned the live
authoritative `RecordData`/`OfficeData` once the entity is merely `knows_entity` (which includes
co-location/last-seen), making institutional facts (holder, vacancy, jurisdiction,
succession, penalties, bounties) omniscient. `loyalty_to` and `stock_storage_policy`
gate on `knows_entity` rather than an explicit belief. All four are social/
institutional facts that require a belief, consulted record, or institutional-belief
entry even when co-located (FND-14, FND-14A, FND-17).

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Verified in `crates/worldwake-sim/src/per_agent_belief_view.rs` (2026-05-21):
   `record_data` (`:1653-1657`) returns `world.get_component_record_data(record)`
   gated on `entity_kind(record) == Record` (which resolves through `knows_entity`);
   `office_data` (`:1659-1663`) is the parallel case for `OfficeData`; `loyalty_to`
   (`:1695-1704`) is self-gated on subject but uses `knows_entity(target)`;
   `stock_storage_policy` (`:2267-2275`) gates on `knows_entity(facility)`.
2. Believed-institutional substrate that already exists (verified in
   `crates/worldwake-sim/src/belief_view.rs`, 2026-05-21): `believed_office_holder`
   (`:929`), `believed_force_controller` (`:758`),
   `believed_support_declarations_for_office` (`:796`), `known_institutional_beliefs`
   (`:393`). These cover office *holder*, *force controller*, and *support
   declarations* — **not** `OfficeData`'s jurisdiction/succession/vacancy/reward-policy
   fields, and there is no consulted-record snapshot for `RecordData`. Spec D2 and the
   `docs/spec-drafting-rules.md` source-class rule govern the corrected contract.
3. Shared boundary under audit: the institutional/social accessor surface in
   `per_agent_belief_view.rs`. Only this file holds world-reading impls;
   `belief_view.rs` `record_data`/`office_data` are safe trait defaults returning
   `None` (`:750/:754`, `:1507/:1511`) and a forwarding shim (`:2500`) — no
   `belief_view.rs` edit needed.
4. Intended invariant: a remote record/office/loyalty/policy change with no lawful
   carrier (consult, testimony, institutional belief) must not change these
   accessors; a candidate depending on the changed fact must be absent until a
   carrier updates belief.
5. Live consumers to trace before edit: `record_data`/`office_data` feed candidate
   generation and HTN method selection (the third-iteration report noted
   `htn/selector.rs` reads `record_data`). Grep their call sites; after the fix a
   bounty/claim/investigation candidate that depended on live record/office truth must
   originate from institutional belief or be correctly absent. This is the intended
   FND-14B behavior.
8. Heuristic being removed: the `knows_entity`-gated current-truth read. The substrate
   it stands in for is the institutional-belief store. Because `office_data`/
   `record_data` return whole `Option<T>` structs, the minimal lawful fix is to return
   `None` unless the believed substrate covers the read — never current truth. This
   does not reopen regressions: candidates that lawfully depend on consulted/believed
   institutional facts still fire.
13. Adjacent contradictions (classified as **future cleanup, its own follow-up**): if
    consumers need richer believed-institutional data than holder/force-controller/
    support (e.g., believed jurisdiction or vacancy for a claim candidate), introducing
    a `BelievedOfficeData`/`BelievedRecordData` snapshot type (populated on lawful
    consult/perception) is a separate spec/ticket. This ticket's committed scope is the
    minimal lawful gate (return `None` unless believed); do not expand it into a new
    institutional-belief system. Record the follow-up explicitly if reassessment shows
    a current consumer is left without its data.

## Architecture Check

1. Routing `office_data`/`record_data` through the believed-institutional substrate
   (returning `None` when not believed) makes institutional facts travel through
   lawful carriers (FND-15) instead of being read from current truth on mere entity
   knowledge. `loyalty_to`/`stock_storage_policy` gating on `believed_entity` matches
   the already-lawful `merchandise_profile` precedent (`:2191`, gated on
   `self || believed_entity`). The minimal-`None` approach avoids inventing a new
   substrate while still closing the leak; the richer snapshot type is deferred per
   YAGNI until a consumer demonstrably needs it.
2. No backwards-compatibility shim: the current-truth reads are replaced outright
   (FND-28); no parallel "read truth if belief absent" fallback remains.

## Verified Layers

1. Remote record/office change invisible without consult -> focused unit test: actor
   has no institutional belief for the office/record; `office_data`/`record_data`
   return `None` despite live world data.
2. Believed institutional fact still surfaced -> focused unit test: seed an
   institutional belief (e.g., believed office holder) and assert the accessor
   reflects it (or `None` for fields the substrate doesn't cover, never current
   truth).
3. `loyalty_to`/`stock_storage_policy` return `None` for `knows_entity`-but-not-
   `believed_entity` targets -> focused unit test.
4. Candidate/HTN-method consequence (a record/office change does not alter candidate
   emission or method selection without a carrier) -> deferred to S162BELVIESOU-005
   goldens; this ticket's proof is the focused accessor tests (strongest lower layer).

## Landed Changes

### 1. `record_data` / `office_data`

`PerAgentBeliefView::record_data` and `PerAgentBeliefView::office_data` now return
`None` instead of reading current authoritative `RecordData` / `OfficeData` from the
world. The existing belief substrate exposes normalized institutional beliefs such
as office holder, force controller, membership, and support declarations, but it
does not carry a whole believed record/office snapshot. The landed result therefore
fails closed instead of reconstructing partial structs or falling back to live truth.

### 2. `loyalty_to` / `stock_storage_policy`

`loyalty_to` keeps its self-subject gate and now requires an explicit target entity
belief. `stock_storage_policy` now requires an explicit facility entity belief.
Neither accessor treats mere co-location or last-seen/entity knowledge as permission
to read social/institutional policy truth.

### 3. Focused unit coverage and dependent fail-closed tests

Added focused `per_agent_belief_view` tests proving that live record/office data,
co-located loyalty, and co-located stock policy stay hidden without explicit belief.
Updated lower-layer `belief_view` reward-source tests and consultation-duration
coverage to record the same fail-closed behavior when a whole believed record/office
snapshot is absent.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs` — modified the four owned accessors and focused tests.
- `crates/worldwake-sim/src/belief_view.rs` — updated reward-source helper expectations that previously depended on the same live `record_data` / `office_data` path.
- `crates/worldwake-ai/src/htn/selector.rs` — no change; the `cargo test -p worldwake-ai htn` proof passed with the fail-closed accessor result.

## Out of Scope

- A new `BelievedOfficeData`/`BelievedRecordData` snapshot type and any consulted-record
  perception write that populates it — deferred follow-up (Assumption Reassessment 13).
- Contention gates (S162BELVIESOU-001), control/rights gates (`archive/tickets/S162BELVIESOU-002.md`).
- Adversarial end-to-end goldens (S162BELVIESOU-005).
- Restoring planner-visible consult duration, bounty reward-source derivation, or
  richer office/record candidates through a lawful whole-record/office belief
  carrier. Those are the same deferred believed snapshot substrate, not an
  authorized fallback to current truth.

## Acceptance Result

### Focused Results

1. `office_data` / `record_data` return `None` for believed office/record entities despite live world data.
2. `loyalty_to` / `stock_storage_policy` return `None` for co-located but not explicitly believed target/facility entities.
3. Explicit institutional beliefs such as `believed_office_holder` still surface through their normalized belief accessors; whole `office_data` remains `None` because no lawful whole-office snapshot exists.
4. Existing suites passed: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai htn`.

### Verified Invariants

1. No belief-facing institutional/social accessor reads current authoritative `RecordData`/`OfficeData`/loyalty/storage-policy on mere `knows_entity`.
2. No parallel "fall back to current truth when belief absent" path exists (FND-28).

## Outcome

Completed on 2026-05-21.

- Closed the live `PerAgentBeliefView` institutional/social truth leak for `record_data`, `office_data`, `loyalty_to`, and `stock_storage_policy`.
- Preserved normalized institutional belief accessors (`believed_office_holder`, membership, support declarations, force controller) as the lawful carrier surface.
- Confirmed HTN selector coverage passes without modifying `worldwake-ai`.
- Kept the richer `BelievedOfficeData` / `BelievedRecordData` substrate deferred; dependent reads now fail closed instead of using current truth.

## Deviations

- The ticket's drafted "believed office holder is reflected through `office_data`" acceptance branch resolved to the alternative already allowed by the ticket: `office_data` returns `None` for uncovered whole-struct fields, while `believed_office_holder` remains the lawful normalized accessor.
- `TemporalBeliefView::estimate_duration` for `DurationExpr::ConsultRecord` and `actor_lawful_reward_source_for_case` no longer derive values through `record_data` / `office_data` when no believed whole-record/office snapshot exists. This is intentional fail-closed behavior for S162-003, not a new live-truth fallback.

## Verification Result

- Passed `cargo test -p worldwake-sim per_agent_belief_view`
- Passed `cargo test -p worldwake-ai htn`
- Passed `cargo test -p worldwake-sim`
