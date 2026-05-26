# S174SHESLESUR-010: D11 + Scenario D — CLI player-POV symmetry for RestOccupancy

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — CLI player-POV gating for `RestCapacity`/`RestOccupancy` reads (S163-style); scenario file + test file
**Deps**: `archive/tickets/S174SHESLESUR-001.md` (RestCapacity/RestOccupancy components), `archive/tickets/S174SHESLESUR-002.md` (PlaceDef.rest_capacity), `archive/tickets/S174SHESLESUR-003.md` (belief-view accessors — CLI uses the same view), `archive/tickets/S174SHESLESUR-004.md` (RestOccupancy lifecycle), `archive/tickets/S174SHESLESUR-005.md` (sleep schema for candidate emission), `archive/tickets/S174SHESLESUR-006.md` (forensic records — CLI may surface these)

## Outcome

Scenario D landed as `scenarios/survival-rest-cli.ron` plus focused `worldwake-cli` coverage in `crates/worldwake-cli/src/handlers/inspect.rs`. The player-facing `look` path now renders current-place rest-site capacity/occupancy through a `FacilityBeliefView`-backed formatter. The formatter reports local occupancy when the controlled agent is co-located with the rest site, reports remote occupancy as unknown when no belief carrier exists, reports believed remote occupancy when a belief carrier exists, and is invariant across Human/Ai `ControlSource` flips.

The ticket scope was corrected during reassessment: S163 already marks `inspect`, `world`, `events`, broad trace rendering, and similar commands as debug/observer surfaces. The normal player-facing place surface is the controlled agent's current-place `look` output; no new remote-place display surface was introduced.

## Problem Resolved

S174 D11 requires the CLI to not surface `RestOccupancy` for a place the controlled agent has no lawful observation of. Without this gating, the CLI could leak remote authoritative occupancy state — violating FND-19 (agent symmetry) and the S163 player-POV boundary. Scenario D exercises the gating end-to-end: a controlled agent at place A with no belief about place B's occupancy must see no `RestOccupancy` data for place B in the CLI display.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: S163 (`archive/specs/S163-cli-player-pov-boundary.md`) established the player-POV gating pattern. The CLI binary at `crates/worldwake-cli/src/bin/observer.rs` (and any player-facing display code in `crates/worldwake-cli/src/handlers/`) reads world state via belief-view accessors when in player mode. Per S163, the controlled agent's belief view is the authoritative arbiter of "what does the player see?".
2. Spec assumption verified against S174 D11. The gating pattern: CLI player-mode display calls `is_co_located_with_rest_site(place)` from the archived ticket 003 belief view (`archive/tickets/S174SHESLESUR-003.md`); for non-co-located places, calls `rest_site_occupant_count(place)` which returns `None` when no belief carrier exists. The `None` case must result in CLI displaying "unknown" / "not visible" rather than authoritative state.
3. Shared abstraction boundary under audit: the CLI player-POV / authoritative-state separation. The same belief-view accessors used by candidate generation (`archive/tickets/S174SHESLESUR-005.md`) must be used by the CLI for display gating — no parallel "CLI-only" state-read path. This matches the S163 architecture.
4. Existing inline tests for CLI player-POV: locate via `grep -rn "player_pov\|player_mode\|controlled_agent" crates/worldwake-cli/`. Update existing CLI tests for related accessors (e.g., wash-basin player POV per S172) to mirror the new rest-site gating.
5. Information-path classification: this is an information-path refactor for the new components — `RestCapacity` and `RestOccupancy` are added with two transport paths from the start (authoritative + belief view). The canonical end-state path for player display is belief view; authoritative reads in player mode are an FND-14 violation.
6. ControlSource note: Scenario D switches a controlled agent between Human and AI to verify symmetry (the same gating applies regardless of ControlSource).
7. The CLI may also surface `FailedRestOpportunity` records from archived ticket 006's `CriticalWindowFrame`. For player POV: only surface failed-rest records for the controlled agent (not for other agents) and only after the events are reachable through ordinary perception. This is an extension of S163's discipline; if the CLI does not currently surface `CriticalWindowFrame` data at all, this concern is N/A for this ticket and the failed-rest CLI surface is deferred.
8. Scope correction: live S163 already marks `inspect`, `world`, `events`, broad observer traces, and similar commands as debug/observer surfaces. The player-facing place surface is `handle_look` for the controlled agent's current place; there is no existing normal-player remote-place renderer to extend. This ticket therefore adds a POV-safe rest-site formatter backed by `FacilityBeliefView`, uses it for current-place `look`, and tests remote unknown/believed occupancy through that formatter rather than introducing a new remote display surface.
9. Test placement correction: the CLI output/formatter boundary belongs in `worldwake-cli` focused tests. A separate `worldwake-ai` golden is not the correct owner unless the scenario needs AI decision behavior, which this player-POV display ticket does not.

## Architecture Result

1. Reusing the belief-view accessors (instead of introducing CLI-specific gating logic) preserves the S163 single-arbiter pattern. FND-26: systems via state.
2. Asserting symmetry across ControlSource flips (Human → AI → Human) proves the CLI gating is not Human-mode-specific. FND-19 alignment.
3. The CLI must not leak `RestOccupancy.occupants` for remote places even when the controlled agent has a belief about the place's capacity — capacity is public topology (FND-14B substrate carve-out), but occupancy is not.

## Layer Proofs

1. Controlled agent at `Home Camp`: `format_player_rest_site_status` reports `0/1 occupied` through `FacilityBeliefView` -> focused CLI formatter assertion
2. Controlled agent at `Home Camp`: remote `Remote Shelter` has authoritative `RestOccupancy { Bram }`, but the same formatter reports `capacity 2, occupancy unknown` without a belief carrier -> focused CLI negative assertion
3. Controlled agent has a seeded belief that `Remote Shelter` is occupied: formatter reports `1/2 occupied` -> focused CLI belief-carrier assertion
4. ControlSource flip Human -> Ai -> Human leaves the formatter output unchanged -> focused CLI symmetry assertion
5. `handle_look` uses the same formatter for the controlled agent's current place -> implementation path in `crates/worldwake-cli/src/handlers/inspect.rs`
6. Scenario authoring remains valid -> `cargo test -p worldwake-cli` scenario lint sweep and load-based tests

## Landed Changes

1. Added `PlayerRestSiteStatus` and a `FacilityBeliefView`-backed formatter in `crates/worldwake-cli/src/handlers/inspect.rs`.
2. Wired `handle_look` to print the current place's rest-site line through `PerAgentBeliefView::from_world_at_tick`.
3. Added `scenarios/survival-rest-cli.ron` with local and remote rest sites plus controlled Aster and remote Bram.
4. Added focused CLI tests for local occupancy, remote unknown occupancy despite authoritative remote `RestOccupancy`, remote believed occupancy, and ControlSource symmetry.

## Landed Files

- `crates/worldwake-cli/src/handlers/inspect.rs`
- `scenarios/survival-rest-cli.ron`

## Out of Scope

- No engine-side changes (occupancy lifecycle landed in `archive/tickets/S174SHESLESUR-004.md`; emission landed in `archive/tickets/S174SHESLESUR-005.md`)
- No `FailedRestOpportunity` CLI surface — if the CLI doesn't currently display `CriticalWindowFrame` data, this is deferred; the rest-site CLI gating is the headline contract
- No other CLI player-POV gaps — only the rest-site fields are introduced here

## Acceptance Criteria

### Passed Checks

1. CLI rest-site POV tests passed all verification-layer assertions.
2. ControlSource flip test passed: Aster as Human -> formatter output; Aster as AI -> same belief-view enforcement; back to Human -> identical formatter output.
3. Existing suite: `cargo test --workspace` passed.

### Invariants

1. The CLI never reads authoritative `RestOccupancy.occupants` for remote places — the belief view returns `None` and the CLI displays "unknown"
2. `RestCapacity` is rendered correctly for both co-located and remote places (public topology)
3. ControlSource flips do not change the gating behavior — Human and AI agents see the same world per FND-19

## Tests

1. `handlers::inspect::tests::test_rest_site_status_uses_pov_belief_view_for_remote_occupancy` — Scenario D local, remote-unknown, and remote-believed occupancy coverage.
2. `handlers::inspect::tests::test_rest_site_status_is_control_source_symmetric` — ControlSource symmetry coverage.

## Verification Result

1. Passed `cargo test -p worldwake-cli rest_site_status`
2. Passed `cargo test -p worldwake-cli`
3. Passed `cargo test --workspace`
4. Passed `cargo clippy --workspace`
5. Passed `cargo clippy --workspace --all-targets -- -D warnings`
6. Passed `cargo fmt --all -- --check`
7. Passed `git diff --check`
8. Waived the verify wrapper because its required sub-gates were run directly above.
