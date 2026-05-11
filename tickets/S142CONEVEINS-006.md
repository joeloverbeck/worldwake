# S142CONEVEINS-006: Add observer Section 12 (Contention) and `--contention-top-n` CLI flag

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small-Medium
**Engine Changes**: None — observer-only diagnostic surface; no simulation runtime change
**Deps**: archive/tickets/S142CONEVEINS-001.md (provides `EventTag::ContentionResolved`, `ContentionEventPayload`, and `EventView::contention_event_payload` for rendering)

## Problem

The observer binary at `crates/worldwake-cli/src/bin/observer.rs` renders the post-tick state of a simulation across 11 numbered sections (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 — verified by grep at the time of writing). Without a Section 12 covering contention events, the spec's "every resolution emits a queryable artifact" promise has no human-readable surface in the canonical inspection tool. This ticket adds `## Section 12 — Contention` to the observer output, rendering each per-tick `ContentionResolved` event with its rule, claimants, arrival ticks, queue positions, and outcomes. The optional `--contention-top-n` CLI flag surfaces the top-N contentions per run by claimant count for quick triage.

## Assumption Reassessment (2026-05-10)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The existing observer sections in `crates/worldwake-cli/src/bin/observer.rs` are at lines 680 (Section 3 — Decision History), 719 (Section 9 — Budget Exhaustion Snapshots), 858 (Section 10 — Critical Window Forensics), 1111 (Section 11 — Artifact Lifecycle), 3188 (Section 1 — Run Metadata), 3212 (Section 2 — Per-Agent Summary), 3373 (Section 4 — Anomaly Flags), 3388 (Section 5 — Raw Event Sample), 3557 (Section 6 — Per-Agent Belief Summary), 3661 (Section 7 — End-State Inventory & Resources), 3734 (Section 8 — Per-Agent Decision Summary). Sections are non-sequential in the file's source order (Section 11 appears at line 1111, Sections 1–8 at lines 3188+); the canonical numbering is the section header text itself, NOT source order. Section 12 is the next unused number.
2. The shared abstraction boundary under audit is the observer's read-only event-log consumption pattern. Per FND-14A footnote and FND-29 (debuggability), observer output is a derived view over authoritative event-log state and does not affect simulation semantics. Per the worldwake-validation-patterns.md "Read-Only Tooling Consumer" pattern, the observer reads via canonical accessors (`events_by_tag(EventTag::ContentionResolved)`), no shortcut accessors.
3. The observer's existing CLI flag conventions (e.g., `--critical-window-top-n` mentioned in the spec but not yet present in the binary per spot-check) follow the `--<section>-top-n` pattern. The new `--contention-top-n` flag follows this convention.
4. Section 12's per-tick rendering format matches the spec's example output: header line `Tick T — Contention: <facility-name>@<place-name> (<action-name>)`, indented `rule:`, indented `claimants (N):`, one line per claimant with `Agent X — arrived t=Y, position Z, <outcome>`. This matches existing sibling section formatting (Section 11 uses parallel indented body lines per item).
5. Per `docs/precision-rules.md` Rule 5 (verification surface mapping): observer output is a derived view; the verification surface for observer correctness is the headless render test that constructs a fixture event log and asserts the rendered text contains the expected section header and per-claimant rendering. The single-layer ticket maps the rendering invariant to focused observer-bin coverage.

## Architecture Check

1. Observer-only ticket: no engine mutation, no `worldwake-sim` or `worldwake-systems` change. Per FND-26, the observer reads authoritative state through canonical accessors and does not influence simulation behavior.
2. Per FND-29 (debuggability is a product feature), this section makes the spec's headline question — "why did Agent A get the slot at tick 412?" — answerable from the observer output without dropping into the event-log replay tool.
3. Per FND-28, no shim or alias path. Section 12 is a net-new section header; no existing section is being replaced or wrapped.
4. The `--contention-top-n` flag is optional; default behavior renders all contention events. The flag is a quality-of-life surface, not a correctness change.

## Verification Layers

1. Section 12 header appears in observer output when at least one `ContentionResolved` event exists in the log — focused observer-bin headless render test
2. Per-claimant rendering format matches spec — focused observer-bin test inspects rendered text for required strings
3. `--contention-top-n N` surfaces top-N by `total_claimants` — focused test fixtures with multiple contention events of varying claimant counts
4. Negative case: empty event log produces no Section 12 header (or an empty section, depending on observer convention) — focused observer-bin test
5. Single-layer ticket on the observer rendering surface; engine emission is in tickets 003/004, AI lookup in ticket 005, end-to-end goldens in ticket 007

## What to Change

### 1. Add Section 12 rendering function

In `crates/worldwake-cli/src/bin/observer.rs`, add a rendering function that takes the event log and writes the Section 12 body. Pattern follows existing sibling sections (e.g., Section 11 — Artifact Lifecycle at `:1111`).

Output format (per spec):
```
## Section 12 — Contention

Tick 412 — Contention: orchard@TownEdge (Harvest Apples)
  rule: ArrivalTime
  claimants (3):
    Agent A — arrived t=410, position 1, Granted
    Agent B — arrived t=411, position 2, QueuedAhead
    Agent C — arrived t=412, position 3, QueuedBehind
```

The function walks `events_by_tag(EventTag::ContentionResolved)`, reads each record through `EventView::contention_event_payload`, and renders each event in tick order. Resolve facility, place, action, and agent names through the existing display helpers (e.g., `entity_display_name(world, id)` per `Read-Only Tooling Consumer` pattern).

### 2. Wire the section into the observer's report-generation flow

Add a call to the new rendering function at the appropriate point in the observer's main report-build sequence. Place it after Section 11 to maintain the numerical ordering in the rendered output even though source order is non-sequential.

### 3. Add `--contention-top-n` CLI flag

Extend the observer's CLI argument parsing to accept an optional `--contention-top-n N` flag. When set, Section 12's rendering is filtered to the top-N contentions by `total_claimants` (descending). When unset, all contentions are rendered. Default: unset.

### 4. Focused observer-bin tests

Add focused tests in the observer's `#[cfg(test)]` block (boundary at `:4517`):
- `section_12_contention_renders_event_with_claimants`: fixture event log with one 3-claimant `ContentionResolved`; assert rendered text contains `## Section 12 — Contention`, the tick line, the rule line, and per-claimant lines.
- `section_12_contention_empty_log_renders_empty`: fixture event log with no `ContentionResolved` events; assert Section 12 either absent or empty per the observer convention used by sibling sections (e.g., Section 9 may render an empty body when no budget-exhaustion events exist; mirror that convention).
- `section_12_contention_top_n_filters_by_claimant_count`: fixture with 3 events of `total_claimants` 5, 3, 2; with `--contention-top-n 2`, assert only the 5- and 3-claimant events render.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Engine emission of `ContentionResolved` (tickets 003 and 004)
- AI population of `contention_event` (ticket 005)
- End-to-end goldens (ticket 007)
- Reformatting existing section headers — Section 12 is a net-new addition; sibling sections remain as-is
- New observer crate dependencies — the existing observer's deps suffice for event-log walking and string formatting

## Acceptance Criteria

### Tests That Must Pass

1. `section_12_contention_renders_event_with_claimants` — section header and per-claimant rendering present.
2. `section_12_contention_empty_log_renders_empty` — observer convention matched.
3. `section_12_contention_top_n_filters_by_claimant_count` — flag filters correctly.
4. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. Section 12 is observer-only: no event-log mutation, no world-state mutation.
2. Per FND-14, the observer reads authoritative event-log state, not omniscient world state for non-co-located facts.
3. The `--contention-top-n` flag is optional; default rendering shows all contentions.
4. Section 12 header text format `## Section 12 — Contention` matches existing sibling section conventions.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (existing `#[cfg(test)]` block at `:4517`) — 3 focused observer-bin headless render tests.

### Commands

1. `cargo test -p worldwake-cli observer`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
