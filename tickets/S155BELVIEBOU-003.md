# S155BELVIEBOU-003: Belief-boundary golden E2E + planner-contract doc

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No (test + doc only)
**Deps**: S155BELVIEBOU-001 (effective_place fix), S155BELVIEBOU-002 (can_control gate)

## Problem

The S155 belief-view fixes (001, 002) close two omniscience leaks at the accessor level, but the canonical end-to-end behaviors must be proven at the golden layer and the planner contract must be documented. Without golden E2E coverage, a future regression that reopens the leak (or a snapshot/strategic path that reads authoritative truth another way) would pass the focused unit tests yet behave omnisciently in a full decision cycle. FND-19 agent symmetry also needs a control-source-swap proof. FND-29 requires the absence of leaks to be falsifiable.

## Assumption Reassessment (2026-05-20)

<!-- Spec S155 reassessed this session (/reassess-spec); abbreviated spot-check confirmed targets. -->

1. **Current code/tests**: golden harness is `crates/worldwake-ai/tests/golden_ai.rs` with per-scenario modules under `crates/worldwake-ai/tests/scenarios/` (post-S154 form). Belief-boundary precedent exists: `tests/scenarios/belief_wall_trap.rs`. The accessor contracts proven here are established by 001 (`effective_place`) and 002 (`can_control`), which must land first.
2. **Current specs/docs**: `specs/S155-belief-view-boundary-correctness.md` D3 (golden + focused unit tests) and D4 (doc). `docs/planner-contracts.md` has the `### Entity admission and the belief barrier` section (line ~74) to extend. `docs/FOUNDATIONS.md` Regression Scenario D (stale belief → travel → mismatch → replan), FND-19 (agent symmetry), FND-29 (debuggability).
3. **Shared boundary under audit (cross-system)**: belief-view accessors (001/002) → planning snapshot/strategic place selection → candidate emission → affordances. The golden proves the full chain does not surface a non-co-located moved target or a belief-inaccessible control affordance.
4. **Intended invariants (restated before trusting scenario narrative)**: (a) an agent that last saw a target at P1 plans toward P1 / seeks information, never the target's current P2 (Regression Scenario D); (b) co-located physical chest actions are visible while owner/effective-right/control facts are not, absent a belief entry (FND-14A corollary); (c) swapping `ControlSource::Ai`↔`Human` on the same body yields an identical lawful affordance set (FND-19).
5. **Live `GoalKind` under test**: the stale-location scenario routes through a pursuit/target goal whose target location is read via `effective_place`; confirm the exact live goal family + affordance surface during implementation against the chosen scenario (do not assume a goal family the live planner no longer routes through). The control-source-swap scenario reads the affordance set via the same belief/affordance surface for both control sources.
6. **AI regression layer**: golden E2E (`golden_ai` scenario modules) is the contract layer here; focused-unit coverage for the accessors lives in 001/002. For the control-source-swap symmetry assertion, the proof surface is the affordance set produced through the shared belief/affordance path, asserted equal across control sources.
8. **Scenario isolation**: the stale-location golden must remove lawful competing information channels (no witness/testimony/record delivering P2) so the only thing under test is the belief-vs-authoritative location read; document the excluded channels in the scenario per precision-rules §8.
12. **Isolation choice**: name which lawful affordances are intentionally excluded from the stale-location setup so the agent cannot independently learn P2.
13. **Adjacent contradictions**: if the golden cannot be made to fail against pre-001/002 code (i.e., the leak doesn't manifest end-to-end through the chosen goal), that is a signal the scenario doesn't exercise the leaked path — fix the scenario to route target location through `effective_place`, do not weaken the assertion.

## Architecture Check

1. Golden E2E is the correct layer for proving the *emergent* absence of omniscient pursuit and for FND-19 symmetry, because the leak's danger is in the full decision cycle (snapshot → strategic → candidate → affordance), not just the accessor return value (already covered by 001/002 focused tests). Per precision-rules §6 the focused/accessor proof stays at the lower layer (001/002); this ticket adds the irreducibly-E2E contracts.
2. No backwards-compatibility concern: test + doc only. The doc update names both surfaces (belief-facing `can_control` vs. authoritative `can_exercise_control`) without introducing an alias.

## Verification Layers

1. No omniscient pursuit of a moved, non-co-located target → golden E2E decision/plan trace: agent targets P1 or seeks information, never P2.
2. Co-location reveals physical facts but not social control/ownership facts → golden/decision-trace assertion on the affordance + belief surface beside a chest.
3. Agent symmetry across control-source swap → affordance-set equality assertion through the shared belief/affordance path for `ControlSource::Ai` vs `Human` on the same body (FND-19).
4. Planner-contract documentation accuracy → doc review (no test surface); the doc states the post-001/002 contract.

## What to Change

### 1. Stale target-location golden (Regression Scenario D)

Add a `golden_ai` scenario module (precedent: `tests/scenarios/belief_wall_trap.rs`) where agent A has a belief/last-seen record of target T at P1, T moves to P2 with no information channel delivering P2 to A (document excluded channels per precision-rules §8), and assert A's plan/decision trace targets P1 or an information-seeking action — never P2. Wire into `golden_ai.rs`. The test must fail against pre-001 code.

### 2. Unknown-ownership-beside-chest assertion (FND-14A corollary)

Assert that, co-located with a chest with no belief entry for ownership/rights, the agent's affordance/belief surface exposes physical chest actions but not owner/effective-right/control facts (the `can_control` gate from 002 returns `false` absent a belief path). May be a focused scenario assertion or an extension of the stale-location module if topology allows; keep it a distinct assertion. Must fail against pre-002 code.

### 3. Control-source-swap symmetry golden (FND-19)

Construct a body and capture the lawful affordance set through the shared belief/affordance path under `ControlSource::Ai`, then under `ControlSource::Human` (no other state change), and assert set equality.

### 4. Doc contract update (D4 — subsumed)

In `docs/planner-contracts.md` under `### Entity admission and the belief barrier`, state: non-self `effective_place` is belief/last-seen only (authoritative read permitted solely for same-tick co-located or directly-possessed entities); planning/UI control visibility uses belief-gated `ControlBeliefView::can_control` while dispatch uses authoritative `World::can_exercise_control`.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/` (new — belief-boundary scenario module(s) for stale-location, unknown-ownership, control-source-swap symmetry)
- `crates/worldwake-ai/tests/golden_ai.rs` (modify — register the new scenario module(s))
- `docs/planner-contracts.md` (modify — extend the `### Entity admission and the belief barrier` section)
- `Likely: docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-index.md` (modify — regenerate with `python3 scripts/golden_inventory.py --write --check-docs` after adding the golden test names)

## Out of Scope

- The accessor fixes themselves (`effective_place` → 001, `can_control` → 002).
- Snapshot admission-source provenance tagging — deferred to S157.
- Broad CLI/player-POV affordance audit beyond the single control-source-swap symmetry golden (S155 Non-Goal).

## Acceptance Criteria

### Tests That Must Pass

1. New stale-location golden: agent plans toward P1 / information-seeking, never the moved P2 (fails against pre-001 code).
2. New unknown-ownership assertion: co-located chest exposes physical actions but not control/ownership facts absent a belief entry (fails against pre-002 code).
3. New control-source-swap golden: identical lawful affordance set across `ControlSource::Ai`↔`Human` on the same body.
4. Existing suite: `cargo test -p worldwake-ai`; `python3 scripts/golden_inventory.py --write --check-docs` clean.

### Invariants

1. No belief-view consumer surfaces a non-co-located moved target's current location through the full decision cycle (FND-14/FND-14A end-to-end).
2. Control/ownership facts require a belief entry even when co-located (FND-14A); physical co-located facts do not.
3. The lawful affordance set is identical across control sources for the same body (FND-19).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/<belief_boundary scenario>.rs` (new) — stale-location pursuit (Regression Scenario D) + unknown-ownership assertion; rationale: prove omniscience absence end-to-end, not just at the accessor.
2. `crates/worldwake-ai/tests/scenarios/<control_source_swap>.rs` (new, or same module) — FND-19 affordance-set symmetry across control sources.
3. `crates/worldwake-ai/tests/golden_ai.rs` (modify) — register new scenario module(s).

### Commands

1. `cargo test -p worldwake-ai --test golden_ai` (run the new belief-boundary scenarios; substring-filter the module path)
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `./scripts/verify.sh`
