# S13POLEMEGOLSUI-003: Wounded Politician Priority Ordering Between Care And Office Claim

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `specs/S13-political-emergence-golden-suites.md`, existing S07 emergent care coverage and office suppression coverage

## Problem

The suite already proves political ambition can be suppressed by survival pressure and that wound-vs-hunger ordering varies by utility weights, but it does not yet prove that the same generic ranking machinery resolves ordering between self-care and political ambition without any office-specific priority code.

## Assumption Reassessment (2026-03-19)

1. Existing emergent care ordering coverage in `crates/worldwake-ai/tests/golden_emergent.rs` is `golden_wound_vs_hunger_pain_first`, `golden_wound_vs_hunger_hunger_first`, and `golden_wound_vs_hunger_replays_deterministically`. Those tests prove cross-domain ordering for care vs hunger, not care vs politics.
2. Existing political suppression coverage is `golden_survival_pressure_suppresses_political_goals` and `golden_survival_pressure_suppresses_political_goals_replays_deterministically` in `crates/worldwake-ai/tests/golden_offices.rs`. That scenario proves a binary suppression gate under survival stress, not ordering between two lawful non-suppressed paths.
3. Focused/unit support already exists in ranking and goal-model coverage, including `ranking::tests::claim_office_uses_enterprise_weight_and_medium_priority`, `ranking::tests::self_treat_wounds_uses_pain_weight_for_motive`, and political goal-model tests around `DeclareSupport`. The missing layer is a golden E2E ordering proof across care and politics.
4. The current golden harness already supports the needed setup: medicine seeding, office seeding, direct local belief seeding, utility-profile customization, action tracing, and stable no-natural-recovery wound setup patterns already used in `crates/worldwake-ai/tests/golden_emergent.rs`. This remains test-and-doc scoped.
5. The original ticket overstated the ordering surface. In the current architecture, political intent is expressed by the `declare_support` action and only later resolves into office installation after the succession delay. The ordering proof therefore must compare `heal` vs `declare_support` lifecycle/state effects, not wound decrease vs final office-holder installation.
6. The original same-world, weight-only mirroring assumption does not match the current ranking architecture. `TreatWounds { self }` combines a pain-derived priority class and a pressure-scaled motive (`pain_weight * pain_pressure`), while `ClaimOffice` is a fixed `Medium` priority goal with flat `enterprise_weight` motive. With the same wound severity in both variants, weight changes alone cannot lawfully flip the ordering. The corrected scope must prove the current architecture's real ordering behavior instead of implying a weight-only divergence that the code does not implement.

## Architecture Check

1. The clean design is to extend `golden_emergent.rs` with two focused variants and one replay test, because the scenario is about generic cross-domain ranking behavior rather than a new political rule or a new office helper abstraction.
2. The test must assert action ordering through existing trace/state surfaces, not by introducing or documenting a hardcoded domain tier. That keeps Principle 3 and Principle 20 intact and avoids backsliding into special-case political priority logic.
3. This is more beneficial than pushing new logic into ranking or succession through a golden-ticket backdoor. If the project later wants same-state weight-only divergence between political ambition and self-care, that should be a separate architecture change with a concrete political opportunity signal, not a silent rebalance hidden behind test work.

## Verification Layers

1. candidate presence for both lawful branches (`TreatWounds { self }` and `ClaimOffice { office }`) -> decision trace in `golden_emergent.rs`
2. ordering between care and political commitment -> action trace (`heal` commit vs `declare_support` commit)
3. wound reduction and medicine conservation -> authoritative world state / commodity totals
4. eventual office occupancy -> authoritative world state (`office_holder`)
5. replay determinism -> world-hash and event-log-hash equality

## What to Change

### 1. Add the care-vs-politics ordering golden scenarios

Add the following tests to `crates/worldwake-ai/tests/golden_emergent.rs`:
- `golden_wounded_politician_pain_first`
- `golden_wounded_politician_enterprise_first`
- `golden_wounded_politician_replays_deterministically`

The scenario pair should:
- Reuse the same office setup, medicine setup, and direct office knowledge setup.
- Keep the wound stable so healing order is controlled by AI choice, not natural recovery.
- Use one medium-pain variant and one low-pain variant so the test matches the actual shared ranking contract: medium pain can outrank office ambition, while low pain can leave the office claim path ahead.
- Prove one variant commits `heal` before `declare_support` and the other commits `declare_support` before `heal`, without introducing office-specific priority code.
- Assert both variants still converge to the same lawful end state: wound load reduced and office held.

### 2. Update golden E2E documentation in the same ticket

Review and update the relevant `docs/golden-e2e*` docs after the scenario pair is implemented.

At minimum:
- `docs/golden-e2e-coverage.md`
- `docs/golden-e2e-scenarios.md`

Update counts and scenario summaries to reflect the suite after this ticket only. Do not batch future-ticket docs changes into this diff.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Changing ranking policy or goal-policy semantics for unrelated domains
- Rebalancing `ClaimOffice` motive scaling or introducing a new political opportunity signal
- Adding office-specific suppression or priority exceptions
- Refactoring existing S07 care scenarios except for minimal shared test helpers inside `golden_emergent.rs`
- Engine changes to care, political actions, or succession mechanics unless current behavior contradicts the ticket assumptions and the ticket is first revised

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
2. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_enterprise_first`
3. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_replays_deterministically`
4. `cargo test -p worldwake-ai --test golden_emergent golden_wound_vs_hunger_pain_first`
5. `cargo test -p worldwake-ai --test golden_offices golden_survival_pressure_suppresses_political_goals`
6. Existing suite: `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. Ordering is still driven by concrete utility weights and state, not by new hardcoded domain precedence or political special cases.
2. Both variants must generate both lawful candidates; the difference is ordering, not branch removal.
3. The pain-first variant must commit `heal` before `declare_support`, and the enterprise-first variant must commit `declare_support` before `heal`.
4. Both variants must end with the same authoritative facts: the agent is office holder and wound load has decreased through lawful care actions.
5. Medicine and support declarations remain explicitly conserved/accounted for; the scenario must not create extra medicine, office support, or duplicate office holders.
6. Same-seed replay remains deterministic at both world-hash and event-log-hash level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — add the pain-first and enterprise-first political emergence variants plus deterministic replay coverage. Rationale: prove mixed-layer cross-domain ordering using the existing generic ranking architecture.
2. `docs/golden-e2e-coverage.md` — record the new cross-domain ordering coverage and revised suite totals.
3. `docs/golden-e2e-scenarios.md` — add the scenario catalog entry describing the two mirrored variants and shared end-state invariant.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
2. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_enterprise_first`
3. `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_replays_deterministically`
4. `cargo test -p worldwake-ai --test golden_emergent`
5. `cargo test -p worldwake-ai --test golden_offices golden_survival_pressure_suppresses_political_goals`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-03-19
- **What actually changed**:
  - Added `golden_wounded_politician_pain_first`, `golden_wounded_politician_enterprise_first`, and `golden_wounded_politician_replays_deterministically` to `crates/worldwake-ai/tests/golden_emergent.rs`.
  - Updated `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` for the new scenario and revised suite counts.
  - Corrected the ticket assumptions before implementation so the test proves the current architecture's real contract: mixed-layer ordering between `heal` and `declare_support`, not a same-world weight-only flip between wound reduction and final office installation.
- **Deviations from original plan**:
  - The original ticket assumed the same wound severity could flip ordering by changing only `pain_weight` and `enterprise_weight`. Current ranking semantics do not support that: `TreatWounds` uses both a pain-derived priority class and a pressure-scaled motive, while `ClaimOffice` is fixed `Medium` with flat enterprise motive.
  - The implemented scenario therefore uses one medium-pain variant and one low-pain variant to prove the existing shared ranking architecture without introducing hidden ranking-policy changes.
- **Verification results**:
  - Passed targeted tests:
    - `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_pain_first`
    - `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_enterprise_first`
    - `cargo test -p worldwake-ai --test golden_emergent golden_wounded_politician_replays_deterministically`
  - Passed related suites:
    - `cargo test -p worldwake-ai --test golden_emergent`
    - `cargo test -p worldwake-ai --test golden_offices golden_survival_pressure_suppresses_political_goals`
    - `cargo test -p worldwake-ai`
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets -- -D warnings`
