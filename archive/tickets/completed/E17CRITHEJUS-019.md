# E17CRITHEJUS-019: Strengthen mixed-layer ticket and golden authoring for abstraction audits and traceability gaps

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ancillary `worldwake-core` / `worldwake-systems` institutional-belief overwrite fix required to restore a green workspace during final verification
**Deps**: None

## Problem

Recent E17 work exposed a repeated documentation/process failure mode: a ticket could be formally “well structured” yet still under-specify the abstraction boundary that actually needed reassessment. That allowed stale narratives like “feature X is missing” to survive long enough to drive implementation churn, when the real issue was a broader shared-boundary contradiction. The repository already has traceability-escalation guidance in `docs/precision-rules.md`, but the canonical ticket authoring contract and template do not yet connect that rule tightly enough to boundary-audit-first ticket drafting and invariant restatement for failing mixed-layer scenarios.

This is a documentation/process gap, but it matters architecturally because weak ticket framing encourages workaround changes that violate the foundations mandate against patches and fossilized abstractions.

## Assumption Reassessment (2026-03-26)

1. `tickets/README.md` already requires `Assumption Reassessment`, `Architecture Check`, `Verification Layers`, exact runnable commands, and divergence-first ticket correction. The live gap is not missing section structure; it is that the mandatory checks still do not explicitly require authors to name the shared abstraction boundary or classify newly exposed adjacent contradictions before implementation.
2. `docs/precision-rules.md` already covers phase distinction, layer precision, scenario isolation, divergence handling, and `Traceability Escalation`. The missing contract is narrower: the canonical ticket authoring rules do not yet force a mixed-layer ticket to declare the exact boundary or data contract under audit, restate the intended invariant before trusting a failing golden, or classify whether adjacent contradictions are in-scope consequences versus separate follow-up work.
3. `tickets/_TEMPLATE.md` already includes prompts for live `GoalKind`, ordering layers, first failure boundaries, scenario isolation, and traceability-related reassessment. The template is still missing explicit prompts for:
   - the exact abstraction boundary or shared contract under reassessment
   - whether newly uncovered contradictions are required consequences, separate bugs, or future cleanup
   - the intended invariant of a failing golden before downstream assertions are authored
4. `AGENTS.md` already tells contributors to update stale tickets first and to prefer decision/action traces over ad-hoc debugging. It does not yet give a concise workflow rule that mixed-layer implementation starts with a boundary audit and invariant restatement before trusting the failing scenario narrative.
5. Coverage gap classification:
   - no engine/runtime contradiction is being claimed here
   - the gap is repository guidance and ticket/spec authoring discipline
   - verification is primarily doc-content inspection; runtime suites are regression safety checks rather than the behavioral contract of this ticket
6. This remains a docs/process ticket. Reassessment does not show a production invariant that requires `Engine Changes: Yes`, so keeping `Engine Changes: None` is still accurate after the narrower scope correction.
7. The changes align with `docs/FOUNDATIONS.md`: the project rejects workaround architecture and requires causally legible, traceable systems. Tightening the authoring contract directly supports the opening architectural mandate plus P3, P7, P13, P16, P24, and P27.
8. Mismatch + correction: the original ticket overstates what is missing in the current docs and implies a broader rewrite of process guidance than is warranted. The corrected scope is to tighten the canonical prompts and mandatory checks in place, avoid duplicating `docs/precision-rules.md`, and keep `AGENTS.md` limited to concise workflow guidance that points back to the canonical contracts.
9. Verification-time divergence: `cargo test --workspace` exposed a pre-existing failure in `office_actions::tests::declare_support_commit_sets_and_overwrites_support_declaration`. The failure was not caused by the docs changes, but full-green finalization required a minimal architectural fix so self-authored support declaration overwrites replace the actor's prior belief under the same key instead of accumulating a false `Conflicted` read.

## Architecture Check

1. The clean fix is to strengthen the repository’s canonical authoring contract so mixed-layer changes must explicitly audit the shared abstraction boundary, restate the intended invariant, classify adjacent contradictions, and name the proof surface expected to explain the behavior.
2. This is better than encoding the lesson in one archived ticket or broadening `AGENTS.md` into a second precision-rules document. The failure mode is procedural and recurring, so the durable fix belongs in the canonical ticket authoring docs and template, with only a short workflow reminder in `AGENTS.md`.
3. No backwards-compatibility aliasing applies here; update the existing authoritative docs in place rather than adding parallel guidance.
4. The ancillary runtime fix also follows the clean architecture rule: keep append semantics for accumulating observed institutional claims, and add an explicit overwrite path for self-authored "current declaration" knowledge. That is cleaner than teaching self-declaration callers to depend on conflict-tolerant reads or weakening the failing test.

## Verification Layers

1. Mixed-layer abstraction audit and adjacent-contradiction classification are stated in the canonical ticket authoring docs -> doc content inspection
2. Golden invariant-restatement and competing-branch isolation are prompted in the canonical template and workflow docs -> doc content inspection
3. Traceability-gap escalation remains canonical and non-duplicative across docs -> doc content inspection
4. Workspace behavior remains unaffected by the doc-only change set -> regression safety check via workspace `cargo test` and `cargo clippy`
5. Self-authored support declaration overwrite keeps actor belief consistent with the latest declaration -> focused `worldwake-core` unit coverage plus focused `worldwake-systems` action coverage

## What to Change

### 1. Tighten `tickets/README.md`

Add explicit rules for mixed-layer tickets:
- identify the shared abstraction boundary or data contract under audit before implementation
- restate the intended invariant before trusting a failing golden or mixed-layer scenario narrative
- classify newly exposed issues as:
  - required consequences of the intended change
  - separate bugs uncovered during reassessment
  - future cleanup that must become its own ticket
- if traceability proves an outcome but not enough provenance, require a follow-up traceability ticket instead of broadening downstream assertions

### 2. Update `tickets/_TEMPLATE.md`

Add prompts that force authors to state:
- the exact abstraction boundary under reassessment
- the intended invariant of any failing golden or mixed-layer scenario before trusting the narrative
- the intended invariant of any golden scenario
- the lawful competing behaviors intentionally excluded from setup
- the trace surface expected to explain the behavior, and what to do if it proves insufficient
- whether newly exposed adjacent contradictions are required consequences, separate bugs, or follow-up cleanup

### 3. Update `AGENTS.md`

Add concise contributor guidance:
- mixed-layer implementation work begins with an abstraction-boundary audit, not code changes
- do not trust a failing golden until the invariant under test and the excluded lawful branches are restated explicitly
- when traces are one layer too coarse, prove the immediate behavior at the strongest available lower layer and open a follow-up traceability ticket if the missing provenance matters architecturally

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)
- `AGENTS.md` (modify)

## Out of Scope

- Any runtime/engine traceability implementation
- Rewriting existing tickets en masse
- Changing the archival workflow

## Acceptance Criteria

### Tests That Must Pass

1. `tickets/README.md` explicitly requires mixed-layer tickets to name the abstraction boundary under audit
2. `tickets/_TEMPLATE.md` prompts authors for abstraction-boundary restatement, invariant restatement, competing-branch isolation, adjacent-contradiction classification, and traceability-gap handling
3. `AGENTS.md` explicitly states the abstraction-audit-first and invariant-restatement workflow rules without forking the canonical precision contract
4. No contradictory guidance remains between the updated docs
5. Existing workspace tests and lint still pass after the doc changes

### Invariants

1. Repository guidance continues to steer contributors toward root-cause architectural fixes rather than local workaround patches, consistent with `docs/FOUNDATIONS.md`
2. Documentation remains single-source and non-duplicative where possible; updates strengthen the canonical contracts instead of forking them

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — added `replace_institutional_belief_overwrites_existing_key_without_conflict` to prove the new overwrite path removes stale same-key institutional claims instead of producing a false conflict.
2. `crates/worldwake-core/src/world_txn.rs` — added `replace_institutional_belief_overwrites_existing_key_and_updates_world_on_commit` to prove the transactional API persists overwrite semantics through commit.
3. Existing regression coverage: `crates/worldwake-systems/src/office_actions.rs` `declare_support_commit_sets_and_overwrites_support_declaration` now passes against the corrected overwrite path and remains the action-layer proof surface.

### Commands

1. `sed -n '1,260p' tickets/README.md`
2. `sed -n '1,260p' tickets/_TEMPLATE.md`
3. `sed -n '1,260p' AGENTS.md`
4. `sed -n '1,260p' docs/precision-rules.md`
5. `cargo test -p worldwake-core replace_institutional_belief_overwrites_existing_key_without_conflict`
6. `cargo test -p worldwake-core replace_institutional_belief_overwrites_existing_key_and_updates_world_on_commit`
7. `cargo test -p worldwake-systems office_actions::tests::declare_support_commit_sets_and_overwrites_support_declaration -- --exact`
8. `cargo test --workspace`
9. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-26
- Actual changes:
  - tightened the canonical ticket authoring contract in `tickets/README.md`
  - extended `tickets/_TEMPLATE.md` with explicit abstraction-boundary, invariant-restatement, adjacent-contradiction, and traceability-escalation prompts
  - added concise workflow reminders to `AGENTS.md` without duplicating `docs/precision-rules.md`
  - added an ancillary institutional-belief overwrite path in `worldwake-core` and used it for self-authored support declarations in `worldwake-systems`
- Deviations from original plan:
  - the original plan assumed docs-only work; full workspace verification exposed a pre-existing failing support-declaration belief-overwrite path, so a small production fix was added to restore a green baseline
- Verification results:
  - focused tests passed for the new overwrite helper and the existing support-declaration regression
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
