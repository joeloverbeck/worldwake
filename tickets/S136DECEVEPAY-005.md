# S136DECEVEPAY-005: Observer Section 3 single-line summary extension

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (CLI-only — observer rendering)
**Deps**: archive/tickets/S136DECEVEPAY-001.md

## Problem

Spec D8: the observer's Section 3 (Decision History) renderer at `crates/worldwake-cli/src/bin/observer.rs:421` (`decision_payload_summary`) currently produces single-line summaries that do not include the new fields added by ticket 001 (`rejection_dimension`, `decisive_*`, `assumptions`). The existing format is a Markdown table `| Tick | Agent | Event | Payload Summary |` (rendered by `render_decision_history_section:607`); the single-line invariant is enforced by the test `decision_payload_summary_is_single_line_for_goal_committed:5744`.

This ticket extends each affected payload's summary with compact suffixes — e.g., `goal=Eat motive=18420 alts=2 dim=MotiveScore assume=2` for `GoalCommitted` — preserving the single-line table format. The detailed multi-line block format proposed in earlier S136 drafts is explicitly out of scope (spec Non-Goal — would break the existing test invariant).

## Assumption Reassessment (2026-05-06)

1. `decision_payload_summary` at `observer.rs:421` is a `match payload { ... }` over `DecisionEventPayload` variants. Each arm produces a single `String`. Variants whose corresponding payloads are widened in this spec (per the per-tag field map: `GoalCommitted`, `PlanAdopted`, `BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`, `SourceExpectationFailure`) need new compact suffixes; the other variants are unchanged.
2. The existing test `decision_payload_summary_is_single_line_for_goal_committed:5744` asserts the single-line invariant for `GoalCommitted` only. Per spec validation, this ticket extends this test (or adds sibling tests) to cover the other 5 widened tags so the single-line guarantee is enforced everywhere this ticket touches.
3. Existing test `render_decision_history_section_covers_all_variants:5697` asserts every `EventTag` variant has a render arm. This ticket does not add new variants, so the test is not directly affected — but the additive payload summaries must not break its assertions.
4. CLI-only ticket — no engine changes, no simulation state mutations. Items 4-15 of the template are not applicable. Verification Layers reflect the CLI-only proof surface (focused unit on `decision_payload_summary` is the canonical surface).
5. Boundary under audit: the observer's payload-rendering surface. Compact-suffix conventions (proposed; finalize at implementation review):
   - `dim=<tag>` for `GoalCommitted` only — sourced from `rejected_alternatives[0].rejection_dimension` if `Some`; rendered with the `RankedGoalComparisonDimensionTag` variant name.
   - `decisive=B<n> R<n> O<n>` — counts of the three Vecs; emitted only when at least one is non-empty.
   - `assume=<n>` — count of `assumptions`; emitted only when non-empty.

## Architecture Check

1. CLI rendering is a derived view (FND-27 — observer Section 3 reconstruction is not authoritative state). The new suffixes do not introduce any cache or persisted aggregate.
2. Single-line invariant preserved: each summary remains a one-string-per-row Markdown-table cell. The existing test `decision_payload_summary_is_single_line_for_goal_committed:5744` (and the new sibling tests) continue to assert no `\n` in any rendered summary.
3. The compact-suffix format avoids the multi-line block rendering proposed in earlier S136 drafts — preserves the existing test invariant and defers richer rendering to a separate observer-format spec (spec Non-Goal).
4. Suffix-elision when fields are empty (e.g., omit `assume=` when `assumptions.is_empty()`) keeps summaries readable for events where the new fields don't apply.

## Verification Layers

1. Single-line invariant → existing test `decision_payload_summary_is_single_line_for_goal_committed:5744` extended; new sibling tests for the other 5 widened tags.
2. Field rendering → focused unit per affected variant asserting the suffix appears with the expected count when populated, and is elided when fields are empty.

## What to Change

### 1. Extend `decision_payload_summary` arms

In `crates/worldwake-cli/src/bin/observer.rs:421`, append compact suffixes to the arms for the 6 widened payload variants. Use `format!`-with-conditional-extension so that empty fields elide their suffix. Sketch for `GoalCommitted`:

```rust
DecisionEventPayload::GoalCommitted(inner) => {
    let mut s = format!(
        "goal={:?} motive={} alts={}",
        inner.goal_key.kind, inner.motive_score, inner.rejected_alternatives.len()
    );
    if let Some(dim) = inner
        .rejected_alternatives
        .first()
        .and_then(|alt| alt.rejection_dimension)
    {
        write!(&mut s, " dim={dim:?}").unwrap();
    }
    if !inner.assumptions.is_empty() {
        write!(&mut s, " assume={}", inner.assumptions.len()).unwrap();
    }
    s
}
```

Apply the same pattern to `PlanAdopted` (`assume=` only), `BlockerRecorded` / `ReplanTriggered` / `ExpectationMismatch` (`decisive=B<n> R<n> O<n>` and `assume=` when populated), and `SourceExpectationFailure` (`decisive=` only — no assumptions per spec D4).

### 2. Extend the single-line invariant test

At `observer.rs:5744`, extend the existing test (or add sibling tests) for each of the 5 other widened tags. Each test:
- Constructs a payload with the new fields populated to non-trivial values.
- Calls `decision_payload_summary`.
- Asserts the result contains no `\n`.
- Asserts the expected suffix appears (e.g., `result.contains("decisive=B1 R0 O2")`).
- Adds a parallel empty-field case asserting suffix elision.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — `decision_payload_summary` arms and single-line invariant tests)

## Out of Scope

- Multi-line block rendering proposed in earlier S136 drafts (spec Non-Goal — would break the test invariant).
- Resolving typed-ref addresses to display-ready entity names (the observer renders counts; replay can drill into specifics).
- `GoalOffered` / `GoalSuppressed` / `GoalAbandoned` / `GoalSuspended` / `PlanInvalidated` / `RepairApplied` rendering — no fields added by S136 to these payloads.
- New observer subsections.
- Engine, planner, or simulation-state changes.

## Acceptance Criteria

### Tests That Must Pass

1. Existing test `decision_payload_summary_is_single_line_for_goal_committed:5744` passes (extended to assert new suffixes when fields populated; extended with empty-field elision assertion).
2. New sibling tests covering the other 5 widened payload variants assert single-line invariant + suffix presence/elision.
3. Existing test `render_decision_history_section_covers_all_variants:5697` passes unchanged.
4. Existing CLI suite passes: `cargo test -p worldwake-cli`.

### Invariants

1. Every `decision_payload_summary` arm output contains no `\n` (single-line invariant — preserves test contract at `observer.rs:5744`).
2. Suffixes appear only when their underlying field is non-empty (compact rendering).
3. The Markdown-table column structure of Section 3 is unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs::tests::decision_payload_summary_is_single_line_for_goal_committed` — extend with `dim=` and `assume=` suffix assertions plus elision case.
2. `crates/worldwake-cli/src/bin/observer.rs::tests` — new sibling tests for `PlanAdopted`, `BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`, `SourceExpectationFailure` single-line + suffix coverage.

### Commands

1. `cargo test -p worldwake-cli decision_payload_summary`
2. `cargo test -p worldwake-cli observer`
3. `cargo test -p worldwake-cli`
4. `./scripts/verify.sh`
