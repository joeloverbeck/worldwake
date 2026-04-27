# S127QUAAWAACQ-005: ResourceExtractionQueues component + per-slot reservation key widening

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — adds `ResourceExtractionQueues` component to `worldwake-core`, registers it via `component_schema.rs`, adds `GoalBeliefView::resource_extraction_queues` and `FacilityBeliefView::resource_extraction_queues` accessors with FND-14A co-location gating, registers queues in scenario translator at `spawn_scenario`, bumps `SAVE_FORMAT_VERSION` to 52. Note: the slot-aware reservation registration contract was reassessed out of scope and assigned to ticket 007 (see Assumption Reassessment item 14).
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
14. **Auto-correction (2026-04-27)**: the original "Widen reservation registration paths" deliverable (former section 3 of `What to Change`) is bogus — the existing `enqueue_for_contention` / `validate_contention_queue_admission` / `ensure_matching_contention_grant` helpers (at `crates/worldwake-systems/src/facility_queue_actions.rs:160-214` and `crates/worldwake-systems/src/production_actions.rs:339-396`) operate on the singleton `ContentionQueue` component used by facilities, beds, corpses, patients, and unique items. None of these entities have `extraction_slots` and the harvest action does **not currently route through any contention helper at all** — `start_harvest` at `production_actions.rs:410` does not enqueue/grant against `ResourceSource`. Adding a `slot_index: u8` parameter to those helpers is meaningless because their target storage is unslotted. The ticket's "per-slot reservation key widening" claim is satisfied structurally: the new `ResourceExtractionQueues` component holds `Vec<ContentionQueue>`, so the key is `(entity, slot_index)` because the storage holds N independent queues. Ticket 007 (`tickets/S127QUAAWAACQ-007.md`) explicitly owns the harvest start handler that consumes the slotted storage; ticket 007 will inline `txn.get_component_resource_extraction_queues(workstation).cloned()` and operate on `queues[chosen_slot]` directly. **Correction applied:** removed former section 3 of `What to Change`, dropped the speculative reservation-call-site sweep entry from `Files to Touch`, narrowed the per-slot isolation tests to exercise the new component's storage (`ResourceExtractionQueues.queues[i].enqueue/promote_head/...`), and dropped invariant 2 ("`slot_index < queues.len()` is enforced at every registration site") since no production registration site is added in this ticket — that invariant lands with ticket 007. The remaining invariant on slot bounds is implicit in `ResourceExtractionQueues.queues.len() == ResourceSource.extraction_slots.get() as usize` (invariant 1). **Why safe:** narrows scope to the storage substrate this ticket actually owns; the slotted-reservation contract becomes live in ticket 007 with the harvest start handler that produces real slot indices. Per-slot isolation is still proved at the `ContentionQueue` substrate level (each slot's queue is independently stateful).

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

### 3. Add `GoalBeliefView::resource_extraction_queues` accessor in `crates/worldwake-sim/src/belief_view.rs`

Same pattern as ticket 004's `last_harvest_trace` accessor:

```rust
fn resource_extraction_queues(&self, entity: EntityId) -> Option<ResourceExtractionQueues>;
```

`RuntimeBeliefView` impl with FND-14A co-location gating; macro/blanket-impl forwarding.

### 4. Register queues at scenario spawn in `crates/worldwake-cli/src/scenario/mod.rs:417`

After constructing `ResourceSource` (per ticket 003), additionally call:

```rust
txn.set_component_resource_extraction_queues(
    source_entity,
    ResourceExtractionQueues {
        queues: vec![ContentionQueue::default(); def.extraction_slots as usize],
    },
)?;
```

### 5. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:6` — bump from `51` to `52`.

### 6. Add focused tests

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
- `crates/worldwake-sim/src/belief_view.rs` (modify — accessor trait method, blanket-impl forwarding)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `RuntimeBeliefView` impl with FND-14A co-location gating; mirrors ticket 004's `last_harvest_trace` pattern)
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
2. Each slot's `ContentionQueue` is independently stateful (per-slot `granted` / `waiting` bookkeeping).
3. The belief-view accessor honors FND-14A co-location gating.
4. `SAVE_FORMAT_VERSION = 52`.

> Note: ticket 007 owns the slotted-reservation registration contract (harvest start handler scanning `queues[..]`); slot-bounds enforcement at registration sites lands there.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/contention.rs` `#[cfg(test)]` — bincode round-trip and per-slot isolation tests.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — co-location gating test (the `RuntimeBeliefView` impl with FND-14A gating lives there; `belief_view.rs` only declares the trait).
3. `crates/worldwake-cli/src/scenario/mod.rs` `#[cfg(test)]` — spawn registers queue per slot.
4. ~~Update existing reservation-path tests~~ — dropped per reassessment item 14: existing reservation paths operate on the unslotted `ContentionQueue` for facility/unique-item contention; the slot-aware contract is owned by ticket 007.

### Commands

1. `cargo test -p worldwake-core resource_extraction_queues per_slot_reservation`
2. `cargo test -p worldwake-sim belief_view_resource_extraction_queues`
3. `cargo test -p worldwake-cli scenario_spawn_registers_queue`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh`

## Outcome

Completed on 2026-04-27.

- Added `ResourceExtractionQueues { queues: Vec<ContentionQueue> }` (`crates/worldwake-core/src/contention.rs`) with `Component` impl and crate-root re-export.
- Registered the component through `with_component_schema_entries!` (`crates/worldwake-core/src/component_schema.rs`) gated to `EntityKind::Facility | EntityKind::Place`, and threaded the macro-expansion-site imports (`delta.rs`, `world.rs`, `component_tables.rs`) and the `ComponentValue` / `ComponentKind::ALL` test inventories.
- Added `resource_extraction_queues(entity) -> Option<ResourceExtractionQueues>` on `GoalBeliefView` and `FacilityBeliefView` (`crates/worldwake-sim/src/belief_view.rs`) with macro forwarding from the goal blanket-impl, plus the `RuntimeBeliefView`-side concrete impl on `PerAgentBeliefView` (`crates/worldwake-sim/src/per_agent_belief_view.rs`) gated by FND-14A co-location.
- Scenario translator (`crates/worldwake-cli/src/scenario/mod.rs:420`) now registers `ResourceExtractionQueues { queues: vec![ContentionQueue::default(); extraction_slots] }` on every resource source at spawn so the queue length matches `ResourceSource::extraction_slots`.
- Bumped `SAVE_FORMAT_VERSION` from 51 to 52 (`crates/worldwake-sim/src/save_load.rs`).
- Focused tests added: `resource_extraction_queues_bincode_roundtrip`, `per_slot_reservation_isolation`, `belief_view_resource_extraction_queues_co_located_only`, `scenario_spawn_registers_queue_per_slot`.

## Deviations

- Per Assumption Reassessment item 14, the original "Widen reservation registration paths" deliverable was dropped: the existing facility/unique-item contention helpers (`enqueue_for_contention`, `validate_contention_queue_admission`, `ensure_matching_contention_grant`) operate on the unslotted `ContentionQueue` component, which is not the storage `ResourceExtractionQueues` adds. The slot-aware reservation contract — picking a free slot, enqueueing on contention — is owned by ticket 007 (`tickets/S127QUAAWAACQ-007.md`), which will inline operations on `ResourceExtractionQueues.queues[chosen_slot]` from inside the harvest start handler. Per-slot isolation is still proved at the substrate level (`per_slot_reservation_isolation` exercises independent `granted` / `waiting` bookkeeping across two `ContentionQueue` slots).

## Verification Result

- Passed `cargo test -p worldwake-core resource_extraction_queues` (1/1 ok).
- Passed `cargo test -p worldwake-core per_slot_reservation` (1/1 ok).
- Passed `cargo test -p worldwake-sim belief_view_resource_extraction_queues` (1/1 ok).
- Passed `cargo test -p worldwake-cli scenario_spawn_registers_queue_per_slot` (1/1 ok).
- Passed `cargo test --workspace` (no failures across all crates).
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh` (exit 0; runs fmt-check, workspace tests, both clippy variants, and `scenario-coverage --check`).
