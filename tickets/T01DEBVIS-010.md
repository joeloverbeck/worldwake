# T01DEBVIS-010: Manual QA checklist + verification pass

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None
**Deps**: [T01DEBVIS-005](../archive/tickets/T01DEBVIS-005.md), [T01DEBVIS-006](../archive/tickets/T01DEBVIS-006.md), [T01DEBVIS-007](../archive/tickets/T01DEBVIS-007.md), T01DEBVIS-008, T01DEBVIS-009, T01DEBVIS-011

## Problem

The visualizer has no automated golden coverage by design (spec T01 Non-Goals: "Golden E2E coverage for visualizer output: no new golden scenarios … visual correctness is manual"). Spec §D13 specifies a 13-step manual QA checklist that must be documented in `crates/worldwake-visualizer/README.md` and run on each landed scenario in `scenarios/` before T01 is considered complete. This ticket lands the README content and runs the checklist as a verification pass on the merged crate.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. T01DEBVIS-005 replaced the original single-paragraph README stub with an interim canvas/manual-QA section. This ticket still owns replacing that partial checklist with the full 13-step QA checklist content from spec §D13.
2. `scenarios/` is the canonical scenario directory at the workspace root. Reassessment 2026-04-25 confirmed the directory exists and contains the survival-* family used by current goldens.
3. Tooling-only documentation ticket — no engine changes, no widget code changes.

## Architecture Check

1. Manual QA documentation is the deliverable; FND-31 (Validation and Falsification Are First-Class) explicitly recognizes that not every system needs golden E2E coverage. The visualizer's manual QA is the validation surface.
2. The README is the canonical home for the checklist — keeping it in-tree (rather than in `docs/`) ties it to the crate's release lifecycle.

## Verification Layers

1. Documentation correctness → `cargo doc -p worldwake-visualizer` builds without warnings; the README is a Markdown file, not a doc-comment, so the doc build is a sanity check on rustdoc-reachable items.
2. Manual QA pass → run the 13-step checklist against each scenario in `scenarios/`; record pass/fail per scenario in the ticket's verification notes (not in the README itself, which is generic).
3. Per template item 6: documentation/verification ticket; no automated decision/action/event-log assertions apply.

## What to Change

### 1. Replace the interim `README.md` with full QA checklist

Modify `crates/worldwake-visualizer/README.md` — replace the interim canvas/manual-QA notes from T01DEBVIS-005 with the 13-item checklist from spec T01 §D13 verbatim:

1. `cargo run -p worldwake-visualizer -- scenarios/<name>.ron` opens window within 2s.
2. `cargo run -p worldwake-visualizer -- --help` prints clap-derived usage and exits.
3. Places render without overlap; graph fits in window via auto-fit on first frame.
4. Dashed edges render with tick-count labels at midpoints.
5. Pan (middle-drag) and zoom (wheel) work on the canvas — confirms `egui::Scene` is reachable on the pinned `egui` version. If `Scene` is unavailable on the resolved version, fall back to a hand-rolled pan/zoom container before continuing.
6. Space advances exactly one tick (tick counter in header increments by 1).
7. Play + speed slider: tick counter advances at approximately the configured rate.
8. Reset returns tick to 0 and places agents at their initial locations.
9. Hover agent → tooltip with zone-colored need bars, including Pain/Danger when non-zero; bars match numeric values.
10. Click agent → modal opens; all 6 tabs render without panic.
11. Traces tab populates with entries after several ticks.
12. Beliefs tab shows entries from `AgentBeliefStore`, `LastSeenMemory`, `ExpectationStore`, and `SourceReliability` after the agent has observed something.
13. Transit: for `survival-scattered.ron`, an agent on a multi-tick edge is visibly lerped across ticks.

Add a "How to run" section with the standard cargo invocations and a "Known scenarios" section listing the scenarios the visualizer has been verified against at landing time.

### 2. Run the QA pass on landed scenarios

For each `.ron` file in `scenarios/`, run the 13-step checklist and record pass/fail in the ticket's verification notes (the user's review surface — not committed to the repo). Any failure is a follow-up bug report against the responsible ticket (T01DEBVIS-001 through -011).

### 3. Add screenshots placeholder

Add a `## Screenshots` section to the README with a "TBD" note. Spec Open Questions §3 defers screenshot/canvas export to a later iteration.

## Files to Touch

- `crates/worldwake-visualizer/README.md` (modify — replace stub with full QA checklist)

## Out of Scope

- Adding actual screenshots (deferred per spec Open Questions §3).
- Persistent UI settings (`eframe::Storage`) — deferred per spec Open Questions §2.
- Cross-scenario diffing — explicit non-goal in spec.
- Replay scrubbing — explicit non-goal in spec.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo doc -p worldwake-visualizer` builds without warnings.
2. The 13-step QA checklist is fully reproduced in `crates/worldwake-visualizer/README.md`.
3. Manual QA pass run on at least: `survival-baseline.ron`, `survival-scattered.ron`, and one additional landed scenario; all 13 steps pass on each.
4. Existing suite: `cargo test --workspace` passes.

### Invariants

1. The README is the single source for the manual QA contract — no parallel checklist in `docs/`.
2. The visualizer continues to require zero engine code changes — this ticket is documentation only.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo doc -p worldwake-visualizer`
2. `cargo run -p worldwake-visualizer -- --help`
3. `cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron` (manual full-checklist pass)
4. `cargo run -p worldwake-visualizer -- scenarios/survival-scattered.ron` (manual full-checklist pass; verify item 13 — transit lerp)
5. `./scripts/verify.sh`
