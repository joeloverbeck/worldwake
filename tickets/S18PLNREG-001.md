# S18PLNREG-001: Add focused planner regression coverage for stale-belief branch replacement

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` focused runtime/integration tests and small helper additions if needed
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, `tickets/README.md`, `archive/tickets/completed/S18PREAWAEME-003.md`, `tickets/S18AIPLTRACE-001.md`

## Problem

`S18PREAWAEME-003` fixed a real planner contradiction and added one golden plus several focused tests, but the coverage is still fragmented. The current regression net proves individual filters and one end-to-end stale-belief scenario, yet it does not provide a focused planner-runtime test cluster that exercises the full belief-local branch replacement contract at the earliest causal boundaries.

That makes it too easy for future changes to reintroduce stale-plan retention, blocker overreach, or candidate-evidence leaks while the failure only becomes obvious in a broad golden. The architecture should prove this contract first at the planner layer and only then at the golden layer.

## Assumption Reassessment (2026-03-21)

1. Existing focused coverage is real but distributed across separate symbols:
   - `goal_model::places_with_resource_source()` pruning in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   - `candidate_generation::acquisition_path_evidence_inner()` evidence filtering in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
   - `plan_selection::select_best_plan()` stale-retention behavior in [plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
   - `BlockedIntent::blocks_goal_generation()` semantics in [blocked_intent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs)
2. Existing golden coverage in `golden_stale_prerequisite_belief_discovery_replan` proves the end-to-end supply-chain recovery, but that scenario also depends on full action registries, travel, harvest, craft, and market restock. It is intentionally broader than the earliest planner contract.
3. The intended verification layer for this ticket is mixed:
   - focused planner/runtime decision-trace coverage for branch replacement
   - focused unit coverage for preconditions already added
   - one golden/E2E scenario for downstream causal confirmation
4. The relevant divergence is not strict tick separation or event-log ordering. The contract is a planning-layer divergence driven by new local evidence invalidating one branch and fresh search selecting another. If ordering matters in focused tests, it is the first post-perception planning tick, not event-log order.
5. This ticket does not remove a heuristic. It hardens the test architecture around already-correct production behavior.
6. This is not a start-failure ticket. The corrected architecture for the motivating scenario is earlier replanning after local perception, not authoritative `StartFailed`.
7. The local needs-only `agent_tick` harness is not sufficient for the motivating branch because the scenario depends on non-needs affordances across travel, harvest, production, and restock. Focused runtime coverage here should either use the existing full registries or an equally lawful narrowed harness that still includes those action families.
8. Mismatch + correction: the gap is not absence of any tests; it is absence of a concentrated regression matrix at the planner-runtime boundary for stale-belief branch replacement.
9. Authoritative arithmetic is not the key issue. The branch depends on belief correction plus planner viability, not on hidden numeric thresholds.

## Architecture Check

1. The clean architecture is layered coverage: prove planner semantics in focused runtime tests first, then keep goldens for downstream causal confirmation. That matches `docs/golden-e2e-testing.md` and avoids using broad scenarios as the first debugging surface.
2. If helper code is needed, it should live in existing test support modules and encode semantic assertions such as “first post-perception selected branch changed because the stale source became non-viable,” not brittle tick-count assumptions.
3. No backwards-compatibility test aliasing or duplicate harness stacks should be introduced. Reuse the current `worldwake-ai` focused test surfaces and golden harness.

## Verification Layers

1. depleted source is absent from prerequisite-place and actionable-evidence inputs -> focused unit tests in `goal_model` and `candidate_generation`
2. stale current plan is not retained once the current goal has no fresh viable plan -> focused plan-selection/runtime decision-trace test
3. local perception invalidates the stale branch and fresh search selects the lawful fallback on the first relevant planning tick -> focused runtime decision-trace test with full lawful affordances
4. end-to-end supply-chain recovery still occurs after branch replacement -> golden/E2E test in `golden_supply_chain`

## What to Change

### 1. Add a concentrated stale-belief planner regression cluster

Add focused runtime/integration tests under `worldwake-ai` that exercise this exact causal chain:

- stale belief makes branch A initially viable
- local perception invalidates branch A
- planner does not retain branch A as the current plan
- fresh search selects branch B
- blocker semantics do not suppress same-goal regeneration

Keep these tests narrower than the golden and assert on decision traces rather than only on later world outcomes.

### 2. Add reusable semantic test helpers if they remove duplication

If repeated assertions appear across the new focused tests and existing golden coverage, add small test-only helpers for:

- extracting the first post-perception planning trace
- asserting selected-plan source and selected next-step semantics
- asserting stale-branch absence without relying on missing action commits

Helpers must encode semantic invariants, not scenario-specific magic values.

### 3. Strengthen the stale-belief golden only where it adds downstream value

Keep the golden focused on the causal branch that matters end-to-end. Do not broaden it into a general planner test matrix. Use it to confirm downstream harvest/craft/restock consequences and deterministic replay after the focused runtime coverage proves the earlier branch replacement contract.

## Files to Touch

- `crates/worldwake-ai/src/plan_selection.rs` (modify, tests)
- `crates/worldwake-ai/src/agent_tick.rs` (modify, tests)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/` (modify only if a small reusable semantic helper is justified)

## Out of Scope

- new production planner behavior beyond what tests reveal as still missing
- broad new multi-agent goldens unrelated to stale-belief branch replacement
- trace-schema expansion beyond what `S18AIPLTRACE-001` defines

## Acceptance Criteria

### Tests That Must Pass

1. a focused runtime test proves the first post-perception planning trace selects a fresh fallback instead of retaining the stale current plan
2. a focused runtime test proves same-goal regeneration remains allowed after source depletion is observed
3. the stale-belief supply-chain golden and its replay companion still pass
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. planner-runtime tests assert semantic branch replacement boundaries, not incidental scheduler timing
2. broad goldens are not the only proof surface for stale-belief recovery behavior

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — add focused runtime/decision-trace coverage for first post-perception branch replacement
2. `crates/worldwake-ai/src/plan_selection.rs` — extend stale-plan-retention focused coverage where needed to match the runtime contract
3. `crates/worldwake-ai/tests/golden_supply_chain.rs` — keep or strengthen the stale-belief golden only for downstream causal confirmation and replay

### Commands

1. `cargo test -p worldwake-ai --lib`
2. `cargo test -p worldwake-ai --test golden_supply_chain`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
