# S44GENCONSUB-007: Attach contention to corpse and care targets

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — contention attachment/cleanup in worldwake-systems combat lifecycle plus minimal corpse/care promotion support in contention_system()
**Deps**: S44GENCONSUB-006

## Problem

The contention substrate now exists at the type and action-validation layers, but the live corpse and care targets still do not get `ContentionQueue` + `ContentionPolicy` attached automatically when they enter those exclusive-access regimes. Dead agents (loot/bury targets) and wounded agents (heal targets) must gain and shed contention state at the correct lifecycle points so `S44GENCONSUB-006`'s lawful queue/grant path is actually reachable in production instead of depending on manual setup.

## Assumption Reassessment (2026-04-04)

1. Dead agents are marked with `DeadAt` component (`crates/worldwake-core/src/combat.rs`). They remain `EntityKind::Agent`. Confirmed.
2. Loot and bury actions target dead agents (`ActionDomain::Corpse`). Confirmed.
3. Heal action targets wounded agents (`ActionDomain::Care`). Confirmed.
4. `S44GENCONSUB-006` already added `queue_for_corpse_use`, `queue_for_care_target`, and grant-gated `loot` / `bury` / `heal` in `crates/worldwake-systems/src/combat.rs`. Those queue/grant paths are only live when the target entity actually carries contention components. Confirmed.
5. The combat fatality path that applies `DeadAt` is in `combat_system()` in `crates/worldwake-systems/src/combat.rs`. This is the correct corpse-attachment boundary. Confirmed.
6. New combat wounds are authored in `commit_attack()` in `crates/worldwake-systems/src/combat.rs`, and passive wound progression/clearance flows through `apply_wound_progression()`. These are the live care-target lifecycle boundaries. Confirmed.
7. The active S44 spec still lists unique items as a Phase 1 contention target, but live transport actions only expose `pick_up` for `EntityKind::ItemLot` in `crates/worldwake-systems/src/transport_actions.rs`. Ticket says Phase 1 includes unique items; live code has no such action path; correction applied: narrow this ticket to corpse and patient attachment, and split unique-item pickup contention into follow-up `S44GENCONSUB-010`; why safe: this preserves FOUNDATIONS P8/P9/P20 by avoiding fake contention attachment for an affordance that does not yet exist lawfully.
8. If a wounded patient dies or heals completely, simply overwriting or removing contention components would leave stale `ContentionIntents` on queued actors. Lifecycle cleanup must be part of this ticket's owned consequence. Confirmed.
9. Live `contention_system()` in `crates/worldwake-systems/src/facility_queue.rs` still only treats facility-exclusive production actions as promotable queue heads. Once corpse/care contention is attached, leaving this logic unchanged would create inert queue state that never matures into grants. Ticket said attachment only; live system contract requires minimal corpse/care promotion support too; correction applied: broaden this ticket to include the smallest contention-system generalization needed for `ActionDomain::Corpse` and `ActionDomain::Care`; why safe: this closes the lawful queue->grant->action chain required by FOUNDATIONS P8/P9/P12/P20/P21 without broadening into unrelated non-facility domains.

## Architecture Check

1. Attaching contention at the combat lifecycle boundaries is cleaner than pre-attaching queues to all agents. The state only exists while corpse/care exclusivity is actually relevant.
2. Because `S44GENCONSUB-006` already made corpse/care queue admission and grant-gated actions live, this ticket must also make those attached queues promotable in the authoritative contention system. Otherwise the new state would be inert substrate, which violates FOUNDATIONS.
3. No backward-compatibility shims.

## Verification Layers

1. Corpse entity after death has `ContentionQueue` + `ContentionPolicy` -> authoritative world state check
2. Wounded agent after wound creation has care contention policy -> authoritative world state check
3. Healed or transitioned entity clears stale care contention state and queued intents -> authoritative world state check
4. Death replaces prior care contention with corpse contention and clears stale queued-healer intents -> authoritative world state check
5. Corpse/care queue heads can promote to grants through `contention_system()` once structurally valid -> authoritative system-state check

## What to Change

### 1. Attach contention to corpses on death

In the combat fatality path, when an agent dies, attach `ContentionQueue::default()` and `ContentionPolicy { grant_hold_ticks: NonZeroU32::new(5).unwrap(), auto_promote: true, max_waiters: None }` to the corpse entity.

If the agent previously carried care contention state, clear the old queue/grant membership and stale queued-healer intents before resetting the corpse to the corpse-specific policy.

### 2. Attach contention to heal targets

When an agent becomes wounded through the live combat lifecycle, ensure it carries `ContentionQueue::default()` and `ContentionPolicy { grant_hold_ticks: NonZeroU32::new(5).unwrap(), auto_promote: true, max_waiters: None }` for one-healer-at-a-time care contention.

### 3. Clear care contention when wounds are no longer present

When an agent is no longer a valid care target because its wound list becomes empty, remove the care contention components and clear any queued/granted actor intent entries tied to that patient.

### 4. Generalize contention promotion just enough for corpse/care

Update `crates/worldwake-systems/src/facility_queue.rs` so the authoritative contention system can recognize corpse and care intended actions as valid promotable queue heads on agent entities, while preserving the existing facility-specific workstation checks for production contention.

Do not broaden this into a generic “all non-facility contention” rewrite. This ticket only owns the minimal promotion/readiness support needed for the corpse/care paths already made live by `S44GENCONSUB-006`.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — corpse/care contention attachment and cleanup lifecycle)
- `crates/worldwake-systems/src/facility_queue.rs` (modify — minimal corpse/care queue promotion support in contention_system())

## Out of Scope

- Unique-item pickup contention (`S44GENCONSUB-010`)
- Phase 2 contention domains (bounty claims, storage, witness time)
- Perception of contention state (`S44GENCONSUB-008`)
- Golden tests proving contention behavior (`S44GENCONSUB-009`)

## Acceptance Criteria

### Tests That Must Pass

1. Agent that dies has `ContentionQueue` and corpse `ContentionPolicy`
2. Corpse `ContentionPolicy` has `auto_promote: true, max_waiters: None`
3. Agent that receives a wound through the live combat path has care contention components
4. Healing or passive wound recovery that empties the wound list clears care contention components and stale queued intents
5. Corpse/care queue heads are not pruned as structurally invalid merely because the target is an `Agent`
6. Existing suite: `cargo test --workspace`

### Invariants

1. Corpse and care contention are only attached when those exclusive affordances become relevant
2. Attached corpse/care contention matures through the same authoritative queue/grant system instead of relying on manual grant seeding or tick-order luck
3. Policy values match the live Phase 1 corpse/care table from the S44 spec

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — contention attachment on death, wound creation, and wound-clearance cleanup
2. `crates/worldwake-systems/src/facility_queue.rs` — corpse/care promotion support through `contention_system()`

### Commands

1. `cargo test -p worldwake-systems combat`
2. `cargo test -p worldwake-ai -- search`
3. `cargo test -p worldwake-ai --test golden_care`
4. `cargo test -p worldwake-ai --test golden_combat`
5. `cargo test -p worldwake-ai --test golden_production`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

## Outcome

Implemented the corpse/patient attachment slice on the broader foundations-aligned boundary the live runtime required.

- `crates/worldwake-systems/src/combat.rs` now attaches and clears corpse/care contention state at the real combat lifecycle boundaries:
  - fatality replaces prior care contention with fresh corpse contention
  - new wounds ensure care contention exists
  - full healing or passive wound clearance removes stale care contention and queued intents
- `crates/worldwake-systems/src/facility_queue.rs` now treats lawful corpse/care intended actions as promotable contention heads, so the queue/grant path from `S44GENCONSUB-006` is reachable in production instead of leaving inert attached state
- `crates/worldwake-ai/src/goal_dispatch_decl.rs`, `crates/worldwake-ai/src/goal_model.rs`, `crates/worldwake-ai/src/planning_state.rs`, and `crates/worldwake-ai/src/search/candidates.rs` were updated so `LootCorpse` and `BuryCorpse` expose queue-first planning lawfully through the planner contract rather than only at the affordance surface
- focused search proof landed in `crates/worldwake-ai/src/search/tests.rs`, and the systems integration path was updated in `crates/worldwake-systems/tests/e12_combat_integration.rs`
- required golden fallout was absorbed honestly in:
  - `crates/worldwake-ai/tests/golden_care.rs`
  - `crates/worldwake-ai/tests/golden_combat.rs`
  - `crates/worldwake-ai/tests/golden_production.rs`
- the unique-item Phase 1 contention claim was split into follow-up `S44GENCONSUB-010` because live transport actions still lacked a lawful ground-`UniqueItem` pickup affordance
