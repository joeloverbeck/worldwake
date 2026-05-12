# S141MOTSOULED-005: `GoalCommittedPayload.decisive_motive_sources` + commit-time population

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core::GoalCommittedPayload` field extension; commit-time emission seam in `worldwake-ai`
**Deps**: `archive/tickets/S141MOTSOULED-001.md` (uses `MotiveSourceRef`), `archive/tickets/S141MOTSOULED-004.md` (reads `offer.motive_sources` at commit time)

## Problem

S141's deliverable D6 makes the post-commit causal record name the load-bearing motive sources rather than only the abstract `motive_score`. Today's `GoalCommittedPayload` at `crates/worldwake-core/src/decision_event_payload.rs:156` carries decision provenance but lacks the per-source breakdown needed for "Why did this agent commit this goal?" reconstruction from event-log history alone (FND-29A).

This ticket adds `decisive_motive_sources: Vec<MotiveSourceRef>` to the payload and wires the commit-time emission to copy `offer.motive_sources` from the just-committed `AgendaEntry`.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalCommittedPayload` lives at `crates/worldwake-core/src/decision_event_payload.rs:156` and derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` per the existing decision-payload convention at line 11. Existing `decisive_*` fields on sibling payloads (`ExpectationMismatchPayload.decisive_beliefs/decisive_records/decisive_world_observations` at lines 346–350, `SourceExpectationFailurePayload.decisive_beliefs/decisive_records/decisive_world_observations` at lines 366–370) all use `Vec<T>` — same collection type and naming convention this ticket follows. 6 `GoalCommittedPayload { ... }` construction sites exist workspace-wide (per Step 2 sub-check (d) re-check):
   - `crates/worldwake-cli/src/bin/observer.rs` (3 sites: lines 5305, 6699, 6723 — 2 are test fixtures in the observer's `#[cfg(test)]` block)
   - `crates/worldwake-core/src/decision_event_payload.rs` (2 sites: the struct definition at 156 and a test fixture at 574)
   - `crates/worldwake-ai/src/agent_tick/planning.rs` (2 sites: lines 1163 and 3940 — the live emission seam plus a test-build fixture)
   - `crates/worldwake-sim/src/save_load.rs` (1 site: line 990 — save-load round-trip fixture)
   - `crates/worldwake-ai/tests/golden_decision_payload.rs` (1 site: line 94 — golden assertion fixture)
2. The commit-time emission seam is `crates/worldwake-ai/src/agent_tick/planning.rs:1163` (the production path that emits `EventTag::GoalCommitted`). This site reads from the committing `AgendaEntry` (which carries `offer: GoalOffer` per `crates/worldwake-ai/src/agenda_types.rs:22`); after 004 lands, `entry.offer.motive_sources` is non-empty and can be cloned into `decisive_motive_sources`.
3. Shared abstraction boundary: `GoalCommittedPayload` is the always-on decision-event payload (per S136). Adding `decisive_motive_sources: Vec<MotiveSourceRef>` is purely additive; the existing payload fields remain unchanged. Per S141 spec FND-01 Section H, `MotiveSourceRef` becomes authoritative state only when embedded in this payload at commit time — the carrying `GoalOffer` (per-tick agenda entry) is not authoritative on its own.
4. Save-format: this ticket does NOT bump `SAVE_FORMAT_VERSION` further. It rides under the 77→78 bump from `archive/tickets/S141MOTSOULED-002.md` via `#[serde(default)]` on the new field (defaulting to `Vec::new()`) for omitted-field payload/current-format deserialization. Full pre-bump save files with header version 77 remain rejected by the loader after `archive/tickets/S141MOTSOULED-002.md`. The bump is single-shot per the S141 reassessment's merge note in `archive/tickets/S141MOTSOULED-002.md`.
5. Per `docs/precision-rules.md` Rule 16 (information-path refactors): post-commit motive-source provenance currently has no transport — the event payload carries `motive_score` but not the per-source decomposition. After this ticket, the event payload is the canonical transport; observer Section 3b (owned by 006) reads from it. There is no duplicate path to retire.

## Architecture Check

1. Embedding motive-source references in the always-on payload (FND-29A causal history) makes the why-of-commit reconstructible from event-log replay alone, without consulting transient ranking-tick state. This is the "history reconstructs the why across ticks" promise of S141 Section H bullet 4 (stored authoritative state vs derived read model).
2. Pattern parity: `decisive_motive_sources: Vec<MotiveSourceRef>` mirrors the existing `decisive_beliefs: Vec<BeliefRef>` / `decisive_records: Vec<RecordRef>` / `decisive_world_observations: Vec<ObservationRef>` shape (FND-26: systems interact through state with consistent payload conventions).
3. `#[serde(default)]` on the new field is the FND-28-compliant boundary shim — it lives at the deserialization boundary only and does not introduce a backward-compat path in the live authority code. The bump in `archive/tickets/S141MOTSOULED-002.md` signals forward-incompatibility; the serde default handles omitted-field current-format payloads.

## Verification Layers

1. Payload shape → focused unit test in `crates/worldwake-core/src/decision_event_payload.rs#[cfg(test)]` asserting `GoalCommittedPayload::default().decisive_motive_sources.is_empty()` and that a populated payload round-trips through bincode unchanged.
2. Commit-time emission → action trace + decision-event-log delta. Verify in `crates/worldwake-ai/tests/golden_decision_payload.rs` that a committed goal emits a `GoalCommitted` event whose `decisive_motive_sources` matches the committing `offer.motive_sources`.
3. Save-load compatibility → focused integration test in `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` round-tripping an event log containing a `GoalCommitted` event with non-empty `decisive_motive_sources` under version 78.
4. Omitted-field deserialization -> a pre-S141 `GoalCommittedPayload` payload shape loads in the current save format with `decisive_motive_sources` populated from `#[serde(default)]` (empty `Vec`). Full version-77 save files remain rejected after `archive/tickets/S141MOTSOULED-002.md`.
5. Per `docs/precision-rules.md` Rule 5 (verification surface mapping): the immediate proof is at the event-log delta layer (payload contents observable in the event-log diff). Observer rendering (006) is the downstream surface; this ticket's verification stops at the payload layer.

## What to Change

### 1. Extend `GoalCommittedPayload` struct

At `crates/worldwake-core/src/decision_event_payload.rs:156` add:

```rust
pub struct GoalCommittedPayload {
    // existing fields preserved
    #[serde(default)]
    pub decisive_motive_sources: Vec<MotiveSourceRef>,
}
```

Insert `use crate::motive_source::MotiveSourceRef;` near the top of the file (or rely on the crate-root re-export established by `archive/tickets/S141MOTSOULED-001.md`).

### 2. Populate at the commit-time emission seam

In `crates/worldwake-ai/src/agent_tick/planning.rs` (production path at line 1163), where `GoalCommittedPayload { ... }` is constructed from the committing `AgendaEntry`, attach `decisive_motive_sources: entry.offer.motive_sources.clone()`. Lifecycle: the agenda entry's offer carries the motive sources populated by `archive/tickets/S141MOTSOULED-004.md`'s `derive_default_motive_sources`; this ticket only copies them into the event payload at the commit boundary.

### 3. Update the 5 remaining `GoalCommittedPayload { ... }` construction sites

- `crates/worldwake-core/src/decision_event_payload.rs:574` (test fixture) — add `decisive_motive_sources: Vec::new()` or a representative non-empty vec for the round-trip test.
- `crates/worldwake-sim/src/save_load.rs:990` (round-trip fixture) — same.
- `crates/worldwake-cli/src/bin/observer.rs:5305, 6699, 6723` (test fixtures) — same; non-empty vec where the test asserts rendering content.
- `crates/worldwake-ai/src/agent_tick/planning.rs:3940` (test-build fixture) — same.
- `crates/worldwake-ai/tests/golden_decision_payload.rs:94` (golden assertion fixture) — extend the golden assertion to expect the new field; this is the primary cross-system proof surface.

### 4. Extend `golden_decision_payload.rs` assertions

Update the existing golden's assertion to verify that `GoalCommitted` events carry non-empty `decisive_motive_sources` whose entries match the committing offer's `motive_sources`. This is the cross-layer proof that the commit-time emission seam is wired correctly.

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — field + struct test fixture)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — emission seam at line 1163 + test fixture at line 3940)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — 3 test fixtures at lines 5305, 6699, 6723)
- `crates/worldwake-sim/src/save_load.rs` (modify — round-trip fixture at line 990; does NOT bump `SAVE_FORMAT_VERSION` further)
- `crates/worldwake-ai/tests/golden_decision_payload.rs` (modify — fixture at line 94 + extended assertion)

## Out of Scope

- `SAVE_FORMAT_VERSION` bump — owned by `archive/tickets/S141MOTSOULED-002.md` (single-shot bump 77→78 covering all S141 serialized-state changes via `#[serde(default)]`).
- Observer rendering of `decisive_motive_sources` — owned by 006.
- `MotiveSourceRef` type definition — owned by `archive/tickets/S141MOTSOULED-001.md`.
- `GoalOffer.motive_sources` field and production population — owned by `archive/tickets/S141MOTSOULED-004.md` (must land first; this ticket only consumes `offer.motive_sources` at commit time).
- Conformance test that every emitted `GoalCommitted` event carries non-empty `decisive_motive_sources` — partially overlaps with 007's conformance suite; this ticket's golden assertion is per-scenario, while 007's is workspace-wide.

## Acceptance Criteria

### Tests That Must Pass

1. `GoalCommittedPayload::default().decisive_motive_sources.is_empty()` — focused unit assertion.
2. `GoalCommittedPayload` bincode round-trip preserves a populated `decisive_motive_sources` vec exactly.
3. Omitted-field deserialization: a pre-S141 `GoalCommittedPayload` payload shape deserializes with `decisive_motive_sources` defaulted to an empty vec via `#[serde(default)]` when embedded in the current save format. Full version-77 save files remain rejected by the save header gate after `archive/tickets/S141MOTSOULED-002.md`.
4. Golden `golden_decision_payload.rs`: a scenario that commits a goal produces a `GoalCommitted` event whose `decisive_motive_sources` matches the committing offer's `motive_sources` element-for-element (insertion-ordered).
5. Existing suite: `cargo test --workspace`.

### Invariants

1. The field type is exactly `Vec<MotiveSourceRef>` — matches the spec D6 prose and the existing `decisive_*: Vec<T>` convention in `decision_event_payload.rs:346-350`.
2. The commit-time emission seam at `crates/worldwake-ai/src/agent_tick/planning.rs:1163` populates `decisive_motive_sources` from the committing `AgendaEntry.offer.motive_sources` — no separate motive-source derivation at commit time. FND-3: the payload reflects authoritative state, not a re-derived view.
3. No `SAVE_FORMAT_VERSION` bump in this ticket — confirms the single-shot bump invariant from `archive/tickets/S141MOTSOULED-002.md`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs#[cfg(test)]` — extend the existing payload round-trip test to cover the added field; add an omitted-field payload deserialization test under the current save format.
2. `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` — extend the event-log round-trip test to include `decisive_motive_sources`.
3. `crates/worldwake-ai/tests/golden_decision_payload.rs` — extend the existing golden assertion to verify `decisive_motive_sources` matches the committing offer's `motive_sources` for at least one committed goal in the scenario.

### Commands

1. `cargo test -p worldwake-core decision_event_payload`
2. `cargo test -p worldwake-sim save_load`
3. `cargo test -p worldwake-ai --test golden_decision_payload`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
