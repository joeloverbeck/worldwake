# DOCGOLARCH-001: Add Heuristic-Substrate and Scenario-Isolation Rules to Ticket and Golden Docs

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/README.md`, `tickets/_TEMPLATE.md`, `docs/golden-e2e-testing.md`, `archive/tickets/completed/S13POLEMEGOLSUI-002-social-tell-political-knowledge.md`, existing `AGENTS.md` ticket discipline

## Problem

S13POLEMEGOLSUI-002 exposed two recurring authoring failures that the current docs do not state explicitly enough:

1. a heuristic can look like an arbitrary blocker when it is actually standing in for missing architecture
2. a cross-system golden can look correct on paper while still allowing lawful competing affordances that obscure the intended invariant

Current ticket and golden-doc guidance already covers trace surfaces and precision, but it does not yet explicitly require:

1. identifying what missing substrate a heuristic is currently substituting for before removing it
2. documenting scenario isolation choices when a golden is intended to prove one specific causal chain

Without those rules, future tickets can mis-scope architecture work and future goldens can become noisy or misleading while still "passing."

## Assumption Reassessment (2026-03-18)

1. `tickets/README.md` already requires assumption reassessment, exact symbol naming, coverage-gap precision, and verification-layer mapping. It does **not** explicitly require authors to identify which missing architectural substrate an existing heuristic is compensating for before proposing removal or weakening.
2. `tickets/_TEMPLATE.md` currently gives no prompt for heuristic-substrate analysis or scenario-isolation notes. That means the main authoring entry point is weaker than the policy contract in `tickets/README.md`, which is a documentation-architecture gap.
3. `docs/golden-e2e-testing.md` already defines assertion hierarchy and trace guidance. It does **not** explicitly tell authors to document scenario-isolation choices when lawful competing affordances exist and the golden is intended to prove one branch.
4. The motivating evidence is present in current and archived coverage, not missing runtime infrastructure: `archive/tickets/completed/S13POLEMEGOLSUI-002-social-tell-political-knowledge.md` documents that weakening `crates/worldwake-ai/src/candidate_generation.rs::emit_social_candidates` same-place Tell suppression caused repeat-gossip regressions; current focused coverage still includes `candidate_generation::tests::social_candidates_skip_subjects_already_known_to_be_colocated`, and current golden coverage still includes `golden_tell_propagates_political_knowledge`, `golden_information_locality_for_political_facts`, and `golden_agent_autonomously_tells_colocated_peer`.
5. Mismatch discovered during reassessment: this ticket's current `Test Plan` incorrectly lists documentation files as "New/Modified Tests" and omits `tickets/_TEMPLATE.md` from scope even though that file is the first artifact authors copy. The clean correction is to keep this ticket doc-only, add the template update, and state explicitly that no runtime tests are added because no runtime invariant changes.

## Architecture Check

1. The clean solution is to strengthen the ticket contract, the ticket template, and the golden authoring rules together rather than relying on tribal knowledge or one archived postmortem ticket.
2. This is architecturally important because heuristic removal without substrate identification tends to strip out architecture-carrying behavior before the real substrate exists, which creates hidden regressions instead of extensible design.
3. Scenario-isolation guidance belongs in the golden-testing conventions doc because it is a reusable test-design rule across domains, while the template should prompt for it so authors do not forget the requirement at draft time.
4. No compatibility layer is needed. The docs should directly replace the weaker guidance with stronger requirements.

## Verification Layers

1. Ticket contract explicitly requires heuristic-substrate analysis -> doc content check in `tickets/README.md`
2. Ticket template prompts authors for heuristic-substrate and scenario-isolation notes -> doc content check in `tickets/_TEMPLATE.md`
3. Golden authoring rules explicitly mention scenario isolation and lawful competing affordances -> doc content check in `docs/golden-e2e-testing.md`
4. Updated guidance remains consistent with existing `AGENTS.md` expectations and the archived S13 motivating case -> manual cross-doc review

## What to Change

### 1. Strengthen `tickets/README.md`

Add an explicit rule that when a ticket proposes removing, weakening, or bypassing an AI heuristic or filter, it must state:

1. what concrete architectural substrate that heuristic is currently standing in for
2. whether the ticket is replacing that substrate or merely removing the heuristic
3. why removal does not open regressions in unrelated scenarios

This makes heuristic-removal work prove architectural replacement rather than just behavioral preference.

### 2. Strengthen `tickets/_TEMPLATE.md`

Add prompts in the template so new tickets must record:

1. when a heuristic is standing in for missing substrate
2. whether the work replaces that substrate or only removes a heuristic
3. when a golden scenario intentionally isolates one lawful branch from competing affordances

This keeps the strongest rules at the point where authors actually draft tickets instead of only in the reference contract.

### 3. Strengthen `docs/golden-e2e-testing.md`

Add explicit guidance that cross-system golden scenarios must document scenario-isolation choices when:

1. the intended invariant is one specific causal chain
2. the current architecture lawfully permits competing branches
3. local perception, ranking, or planner branching could obscure the intended proof

The rule should tell authors to remove unrelated lawful affordances when they are not part of the contract under test, and to say so explicitly in the ticket/spec.

### 4. Align terminology across the touched docs

Use one consistent vocabulary for:

1. heuristic
2. missing substrate
3. scenario isolation
4. lawful competing affordance
5. intended branch/invariant

That keeps future tickets from drifting into vague phrases like "the AI is blocked by this check" when the real issue is architectural incompleteness.

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)
- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- implementing E15c or any runtime social-memory code
- changing current golden scenarios
- adding new AI traces or debug fields
- rewriting AGENTS.md unless a contradiction is discovered

## Acceptance Criteria

### Tests That Must Pass

1. `rg -n "heuristic|substrate|scenario isolation|lawful competing affordance|intended branch" tickets/README.md tickets/_TEMPLATE.md docs/golden-e2e-testing.md`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Ticket docs must require heuristic-removal proposals to identify the missing replacement substrate.
2. Golden docs must require scenario-isolation notes when lawful competing affordances exist.
3. The guidance must remain aligned with `docs/FOUNDATIONS.md` emphasis on explicit local causality and state-mediated behavior.

## Test Plan

### New/Modified Tests

1. None. This is a documentation/process ticket; no runtime behavior or runtime invariant changes, and existing focused plus golden coverage already demonstrates the motivating failure mode.

### Commands

1. `rg -n "heuristic|substrate|scenario isolation|lawful competing affordance|intended branch" tickets/README.md tickets/_TEMPLATE.md docs/golden-e2e-testing.md`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-18
- What actually changed:
  - Updated `tickets/README.md` to require heuristic-substrate analysis for heuristic/filter changes and explicit scenario-isolation notes for golden scenarios with lawful competing affordances.
  - Updated `tickets/_TEMPLATE.md` so new tickets prompt for heuristic-substrate analysis and scenario-isolation choices at draft time.
  - Updated `docs/golden-e2e-testing.md` with a dedicated `Scenario Isolation` section and corresponding ticket expectations.
- Deviations from original plan:
  - Expanded scope to include `tickets/_TEMPLATE.md` because leaving the template weaker than the contract would preserve the same authoring failure at the primary drafting surface.
  - Added explicit workspace test and lint verification to match the requested finalization bar, even though this remained a doc-only ticket with no runtime behavior changes.
  - No runtime tests were added or modified; reassessment confirmed the motivating gap was documentation/process, not missing executable coverage.
- Verification results:
  - `rg -n "heuristic|substrate|scenario isolation|lawful competing affordance|intended branch" tickets/README.md tickets/_TEMPLATE.md docs/golden-e2e-testing.md`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
