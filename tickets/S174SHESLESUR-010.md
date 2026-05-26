# S174SHESLESUR-010: D11 + Scenario D — CLI player-POV symmetry for RestOccupancy

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — CLI player-POV gating for `RestCapacity`/`RestOccupancy` reads (S163-style); scenario file + test file
**Deps**: `archive/tickets/S174SHESLESUR-001.md` (RestCapacity/RestOccupancy components), `archive/tickets/S174SHESLESUR-002.md` (PlaceDef.rest_capacity), `archive/tickets/S174SHESLESUR-003.md` (belief-view accessors — CLI uses the same view), `archive/tickets/S174SHESLESUR-004.md` (RestOccupancy lifecycle), `archive/tickets/S174SHESLESUR-005.md` (sleep schema for candidate emission), `archive/tickets/S174SHESLESUR-006.md` (forensic records — CLI may surface these)

## Problem

S174 D11 requires the CLI to not surface `RestOccupancy` for a place the controlled agent has no lawful observation of. Without this gating, the CLI could leak remote authoritative occupancy state — violating FND-19 (agent symmetry) and the S163 player-POV boundary. Scenario D exercises the gating end-to-end: a controlled agent at place A with no belief about place B's occupancy must see no `RestOccupancy` data for place B in the CLI display.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: S163 (`archive/specs/S163-cli-player-pov-boundary.md`) established the player-POV gating pattern. The CLI binary at `crates/worldwake-cli/src/bin/observer.rs` (and any player-facing display code in `crates/worldwake-cli/src/handlers/`) reads world state via belief-view accessors when in player mode. Per S163, the controlled agent's belief view is the authoritative arbiter of "what does the player see?".
2. Spec assumption verified against S174 D11. The gating pattern: CLI player-mode display calls `is_co_located_with_rest_site(place)` from the archived ticket 003 belief view (`archive/tickets/S174SHESLESUR-003.md`); for non-co-located places, calls `rest_site_occupant_count(place)` which returns `None` when no belief carrier exists. The `None` case must result in CLI displaying "unknown" / "not visible" rather than authoritative state.
3. Shared abstraction boundary under audit: the CLI player-POV / authoritative-state separation. The same belief-view accessors used by candidate generation (`archive/tickets/S174SHESLESUR-005.md`) must be used by the CLI for display gating — no parallel "CLI-only" state-read path. This matches the S163 architecture.
4. Existing inline tests for CLI player-POV: locate via `grep -rn "player_pov\|player_mode\|controlled_agent" crates/worldwake-cli/`. Update existing CLI tests for related accessors (e.g., wash-basin player POV per S172) to mirror the new rest-site gating.
5. Information-path classification: this is an information-path refactor for the new components — `RestCapacity` and `RestOccupancy` are added with two transport paths from the start (authoritative + belief view). The canonical end-state path for player display is belief view; authoritative reads in player mode are an FND-14 violation.
6. ControlSource note: Scenario D switches a controlled agent between Human and AI to verify symmetry (the same gating applies regardless of ControlSource).
7. The CLI may also surface `FailedRestOpportunity` records from archived ticket 006's `CriticalWindowFrame`. For player POV: only surface failed-rest records for the controlled agent (not for other agents) and only after the events are reachable through ordinary perception. This is an extension of S163's discipline; if the CLI does not currently surface `CriticalWindowFrame` data at all, this concern is N/A for this ticket and the failed-rest CLI surface is deferred.

## Architecture Check

1. Reusing the belief-view accessors (instead of introducing CLI-specific gating logic) preserves the S163 single-arbiter pattern. FND-26: systems via state.
2. Asserting symmetry across ControlSource flips (Human → AI → Human) proves the CLI gating is not Human-mode-specific. FND-19 alignment.
3. The CLI must not leak `RestOccupancy.occupants` for remote places even when the controlled agent has a belief about the place's capacity — capacity is public topology (FND-14B substrate carve-out), but occupancy is not.

## Verification Layers

1. Controlled agent at place A: CLI displays `RestCapacity` and `RestOccupancy` for place A correctly (co-located FND-14A read) -> CLI snapshot assertion
2. Controlled agent at place A: CLI does NOT display `RestOccupancy.occupants` for place B when the agent has no belief about B -> CLI snapshot negative assertion
3. Controlled agent has a belief about place B's occupancy (e.g., from prior co-location or witness report): CLI displays the believed occupancy state -> CLI snapshot assertion via belief carrier
4. ControlSource flip (Human → AI → Human): the same gating applies; no asymmetry -> integration test
5. `RestCapacity` is publicly visible (topology) regardless of co-location -> CLI snapshot assertion
6. Deterministic replay -> identical CLI output across two runs

## What to Change

### 1. Locate the CLI player-POV display surface for places

Grep `crates/worldwake-cli/src/bin/observer.rs` and `crates/worldwake-cli/src/handlers/` for existing player-mode place-display code. The S163 precedent likely shows place fields rendered via belief-view accessors. Identify the function that renders a Place's fields and add the rest-site fields alongside.

### 2. Wire `RestCapacity` and `RestOccupancy` through the player-POV display

For each Place rendered:

- Call `belief_view.rest_site_capacity(place)`; render the capacity if `Some(_)`, else skip the rest-site fields (place is not a known rest site, no need to display rest occupancy).
- Call `belief_view.rest_site_occupant_count(place)`; render the count if `Some(_)`, else render "unknown" / "not visible".
- For richer display (showing occupant names rather than just count), use the belief view's actor-level accessors — if `belief_view.is_co_located_with_rest_site(place)`, the CLI may iterate `RestOccupancy.occupants` directly (FND-14A read); otherwise the CLI must use only the belief count and skip individual occupant identification.

The exact rendering format depends on the existing observer/CLI surface — match the precedent established for `SelfCareOccupancy` in S173 (which has its own player-POV gating).

### 3. Author the scenario RON file

Create `scenarios/survival-rest-cli.ron` with:

- Two places: `home_camp` (Aster's spawn place, with `RestCapacity(1)`) and `remote_shelter` (with `RestCapacity(2)`, far enough that Aster has no co-location belief)
- An edge between them
- Aster (controlled), Bram (sleeping at `remote_shelter`)
- Stable seed

### 4. Author the corresponding test file

Create `crates/worldwake-ai/tests/scenarios/survival_rest_cli.rs` (or `crates/worldwake-cli/tests/...` if CLI tests live elsewhere). Assertions per the 6 verification layers above. The CLI output is asserted via the observer's snapshot/render surface.

### 5. Hook the test

Add the new `mod` declaration to the appropriate test module.

## Files to Touch

- Likely: `crates/worldwake-cli/src/bin/observer.rs` (modify — extend place-display surface for rest fields); locate via `grep -n "RestCapacity\|RestOccupancy\|SleepQualityProfile" crates/worldwake-cli/src/bin/observer.rs`
- Likely: `crates/worldwake-cli/src/handlers/` (modify — if place-rendering logic lives in a handler); locate via `grep -rn "place.*display\|render_place" crates/worldwake-cli/src/handlers/`
- `scenarios/survival-rest-cli.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_rest_cli.rs` (new; verify whether CLI tests live in worldwake-ai or worldwake-cli at ticket-implementation time)
- Appropriate `tests/scenarios/mod.rs` (modify — add `mod survival_rest_cli;`)

## Out of Scope

- No engine-side changes (occupancy lifecycle landed in `archive/tickets/S174SHESLESUR-004.md`; emission landed in `archive/tickets/S174SHESLESUR-005.md`)
- No `FailedRestOpportunity` CLI surface — if the CLI doesn't currently display `CriticalWindowFrame` data, this is deferred; the rest-site CLI gating is the headline contract
- No other CLI player-POV gaps — only the rest-site fields are introduced here

## Acceptance Criteria

### Tests That Must Pass

1. New scenario test `survival_rest_cli::scenario_d_player_pov_symmetry` passes all 6 verification-layer assertions
2. ControlSource flip test: Aster as Human → CLI gating; Aster as AI → no CLI rendering but same belief-view enforcement; flip back to Human → identical CLI output
3. Existing suite: `cargo test --workspace` passes (no engine regressions)

### Invariants

1. The CLI never reads authoritative `RestOccupancy.occupants` for remote places — the belief view returns `None` and the CLI displays "unknown"
2. `RestCapacity` is rendered correctly for both co-located and remote places (public topology)
3. ControlSource flips do not change the gating behavior — Human and AI agents see the same world per FND-19

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_rest_cli.rs` (new) — Scenario D E2E
2. Likely: existing tests in `crates/worldwake-cli/src/bin/observer.rs` (extend) — focused coverage for the new place-display fields

### Commands

1. `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_rest_cli` (scenario test)
2. `cargo test -p worldwake-cli` (CLI module tests)
3. `cargo test --workspace`
4. `./scripts/verify.sh`
