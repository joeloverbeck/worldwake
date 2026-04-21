# S113BELENV-002: `BeliefSnapshot` on decision-event payloads + save-format bump

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — decision-event payloads (`worldwake-core/src/decision_event_payload.rs`), save format version (`worldwake-sim/src/save_load.rs`), 14 payload construction sites across 6 files
**Deps**: S113BELENV-001

## Problem

When an agent's plan is invalidated or blocked because a belief was stale or contradicted, the event log today records *that* the invalidation happened and *what kind* of discrepancy fired, but not the envelope state at the moment of the decision. As a result, "why did the agent act on stale belief X?" cannot be answered from the event log alone — the causal history is too thin to reconstruct whether the belief's confidence was borderline, clearly stale, or actively contradicted when the planner made the call (P29, P29A).

D6 adds an optional `BeliefSnapshot` to `BlockerRecordedPayload` and `PlanInvalidatedPayload` capturing `(confidence, status, acquired_tick)` at the decision point. `BeliefStatus` lives in `worldwake-sim`; the payload types live in `worldwake-core`. To avoid a reverse dependency, this ticket introduces `BeliefStatusTag` in core as a serializable mirror of `BeliefStatus` — a historical-record tag on the append-only event log, not authoritative live state.

## Assumption Reassessment (2026-04-21)

1. `BlockerRecordedPayload` is defined at `crates/worldwake-core/src/decision_event_payload.rs:250-256` with fields `agent`, `blocker_key`, `discrepancy: Option<Discrepancy>`, `blocking_fact: Option<BlockingFact>`, `expires_tick`. `PlanInvalidatedPayload` is defined at the same file, lines 144-178, with fields `agent`, `goal_key`, `reason: PlanInvalidationReason`. Construction-site grep (`BlockerRecordedPayload {` / `PlanInvalidatedPayload {`) finds 14 sites across 6 production files: `decision_event_payload.rs` (4), `worldwake-ai/src/agent_tick/mod.rs` (1), `worldwake-ai/src/agent_tick/execution.rs` (2), `worldwake-ai/src/agent_tick/tests.rs` (2), `worldwake-cli/src/bin/observer.rs` (2), `worldwake-sim/src/save_load.rs` (2). Plus 3 matches in `archive/` (not construction sites).
2. `SAVE_FORMAT_VERSION = 34` at `crates/worldwake-sim/src/save_load.rs:6`. Serialization uses `bincode` (line 86: `bincode::serialize`, line 139: `bincode::deserialize`). **Bincode is positional — adding a field to a serialized struct breaks backward compatibility regardless of `#[serde(default)]`.** The save format must bump to 35 for this change; the old format cannot be round-tripped in place. Shared abstraction boundary under audit: the decision-event payload schema (append-only history) and the save format version contract.
3. The spec (S113) does not currently mention the save-format bump explicitly; this ticket makes it an in-scope consequence of D6. Adding it later as a separate ticket would leave an intermediate state where saves written with S113 code cannot be loaded by pre-S113 code without an error — preferable to bump in the same ticket so the version boundary and the schema change co-commit.
6. This is an event-log / save-format ticket — intended verification layer is focused unit tests (payload construction, serialization round-trip) plus `event-log delta` coverage where existing S110 tests exercise these payloads.
8. No heuristic is being removed or bypassed.
12. No existing golden scenario depends on the historical absence of `belief_snapshot`; the `None` default preserves existing test behavior.
13. The save-format bump is a required consequence of the schema change (bincode's positional serialization requires it). Not a separate bug.

## Architecture Check

1. `BeliefSnapshot` and `BeliefStatusTag` in core mirror `BeliefValue`/`BeliefStatus` in sim without creating a reverse `core → sim` dependency. The duplication is small (one enum with five variants, one three-field struct) and the alternative — hoisting `BeliefStatus` to core — would pull read-model projection types into core, which is the wrong direction since core holds stored state and sim holds read models (FND-27, workspace layering).
2. Snapshots are append-only historical records, not live state. Once written to the event log, they are never re-derived; they are the frozen view of belief at the decision moment (P29A). Recomputing at read time would rewrite history.
3. No backward-compatibility shim. `SAVE_FORMAT_VERSION` bump is the architecturally correct response to a schema change in a positionally-serialized format (P28 — no dual authority for the save format).

## Verification Layers

1. Payload construction with `Some(BeliefSnapshot { .. })` and `None` → focused unit tests in `decision_event_payload.rs` `#[cfg(test)]`.
2. Serialization round-trip through `bincode::serialize` / `bincode::deserialize` with the new field → focused unit test in `save_load.rs`.
3. `SAVE_FORMAT_VERSION = 35` asserted at compile time + load test that rejects version-34 payloads cleanly (existing `save_load.rs` version-mismatch path is the proof surface) → event-log-delta / save-load layer.
4. Pre-existing observer display of payloads is not broken by the new optional field → focused runtime test in `crates/worldwake-cli/src/bin/observer.rs` tests if present, otherwise visual confirmation via a trace run (noted as follow-up if no automated coverage exists).
5. Single layer-group: persistence + payload shape. Higher-layer integrations (AI reading the snapshot back) are out of scope — follow-up work once the snapshot is populated by T003.

## What to Change

### 1. Add `BeliefSnapshot` and `BeliefStatusTag` to `decision_event_payload.rs`

At `crates/worldwake-core/src/decision_event_payload.rs` (near the top with other supporting types):

```rust
/// Serializable projection of `worldwake-sim::BeliefValue` metadata at
/// the moment of a belief-driven blocker or invalidation. Frozen
/// historical record — never re-derived after writing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefSnapshot {
    pub confidence: Permille,
    pub status: BeliefStatusTag,
    pub acquired_tick: Tick,
}

/// Historical-record mirror of `worldwake-sim::BeliefStatus`. Kept in
/// core because `BeliefStatus` lives in sim; reversing the dependency
/// (core -> sim) would violate workspace layering.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BeliefStatusTag {
    Certain,
    Probable,
    Stale,
    Disputed,
    Contradicted,
}
```

Expose both as `pub` via `lib.rs` re-exports if the module is not already public-surfaced.

### 2. Extend `BlockerRecordedPayload`

Add the field at the end of the struct (matches bincode append-order):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerRecordedPayload {
    pub agent: EntityId,
    pub blocker_key: BlockerKey,
    pub discrepancy: Option<Discrepancy>,
    pub blocking_fact: Option<BlockingFact>,
    pub expires_tick: Tick,
    #[serde(default)]
    pub belief_snapshot: Option<BeliefSnapshot>,
}
```

### 3. Extend `PlanInvalidatedPayload`

Same pattern — add `belief_snapshot: Option<BeliefSnapshot>` with `#[serde(default)]` at the end of the struct. Populate only when the `reason: PlanInvalidationReason` variant represents a belief-driven invalidation. The specific variants that carry a snapshot are enumerated in T003 when those variants actually produce snapshots; in this ticket every construction site simply sets `belief_snapshot: None`.

### 4. Update all 14 construction sites

Each site adds `belief_snapshot: None` to the payload literal. File list (confirmed via grep):

- `crates/worldwake-core/src/decision_event_payload.rs` — 4 sites (likely test fixtures and/or default constructors)
- `crates/worldwake-ai/src/agent_tick/mod.rs` — 1 site
- `crates/worldwake-ai/src/agent_tick/execution.rs` — 2 sites
- `crates/worldwake-ai/src/agent_tick/tests.rs` — 2 sites
- `crates/worldwake-cli/src/bin/observer.rs` — 2 sites
- `crates/worldwake-sim/src/save_load.rs` — 2 sites

Total: 14 sites. Workspace must build cleanly after each file's edits — prefer editing all payload constructors for a given payload type in one pass.

### 5. Bump `SAVE_FORMAT_VERSION` to 35

In `crates/worldwake-sim/src/save_load.rs:6`:

```rust
pub const SAVE_FORMAT_VERSION: u32 = 35;
```

Verify the load path at line 129 (`SAVE_FORMAT_VERSION => load_current_format(payload)`) and the mismatch arm at line 132 still behave correctly — they read the constant, so no further change should be needed.

### 6. Unit tests

Add to `decision_event_payload.rs` `#[cfg(test)]`:

1. `BlockerRecordedPayload` constructed with `belief_snapshot: Some(...)` round-trips through `bincode`.
2. `BlockerRecordedPayload` constructed with `belief_snapshot: None` round-trips through `bincode`.
3. `PlanInvalidatedPayload` same two cases.
4. `BeliefStatusTag` serialization produces stable, compact encoding (no magic-string dependency on variant names — bincode uses ordinal indexes).

Add to `save_load.rs` `#[cfg(test)]` (or the relevant existing version-handling test):

5. Full save/load round-trip after the bump — confirm `SAVE_FORMAT_VERSION == 35` and that a payload containing `Some(BeliefSnapshot)` survives serialization.

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add `BeliefSnapshot`, `BeliefStatusTag`, extend two payload structs, update existing construction sites in this file, add unit tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `BeliefSnapshot`, `BeliefStatusTag` if needed)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — 1 construction site)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — 2 construction sites)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — 2 construction sites)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — 2 construction sites)
- `crates/worldwake-sim/src/save_load.rs` (modify — 2 construction sites, `SAVE_FORMAT_VERSION` bump, round-trip test)

## Out of Scope

- Populating `belief_snapshot` with real envelope data from live blockers/invalidations (T003 does this for the specific `Discrepancy` variants it wires up).
- Reading the snapshot back in AI planning (future work — the snapshot is historical-only right now).
- Migrating historical saves — the bump rejects version-34 saves; save-migration tooling is not part of S113.
- Observer-binary UI changes to display the snapshot (follow-up once the field is populated).

## Acceptance Criteria

### Tests That Must Pass

1. All 5 new unit tests in §6 above pass.
2. `cargo test -p worldwake-core decision_event_payload` passes.
3. `cargo test -p worldwake-sim save_load` passes with `SAVE_FORMAT_VERSION == 35`.
4. Full existing suite: `cargo test --workspace` (ensures no construction-site omission broke a distant test).

### Invariants

1. `SAVE_FORMAT_VERSION` is bumped atomically with the schema change — no intermediate state in which version 34 code can deserialize version-35 payloads or vice versa (P28).
2. `belief_snapshot: None` is the lawful default for all non-belief-driven blockers and invalidations; the append-only event log preserves this historically (P29A).
3. `BeliefStatusTag` in core and `BeliefStatus` in sim have identical variant sets and ordinals — drift between them would break round-trip fidelity. Verified by the round-trip test fixtures.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs` `#[cfg(test)]` — 4 new round-trip unit tests per §6 items 1–4.
2. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — 1 new save-format round-trip test per §6 item 5; update any existing version-bump assertions to `35`.

### Commands

1. `cargo test -p worldwake-core decision_event_payload` (targeted, new tests).
2. `cargo test -p worldwake-sim save_load` (targeted, version + round-trip).
3. `cargo test --workspace` (full suite — catches construction-site omissions across crates).
4. `cargo clippy --workspace --all-targets -- -D warnings` (CI parity).
5. `./scripts/verify.sh` before PR.
