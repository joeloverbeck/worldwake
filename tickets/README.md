# Ticket Authoring Contract

This directory contains active implementation tickets.

To keep architecture clean, robust, and extensible, every new ticket must be created from `tickets/_TEMPLATE.md` and must satisfy the checks below.

## Core Architectural Contract

1. No backwards-compatibility shims or alias paths in new work.
2. If current code and ticket assumptions diverge, update the ticket first before implementation.

## Required Ticket Sections

1. `Assumption Reassessment (YYYY-MM-DD)`:
   - Validate ticket assumptions against current code/tests.
   - Explicitly call out mismatches and corrected scope.
   - Cite exact files, symbols, or tests for any non-trivial architectural claim.
   - When claiming a coverage gap, search for existing focused/unit, runtime trace/integration, and golden/E2E coverage first; name the exact tests found or state that none were found and how you checked.
   - Distinguish missing focused/unit coverage from missing golden/E2E coverage when the ticket claims a testing gap.
   - If similarly named helpers exist in multiple layers, name the exact layer and symbol being discussed.
2. `Architecture Check`:
   - Explain why the proposed design is cleaner than alternatives.
3. `Tests`:
   - List new/modified tests and rationale per test.
   - Include targeted and full-suite verification commands.
   - Commands must be copy-paste runnable against real test names or real targets, not approximate file-name filters.

## Required Precision For Assumptions And Tests

1. Do not collapse distinct phases of behavior into one vague claim. Tickets must distinguish:
   - candidate generation
   - ranking / suppression / filtering
   - plan search / execution
   - authoritative outcome
2. If an AI regression is the target, also name the intended verification layer explicitly:
   - candidate-generation focused/unit coverage
   - runtime `agent_tick` decision-trace / integration coverage
   - golden E2E coverage
3. Do not collapse distinct architectural layers into one vague claim. Tickets must distinguish:
   - AI / belief-view / planning-layer logic
   - authoritative system / action / world-validation logic
3. If a runtime `agent_tick` regression depends on non-needs affordances or political/system actions, state the harness boundary explicitly:
   - local needs-only harness is sufficient
   - full action registries are required
4. If a ticket depends on ordering, state which ordering is the contract:
   - action lifecycle ordering
   - event-log ordering
   - authoritative world-state ordering
5. If current code and ticket assumptions diverge, update the ticket before implementation and update scope to match the actual architecture.
6. If a proposed test relies on a timing assumption, prefer the semantic invariant instead of an incidental tick-boundary assumption unless the tick boundary is itself the contract.
7. If the invariant is about AI reasoning, candidate absence, suppression, or planner behavior, prefer decision-trace assertions over weaker indirect evidence such as missing event-log entries.

## Mandatory Pre-Implementation Checks

1. Dependency references point to existing repository files (active or archived paths are both valid when explicit).
2. Type and data contracts match current code.
3. Files-to-touch list matches current file layout and ownership.
4. Scope does not duplicate already-delivered architecture.
5. Test commands have been dry-run checked or verified against the current test binary layout.
6. Claimed helper/function usage is verified against the exact current symbol location, not inferred from a similarly named helper elsewhere in the repo.
7. For AI-test tickets, use `cargo test -p worldwake-ai -- --list` or an equivalently narrow real command to confirm the current test names/targets before writing verification steps.

## Archival Reminder

Follow `docs/archival-workflow.md` as the canonical process.
