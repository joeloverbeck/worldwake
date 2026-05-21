# S162BELVIESOU-006: Lawful believed record and office snapshots

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — believed institutional snapshot substrate in `worldwake-core` / `worldwake-sim`, plus AI consumers that currently need whole `RecordData` / `OfficeData` semantics.
**Deps**: `archive/tickets/S162BELVIESOU-003.md`, Spec `specs/S162-belief-view-source-gate-hardening.md` (D2 open implementation question), `tickets/S162BELVIESOU-005.md` consumes this for office/record carrier-positive goldens.

## Problem

`archive/tickets/S162BELVIESOU-003.md` correctly closed the leak by making
`PerAgentBeliefView::record_data` and `PerAgentBeliefView::office_data` return
`None` when no lawful whole-record/office belief snapshot exists. That fail-closed
behavior removed current-truth leakage, but it also left legitimate downstream
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
2. The active spec already names this open implementation question:
   `BelievedOfficeData` / `BelievedRecordData` snapshot types may be needed when
   consumers require more than the normalized institutional belief accessors can
   carry.
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

## Verification Layers

1. Snapshot acquisition/projection -> focused core/sim tests showing record/office
   metadata enters the actor belief store only through a lawful carrier.
2. `record_data` / `office_data` positive and negative reads -> focused
   `per_agent_belief_view` tests for absent carrier, present carrier, stale carrier,
   and conflicting carrier behavior if supported by the chosen data model.
3. Consult-record duration and reward-source recovery -> focused sim tests proving
   those helpers remain `None` without the snapshot and return values with it.
4. AI candidate/ranking/HTN consumer recovery -> focused `worldwake-ai` tests for the
   exact `GoalKind` and consumer surfaces that need whole-record/office metadata.
5. S162BELVIESOU-005 may then add the office/record carrier-positive adversarial
   goldens; this ticket does not own the full golden matrix.

## What to Change

### 1. Add believed whole-record/office snapshot carrier

Design and implement the minimal `BelievedRecordData` / `BelievedOfficeData` or
equivalent snapshot substrate needed to carry whole-record/office metadata lawfully.
The carrier must include enough source/freshness/provenance to distinguish direct
consultation, testimony, local public-record observation, and stale/unknown cases.

### 2. Populate snapshots through lawful acquisition paths

Wire the carrier through existing record consultation / institutional belief
projection paths or the narrowest lawful acquisition seam found during
reassessment. Do not infer remote record/office data from entity existence or
co-location unless the specific field is a lawful directly perceivable physical
fact under FND-14A.

### 3. Re-enable consumers through the snapshot

Update `PerAgentBeliefView::record_data` / `office_data` and the consumers named in
Assumption Reassessment 5 so positive-carrier cases work again while no-carrier
cases continue to fail closed.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` or adjacent belief value modules (modify — snapshot value/storage if needed)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — positive carrier reads and focused tests)
- `crates/worldwake-sim/src/belief_view.rs` (modify — helper expectations and shared trait surfaces if needed)
- `crates/worldwake-ai/src/*` (modify — exact consumers only after reassessment)

## Out of Scope

- Reopening `knows_entity` / co-location / last-seen access to whole `RecordData` or
  `OfficeData`.
- The adversarial end-to-end golden matrix itself (S162BELVIESOU-005).
- Snapshot-through-view structural guard (S162BELVIESOU-004).

## Acceptance Criteria

### Tests That Must Pass

1. New: `record_data` / `office_data` remain `None` without a lawful snapshot carrier.
2. New: `record_data` / `office_data` return believed snapshot data after lawful
   acquisition, with no direct authoritative fallback.
3. New: consult-record duration and reward-source helper tests cover no-carrier and
   positive-carrier cases.
4. New/updated: AI consumer tests prove the selected `GoalKind` surfaces recover only
   when the lawful snapshot exists.
5. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-ai htn`
   or a stronger AI command justified by the touched consumers.

### Invariants

1. Whole-record/office metadata is planner-visible only through belief-backed
   snapshot state with source/freshness/provenance.
2. No fallback reads current authoritative `RecordData` / `OfficeData` from
   `PerAgentBeliefView` when the actor lacks the snapshot.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — negative and positive
   snapshot carrier coverage.
2. `crates/worldwake-sim/src/belief_view.rs` and/or action-duration tests — consult
   duration and reward-source recovery through the lawful carrier.
3. `crates/worldwake-ai/src/...` — focused consumer coverage for exact candidate,
   ranking, goal-model, or HTN surfaces touched.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-ai htn`
4. `./scripts/verify.sh` (before PR)
