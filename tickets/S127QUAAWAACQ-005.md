# S127QUAAWAACQ-005: ResourceExtractionQueues component + per-slot reservation key widening

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — adds `ResourceExtractionQueues` component to `worldwake-core`, registers it via `component_schema.rs`, widens reservation registration paths to take `slot_index: u8`, adds `GoalBeliefView::resource_extraction_queues` accessor, registers queues in scenario translator at `spawn_scenario`, bumps `SAVE_FORMAT_VERSION`
**Deps**: S127QUAAWAACQ-003

## Problem

S127 makes per-slot extraction contention explicit world state (D6). With `extraction_slots > 1`, multiple agents may hold concurrent reservations on the same source by claiming different slot indices. The reservation key widens from `entity` to `(entity, slot_index)`. Per FND-26 the queues live in a dedicated `ResourceExtractionQueues` component — separate from `ResourceSource`'s commodity/quantity state — so the two concerns don't couple. This ticket lands D6's component, the per-slot key widening, D9's second half (`resource_extraction_queues(entity)` belief-view accessor with FND-14A gating), and D10's third slice (the spawn translator registers `vec![ContentionQueue::default(); extraction_slots]` on each resource source). Bumps `SAVE_FORMAT_VERSION`.

## Assumption Reassessment (2026-04-26)

1. `crates/worldwake-core/src/contention.rs:10-14` defines `ContentionQueue { next_ordinal: u32, waiting: BTreeMap<u32, ContentionWaiter>, granted: Option<ContentionGrant> }` — confirmed during reassessment. Single-queue-per-entity shape; this ticket adds a sibling component holding a `Vec<ContentionQueue>`, not modifying `ContentionQueue` itself.
2. `specs/S127-quantity-aware-acquisition.md` D6 prescribes `ResourceExtractionQueues { queues: Vec<ContentionQueue> }`. D9 prescribes `resource_extraction_queues(entity)` accessor. D10 prescribes spawn-time registration with `vec![ContentionQueue::default(); extraction_slots]`.
3. Shared boundary: the **reservation registration path** — every site that today keys a reservation by `entity` must accept a `slot_index: u8` parameter. Reassessment surfaced this as Issue I8; the spec resolves it via Question 3 option (b) (separate component holding `Vec<ContentionQueue>`). The exact reservation registration call sites must be enumerated during implementation: `grep -rn "ContentionQueue\|reservation\|reserve_" crates/` to find every site that registers, releases, or transitions reservations.
4. Component registration in `component_schema.rs` follows the same pattern as ticket 004's `LastHarvestTrace` — `with_component_schema_entries!` macro entry, plus macro-expansion-site imports in `delta.rs`, `world.rs`, `component_tables.rs` per `tickets/README.md` check #13.
5. Scenario translator at `crates/worldwake-cli/src/scenario/mod.rs:417` constructs `ResourceSource` (post-ticket-003). After this ticket, the same site must additionally register `ResourceExtractionQueues { queues: vec![ContentionQueue::default(); def.extraction_slots as usize] }` on the source entity using the macro-generated `set_component_resource_extraction_queues` accessor.
6. `GoalBeliefView::resource_extraction_queues(entity) -> Option<ResourceExtractionQueues>` mirrors ticket 004's `last_harvest_trace` accessor pattern: FND-14A co-location gating, lives on the same `EntityBeliefView` sub-trait, follows the existing `resource_source(entity)` precedent at `belief_view.rs:417`.
7. `SAVE_FORMAT_VERSION` after ticket 004 is `51`; this ticket bumps to `52`.
8. Existing tests exercising the reservation registration path: locate during implementation by grepping `crates/` for tests on `ContentionQueue` and `reserve_*` symbols. Update Test Plan once known.
9. Stale-request / contested-affordance boundary check: the first-failure boundary for "all slots occupied" is the **harvest action's start handler** (ticket 007 will scan `queues[..]` for a free slot and fall back to enqueuing); this ticket only widens the key surface and registers the storage. Contention-conflict emission via `BlockingFact(ReservationConflict)` continues to fire from the same blocker-memory path it does today.
13. Adjacent contradictions: tickets 006 and 007 both consume `ResourceExtractionQueues` (commit handler appends and ticket 007's start handler reads queues to pick a slot). They depend on this ticket — sequential consumers, no contradiction.

## Architecture Check

1. Separating `ResourceExtractionQueues` from `ResourceSource` aligns with FND-26 (systems interact through state, not through each other) — production reads/writes `ResourceSource`, contention reads/writes the queues component, perception surfaces both via FND-14A. Inlining a `Vec<ContentionQueue>` field on `ResourceSource` would couple commodity/quantity state with reservation state, violating that separation.
2. Per-slot keying `(entity, slot_index: u8)` makes occupancy concrete (FND-8) — the waiting agent's projected delay is `extraction_duration_ticks * queue_position`, computable from the slot's queue state, no opaque blocker cooldown.
3. Reservation key widening preserves existing `ContentionQueue` shape — the underlying queue substrate doesn't change, only its registration index does. This is an additive change to the reservation surface, not a rewrite.

## Verification Layers

1. `ResourceExtractionQueues` round-trips via bincode preserving all queue contents → focused unit test in `contention.rs` `#[cfg(test)]`.
2. Per-slot reservation registration: registering a reservation at `(entity, slot_index = 0)` and another at `(entity, slot_index = 1)` produces two independent queues with separate `granted` slots → focused test (action-handler-adjacent layer).
3. Reservation rejection at occupied slot: registering a third reservation at an already-granted slot enqueues into `waiting`, not `granted` → focused test asserting authoritative reservation state. Map this to **focused authoritative runtime coverage** per `docs/precision-rules.md` Rule 9.
4. Belief-view accessor returns queues only when co-located → focused test mirroring ticket 004's `last_harvest_trace` co-location test.
5. Scenario spawn registers `queues.len() == extraction_slots` → focused test in `scenario/mod.rs`.
6. Save format rejects version `51` saves → existing infrastructure.

## What to Change

### 1. Define `ResourceExtractionQueues` in `crates/worldwake-core/src/contention.rs`

Add per spec D6 alongside the existing `ContentionQueue`:

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceExtractionQueues {
    pub queues: Vec<ContentionQueue>,
}

impl Component for ResourceExtractionQueues {}
```

### 2. Register in `component_schema.rs`

Same pattern as ticket 004's `LastHarvestTrace`: `with_component_schema_entries!` entry, plus `delta.rs`/`world.rs`/`component_tables.rs` import updates per `tickets/README.md` check #13.

### 3. Widen reservation registration paths

Enumerate every site that registers, releases, or transitions a reservation against an entity-keyed queue. Each site gains a `slot_index: u8` parameter that selects which `queues[slot_index]` to operate on. Backwards-compatible default is `slot_index = 0` for non-multi-slot facilities, but per FND-28 we do NOT add a defaulting wrapper — every call site is updated explicitly. Discovery instruction: `grep -rn "ContentionQueue\|reserve_\|grant_\|release_" crates/worldwake-core/src/ crates/worldwake-sim/src/ crates/worldwake-systems/src/`.

### 4. Add `GoalBeliefView::resource_extraction_queues` accessor in `crates/worldwake-sim/src/belief_view.rs`

Same pattern as ticket 004's `last_harvest_trace` accessor:

```rust
fn resource_extraction_queues(&self, entity: EntityId) -> Option<ResourceExtractionQueues>;
```

`RuntimeBeliefView` impl with FND-14A co-location gating; macro/blanket-impl forwarding.

### 5. Register queues at scenario spawn in `crates/worldwake-cli/src/scenario/mod.rs:417`

After constructing `ResourceSource` (per ticket 003), additionally call:

```rust
txn.set_component_resource_extraction_queues(
    source_entity,
    ResourceExtractionQueues {
        queues: vec![ContentionQueue::default(); def.extraction_slots as usize],
    },
)?;
```

### 6. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:6` — bump from `51` to `52`.

### 7. Add focused tests

- `resource_extraction_queues_bincode_roundtrip` in `contention.rs`
- `per_slot_reservation_isolation` — two reservations at different slot indices have independent grants
- `belief_view_resource_extraction_queues_co_located_only` in `belief_view.rs`
- `scenario_spawn_registers_queue_per_slot` in `scenario/mod.rs` — confirms `queues.len() == extraction_slots` after spawn

## Files to Touch

- `crates/worldwake-core/src/contention.rs` (modify — add component, focused tests)
- `crates/worldwake-core/src/component_schema.rs` (modify — register `ResourceExtractionQueues`)
- `crates/worldwake-core/src/delta.rs` (modify — macro-expansion-site import)
- `crates/worldwake-core/src/world.rs` (modify — macro-expansion-site import)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro-expansion-site import)
- `crates/worldwake-sim/src/belief_view.rs` (modify — accessor trait method, RuntimeBeliefView impl, blanket-impl forwarding)
- **Likely:** all reservation-registration call sites discovered via `grep -rn "ContentionQueue\|reserve_\|grant_\|release_" crates/worldwake-core/src/ crates/worldwake-sim/src/ crates/worldwake-systems/src/` (modify — add `slot_index` parameter)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — register queues at spawn)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION)

## Out of Scope

- Harvest action start handler scanning `queues[..]` for free slots — ticket 007.
- Harvest commit handler appending to `LastHarvestTrace` — ticket 006.
- Source reliability (S131) reading queue→grant transitions for `average_wait_ticks` — out of scope (S131 is a draft soft-dep).
- `LastHarvestTrace` component — ticket 004.

## Acceptance Criteria

### Tests That Must Pass

1. `resource_extraction_queues_bincode_roundtrip` — round-trip preserves all queues.
2. `per_slot_reservation_isolation` — reservations at different slot indices have independent grants.
3. `belief_view_resource_extraction_queues_co_located_only` — non-co-located access returns `None`.
4. `scenario_spawn_registers_queue_per_slot` — spawning a scenario with `extraction_slots = 5` produces a `ResourceExtractionQueues` with `queues.len() == 5`.
5. Existing reservation-path tests still pass (recorded during reassessment).
6. Existing suite: `cargo test --workspace`.

### Invariants

1. `ResourceExtractionQueues.queues.len() == ResourceSource.extraction_slots.get() as usize` always (post-spawn).
2. Reservations key by `(entity, slot_index)`; `slot_index < queues.len()` is enforced at every registration site.
3. The belief-view accessor honors FND-14A co-location gating.
4. `SAVE_FORMAT_VERSION = 52`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/contention.rs` `#[cfg(test)]` — bincode round-trip and per-slot isolation tests.
2. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — co-location gating test.
3. `crates/worldwake-cli/src/scenario/mod.rs` `#[cfg(test)]` — spawn registers queue per slot.
4. Update existing reservation-path tests (named during reassessment) to use the slot-aware key.

### Commands

1. `cargo test -p worldwake-core resource_extraction_queues per_slot_reservation`
2. `cargo test -p worldwake-sim belief_view_resource_extraction_queues`
3. `cargo test -p worldwake-cli scenario_spawn_registers_queue`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh`
