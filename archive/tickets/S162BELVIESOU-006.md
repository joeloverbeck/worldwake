# S162BELVIESOU-006: Lawful believed record and office snapshots

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — believed institutional snapshot substrate in `worldwake-core` / `worldwake-sim`, plus AI consumers that currently need whole `RecordData` / `OfficeData` semantics.
**Deps**: `archive/tickets/S162BELVIESOU-003.md`, Spec `specs/S162-belief-view-source-gate-hardening.md` (D2 open implementation question resolved by this ticket), `tickets/S162BELVIESOU-005.md` consumes this for office/record carrier-positive goldens.

## Problem

Before this ticket, the following gap remained after
`archive/tickets/S162BELVIESOU-003.md`:

`archive/tickets/S162BELVIESOU-003.md` correctly closed the leak by making
`PerAgentBeliefView::record_data` and `PerAgentBeliefView::office_data` return
`None` when no lawful whole-record/office belief snapshot exists. That fail-closed
behavior removed live-truth leakage, but it also left legitimate downstream
planner-visible uses without a lawful carrier for whole-record/office metadata:
consult-record duration estimation, reward-source derivation for known accusation
cases, political office vacancy/succession candidate logic, and office/record
carrier-positive adversarial goldens.

## Assumption Reassessment (2026-05-21)

1. `archive/tickets/S162BELVIESOU-003.md` landed the minimal lawful gate: whole
   `record_data` / `office_data` no longer read authoritative truth from
   `PerAgentBeliefView`; normalized institutional beliefs such as
   `believed_office_holder`, `believed_force_controller`, membership, and support
   declarations remain available.
2. The active spec named this open implementation question before this ticket:
   `BelievedOfficeData` / `BelievedRecordData` snapshot types might be needed when
   consumers required more than the normalized institutional belief accessors could
   carry. This ticket resolved that question with the landed
   `BelievedOfficeDataSnapshot` / `BelievedRecordDataSnapshot` types.
3. Shared boundary under audit: the lawful information path from record consultation,
   direct local public-record observation, testimony, or institutional belief into
   planner-visible whole-record/office metadata. The canonical end state is one
   belief-backed snapshot/read surface; this ticket must not reintroduce a
   `knows_entity` or current-world fallback.
4. Intended invariant: a consumer may see whole-record/office metadata only when the
   actor has a lawful carrier for that snapshot. Without that carrier,
   `record_data` / `office_data` remain `None`; with it, consumers regain the needed
   metadata with provenance/freshness attached to the belief path.
5. Live consumers to reassess before coding include:
   `estimate_duration_from_beliefs(DurationExpr::ConsultRecord)`,
   `actor_lawful_reward_source_from_beliefs`, `candidate_generation.rs` record/office
   branches, `goal_model.rs`, `ranking.rs`, `planning_snapshot.rs`, and
   `effect_sink_hypothetical.rs`.
13. Adjacent contradiction classification: this is required follow-up work exposed by
    S162-003, not unfinished S162-003 scope. S162-003 owned closing the leak; this
    ticket owns restoring lawful positive-carrier behavior.

## Architecture Check

1. A belief-backed snapshot substrate is cleaner than partial struct reconstruction
   inside `PerAgentBeliefView` because it makes acquisition, freshness, and source
   provenance explicit and keeps FND-14B source classification at the carrier
   boundary.
2. No backwards-compatibility shim is allowed. Do not add "read from world when
   belief missing" behavior; update consumers to use the lawful snapshot or remain
   absent.

## Verified Layers

1. Snapshot storage and event-log diff projection -> focused
   `worldwake-core` `belief_store_diff_roundtrip_believed_record_and_office_snapshots`
   plus the full `worldwake-core` suite.
2. `record_data` / `office_data` positive and negative reads -> focused
   `worldwake-sim per_agent_belief_view` tests covering absent carriers,
   present believed record snapshots, and present believed office snapshots.
3. Consult-record acquisition -> focused and full `worldwake-systems` tests proving
   `consult_record` projects a believed record snapshot and, when the consulted
   record's issuer has `OfficeData`, a believed issuer-office snapshot.
4. Consult-record duration recovery -> `estimate_duration_uses_believed_record_snapshot`
   proves duration estimation remains hidden without the carrier and recovers with it.
5. AI HTN/planning regression -> `cargo test -p worldwake-ai htn` passed after the
   snapshot storage and fixture updates. The full adversarial office/record golden
   matrix remains owned by `tickets/S162BELVIESOU-005.md`.

## Landed Changes

### 1. Believed whole-record/office snapshot carrier

Added `InstitutionalSnapshotSource`, `BelievedRecordDataSnapshot`, and
`BelievedOfficeDataSnapshot` in `worldwake-core`. `AgentBeliefStore` now carries
`believed_record_data` and `believed_office_data` maps with source, learned tick,
and learned location. `BeliefStoreDiff` includes those maps so event-log component
deltas preserve the new belief-store state.

### 2. Lawful acquisition through record consultation

`consult_record` now stores a believed `RecordData` snapshot for the consulted
record. When the consulted record's issuer is an office with `OfficeData`, it also
stores a believed issuer-office snapshot with `RecordConsultation` provenance.

### 3. Snapshot-backed consumers

`PerAgentBeliefView::record_data` and `PerAgentBeliefView::office_data` now return
only the believed snapshots and still have no authoritative fallback. Existing
systems tests that intentionally expected planner-visible office/register metadata
now seed the lawful snapshot carrier instead of relying on entity belief or live
component truth. `SAVE_FORMAT_VERSION` was bumped from 98 to 99 because the
persisted `AgentBeliefStore` shape changed.

## Landed Files

- `crates/worldwake-core/src/institutional.rs` — snapshot source and value types.
- `crates/worldwake-core/src/belief.rs` — snapshot storage, accessors, diff support,
  and focused diff regression.
- `crates/worldwake-core/src/world_txn.rs` — transaction projection helpers for
  believed record and office snapshots.
- `crates/worldwake-core/src/lib.rs`, `component_tables.rs`, `delta.rs`, `world.rs`
  — re-exports and explicit sample literals for the new belief-store fields.
- `crates/worldwake-sim/src/per_agent_belief_view.rs` — snapshot-backed reads and
  focused positive/negative tests.
- `crates/worldwake-sim/src/save_load.rs` — save format version bump to 99.
- `crates/worldwake-systems/src/consult_record_actions.rs` — consult-record
  projection of the believed snapshots and producer tests.
- `crates/worldwake-systems/src/office_actions.rs`,
  `crates/worldwake-systems/src/report_actions.rs`,
  `crates/worldwake-ai/src/candidate_generation.rs` — fixture updates required by
  the new lawful carrier contract.

## Out of Scope

- Reopening `knows_entity` / co-location / last-seen access to whole `RecordData` or
  `OfficeData`.
- The adversarial end-to-end golden matrix itself (S162BELVIESOU-005).
- Snapshot-through-view structural guard (S162BELVIESOU-004).

## Acceptance Result

1. Passed: `record_data` / `office_data` remain `None` without a lawful snapshot
   carrier and return believed snapshot data only when the carrier exists.
2. Passed: consult-record projection stores record snapshots and issuer-office
   snapshots; consult-record duration estimation recovers through the believed
   record snapshot.
3. Passed: the touched AI HTN/planning surface still composes after the carrier
   migration. The end-to-end adversarial office/record golden matrix is still
   deliberately deferred to `tickets/S162BELVIESOU-005.md`.

### Invariants

1. Whole-record/office metadata is planner-visible only through belief-backed
   snapshot state with source/freshness/provenance.
2. No fallback reads current authoritative `RecordData` / `OfficeData` from
   `PerAgentBeliefView` when the actor lacks the snapshot.

## Outcome

Completed on 2026-05-21.

- Added the lawful believed whole-record/office snapshot substrate and made
  `PerAgentBeliefView::record_data` / `office_data` read only that substrate.
- Wired `consult_record` as the first lawful acquisition path for the snapshots.
- Updated same-domain systems fixtures to seed the new carrier for cases that
  intentionally expect planner-visible office/register metadata.
- Bumped the current save format to 99 for the persisted `AgentBeliefStore` shape.
- Left the adversarial end-to-end office/record golden matrix to
  `tickets/S162BELVIESOU-005.md`, as drafted.

## Deviations

- The landed source type names are `BelievedRecordDataSnapshot` and
  `BelievedOfficeDataSnapshot`, not the shorter draft names.
- No direct `crates/worldwake-sim/src/belief_view.rs` implementation edit was needed;
  its existing trait defaults and helper calls work through the concrete
  `PerAgentBeliefView` implementation.
- The reward-source path was covered by the full systems suite and the
  snapshot-backed office/register fixture fallout rather than a new standalone
  reward-source helper test.

## Verification Result

- Passed `cargo test -p worldwake-core belief_store_diff_roundtrip_believed_record_and_office_snapshots --quiet`
- Passed `cargo test -p worldwake-core --quiet`
- Passed `cargo test -p worldwake-sim per_agent_belief_view --quiet`
- Passed `cargo test -p worldwake-sim --quiet`
- Passed `cargo test -p worldwake-systems consult_record --quiet`
- Passed `cargo test -p worldwake-systems --quiet`
- Passed `cargo test -p worldwake-ai htn --quiet`
