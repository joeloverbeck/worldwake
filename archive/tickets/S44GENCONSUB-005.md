# S44GENCONSUB-005: ContentionStatus + Affordance annotation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new field on Affordance struct (worldwake-sim), new enum in worldwake-core
**Deps**: S44GENCONSUB-003

## Problem

The planner currently has no visibility into contention state when evaluating affordances. An agent may plan for a contention-full target with no signal that the resource is contested. Adding a `ContentionStatus` annotation to `Affordance` lets the planner filter or deprioritize contested targets, satisfying FOUNDATIONS P21 (intentions are not entitlements).

## Assumption Reassessment (2026-04-04)

1. `Affordance` struct at `crates/worldwake-sim/src/affordance.rs:7-13` has fields: `def_id`, `actor`, `bound_targets`, `payload_override`, `explanation`. No contention field. Confirmed.
2. `get_affordances()` at `crates/worldwake-sim/src/affordance_query.rs:9` returns `Vec<Affordance>`. All callers must handle the new field. Confirmed.
3. `get_affordances_for_defs()` at line 60 is the filtered variant — also returns `Vec<Affordance>`.
4. Affordance consumers in worldwake-ai read def_id, actor, bound_targets, payload_override, but the broader fallout is shared-shape rather than active logic changes: many `Affordance { ... }` constructors and a few struct patterns exist across worldwake-sim, worldwake-ai, worldwake-systems, and worldwake-cli tests. This is not "minimal consumer changes"; it is bounded but repo-wide constructor fallout.
5. `ContentionStatus` is a derived value computed per-query from queue state — not stored. Confirmed in spec Section H.4.
6. The current `RuntimeBeliefView` surface exposes `has_contention_policy`, `facility_queue_position`, and `facility_grant`, but not enough information to distinguish `Available` from `Full`. This ticket must widen that view contract with read-only queue-capacity state so affordance query can derive `Full` honestly.

## Architecture Check

1. Adding `contention_status: ContentionStatus` to `Affordance` with default `Unmanaged` preserves existing unmanaged affordance meaning, but all constructor sites must be updated explicitly.
2. No backward-compatibility shims — this is a direct field and trait-surface expansion, not a wrapper around an old path.

## Verification Layers

1. Affordance for unmanaged entity has `ContentionStatus::Unmanaged` → unit test
2. Affordance for entity with grant held by actor has `Granted` → unit test
3. Affordance for entity with full queue has `Full` → unit test
4. Affordance for entity with available queue has `Available` → unit test
5. Cross-layer: affordance query reads contention state through the widened `RuntimeBeliefView`, and AI/systems/CLI carry the new `Affordance` field through unchanged. Both remain read-only on contention state (P26).

## What to Change

### 1. Add ContentionStatus enum to worldwake-core

In `crates/worldwake-core/src/contention.rs`:
```rust
pub enum ContentionStatus {
    Unmanaged,
    Granted,
    Queued { position: u32 },
    Available,
    Full,
}
```
Derive: Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize. Implement Default as `Unmanaged`.

### 2. Add contention_status field to Affordance

In `crates/worldwake-sim/src/affordance.rs`, add `pub contention_status: ContentionStatus` to `Affordance`. Update all construction sites to include the field (default `Unmanaged` for non-contention paths).

### 3. Widen RuntimeBeliefView for derived contention status

In `crates/worldwake-sim/src/belief_view.rs`, extend `RuntimeBeliefView` with the read-only contention helpers needed to derive `Available` vs `Full` in affordance query, and update concrete/stub implementations accordingly.

### 4. Compute contention status in get_affordances()

In `crates/worldwake-sim/src/affordance_query.rs`, after enumerating affordances, check whether the bound targets include a contention-managed entity. If so, compute `ContentionStatus` from current queue state: actor holds grant → `Granted`; actor in queue → `Queued { position }`; queue joinable → `Available`; queue at capacity → `Full`.

### 5. Update Affordance consumers

Update any code in worldwake-sim, worldwake-ai, worldwake-systems, or worldwake-cli that constructs or pattern-matches on `Affordance` to include the new field.

## Files to Touch

- `crates/worldwake-core/src/contention.rs` (modify — add ContentionStatus)
- `crates/worldwake-core/src/lib.rs` (modify — add re-export)
- `crates/worldwake-sim/src/belief_view.rs` (modify — widen RuntimeBeliefView)
- `crates/worldwake-sim/src/affordance.rs` (modify — add field)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — compute status)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement widened view helpers)
- `crates/worldwake-ai/src/*.rs` (modify — update RuntimeBeliefView stubs and Affordance construction/matching as needed)
- `crates/worldwake-systems/src/*.rs` / `crates/worldwake-cli/src/*.rs` (modify — update Affordance construction sites as needed)

## Out of Scope

- Planner feasibility scoring based on ContentionStatus (future refinement)
- Action validation gating (S44GENCONSUB-006)

## Acceptance Criteria

### Tests That Must Pass

1. Affordance for entity without ContentionQueue has `Unmanaged` status
2. Affordance for entity where actor holds grant has `Granted` status
3. Affordance for entity with full queue (max_waiters reached) has `Full` status
4. Existing suite: `cargo test --workspace`

### Invariants

1. `ContentionStatus` is always derived from current queue state, never stored
2. Default `Unmanaged` for all non-contention-managed entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/affordance_query.rs` (tests) — contention status computation
2. `crates/worldwake-core/src/contention.rs` (tests) — ContentionStatus bounds and serialization
3. RuntimeBeliefView stub implementations updated where this shared query surface is mocked

### Commands

1. `cargo test -p worldwake-sim affordance`
2. `cargo test -p worldwake-core contention`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- **Completed**: 2026-04-04
- Added `ContentionStatus` to `crates/worldwake-core/src/contention.rs` and re-exported it from `crates/worldwake-core/src/lib.rs`.
- Added `contention_status` to `crates/worldwake-sim/src/affordance.rs`.
- Extended `RuntimeBeliefView` in `crates/worldwake-sim/src/belief_view.rs` with read-only queue-capacity state and implemented it in `crates/worldwake-sim/src/per_agent_belief_view.rs` so affordance query can distinguish `Available` from `Full` honestly.
- Updated `crates/worldwake-sim/src/affordance_query.rs` to derive `Unmanaged`, `Granted`, `Queued`, `Available`, and `Full` from live contention state.
- Added focused core and sim proofs for `ContentionStatus` defaults/serialization and for unmanaged, granted, queued, available, and full affordance statuses.
- Absorbed the real shared-shape fallout by updating direct `Affordance` constructors in sim, AI, systems, and CLI tests/helpers to carry `ContentionStatus::Unmanaged` where affordances are built manually.
- **Deviation from original plan**: the live `RuntimeBeliefView` surface did not expose enough information to distinguish `Available` from `Full`, so this ticket widened that read surface in addition to adding the enum and affordance field. The downstream fallout was broader shared constructor churn than the original ticket claimed, but it remained read-only and bounded.
- **Verification**:
  - `cargo test -p worldwake-sim affordance`
  - `cargo test -p worldwake-core contention`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-cli`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
