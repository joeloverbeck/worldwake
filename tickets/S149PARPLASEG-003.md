# S149PARPLASEG-003: AgendaEntry partial-plan storage and save/load

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `AgendaEntry` runtime structure; agenda-state persistence
**Deps**: archive/tickets/S149PARPLASEG-002.md

## Problem

Suspended intentions must retain their partial plan so the agenda manager can resume from the prefix-tail rather than replanning from scratch. D10 adds `partial_plan_segment: Option<PartialPlanSegment>` to `AgendaEntry` and extends save/load so segments persist with their entries in `AgendaState.suspended`.

## Assumption Reassessment (2026-05-20)

1. `AgendaEntry` is at `crates/worldwake-ai/src/agenda_types.rs:22` and derives `Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize` (line 14). Because it derives `Default` and the new field is `Option<_>`, the field defaults to `None` automatically; the 70 `AgendaEntry { ... }` construction sites in `worldwake-ai` either go through the `impl AgendaEntry` constructor (line 42) or can rely on `Default`. The construction-site count is therefore informational, not load-bearing.
2. `AgendaState` (agenda_types.rs:15) holds `committed: Option<AgendaEntry>`, `pending: BTreeMap<AgendaEntryKey, AgendaEntry>`, `suspended: BTreeMap<AgendaEntryKey, AgendaEntry>`. Segments persist with the `suspended` entries through the existing agenda-state save path.
3. Shared boundary under audit: the `AgendaEntry` serde representation and the agenda-state save/load path. `SAVE_FORMAT_VERSION` was bumped 90→91 in ticket 001; this ticket's field is additive and carries `#[serde(default)]`, so it deserializes against the 001-era format without a second bump. `AgendaState` is ai-crate runtime state — the implementation must route serialization through the existing path that already persists `AgendaState.suspended` rather than assuming a sim-crate accessor to ai state (confirm the path during implementation; `Likely: crates/worldwake-ai/src/` agenda-state save module — grep `AgendaState` serialization consumers).
4. The field type `PartialPlanSegment` and its `Default`/derive bounds are introduced by ticket 002; this ticket only adds the field and persistence.

## Architecture Check

1. An additive `Option` field on a `Default`-deriving struct is the minimal storage change; `#[serde(default)]` preserves load compatibility with pre-field saves without a custom `Deserialize` impl or a second version bump (FND-12: boundary tolerates old byte streams, normalizes to current model).
2. No parallel storage: the segment lives inside its owning `AgendaEntry`, consistent with the spec's "no shared partial-plan pool" non-goal.

## Verification Layers

1. Field persists with suspended entry → save/load roundtrip test asserting a suspended `AgendaEntry` with `Some(segment)` survives serialize/deserialize.
2. Old-format tolerance → deserialize a fixture lacking the field and assert it loads with `partial_plan_segment: None` (FND-12 boundary). Single-system ticket (no cross-system ordering); the two surfaces above are the authoritative proof of the storage contract.

## What to Change

### 1. Add the field

In `crates/worldwake-ai/src/agenda_types.rs`, add `#[serde(default)] pub partial_plan_segment: Option<PartialPlanSegment>` to `AgendaEntry`. Update the `impl AgendaEntry` constructor (line 42) to initialize it `None`. Verify literal construction sites compile (Default/spread covers them).

### 2. Save/load coverage

Extend the agenda-state serialization path so suspended entries carry their segment. Add roundtrip and old-format-tolerance tests.

## Files to Touch

- `crates/worldwake-ai/src/agenda_types.rs` (modify) — field + constructor
- `Likely: crates/worldwake-ai/src/agenda_manager.rs` or the agenda-state save module (modify) — persistence coverage; grep `AgendaState` serialization consumers to pin the exact site
- `crates/worldwake-ai/tests/fixtures/` (new fixture) — old-format-tolerance sample, if the test harness uses fixtures

## Out of Scope

- A second `SAVE_FORMAT_VERSION` bump (handled by ticket 001; this field rides `#[serde(default)]`).
- Populating the segment at barrier sites (ticket 005) and reading it for resumption (ticket 005).

## Acceptance Criteria

### Tests That Must Pass

1. New: a suspended `AgendaEntry` with `Some(PartialPlanSegment)` roundtrips through the agenda-state save path.
2. New: an agenda-state byte stream lacking the field deserializes with `partial_plan_segment: None`.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `partial_plan_segment` defaults to `None` on every construction path; no existing code sets it (population is ticket 005).
2. No second `SAVE_FORMAT_VERSION` bump is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` or agenda save module (inline) — segment-persistence roundtrip + old-format tolerance.

### Commands

1. `cargo test -p worldwake-ai agenda`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
