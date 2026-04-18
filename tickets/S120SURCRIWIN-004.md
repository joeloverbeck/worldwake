# S120SURCRIWIN-004: Documentation for survival-forensics canonical helper

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation-only ticket.
**Deps**: `archive/tickets/S120SURCRIWIN-002.md`

## Problem

After `S120SURCRIWIN-001` and `S120SURCRIWIN-002` land the canonical runtime extractor and golden-harness wrappers, engineers debugging a long-run survival failure still have to discover the surface by reading source. The current project docs — `docs/golden-e2e-testing.md` and `docs/debugging-traces.md` — do not point survival-debugging work at the shared `CriticalWindowReport` helper. Without this documentation, the next long-run survival contradiction risks repeating the S116 pattern of bespoke `#[ignore]`'d reproducers and local probe code.

This ticket implements S120 deliverable D7: updates `docs/golden-e2e-testing.md` and `docs/debugging-traces.md` to point at the canonical helper as the default first step when investigating authored-critical window failures.

## Assumption Reassessment (2026-04-18)

1. Existing documentation targets:
   - `docs/golden-e2e-testing.md` — 473 lines, organized by `## Assertion Hierarchy`, `## Needs-State Assertion Guidance`, `## Survival Health Contracts`, `## Ordering Rules`, `## Trace Guidance`, `## Scenario Isolation`, and domain-specific calibration sections. A new subsection under `## Trace Guidance` or a new top-level `## Survival Critical-Window Forensics` section is the natural insertion point.
   - `docs/debugging-traces.md` — 148 lines, covering decision traces, action traces, tick alignment, observation strategy, system tick ordering, and force-control lifecycle. A new section referencing the canonical helper belongs here too.
   Both files validated during `/reassess-spec` pass on 2026-04-18.
2. Spec reference: `specs/S120-survival-critical-window-forensics.md` D7 (lines 194–196): "Update `docs/golden-e2e-testing.md` and/or `docs/debugging-traces.md` to point survival-debugging work at the shared critical-window report helper rather than ad hoc ignored reproducers."
3. Shared abstraction boundary: the documentation names `worldwake-ai::survival_forensics` as the canonical helper and `golden_harness::expect_*_window` helpers as the assertion wrappers. No new boundary is introduced; the docs describe the boundary already landed in the prior two tickets.
13. Adjacent contradiction audit: `docs/debugging-traces.md` currently describes decision traces and action traces as the primary debugging surface. The new section positions `CriticalWindowReport` as a *composed* surface over those two existing trace systems (plus authoritative physiology state) — not as a replacement. This is the in-scope consequence.

## Architecture Check

1. Documentation-only placement avoids interfering with the runtime or test surfaces — reviewers of this ticket can land it without running any simulation tests.
2. Cross-referencing both documentation targets (golden-e2e-testing.md for test authors, debugging-traces.md for debuggers) ensures the surface is discoverable from both entry points into survival-work.
3. Deferring this ticket until `S120SURCRIWIN-002` lands means the documentation can cite real helper names and real test paths rather than forward-declaring them.

## Verification Layers

1. Documentation accuracy → documentation-only ticket; verification is command-based (link validity, referenced symbols exist in current code).
2. No additional layer mapping applies per precision-rules Rule 5.

## What to Change

### 1. `docs/golden-e2e-testing.md` — add Survival Critical-Window Forensics section

Add a new section positioned after `## Survival Health Contracts` (around line 54) and before `## Ordering Rules`:

```markdown
## Survival Critical-Window Forensics

When a survival golden's `assert_authored_critical_runs` fails with "Agent X hunger exceeded authored critical ... for N consecutive ticks", do not reach for a bespoke `#[ignore]`'d reproducer or add local probe code. Use the canonical forensic helper instead.

### How

1. The survival goldens capture per-tick `CriticalWindowReport` data via the `SurvivalForensicExtractor` already wired into the harness (see `crates/worldwake-ai/tests/golden_harness/mod.rs`).
2. On assertion failure, attach or print the extractor's `finalize()` output via `dump_reports_for_debug(&reports)`.
3. Use `expect_sleep_progress_barrier_window`, `expect_wash_vs_water_competition_window`, or a new helper to assert the specific causal class you suspect.

### What the report tells you

A `CriticalWindowReport` records, per bounded-captured frame: selected goal, selected plan source, top competitors with typed provenance family, active action, typed exhaustion state (`FrontierExhausted`, `BudgetExhausted`, `Unsupported`), blocker summary, and local authoritative place state (water source / wash basin / sleep affordance / food source presence). This separates "planner failed despite local affordance" from "planner had no lawful local affordance and needed remote pursuit."

### When to add a new focused helper

If a new survival contradiction class emerges (e.g., bladder-vs-latrine competition under water contention), add a new `expect_*_window` helper in `crates/worldwake-ai/tests/golden_harness/` and a new focused test — not a scenario-level `#[ignore]`'d reproducer.
```

### 2. `docs/debugging-traces.md` — add Critical Window Forensics section

Add a new section at the end of the file (or after the Observation Strategy section), cross-referencing the canonical helper:

```markdown
## Critical Window Forensics

For survival failures where a homeostatic need stays above its authored critical threshold for tens or hundreds of ticks, raw decision traces and action traces are necessary but not sufficient. They prove single-tick facts; they do not compose into a stable read-model for a prolonged window.

Use `worldwake-ai::survival_forensics::CriticalWindowReport` (and its `SurvivalForensicExtractor`) as the composed read-model. It bundles per-frame decision-trace snapshots, action-trace snapshots, and authoritative local-place state with bounded frame capture (first 5 / last 5 / up to 5 evenly spaced interior / all change points).

Cross-reference: `docs/golden-e2e-testing.md` § Survival Critical-Window Forensics.
```

### 3. Optional cross-references

If the spec chain already mentions `docs/debugging-traces.md` elsewhere in the surviving documentation landscape (e.g., CLAUDE.md's Debugging section), leave those pointers untouched — this ticket only adds new content, it does not reorganize existing pointers.

## Files to Touch

- `docs/golden-e2e-testing.md` (modify — add Survival Critical-Window Forensics section)
- `docs/debugging-traces.md` (modify — add Critical Window Forensics section)

## Out of Scope

- Changes to `worldwake-ai::survival_forensics` or its consumers — all runtime/test code lives in prior tickets.
- Renaming or reorganizing existing sections of the two documentation files.
- Integration with `/scenario-analysis` skill documentation — the skill already consumes observer dumps; no documentation change is required for it to incorporate Section 9 rendered by `S120SURCRIWIN-003`.
- Observer binary Section 9 — see `S120SURCRIWIN-003`.
- Adding a new top-level docs file. A single new section in each of two existing files is the minimum-viable surface change per the spec.

## Acceptance Criteria

### Tests That Must Pass

1. `scripts/verify.sh` (if the repo has a docs-link verifier) or `cargo test --workspace` remains green (documentation-only change has no code test impact).
2. Referenced symbols exist: `worldwake-ai::survival_forensics::CriticalWindowReport`, `SurvivalForensicExtractor`, `expect_sleep_progress_barrier_window`, `expect_wash_vs_water_competition_window`, `dump_reports_for_debug` — all resolve to code landed by `S120SURCRIWIN-001` and `S120SURCRIWIN-002`.
3. Existing suite: `cargo test --workspace` remains green (no code change).
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings` passes (no code change).

### Invariants

1. New documentation sections cite only real, landed symbols (validated by grep against the current tree after `S120SURCRIWIN-001`/`002` merge).
2. No existing section in either documentation file is reordered, renamed, or removed.
3. The new sections reference each other (cross-link `docs/golden-e2e-testing.md` § Survival Critical-Window Forensics from `docs/debugging-traces.md` § Critical Window Forensics and vice versa if natural).

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `grep -n "CriticalWindowReport\|SurvivalForensicExtractor\|expect_sleep_progress_barrier_window\|expect_wash_vs_water_competition_window" docs/golden-e2e-testing.md docs/debugging-traces.md crates/worldwake-ai/src/survival_forensics.rs crates/worldwake-ai/tests/golden_harness/` — spot-check that every documentation reference resolves to a real symbol.
2. `cargo test --workspace` — full regression sweep (no code changes, pure sanity check).
3. `cargo clippy --workspace --all-targets -- -D warnings` — workspace lint (CI parity).
