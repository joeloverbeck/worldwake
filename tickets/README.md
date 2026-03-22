# Ticket Authoring Contract

This directory contains active implementation tickets.

To keep architecture clean, robust, and extensible, every new ticket must be created from `tickets/_TEMPLATE.md` and must satisfy the checks below.

All precision rules for technical claims — ordering, layer naming, coverage gaps, scenario isolation, and domain-specific requirements — are defined in `docs/precision-rules.md`. Apply those rules when writing any ticket section.

## Core Architectural Contract

1. No backwards-compatibility shims or alias paths in new work.
2. If current code and ticket assumptions diverge, update the ticket first before implementation.

## Required Ticket Sections

1. `Assumption Reassessment (YYYY-MM-DD)`:
   - Validate ticket assumptions against current code/tests.
   - Explicitly call out mismatches and corrected scope.
   - Cite exact files, symbols, or tests for any non-trivial architectural claim.
   - Apply all domain-specific precision requirements from `docs/precision-rules.md` (ordering, political claims, stale requests, ControlSource, heuristic removal, cumulative arithmetic, scenario isolation, coverage gaps, layer precision).
   - For planner- or golden-driven tickets, name the live `GoalKind` under test and the exact current operator, affordance, or prerequisite surface the scenario depends on. If reassessment shows the live goal family or operator surface differs from the original narrative, correct the ticket scope before implementation.
   - For ranking-sensitive tickets, validate the live ranking arithmetic before claiming branch symmetry, equal motive scores, or "priority-class only" divergence. Equal weights alone are not enough; check the full active substrate such as pressure, weights, promotions, and caps.
2. `Architecture Check`:
   - Explain why the proposed design is cleaner than alternatives.
3. `Verification Layers`:
   - Required for any mixed-layer or cross-system ticket.
   - Map each important invariant to the exact verification surface that proves it.
   - Use one line per invariant, for example:
     - candidate absence / reasoning behavior -> decision trace or focused runtime coverage
     - action lifecycle ordering -> action trace
     - authoritative mutation ordering -> event-log delta and/or authoritative world state
   - Do not collapse multiple layers into one generic "trace" or scenario-level assertion surface.
4. `Tests`:
   - List new/modified tests and rationale per test.
   - Include targeted and full-suite verification commands.
   - Commands must be copy-paste runnable against real test names or real targets, not approximate file-name filters.

## Mandatory Pre-Implementation Checks

1. Dependency references point to existing repository files (active or archived paths are both valid when explicit).
2. Type and data contracts match current code.
3. Files-to-touch list matches current file layout and ownership.
4. Scope does not duplicate already-delivered architecture.
5. Test commands have been dry-run checked or verified against the current test binary layout.
6. Claimed helper/function usage is verified against the exact current symbol location, not inferred from a similarly named helper elsewhere in the repo.
7. For AI-test tickets, use `cargo test -p worldwake-ai -- --list` or an equivalently narrow real command to confirm the current test names/targets before writing verification steps.
8. For stale-request, contested-affordance, or start-failure tickets, verify whether the first live rejection occurs in the shared runtime request layer before assigning scope to domain-specific handlers.
9. For ranking-sensitive golden or AI tickets, verify any claimed tie, neutrality, or branch symmetry against the current live arithmetic and cite the exact compared tests, scenarios, or symbols rather than inferring symmetry from equal utility weights alone.

## Archival Reminder

Follow `docs/archival-workflow.md` as the canonical process.
