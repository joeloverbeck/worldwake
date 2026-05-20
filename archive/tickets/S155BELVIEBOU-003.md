# S155BELVIEBOU-003: Belief-boundary golden E2E + planner-contract doc

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No (test + doc only)
**Deps**: `archive/tickets/S155BELVIEBOU-001.md` (effective_place fix), `archive/tickets/S155BELVIEBOU-002.md` (can_control gate)

## Problem

The S155 belief-view fixes (001, 002) close two omniscience leaks at the accessor level, but the canonical end-to-end behaviors must be proven at the golden layer and the planner contract must be documented. Without golden E2E coverage, a future regression that reopens the leak (or a snapshot/strategic path that reads authoritative truth another way) would pass the focused unit tests yet behave omnisciently in a full decision cycle. FND-19 agent symmetry also needs a control-source-swap proof. FND-29 requires the absence of leaks to be falsifiable.

## Assumption Reassessment (2026-05-20)

<!-- Spec S155 reassessed this session (/reassess-spec); abbreviated spot-check confirmed targets. -->

1. **Current code/tests**: golden harness is `crates/worldwake-ai/tests/golden_ai.rs` with per-scenario modules under `crates/worldwake-ai/tests/scenarios/` (post-S154 form). Belief-boundary precedent exists: `tests/scenarios/belief_wall_trap.rs`. The accessor contracts proven here were established by completed dependencies 001 (`effective_place`) and 002 (`can_control`).
2. **Current specs/docs**: `archive/specs/S155-belief-view-boundary-correctness.md` D3 (golden + focused unit tests) and D4 (doc). `docs/planner-contracts.md` has the `### Entity admission and the belief barrier` section (line ~74) to extend. `docs/FOUNDATIONS.md` Regression Scenario D (stale belief -> travel -> mismatch -> replan), FND-19 (agent symmetry), FND-29 (debuggability).
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

## Verified Layers

1. No omniscient pursuit of a moved, non-co-located target → golden E2E decision/plan trace: agent targets P1 or seeks information, never P2.
2. Co-location reveals physical facts but not social control/ownership facts → golden/decision-trace assertion on the affordance + belief surface beside a chest.
3. Agent symmetry across control-source swap → affordance-set equality assertion through the shared belief/affordance path for `ControlSource::Ai` vs `Human` on the same body (FND-19).
4. Planner-contract documentation accuracy → doc review (no test surface); the doc states the post-001/002 contract.

## Landed Changes

### 1. Stale target-location golden (Regression Scenario D)

Extended `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` with Scenario 454. The fixture seeds an actor belief that a hostile target was at `ORCHARD_FARM`, silently moves the target to `RULERS_HALL`, and asserts both candidate evidence and decision trace pursuit diagnostics use the stale believed place rather than the current authoritative place.

### 2. Unknown-ownership-beside-chest assertion (FND-14A corollary)

Kept the existing Scenario 420 belief-wall ownership coverage in the same module and regenerated the generated docs so it remains listed as the FND-14A physical/social split proof. The active golden asserts local physical observation remains visible while authority beliefs are unknown and theft does not emit without an explicit owner belief.

### 3. Control-source-swap symmetry golden (FND-19)

Added Scenario 455 in the same module. The test fingerprints the full belief-facing affordance set with the actor under `ControlSource::Ai`, switches the same body to `ControlSource::Human` without other state changes, and asserts the affordance set is identical.

### 4. Doc contract update (D4 — subsumed)

Updated `docs/planner-contracts.md` under `### Entity admission and the belief barrier` to state that non-self `effective_place` is belief/last-seen only except same-tick co-location or direct possession, and that planning/UI control visibility uses belief-gated `ControlBeliefView::can_control` while dispatch uses authoritative `World::can_exercise_control`.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs`
- `docs/planner-contracts.md`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-details/belief-wall-trap.md`
- `docs/generated/golden-scenario-details/scaled-contention.md`
- `docs/generated/golden-scenario-index.md`

## Out of Scope

- The accessor fixes themselves (`effective_place` → 001, `can_control` → 002).
- Snapshot admission-source provenance tagging — completed later by
  `archive/specs/S157-planner-snapshot-admission-provenance.md`.
- Broad CLI/player-POV affordance audit beyond the single control-source-swap symmetry golden (S155 Non-Goal).

## Acceptance Result

### Verified Outcomes

1. Scenario 454 proves stale remote pursuit reads the actor's last-known place and excludes the target's current remote place from candidate evidence and decision-trace pursuit diagnostics.
2. Scenario 420 continues to prove co-located physical chest/facility facts are visible while owner, holder, jurisdiction, and office-holder facts stay unknown without belief entries.
3. Scenario 455 proves identical belief-facing affordance fingerprints across `ControlSource::Ai` and `ControlSource::Human` on the same body.
4. `cargo test -p worldwake-ai` and `python3 scripts/golden_inventory.py --write --check-docs` both passed.

### Invariants

1. No belief-view consumer surfaces a non-co-located moved target's current location through the full decision cycle (FND-14/FND-14A end-to-end).
2. Control/ownership facts require a belief entry even when co-located (FND-14A); physical co-located facts do not.
3. The lawful affordance set is identical across control sources for the same body (FND-19).

## Outcome

Completed on 2026-05-20.

- Added active golden coverage in the existing `belief_wall_trap.rs` suite for stale remote pursuit and control-source-swap symmetry.
- Preserved the existing unknown-ownership belief-wall golden as the FND-14A physical/social split proof and regenerated the generated golden inventory/index/detail docs.
- Updated `docs/planner-contracts.md` with the S155 belief-view contract for non-self `effective_place` and belief-gated planning/UI control visibility.

## Deviations

- Reused and extended `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` instead of creating a new scenario module or editing `golden_ai.rs`; the module was already registered and was the strongest existing belief-boundary owner.
- `docs/generated/golden-scenario-details/scaled-contention.md` changed only by generated source-line drift after inserting new belief-wall tests.
- `./scripts/verify.sh` remains the harness final pre-push gate for the whole S155 family; this ticket's completed proof used the focused belief-wall selector, the golden inventory generator, and the full `worldwake-ai` suite.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai belief_wall_trap -- --nocapture`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
- Waived `./scripts/verify.sh` for this per-ticket closeout because the harness runs it before final branch push; the ticket-specific proof boundary is the full `worldwake-ai` suite plus regenerated golden docs.
