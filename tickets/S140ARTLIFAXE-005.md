# S140ARTLIFAXE-005: Observer Section 11 — Artifact Lifecycle

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — observer-side rendering only; reads existing `ArtifactHeader` axis fields (post-001) and decoded `ArtifactTransition` events (post-001/002)
**Deps**: archive/tickets/S140ARTLIFAXE-001.md, S140ARTLIFAXE-002

## Problem

After 001 and 002 land, every artifact in the run carries 5 axis values plus an event-log history of `ArtifactTransition` events showing how each axis moved. Without observer surfacing, this history is invisible to the post-run inspector. Per spec D8, the observer adds Section 11 ("Artifact Lifecycle") iterating artifacts referenced in the run and rendering per-axis state plus the axis-transition timeline. Sections 1-10 are already in use (verified at reassess: `grep "## Section" observer.rs` showed `Section 1`–`10`); Section 11 is the appropriate insertion identifier.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-cli/src/bin/observer.rs` currently has Sections 1-10 (verified at reassess via `grep "## Section" observer.rs`). The most-recent section is `## Section 10 — Critical Window Forensics` at line 858. After 001 lands, `ArtifactHeader` carries 5 axis fields readable directly; after 002 lands, `EventTag::ArtifactTransition` events are emitted and decodable for the per-axis history view.
2. Spec deliverable D8 specifies the section header text (`## Section 11 — Artifact Lifecycle`), the per-artifact rendering format, and that existing sections are not renumbered.
3. **Cross-system shared abstraction boundary**: The boundary under audit is the observer's read interface (consuming public APIs from `worldwake-core` and `worldwake-sim` per the "Read-Only Tooling Consumer" pattern). No engine code is added; the observer reads `ArtifactHeader` via component getters and decodes `ArtifactTransition` events via the event-log scan path used by other sections.
6. **AI-regression layer**: N/A — this ticket is observer-only (no engine change, no AI behavior change). Per `docs/precision-rules.md` Rule 3, the verification layer for tooling-only specs is the headless render test on the CLI surface.
13. **Adjacent-contradiction classification**: If implementation discovers that observer's existing event-log scan path doesn't decode `EventTag::ArtifactTransition` (because the decode dispatch wasn't fully wired by 001/002), classify it as a missed scope item from 001 and address it as part of this ticket — not a separate follow-up.

## Architecture Check

1. Observer rendering is a derived view (FND-27 cache-not-truth). Section 11's content is computed each tick (or at render time) from authoritative state (`ArtifactHeader` field values) plus an event-log scan (decoded `ArtifactTransition` events). Deleting the section and recomputing produces the same content.
2. The section is additive — Sections 1-10 are unchanged; no renumbering. This preserves the audit trail of prior reports that referenced specific section numbers.
3. Per the "Read-Only Tooling Consumer" pattern from `references/worldwake-validation-patterns.md`, the observer reads through public APIs only. No new shortcut accessors (`active_artifact_of`, etc.) are introduced.

## Verification Layers

1. Section 11 header text + insertion order → headless observer-render test asserting the section appears between Sections 10 and the next existing section, with the exact header text.
2. Per-artifact axis rendering → render test asserting all 5 axis values are rendered for a known fixture artifact.
3. Axis-transition history rendering → render test asserting that every emitted `ArtifactTransition` event for the fixture appears in the section's timeline view.
4. Single-layer scope: this is a tooling-only ticket; no engine, AI, or scenario semantics change. The verification surface is the rendered observer output text (and any structured output the observer produces).

## What to Change

### 1. Add Section 11 emission in `observer.rs`

Insert a new `writeln!(out, "## Section 11 — Artifact Lifecycle\n")` block after the existing Section 10 emission (line 858 pre-001/002; pin during implementation).

### 2. Iterate artifacts referenced in the run

Use the artifact enumeration approach from existing observer sections (likely an iteration over `World::entities_with_name_and_artifact_data()` or analogous; pin during implementation by examining how Sections 5/6 enumerate). For each artifact, read `ArtifactHeader` via the component getter and render the 5 axis values per the spec's example format:

```
Bounty B7 (issued tick 100, by office Watch, place TownSq)
  existence: Exists
  visibility: Posted (since t=102)
  legal_effect: Fulfilled (t=480, by Hunter Theron, evidence Wolf-Pelt)
  credibility: Credible
  actionability: Closed (t=480, cause: BountyFulfilled)
  axis history: 8 transitions
```

### 3. Render the axis-transition timeline

Scan the event log for `EventTag::ArtifactTransition` events filtered by `artifact == <current artifact id>`. Render the count (`axis history: N transitions`). Optionally extend with a per-transition listing if the spec example warrants — start with the count line and add a `--verbose` axis-history listing only if implementation cost is small.

### 4. Add a render test

Construct a small fixture (programmatic, not a `.ron` scenario) with one bounty that goes through fulfillment, run the observer-render path, and assert the Section 11 output contains the expected substrings.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — add Section 11 emission)
- Likely: `crates/worldwake-cli/src/bin/observer.rs` test module or a sibling integration test (modify — render-test fixture)

## Out of Scope

- Engine artifact lifecycle (covered by 001, 002).
- Planner observability (covered by 003).
- Scenario authoring (covered by 004).
- E2E goldens (covered by 006).
- Renumbering existing Sections 1-10.
- Replacing Section 5 ("Raw Event Sample") with artifact rendering — Section 5 is preserved per spec; Section 11 is the new insertion.

## Acceptance Criteria

### Tests That Must Pass

1. New observer-render test: `section_11_artifact_lifecycle_renders_axis_state` — fixture artifact's 5 axis values appear in the rendered output.
2. New observer-render test: `section_11_renders_axis_transition_count` — fixture artifact's transition count matches the emitted event count.
3. Existing observer tests pass unchanged (no renumbering): `assert!(out.contains("## Section 3 — Decision History"))` etc. continue to pass.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. Section header text is exactly `## Section 11 — Artifact Lifecycle` (single space sequence, em-dash matching existing sections).
2. Sections 1-10 are unchanged and not renumbered.
3. Section 11 is rendered if any artifact exists in the run; absent (or empty-section) when no artifact references appear.
4. The observer reads only via public APIs (no shortcut accessors introduced).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (modify) — add inline render test `section_11_artifact_lifecycle_renders_axis_state` constructing a programmatic fixture and asserting the rendered Section 11 contains all 5 axis label substrings.
2. `crates/worldwake-cli/src/bin/observer.rs` (modify) — add `section_11_renders_axis_transition_count` exercising the event-log scan path.

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test --workspace`
3. `scripts/verify.sh`
