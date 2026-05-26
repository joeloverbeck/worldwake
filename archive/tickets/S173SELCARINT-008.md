# S173SELCARINT-008: Scenario D — Player POV symmetry for occupancy

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (CLI/scenario assertion only)
**Deps**: `archive/tickets/S173SELCARINT-001.md` (uses `SelfCareOccupancy`), `archive/tickets/S173SELCARINT-004.md` (occupancy lifecycle in wash/toilet), `archive/tickets/S173SELCARINT-006.md` (emitter-time read pattern), `specs/S173-self-care-interruption-occupancy.md` (D9, Scenario D)

## Problem

The CLI player-POV boundary (`archive/specs/S163-cli-player-pov-boundary.md`, archived) enforces that the controlled agent's display surface only shows what that agent could lawfully perceive, infer, or recall. With `SelfCareOccupancy` introduced as a new authoritative facility-side component, the CLI respects the same gating — a controlled agent at a place without a co-located basin and without belief about a remote basin must not see remote `SelfCareOccupancy` state in any CLI accessor output (FND-19 agent symmetry, FND-14 world-state vs belief-state separation). This ticket added the assertion as an extension of S172's Scenario D location, completing the player-POV gating contract for self-care occupancy.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. S163 (CLI POV boundary) is archived at `archive/specs/S163-cli-player-pov-boundary.md`. S172 (Wash discovery and budget closure) is archived at `archive/specs/S172-wash-discovery-budget-closure.md` and contains a Scenario D for the player-POV pattern. The exact S172 Scenario D test is `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent`; this ticket extended that scenario rather than creating a fresh scenario file.
2. CLI accessor surface: live reassessment found no direct `SelfCareOccupancy` renderer in `crates/worldwake-cli/src`. The assertion therefore verifies the **absence** of a player-POV leak through the shared `PerAgentBeliefView` surface consumed by CLI/player-POV code, not the addition of a new render path.
3. Shared abstraction boundary: the player-POV display path. Per FND-19, swapping a controlled agent from AI to human or human to AI must not change the legal action set or the visible information set. The CLI surface for `SelfCareOccupancy` must obey the same FND-14A/14B source-class split as the planner-side emitter from ticket 006.
4. Authoritative-to-AI Impact Rule does not apply directly — this is a CLI-only ticket, not a planner-modifying one. The Auth-to-AI checklist coverage lives in tickets 004, 005, 006.
5. Goal infrastructure not affected — no candidate emission, no plan composition.

## Architecture Check

1. The CLI accessor's gating reuses the existing player-POV machinery from S163 — no new mechanism. The assertion proves that the existing machinery covers the new component type by virtue of the `SelfCareOccupancy` accessor going through the same belief-view-or-co-located-observation surface as `WashBasinState`.
2. Per FND-19 agent symmetry, the same gating that filters remote `WashBasinState` reads filters remote `SelfCareOccupancy` reads — both are facility-side authoritative state subject to FND-14B.
3. The assertion is a negative-case test (no remote `SelfCareOccupancy` displayed) per `docs/precision-rules.md` Rule 8 scenario isolation. The lawful competing affordance (a controlled agent at the basin's place WOULD see the occupancy via FND-14A) is documented as intentionally excluded from the negative-case scenario.

## Verified Layers

1. CLI accessor output filtering → focused assertion in `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent`: the controlled agent stays at a place without a co-located basin and without belief of the remote basin; the remote basin has authoritative `SelfCareOccupancy`; `FacilityBeliefView::self_care_occupant` returns `None` for the controlled agent.
2. Positive case sanity check → the same focused assertion verifies that the co-located occupant can see the basin occupancy through the FND-14A path. This guards against over-restrictive gating.
3. Single-layer ticket (CLI gating only). No engine-layer changes; the existing wash/toilet contract from ticket 004 is exercised as setup, not asserted upon.

## Landed Changes

### 1. Extended S172's Scenario D with `SelfCareOccupancy` gating assertion

The S172 Scenario D test surface was `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent`, which exercises the `PerAgentBeliefView` surface consumed by CLI/player-POV code. The existing `build_belief_only_wash_harness()` setup now includes:

- A remote basin at a remote place, with authoritative `SelfCareOccupancy { occupant: remote_holder, use_kind: Wash, ... }` pre-written.
- Assertion: the controlled agent's player-POV belief view shows no remote self-care occupant for that basin.

No new CLI accessor or sibling test file was added because the existing S172 Scenario D file naturally owned the assertion.

### 2. Added positive-case sanity assertion

The same focused test also creates a co-located view for the authoritative occupant and asserts `FacilityBeliefView::self_care_occupant` returns that occupant, proving the gating does not hide lawful same-tick local observation.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` — modified the existing Scenario D proof.
- `specs/S173-self-care-interruption-occupancy.md` — truth-synced Scenario D/spec-family status.
- No `crates/worldwake-cli/src/...` change was required; the player-POV surface already routes through `PerAgentBeliefView`.
- No generated golden docs changed because no scenario metadata or test name changed.

## Out of Scope

- Scenarios A, B, C — owned by ticket 007.
- Scenario E (deprivation collapse) — owned by ticket 009.
- New CLI rendering of `SelfCareOccupancy` for the co-located case — this ticket asserts the gating, not new render surfaces. A future render-surface ticket can add display text if the observer/player UI should show "basin is occupied by Agent X" for the controlled agent's co-located view.

## Acceptance Result

### Tests Passed

1. Extended S172 Scenario D — controlled agent at a place without co-location and without belief sees no remote `SelfCareOccupancy` occupant through the player-POV belief view.
2. Positive-case sanity — a co-located agent sees the authoritative self-care occupant via FND-14A.
3. Existing S172 Scenario D assertions still pass unchanged.
4. Existing suite substitute: `cargo test -p worldwake-ai --test golden_ai survival_drive_escalation` is the closest owning test-family command for this scenario. The `survival_self_care` family owns Scenarios A-C, not Scenario D.

### Invariants

1. The CLI surface for `SelfCareOccupancy` is gated by the same FND-14A/14B source-class rules as the planner-side emitter from ticket 006 — no parallel display path.
2. FND-19 agent symmetry holds: swapping the controlled agent between AI and human control does not change the displayed state.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent` — extended with remote no-leak and co-located visible `SelfCareOccupancy` assertions.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent -- --exact`
2. `cargo test -p worldwake-ai --test golden_ai survival_drive_escalation`
3. `cargo fmt --all`

## Outcome

Completed on 2026-05-26.

- Added authoritative `SelfCareOccupancy` to the existing remote-basin Scenario D harness.
- Proved the controlled agent's player-POV belief view does not leak the remote basin's occupant without co-location or belief.
- Proved the co-located occupant can see the occupancy through the FND-14A path, so the gate is not over-restrictive.
- No CLI source change, generated-doc change, or new scenario file was required.

## Deviations

- The drafted `survival_self_care` command was replaced with the live owning `survival_drive_escalation` filter because Scenario D lives in `survival_drive_escalation.rs`; `survival_self_care` owns Scenarios A-C.
- `./scripts/verify.sh` is waived at per-ticket closeout because this ticket is running inside `$implement-spec-tickets`; the harness final branch phase owns the pre-push full verification gate.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent -- --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai survival_drive_escalation` (1 passed, 3 existing CI-only ignored scenarios).
- Passed `cargo fmt --all`.
- Waived `./scripts/verify.sh` for per-ticket closeout because `$implement-spec-tickets` owns the final pre-push wrapper gate.
