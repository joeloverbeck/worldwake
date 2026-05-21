# S162BELVIESOU-003: Institutional & social belief-gating

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-sim` belief-view institutional/social accessors (`per_agent_belief_view.rs`); possibly `worldwake-core` if a believed-institutional snapshot type is introduced (deferred decision).
**Deps**: Spec `specs/S162-belief-view-source-gate-hardening.md` (D2, D4)

## Problem

`record_data` and `office_data` return the **current** authoritative `RecordData`/
`OfficeData` once the entity is merely `knows_entity` (which includes
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

## Verification Layers

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

## What to Change

### 1. `record_data` (`:1653`) / `office_data` (`:1659`)

Replace the `get_component_record_data`/`get_component_office_data` current-truth
read with a believed-backed read: reconstruct from `believed_office_holder` /
`believed_force_controller` / `believed_support_declarations_for_office` /
`known_institutional_beliefs` where available; return `None` (the whole `Option`)
when the believed substrate does not cover the requested record/office. Never read
current authoritative truth. Trace `htn/selector.rs` and candidate-generation call
sites first and confirm each dependent candidate originates from institutional belief
or is correctly absent.

### 2. `loyalty_to` (`:1695`) / `stock_storage_policy` (`:2267`)

Replace the `knows_entity` gate with `self.believed_entity(..).is_some()` (keeping
`loyalty_to`'s existing self gate on subject), mirroring `merchandise_profile`.

### 3. Focused unit coverage

Add tests for remote-invisible and believed-surfaced cases per accessor.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — four accessors + `#[cfg(test)]` tests)
- `Likely: crates/worldwake-ai/src/htn/selector.rs` (modify — only if a method precondition relied on the now-`None` record/office read and needs to read believed-institutional state instead; confirm via `grep -n "record_data\|office_data" crates/worldwake-ai/src/htn/selector.rs` during reassessment)

## Out of Scope

- A new `BelievedOfficeData`/`BelievedRecordData` snapshot type and any consulted-record
  perception write that populates it — deferred follow-up (Assumption Reassessment 13).
- Contention gates (S162BELVIESOU-001), control/rights gates (S162BELVIESOU-002).
- Adversarial end-to-end goldens (S162BELVIESOU-005).

## Acceptance Criteria

### Tests That Must Pass

1. New: `office_data`/`record_data` return `None` for an office/record the actor has no institutional belief about, despite live world data.
2. New: `loyalty_to`/`stock_storage_policy` return `None` for a `knows_entity`-but-not-`believed_entity` target/facility.
3. New: a believed office holder is reflected through `office_data` (or the accessor returns `None` for uncovered fields — never current truth).
4. Existing suite: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai htn`

### Invariants

1. No belief-facing institutional/social accessor reads current authoritative `RecordData`/`OfficeData`/loyalty/storage-policy on mere `knows_entity`.
2. No parallel "fall back to current truth when belief absent" path exists (FND-28).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (`#[cfg(test)]`) — remote-invisible + believed-surfaced cases for the four accessors; rationale: prove the institutional/social facts are belief-gated.
2. `crates/worldwake-ai/src/htn/selector.rs` (`#[cfg(test)]`, only if touched) — method selection unaffected by a remote record/office change absent a carrier.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-ai htn`
3. `./scripts/verify.sh` (before PR)
