---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase. Do not stop at intermediate reassessment or partial fallout; continue until the ticket is completed, fully verified, or blocked by a user decision that requires 1-3-1.

## Workflow

### 0. Classify ticket shape and pick the right path

Before running the full workflow, classify the ticket:

**Small/local tickets** (fast path) — single-file additive CLI/tooling/reporting/action-registry change, narrow helper extraction, formatting update, or other owned-module additive change with no shared type/planner/golden/persistence/cross-crate fallout expected. Typical examples include a single-file transport/action registration, local handler addition, narrow helper extraction, or bin-local coverage for factored logic:
1. Resolve the exact live ticket/spec path, including typos or shorthand.
2. Confirm the dependency path and the exact owned symbol/file boundary.
3. Run a narrow constructor/usage sweep for the changed shape: confirm the named symbols and accessors exist, search local callers/render sites, check obvious constructor or test-helper fallout, and identify the narrowest real proof entry point.
4. Implement the owned change with focused proof first.
5. Run the affected crate's tests as the normal broadened proof for the ticket. For Rust tickets, if the ticket's Test Plan or repo norms call for CI-matching clippy, run `cargo clippy --workspace --all-targets -- -D warnings` as part of normal broadened verification; use compile/lint fallout to catch remaining shared-shape literals/helpers and local cleanup.
6. Close out the ticket with the actual verification set and tracked-vs-untracked note. This normally includes updating the ticket file itself with completion metadata such as `Status`, `Outcome`, `Deviations` when needed, and `Verification Result`, not just reporting those details in the conversation.

For CLI/tooling-only tickets, if the owned logic can be factored into local helpers, prefer bin-local `#[cfg(test)]` coverage over command-only validation.

When a ticket stays local to one CLI/tooling module but the live computation surface and the final render/output surface are different sections of that same file, name both local boundaries during reassessment and prefer a shared local helper over duplicating logic between sections.

Do not skip reassessment for small tickets, but scale it down: read the ticket, cited references, and owned symbol/file; confirm the dependency path is present; run a narrow existence/fallout sweep for prior implementation or obvious constructor/usage fallout. Do not force the full Step 2 matrix when the owned surface is genuinely small and local.

For small/local tickets, load the reference docs only if reassessment exposes ambiguity, mismatch, or broader fallout. The normal fast path is the ticket, its cited references, the owned symbol/file boundary, focused proof, the affected crate's tests, and any explicitly required CI-matching lint surface.

For straightforward shared-type additive tickets (new field on an existing struct/component, derive-safe enum payload addition, or similar constructor fallout with no boundary dispute yet visible), start with the ticket, cited spec/docs, `references/reassessment-checks.md`, `references/verification.md`, and `references/closeout.md`. During the constructor fallout sweep, distinguish full manual struct literals from partial literals that already inherit new fields via `..Default::default()` or equivalent helpers before accepting the ticket's cited file list as real edit scope. Load `mismatch-handling.md`, `scope-extraction.md`, or `implementation-discipline.md` only if reassessment exposes a mismatch, ownership ambiguity, or non-mechanical implementation choice.

Single-file planner-root, snapshot-completeness, planner-traceability, or AI carriage-path tickets still use the full workflow when the contract under audit crosses the planner boundary even if the eventual edit surface stays narrow and local.
Golden E2E tickets motivated by planner failures, observer reports, or scenario-specific regressions also use the full workflow even when the landed edit surface is one test file and the ticket remains test-only.

**All other tickets** — use the full workflow below (Steps 1-8).

For full-workflow tickets, start by loading `references/reassessment-checks.md`, then `references/verification.md` and `references/closeout.md`; load `mismatch-handling.md`, `scope-extraction.md`, and `implementation-discipline.md` when reassessment or implementation reaches those steps or exposes the need. If the ticket lands a new module, type surface, helper, or staged function/method ahead of downstream integration, check during implementation whether that surface will be intentionally unused on landing and mark it deliberately before broad verification so staged scaffolding does not fail CI-matching lint passes.

When the ticket was authored by `/spec-to-tickets` in the current session from a freshly reassessed spec, scale reassessment to a targeted sweep: confirm the ticket's owned types still exist at stated paths, check for exhaustive matchers on modified enums, verify trait bounds on any types used in new test code, check for manual struct literals of modified types (constructors, test helpers, `from_*_for_test` patterns) that would need updating for new fields, and before adding new test-only accessors or helpers, check whether existing test infrastructure (e.g., `ActualWorldState::from_world`, test harness methods) already provides the needed capability.

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference (specs, docs, code symbols, test files).
3. When the user supplies a glob, shorthand, or obvious near-match typo, confirm the exact live file path before reading or relying on it.
4. When the ticket name implies a numbered family or the user cites a parent spec, search for sibling tickets in the same family and confirm whether adjacent missing substrate is already owned elsewhere before broadening or narrowing the current ticket.
5. Check whether the active ticket file is tracked or untracked in the current worktree. Untracked ticket drafts are valid active state, but they will not appear in ordinary `git diff` output. Keep this in mind during diffs, close-out evidence, and follow-up ticket reporting throughout the workflow.
6. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

### 2. Reassess assumptions before coding

Verify the ticket against the current codebase, not stale architectural memory. Check `Deps` — confirm each dependency is present on the current branch. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit.

When a ticket is motivated by an observer report, golden failure, or named scenario condition, verify the exact motivating substrate before distilling the harness or fixture. Confirm the scenario file, named places/entities, travel graph, and the reported failure location/path still match the ticket's execution narrative, then record what is preserved versus intentionally omitted in the distilled setup. Do not substitute a nearby prototype-world approximation when the ticket's claimed proof depends on a specific scenario/location condition.

Before trusting the ticket as executable, cross-check its internal sections for contradictions. Reconcile conflicts between `Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, `Acceptance Criteria`, `Verification Layers`, and any explicit in/out-of-scope notes before coding. Treat `Out of Scope` and any explicit "no new variants", "tests only", or "Engine Changes: None" claims as first-class contradiction surfaces during reassessment. If those sections disagree about ownership, proof surface, or whether production changes are required, update the ticket first instead of carrying the contradiction into implementation.

For cross-crate accessor, trait-surface, or API-surface tickets, verify the real downstream caller-facing boundary before coding, not just the immediate trait or type named in the ticket. If live callers consume the data through a broader wrapper, supertrait, blanket impl, or facade surface, correct the ticket to that owned boundary before editing code.
When a ticket adds a field to a shared struct/component that is serde-deserialized from scenarios, saves, or other explicit inputs, verify omitted-field compatibility during reassessment instead of assuming the struct's `Default` impl is sufficient. Decide whether the ticket must own a field-level serde default, explicit input migration, or fixture/scenario updates before implementation.
When a ticket adds internal diagnostic, trace, or metadata carriage, preserve existing public/external call signatures unless the ticket explicitly owns that API change; prefer an internal helper, wrapper, or traced variant for the new carrier rather than widening public fallout by default.
When a ticket adds a field to an internal diagnostic, trace, or metadata struct, sweep the full carriage chain before accepting a single-file scope: producer, internal conversion/wrapper layers, renderers or report surfaces, manual struct literals, and all-target test/CLI consumers. Treat those as part of the owned reassessment boundary even when the original ticket only names the producer file.

When the ticket includes a proposed function signature, helper sketch, or API snippet, verify that the live helper contract actually supports that shape. If the current branch requires an additional dependency, carrier, or argument to use the cited helper lawfully, correct the signature sketch and matching `What to Change` snippets during reassessment before implementation.

When a ticket relies on an existing helper or accessor, verify not only that the symbol exists on the expected boundary but that its live implementation returns the intended semantic quantity for the concrete subject type under test. Do not trust plausible naming alone when helpers can be overloaded, entity-type-specific, or historically repurposed; if the live helper computes a different concept than the ticket assumes, correct the ticket to the lawful contract before editing code.

When a ticket's proof or negative case depends on a profile, component, or carrier being absent, verify whether that data is actually optional on the live runtime subject under test. If the runtime bootstrap or factory path seeds it universally by default, correct the ticket and proof surface to the lawful distinction that still exists (for example self vs. non-self access, empty contents vs. missing carrier, or pre-perception vs. post-perception state) instead of writing tests around an impossible "component missing" state.

For planner-visible belief, profile, or snapshot-completeness tickets, verify the full carriage path before coding: runtime belief view -> snapshot builder -> snapshot storage -> `PlanningState`/planner-facing view surface. Do not stop at the final accessor if planner-visible data can be dropped earlier in the pipeline.
For planner behavior coverage tickets that add representative goal tests, also verify that the local test harness or belief fixture carries the full lawful planner inputs for each goal family under test before treating a failure as a production contradiction. Profiles, routes, violation records, evidence carriers, and similar planner-visible state often need fixture support even when the production planner path is already correct.

For dedicated goal-root, planner-root, or golden-isolation tickets, verify that the claimed downstream effect is uniquely attributable to the named goal/root rather than already reachable through a more generic operator family. If a generic path can already lawfully produce the same outcome, narrow the ticket and scenario so they prove the dedicated goal's distinct contract instead of over-claiming a broader downstream chain.

When a staged planner module or substrate already supports multiple goal families, verify each proposed live family against existing conformance/golden ownership before integrating them together. If live proof only clearly justifies part of that staged surface, default the ticket to the narrowest goal-family slice that is already supported rather than activating every plausible family at once.

For planner-root and tactical-barrier tickets, verify that each planner-produced subgoal is a lawful tactical destination rather than a transient probe, fallback waypoint, or exploration scaffold. Do not assume every emitted subgoal should become a scoped barrier target just because it passes through the planner; if the live search contract treats a subgoal as exploratory carriage rather than a durable destination, keep the ticket scoped to the lawful destination family and record the deviation explicitly.

When a planner ticket changes the shape of strategic output, verify how much of that output the downstream tactical/search layer actually consumes. If the live boundary only reads the first/current strategic step, do not author or implement a multi-step strategic fallback shape as though later steps are planner-visible; correct the ticket to the real consumed contract before coding.

Before making a generic planner fallback live as a tactical barrier, check whether grounded goals with explicit evidence carriers (`evidence_entities`, `evidence_places`, or equivalent exact-bound evidence) should keep their existing evidence-backed search path instead. Do not let a new generic probe barrier override lawful evidence-backed routing or exact-goal operator paths unless the ticket explicitly owns that broader change.

Load `references/reassessment-checks.md`.

### 3. Handle mismatches explicitly

Load `references/mismatch-handling.md`.

When reassessment shows that part of the ticket's claimed substrate is already present in live code, update the ticket before coding so it describes only the remaining owned delta. Reflect that narrowed scope in the ticket's `Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria` sections instead of leaving stale "add X" language in place.

When the ticket points at the wrong live section, function, symbol, or report/render location, correct that stale reference during reassessment before relying on the ticket's execution narrative. Do not preserve a misleading "change goes here" description once the live owned boundary is known.

After narrowing a ticket because substrate is already live, re-sweep the adjacent fallout that commonly remains owned by the current ticket: declaration/dispatch tables, snapshot/state carriers, local test stubs/helpers, synthetic candidate/root helpers, and the broadened verification selectors that should now prove only the remaining live delta.

If focused proof added during implementation reveals a production contradiction that reassessment did not yet expose, stop and correct the ticket before proceeding further. Update the same sections (`Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria`) so the ticket no longer claims "tests only" or `Engine Changes: None` when the live invariant actually requires production changes.

If focused proof instead falsifies the suspected production contradiction and shows the live fix is narrower (for example, golden-scenario isolation or fixture recalibration), stop and narrow the ticket before proceeding further. Update the same sections (`Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria`) so the ticket no longer claims production ownership when the honest contract is test-only or fixture-only.

When reassessment or focused proof changes the real edit surface, update `Files to Touch` and any file- or symbol-level scope notes immediately instead of leaving them stale until closeout. The ticket should keep reflecting the current owned boundary as implementation proceeds.

When `Acceptance Criteria` or the `Test Plan` names a focused test that is already owned by an adjacent active ticket, resolve that ownership during reassessment instead of leaving a split contract implicit. Either absorb the test into the current ticket and update sibling ownership, or remove it from the current ticket's must-pass list and cite the sibling ticket explicitly.

When reassessment changes the owned contract relative to a cited active spec, update that parent spec in the same pass unless the work is intentionally deferred behind a named follow-up ticket. Do not leave the ticket corrected while the live parent spec still describes the disproven contract.

When broadened verification later exposes fallout that crosses the original ticket seam or touches adjacent tickets in the same numbered family, re-run the sibling-ticket ownership check before silently broadening scope. If the new fallout is still part of the current ticket's lawful contract, update the ticket to reflect that expanded owned surface; if it belongs to an adjacent ticket, stop and use 1-3-1 rather than absorbing it implicitly.

### 4. Extract the implementation scope

Load `references/scope-extraction.md`.

### 5. Implement with Worldwake discipline

Load `references/implementation-discipline.md`.

When the clean fix requires extracting a helper out of an existing module into a neutral shared location, explicitly sweep sibling and transitive import sites for the old module path before relying on compile fallout alone. Shared-helper extraction often leaves behind stale `use crate::old_module::helper` assumptions even when the owned behavioral change is otherwise correct.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden.

Load `references/verification.md`.

When the ticket adds, renames, or materially re-scopes a `golden_*.rs` file or scenario block, run the repository's golden inventory/doc refresh as part of broadened verification and treat the generated docs as expected fallout to review and keep aligned with the landed scenario metadata.

If reassessment revealed that additive substrate from an earlier ticket already landed, include repository-wide live-contract fallout in the broadened verification sweep, not just the ticket's newly edited file set. Typical fallout includes stale `ALL` lists, exhaustiveness fixtures, representative-goal inventories, explicit length assertions that still reflect the pre-addition shape, and adjacent registry/declaration surfaces such as feasibility or invalidation strategies, provenance-family mappings, and other dispatch-table contracts that must now treat the additive shape as live behavior rather than inert scaffolding.

For additive planner-root tickets, also sweep helpers keyed by shared planner transitions or op-family semantics rather than only declaration tables and enum matches. Typical fallout includes planner-only synthetic candidate builders, search helpers that expand candidates from shared `PlannerTransitionKind` behavior, and exhaustive `PlannerOpKind` matches in non-obvious support modules such as observation/runtime reconciliation, blocker classification, or related-place/related-entity helpers.

For behavior-expanding tickets, expect broadened golden fallout to include stale scenario isolation, not just compile or enum-shape fallout. If an existing golden now reaches a newly lawful branch, tighten the scenario so it still proves its intended invariant using explicit local belief seeding, profile/perception overrides, or other lawful setup constraints rather than silently preserving the old behavior.

When a new fallback contract becomes lawful, re-check nearby planner/search tests and traces that previously asserted failure, suppression, or exhaustion. The honest post-change contract may now be `Found(ProgressBarrier)` or another bounded fallback plan rather than `not found`, and those expectation shifts should be treated as intentional verification fallout, not as automatic regressions.

When broadened verification fails, treat each failure as current-ticket fallout and continue the fix-and-rerun loop until the broadened target passes or you hit a real 1-3-1 blocker. Do not stop after the first full-suite failure if the next step is a straightforward fallout fix within the ticket's live scope.
After each fallout fix, rerun the same broadened verification target that exposed the failure before treating the branch as green. Do not rely on focused follow-up checks alone when the broader package or suite has not yet been rerun clean.

### 7. Close out the ticket honestly

Load `references/closeout.md`.

Before finalizing closeout, compare the landed diff and verification evidence against the ticket's final `What to Change`, `Files to Touch`, `Out of Scope`, and verification sections. If reassessment or implementation drift changed the actual owned surface, update the ticket so the recorded scope, touched files/symbols, deviations, stale exclusions, and proof set match the work that really landed.

### 8. Close the loop on the ticket

Covered in `references/closeout.md` (Step 8 section).

## Guardrails

- Name exact files, symbols, layers, and invariants for non-trivial claims.
- Treat tests, traces, event logs, and authoritative state as different proof surfaces.
- Architectural contradictions: solve or escalate with 1-3-1 (see mismatch-handling.md, Escalation decision tree). Do not patch around them.

## Example Usage

```
/implement-ticket tickets/LEGACTTOO-009*
/implement-ticket tickets/FITLSEC7RULGAP-001*
/implement-ticket .claude/worktrees/my-feature/tickets/FOO-003*
```
