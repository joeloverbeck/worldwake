# E16CINSBELRECCON-007: Atomic Record Mutation at the Institutional Transaction Boundary

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend authoritative institutional mutation wrappers and update affected callers/fixtures
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-002, E16CINSBELRECCON-004

## Problem

When authoritative institutional truth changes, the same transaction that mutates that truth must append or supersede the corresponding record entry. The live code centralizes authoritative office/support mutation in `worldwake_core::WorldTxn`; if record writes stay in selected `worldwake-systems` handlers, any other caller can still mutate truth without updating records. That leaves records as stale partial mirrors instead of first-class world artifacts, violating E16c §8 and Principle 25.

## Assumption Reassessment (2026-03-21)

1. `crates/worldwake-systems/src/office_actions.rs` does not own support-state mutation logic; `commit_declare_support()` validates context and delegates to `WorldTxn::declare_support()`. `crates/worldwake-systems/src/offices.rs` similarly delegates office-holder mutation to `WorldTxn::assign_office()` / `WorldTxn::vacate_office()` from `install_office_holder()` and vacancy activation.
2. E16c still requires atomic record mutation with authoritative truth changes. That requirement is architectural, but the live ownership point is the transaction boundary in `crates/worldwake-core/src/world_txn.rs`, not the outer action/system handlers.
3. Record primitives already exist in `worldwake-core`: `RecordData`, `RecordKind`, `InstitutionalClaim`, `WorldTxn::append_record_entry()`, and `WorldTxn::supersede_record_entry()`. The missing piece is automatic coupling between institutional truth mutation and those record helpers.
4. The ticket's original assumption that relevant records "must already exist" is incomplete. Current code does not automatically provision `OfficeRegister` / `SupportLedger` entities for newly created political places or offices. Only ad hoc tests currently create records. Implementing strict atomic updates therefore also requires updating the relevant political test fixtures/harness setup to create the expected records explicitly.
5. There is currently no shared helper that resolves "the record of kind K at place P". The lookup surface available today is `World::query_record_data()` plus `RecordData.home_place` / `RecordData.record_kind`.
6. Closure boundary is authoritative office-holder mutation (`assign_office`, `vacate_office`) and support mutation (`declare_support`). `clear_support_declarations_for_office()` remains a related cleanup path but does not itself create a new public support declaration claim; reassess during implementation whether it should stay out of scope for this ticket.
7. This is not a planner ticket, but affected runtime callers include world/system/action fixtures and the AI golden harness paths that currently call `txn.assign_office()` / `txn.declare_support()` directly.
8. Mismatch corrected: the primary file to change is `crates/worldwake-core/src/world_txn.rs`, with secondary fixture/harness updates in callers that now need explicit record provisioning.

## Architecture Check

1. The clean architecture is to bind record mutation to the same `WorldTxn` wrapper that mutates authoritative institutional truth. That is the narrowest enforcement point that covers all current callers without duplicating lookup/write logic across systems, actions, tests, and future legal/faction flows.
2. Keeping record writes in `office_actions.rs` and `offices.rs` would be weaker than the current architecture needs. Those modules are policy/orchestration layers, not the unique truth-mutation boundary. Duplicating record logic there would invite future drift the first time another caller uses `WorldTxn` directly.
3. Lazy record reconciliation remains architecturally wrong because it turns durable records into derived caches.
4. Lazy record auto-creation during support/office mutation is also weaker than explicit provisioning because it hides issuer/place policy and blurs record ownership. The ticket should prefer explicit record provisioning in bootstrap/fixture setup and a hard failure when required records are missing.
5. No backward-compatibility shims, alias paths, or duplicate mutation APIs.

## Verification Layers

1. `WorldTxn::assign_office()` / `WorldTxn::vacate_office()` mutate authoritative office-holder truth and append/supersede `OfficeHolder` claims -> focused `worldwake-core` transaction tests inspect `RecordData` and relation deltas after one commit.
2. `WorldTxn::declare_support()` mutates authoritative support truth and append/supersede `SupportDeclaration` claims -> focused `worldwake-core` transaction tests inspect `RecordData`, `support_declaration()`, and `active_entries()`.
3. Action path still uses the transaction-boundary behavior -> `office_actions` focused test verifies `declare_support` commit updates both authoritative support state and the support ledger in the same action commit.
4. Succession path still uses the transaction-boundary behavior -> `offices` focused tests verify vacancy/install flows mutate authoritative office state and the office register through the system path.
5. Missing record provisioning fails fast rather than silently drifting -> focused runtime/transaction test asserts `WorldError::InvalidOperation` on authoritative mutation without the required record.

## What to Change

### 1. Extend authoritative mutation wrappers in `world_txn.rs`

- Add a shared record lookup helper at the transaction boundary that resolves a record entity by `RecordKind` + `home_place`.
- `assign_office()` must append or supersede an `InstitutionalClaim::OfficeHolder { holder: Some(...) }` entry in the matching `OfficeRegister`.
- `vacate_office()` must append or supersede an `InstitutionalClaim::OfficeHolder { holder: None }` entry in the matching `OfficeRegister`.
- The lookup should use the office's jurisdiction place from `OfficeData`, not a system-local assumption.
- If the required record is missing, return a hard `WorldError::InvalidOperation` instead of silently succeeding.

### 2. Extend support mutation wrapper in `world_txn.rs`

- `declare_support()` must append or supersede an `InstitutionalClaim::SupportDeclaration` entry in the `SupportLedger` at the office jurisdiction place.
- Supersession key is the current active entry for the `(supporter, office)` pair in that ledger.
- Missing ledger is a hard error, not a silent no-op.

### 3. Update affected setup paths to provision required records explicitly

- Update the focused `worldwake-systems` fixtures and the AI golden harness office seeding path so political scenarios create the required `OfficeRegister` / `SupportLedger` records for the jurisdiction place they exercise.
- Do not add hidden auto-create behavior inside `assign_office()` / `declare_support()`.

## Files to Touch

- `crates/worldwake-core/src/world_txn.rs` (modify — authoritative record lookup + atomic office/support record mutation)
- `crates/worldwake-systems/src/office_actions.rs` (tests/fixtures only unless implementation reveals a real action-layer gap)
- `crates/worldwake-systems/src/offices.rs` (tests/fixtures only unless implementation reveals a real system-layer gap)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (or equivalent political harness helper) to provision required records for office scenarios
- Any additional focused test fixture files that currently use `txn.assign_office()` / `txn.declare_support()` without creating records first

## Out of Scope

- Faction roster mutation (`add_member` / `remove_member`) remains deferred; this ticket closes only the office/support paths exercised by current E16c deliverables and callers.
- ConsultRecord action (ticket -005)
- Perception projection (ticket -006)
- AI reading records (Phase B2 tickets)
- E16b force-claim record mutations (deferred to E16b)
- Global/bootstrap record policy beyond the political places exercised by current tests and harnesses

## Acceptance Criteria

### Tests That Must Pass

1. After `WorldTxn::assign_office()`, the jurisdiction-place `OfficeRegister` contains an active `OfficeHolder` entry for the new holder.
2. After `WorldTxn::vacate_office()`, that same register contains an active `OfficeHolder { holder: None }` entry, superseding the prior holder entry.
3. After support re-assignment through `WorldTxn::declare_support()`, the jurisdiction-place `SupportLedger` contains exactly one active entry for `(supporter, office)` and the new entry supersedes the old one.
4. Record mutation happens in the same `WorldTxn` commit as the authoritative state mutation; no reconciliation tick or second transaction is required.
5. Calling `assign_office()`, `vacate_office()`, or `declare_support()` without the required record present fails with an explicit error.
6. Existing focused system/action paths still pass after their fixtures explicitly provision records.

### Invariants

1. Records are never regenerated from authoritative state; they are appended/superseded only at the authoritative transaction boundary.
2. Record mutations are atomic with authoritative mutations in the same `WorldTxn`.
3. `RecordData.active_entries()` remains the source for current record truth within a record artifact.
4. Missing record infrastructure is an explicit setup error, not a silent degradation path.

## Tests

### New/Modified Tests and Rationale

1. `crates/worldwake-core/src/world_txn.rs` — add focused tests for `assign_office()` / `vacate_office()` record mutation and missing-record failure.
Rationale: proves the actual authoritative mutation boundary owns record writes and fast-fails on missing setup.
2. `crates/worldwake-core/src/world_txn.rs` — add focused tests for `declare_support()` record append/supersede behavior.
Rationale: proves supersession semantics for the live support mutation API rather than only action-layer orchestration.
3. `crates/worldwake-systems/src/office_actions.rs` — strengthen `declare_support_commit_sets_and_overwrites_support_declaration`.
Rationale: proves the real action commit path still updates both authoritative support state and the support ledger after the boundary shift.
4. `crates/worldwake-systems/src/offices.rs` — strengthen vacancy/succession tests to assert register entries and explicit record provisioning.
Rationale: proves the real succession system path uses the same atomic boundary and that fixtures model the required record artifacts.
5. `crates/worldwake-ai/tests/golden_harness/mod.rs` and any affected golden tests — provision records in office scenarios if needed.
Rationale: preserves the political golden path under the stricter invariant that records must exist before office/support mutation.

### Commands

1. `cargo test -p worldwake-core world_txn::tests::support_declaration_wrappers_record_add_overwrite_and_clear_deltas`
2. `cargo test -p worldwake-core world_txn::tests::office_assignment_records_register_entries_atomically`
3. `cargo test -p worldwake-systems office_actions::tests::declare_support_commit_sets_and_overwrites_support_declaration`
4. `cargo test -p worldwake-systems offices::tests::vacancy_activation_sets_vacancy_since_clears_relation_and_emits_visible_event`
5. `cargo test -p worldwake-systems offices::tests::support_succession_installs_unique_top_supported_candidate_and_clears_declarations`
6. `cargo test -p worldwake-ai golden_simple_office_claim_via_declare_support`
7. `cargo clippy --workspace`
8. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- What changed:
  - Moved record ownership to `crates/worldwake-core/src/world_txn.rs`, so `assign_office()`, `vacate_office()`, and `declare_support()` now append or supersede `OfficeRegister` / `SupportLedger` entries atomically.
  - Added transaction-boundary record lookup and active-entry supersession helpers keyed by `RecordKind` + `home_place`.
  - Added a raw authoritative office-holder read in `crates/worldwake-core/src/world/social.rs` so vacancy activation after holder death still emits the correct relation/record transition instead of being masked by the live-holder convenience accessor.
  - Updated focused system/sim/AI test fixtures and the golden office harness to provision institutional records explicitly where political mutations are exercised.
- Deviations from original plan:
  - The original handler-scoped plan was not implemented. The final change is anchored in `WorldTxn`, which is the actual authoritative mutation boundary.
  - Non-political office assignments that lack `OfficeData` were left as generic office-control relations and do not force record writes. This avoids breaking unrelated office-holder usage while still enforcing the institutional path exercised by E16c.
- Verification results:
  - Focused tests passed for `worldwake-core`, `worldwake-systems`, `worldwake-sim`, and the affected AI golden office scenario.
  - `cargo test -p worldwake-core` passed.
  - `cargo test -p worldwake-sim` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo test -p worldwake-ai golden_simple_office_claim_via_declare_support` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
