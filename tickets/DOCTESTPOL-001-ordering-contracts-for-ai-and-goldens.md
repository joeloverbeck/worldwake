# DOCTESTPOL-001: Tighten Ordering And Verification Contracts For AI And Golden Tickets

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/README.md`, `docs/golden-e2e-testing.md`, `docs/FOUNDATIONS.md`, archived `archive/tickets/S13POLEMEGOLSUI-003-wounded-politician-priority-ordering.md`

## Problem

Recent ticket work exposed a repeatable failure mode in planning and golden-test tickets: the ticket text can blur candidate generation, ranking, action ordering, and delayed authoritative resolution into one vague "ordering" claim. That creates stale or misleading acceptance criteria, weakens architectural reasoning, and makes it easier to design tests against the wrong layer.

This ticket tightens the documentation contract so future tickets must name the exact comparison layer and state whether the claimed divergence depends on priority class, motive score, suppression, or delayed system resolution.

## Assumption Reassessment (2026-03-19)

1. `tickets/README.md` already requires precise layer naming and explicit verification-layer mapping, but it does not yet require ordering tickets to declare whether the compared branches are actually symmetric in the current architecture or whether the claim depends on priority class, motive score, suppression, or delayed system resolution.
2. `docs/golden-e2e-testing.md` already distinguishes action-trace, decision-trace, event-log, and authoritative-world-state assertions, but it does not explicitly warn that delayed office installation is not a direct proxy for political action ordering or that "same-state, weight-only divergence" should only be claimed when both branches share a comparable ranking substrate.
3. Archived ticket [S13POLEMEGOLSUI-003-wounded-politician-priority-ordering.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S13POLEMEGOLSUI-003-wounded-politician-priority-ordering.md) demonstrates the gap concretely: the original ticket conflated `heal` vs `declare_support` ordering with later office installation and initially assumed a same-world weight-only crossover the current ranking architecture does not implement.
4. This is a docs-contract ticket, not an AI behavior change. Existing code coverage already exists for the underlying runtime behavior in `crates/worldwake-ai/src/ranking.rs`, `crates/worldwake-ai/tests/golden_emergent.rs`, and `crates/worldwake-ai/tests/golden_offices.rs`; the gap is specification quality, not missing engine behavior.
5. Mismatch + correction: the current ticket/template contract is directionally correct but not yet explicit enough for cross-domain ordering tickets. The corrected scope is to strengthen docs and template language, not to change ranking semantics.

## Architecture Check

1. The clean fix is to strengthen the authoring contract and golden-testing guidance instead of relying on reviewers to rediscover the same category error ticket by ticket. That improves design quality without introducing any runtime aliasing or compatibility layers.
2. This aligns with Principle 27 in `docs/FOUNDATIONS.md`: debugability and explainability are product features, and the ticket/docs layer should require explanations that match the actual architecture.

## Verification Layers

1. ticket authoring contract names exact behavior layers and ordering surfaces -> docs review in `tickets/README.md` and `tickets/_TEMPLATE.md`
2. golden testing guidance names correct assertion-surface selection and mixed-layer ordering rules -> docs review in `docs/golden-e2e-testing.md`
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

1. The updated docs/template explicitly require ordering tickets to name the exact behavioral layer and dependency type (`priority class`, `motive score`, `suppression`, `delayed system resolution`) instead of allowing a vague ordering claim.
2. The updated golden guidance explicitly states that delayed authoritative installation is not a proxy for political action ordering and that same-state weight-only divergence requires a comparable ranking substrate.
3. Existing suite: `cargo test --workspace`

### Invariants

1. No documentation change may imply a new runtime contract that the current engine does not implement.
2. The docs must reinforce, not weaken, the existing no-shim / no-alias / strongest-assertion-surface discipline.

## Test Plan

### New/Modified Tests

1. `/home/joeloverbeck/projects/worldwake/tickets/README.md` — clarify required ordering-contract fields so future tickets cannot collapse mixed layers.
2. `/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md` — make the stronger contract visible at ticket creation time.
3. `/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md` — codify assertion-surface guidance for mixed-layer and delayed-resolution scenarios.

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `scripts/verify.sh`

