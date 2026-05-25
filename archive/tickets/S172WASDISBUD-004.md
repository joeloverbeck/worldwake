# S172WASDISBUD-004: CLI POV assertion against remote Wash basin state leak

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — test-only POV boundary regression
**Deps**: archive/specs/S172-wash-discovery-budget-closure.md (D6); archive/specs/S158-belief-view-remote-truth-leak-closure.md; archive/specs/S162-belief-view-source-gate-hardening.md; archive/specs/S163-cli-player-pov-boundary.md

## Problem

S158/S162/S163 hardened the belief-view accessors and CLI POV boundary so the human-controlled agent cannot see authoritative remote state through UI paths. S172 D6 requires a scenario-level assertion that pins this guarantee specifically for `WashBasin` state — confirming that the controlled agent at a place without a co-located basin, and without a belief entry for any remote basin, sees nothing about clean-water levels, basin dirtiness, or queue/contention state through CLI accessors. Without this pinning, a future CLI accessor that bypasses the belief-source-class rule for `WashBasinState` could regress without breaking any existing test.

## Assumption Reassessment (2026-05-25)

1. The CLI POV boundary machinery exists (S163 archived). The belief-view leak closure for remote facility state exists (S158, S162 archived). The dual-mode accessor `wash_basin_state` at `crates/worldwake-sim/src/per_agent_belief_view.rs:824` returns the world-authoritative `WashBasinState` only when `has_authoritative_local_visibility(basin)` holds (FND-14A co-located case); otherwise it falls back to `BelievedEntityState::wash_basin_state`. If the belief store has no entry, the accessor returns `Default::default()`.
2. S172 Deliverable 6 (`archive/specs/S172-wash-discovery-budget-closure.md`): "Add a single CLI assertion against an existing scenario (e.g., `survival-drive-escalation`) confirming that with the controlled agent at a place without a co-located `WashBasin` and with no remote-basin belief, the UI does not display clean-water levels, basin dirtiness, or queue state for any remote basin."
3. Shared abstraction boundary: the CLI POV accessor pipeline that surfaces basin state for human-facing rendering. The contract is "no remote `WashBasinState` is rendered for the controlled agent without a belief entry." The boundary lives in the CLI crate's belief-view consumer surface, not in the simulation crates.
4. Intended invariant: CLI accessor pipelines respect FND-19 agent symmetry and FND-27 derived-summaries-are-caches — no UI accessor may surface authoritative remote `WashBasin` state.
5. Live `GoalKind` under test: not directly under test — this is a UI-layer assertion. The candidate path is exercised, but the surface tested is the CLI POV accessor return.
6. AI regression layer: not directly applicable — this ticket asserts a UI-rendering contract, not a planner invariant.
12. Scenario isolation: the test must place the controlled agent at a location without a co-located `WashBasin` AND ensure the agent has no belief seeded for any remote basin. Any belief-population path (perception of the basin's place, witness report, scenario-authored belief seed for the basin) is intentionally excluded.
13. Adjacent contradictions: if the CLI accessor returns non-default `WashBasinState` for an unseen remote basin, that is a CRITICAL regression in S158/S162/S163 architecture — open a separate ticket; do not patch the symptom in this ticket.

## Architecture Check

1. Scenario-level CLI assertion is cheaper and more representative than a unit test against the accessor in isolation — the test exercises the full pipeline (belief-view → CLI accessor → render decision).
2. No backwards-compatibility aliasing: the test consumes existing CLI accessors as-is; it does not introduce a new accessor.
3. FND-19 + FND-27 alignment: the test pins the POV-boundary contract for the Wash domain, complementing S158/S162/S163's broader leak-closure work.

## Verified Layers

1. CLI accessor return contract for remote `WashBasinState` → focused unit-style assertion against the CLI accessor invoked from a scenario harness; the accessor must return `None` or `Default::default()`, never `Some(authoritative_remote_state)`.
2. Belief-view consistency → confirm `per_agent_belief_view::wash_basin_state(controlled_agent, remote_basin)` returns `Default::default()` (no belief entry; co-location absent).
3. Single-layer ticket — additional layer mapping is not applicable. The action-trace and decision-trace layers are unaffected by this UI-only assertion; the surface is the CLI accessor's return value, which is fully captured by a focused assertion against it.

## Landed Changes

### 1. Added CLI POV assertion to a Wash-relevant scenario

`crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` now extends the existing belief-only Wash harness and adds `cli_does_not_leak_remote_wash_basin_state_for_controlled_agent`.

- The controlled-agent fixture remains at `ORCHARD_FARM` with only local beliefs seeded.
- A remote `WashBasin` at `VILLAGE_SQUARE` carries non-default `WashBasinState` plus remote contention queue/grant state.
- The test invokes the live POV boundary exposed to CLI consumers: `PerAgentBeliefView` through `GoalBeliefView::wash_basin_state`, `FacilityBeliefView::wash_basin_state`, and `TemporalBeliefView` queue/grant accessors.
- Assertions fail with the remote basin id and leaked field surface if authoritative remote state becomes visible.

### 2. Negative-case assertion shape

The added test fails loudly with the remote basin's entity id and the leaked surface: `GoalBeliefView::wash_basin_state`, `FacilityBeliefView::wash_basin_state`, queue position, or contention grant.

### 3. CLI accessor discovery

Live reassessment found no dedicated WashBasin renderer/accessor in `crates/worldwake-cli/src/`. The highest current CLI-facing POV boundary is the belief-view surface exported by `worldwake-sim` and consumed by the CLI crate, so the regression pins that boundary directly instead of introducing a new accessor.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` — modified the belief-only Wash harness and added the focused POV leak assertion.

## Out of Scope

- Any change to `per_agent_belief_view::wash_basin_state` or any other belief-view accessor — those are S158/S162's deliverables.
- Any change to S163's CLI POV boundary infrastructure — already landed.
- Any extension to cover Sleep, Toilet, or other self-care facility state in UI — Wash is the only self-care facility surface S172 requires; sibling assertions for other facilities belong in their own tickets if warranted.
- Belief-only candidate-path regression — covered by `archive/tickets/S172WASDISBUD-003.md`.
- Test consolidation across scattered/contested/drive-escalation belief-only proofs — defer.

## Acceptance Result

### Tests Passed Or Waived

1. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent -- --exact`.
2. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::escalation_respects_belief_only_planning -- --ignored --exact`.
3. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation`.
4. Passed `cargo test -p worldwake-cli`.
5. Passed `cargo clippy --workspace --all-targets -- -D warnings`.
6. Waived `./scripts/verify.sh` for per-ticket closeout because this ticket runs inside `implement-spec-tickets`; the harness final branch phase owns the full pre-PR gate before push.

### Invariants

1. The CLI accessor returns no authoritative remote `WashBasin` state for the controlled agent without a belief entry.
2. The accessor's behavior matches S158/S162/S163 architecture — the test pins, does not establish, the contract.
3. No new CLI accessor is introduced.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` — added `cli_does_not_leak_remote_wash_basin_state_for_controlled_agent`.

## Outcome

Completed on 2026-05-25.

- Added the S172 Deliverable 6 player-POV regression for remote Wash basin state.
- Reused the existing survival drive-escalation belief-only Wash harness instead of creating a separate CLI test file.
- Seeded non-default authoritative remote `WashBasinState` and remote contention state, then proved the controlled agent's POV belief-view surfaces hide that state without co-location or belief.
- No production behavior or CLI renderer was changed.

## Deviations

- The draft expected a dedicated CLI WashBasin rendering accessor if one existed. Live reassessment found no such accessor, so the landed test pins the exported `PerAgentBeliefView` POV boundary that CLI consumers already use.
- The drafted `cargo test -p worldwake-ai --test survival_drive_escalation ...` command was not a live integration-test binary. The truthful target is `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::...`.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::cli_does_not_leak_remote_wash_basin_state_for_controlled_agent -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::escalation_respects_belief_only_planning -- --ignored --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Waived `./scripts/verify.sh` for per-ticket closeout because `implement-spec-tickets` owns the final pre-push full verification gate.
