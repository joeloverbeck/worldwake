# S44GENCONSUB-009: Golden tests + SAVE_FORMAT_VERSION

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — golden test scenarios, save format version bump
**Deps**: S44GENCONSUB-006, S44GENCONSUB-007, S44GENCONSUB-008

## Problem

The contention substrate needs end-to-end proof that multi-agent contention resolves through inspectable world state, not invisible tick order. FOUNDATIONS Canonical Scenario E requires "any resulting line, grant, blocker, or reservation is inspectable world state rather than invisible runtime magic." Three golden test scenarios prove the substrate works across the full stack: action validation, contention system, perception, and AI replanning.

## Assumption Reassessment (2026-04-03)

1. Golden tests live in `crates/worldwake-ai/tests/` as `golden_*.rs` files. They use deterministic replay with `ChaCha8Rng` seeding. Confirmed via prior session knowledge.
2. `SAVE_FORMAT_VERSION` at `crates/worldwake-sim/src/save_load.rs:6`, currently 15. Must be bumped for new components.
3. Golden tests require `PerceptionProfile` on agents that need to observe post-action output (per CLAUDE.md). Critical for Scenario A where agents must perceive contention state.
4. After S44GENCONSUB-006/007/008: ContentionQueue is attached to entities, action validation gates through grants, and perception projects contention state into beliefs.
5. Scenario A needs: two agents co-located with a corpse, both wanting to loot. First gets grant, second queued. After first completes, second promoted.
6. Scenario B needs: agent queued for a facility, then travels away. System prunes departed agent.
7. Scenario C needs: entity with `max_waiters: Some(1)`. Two agents try to join. First succeeds, second rejected. Second replans.
8. All scenarios need deterministic replay companions.

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
6. Scenario C: rejected agent replans → decision trace (new goal selected)
7. Cross-layer: these are full-stack E2E tests covering core → sim → systems → ai → cli.

## What to Change

### 1. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`: increment `SAVE_FORMAT_VERSION` from current value.

### 2. Golden Scenario A: Corpse loot contention

Create `crates/worldwake-ai/tests/golden_corpse_contention.rs`:
- Setup: two agents with PerceptionProfile at same place, one corpse (dead agent with items)
- Both agents have needs driving loot motivation
- Tick forward: first agent gets grant, starts looting. Second agent either queued or replans.
- After first completes: second promoted, loots remaining items
- Assertions: ContentionGrant visible in world state, BelievedContentionState in observer beliefs, both agents eventually loot

### 3. Golden Scenario B: Contention with departure

Create `crates/worldwake-ai/tests/golden_contention_departure.rs`:
- Setup: agent queued for a facility with ContentionQueue, then given travel intent
- Tick forward: agent departs place
- Contention system prunes departed agent
- Next waiter promoted
- Assertions: departed agent no longer in queue, next agent holds grant

### 4. Golden Scenario C: Full queue rejection

Create `crates/worldwake-ai/tests/golden_contention_rejection.rs`:
- Setup: entity with `ContentionPolicy { max_waiters: Some(1) }`. Three agents at same place.
- First agent gets grant, second queued (position 0), third tries to join
- Third receives contention_rejected, replans to alternative
- Assertions: StartFailed trace with contention reason, third agent selects different goal

### 5. Deterministic replay companions

Each scenario includes a replay companion that re-runs with the same seed and verifies identical outcome.

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (modify — bump version)
- `crates/worldwake-ai/tests/golden_corpse_contention.rs` (new)
- `crates/worldwake-ai/tests/golden_contention_departure.rs` (new)
- `crates/worldwake-ai/tests/golden_contention_rejection.rs` (new)

## Out of Scope

- Phase 2 contention domains (bounty claims, storage, witness time)
- Performance optimization of contention system
- AI heuristics for queue avoidance (future refinement)

## Acceptance Criteria

### Tests That Must Pass

1. Golden Scenario A: two-agent corpse loot resolves through visible queue/grant state
2. Golden Scenario B: departed agent pruned from queue, next waiter promoted
3. Golden Scenario C: full queue rejection produces structured StartFailed, rejected agent replans
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

1. `crates/worldwake-ai/tests/golden_corpse_contention.rs` — Scenario A
2. `crates/worldwake-ai/tests/golden_contention_departure.rs` — Scenario B
3. `crates/worldwake-ai/tests/golden_contention_rejection.rs` — Scenario C

### Commands

1. `cargo test -p worldwake-ai golden_corpse_contention`
2. `cargo test -p worldwake-ai golden_contention_departure`
3. `cargo test -p worldwake-ai golden_contention_rejection`
4. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
