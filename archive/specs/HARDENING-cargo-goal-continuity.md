# Hardening Spec: Cargo Goal Continuity & Destination-Aware Delivery

**Status**: ✅ COMPLETED

## Summary

`GoalKind::MoveCargo` exists today, but it is architecturally incomplete:

- candidate generation never emits it
- search explicitly treats it as unsupported
- goal satisfaction is permanently `false`
- its current identity (`lot + destination`) is brittle under partial pickup because the authoritative commit path can split the lot into a new entity

The result is that cargo movement is not a real autonomous capability. The planner can reason about partial pickup and hypothetical lot bindings, but the goal layer cannot express a stable cargo-delivery intent that survives materialization and replanning.

This spec fixes that by redesigning cargo-goal identity around a concrete delivery batch instead of a specific lot entity, then wiring candidate generation, search, and satisfaction semantics around that stable identity.

The key architectural decision is:

- do **not** special-case runtime dirtiness to "pretend nothing changed" after cargo mutations
- instead, make the cargo goal itself stable across those expected mutations so replanning and plan retention remain honest

That is cleaner, more extensible, and better aligned with the materialization-aware planning architecture already introduced by `archive/specs/HARDENING-hypothetical-entity-identity.md`.

## Phase

Pre-E14 hardening of the E01-E13 foundation

## Crates

- `worldwake-core`
- `worldwake-sim`
- `worldwake-ai`
- `worldwake-systems` only for integration verification

## Problem

### 1. Current Cargo Goal Identity Is Too Fragile

The current shared goal type is:

```rust
GoalKind::MoveCargo { lot: EntityId, destination: EntityId }
```

That identity does not survive exact partial pickup:

1. the goal names authoritative lot `L10`
2. exact `pick_up` may reduce `L10`
3. the carried batch becomes a new authoritative lot `L27`
4. the original goal key now points at the wrong entity

The planner can continue within a single precomputed plan because `PlanningEntityRef::Hypothetical(...)` and runtime bindings exist. But if replanning happens after the materializing step, the goal identity itself is stale.

### 2. Current Runtime Continuity Depends On Goal Re-Emission

`agent_tick.rs` marks the runtime dirty when observation snapshots change. That is not inherently wrong. In fact, cargo mutations are real world changes and should remain visible.

The problem is that the goal layer cannot re-emit the same cargo intent after that real mutation:

- `generate_candidates()` never emits `MoveCargo`
- `search.rs` marks `MoveCargo` unsupported
- `GoalKind::MoveCargo::is_satisfied(...)` is permanently `false`

So cargo plans are structurally unable to persist through ordinary plan refresh.

### 3. Merchant Restock Semantics Are Logistically Incomplete

Enterprise logic today can emit `RestockCommodity`, but the current satisfaction check for restocking is only:

```rust
state.commodity_quantity(actor, commodity) > Quantity(0)
```

That means a merchant can satisfy "restock bread" merely by acquiring bread anywhere, even away from the merchant's home market. The logistics leg back to market is not modeled as a first-class goal.

### 4. The Wrong Fix Would Be A Runtime Exception

A tempting patch would be:

- suppress dirtying for cargo mutations
- or special-case `MoveCargo` continuation through binding tables
- or compare old/new cargo goals through ad hoc materialization aliases

Those are all inferior:

- they make runtime continuity depend on exceptions rather than honest state
- they couple goal identity to planner internals
- they do not generalize well to future materializing domains

The real fix belongs in goal identity and candidate derivation, not in hiding state changes.

## Goals

1. Make cargo movement a real autonomous goal family rather than a deferred placeholder.
2. Give cargo goals a stable identity that survives partial pickup, split lots, and replanning.
3. Derive cargo goals from concrete, local, controllable world state rather than abstract logistics scores.
4. Let cargo-delivery plans remain valid through expected materializing steps without runtime hacks.
5. Ensure enterprise procurement is destination-aware: restock is not complete until stock reaches the relevant market.
6. Preserve determinism and the existing materialization-binding execution boundary.
7. Preserve Principle 7 locality by deriving cargo goals only from cargo the agent can concretely perceive/control now, plus already-stored destination knowledge such as `home_market` and `DemandMemory`.

## Non-Goals

1. This spec does not add a global hauling-job system.
2. This spec does not add omniscient remote inventory management on behalf of agents.
3. This spec does not redesign `replay_and_verify()`.
4. This spec does not introduce compatibility aliases for the old lot-based `MoveCargo` goal key.
5. This spec does not solve every observation-snapshot efficiency problem; broader relevance filtering remains separate hardening work.

## Design Overview

The fix has four parts:

1. replace brittle lot-based cargo goal identity with a concrete delivery-batch identity
2. emit cargo goals from concrete local controllable lots plus concrete destination demand
3. support cargo goals in search/goal satisfaction using destination-aware controlled quantity
4. let replanning remain honest: expected cargo mutations may dirty the runtime, but the same cargo goal can be rediscovered and retained

## Core Goal Redesign

### 1. Replace `MoveCargo { lot, destination }`

Replace the shared goal variant with:

```rust
GoalKind::MoveCargo {
    commodity: CommodityKind,
    destination: EntityId,
}
```

This means:

- the goal represents delivery of a commodity to a destination
- the goal does **not** name a specific lot entity
- the goal does **not** include quantity — quantity is a volatile planning parameter, not goal identity
- exact lot identity and batch size remain planning/execution concerns, not goal-identity concerns

This is the central architectural change.

**Why no `quantity` in goal identity**: All three inputs to `deliverable_quantity` (local stock, carry fit, restock gap) are volatile across replans. If quantity were part of `GoalKind`, conditions changing between replans would produce a different `GoalKey`, causing goal-switching logic to treat it as a new goal → plan abandoned → potential thrashing. The batch quantity belongs in `GroundedGoal` evidence where the planner can access it without coupling goal identity to volatile conditions.

### 2. Why Batch Identity Is Better

Commodity-destination identity survives the exact split path:

- before pickup: local lot `Water x 3`, carry fit `Water x 2`
- emitted cargo goal: `MoveCargo { Water, destination }` with `deliverable_quantity = 2` in evidence
- planner may split the source lot and create hypothetical carried lot `H1`
- runtime binds `H1 -> L27`
- if replanning occurs, the same goal still exists: `MoveCargo { Water, destination }`
- re-derived `deliverable_quantity` may differ (e.g., now only 1 remaining locally) but goal key is stable

No alias table is needed at the goal layer. No goal-switching thrashing from volatile quantity changes.

### 3. No Backward Compatibility Path

Do **not** keep both variants.

Do **not** add:

- legacy lot-based `MoveCargo`
- a wrapper that converts old cargo goals into new ones
- dual interpretation based on missing fields

Update all users of `GoalKind::MoveCargo` directly.

## Candidate Generation

### 1. Concrete Cargo Candidate Source

`generate_candidates()` must emit `MoveCargo` when all of the following are true:

1. the agent has a concrete destination need for a commodity
2. the destination is a concrete place
3. the agent currently has a local controllable batch of that commodity that is not already at the destination

For the current architecture, the primary destination source is:

- `MerchandiseProfile.home_market` combined with `DemandMemory`-derived restock pressure

The goal must be derived from local controlled cargo, not remote speculation.

### 2. Allowed Cargo Sources

Only cargo the agent can concretely act on now may seed a `MoveCargo` goal:

- direct possessions
- local ground lots the agent can control

Do **not** emit cargo goals from remote owned lots in another place.

That preserves locality and avoids omniscient logistics orchestration.

### 3. Delivery Batch Quantity

For each eligible local cargo opportunity, derive a `deliverable_quantity` as a **private helper in `candidate_generation.rs`** (not on `BeliefView`):

```rust
deliverable_quantity = min(
    local_controllable_quantity,
    restock_gap_at_destination,
    actor_current_carry_fit_for_that_commodity_batch
)
```

**Important**: `restock_gap_at_destination` is a NEW helper (see Section B below), distinct from the existing `restock_gap_for_market` in `enterprise.rs`. The existing helper computes gap using `commodity_quantity(agent, commodity)` which counts total agent stock everywhere. The new helper must use `controlled_commodity_quantity_at_place(agent, destination, commodity)` to compute gap specifically at the destination market. The existing `restock_gap_for_market` must not be modified — it continues to serve its existing purpose in `RestockCommodity` scoring.

Requirements:

- use exact carry/load math already present in the belief surface
- use `Quantity`, not floats
- if `deliverable_quantity == Quantity(0)`, emit no `MoveCargo` goal for that commodity-destination pair

This keeps the evidence batch concrete and single-trip-sized under the current planner architecture.

### 4. Evidence Model

`GroundedGoal` evidence for cargo delivery must include:

- the destination place
- the concrete local lots that could satisfy the batch
- the computed `deliverable_quantity` (for planner batch sizing, **not** for goal identity)

That gives search enough evidence to build a planning snapshot with:

- the source lot(s)
- the destination
- the actor's local placement and load state
- the batch size to plan for

## Belief / Planning Read Model Additions

To support batch-based cargo goals cleanly, add destination-aware quantity helpers.

Acceptable API shape:

```rust
fn controlled_commodity_quantity_at_place(
    &self,
    agent: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> Quantity;

fn local_controlled_lots_for(
    &self,
    agent: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> Vec<EntityId>;
```

Requirements:

- deterministic ordering
- concrete-state based
- implementable for both authoritative `BeliefView` and `PlanningState`

These helpers are better than overloading the existing global `commodity_quantity(agent, commodity)` because cargo satisfaction is explicitly destination-sensitive.

## Goal Model Semantics

### 1. Satisfaction

`GoalKind::MoveCargo { commodity, destination }` is satisfied when the destination has enough stock to meet observed demand:

```rust
restock_gap_at_destination(actor, destination, commodity).is_none()
```

Equivalently: `controlled_commodity_quantity_at_place(actor, destination, commodity) >= observed_demand_at(destination, commodity)`.

This is concrete, deterministic, stable across lot splitting, and — crucially — does not depend on a volatile quantity embedded in the goal key. Satisfaction tracks the underlying business need (demand met at destination) rather than a snapshot batch size.

### 2. Relevant Operation Kinds

`MoveCargo` remains a cargo/logistics goal, so its relevant operations stay:

- `PlannerOpKind::Travel`
- `PlannerOpKind::MoveCargo`

No trade/production fallback should be smuggled into cargo satisfaction itself.

### 3. Progress Barrier

`MoveCargo` should **not** be modeled as permanently unsatisfied.

Under the new batch identity:

- `pick_up` and partial `pick_up` are ordinary intermediate steps
- `travel` is ordinary progression
- if the delivered batch reaches destination, the goal satisfies directly

No special progress-barrier semantics are required purely because the source lot may split.

## Search / Planner Semantics

### 1. Remove `MoveCargo` From Unsupported Goals

`search.rs` must stop treating `GoalKind::MoveCargo` as unsupported.

### 2. Candidate Selection

Search should continue to use concrete action affordances and planner-only synthetic candidates:

- `pick_up`
- `travel`
- synthetic `put_down` only when the chosen plan actually requires grounding a hypothetical cargo entity

The goal no longer forces `put_down` as part of its identity. If carried cargo at destination already satisfies the batch, search may terminate there.

### 3. Partial Pickup Compatibility

The existing hypothetical-identity architecture remains authoritative:

- exact pickup may create a hypothetical split-off lot
- later steps may target that hypothetical lot
- runtime still binds it through `MaterializationBindings`

This spec does **not** replace that mechanism.

Instead, it makes the surrounding goal identity robust enough that replanning after such a step is still coherent.

## Agent Runtime Continuity

### 1. Keep Honest Dirtying

Do **not** add cargo-specific suppression to `observation_snapshot_changed()` as the primary fix.

Cargo mutations are real observations:

- possession changed
- commodity quantity changed
- place may change

The runtime may still become dirty after a successful cargo step.

### 2. Why Continuity Still Works

With the new cargo goal identity:

- the candidate stream can rediscover the same `MoveCargo { commodity, destination }`
- `select_best_plan()` can retain the current non-empty plan for the same goal
- partial-pickup materialization no longer invalidates the goal key itself

This is the desired architecture: continuity through stable goals, not through hidden mutations.

### 3. Observation Relevance Filtering

General relevance filtering for dirty snapshots remains valuable, but it should be treated as orthogonal hardening, not as the core cargo fix.

If implemented later, it should reduce wasted replans across many goal families, not just cargo.

## Enterprise Interaction

### 1. Restock Is Not Complete At Remote Acquisition

Restock architecture must remain physically honest:

- acquiring stock away from market is procurement
- delivering that stock to the market is logistics

Those are related but distinct concerns.

### 2. Recommended Flow

The clean current-phase flow is:

1. `RestockCommodity` causes procurement behavior
2. once a local controllable batch exists and the home-market gap remains, candidate generation emits `MoveCargo`
3. `MoveCargo` delivers the batch to the market
4. after delivery, restock pressure naturally dampens because destination stock now exists there

This keeps enterprise procurement and cargo delivery decoupled through shared concrete state.

## Component Registration

No new authoritative ECS component is required by default.

Explicit requirements:

- the cargo-goal redesign lives primarily in shared goal schema plus AI/sim read-model helpers
- no planner/runtime-only identity type should leak into authoritative world components
- `MerchandiseProfile` and `DemandMemory` remain the authoritative inputs for enterprise cargo demand

If implementation discovers a need for persistent cargo-task memory, that must be justified in a revision to this spec before coding.

## SystemFn Integration

### worldwake-core

Reads:

- `CommodityKind`
- `Quantity`
- `GoalKind`
- `MerchandiseProfile`
- `DemandMemory`

Writes:

- updated `GoalKind::MoveCargo` shared schema

### worldwake-sim

Reads:

- authoritative belief queries for local controllable cargo and destination-aware controlled quantity

Writes:

- no new authoritative runtime state required

### worldwake-ai

Reads:

- local controllable cargo
- actor carry/load state
- enterprise demand signals and destinations
- destination-aware controlled commodity quantity

Writes:

- concrete `MoveCargo` candidates
- cargo plans using existing planner/materialization machinery

### worldwake-systems

Reads:

- existing transport and travel semantics through affordances/action handlers

Writes:

- no architecture changes required beyond tests unless implementation discovers an authoritative bug

## Cross-System Interactions (Principle 12)

- enterprise pressure is read from authoritative demand-memory state
- cargo goals are produced in `worldwake-ai`
- search/planner still reasons through shared action definitions and planning state
- execution still happens through `worldwake-sim` + `worldwake-systems`

No system-to-system direct shortcuts are added.

## FND-01 Section H

### Information-Path Analysis

- Cargo goals must be derived only from:
  - current local controlled cargo
  - current actor load/capacity
  - stored destination knowledge already on the agent (`home_market`, `DemandMemory`)
- The agent must not query remote inventory at the destination to synthesize cargo plans unless that inventory is already reflected in local/stored belief inputs.
- The destination-aware quantity helper is a read-model convenience over concrete state, not a license for omniscient logistics queries.

### Positive-Feedback Analysis

- Restock pressure can create cargo movement.
- Successful cargo delivery can enable trade.
- Trade can produce more demand-memory observations, which can create future cargo pressure.

This is an intended causal loop, but it must remain physically bounded.

### Concrete Dampeners

The physical dampeners are:

- finite carry capacity
- finite commodity stock
- finite travel time over the place graph
- finite remembered demand based on actual observations and retention windows

These are concrete world limits. No numeric clamp should be introduced merely to stop cargo churn.

### Stored State vs Derived Read-Model

**Stored (authoritative):**

- lot quantities
- possession / placement relations
- `MerchandiseProfile.home_market`
- `DemandMemory`
- carry capacity and entity load inputs

**Stored (planner/runtime but non-authoritative):**

- current plan
- materialization bindings
- planning hypothetical entities / refs

**Derived (transient):**

- local controllable cargo batches
- destination-aware controlled commodity quantity
- deliverable batch quantity for a cargo candidate
- restock-driven cargo opportunity at a destination

## Invariants

1. Cargo goal identity must not depend on a single authoritative lot surviving unchanged.
2. Cargo candidate generation uses only concrete local cargo the agent can act on now.
3. Cargo satisfaction is destination-aware, not just "agent owns some commodity somewhere."
4. Partial pickup and lot splitting must not invalidate the cargo goal key.
5. The planner/runtime binding system remains the sole bridge from hypothetical step targets to authoritative entities.
6. No backward-compatibility alias path remains for lot-based cargo goals.

## Implementation Sections

### Section A: Shared Goal Identity Migration

Implement:

1. replace lot-based `GoalKind::MoveCargo { lot, destination }` with `MoveCargo { commodity, destination }`
2. update `GoalKey::from(GoalKind::MoveCargo)` extraction to use:
   - `commodity = Some(commodity)`
   - `entity = None`
   - `place = Some(destination)`
3. update `ranking.rs` — motive scoring at line ~258 currently destructures `GoalKind::MoveCargo { lot, destination }` and reads `lot` to get commodity via `item_lot_commodity(lot)`. Change to direct commodity-based scoring: `GoalKind::MoveCargo { commodity, destination } => market_signal_for_place(view, agent, commodity, destination)`
4. update all remaining tests and pattern matches across crates

### Section B: Destination-Aware Read Helpers

Implement:

1. destination-aware controlled commodity quantity helper(s) on `BeliefView` and `PlanningState`
2. local controllable cargo-lot enumeration helper(s) on `BeliefView` and `PlanningState`
3. `restock_gap_at_destination` helper in `enterprise.rs` — a NEW function (not modifying the existing `restock_gap_for_market`) that computes the gap using `controlled_commodity_quantity_at_place(agent, destination, commodity)` rather than `commodity_quantity(agent, commodity)`. This provides destination-local stock awareness for cargo candidate sizing and satisfaction checks.

### Section C: Cargo Candidate Generation

Implement:

1. concrete cargo-opportunity derivation from local controllable lots
2. `deliverable_quantity` as a private helper in `candidate_generation.rs` using `restock_gap_at_destination` and exact carry math
3. `MoveCargo` candidate emission with `deliverable_quantity` stored in `GroundedGoal` evidence (not in goal identity)
4. remove "deferred cargo goals" behavior — specifically update or remove the `deferred_goal_kinds_are_not_emitted` test at `candidate_generation.rs:1542` which currently asserts `MoveCargo` is excluded from candidates

### Section D: Goal Model & Search Support

Implement:

1. concrete `MoveCargo` satisfaction using `restock_gap_at_destination` (replacing the permanent `false` at `goal_model.rs:333`)
2. remove unsupported-goal rejection for cargo in `search.rs:236-241`
3. ensure cargo plans can terminate on delivery without requiring lot identity continuity
4. `apply_planner_step` for `MoveCargo`: confirm this intentionally remains a no-op (falls through to `_ => state` at `goal_model.rs:258`). State transitions for cargo happen via `PickUpGroundLot`/`PutDownGroundLot` transition kinds in `planner_ops.rs`, not via goal-level step application.
5. `is_progress_barrier` for `MoveCargo`: currently falls through to `_ => false` at `goal_model.rs:280`. Consider whether `pick_up` under a `MoveCargo` goal should be treated as a progress barrier (it can split lots, creating materialization). If so, add `GoalKind::MoveCargo { .. }` to the match arm at line 268 with `PlannerOpKind::MoveCargo` as the barrier op kind. If not, document the rationale (carried goods at destination satisfy the goal without requiring further put_down steps).

### Expected Search Plan Shape

For clarity, the expected plan shape for `MoveCargo` is:

- `pick_up` → `travel` → [optional `put_down`]

Satisfaction can be met by carrying goods at the destination (no `put_down` required if the satisfaction check counts carried-at-destination stock). This means a 2-step plan (`pick_up → travel`) is the minimal valid cargo plan.

### Multi-Trip Delivery Behavior

Agents deliver carry-capacity batches one trip at a time. If `deliverable_quantity` exceeds carry capacity, the evidence batch is capped to one trip. After delivery, the same `MoveCargo { commodity, destination }` goal may be re-emitted if the destination gap persists, triggering another trip. This is expected emergent behavior, not a bug.

### Section E: Verification

Add/strengthen tests for:

1. cargo candidate emission from merchant demand + local controllable stock
2. cargo-goal satisfaction after exact partial pickup and travel
3. same-goal continuity across replanning after materialization
4. merchant restock requiring delivery to `home_market`, not just remote acquisition
5. deterministic replay/conservation for representative cargo-delivery sequences

## Acceptance Criteria

1. `MoveCargo` is emitted as a real candidate when concrete local cargo plus concrete destination demand exist.
2. `MoveCargo` is no longer marked unsupported in search.
3. A partial-pickup cargo plan can survive replanning because the goal key remains stable across lot materialization.
4. Cargo satisfaction is destination-aware and concrete.
5. Merchant restock is not considered complete merely because the merchant acquired stock away from the home market.
6. Existing hypothetical-entity binding architecture remains intact and is reused rather than bypassed.
7. `cargo test --workspace` and `cargo clippy --workspace` pass after implementation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs`
   - cargo goals are emitted from local controllable stock plus destination demand
   - cargo goals are not emitted from remote stock the agent is not currently positioned to move
   - update or replace `deferred_goal_kinds_are_not_emitted` (line 1542) — MoveCargo must no longer be excluded
2. `crates/worldwake-ai/src/ranking.rs`
   - MoveCargo motive scoring works with commodity-based destructuring (no lot lookup)
3. `crates/worldwake-ai/src/goal_model.rs`
   - `MoveCargo` satisfaction uses `restock_gap_at_destination`
4. `crates/worldwake-ai/src/search.rs`
   - cargo goals are searchable and can terminate after delivery
   - partial-pickup cargo planning remains exact under the new goal identity
5. `crates/worldwake-ai/src/agent_tick.rs`
   - successful materializing cargo steps followed by dirty replanning retain the same cargo goal and continue coherently
6. `crates/worldwake-systems/tests/e10_production_transport_integration.rs`
   - enterprise cargo delivery / replay / conservation coverage where applicable

### Concrete Test Scenarios

1. **Basic delivery**: agent at Place A, Bread x 3 local, home_market=B with demand, carry fits 3 → `MoveCargo { Bread, B }` emitted, plan = `pick_up → travel`
2. **Partial pickup with split**: Water x 5 ground lot, carry fits 2 → evidence `deliverable_quantity = 2`, hypothetical split, plan completes
3. **No cargo from remote stock**: agent owns goods at non-local place → no `MoveCargo` emitted (locality)
4. **Goal stability across replan**: `pick_up` triggers lot split, dirty replan, same `MoveCargo { Water, B }` re-derived with potentially different `deliverable_quantity` in evidence — goal key unchanged
5. **Satisfaction at destination while carrying**: agent arrives at destination carrying the commodity — satisfaction check counts carried-at-destination stock (no `put_down` required)
6. **Zero deliverable suppression**: full carry capacity or zero restock gap → no `MoveCargo` emitted
7. **Conservation invariant across full delivery sequence**: lot quantities preserved through pick_up, travel, optional put_down
8. **Deterministic replay of cargo delivery**: full cargo delivery sequence replays identically from same seed and inputs

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai goal_model`
3. `cargo test -p worldwake-ai ranking`
4. `cargo test -p worldwake-ai search`
5. `cargo test -p worldwake-ai agent_tick`
6. `cargo test -p worldwake-systems --test e10_production_transport_integration`
7. `cargo test --workspace`
8. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-12
- What changed: `GoalKind::MoveCargo` now uses commodity-plus-destination identity, cargo candidate generation emits concrete destination-aware delivery goals from local controllable lots, `restock_gap_at_destination` and destination-local quantity helpers back both candidate sizing and satisfaction, and cargo search/goal-model support now treats `MoveCargo` as a real searchable goal. Merchant restock continuity now requires delivery to the relevant destination market rather than remote acquisition alone.
- Deviations from original plan: the work was delivered incrementally through the HARCARGOACON ticket series, with the earlier goal-identity slice landing before the remaining read-helper, candidate-generation, search, and verification slices. No runtime dirty-snapshot exception path was added; continuity stayed goal-based as intended.
- Verification results: repository tests now cover cargo goal emission from local stock plus destination demand, suppression for remote stock and zero-deliverable cases, commodity-based ranking, destination-aware satisfaction, searchable cargo plans across partial pickup/materialization, agent-tick continuity, and transport integration verification.
