# PERAGRSTY-003: Golden test proving reasoning style diversity

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: PERAGRSTY-002

## Problem

After PERAGRSTY-001 and -002, agents can have different `ReasoningProfile` values, but no test proves that these differences produce observable behavioral divergence. Without a golden test, a future regression could silently ignore per-agent profiles and nobody would notice. This ticket adds at least one golden E2E test proving that two agents with different reasoning profiles make different decisions from identical starting conditions.

## Assumption Reassessment (2026-04-03)

1. After PERAGRSTY-002, `ReasoningProfile` is resolved per-agent from the world's component tables. Agents without an explicit profile get `ReasoningProfile::default()`.
2. Golden tests live in `crates/worldwake-ai/tests/` as `golden_*.rs` files and should follow `docs/golden-e2e-testing.md`, including scenario metadata blocks, decision-trace-first assertions for AI reasoning, and deterministic replay companions.
3. `PerceptionProfile` is required on agents that need to observe post-production output (per CLAUDE.md golden test note). Any agent in the golden test that must perceive the world needs this profile.
4. The live goal-switch path for active actions is `handle_active_action_phase()` in `crates/worldwake-ai/src/agent_tick/active_action.rs`, which resolves `goal_switch_margin_details()` and feeds `evaluate_interrupt()` in `crates/worldwake-ai/src/interrupts.rs`. `ReasoningProfile.switch_margin` matters only when there is no active `IntentionDispositionProfile` overriding the frame margin and only for same-priority challengers; higher-priority interrupts bypass the margin.
5. Existing focused tests already cover the switch-margin mechanism at lower layers (`goal_switching.rs`, `interrupts.rs`, and `agent_tick/tests.rs`), but the current golden harness does not expose a stable same-class switch-margin E2E boundary without adding extra scaffolding that changes the scenario itself.
6. `search_plan()` consumes `ReasoningProfile.max_node_expansions` directly. Existing focused search tests already prove low expansion counts can lose plans that larger budgets find, so the golden should pick a live multi-step scenario and prove divergence through decision traces rather than relying on the spec's illustrative numbers.
7. Deterministic replay requires `ChaCha8Rng` seeding. Golden tests use deterministic seeds.
8. Not a stale-request/contested-affordance/political/ControlSource/heuristic-removal ticket — domain-specific precision items 8-15 are N/A.
9. To keep the "only the profile changed" invariant literal and avoid accidental competition discounts or observation cross-talk, the strongest golden shape is two isolated harness runs with identical setup and seeds, differing only in `ReasoningProfile`.

## Architecture Check

1. Golden tests are the established E2E proof surface for agent behavioral contracts in this project. A reasoning-diversity golden test fits naturally alongside existing golden tests for production, combat, trade, etc.
2. No backward-compatibility shims. This is a pure test addition.

## Verification Layers

1. Tight-search run (`max_node_expansions: 2`) fails to select the target multi-step plan at the planning boundary
2. Default-search run finds and selects the target multi-step plan -> decision trace exposes the expected searched plan shape
3. Deterministic replay companion produces identical world/event-log hashes for the scenario

## What to Change

### 1. Add golden test file

Create `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`:

**Scenario 1 — Search depth divergence:**
- Build one isolated harness scenario with a live multi-step planning target, then run it twice with identical setup and seed except for `ReasoningProfile.max_node_expansions`.
- Tight-search run: `max_node_expansions: 2`, matching the live divergence threshold already demonstrated by focused planner tests.
- Default-search run: `ReasoningProfile::default()`.
- Assert via tick-0 planning traces that the default-search run selects the target plan, while the tight-search run does not select that plan under otherwise identical conditions.
- Prefer explicit plan-shape assertions (`selected_plan_source`, key `PlannerOpKind`s, expected target places) over weaker downstream "event eventually happened" checks.

Add a deterministic replay companion test for the scenario, following existing `golden_*_replays_deterministically` patterns.

### 2. Register test in workspace

Ensure the new test file is picked up by `cargo test -p worldwake-ai`. No `Cargo.toml` changes needed — files in `tests/` are auto-discovered.

## Files to Touch

- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` (new)

## Out of Scope

- Testing all 12 `ReasoningProfile` fields individually (this proves the mechanism works; exhaustive field coverage is future work)
- Adding non-default profiles to agents in the CLI or scenario files (separate spec scope)
- Modifying `IntentionDispositionProfile` interaction (unchanged, already tested by existing goal-switching tests)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_reasoning_diversity::search_depth_divergence` — tight search budget fails to land the remote craft plan while default reasoning succeeds
2. `golden_reasoning_diversity::search_depth_divergence_replays_deterministically`
3. Existing suite: `cargo test --workspace`

### Invariants

1. Only `ReasoningProfile.max_node_expansions` differs between the paired runs — all other setup is identical
2. The scenario is deterministic and replay-safe (seeded RNG, `BTreeMap` state)
3. `PerceptionProfile` is attached to the acting agent because the scenario depends on observing remote recipe input state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` — golden scenario proving per-agent search depth produces observable behavioral divergence

### Commands

1. `cargo test -p worldwake-ai golden_reasoning_diversity`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completed: 2026-04-03
- Added `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` with a stable golden E2E proof that `ReasoningProfile.max_node_expansions` changes AI behavior under otherwise identical seeded setup.
- Landed paired coverage:
  - `search_depth_divergence`
  - `search_depth_divergence_replays_deterministically`
- Refreshed generated golden inventory/docs so the new `// Scenario 97:` metadata is tracked in the canonical inventory and scenario map.
- Deviation from original plan: the ticket was reassessed away from a broader two-divergence concept. The search-depth proof was retained as the honest golden E2E slice, while switch-margin remains covered at focused lower layers because the current golden harness does not expose a stable same-class switch-margin boundary without scenario-distorting scaffolding.
- Verification:
  - `cargo test -p worldwake-ai --test golden_reasoning_diversity -- --nocapture`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
