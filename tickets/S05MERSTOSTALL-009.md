# S05MERSTOSTALL-009: Add inventory audit hooks for stock containers

**Status**: PENDING
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: Yes — audit goal/action for facility stock inspection
**Deps**: S05MERSTOSTALL-005, S05MERSTOSTALL-008

## Problem

Facility controllers need to detect missing stock through belief-mismatch → `InvestigateViolation` → `SuspectedTheft`. Without audit hooks, theft from facility containers goes undetected — the controller never realizes stock is missing.

## Assumption Reassessment (2026-04-01)

1. Container contents must be perceptible to the facility controller — check perception model for whether container contents are observable, and whether `PerceptionProfile` is required.
2. Facility stock expectations must be seeded in the controller's beliefs — check whether initial stock beliefs are set during store/stage actions.
3. `InvestigateViolation` goal/action exists in the E17 pipeline — check current implementation.
4. `SuspectedTheft` event type exists — check current crime model for this event.
5. Belief-mismatch detection mechanism exists or needs creation — check whether existing perception updates can trigger investigation when expected items are missing.

## Architecture Check

1. Audit reuses the existing E17 investigation pipeline (belief mismatch → InvestigateViolation → SuspectedTheft) rather than introducing a dedicated audit system. The facility controller's beliefs about container contents serve as the "inventory record," and perception-driven mismatch detection triggers investigation.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Container contents perceptible to facility controller → perception test
2. Missing stock detected via belief mismatch → belief state test
3. `InvestigateViolation` fires on detected mismatch → candidate generation test
4. `SuspectedTheft` produced at end of investigation → event-log delta (integration test)

## What to Change

### 1. Ensure container contents are perceptible

In perception/placement modules: verify that agents at a facility's place can perceive the contents of display and stock containers. If not, add perception rules for container contents visibility.

### 2. Seed facility stock expectations

Ensure that store/stage actions update the controller's beliefs about what should be in each container, so that subsequent perception can detect mismatches.

### 3. Wire audit-driven investigation

Ensure that belief-mismatch detection (expected item missing from container) triggers `InvestigateViolation` through the existing E17 pipeline.

## Files to Touch

- `crates/worldwake-sim/src/placement.rs` (modify — if perception rules needed)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — container contents visibility)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — investigation candidates from stock mismatch)

## Out of Scope

- Spoilage or degradation of stored goods
- Institutional inventory records (beyond agent beliefs)
- Golden tests for audit scenarios (010)

## Acceptance Criteria

### Tests That Must Pass

1. Container contents are perceptible to the facility controller
2. Missing stock detected through belief mismatch
3. `InvestigateViolation` candidate generated on detected mismatch
4. `SuspectedTheft` produced at end of investigation chain
5. Existing suite: `cargo test --workspace`

### Invariants

1. Information locality — controller detects theft through perception, not global queries
2. Belief-only planning — investigation planned from beliefs, not world state
3. E17 pipeline reused — no parallel investigation mechanism

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/placement.rs` — container contents perceptible at facility place
2. `crates/worldwake-ai/src/candidate_generation.rs` — investigation candidates from stock mismatch
3. Integration test — full chain from missing stock to SuspectedTheft

### Commands

1. `cargo test -p worldwake-sim -- container`
2. `cargo test -p worldwake-ai -- investigate`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
