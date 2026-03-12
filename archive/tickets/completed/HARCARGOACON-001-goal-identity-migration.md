# HARCARGOACON-001: Migrate MoveCargo goal identity from lot-based to commodity-based

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` (shared goal schema), `worldwake-ai` (goal consumers and tests)
**Deps**: None (first ticket in the cargo-goal hardening chain)

## Problem

`GoalKind::MoveCargo { lot: EntityId, destination: EntityId }` makes the goal key depend on a specific authoritative lot entity. That is the wrong identity boundary.

Cargo movement intent is "move commodity X to place Y", while lot identity is a volatile execution detail that can change during exact materialization. Partial pickup can split a lot and mint a new authoritative entity, so the current goal key becomes stale across replanning even though the agent is still pursuing the same cargo-delivery intent.

The hardening spec therefore requires `MoveCargo` goal identity to be based on `commodity + destination`, not `lot + destination`.

## Assumption Reassessment (2026-03-12)

### Confirmed Facts

1. `GoalKind::MoveCargo` is currently defined as `MoveCargo { lot: EntityId, destination: EntityId }` in `crates/worldwake-core/src/goal.rs`.
2. `GoalKey::from` currently extracts `entity = Some(lot)` and `place = Some(destination)` for `MoveCargo`.
3. The direct field-sensitive downstream consumers are:
   - `crates/worldwake-ai/src/ranking.rs`
   - `crates/worldwake-ai/src/agent_tick.rs` test code
   - `crates/worldwake-core/src/goal.rs` tests
4. Additional `GoalKind::MoveCargo { .. }` matches exist in AI code (`goal_model.rs`, `candidate_generation.rs`, `search.rs`) but they are wildcard matches and do not depend on the old `lot` field layout.
5. `search.rs` still marks `MoveCargo` unsupported, `candidate_generation.rs` still does not emit `MoveCargo`, and `goal_model.rs` still treats `MoveCargo` as unsatisfied. Those are real architectural gaps, but they are separate follow-on work described in `specs/HARDENING-cargo-goal-continuity.md`, not part of this ticket's implementation slice.

### Discrepancies Corrected From The Original Ticket

1. The original ticket implied this change was close to making cargo movement operational. It does not. After this ticket, cargo-goal identity is cleaner, but cargo candidate generation, search support, and satisfaction semantics remain unfinished by design.
2. The original ticket's test plan was too narrow. This slice should add at least one invariant-focused test proving that lot identity no longer affects `MoveCargo` goal identity.
3. The original "files to touch" list was directionally right for implementation, but the assumptions section understated the real codebase blast radius by mixing wildcard matches and field-sensitive matches together.

## Architecture Assessment

This migration is more beneficial than the current architecture and should be done even though the rest of the cargo-goal pipeline is still incomplete.

Why:

1. Goal identity should encode stable intent, not volatile execution artifacts. `commodity + destination` is the stable intent boundary; `lot + destination` is not.
2. This keeps the goal layer decoupled from planner materialization details. The runtime binding system should handle hypothetical-to-authoritative entity continuity; goal identity should not depend on that mechanism.
3. The change is extensible. Future cargo planning can vary source lot, split behavior, and batch sizing without changing the semantic goal key.

Important limit:

1. This ticket improves the architecture, but it does **not** by itself make cargo movement a complete autonomous capability. The rest of the hardening chain must remove the unsupported/deferred behavior and add destination-aware satisfaction/candidate logic.

## Scope

### In Scope

1. Replace `GoalKind::MoveCargo { lot, destination }` with `GoalKind::MoveCargo { commodity, destination }`.
2. Update `GoalKey::from` so `MoveCargo` contributes:
   - `commodity = Some(commodity)`
   - `entity = None`
   - `place = Some(destination)`
3. Update direct field-sensitive downstream logic to use `commodity` instead of `lot`.
4. Update and strengthen tests for the new identity invariant.

### Out Of Scope

1. Emitting `MoveCargo` from candidate generation.
2. Removing `MoveCargo` from unsupported goals in search.
3. Adding `MoveCargo` satisfaction semantics.
4. Adding destination-aware belief/planning helpers.
5. Changing `PlannerOpKind::MoveCargo`.
6. Adding compatibility aliases or dual-schema support for the old lot-based variant.

## What To Change

### 1. Shared Goal Schema

In `crates/worldwake-core/src/goal.rs`, replace:

```rust
MoveCargo { lot: EntityId, destination: EntityId }
```

with:

```rust
MoveCargo {
    commodity: CommodityKind,
    destination: EntityId,
}
```

### 2. Canonical GoalKey Extraction

Change the `GoalKey::from` arm for cargo goals from lot-based extraction to commodity-based extraction:

```rust
GoalKind::MoveCargo { commodity, destination } => {
    (Some(commodity), None, Some(destination))
}
```

### 3. Ranking Motive Scoring

In `crates/worldwake-ai/src/ranking.rs`, stop deriving the cargo commodity from a lot entity:

```rust
GoalKind::MoveCargo { commodity, destination } => {
    let signal = market_signal_for_place(context.view, context.agent, commodity, destination);
    score_product(context.utility.enterprise_weight, signal)
}
```

This is the only behaviorally meaningful code change in this ticket.

### 4. Tests

Update existing construction sites and add coverage for the identity invariant:

1. `crates/worldwake-core/src/goal.rs`
   - update the existing `MoveCargo` key extraction test
   - add a test proving two cargo goals with the same `commodity + destination` but different former lot identities would now map to the same canonical `GoalKey` fields
2. `crates/worldwake-ai/src/agent_tick.rs`
   - update `GoalKind::MoveCargo` test construction to use `commodity`
3. `crates/worldwake-ai/src/ranking.rs`
   - add or update a test showing cargo motive scoring reads the goal commodity directly rather than depending on a lot lookup

## Files Expected To Change

1. `crates/worldwake-core/src/goal.rs`
2. `crates/worldwake-ai/src/ranking.rs`
3. `crates/worldwake-ai/src/agent_tick.rs`

## Acceptance Criteria

1. `GoalKind::MoveCargo` no longer stores a lot `EntityId`.
2. `GoalKey` for `MoveCargo` uses `commodity = Some(commodity)`, `entity = None`, `place = Some(destination)`.
3. Cargo motive scoring in `ranking.rs` uses the goal commodity directly and no longer looks up lot commodity from belief state.
4. No backward-compatibility alias for the old lot-based variant exists anywhere.
5. Tests cover the new goal-identity invariant, not just compile fallout.
6. `cargo test --workspace` passes.
7. `cargo clippy --workspace` passes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs`
   - `goal_key_extracts_entity_and_place_for_move_cargo` updated to assert commodity-based extraction
   - new invariant test for stable cargo goal identity by `commodity + destination`
2. `crates/worldwake-ai/src/ranking.rs`
   - new or updated test for commodity-driven `MoveCargo` motive scoring
3. `crates/worldwake-ai/src/agent_tick.rs`
   - update existing cargo-goal construction in runtime continuity coverage

### Commands

1. `cargo test -p worldwake-core goal`
2. `cargo test -p worldwake-ai ranking`
3. `cargo test -p worldwake-ai agent_tick`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - migrated `GoalKind::MoveCargo` from `lot + destination` to `commodity + destination`
  - updated `GoalKey` extraction to use canonical cargo intent identity (`commodity`, `place`) with no cargo entity field
  - updated cargo ranking to score directly from the goal commodity instead of looking up commodity from a lot entity
  - updated affected tests and added invariant-focused coverage for stable cargo goal identity and direct commodity-based ranking
- Deviations from original plan:
  - the ticket itself was corrected before implementation to make clear that this slice improves cargo-goal architecture but does not yet make cargo movement a fully supported autonomous goal
  - test coverage was strengthened beyond compile-fix updates to capture the intended identity invariant explicitly
- Verification results:
  - `cargo test -p worldwake-core goal` passed
  - `cargo test -p worldwake-ai ranking` passed
  - `cargo test -p worldwake-ai agent_tick` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
