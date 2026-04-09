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
- `Files to Touch`, `Verification Layers`, or command lists that need to match current codebase
- Component-registration fallout from live macro expansion or schema discovery
- Stale acceptance criteria, scenario assertion surfaces, or proof targets where the live symbols and behavior make the narrower honest contract directionally unambiguous

Record each auto-correction: ticket says / live code has / correction applied / why safe.
Place notes under the ticket's `Assumption Reassessment` section as numbered entries. If the section is missing, add one.

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
