# Precision Rules for Technical Claims

These rules govern how to make precise technical claims in tickets, specs, golden test rationale, and any other project documentation. They apply whenever you are writing about system behavior, coverage gaps, ordering contracts, or scenario design.

Referenced by: `tickets/README.md`, `docs/golden-e2e-testing.md`, `CLAUDE.md`.

## 1. Phase Distinction

Do not collapse distinct phases of behavior into one vague claim. Tickets and specs must distinguish:

- candidate generation
- ranking / suppression / filtering
- plan search / execution
- authoritative outcome

## 2. Layer Precision

Name the exact architectural layer and symbol for non-trivial claims. Do not collapse distinct layers into one vague statement. Distinguish:

- AI / belief-view / planning-layer logic
- authoritative system / action / world-validation logic

If similarly named helpers exist in multiple layers, name the exact layer and symbol being discussed.

## 3. Coverage Gap Classification

When claiming a testing gap, search for existing coverage first and name the exact tests found or state that none were found and how you checked. Distinguish:

- missing focused/unit coverage
- missing runtime trace/integration coverage
- missing golden/E2E coverage

If an AI regression is the target, also name the intended verification layer explicitly: candidate-generation focused/unit coverage, runtime `agent_tick` decision-trace/integration coverage, or golden E2E coverage.

If a runtime `agent_tick` regression depends on non-needs affordances or political/system actions, state the harness boundary explicitly: local needs-only harness is sufficient, or full action registries are required.

## 4. Ordering Contracts

If a claim depends on ordering, state which ordering is the contract:

- strict tick separation
- action lifecycle ordering
- event-log ordering
- authoritative world-state ordering

Ordering-sensitive claims must also state what drives the claimed divergence:

- priority class
- motive score
- suppression/filtering
- delayed system resolution
- a mixed-layer combination of the above

If delayed authoritative effects exist downstream of the behavior under test, do not use those later effects as a proxy for earlier ordering when a lower-layer assertion surface exists. Name both layers explicitly instead.

## 5. Verification Surface Mapping

For mixed-layer or cross-system claims, map each important invariant to the exact verification surface that proves it. Use one line per invariant:

- candidate absence / reasoning behavior -> decision trace or focused runtime coverage
- action lifecycle ordering -> action trace
- authoritative mutation ordering -> event-log delta and/or authoritative world state

Do not collapse multiple layers into one generic "trace" or scenario-level assertion surface. For single-layer claims, state why additional layer mapping is not applicable.

## 6. Decision-Trace Preference

For AI reasoning, candidate absence, suppression, or planner behavior, prefer decision-trace assertions over weaker indirect evidence such as missing event-log entries or missing committed actions.

Treat `Engine Changes: None` or "tests only" as a provisional hypothesis until reassessment confirms no production contradiction. Prefer the earliest causal boundary that proves the contract instead of broad downstream behavior when both are available.

## 7. Cumulative Arithmetic

If a claim depends on authoritative arithmetic or cumulative state, do not write it in purely narrative terms. State the concrete delta, cadence, threshold, capacity, or other live formula inputs that make the scenario reachable under current code.

For threshold/load/capacity-driven scenarios, validate survivability or non-survivability explicitly when repeated damage, depletion, recovery, or accumulation is part of the contract. If the current numbers make the intended branch impossible, correct the scenario numbers instead of weakening production semantics.

## 8. Scenario Isolation

When a golden scenario is intended to prove one specific causal branch while the current architecture lawfully permits competing affordances, document the scenario-isolation choice explicitly:

1. the intended branch or invariant under test
2. the lawful competing affordances the current architecture would otherwise allow
3. which unrelated lawful branches were intentionally removed from setup, and why they are outside the contract under test

## 9. Stale-Request and Start-Failure Boundaries

For stale-request, contested-affordance, or start-failure claims, name the first failure boundary explicitly:

- request resolution / affordance reproduction
- authoritative start
- post-start abort / commit-time revalidation

Verify the shared runtime request path before assigning scope to a domain action handler or AI failure-reconciliation helper. Name the exact shared symbols checked.

Map the boundary-specific proof surface explicitly:

- request resolution / affordance reproduction -> focused runtime request-resolution coverage
- authoritative start / abort lifecycle -> action trace and/or focused authoritative runtime coverage
- AI recovery / blocker reconciliation -> decision trace
- golden E2E -> only when the recovery chain itself is part of the contract

## 10. Political Office-Claim Precision

For political office-claim or support-law closure claims, do not compress closure into vague language like "someone else got there first." State whether the proof hinges on:

- support declaration
- visible-vacancy loss
- succession resolution
- office-holder mutation

Name the exact closure boundary being asserted. Cite the exact current symbols checked in both the AI/belief layer and the authoritative law/action layer.

## 11. ControlSource and Runtime Intent

For claims that manipulate `ControlSource`, queued inputs, driver resets, or other harness/runtime conditions, do not assume those changes automatically clear intent. State whether the current architecture can lawfully retain or continue an already-selected plan shape, and identify the exact runtime/trace symbols checked for that claim.

## 12. Heuristic Removal Discipline

If a claim proposes removing, weakening, bypassing, or replacing a heuristic/filter, state:

- which missing architectural substrate that heuristic is currently standing in for
- whether this ticket introduces that substrate
- why the change does not reopen regressions in unrelated scenarios

## 13. Divergence Protocol

If current code and ticket/spec assumptions diverge, update the document first before implementation and update scope to match the actual architecture.

Do not leave a ticket marked `Engine Changes: None` or "tests only" when the requested invariant actually exposes an architectural contradiction in production code. Correct the scope first.

## 14. Timing vs Semantics

If a proposed test relies on a timing assumption, prefer the semantic invariant instead of an incidental tick-boundary assumption unless the tick boundary is itself the contract.

If the scenario can lawfully produce a same-tick cross-agent chain, do not write the claim as "actor B must act on a later tick" unless strict tick separation is the actual engine rule. In those cases, prefer the explicit action-trace ordering key `(tick, sequence_in_tick)` over incidental tick numbers.

If the ordering claim depends on ordering, state whether the compared branches are symmetric in the current architecture or whether they depend on different ranking or resolution substrates.
