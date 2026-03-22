# DOCTESTPOL-001: Tighten Ordering And Verification Contracts For AI And Golden Tickets

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/README.md`, `docs/golden-e2e-testing.md`, `docs/FOUNDATIONS.md`, archived `archive/tickets/S13POLEMEGOLSUI-003-wounded-politician-priority-ordering.md`

## Problem

Recent ticket work exposed a repeatable failure mode in planning and golden-test tickets: the ticket text can blur candidate generation, ranking, action ordering, and delayed authoritative resolution into one vague "ordering" claim. That creates stale or misleading acceptance criteria, weakens architectural reasoning, and makes it easier to design tests against the wrong layer.

This ticket tightens the documentation contract so future tickets must name the exact comparison layer and state whether the claimed divergence depends on priority class, motive score, suppression, or delayed system resolution.

## Assumption Reassessment (2026-03-19)

1. `tickets/README.md` already requires precise layer naming, harness-boundary naming, and explicit verification-layer mapping, but it does not yet require ordering-sensitive tickets to declare whether the compared branches are symmetric in the current architecture or whether the claimed divergence depends on priority class, motive score, suppression, delayed system resolution, or mixed-layer timing.
2. `docs/golden-e2e-testing.md` already distinguishes authoritative world state, action traces, decision traces, and event-log assertions, and it already tells mixed-layer goldens to use the strongest assertion surface available. The remaining gap is narrower: it does not yet call out the specific ordering pitfall exposed by the wounded-politician work, where delayed office installation is not a direct proxy for earlier political action ordering, and where "same-state, weight-only divergence" claims are only valid when both branches share a comparable ranking substrate.
3. Archived ticket [S13POLEMEGOLSUI-003-wounded-politician-priority-ordering.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S13POLEMEGOLSUI-003-wounded-politician-priority-ordering.md) demonstrates the gap concretely. Its corrected `Assumption Reassessment` and `Verification Layers` sections already distinguish candidate generation, action lifecycle ordering, authoritative office installation, and the ranking-architecture mismatch around same-world weight-only crossover.
4. This is a docs-contract ticket, not an AI behavior change. Existing runtime coverage already exists and has been verified against current test names: `ranking::tests::claim_office_uses_enterprise_weight_and_medium_priority`, `ranking::tests::self_treat_wounds_uses_pain_weight_for_motive`, `golden_wounded_politician_pain_first`, `golden_wounded_politician_enterprise_first`, `golden_wounded_politician_replays_deterministically`, and `golden_survival_pressure_suppresses_political_goals`.
5. Mismatch + correction: the current ticket overstates the documentation gap slightly and contains one factual error. The docs already cover strongest-assertion-surface discipline in general, and `docs/FOUNDATIONS.md` does not contain a `Principle 27`. The corrected scope is to strengthen the ticket contract and golden guidance with the missing ordering-specific caveats, not to imply a broader missing verification architecture or any runtime ranking change.

## Architecture Check

1. The clean fix is to strengthen the authoring contract and golden-testing guidance instead of relying on reviewers to rediscover the same category error ticket by ticket. That improves design quality without introducing any runtime aliasing or compatibility layers.
2. This aligns with the foundations document's overall explainable-emergence standard: tickets and golden guidance should require claims that can be traced back to the actual causal layer under test instead of collapsing multiple lawful delays into one vague ordering claim.

## Verification Layers

1. ticket authoring contract names exact behavior layers and ordering surfaces -> docs review in `tickets/README.md` and `tickets/_TEMPLATE.md`
2. golden testing guidance names correct assertion-surface selection and ordering-specific mixed-layer caveats -> docs review in `docs/golden-e2e-testing.md`
3. single-layer ticket: no additional runtime trace/event-log mapping is applicable because this change is documentation-only

## What to Change

### 1. Tighten the ticket authoring contract

Update `tickets/README.md` and `tickets/_TEMPLATE.md` so ordering-sensitive tickets must explicitly state:
- which ordering layer is the contract: candidate generation, ranking/suppression, action lifecycle, authoritative state, or delayed system resolution
- whether the compared branches are symmetric in the current architecture
- whether the intended divergence depends on priority class, motive score, suppression, delayed resolution, or a combination
- that delayed authoritative effects must not be used as a proxy for earlier action ordering when a lower-layer surface exists

### 2. Tighten golden E2E guidance

Update `docs/golden-e2e-testing.md` with explicit guidance that:
- office installation is not a direct proxy for political-action ordering because succession can add lawful delay
- "same-state, weight-only divergence" should only be claimed if both branches are driven by comparable ranking substrates
- mixed-layer goldens must prove each layer on its own strongest assertion surface instead of collapsing the full chain into one outcome check

## Files to Touch

- `/home/joeloverbeck/projects/worldwake/tickets/README.md` (modify)
- `/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md` (modify)
- `/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md` (modify)

## Out of Scope

- Any change to ranking policy, candidate generation, suppression, or succession semantics
- New golden scenarios or unit tests for runtime behavior
- Changes to archived tickets beyond using them as references for the doc wording

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `scripts/verify.sh`

### Invariants

1. No documentation change may imply a new runtime contract that the current engine does not implement.
2. The docs must reinforce, not weaken, the existing no-shim / no-alias / strongest-assertion-surface discipline.

## Test Plan

### New/Modified Tests

1. None. This is a documentation-only ticket; no runtime tests are added or modified.
2. Existing runtime coverage relied on by this ticket remains `ranking::tests::claim_office_uses_enterprise_weight_and_medium_priority`, `ranking::tests::self_treat_wounds_uses_pain_weight_for_motive`, `golden_wounded_politician_pain_first`, `golden_wounded_politician_enterprise_first`, `golden_wounded_politician_replays_deterministically`, and `golden_survival_pressure_suppresses_political_goals`.
3. The deliverable is updated documentation in `tickets/README.md`, `tickets/_TEMPLATE.md`, and `docs/golden-e2e-testing.md`; verification is command-based rather than new test coverage.

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `scripts/verify.sh`

## Outcome

- **Completion date**: 2026-03-19
- **What actually changed**:
  - Corrected the active ticket before implementation so its assumptions now match the current docs and test surfaces.
  - Tightened `tickets/README.md` so ordering-sensitive tickets must name branch symmetry, divergence drivers, and when delayed authoritative effects cannot stand in for earlier ordering.
  - Tightened `tickets/_TEMPLATE.md` so new tickets prompt for those ordering-specific details up front.
  - Updated `docs/golden-e2e-testing.md` with explicit guidance for delayed office-installation timing and for avoiding false "same-state, weight-only divergence" claims across asymmetric ranking substrates.
- **Deviations from original plan**:
  - No runtime tests were added or modified because the reassessment confirmed this was a documentation-contract gap, not missing engine coverage.
  - The ticket itself required correction before doc implementation because it cited a nonexistent `Principle 27` and listed documentation files as tests.
- **Verification results**:
  - Passed `cargo test --workspace`
  - Passed `cargo clippy --workspace --all-targets -- -D warnings`
  - Passed `scripts/verify.sh`
