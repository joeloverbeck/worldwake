# S109TYPDISTAX-005: Replace UnknownBlockerTrace with DiscrepancyTrace

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `decision_trace.rs` struct replacement + `agent_tick/mod.rs` filter update
**Deps**: S109TYPDISTAX-002, S109TYPDISTAX-004

## Problem

After T004 migrates emission, `DiscrepancyMemory` carries the typed-failure information previously expressed via `BlockingFact::Unknown` entries on `BlockerMemory`. The diagnostic-trace surface (`PlanningPipelineTrace::unknown_blockers: Vec<UnknownBlockerTrace>`) was designed around the old `Unknown` abstraction and now hides the discrepancy classification that observer tooling needs for FND-29 debuggability.

This ticket replaces `UnknownBlockerTrace` with a typed `DiscrepancyTrace` struct and renames `PlanningPipelineTrace::unknown_blockers` to `discrepancy_trace`. The filter populating the field reads `DiscrepancyMemory` directly (no longer filters `BlockerMemory` for `Unknown` variants). `BlockerMemory` entries are not surfaced in this trace — observer tooling consumes them through the existing blocker-memory snapshot path.

## Assumption Reassessment (2026-04-19)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `UnknownBlockerTrace` is defined at `crates/worldwake-ai/src/decision_trace.rs:276–285` and referenced from the `PlanningPipelineTrace::unknown_blockers: Vec<UnknownBlockerTrace>` field at `decision_trace.rs:244–246`. The producing site is the filter at `crates/worldwake-ai/src/agent_tick/mod.rs:837–852`, which T004 already rewrote to read `DiscrepancyMemory` while keeping the `UnknownBlockerTrace` wrapper shape. No downstream consumer code (observer binary, CLI) references `UnknownBlockerTrace` by name — the field is consumed as a `Vec` of items with `goal_key`, `failed_action_def`, `op_kind`, `target`, `place`.
2. S109 spec follow-up deliverable F2 prescribes the exact struct shape: `DiscrepancyTrace { discrepancy: Discrepancy, blocker_key: BlockerKey, expires_tick: Tick }`. The spec notes BlockerMemory entries are not surfaced.
3. Shared abstraction boundary: the `PlanningPipelineTrace` struct and the `decision_trace` module's public surface. After this ticket, the field name and struct change; consumers (observer binary, forensic tooling) must recompile. This is an expected consequence of renaming — no backwards-compatibility shim is introduced.
13. No adjacent contradictions. T004 left `unknown_blockers: Vec<UnknownBlockerTrace>` as an interim adapter populated from `DiscrepancyMemory` — this ticket completes the rename. After this ticket, `grep -rn "UnknownBlockerTrace\|unknown_blockers" crates/` returns zero matches in runtime code.

## Architecture Check

1. The diagnostic-trace surface should match the authoritative memory surface it derives from. After T004, that surface is `DiscrepancyMemory` carrying typed `Discrepancy` variants. Naming the trace field `discrepancy_trace` and the struct `DiscrepancyTrace` reflects what the field actually carries; keeping `UnknownBlockerTrace` as a name would confuse future readers about where the data comes from and what kind of failure it represents.
2. No backwards-compatibility alias. `UnknownBlockerTrace` is removed; `DiscrepancyTrace` replaces it. Consumers (observer, tests) are updated in-scope. FND-28 compliant.

## Verification Layers

1. `DiscrepancyTrace` struct shape → focused unit test in `decision_trace.rs#[cfg(test)]` that constructs the struct with all four fields populated.
2. `PlanningPipelineTrace::discrepancy_trace` field populates from `DiscrepancyMemory` during a real `agent_tick` call → runtime integration test at `agent_tick/tests.rs`: seed `DiscrepancyMemory` with two entries, tick the agent, assert `decision_trace.discrepancy_trace.len() == 2` and the variants/keys match.
3. `BlockerMemory` entries are NOT surfaced in `discrepancy_trace` → focused test in `agent_tick/tests.rs`: seed `BlockerMemory` with a `BlockingFact::SellerOutOfStock` entry, tick, assert `discrepancy_trace` is empty (or only contains `DiscrepancyMemory` entries if both are seeded).
6. Single-layer ticket for diagnostic trace: decision-trace assertion surface is the direct proof.

## What to Change

### 1. Replace struct definition

In `crates/worldwake-ai/src/decision_trace.rs:276–285`, replace the `UnknownBlockerTrace` struct with:

```rust
/// Diagnostic trace for typed discrepancy entries active during planning.
/// Derived from `DiscrepancyMemory` at trace construction time (P27: derived view).
#[derive(Clone, Debug)]
pub struct DiscrepancyTrace {
    pub discrepancy: Discrepancy,
    pub blocker_key: BlockerKey,
    pub expires_tick: Tick,
}
```

Update the `PlanningPipelineTrace::unknown_blockers` field at line 244–246 to:

```rust
/// Active discrepancy-memory entries at trace construction time. Derived
/// view for debuggability (P27).
pub discrepancy_trace: Vec<DiscrepancyTrace>,
```

Update imports in `decision_trace.rs` to include `Discrepancy` from `worldwake-core`. Update the `lib.rs` re-export of `UnknownBlockerTrace` (if any) to `DiscrepancyTrace`.

### 2. Update the populating filter

In `crates/worldwake-ai/src/agent_tick/mod.rs:837–852`, replace the T004-interim `unknown_blockers` filter with:

```rust
discrepancy_trace: discrepancy_memory
    .entries
    .values()
    .filter(|e| e.expires_tick > tick)
    .map(|e| DiscrepancyTrace {
        discrepancy: e.discrepancy,
        blocker_key: e.blocker_key,
        expires_tick: e.expires_tick,
    })
    .collect(),
```

Remove the `op_kind` / `failed_action_def` / `target` / `place` mapping — those are all recoverable from `blocker_key`.

### 3. Update consumers

Grep for `unknown_blockers` and `UnknownBlockerTrace` across the workspace. Replace each with `discrepancy_trace` / `DiscrepancyTrace` in:

- Observer binary (`crates/worldwake-cli/src/bin/observer.rs` or wherever the trace field is rendered).
- Any CLI handler that formats decision traces.
- Tests that assert against the field name.

If observer output format changes visibly, update the corresponding snapshot/golden fixtures (note: spec does not call for format changes, so consumers should render the new struct with its typed `Discrepancy` variant — which is strictly more informative than the old struct's `op_kind`).

### 4. Test updates

In `decision_trace.rs#[cfg(test)]`:

- `discrepancy_trace_struct_carries_typed_discrepancy` — construct a `DiscrepancyTrace` and assert field access.

In `agent_tick/tests.rs`:

- `discrepancy_trace_populated_from_discrepancy_memory` — seed the agent's `DiscrepancyMemory` with two entries (different variants), tick the agent, assert `decision_trace.discrepancy_trace` has both entries.
- `blocker_memory_entries_not_in_discrepancy_trace` — seed `BlockerMemory` only; assert `discrepancy_trace` is empty.
- `discrepancy_trace_excludes_expired_entries` — seed with an expired entry; assert it's filtered.

Remove any tests previously named `unknown_blockers_*` — their coverage migrates to the discrepancy-trace tests above.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — struct rename + field rename + imports)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — filter at lines 837–852)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test renames + new coverage)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export rename if `UnknownBlockerTrace` was re-exported)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — consumer rendering; if `unknown_blockers` is accessed by name)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — same, if applicable)

## Out of Scope

- No removal of `BlockingFact::Unknown`/`AssumptionFailed` variants (T006).
- No changes to `DiscrepancyMemory` shape or `Discrepancy` variants.
- No changes to emission routing (T004).
- No scenario RON changes.
- No golden test extensions (test 9 lands in T006).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai decision_trace` — new struct-shape test.
2. `cargo test -p worldwake-ai agent_tick::tests::discrepancy_trace` — new trace-population tests.
3. Existing `agent_tick` suite: `cargo test -p worldwake-ai agent_tick`.
4. Full workspace: `cargo test --workspace`.

### Invariants

1. `grep -rn "UnknownBlockerTrace\|unknown_blockers" crates/` returns 0 matches in runtime code (inline tests allowed only if they reference archived ticket IDs intentionally — none expected).
2. Every `DiscrepancyMemory` entry with `expires_tick > current_tick` appears exactly once in `decision_trace.discrepancy_trace` for the tick it's queried.
3. No `BlockerMemory` entry appears in `discrepancy_trace`.
4. `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` `#[cfg(test)]` — `discrepancy_trace_struct_carries_typed_discrepancy`.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — three new tests listed in Section 4 above; removal of obsolete `unknown_blockers_*` tests.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-ai agent_tick::tests::discrepancy_trace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
