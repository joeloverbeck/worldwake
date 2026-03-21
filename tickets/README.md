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
   - If the ticket is about political office claims or support-law closure, name the exact office-claim closure boundary being asserted: support declaration, visible-vacancy loss, succession resolution, or office-holder mutation.
   - For political closure claims, cite the exact current symbols checked in both the AI/belief layer and the authoritative law/action layer.
   - If the ticket depends on ordering, state whether the compared branches are symmetric in the current architecture or whether they depend on different ranking or resolution substrates.
   - If the ticket proposes removing, weakening, bypassing, or replacing a heuristic/filter, state which missing architectural substrate that heuristic is currently standing in for, whether this ticket introduces that substrate, and why the change does not reopen regressions in unrelated scenarios.
   - If the ticket involves stale requests, contested affordances, or start-failure recovery, name the first failure boundary explicitly: request resolution / affordance reproduction, authoritative start, or post-start abort / commit-time revalidation.
   - If the ticket manipulates `ControlSource`, queued inputs, driver resets, or other harness/runtime conditions, state whether retained runtime intent can lawfully continue and cite the exact current runtime/trace symbols checked.
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
   - strict tick separation
   - action lifecycle ordering
   - event-log ordering
   - authoritative world-state ordering
5. Ordering-sensitive tickets must also state what drives the claimed divergence:
   - priority class
   - motive score
   - suppression/filtering
   - delayed system resolution
   - a mixed-layer combination of the above
6. If delayed authoritative effects exist downstream of the behavior under test, do not use those later effects as a proxy for earlier ordering when a lower-layer assertion surface exists. Name both layers explicitly instead.
7. If current code and ticket assumptions diverge, update the ticket before implementation and update scope to match the actual architecture.
8. If a proposed test relies on a timing assumption, prefer the semantic invariant instead of an incidental tick-boundary assumption unless the tick boundary is itself the contract.
9. If the scenario can lawfully produce a same-tick cross-agent chain, do not write the ticket as “actor B must act on a later tick” unless strict tick separation is the actual engine rule. In those cases, prefer the explicit action-trace ordering key `(tick, sequence_in_tick)` over incidental tick numbers.
10. If the invariant is about AI reasoning, candidate absence, suppression, or planner behavior, prefer decision-trace assertions over weaker indirect evidence such as missing event-log entries.
11. Treat `Engine Changes: None` or "tests only" as a provisional hypothesis until reassessment confirms no production contradiction, and prefer the earliest causal boundary that proves the contract instead of broad downstream behavior when both are available.
12. For mixed-layer scenarios, list the invariant-to-layer mapping explicitly instead of implying that one assertion surface proves the whole chain.
13. If a golden scenario is intended to prove one specific causal branch while the current architecture lawfully permits competing affordances, document the scenario-isolation choice explicitly and explain which unrelated lawful branches were intentionally removed from setup.
14. For stale-request, contested-affordance, or start-failure tickets, verify the shared runtime request path before assigning scope to a domain action handler or AI failure-reconciliation helper. Name the exact shared symbols you checked.
15. For tickets in that class, map the boundary-specific proof surface explicitly:
   - request resolution / affordance reproduction -> focused runtime request-resolution coverage
   - authoritative start / abort lifecycle -> action trace and/or focused authoritative runtime coverage
   - AI recovery / blocker reconciliation -> decision trace
   - golden E2E -> only when the recovery chain itself is part of the contract
16. For political office-claim tickets, do not compress closure into vague language like "someone else got there first." State whether the proof hinges on support declaration, visible-vacancy loss, succession resolution, or office-holder mutation, and name the exact current symbols that establish that boundary.
17. For tickets that manipulate control handoff or harness runtime state, do not assume those changes automatically clear intent. State whether the current architecture can lawfully retain or continue an already-selected plan shape, and identify the exact runtime/trace symbols checked for that claim.
18. If a ticket/spec scenario depends on authoritative arithmetic or cumulative state, do not write it in purely narrative terms. State the concrete delta, cadence, threshold, capacity, or other live formula inputs that make the scenario reachable under current code.
19. For threshold/load/capacity-driven scenarios, validate survivability or non-survivability explicitly when repeated damage, depletion, recovery, or accumulation is part of the contract. If the current numbers make the intended branch impossible, correct the ticket before implementation instead of weakening production semantics.

## Mandatory Pre-Implementation Checks

1. Dependency references point to existing repository files (active or archived paths are both valid when explicit).
2. Type and data contracts match current code.
3. Files-to-touch list matches current file layout and ownership.
4. Scope does not duplicate already-delivered architecture.
5. Test commands have been dry-run checked or verified against the current test binary layout.
6. Claimed helper/function usage is verified against the exact current symbol location, not inferred from a similarly named helper elsewhere in the repo.
7. For AI-test tickets, use `cargo test -p worldwake-ai -- --list` or an equivalently narrow real command to confirm the current test names/targets before writing verification steps.
8. For stale-request, contested-affordance, or start-failure tickets, verify whether the first live rejection occurs in the shared runtime request layer before assigning scope to domain-specific handlers.

## Archival Reminder

Follow `docs/archival-workflow.md` as the canonical process.
