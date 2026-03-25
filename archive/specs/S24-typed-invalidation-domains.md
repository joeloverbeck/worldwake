**Status**: ✅ COMPLETED

# S24: Typed Invalidation Domains

## Summary

Decompose the opaque `SnapshotChanged` variant in `DirtyReason` into typed observation sub-domains (position, needs, wounds, inventory, facilities) so that traces report *which* dimension triggered a replan. Add typed frame-lifecycle domains for the invalidation paths introduced by S22 (Generalized Intention Frames). Replace the `dirty: bool` field on `AgentDecisionRuntime` with a `DirtySet` newtype over `u16` that subsumes the existing `DirtyReason` categories, the new snapshot sub-domains, and the new frame lifecycle domains, eliminating the dual-tracking of `dirty: bool` plus `dirty_reasons: Vec<DirtyReason>`.

## Phase

Phase 3+: AI Architecture Overhaul (post-E13, Wave 2)

## Crate

`worldwake-ai`

## Dependencies

- S21 (promote causal runtime state) -- S21 restructures `AgentDecisionRuntime` fields. This spec replaces the `dirty: bool` field that S21 classifies as ephemeral. Implementing S24 after S21 avoids merge conflicts in `decision_runtime.rs`.
- S22 (generalized intention frames) -- S22 introduced frame-related invalidation paths (`frame.rs`, `active_action.rs`, `mod.rs`) that set `dirty = true` outside the read phase. This spec adds typed domain bits for those sites.

## FOUNDATIONS Alignment

- **P25** (Derived summaries are caches, never truth): The plan validity flag is a derived cache of observation state. When the source (world observations) changes, the plan must be invalidated. Typed domains make the invalidation contract explicit -- each snapshot dimension maps to a named bit, and a new dimension without a corresponding bit is visibly missing.
- **P3** (Concrete state over abstract scores): Replacing an opaque `bool` with a typed set of concrete reasons makes the invalidation state inspectable rather than abstract.
- **P27** (Debuggability is a product feature): Typed dirty domains answer "why did agent X replan?" with specific domain names (e.g., `NEEDS|POSITION` or `FRAME_BLOCKAGE`) instead of opaque `SnapshotChanged` or bare `true`.

## Motivation

The current invalidation system has three structural problems:

### 1. Opaque snapshot invalidation

`observation_snapshot_changed()` (`observation.rs:345`) compares 6 dimensions -- effective_place, needs, wounds, commodity_signature, unique_item_signature, facility_access_signature -- but returns a single `bool`. When this bool is true, `DirtyReason::SnapshotChanged` is pushed to the trace `Vec`. A developer debugging "why did agent X replan?" sees `SnapshotChanged` but must manually inspect all 6 comparisons to identify which dimension actually changed.

### 2. Dual-tracking with divergence risk

`AgentDecisionRuntime` carries `dirty: bool` (`decision_runtime.rs:~60`) as the authoritative replan trigger, while `refresh_runtime_for_read_phase()` (`observation.rs:62-153`) separately builds a `Vec<DirtyReason>` for trace output. The boolean and the vec are kept in sync by the line `runtime.dirty = runtime.dirty || !dirty_reasons.is_empty()` (`observation.rs:120`), but this dual-tracking invites divergence: a future change that sets `dirty = true` without pushing a corresponding `DirtyReason` (or vice versa) would silently break trace fidelity.

This divergence has already occurred: S22 introduced 5 new `dirty = true` sites in `frame.rs`, `active_action.rs`, and `mod.rs` that set the boolean but push no `DirtyReason`. Traces for these replans show no reason at all.

### 3. Silent omission risk for new dimensions

Adding a new causally relevant observation dimension (e.g., social state, threat awareness) requires threading it into `observation_snapshot_changed()`. If forgotten, agents never replan when that dimension changes. A typed domain set makes the expected dimensions explicit -- a new `last_*` snapshot field without a corresponding domain bit is a visible gap.

## Current State (as of codebase inspection 2026-03-24)

### Module structure (S20 refactoring)

`agent_tick.rs` was split by S20 into `agent_tick/{mod.rs, observation.rs, planning.rs, frame.rs, active_action.rs, candidates.rs, execution.rs}`.

### Existing types

- `DirtyReason` enum in `decision_trace.rs:594` with 7 variants: `NoPlan`, `PlanFinished`, `ReplanSignal`, `QueueTransition`, `BlockerCleanup`, `SnapshotChanged`, `QueuePatienceExhausted`
- `PlanningPipelineTrace.dirty_reasons: Vec<DirtyReason>` in `decision_trace.rs:~142`
- `AgentDecisionRuntime.dirty: bool` in `decision_runtime.rs:~60`

### Existing flow

1. `refresh_runtime_for_read_phase()` (`observation.rs:62`) calls `observation_snapshot_changed()` (`observation.rs:345`) which returns `bool`
2. If true, pushes `DirtyReason::SnapshotChanged` to a local `Vec<DirtyReason>`
3. Sets `runtime.dirty = runtime.dirty || !dirty_reasons.is_empty()` (`observation.rs:120`)
4. The vec flows into `ReadPhaseResult` and then into `PlanningPipelineTrace` for trace output
5. `runtime.dirty` is the actual decision gate for replanning (checked at `planning.rs:257, 428`)
6. `is_snapshot_changed_only()` (`planning.rs:351`) checks if plan continuation (revalidation instead of full replan) is appropriate

### Existing observation dimensions (observation.rs:363-374)

| Comparison | Snapshot field | New domain name |
|---|---|---|
| `last_effective_place != view.effective_place(agent)` | `last_effective_place` | `POSITION` |
| `last_needs != view.homeostatic_needs(agent)` | `last_needs` | `NEEDS` |
| `last_wounds != view.wounds(agent)` | `last_wounds` | `WOUNDS` |
| `filtered_commodity_signature(...)` | `last_commodity_signature` | `COMMODITY` |
| `last_unique_item_signature != unique_item_signature(...)` | `last_unique_item_signature` | `UNIQUE_ITEMS` |
| `last_facility_access_signature != facility_access_signature(...)` | `last_facility_access_signature` | `FACILITIES` |

## Design

### DirtySet newtype

A `u16` newtype with named bit constants, replacing both `dirty: bool` on the runtime and `Vec<DirtyReason>` in traces. No external `bitflags` crate -- the project's minimal-dependency policy (serde, bincode, rand_chacha, blake3) is preserved.

```rust
/// Typed set of reasons why an agent's plan is invalidated this tick.
///
/// Each bit represents one invalidation domain. The set replaces the
/// former `dirty: bool` on `AgentDecisionRuntime` and the
/// `Vec<DirtyReason>` on `PlanningPipelineTrace`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DirtySet(u16);

impl DirtySet {
    // ── Structural reasons (not snapshot-derived) ──
    pub const NO_PLAN: Self          = Self(1 << 0);
    pub const PLAN_FINISHED: Self    = Self(1 << 1);
    pub const REPLAN_SIGNAL: Self    = Self(1 << 2);
    pub const QUEUE_TRANSITION: Self = Self(1 << 3);
    pub const BLOCKER_CLEANUP: Self  = Self(1 << 4);
    pub const QUEUE_PATIENCE: Self   = Self(1 << 5);

    // ── Snapshot observation domains ──
    pub const POSITION: Self      = Self(1 << 6);
    pub const NEEDS: Self         = Self(1 << 7);
    pub const WOUNDS: Self        = Self(1 << 8);
    pub const COMMODITY: Self     = Self(1 << 9);
    pub const UNIQUE_ITEMS: Self  = Self(1 << 10);
    pub const FACILITIES: Self    = Self(1 << 11);

    // ── Frame lifecycle domains (S22) ──
    pub const FRAME_BLOCKAGE: Self     = Self(1 << 12);
    pub const FRAME_PATIENCE: Self     = Self(1 << 13);
    pub const ASSUMPTION_FAILED: Self  = Self(1 << 14);

    // Bit 15: reserved for future use.

    // ── Aggregate masks ──

    /// All structural (non-snapshot, non-frame) bits.
    pub const STRUCTURAL_MASK: Self = Self(
        Self::NO_PLAN.0
            | Self::PLAN_FINISHED.0
            | Self::REPLAN_SIGNAL.0
            | Self::QUEUE_TRANSITION.0
            | Self::BLOCKER_CLEANUP.0
            | Self::QUEUE_PATIENCE.0,
    );

    /// All snapshot-derived bits. Used by `is_snapshot_only()`.
    pub const SNAPSHOT_MASK: Self = Self(
        Self::POSITION.0
            | Self::NEEDS.0
            | Self::WOUNDS.0
            | Self::COMMODITY.0
            | Self::UNIQUE_ITEMS.0
            | Self::FACILITIES.0,
    );

    /// All frame lifecycle bits.
    pub const FRAME_MASK: Self = Self(
        Self::FRAME_BLOCKAGE.0
            | Self::FRAME_PATIENCE.0
            | Self::ASSUMPTION_FAILED.0,
    );

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Insert all bits from `other`.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// True when at least one snapshot bit is set and NO structural
    /// or frame bits are set.
    #[must_use]
    pub fn is_snapshot_only(self) -> bool {
        !self.is_empty() && (self.0 & !Self::SNAPSHOT_MASK.0) == 0
    }

    /// True when the given bit(s) are all set.
    #[must_use]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Human-readable pipe-separated domain names (e.g., "NEEDS|POSITION").
    #[must_use]
    pub fn display_names(self) -> String {
        let mut names = Vec::new();
        if self.contains(Self::NO_PLAN) { names.push("NO_PLAN"); }
        if self.contains(Self::PLAN_FINISHED) { names.push("PLAN_FINISHED"); }
        if self.contains(Self::REPLAN_SIGNAL) { names.push("REPLAN_SIGNAL"); }
        if self.contains(Self::QUEUE_TRANSITION) { names.push("QUEUE_TRANSITION"); }
        if self.contains(Self::BLOCKER_CLEANUP) { names.push("BLOCKER_CLEANUP"); }
        if self.contains(Self::QUEUE_PATIENCE) { names.push("QUEUE_PATIENCE"); }
        if self.contains(Self::POSITION) { names.push("POSITION"); }
        if self.contains(Self::NEEDS) { names.push("NEEDS"); }
        if self.contains(Self::WOUNDS) { names.push("WOUNDS"); }
        if self.contains(Self::COMMODITY) { names.push("COMMODITY"); }
        if self.contains(Self::UNIQUE_ITEMS) { names.push("UNIQUE_ITEMS"); }
        if self.contains(Self::FACILITIES) { names.push("FACILITIES"); }
        if self.contains(Self::FRAME_BLOCKAGE) { names.push("FRAME_BLOCKAGE"); }
        if self.contains(Self::FRAME_PATIENCE) { names.push("FRAME_PATIENCE"); }
        if self.contains(Self::ASSUMPTION_FAILED) { names.push("ASSUMPTION_FAILED"); }
        if names.is_empty() { "CLEAN".to_string() } else { names.join("|") }
    }
}

impl std::fmt::Display for DirtySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display_names())
    }
}

impl std::ops::BitOr for DirtySet {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DirtySet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
```

### Migration: AgentDecisionRuntime

Replace in `decision_runtime.rs`:

```
// Before
pub dirty: bool,

// After
pub dirty: DirtySet,
```

The field name stays `dirty` to minimize diff noise. All reads change:
- `if runtime.dirty` becomes `if !runtime.dirty.is_empty()`
- `runtime.dirty = true` becomes `runtime.dirty.insert(DirtySet::APPROPRIATE_DOMAIN)` (see sites table below)
- `runtime.dirty = false` becomes `runtime.dirty = DirtySet::default()`

### Migration: observation_snapshot_changed

Replace the function signature and body in `observation.rs:345`:

```rust
// Before: returns bool
fn observation_snapshot_changed(...) -> bool { ... }

// After: returns DirtySet (only snapshot bits set)
fn observation_snapshot_changed(...) -> DirtySet {
    let mut reasons = DirtySet::default();
    if runtime.last_effective_place != view.effective_place(agent) {
        reasons.insert(DirtySet::POSITION);
    }
    if runtime.last_needs != view.homeostatic_needs(agent) {
        reasons.insert(DirtySet::NEEDS);
    }
    if runtime.last_wounds != view.wounds(agent) {
        reasons.insert(DirtySet::WOUNDS);
    }
    // ... commodity, unique_items, facilities similarly
    reasons
}
```

### Migration: refresh_runtime_for_read_phase

Replace the dual-tracking pattern in `observation.rs:62-153`:

```rust
// Before: build Vec<DirtyReason> separately, then sync with runtime.dirty
let snapshot_changed = observation_snapshot_changed(&view, agent, runtime, phase.recipe_registry);
let mut dirty_reasons = Vec::new();
// ... push individual DirtyReasons ...
if snapshot_changed { dirty_reasons.push(DirtyReason::SnapshotChanged); }
runtime.dirty = runtime.dirty || !dirty_reasons.is_empty();

// After: build DirtySet directly on runtime
let snapshot_domains = observation_snapshot_changed(&view, agent, runtime, phase.recipe_registry);
runtime.dirty.insert(snapshot_domains);
if runtime.current_plan.is_none() { runtime.dirty.insert(DirtySet::NO_PLAN); }
if plan_finished(runtime) { runtime.dirty.insert(DirtySet::PLAN_FINISHED); }
if !replan_signals.is_empty() { runtime.dirty.insert(DirtySet::REPLAN_SIGNAL); }
if queue_transition_changed { runtime.dirty.insert(DirtySet::QUEUE_TRANSITION); }
if blocked_changed_from_cleanup { runtime.dirty.insert(DirtySet::BLOCKER_CLEANUP); }
if queue_patience_exhausted { runtime.dirty.insert(DirtySet::QUEUE_PATIENCE); }
```

The `Vec<DirtyReason>` local variable, all `push(DirtyReason::...)` calls, and the dual-tracking sync line (`observation.rs:120`) are removed entirely.

### Migration: is_snapshot_changed_only

Replace `is_snapshot_changed_only(dirty_reasons: &[DirtyReason]) -> bool` (`planning.rs:351`) call sites at `planning.rs:258` and `planning.rs:429` with `runtime.dirty.is_snapshot_only()`. Then remove the `is_snapshot_changed_only` function. This is semantically equivalent but operates on the unified `DirtySet` rather than filtering a vec, and correctly excludes frame lifecycle bits (not just structural ones).

### Migration: DirtyReason enum removal

The `DirtyReason` enum in `decision_trace.rs:594` is removed entirely. All sites that construct or pattern-match on it are migrated to `DirtySet` operations. The `PlanningPipelineTrace.dirty_reasons: Vec<DirtyReason>` field becomes `dirty: DirtySet`.

### Migration: PlanningPipelineTrace

```rust
// Before
pub dirty_reasons: Vec<DirtyReason>,

// After
pub dirty: DirtySet,
```

### Migration: ReadPhaseResult

```rust
// Before
dirty_reasons: Vec<DirtyReason>,

// After (field removed -- dirty state lives on runtime.dirty directly)
// The DirtySet is read from runtime.dirty when constructing the trace.
```

The `dirty_reasons` field is also removed from the `ReadPhaseResult` construction at `observation.rs:~135` and all downstream access sites. The `dirty_reasons: &[DirtyReason]` parameter on `select_and_execute_plan()` (`planning.rs:~251`) and `select_and_execute_plan_traced()` (`planning.rs:~380`) is removed; these functions read `runtime.dirty` directly.

### Trace integration

- `format_outcome()` and `summary()` include `dirty.display_names()` in their output when the outcome is `Planning`.
- `dump_agent()` shows typed domain names, e.g., `[tick 5] PLAN (dirty: NEEDS|POSITION): selected=...`
- The `plan_continued` flag on `PlanningPipelineTrace` remains -- it is set when `dirty.is_snapshot_only()` and the current plan passes revalidation.

### Complete dirty-site migration table

All sites across the AI crate that read, set, or clear `runtime.dirty`:

#### Mutation sites (set dirty)

| File | Line | Context | After |
|---|---|---|---|
| `observation.rs:~98` | No plan detected | `runtime.dirty.insert(DirtySet::NO_PLAN)` |
| `observation.rs:~101` | Plan finished | `runtime.dirty.insert(DirtySet::PLAN_FINISHED)` |
| `observation.rs:~104` | Replan signal | `runtime.dirty.insert(DirtySet::REPLAN_SIGNAL)` |
| `observation.rs:~107` | Queue transition | `runtime.dirty.insert(DirtySet::QUEUE_TRANSITION)` |
| `observation.rs:~110` | Blocker cleanup | `runtime.dirty.insert(DirtySet::BLOCKER_CLEANUP)` |
| `observation.rs:~113` | Snapshot changed | Removed -- replaced by typed bits from `observation_snapshot_changed()` returning `DirtySet` |
| `observation.rs:~116` | Queue patience | `runtime.dirty.insert(DirtySet::QUEUE_PATIENCE)` |
| `observation.rs:120` | Dual-tracking sync | Removed entirely |
| `frame.rs:197` | Travel step blockage (S22) | `runtime.dirty.insert(DirtySet::FRAME_BLOCKAGE)` |
| `frame.rs:405` | Frame patience exhaustion (S22) | `runtime.dirty.insert(DirtySet::FRAME_PATIENCE)` |
| `mod.rs:378` | Critical assumption failure (S22) | `runtime.dirty.insert(DirtySet::ASSUMPTION_FAILED)` |
| `active_action.rs:205` | ProgressBarrier terminal | `runtime.dirty.insert(DirtySet::PLAN_FINISHED)` |
| `active_action.rs:223` | GoalSatisfied/CombatCommitment terminal | `runtime.dirty.insert(DirtySet::PLAN_FINISHED)` |
| `failure_handling.rs:81` | Plan failure | `runtime.dirty.insert(DirtySet::REPLAN_SIGNAL)` |

#### Clear sites (reset dirty)

| File | Line | Context | After |
|---|---|---|---|
| `mod.rs:269` | Dead agent early return | `runtime.dirty = DirtySet::default()` |
| `planning.rs:273` | Snapshot continuation (non-traced path) | `runtime.dirty = DirtySet::default()` |
| `planning.rs:333` | After plan selection (non-traced path) | `runtime.dirty = DirtySet::default()` |
| `planning.rs:444` | Snapshot continuation (traced path) | `runtime.dirty = DirtySet::default()` |
| `planning.rs:575` | After plan selection (traced path) | `runtime.dirty = DirtySet::default()` |

#### Read sites (check dirty)

| File | Line | Current | After |
|---|---|---|---|
| `planning.rs:257` | Guard in non-traced path | `if !runtime.dirty.is_empty()` |
| `planning.rs:428` | Guard in traced path | `if !runtime.dirty.is_empty()` |

## Tickets

### S24-001: Define DirtySet type and unit tests

- Create `DirtySet` newtype in a new module `dirty_set.rs` within `worldwake-ai`
- Define all 15 bit constants (6 structural, 6 snapshot, 3 frame) plus `STRUCTURAL_MASK`, `SNAPSHOT_MASK`, `FRAME_MASK`
- Implement `is_empty`, `insert`, `is_snapshot_only`, `contains`, `display_names`, `Display`
- Implement `Default` (zero / clean)
- Implement `BitOr` and `BitOrAssign` for ergonomic set combination
- Add to `lib.rs` re-exports
- Unit tests:
  - Default is empty
  - Insert sets bits, is_empty returns false
  - `is_snapshot_only` true for snapshot-only bits, false when structural or frame bits present
  - `contains` checks individual and combined bits
  - `display_names` formats correctly for empty, single, and multiple bits
  - `SNAPSHOT_MASK` covers exactly the 6 snapshot bits
  - `FRAME_MASK` covers exactly the 3 frame bits
  - `STRUCTURAL_MASK` covers exactly the 6 structural bits
  - `BitOr` combines sets correctly
  - `BitOrAssign` combines sets in place
- Verify: `cargo test -p worldwake-ai`

### S24-002: Replace dirty:bool with DirtySet on AgentDecisionRuntime

- Replace `pub dirty: bool` with `pub dirty: DirtySet` in `decision_runtime.rs`
- Update the default-state test (`agent_decision_runtime_defaults_to_empty_clean_state`)
- Update all reads of `runtime.dirty` across `planning.rs` to use `!runtime.dirty.is_empty()`
- Update all writes of `runtime.dirty = true` to `runtime.dirty.insert(DirtySet::APPROPRIATE_DOMAIN)` per the mutation sites table:
  - `frame.rs:197` → `DirtySet::FRAME_BLOCKAGE`
  - `frame.rs:405` → `DirtySet::FRAME_PATIENCE`
  - `mod.rs:378` → `DirtySet::ASSUMPTION_FAILED`
  - `active_action.rs:205` → `DirtySet::PLAN_FINISHED`
  - `active_action.rs:223` → `DirtySet::PLAN_FINISHED`
  - `failure_handling.rs:81` → `DirtySet::REPLAN_SIGNAL`
- Update all writes of `runtime.dirty = false` to `runtime.dirty = DirtySet::default()` per the clear sites table
- Update test setup sites that initialize `dirty: false` to `dirty: DirtySet::default()` and `dirty: true` to non-empty `DirtySet`
- Verify: `cargo test -p worldwake-ai` -- all existing tests pass

### S24-003: Decompose observation_snapshot_changed into typed domains

- Change `observation_snapshot_changed()` (`observation.rs:345`) return type from `bool` to `DirtySet`
- Each of the 6 comparisons inserts its domain-specific bit
- Update `refresh_runtime_for_read_phase()` (`observation.rs:62`) to merge snapshot domains directly into `runtime.dirty` via `runtime.dirty.insert(snapshot_domains)` instead of pushing `DirtyReason::SnapshotChanged`
- Replace the `Vec<DirtyReason>` local variable and all `dirty_reasons.push(...)` calls with direct `runtime.dirty.insert(DirtySet::...)` per structural reason
- Remove the dual-tracking sync line (`observation.rs:120`)
- Remove `dirty_reasons` field from `ReadPhaseResult` (`observation.rs:~46`)
- Remove `dirty_reasons: &[DirtyReason]` parameter from `select_and_execute_plan()` and `select_and_execute_plan_traced()` in `planning.rs`
- Replace `is_snapshot_changed_only(dirty_reasons)` calls (`planning.rs:258, 429`) with `runtime.dirty.is_snapshot_only()`
- Remove the `is_snapshot_changed_only` function (`planning.rs:351-356`)
- Verify: `cargo test -p worldwake-ai` -- all golden tests pass

### S24-004: Remove DirtyReason enum, migrate traces to DirtySet

- Remove `DirtyReason` enum from `decision_trace.rs:594`
- Replace `PlanningPipelineTrace.dirty_reasons: Vec<DirtyReason>` with `dirty: DirtySet`
- Update all trace construction sites in `mod.rs` to pass `runtime.dirty` directly
- Remove `DirtyReason` from `lib.rs` re-export; ensure `DirtySet` is exported
- Remove all `DirtyReason` imports throughout the crate
- Update `format_outcome()` to include `dirty.display_names()` in dump output
- Update `summary()` to include dirty domain names
- Update tests that construct `PlanningPipelineTrace` with `dirty_reasons: vec![...]` to use `dirty: DirtySet::...`
- Verify: `cargo test -p worldwake-ai` -- all tests pass

### S24-005: Workspace verification and trace output validation

- `cargo test --workspace` -- all pass
- `cargo clippy --workspace` -- no new warnings
- Manual verification: enable tracing in a golden test, confirm `dump_agent` output shows typed domain names (e.g., `dirty: NEEDS|POSITION` or `dirty: FRAME_BLOCKAGE`) instead of `SnapshotChanged`
- Confirm all golden tests pass unchanged (behavioral equivalence -- only trace output format changes)
- Confirm no `DirtyReason` references remain in codebase

## FND-01 Section H Analysis

### Information-path analysis

`DirtySet` is internal AI runtime state. It does not propagate to other agents or cross system boundaries. No information-path concerns.

### Positive-feedback analysis

None. This spec changes the representation of an existing invalidation mechanism without altering invalidation semantics. No new amplifying loops are introduced.

### Concrete dampeners

N/A -- no positive-feedback loops.

### Stored state vs. derived read-model list

- **Stored (ephemeral runtime, not authoritative)**: `AgentDecisionRuntime.dirty: DirtySet` -- accumulated across dirty-detection calls within a tick, cleared when the agent replans or continues its plan. This is ephemeral per-tick runtime state, not a persisted authoritative component (consistent with S21's classification of `dirty` as rederivable).
- **Stored (trace diagnostic)**: `PlanningPipelineTrace.dirty: DirtySet` -- recorded in the trace sink for diagnostic queries. Trace data is opt-in and never read by decision logic.
- **Derived**: The individual bits are derived from snapshot comparisons (`observation_snapshot_changed`), structural checks (`plan_finished`, etc.), and frame lifecycle events (`handle_recoverable_travel_step_blockage`, frame patience exhaustion, critical assumption failure) each tick.

## Verification

1. `cargo test --workspace` -- all pass
2. `cargo clippy --workspace` -- no new warnings
3. All golden tests pass unchanged (behavioral equivalence)
4. Decision trace `dump_agent()` output shows typed dirty domain names
5. `summary()` includes dirty domain names for planning outcomes
6. No `DirtyReason` enum references remain in the codebase after S24-004
7. `is_snapshot_only()` correctly returns `false` when any frame or structural bit is set

## Outcome

- **Completion date**: 2026-03-25
- **What changed**: Replaced `dirty: bool` + `Vec<DirtyReason>` dual-tracking with `DirtySet` (u16 bitflag newtype) across the entire AI pipeline. `DirtyReason` enum removed. 14 typed domains: 6 structural (NO_PLAN, PLAN_FINISHED, REPLAN_SIGNAL, QUEUE_TRANSITION, BLOCKER_CLEANUP, QUEUE_PATIENCE) + 6 snapshot (POSITION, NEEDS, WOUNDS, COMMODITY, UNIQUE_ITEMS, FACILITIES) + 2 frame lifecycle (FRAME_BLOCKAGE, FRAME_PATIENCE) + ASSUMPTION_FAILED. Trace output now shows specific domain names instead of opaque `SnapshotChanged`. `is_snapshot_only()` enables plan-continuation optimization when only snapshot domains changed.
- **Deviations**: None — all 5 tickets (S24TYPINVDOM-001 through -005) implemented as specified.
- **Verification**: All workspace build/test/clippy clean. Zero `DirtyReason` residue. All golden tests pass unchanged (behavioral equivalence confirmed).
