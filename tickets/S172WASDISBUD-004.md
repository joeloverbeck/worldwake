# S172WASDISBUD-004: CLI POV assertion against remote Wash basin state leak

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: specs/S172-wash-discovery-budget-closure.md (D6); archive/specs/S158-belief-view-remote-truth-leak-closure.md; archive/specs/S162-belief-view-source-gate-hardening.md; archive/specs/S163-cli-player-pov-boundary.md

## Problem

S158/S162/S163 hardened the belief-view accessors and CLI POV boundary so the human-controlled agent cannot see authoritative remote state through UI paths. S172 D6 requires a scenario-level assertion that pins this guarantee specifically for `WashBasin` state — confirming that the controlled agent at a place without a co-located basin, and without a belief entry for any remote basin, sees nothing about clean-water levels, basin dirtiness, or queue/contention state through CLI accessors. Without this pinning, a future CLI accessor that bypasses the belief-source-class rule for `WashBasinState` could regress without breaking any existing test.

## Assumption Reassessment (2026-05-25)

1. The CLI POV boundary machinery exists (S163 archived). The belief-view leak closure for remote facility state exists (S158, S162 archived). The dual-mode accessor `wash_basin_state` at `crates/worldwake-sim/src/per_agent_belief_view.rs:824` returns the world-authoritative `WashBasinState` only when `has_authoritative_local_visibility(basin)` holds (FND-14A co-located case); otherwise it falls back to `BelievedEntityState::wash_basin_state`. If the belief store has no entry, the accessor returns `Default::default()`.
2. S172 Deliverable 6 (`specs/S172-wash-discovery-budget-closure.md`): "Add a single CLI assertion against an existing scenario (e.g., `survival-drive-escalation`) confirming that with the controlled agent at a place without a co-located `WashBasin` and with no remote-basin belief, the UI does not display clean-water levels, basin dirtiness, or queue state for any remote basin."
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

## Verification Layers

1. CLI accessor return contract for remote `WashBasinState` → focused unit-style assertion against the CLI accessor invoked from a scenario harness; the accessor must return `None` or `Default::default()`, never `Some(authoritative_remote_state)`.
2. Belief-view consistency → confirm `per_agent_belief_view::wash_basin_state(controlled_agent, remote_basin)` returns `Default::default()` (no belief entry; co-location absent).
3. Single-layer ticket — additional layer mapping is not applicable. The action-trace and decision-trace layers are unaffected by this UI-only assertion; the surface is the CLI accessor's return value, which is fully captured by a focused assertion against it.

## What to Change

### 1. Add CLI POV assertion to a Wash-relevant scenario

`crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` (extend an existing test) OR a new test file under `crates/worldwake-cli/tests/` (if a more idiomatic CLI-test surface exists). Default to extending `survival_drive_escalation.rs` since the existing belief-only Wash harness is already there.

Add `#[test] fn cli_does_not_leak_remote_wash_basin_state_for_controlled_agent`:
- Set up the controlled agent (or simulate human control) at a place without a co-located `WashBasin` and without any belief entry for the remote basin in `scenarios/survival-drive-escalation.ron`.
- Place a `WashBasin` at a remote location with non-default `WashBasinState` (clean_water > 0, some dirtiness).
- Invoke the CLI POV accessor that renders facility/basin state for the controlled agent (specific accessor TBD: grep `crates/worldwake-cli/src/` for `wash_basin_state` accessors during reassessment).
- Assert the accessor returns `None`, `Default::default()`, or otherwise does not surface the remote basin's authoritative state.

### 2. Negative-case assertion shape

The test must fail loudly with the remote basin's entity-id and the leaked field (clean_water count, dirtiness level) in the failure message so a regression's symptom is immediately diagnosable.

### 3. CLI accessor discovery

If the exact CLI accessor name is not obvious from grep, the implementer documents the discovery step in the implementation PR: `grep -rn "wash_basin_state\|WashBasinState" crates/worldwake-cli/src/` to find the consumer surface, then assert against the highest-level CLI accessor that surfaces basin state for rendering. Pin the accessor name in the test for future regression reproducibility.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` (modify) OR
- `crates/worldwake-cli/tests/<new-cli-pov-test>.rs` (new) — choose based on which test surface most naturally exercises the CLI accessor; default to the AI test file.
- Likely: a small CLI accessor invocation in the test file. If a new helper is required to bridge scenario state and the CLI accessor, add it to `crates/worldwake-cli/src/` in a new test-support module (not under `tests/` — see Dual-Use Read-Model Types pattern).

## Out of Scope

- Any change to `per_agent_belief_view::wash_basin_state` or any other belief-view accessor — those are S158/S162's deliverables.
- Any change to S163's CLI POV boundary infrastructure — already landed.
- Any extension to cover Sleep, Toilet, or other self-care facility state in UI — Wash is the only self-care facility surface S172 requires; sibling assertions for other facilities belong in their own tickets if warranted.
- Belief-only candidate-path regression — covered by `archive/tickets/S172WASDISBUD-003.md`.
- Test consolidation across scattered/contested/drive-escalation belief-only proofs — defer.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test survival_drive_escalation cli_does_not_leak_remote_wash_basin_state_for_controlled_agent` (or appropriate CLI-crate test path) — new test passes.
2. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::escalation_respects_belief_only_planning -- --ignored --exact` — existing belief-only test still passes (sibling regression check).
3. Existing suite: `cargo test -p worldwake-ai --test survival_drive_escalation` and `cargo test -p worldwake-cli`.

### Invariants

1. The CLI accessor returns no authoritative remote `WashBasin` state for the controlled agent without a belief entry.
2. The accessor's behavior matches S158/S162/S163 architecture — the test pins, does not establish, the contract.
3. No new CLI accessor is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs` — add `cli_does_not_leak_remote_wash_basin_state_for_controlled_agent` (or new CLI-crate test file).

### Commands

1. `cargo test -p worldwake-ai --test survival_drive_escalation` — targeted suite verification (if the test lives in the AI crate).
2. `cargo test -p worldwake-cli` — CLI-crate verification if the test lives there.
3. `cargo clippy --workspace --all-targets -- -D warnings` — lint check.
4. `./scripts/verify.sh` — pre-PR full verification.
