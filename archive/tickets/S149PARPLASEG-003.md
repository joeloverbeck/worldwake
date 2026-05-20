# S149PARPLASEG-003: AgendaEntry partial-plan storage and save/load

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `AgendaEntry` runtime structure; agenda-state persistence; save-format version
**Deps**: archive/tickets/S149PARPLASEG-002.md

## Problem

Suspended intentions must retain their partial plan so the agenda manager can resume from the prefix-tail rather than replanning from scratch. D10 adds `partial_plan_segment: Option<PartialPlanSegment>` to `AgendaEntry` and extends save/load so segments persist with their entries in `AgendaState.suspended`.

## Assumption Reassessment (2026-05-20)

1. `AgendaEntry` is at `crates/worldwake-ai/src/agenda_types.rs:22` and derives `Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize` (line 14). Because it derives `Default` and the new field is `Option<_>`, the field defaults to `None` automatically; the 70 `AgendaEntry { ... }` construction sites in `worldwake-ai` either go through the `impl AgendaEntry` constructor (line 42) or can rely on `Default`. The construction-site count is therefore informational, not load-bearing.
2. `AgendaState` (agenda_types.rs:15) holds `committed: Option<AgendaEntry>`, `pending: BTreeMap<AgendaEntryKey, AgendaEntry>`, `suspended: BTreeMap<AgendaEntryKey, AgendaEntry>`. Segments persist with the `suspended` entries through the existing agenda-state save path.
3. Shared boundary under audit: the `AgendaEntry` serde representation and the agenda-state save/load path. `SAVE_FORMAT_VERSION` was bumped 90→91 in ticket 001; live reassessment for this ticket proved that bincode deserialization of a 91-era `AgendaEntry` shape without the trailing `partial_plan_segment` field fails with `UnexpectedEof`. Per `docs/FOUNDATIONS.md` FND-12 and FND-28, this ticket bumps `SAVE_FORMAT_VERSION` 91→92 and rejects older save/runtime bytes at the existing save-header boundary rather than adding a compatibility decoder or claiming false omitted-field tolerance. `AgendaState` is ai-crate runtime state nested under `AgentDecisionRuntime`, whose bincode save payload is carried by the existing `worldwake-sim` save/load envelope.
4. The field type `PartialPlanSegment` and its `Default`/derive bounds are introduced by ticket 002; this ticket only adds the field and persistence.

## Architecture Check

1. An additive `Option` field on a `Default`-deriving struct is the minimal storage change for current-format runtime state. The current save envelope bumps to version 92 because bincode cannot truthfully deserialize the pre-field runtime shape by `#[serde(default)]` alone; rejecting version 91 at the save header is cleaner than a compatibility shim (FND-12/FND-28).
2. No parallel storage: the segment lives inside its owning `AgendaEntry`, consistent with the spec's "no shared partial-plan pool" non-goal.

## Verified Layers

1. Field persists with suspended entry → save/load roundtrip test asserting a suspended `AgendaEntry` with `Some(segment)` survives serialize/deserialize.
2. Save-version boundary → `worldwake-sim` save/load test asserts version 92 and rejects version 91 at the save header, so older runtime bytes are not misrepresented as compatible. Single-system ticket (no cross-system ordering); the two surfaces above are the authoritative proof of the storage contract.

## Landed Changes

### 1. Added the field

In `crates/worldwake-ai/src/agenda_types.rs`, added `#[serde(default)] pub partial_plan_segment: Option<PartialPlanSegment>` to `AgendaEntry`. `AgendaEntry::pending(...)` and direct literal construction sites now initialize it to `None`, preserving the ticket-003 placeholder state until ticket 005 populates barrier segments.

### 2. Proved save/load coverage

Added current-format agenda-state and `AgentDecisionRuntime` bincode roundtrip coverage for a suspended entry carrying `Some(PartialPlanSegment)`. Bumped `SAVE_FORMAT_VERSION` 91→92 and updated the save-header rejection test so version 91 is rejected without a compatibility shim.

## Touched Files

- `crates/worldwake-ai/src/agenda_types.rs` (modify) — field + constructor
- `crates/worldwake-ai/src/decision_runtime.rs` (modify) — `AgentDecisionRuntime` runtime-payload roundtrip carries a suspended partial segment
- `crates/worldwake-sim/src/save_load.rs` (modify) — `SAVE_FORMAT_VERSION` 91→92 and version-91 rejection proof
- `crates/worldwake-ai/src/*`, `crates/worldwake-ai/tests/scenarios/portfolio_planning.rs`, `crates/worldwake-cli/src/bin/observer.rs` (modify) — direct `AgendaEntry` literals initialized the new field to `None`
- `archive/specs/S149-partial-plan-segments-and-typed-terminals.md` (modify) — parent spec truthed to the version-92 save boundary

## Out of Scope

- A compatibility decoder for pre-92 runtime bytes. Version 91 saves are rejected at the existing save-header boundary instead.
- Populating the segment at barrier sites (ticket 005) and reading it for resumption (ticket 005).

## Acceptance Results

### Test Results

1. Passed: a suspended `AgendaEntry` with `Some(PartialPlanSegment)` roundtrips through the agenda-state save path.
2. Passed: `SAVE_FORMAT_VERSION` is 92 and a version-91 save is rejected at the save header.
3. Passed: existing suite `cargo test -p worldwake-ai`.

### Invariants

1. `partial_plan_segment` defaults to `None` on every construction path; no existing code sets it (population is ticket 005).
2. No backward-compatibility shim or custom old-runtime decoder is introduced; version 91 is rejected explicitly.

## Verification Result

1. Passed: `cargo test -p worldwake-ai agenda_state`
2. Passed: `cargo test -p worldwake-ai agent_decision_runtime_bincode_round_trip_preserves_all_fields`
3. Passed: `cargo test -p worldwake-sim save_format_version_is_92_after_s149_partial_plan_storage_landing`
4. Passed: `cargo test -p worldwake-sim load_rejects_pre_s149_partial_plan_storage_version_91_without_migration_shim`
5. Passed: `cargo fmt --all`
6. Passed: `cargo test -p worldwake-ai`
7. Passed: `cargo test -p worldwake-sim save_load`
8. Passed: `git diff --check`

## Outcome

Completed: 2026-05-20

`AgendaEntry` now carries `partial_plan_segment: Option<PartialPlanSegment>`, defaulting to `None` on constructor and literal paths. Suspended agenda entries preserve a concrete `PartialPlanSegment` through `AgendaState` and the existing `AgentDecisionRuntime` runtime-payload bincode path. The save format is now version 92; version 91 saves are rejected at the existing save-header boundary instead of being decoded through a backward-compatibility shim.

Deviation from draft: live proof showed bincode cannot deserialize the pre-field version-91 runtime shape with `#[serde(default)]` alone. The ticket and parent spec were corrected to use an explicit version bump and rejection proof, aligning the save boundary with FND-12 and FND-28.
