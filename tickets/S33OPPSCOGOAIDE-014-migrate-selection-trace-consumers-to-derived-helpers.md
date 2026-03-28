# S33OPPSCOGOAIDE-014: Migrate selection-trace consumers to derived helper APIs

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace consumer and test-query surface cleanup
**Deps**: tickets/S33OPPSCOGOAIDE-013-canonicalize-selection-trace-identity.md, archive/tickets/completed/S33OPPSCOGOAIDE-012-trace-test-query-surface.md

## Problem

Even after the helper work in S33OPPSCOGOAIDE-012, several focused and golden tests still read the raw selected-goal storage path directly. Once `SelectionTrace` is canonicalized on `selected_opportunity`, those consumers should move to explicit derived helpers instead of recreating field-shape knowledge in each test.

This is the consumer-side half of the same architectural cleanup: the trace model should have one concrete selected-branch identity, and trace consumers should query that identity through stable helper methods.

## Assumption Reassessment (2026-03-28)

1. The exact shared abstraction boundary under audit is the public decision-trace consumer surface after selection has been canonicalized on `selected_opportunity`: `SelectionTrace` helper methods in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and the focused/golden tests that still read `selection.selected` directly.
2. Current remaining direct consumers were verified in the live codebase with `rg` on 2026-03-28. Representative reads exist in [`crates/worldwake-ai/tests/golden_care.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs), [`crates/worldwake-ai/tests/golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs), [`crates/worldwake-ai/tests/golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs), and [`crates/worldwake-ai/tests/golden_combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs).
3. This is a coverage-surface and consumer-contract ticket, not a planner-behavior ticket. Existing focused and golden scenarios already prove the relevant selection behavior; the gap is that they still encode raw field layout assumptions.
4. The intended verification layer is decision-trace helper coverage plus representative golden migrations. Action-trace or authoritative-state proof is not the contract here because the change is only to the decision-trace read surface.
5. The same fact currently has two consumption paths in tests:
   - raw reads of `planning.selection.selected`
   - helper methods such as `selected_goal_is()` and opportunity-scoped helper queries added in S33OPPSCOGOAIDE-012
   After this ticket, the canonical path for selected-goal assertions should be derived helper queries on `SelectionTrace`.
6. Planner- and golden-driven scenarios involved here include live `GoalKind` families `TreatWounds`, `RestockCommodity`, `InvestigateViolation`, `ReduceDanger`, and `ClaimOffice`. The relevant operator/prerequisite surfaces vary by scenario, but the asserted invariant is the same: tests should prove selected-goal/selected-branch facts through helper APIs, not by traversing raw trace fields.
7. Coverage gap classification:
   - focused runtime coverage already exists in `agent_tick::tests::*trace*`
   - golden coverage already exists in the named files above
   - the missing piece is stable helper-based consumption coverage across those existing tests, not new behavioral scenario coverage
8. Mismatch + correction: this ticket is not “replace every raw trace field read in the repo.” Scope is only the remaining selected-goal consumer seam tied to S33 selection identity cleanup. Other raw trace reads should become their own ticket only if they represent a distinct architectural contradiction.
9. Adjacent contradiction exposed by reassessment: if S33OPPSCOGOAIDE-013 removes the stored `selected` field but this migration ticket does not land, tests will fail for mechanical reasons and consumers will be pushed toward ad-hoc re-derivation. That is an in-scope consequence this ticket exists to avoid.
10. No ranking arithmetic claims or ordering claims are being changed here. When a scenario asserts “X was selected,” the proof surface remains decision-trace selection state, not downstream event ordering.

## Architecture Check

1. Migrating consumers to helper APIs is cleaner than updating each test to re-derive `GoalKey` from `selected_opportunity` by hand. One helper surface is easier to reason about and less brittle than many bespoke local rewrites.
2. This aligns with `docs/FOUNDATIONS.md` Principle 25: derived summaries belong in one intentional read surface, not scattered as duplicate local logic across tests.
3. No backwards-compatibility shim should be added for removed raw fields. Consumers should move to the canonical helper surface rather than preserving old layout expectations.

## Verification Layers

1. Helper-based selected-goal assertions remain sufficient for focused runtime scenarios -> focused `agent_tick` runtime trace tests.
2. Helper-based selected-goal assertions remain sufficient for representative golden scenarios across multiple goal families -> representative `golden_*` tests.
3. Additional action-trace or authoritative-world verification is not applicable because this ticket changes only trace consumer assertions and helper-query ergonomics.

## What to Change

### 1. Complete helper-based migration for selected-goal assertions

- Replace remaining direct reads of `selection.selected` in focused and golden tests with `selected_goal_is()`, `selected_goal()`, or equivalent derived helper APIs added by S33OPPSCOGOAIDE-013.
- Where a scenario really needs the concrete branch identity, prefer `selected_opportunity` or an opportunity-scoped helper instead of reintroducing goal-only shortcuts.

### 2. Tighten helper ergonomics only where migration proves they are missing

- If migration exposes repetitive helper gaps, add small explicit read helpers on `SelectionTrace` rather than open-coding derived logic in each test.
- Do not expand this into a broad trace-fixture rewrite or unrelated candidate-generation helper work.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — only if a small helper addition is needed during migration)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — any remaining direct selected-goal reads)
- `crates/worldwake-ai/tests/golden_care.rs` (modify)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify)

## Out of Scope

- Changing selection behavior, ranking, suppression, or plan search
- Candidate-generation helper migrations unrelated to selected-goal reads
- Save/load changes
- Repo-wide normalization of every direct trace field access

## Acceptance Criteria

### Tests That Must Pass

1. Representative focused and golden tests assert selected-goal facts through derived helper APIs rather than raw `selection.selected` field access.
2. The migrated tests still prove the same scenario invariants as before.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Selected-goal consumer queries derive from canonical selected-opportunity trace state.
2. No consumer reintroduces a new ad-hoc goal-only alias path.
3. No compatibility shim is added to keep removed raw-field reads alive.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_care.rs` — migrate remote-care selected-goal assertions.
   Rationale: proves helper-based selected-goal reads work in a care/travel scenario.
2. `crates/worldwake-ai/tests/golden_supply_chain.rs` — migrate restock selected-goal assertions that currently read `selection.selected`.
   Rationale: proves helper-based selected-goal reads work in a stale-belief/replan scenario.
3. `crates/worldwake-ai/tests/golden_emergent.rs` and `crates/worldwake-ai/tests/golden_combat.rs` — migrate representative political, investigation, and self-care selected-goal assertions.
   Rationale: proves the helper surface is sufficient across multiple goal families without relying on raw trace layout.

### Commands

1. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker`
2. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan`
3. `cargo test -p worldwake-ai golden_same_place_concurrent_violations_stay_distinct`
4. `cargo test -p worldwake-ai golden_traceability_explains_stale_fine_branch_without_source_diving`
5. `cargo test -p worldwake-ai golden_reduce_danger_defensive_mitigation`
6. `cargo test -p worldwake-ai`
