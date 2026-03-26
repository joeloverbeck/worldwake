# E17CRITHEJUS-019: Tighten cross-layer ticketing rules for information-path refactors

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation/process contract only
**Deps**: E17CRITHEJUS-017, E17CRITHEJUS-018

## Problem

`E17CRITHEJUS-017` and `E17CRITHEJUS-018` cleaned up the institutional Tell boundary and strengthened its traceability, but they also exposed a remaining documentation/process gap: a mixed-layer refactor can still present a narratively correct ticket while failing to state whether one fact currently travels through multiple lawful-looking paths, which path becomes canonical, and whether the trace surface is strong enough to debug the new boundary.

In the concrete E17 case, institutional claims existed both as first-class `TellTopic::InstitutionalClaim { .. }` artifacts and as legacy sidecars hanging off `TellTopic::EntityBelief { subject: office_or_record }`. The existing ticket rules required reassessment, layer naming, and traceability escalation, but they did not explicitly force the document to state:

1. whether one fact currently has multiple transport paths
2. which path is canonical after the change
3. whether the trace surface itself must be upgraded because the new contract would otherwise be hard to debug

This is a documentation/process gap, not a runtime-architecture gap and not a foundations gap. `docs/FOUNDATIONS.md` already requires concrete information carriers and explainable information flow; the ticketing contract needs to operationalize that requirement more explicitly.

## Assumption Reassessment (2026-03-26)

1. `E17CRITHEJUS-017` is completed and archived at [archive/tickets/E17CRITHEJUS-017.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-017.md). That ticket removed the entity-sidecar institutional Tell path and established `TellTopic::InstitutionalClaim { claim }` as the canonical architecture. This ticket must not imply the runtime architecture is still duplicated.
2. `E17CRITHEJUS-018` is completed and archived at [archive/tickets/E17CRITHEJUS-018.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-018.md). It already strengthened ranking and Tell traceability, so this ticket must not claim the live code still lacks the E17 trace surfaces that were added there.
3. Current code confirms the cleaned boundary:
   - `crates/worldwake-systems/src/tell_actions.rs` contains focused coverage such as `tell_commit_entity_belief_no_longer_relays_institutional_claim_sidecars` and `tell_affordances_include_local_institutional_claim_topics_even_when_office_is_visible`.
   - `crates/worldwake-ai/src/candidate_generation.rs` contains focused coverage such as `social_candidates_emit_institutional_claim_topics_even_when_office_entity_is_visible`.
4. `tickets/README.md` already requires assumption reassessment, exact shared-boundary naming, live goal/operator identification, ranking-sensitive checks, and follow-up traceability tickets when traces are insufficient.
5. `docs/precision-rules.md` already distinguishes layers, ordering contracts, coverage classes, heuristic removal, divergence protocol, and traceability escalation. The missing specificity is not “tickets ignore traces”; it is “tickets do not yet force explicit one-fact/one-transport-path statements for information-bearing artifacts and explicit trace-surface reassessment for the cleaned path.”
6. The exact document boundary under audit is ticket authoring for mixed-layer social-information refactors, especially changes that touch `TellTopic`, `SharedTellState`, tell-memory lanes, social observations, records, institutional claims, or other information-carrying artifacts.
7. The intended invariant, consistent with `docs/FOUNDATIONS.md`, is that beliefs and social artifacts travel through explicit concrete carriers with one canonical architectural path after a refactor. Duplicate transport paths may appear during implementation, but the ticket must call them out explicitly and state whether this ticket removes them or leaves a named follow-up cleanup ticket.
8. The motivating live goal family from `E17CRITHEJUS-017` was `GoalKind::ShareBelief { .. }`, but this ticket is documentation-only. No planner or runtime code changes are proposed here.
9. Existing repository guidance in the root [AGENTS.md](/home/joeloverbeck/projects/worldwake/AGENTS.md) already says mixed-layer tickets must name the exact shared abstraction boundary and that traces may require a follow-up traceability ticket. The remaining work is to harmonize that guidance with `tickets/README.md` and `docs/precision-rules.md` so the canonical-path requirement is explicit instead of implied.
10. No ordering contract change is proposed. This ticket clarifies authoring obligations for future tickets whose claims depend on ranking order, action lifecycle order, or authoritative mutation order.
11. No heuristic removal is proposed. The ticket strengthens documentation so future tickets must say when a heuristic/filter is papering over a missing substrate and whether the ticket actually installs that substrate.
12. This is not a stale-request or start-failure ticket. The gap is document precision for mixed-layer information-path refactors after the runtime cleanup already landed.
13. Foundations alignment is direct:
    - Principle 7: communication must have an explicit path
    - Principle 13: information path must be explainable
    - Principle 16: memories and records are world state
    - Principle 23: social artifacts are first-class
    - Principle 24: systems interact through state, not hidden coupling
14. Adjacent contradictions exposed during reassessment:
    - required consequence of this ticket: ticket docs should explicitly require canonical-path declarations and trace-surface reassessment for information-path changes
    - separate future cleanup: broader process changes outside ticket/spec drafting do not belong here
15. Mismatch + correction:
    - this is not a request to change `docs/FOUNDATIONS.md`; foundations already says the right thing
    - this is not a request for more E17 runtime refactoring; the needed change is in the ticketing/precision contract that operationalizes those principles during future implementation planning
    - the ticket should explicitly name the live focused tests above so the process gap is grounded in current code rather than a stale narrative

## Architecture Check

1. Tightening the ticketing contract is cleaner than relying on contributor memory or post-hoc review comments. Mixed-layer social-information refactors should fail fast in the ticket if they leave duplicate transport paths or an underpowered explanation surface.
2. This is better than broad “be precise” process text. The new rules should target the exact architecture hazard exposed by `E17CRITHEJUS-017`: one fact flowing through more than one transport path after the refactor narrative claims a cleanup.
3. This is also better than reopening runtime code that already landed cleanly. The architecture benefit here comes from protecting the cleaned single-path design from future ticket drift, not from adding another compatibility layer or another alias path.
4. No backwards-compatibility aliasing or additional process branches are introduced. The ticket only sharpens the existing single-source authoring contract.

## Verification Layers

1. Ticket contract explicitly requires “one fact, one canonical transport path” declarations for information-bearing refactors -> `tickets/README.md`
2. Precision rules explicitly require canonical-path and trace-surface reassessment when information-path abstractions change -> `docs/precision-rules.md`
3. Root repository guidance remains aligned with the ticket contract for mixed-layer canonical-path and traceability escalation -> `AGENTS.md`
4. Existing runtime tests continue to prove the live E17 boundary remains clean while this ticket only changes process docs -> focused `tell_actions.rs` and `candidate_generation.rs` tests
5. Single-layer documentation ticket; no new runtime verification mapping is applicable beyond guarding the assumptions above

## What to Change

### 1. Update the ticket authoring contract

- Amend `tickets/README.md` so mixed-layer tickets involving beliefs, tells, records, institutional claims, rumors, or other information carriers must explicitly state:
  - whether multiple transport paths currently exist for the same fact
  - which path is canonical after the change
  - whether duplicate paths are removed in-scope or deferred to a named follow-up ticket

### 2. Extend precision rules for information-path refactors

- Amend `docs/precision-rules.md` with a focused rule for information-bearing abstraction changes:
  - require “one fact, one transport path” analysis
  - require explicit classification of temporary mixed-state coexistence versus intended end-state architecture
  - require trace-surface reassessment when the new canonical path would otherwise be harder to debug than the old one

### 3. Harmonize root agent guidance

- Update the repository `AGENTS.md` ticket expectations or debugging sections only as needed so they reference the same canonical-path and traceability-escalation obligations as `tickets/README.md` and `docs/precision-rules.md`.
- Keep the changes narrow; do not duplicate the full precision-rules document into multiple locations.

## Files to Touch

- `tickets/README.md` (modify)
- `docs/precision-rules.md` (modify)
- `AGENTS.md` (modify, only if needed for alignment)

## Out of Scope

- Any production code change in `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai`
- Reopening the architectural decisions from `E17CRITHEJUS-017`
- General documentation cleanup unrelated to mixed-layer information-path refactors
- Changes to `docs/FOUNDATIONS.md`

## Acceptance Criteria

### Tests That Must Pass

1. `tickets/README.md` explicitly requires canonical-path declarations for mixed-layer information-path refactors
2. `docs/precision-rules.md` explicitly requires trace-surface reassessment when new information paths become architecturally canonical
3. Repository guidance remains consistent across the updated docs with no conflicting instructions
4. Existing focused E17 Tell/information-path tests still pass so the ticket’s assumptions remain grounded in current code

### Invariants

1. Future tickets for social-information or record/institution refactors must fail early if they leave two transport paths for the same fact without saying so explicitly
2. The documentation changes must reinforce Principles 7, 13, 16, 23, and 24 without redefining or weakening `docs/FOUNDATIONS.md`

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification relies on command-based checks plus existing focused runtime tests named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_entity_belief_no_longer_relays_institutional_claim_sidecars -- --exact`
2. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_include_local_institutional_claim_topics_even_when_office_is_visible -- --exact`
3. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_emit_institutional_claim_topics_even_when_office_entity_is_visible -- --exact`
4. `rg -n "canonical|transport path|trace-surface|traceability|mixed-layer|information-path" tickets/README.md docs/precision-rules.md AGENTS.md`
5. `git diff -- tickets/README.md docs/precision-rules.md AGENTS.md tickets/E17CRITHEJUS-019.md`

## Outcome

Completed: 2026-03-26

- What actually changed:
  - Reassessed the ticket against the live post-`E17CRITHEJUS-017` / post-`E17CRITHEJUS-018` code and corrected its assumptions so it no longer implies the runtime architecture is still duplicated.
  - Updated `tickets/README.md` to require explicit canonical-path declarations for information-path refactors and to require pre-implementation confirmation that the planned proof surface remains strong enough for the canonical path.
  - Added `## 16. Information-Path Refactors` to `docs/precision-rules.md` so tickets/specs must name duplicate lawful paths, canonical end-state path, mixed-state cleanup scope, and any needed traceability follow-up.
  - Added one alignment bullet to `AGENTS.md` so repository guidance matches the ticketing and precision rules without duplicating those documents.
- Deviations from original plan:
  - No production/runtime code changes were needed. Reassessment showed the architecture cleanup and traceability work already landed in `E17CRITHEJUS-017` and `E17CRITHEJUS-018`; this ticket only needed to codify the process lessons.
  - No new or modified Rust tests were added because the ticket is documentation-only. Existing focused E17 tests were used to ground and verify the corrected assumptions.
- Verification results:
  - `cargo test -p worldwake-systems tell_actions::tests::tell_commit_entity_belief_no_longer_relays_institutional_claim_sidecars -- --exact`
  - `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_include_local_institutional_claim_topics_even_when_office_is_visible -- --exact`
  - `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_emit_institutional_claim_topics_even_when_office_entity_is_visible -- --exact`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `rg -n "canonical|transport path|trace-surface|traceability|mixed-layer|information-path" tickets/README.md docs/precision-rules.md AGENTS.md tickets/E17CRITHEJUS-019.md`
