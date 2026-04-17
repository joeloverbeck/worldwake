# Mismatch Handling

How to handle disagreements between ticket assumptions and live code (Step 3).

## Core rule

If the ticket and live code disagree, stop and surface the discrepancy before implementation.

For each mismatch, state:
- what the ticket says
- what the codebase currently has
- whether the ticket should be corrected, the implementation should adapt, or the issue is blocked

## Low-risk auto-corrections

Update the ticket immediately (without stopping) when the correction is mechanical and directionally unambiguous:
- Exact live spec path from a user-supplied glob
- Stale file/symbol/test references
- Stale inline code snippets, API sketches, or sample struct literals whose live symbols or field names have drifted
- `Files to Touch`, `Verification Layers`, or command lists that need to match current codebase
- Component-registration fallout from live macro expansion or schema discovery
- Stale acceptance criteria, scenario assertion surfaces, or proof targets where the live symbols and behavior make the narrower honest contract directionally unambiguous

Record each auto-correction: ticket says / live code has / correction applied / why safe.
Place notes under the ticket's `Assumption Reassessment` section as numbered entries. If the section is missing, add one.

During reassessment, validate inline examples against the live codebase before coding:
- macro names or forwarding paths shown in the ticket/spec
- sample method signatures and return shapes
- sample struct literals and field names

If the embedded example is stale but the intended direction is still unambiguous, auto-correct the ticket text in the same pass instead of carrying the stale snippet into implementation.

### Affected section updates

When any correction changes the real fallout surface, update **all** affected ticket sections in the same pass: `What to Change`, `Files to Touch`, `Verification Layers`, `Test Plan`, and when scope narrows, also `Acceptance Criteria`, `Tests That Must Pass`, and command expectations.

When reassessment converts a ticket into reassessment-only, doc-only, or no-production-change completion, remove leftover placeholder scaffolding from acceptance criteria, verification, test-plan, and command sections.

If all owned proof surfaces already pass and the live outcome is only factual validation plus adjacent-fallout triage, convert the ticket immediately to a validation-only close-out. Remove stale implementation scaffolding, keep the proof surface honest, and create a follow-up ticket for any remaining work.

## Escalation decision tree

| Situation | Action |
|-----------|--------|
| Low-risk factual mismatch (stale reference, path, command) | Auto-correct; record note |
| One correction reveals a second contradiction in the same surface | Rerun boundary check before coding; do not treat first correction as final |
| Later reassessment shows a subdomain can no longer land and no ticket owns it | Create or update follow-up ticket chain immediately |
| Architectural, ambiguous, or changes owned boundary | Stop; use 1-3-1 (1 problem, 3 options, 1 recommendation) |
| Adjacent blocker exposed by verification -- small, local, needed for verification | Absorb; note why in ticket |
| Adjacent blocker -- broad or would expand ticket materially | Stop; use 1-3-1 |
| Deeper shared-layer contradiction outside ticket scope | Do not pull into ticket; use 1-3-1 |

When using 1-3-1, evaluate each option against the relevant FOUNDATIONS principles. Name the principle numbers and state whether each option aligns or violates. A FOUNDATIONS violation disqualifies an option regardless of implementation simplicity.

Do not silently skip deliverables. Do not weaken the ticket without user confirmation.

When the user confirms a direction that changes architecture boundary, affected files, or proof surface, apply the affected section updates rule above before coding.

## Narrowing when substrate is already live

When reassessment shows that part of the ticket's claimed substrate is already present in live code, update the ticket before coding so it describes only the remaining owned delta. Reflect that narrowed scope in the ticket's `Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria` sections instead of leaving stale "add X" language in place.

When the ticket points at the wrong live section, function, symbol, or report/render location, correct that stale reference during reassessment before relying on the ticket's execution narrative. Do not preserve a misleading "change goes here" description once the live owned boundary is known.

After narrowing a ticket because substrate is already live, re-sweep the adjacent fallout that commonly remains owned by the current ticket: declaration/dispatch tables, snapshot/state carriers, local test stubs/helpers, synthetic candidate/root helpers, and the broadened verification selectors that should now prove only the remaining live delta.

## Focused-proof contradictions

If focused proof added during implementation reveals a production contradiction that reassessment did not yet expose, stop and correct the ticket before proceeding further. Update the same sections (`Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria`) so the ticket no longer claims "tests only" or `Engine Changes: None` when the live invariant actually requires production changes.

If focused proof instead falsifies the suspected production contradiction and shows the live fix is narrower (for example, golden-scenario isolation or fixture recalibration), stop and narrow the ticket before proceeding further. Update the same sections (`Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, and `Acceptance Criteria`) so the ticket no longer claims production ownership when the honest contract is test-only or fixture-only.
If a focused rerun disproves a newly adopted root-cause hypothesis, update the ticket again immediately: remove the falsified explanation from `Problem`, `Architecture Check`, `What to Change`, and any command/acceptance text that depended on it, and revert any exploratory production change that no longer explains the motivating failure.
When a stronger comparative golden claim is falsified but a narrower strengthened-state or retained-carrier effect is still real, keep the ticket and scenario at that narrower comparative contract instead of forcing a false total-failure control. Update the ticket's reassessment and assertions to prove the live comparative effect that actually survives the rerun.

## Edit-surface drift during implementation

When reassessment or focused proof changes the real edit surface, update `Files to Touch` and any file- or symbol-level scope notes immediately instead of leaving them stale until closeout. The ticket should keep reflecting the current owned boundary as implementation proceeds.
If the required verification command stays red after an initial fix but the blocking file, symbol, or lint root cause changes, update the ticket's reassessment and `Problem` text to reflect that new live blocker instead of only widening `Files to Touch`.
For component-registration and other additive schema tickets, explicitly sweep authoritative inventories and registration proofs before calling the scope final: hardcoded `ALL` arrays, sample value builders, test-module imports that mention the new type only in fixtures/assertions, exact create-agent/create-entity delta assertions, and similar exhaustive registration fixtures often remain owned fallout even when the production code change is small. For exact bootstrap delta tests, verify whether the live ordering rule comes from component-schema/macro projection instead of the local insertion-call sequence before updating expected arrays by eye.
When the owned invariant is "component/state mirrors authoritative post-mutation state," keep `Files to Touch`, `What to Change`, and `Acceptance Criteria` aligned with every real truth-transition path you uncover during reassessment. Do not leave the ticket claiming a single-hook implementation once the live boundary clearly spans multiple mutation families.

## Sibling-owned tests and ownership splits

When `Acceptance Criteria` or the `Test Plan` names a focused test that is already owned by an adjacent active ticket, resolve that ownership during reassessment instead of leaving a split contract implicit. Either absorb the test into the current ticket and update sibling ownership, or remove it from the current ticket's must-pass list and cite the sibling ticket explicitly.
If the ticket says there are no focused tests in scope but the owned file already contains focused tests that exercise the changed contract, treat those existing tests as the narrow proof surface. Update the ticket's `Verification Layers`, `Test Plan`, and command list to reflect the live proof surface while keeping sibling-ticket ownership explicit about any additional focused coverage that still belongs elsewhere.

## Spec/parent alignment and deferred remainders

When reassessment changes the owned contract relative to a cited active spec, update that parent spec in the same pass unless the work is intentionally deferred behind a named follow-up ticket. Do not leave the ticket corrected while the live parent spec still describes the disproven contract.
When a scenario-only, golden-setup, or other authored-substrate ticket truthfully proves its owned setup but still exposes remaining engine behavior, narrow the current ticket to the setup contract it actually proves, record the surviving engine remainder as a mismatch, and create or update an explicit follow-up ticket for that engine work instead of continuing to distort the setup to hide it.

When broadened verification later exposes fallout that crosses the original ticket seam or touches adjacent tickets in the same numbered family, re-run the sibling-ticket ownership check before silently broadening scope. If the new fallout is still part of the current ticket's lawful contract, update the ticket to reflect that expanded owned surface; if it belongs to an adjacent ticket, stop and use 1-3-1 rather than absorbing it implicitly.
