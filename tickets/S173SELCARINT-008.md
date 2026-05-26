# S173SELCARINT-008: Scenario D — Player POV symmetry for occupancy

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (CLI/scenario assertion only)
**Deps**: `archive/tickets/S173SELCARINT-001.md` (uses `SelfCareOccupancy`), `archive/tickets/S173SELCARINT-004.md` (occupancy lifecycle in wash/toilet), `archive/tickets/S173SELCARINT-006.md` (emitter-time read pattern), `specs/S173-self-care-interruption-occupancy.md` (D9, Scenario D)

## Problem

The CLI player-POV boundary (`archive/specs/S163-cli-player-pov-boundary.md`, archived) enforces that the controlled agent's display surface only shows what that agent could lawfully perceive, infer, or recall. With `SelfCareOccupancy` introduced as a new authoritative facility-side component, the CLI must respect the same gating — a controlled agent at a place without a co-located basin and without belief about a remote basin must not see remote `SelfCareOccupancy` state in any CLI accessor output (FND-19 agent symmetry, FND-14 world-state vs belief-state separation). This ticket adds the assertion as an extension of S172's Scenario D location, completing the player-POV gating contract for self-care occupancy.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. S163 (CLI POV boundary) is archived at `archive/specs/S163-cli-player-pov-boundary.md`. S172 (Wash discovery and budget closure) is archived at `archive/specs/S172-wash-discovery-budget-closure.md` and contains a Scenario D for the player-POV pattern. Verify the exact S172 Scenario D test name and location at implementation time — likely in `crates/worldwake-ai/tests/scenarios/` or `crates/worldwake-cli/tests/`. The new assertion extends that existing scenario rather than creating a fresh scenario file unless the existing scenario's surface doesn't naturally absorb the new assertion.
2. CLI accessor surface: `crates/worldwake-cli/src/bin/observer.rs` and `crates/worldwake-cli/src/handlers/*` (verify at implementation time). The relevant accessor for the controlled agent's view of facility state would be where `WashBasinState` or facility-occupancy summaries are rendered. Per Step 2 spot-check, no existing direct CLI consumer of `SelfCareOccupancy` exists — this ticket's assertion verifies the **absence** of the leak, not the addition of a new render path.
3. Shared abstraction boundary: the player-POV display path. Per FND-19, swapping a controlled agent from AI to human or human to AI must not change the legal action set or the visible information set. The CLI surface for `SelfCareOccupancy` must obey the same FND-14A/14B source-class split as the planner-side emitter from ticket 006.
4. Authoritative-to-AI Impact Rule does not apply directly — this is a CLI-only ticket, not a planner-modifying one. The Auth-to-AI checklist coverage lives in tickets 004, 005, 006.
5. Goal infrastructure not affected — no candidate emission, no plan composition.

## Architecture Check

1. The CLI accessor's gating reuses the existing player-POV machinery from S163 — no new mechanism. The assertion proves that the existing machinery covers the new component type by virtue of the `SelfCareOccupancy` accessor going through the same belief-view-or-co-located-observation surface as `WashBasinState`.
2. Per FND-19 agent symmetry, the same gating that filters remote `WashBasinState` reads filters remote `SelfCareOccupancy` reads — both are facility-side authoritative state subject to FND-14B.
3. The assertion is a negative-case test (no remote `SelfCareOccupancy` displayed) per `docs/precision-rules.md` Rule 8 scenario isolation. The lawful competing affordance (a controlled agent at the basin's place WOULD see the occupancy via FND-14A) is documented as intentionally excluded from the negative-case scenario.

## Verification Layers

1. CLI accessor output filtering → focused assertion in the extended S172 Scenario D: scenario sets up a controlled agent at a place WITHOUT a co-located basin and WITHOUT belief of a remote basin; basin has `SelfCareOccupancy` present in world state; CLI accessor output for the controlled agent shows no occupancy data.
2. Positive case (sanity check): controlled agent co-located with the basin DOES see the occupancy through the FND-14A path. This guard prevents a regression where the gating becomes over-restrictive and hides legitimately-observable state.
3. Single-layer ticket (CLI gating only). No engine-layer changes; the existing wash/toilet contract from ticket 004 is exercised as setup, not asserted upon.

## What to Change

### 1. Extend S172's Scenario D with `SelfCareOccupancy` gating assertion

Locate the S172 Scenario D test file (verify path at implementation time via `archive/specs/S172-wash-discovery-budget-closure.md` and the corresponding scenario file in `crates/worldwake-ai/tests/scenarios/` or `crates/worldwake-cli/tests/`). Extend the existing setup with:

- A second basin at a remote place, with `SelfCareOccupancy { occupant: bot_agent }` pre-written.
- Assertion: the controlled agent's CLI display surface shows no occupancy data for the remote basin.

If the S172 Scenario D file does not naturally absorb the new assertion (e.g., its scope is tightly bounded), create a sibling test file `crates/worldwake-cli/tests/.../player_pov_self_care_occupancy.rs` or equivalent following the same shape.

### 2. (Optional) Positive-case sanity assertion

In the same test file, add a second sub-scenario where the controlled agent IS at the basin's place — the CLI surface SHOULD render the occupancy (via FND-14A). This guards against over-restrictive gating.

## Files to Touch

- The S172 Scenario D test file (path verified at implementation time) — modify
- Possibly: `crates/worldwake-cli/src/...` accessor or handler file if a new CLI accessor needs to be wired up. **Likely**: no new CLI accessor; the existing surface gates correctly by reusing the belief-view path from ticket 006. Verify at implementation time.
- `docs/generated/golden-scenario-index.md` if the scenario name changes or is extended — regenerated.

## Out of Scope

- Scenarios A, B, C — owned by ticket 007.
- Scenario E (deprivation collapse) — owned by ticket 009.
- New CLI rendering of `SelfCareOccupancy` for the co-located case — this ticket asserts the gating, not new render surfaces. If a render-surface ticket is warranted (e.g., the observer binary should display "basin is occupied by Agent X" for the controlled agent's co-located view), it becomes a follow-up.

## Acceptance Criteria

### Tests That Must Pass

1. Extended S172 Scenario D — controlled agent at a place without co-location and without belief sees no remote `SelfCareOccupancy` state in CLI output.
2. (If positive-case sanity is included) Controlled agent co-located with the basin sees the occupancy via FND-14A.
3. Existing S172 Scenario D assertions all pass unchanged.
4. Existing suite: `cargo test -p worldwake-ai --test golden_ai survival_self_care` or the closest test name.

### Invariants

1. The CLI surface for `SelfCareOccupancy` is gated by the same FND-14A/14B source-class rules as the planner-side emitter from ticket 006 — no parallel display path.
2. FND-19 agent symmetry holds: swapping the controlled agent between AI and human control does not change the displayed state.

## Test Plan

### New/Modified Tests

1. The S172 Scenario D test file — extended with `SelfCareOccupancy` gating assertion.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai survival_self_care` (verify at implementation time per `docs/generated/golden-scenario-index.md`)
2. `cargo test -p worldwake-cli` if any CLI-side tests are touched
3. `python3 scripts/golden_inventory.py --write --check-docs` if the inventory needs refreshing
4. `./scripts/verify.sh` before commit.
