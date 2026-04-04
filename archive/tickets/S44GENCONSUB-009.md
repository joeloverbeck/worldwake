# S44GENCONSUB-009: Golden tests + SAVE_FORMAT_VERSION

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — golden test scenarios, save format version bump
**Deps**: S44GENCONSUB-006, S44GENCONSUB-007, S44GENCONSUB-008, S44GENCONSUB-010

## Problem

The contention substrate needs end-to-end proof that multi-agent contention resolves through inspectable world state, not invisible tick order. FOUNDATIONS Canonical Scenario E requires "any resulting line, grant, blocker, or reservation is inspectable world state rather than invisible runtime magic." The golden closeout must now cover both queue-based domains and the newly live race-mode unique-item pickup path across action validation, contention system, perception, and AI replanning.

## Assumption Reassessment (2026-04-04)

1. Golden tests live in `crates/worldwake-ai/tests/` as `golden_*.rs` files and generated inventory/docs must be refreshed after adding new `// Scenario` blocks. Confirmed against `docs/golden-e2e-testing.md` and `scripts/golden_inventory.py`.
2. `SAVE_FORMAT_VERSION` at `crates/worldwake-sim/src/save_load.rs:6` is still `15`. It must be bumped because the generalized contention chain added persisted world-state and belief-state shape, including `ContentionQueue`, `ContentionPolicy`, `ContentionIntents`, `ContentionDispositionProfile`, and `BelievedContentionState`.
3. Golden tests that need agents to observe contention aftermath still require explicit `PerceptionProfile` setup. This is especially relevant for the corpse-contention scenario because the queue/grant state must become a local belief, not only world state.
4. The ticket's proposed new-file layout is stale. Live contention golden ownership is stronger in existing suites:
   - corpse / loot contention belongs in `crates/worldwake-ai/tests/golden_combat.rs`
   - facility queue prune / promotion belongs in `crates/worldwake-ai/tests/golden_production.rs`
   - save/load round-trip belongs in `crates/worldwake-ai/tests/golden_determinism.rs`
   Creating three new tiny golden files would duplicate existing suite boundaries.
5. Existing golden coverage already proves facility queue turns, dead-waiter prune, patience timeout, and grant reuse in `golden_production.rs`. The remaining facility-side gap is the departure-prune branch specifically, not generic queue pruning.
6. Existing lower-layer tests already prove unique-item race-mode rejection in `transport_actions.rs`, but no current golden proves the full path from contention rejection to AI redirect. That remains a real golden gap.
7. Existing save/load goldens already prove `ContentionIntents` survive round-trip, but they do not yet prove the newer generalized contention queue/grant/perception state survives round-trip.
8. The live missing golden contracts are:
   - corpse queue/grant contention with visible belief-state projection
   - departed facility waiter pruned and next waiter promoted
   - unique-item race-mode rejection followed by lawful AI redirect
   - save/load round-trip over generalized contention world/belief state

## Architecture Check

1. Golden tests are the canonical Scenario E acceptance test — they prove the contention substrate produces the required chains from generic systems, not authored sequences.
2. SAVE_FORMAT_VERSION bump is mandatory because new component types change the serialization format.
3. No backward-compatibility shims.

## Verification Layers

1. Scenario A: grant assignment → authoritative world state (ContentionGrant on entity)
2. Scenario A: queue state visible to co-located agent → belief state (BelievedContentionState)
3. Scenario A: second agent promoted after first completes → authoritative world state
4. Scenario B: departed agent pruned → authoritative world state (queue no longer contains agent)
5. Scenario C: full queue rejection → action trace (StartFailed with contention_rejected)
6. Scenario C: rejected agent follows a lawful alternative local branch → authoritative aftermath plus action trace
7. Cross-layer: these are full-stack E2E tests covering core → sim → systems → ai → cli.

## What to Change

### 1. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`: increment `SAVE_FORMAT_VERSION` from current value.

### 2. Golden Scenario A: Corpse loot contention

Add a new scenario block to `crates/worldwake-ai/tests/golden_combat.rs`:
- Setup: two agents with PerceptionProfile at same place, one corpse (dead agent with items)
- Both agents have needs driving loot motivation
- Tick forward: first agent gets grant, starts looting. Second agent begins as an explicit queued waiter on the same corpse.
- After first completes: second promoted, loots remaining items
- Assertions: ContentionGrant visible in world state, BelievedContentionState in observer beliefs, both agents eventually loot

### 3. Golden Scenario B: Contention with departure

Add a new scenario block to `crates/worldwake-ai/tests/golden_production.rs`:
- Setup: agent queued for a facility with ContentionQueue, then given travel intent
- Tick forward: agent departs place
- Contention system prunes departed agent
- Next waiter promoted
- Assertions: departed agent no longer in queue, next agent holds grant

### 4. Golden Scenario C: Full queue rejection

Add a new scenario block to `crates/worldwake-ai/tests/golden_production.rs`:
- Setup: ground unowned `UniqueItem` race domain with multiple co-located agents and an alternate local branch for the loser
- First agent claims the unique item grant through `pick_up`
- Second agent receives `contention_rejected` at authoritative start and redirects lawfully
- Assertions: structured rejection trace for the loser, no double-pickup, loser follows an alternative local branch instead of acting on the claimed item

### 5. Save/load round-trip

Extend `crates/worldwake-ai/tests/golden_determinism.rs` with a generalized-contention round-trip scenario:
- Setup: live contention-managed entity with queue/grant state and an observing agent carrying `BelievedContentionState`
- Save/load mid-scenario
- Assertions: contention components and belief-state survive round-trip, resumed run stays deterministic

### 6. Deterministic replay companions

Each scenario includes a replay companion that re-runs with the same seed and verifies identical outcome.

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (modify — bump version)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify — corpse contention golden)
- `crates/worldwake-ai/tests/golden_production.rs` (modify — departure prune and unique-item rejection goldens)
- `crates/worldwake-ai/tests/golden_determinism.rs` (modify — generalized contention save/load round-trip)
- `docs/generated/golden-coverage-matrix.md` (generated)
- `docs/generated/golden-e2e-inventory.md` (generated)
- `docs/generated/golden-scenario-map.md` (generated)

## Out of Scope

- Phase 2 contention domains (bounty claims, storage, witness time)
- Performance optimization of contention system
- AI heuristics for queue avoidance (future refinement)

## Acceptance Criteria

### Tests That Must Pass

1. Golden Scenario A: two-agent corpse loot resolves through visible queue/grant state
2. Golden Scenario B: departed agent pruned from queue, next waiter promoted
3. Golden Scenario C: unique-item race rejection produces structured `contention_rejected`, winner retains sole claim, and loser follows a lawful alternative local branch
4. All scenarios produce identical results on deterministic replay
5. Save/load round-trip preserves all contention components
6. Existing suite: `cargo test --workspace`

### Invariants

1. Contention state is inspectable world state (Canonical Scenario E)
2. No agent acts on a contention-managed entity without holding the grant
3. Dead/departed agents never block queue progress
4. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs` — Scenario A
2. `crates/worldwake-ai/tests/golden_production.rs` — Scenarios B and C
3. `crates/worldwake-ai/tests/golden_determinism.rs` — save/load contention round-trip
4. `python3 scripts/golden_inventory.py --write --check-docs` — generated docs refresh

### Commands

1. `cargo test -p worldwake-ai --test golden_combat`
2. `cargo test -p worldwake-ai --test golden_production`
3. `cargo test -p worldwake-ai --test golden_determinism`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo test -p worldwake-sim save_load`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`

## Outcome

Completed: 2026-04-04

What changed:
- Added Scenario 101 `golden_corpse_contention_projects_visible_queue_and_grant_state` in `crates/worldwake-ai/tests/golden_combat.rs`.
- Added Scenario 102 `golden_departed_waiter_pruned_from_facility_queue` and Scenario 103 `golden_unique_item_race_rejection_redirects_to_local_alternative` in `crates/worldwake-ai/tests/golden_production.rs`.
- Added Scenario 104 `golden_save_load_preserves_generalized_contention_state` in `crates/worldwake-ai/tests/golden_determinism.rs`.
- Bumped `SAVE_FORMAT_VERSION` from `15` to `16` in `crates/worldwake-sim/src/save_load.rs`.
- Refreshed `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-scenario-map.md`.

Deviations from original plan:
- The golden work landed in existing owning suites rather than new `golden_*` files.
- Scenario A proved the honest live boundary with seeded corpse contention state plus real promotion and belief projection.
- Scenario C proved authoritative `contention_rejected` plus lawful alternative-path aftermath; it did not pin a planner-specific redirect trace.

Verification results:
- Passed `cargo test -p worldwake-ai --test golden_combat golden_corpse_contention_projects_visible_queue_and_grant_state`
- Passed `cargo test -p worldwake-ai --test golden_production golden_departed_waiter_pruned_from_facility_queue`
- Passed `cargo test -p worldwake-ai --test golden_production golden_unique_item_race_rejection_redirects_to_local_alternative`
- Passed `cargo test -p worldwake-ai --test golden_determinism golden_save_load_preserves_generalized_contention_state`
- Passed `cargo test -p worldwake-sim save_load`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai --test golden_combat`
- Passed `cargo test -p worldwake-ai --test golden_production`
- Passed `cargo test -p worldwake-ai --test golden_determinism`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
