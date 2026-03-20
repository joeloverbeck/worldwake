**Status**: PENDING

# Indefinite Action Re-Evaluation

## Summary

Remove the `DurationExpr::Indefinite` and `ActionDuration::Indefinite` variants entirely from the codebase, enforcing Principle 8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) at the type level. The only current user of `Indefinite` in production code is the `defend` action. Replace it with a new `DurationExpr::ActorDefendStance` variant that reads `defend_stance_ticks` from the actor's `CombatProfile` at action start time — following the same resolution pattern already used by `DurationExpr::CombatWeapon`.

When defend's finite duration expires, the action commits normally. The agent re-enters the decision cycle and either re-selects defend (if danger persists) or switches to a different goal (if the threat resolved). No future action can be indefinite.

## Discovered Via

Golden E2E emergent tests (S07 care interaction coverage). A pre-wounded fighter near a hostile target selected `ReduceDanger → defend` as its highest-priority goal. Defend ran indefinitely — the fighter never attacked, never looted, never self-healed. The agent was permanently deadlocked in a defensive stance against a target it could have killed.

## Foundation Alignment

- **Principle 8** (Every Action Has Preconditions, Duration, Cost, and Occupancy): The defend action currently violates the duration requirement — it has no defined endpoint. After this change, no action in the system can be indefinite; the type system enforces this.
- **Principle 19** (Intentions Are Revisable Commitments): "Agents must monitor the assumptions beneath an active intention and suspend, revise, or replace that intention when new local evidence invalidates it." An agent defending against a dead target is not monitoring its assumptions.
- **Principle 1** (Maximal Emergence): Emergent behavior requires agents to respond to *current* state. An agent stuck in a stale action cannot participate in emergence.
- **Principle 18** (Resource-Bounded Practical Reasoning): Indefinite actions bypass the decision cycle entirely. The agent never re-evaluates whether defending is still the best use of its time.
- **Principle 20** (Agent Diversity): The defend duration becomes a per-agent parameter on `CombatProfile`, allowing different agents to hold defensive stances for different periods.

## Phase

Phase 3: Information & Politics (design fix, no phase dependency)

## Crates

- `worldwake-core` (add `defend_stance_ticks` to `CombatProfile`)
- `worldwake-sim` (remove `Indefinite` from `DurationExpr` and `ActionDuration`, add `ActorDefendStance`, update `belief_view.rs`)
- `worldwake-systems` (change defend `ActionDef` to use `DurationExpr::ActorDefendStance`)
- `worldwake-ai` (remove planner's `Indefinite` special-casing in `search.rs`)
- `worldwake-cli` (remove `Indefinite` display branches)

## Dependencies

None. All prerequisite infrastructure exists.

## Design

### New `DurationExpr::ActorDefendStance` Variant

Add a new variant to `DurationExpr` in `action_semantics.rs`:

```rust
pub enum DurationExpr {
    Fixed(NonZeroU32),
    TargetConsumable { target_index: u8 },
    TravelToTarget { target_index: u8 },
    ActorMetabolism { kind: MetabolismDurationKind },
    ActorTradeDisposition,
    ActorDefendStance,   // NEW — replaces Indefinite
    CombatWeapon,
    TargetTreatment { target_index: u8, commodity: CommodityKind },
}
```

This follows the existing pattern where `CombatWeapon` reads from `CombatProfile` and `ActionPayload` at resolution time. `ActorDefendStance` reads `defend_stance_ticks` from the actor's `CombatProfile`.

### Resolution in `DurationExpr::resolve_for()`

The resolution function in `action_semantics.rs` currently handles `CombatWeapon` by reading `CombatProfile`:

```rust
// Existing pattern (CombatWeapon):
Self::CombatWeapon => {
    let combat = payload.as_combat().ok_or_else(|| ...)?;
    match combat.weapon {
        CombatWeaponRef::Unarmed => world
            .get_component_combat_profile(actor)
            .map(|profile| ActionDuration::Finite(profile.unarmed_attack_ticks.get()))
            .ok_or_else(|| format!("actor {actor} lacks combat profile")),
        ...
    }
}
```

The new variant follows the same pattern:

```rust
// New (ActorDefendStance):
Self::ActorDefendStance => world
    .get_component_combat_profile(actor)
    .map(|profile| ActionDuration::Finite(profile.defend_stance_ticks.get()))
    .ok_or_else(|| format!("actor {actor} lacks combat profile")),
```

### Resolution in `estimate_duration_from_beliefs()`

The belief-view estimation function in `belief_view.rs` (line 678) needs a matching arm:

```rust
// Current:
DurationExpr::Indefinite => Some(ActionDuration::Indefinite),

// Replace with:
DurationExpr::ActorDefendStance => view
    .combat_profile(actor)
    .map(|profile| ActionDuration::Finite(profile.defend_stance_ticks.get())),
```

This allows the planner to cost defend actions from beliefs, following the same pattern as `CombatWeapon` (line 715–725 in the same function).

### New Field on `CombatProfile`

Add `defend_stance_ticks: NonZeroU32` to `CombatProfile` in `worldwake-core/src/combat.rs`:

```rust
pub struct CombatProfile {
    pub wound_capacity: Permille,
    pub incapacitation_threshold: Permille,
    pub attack_skill: Permille,
    pub guard_skill: Permille,
    pub defend_bonus: Permille,
    pub natural_clot_resistance: Permille,
    pub natural_recovery_rate: Permille,
    pub unarmed_wound_severity: Permille,
    pub unarmed_bleed_rate: Permille,
    pub unarmed_attack_ticks: NonZeroU32,
    pub defend_stance_ticks: NonZeroU32,  // NEW — default nz(10)
}
```

The `CombatProfile::new()` constructor gains an 11th parameter. All existing call sites must be updated.

### Remove `ActionDuration::Indefinite`

Remove the `Indefinite` variant from `ActionDuration` in `action_duration.rs`. After this change, `ActionDuration` becomes:

```rust
pub enum ActionDuration {
    Finite(u32),
}
```

This enforces Principle 8 at the type level — no code path can produce an indefinite action. The `advance()` method simplifies (no `Indefinite` branch). The `fixed_ticks()` method always returns `Some`.

Note: If a single-variant enum is undesirable, the alternative is to keep `ActionDuration` as a newtype `ActionDuration(u32)`. Either way, the `Indefinite` variant is removed.

### Planner Cost Integration

Currently in `search.rs` (line 392–396), the planner special-cases `Indefinite`:

```rust
let estimated_ticks = match duration {
    ActionDuration::Finite(ticks) => ticks,
    ActionDuration::Indefinite if semantics.may_appear_mid_plan => return None,
    ActionDuration::Indefinite => 0,
};
```

After removing `Indefinite`, this simplifies to:

```rust
let estimated_ticks = match duration {
    ActionDuration::Finite(ticks) => ticks,
};
```

The planner now gets a real cost estimate for defend via `ActorDefendStance` → `defend_stance_ticks`. No more 0-tick hack for leaf actions or rejection for mid-plan appearances.

## Deliverables

### 1. Add `defend_stance_ticks` to `CombatProfile`

New field on `CombatProfile` in `worldwake-core/src/combat.rs`. Default value: `nz(10)` (10 ticks per defend stance).

Update `CombatProfile::new()` to accept the new parameter (11th argument).

**Call sites to update** (39 occurrences across 14 files):

| File | Count | Context |
|------|-------|---------|
| `crates/worldwake-core/src/combat.rs` | 1 | `sample_combat_profile()` test helper |
| `crates/worldwake-core/src/world.rs` | 1 | Test helper |
| `crates/worldwake-core/src/delta.rs` | 1 | Test helper |
| `crates/worldwake-core/src/component_tables.rs` | 2 | Test helpers |
| `crates/worldwake-core/src/wounds.rs` | 1 | Test helper |
| `crates/worldwake-sim/src/action_semantics.rs` | 1 | `DurationExpr` resolution test |
| `crates/worldwake-sim/src/action_validation.rs` | 1 | Action validation test |
| `crates/worldwake-sim/src/start_gate.rs` | 1 | Start gate test |
| `crates/worldwake-systems/src/combat.rs` | 7 | Defend handler test + combat tests |
| `crates/worldwake-systems/src/office_actions.rs` | 2 | Office action tests |
| `crates/worldwake-systems/tests/e12_combat_integration.rs` | 2 | Integration tests |
| `crates/worldwake-ai/src/goal_model.rs` | 2 | Goal model tests |
| `crates/worldwake-ai/src/plan_revalidation.rs` | 1 | Plan revalidation test |
| `crates/worldwake-ai/src/search.rs` | 1 | Search test |
| `crates/worldwake-ai/tests/golden_harness/mod.rs` | 1 | Golden test harness default |
| `crates/worldwake-ai/tests/golden_combat.rs` | 4 | Combat golden tests |
| `crates/worldwake-ai/tests/golden_emergent.rs` | 3 | Emergent golden tests |
| `crates/worldwake-ai/tests/golden_production.rs` | 1 | Production golden tests |
| `crates/worldwake-ai/tests/golden_offices.rs` | 1 | Office golden tests |

All call sites should use `nz(10)` as the new argument unless a specific test requires a different value.

### 2. Add `DurationExpr::ActorDefendStance` and remove `DurationExpr::Indefinite`

In `crates/worldwake-sim/src/action_semantics.rs`:
- Remove the `Indefinite` variant from the `DurationExpr` enum
- Add `ActorDefendStance` variant
- Update `fixed_ticks()` — `ActorDefendStance` returns `None` (resolved at action start)
- Update `resolve_for()` — reads `defend_stance_ticks` from actor's `CombatProfile`
- Update the `ALL_DURATION_EXPRS` test array (replace `Indefinite` with `ActorDefendStance`)
- Update `resolve_for` tests: remove `Indefinite` → `ActionDuration::Indefinite` assertion, add `ActorDefendStance` → `ActionDuration::Finite(defend_stance_ticks)` assertion
- Update bincode roundtrip tests

### 3. Remove `ActionDuration::Indefinite`

In `crates/worldwake-sim/src/action_duration.rs`:
- Remove the `Indefinite` variant from the `ActionDuration` enum
- Simplify `fixed_ticks()` — always returns `Some(ticks)`
- Simplify `advance()` — remove the `Indefinite => false` arm
- Remove `indefinite_duration_never_auto_completes` test
- Update `action_duration_roundtrips_through_bincode` test

### 4. Update `estimate_duration_from_beliefs()`

In `crates/worldwake-sim/src/belief_view.rs` (line 714):
- Replace `DurationExpr::Indefinite => Some(ActionDuration::Indefinite)` with `DurationExpr::ActorDefendStance` arm that reads `defend_stance_ticks` from `view.combat_profile(actor)`

### 5. Change defend action definition

In `crates/worldwake-systems/src/combat.rs` (line 399):
- Change `duration: DurationExpr::Indefinite` to `duration: DurationExpr::ActorDefendStance`
- Update the test at line 1613 that asserts `DurationExpr::Indefinite`

### 6. Clean up `start_gate.rs` reservation range

In `crates/worldwake-sim/src/start_gate.rs` (line 202–205):
- Remove the `ActionDuration::Indefinite => Err(...)` branch from `reservation_range()`
- This becomes dead code once `ActionDuration::Indefinite` is removed (the compiler will enforce this)
- Update test at line 619 that constructs `ActionDuration::Indefinite`
- Update test at line 646 that sets `DurationExpr::Indefinite`

### 7. Clean up planner search

In `crates/worldwake-ai/src/search.rs` (line 394–395):
- Remove the two `ActionDuration::Indefinite` arms from the `estimated_ticks` match
- The match simplifies to extracting `ticks` from `ActionDuration::Finite(ticks)`

### 8. Clean up remaining `Indefinite` references

| File | Line(s) | Change |
|------|---------|--------|
| `crates/worldwake-sim/src/tick_action.rs` | 535, 852 | Remove test code using `DurationExpr::Indefinite` and `ActionDuration::Indefinite` |
| `crates/worldwake-sim/src/affordance_query.rs` | 719, 1217, 1324 | Remove `ActionDuration::Indefinite` fallback in affordance cost estimation; remove test `DurationExpr::Indefinite` usages |
| `crates/worldwake-sim/src/trade_valuation.rs` | 529 | Remove `.or(Some(ActionDuration::Indefinite))` fallback — if duration can't be estimated, return `None` |
| `crates/worldwake-cli/src/handlers/tick.rs` | 103 | Remove `ActionDuration::Indefinite => "indefinite"` display branch |
| `crates/worldwake-cli/src/handlers/world_overview.rs` | 127 | Remove `ActionDuration::Indefinite => String::new()` display branch |
| `crates/worldwake-systems/src/combat.rs` | 2876, 2904 | Update tests asserting `ActionDuration::Indefinite` to expect `ActionDuration::Finite(10)` (or whatever `defend_stance_ticks` the test profile uses) |

### 9. Update CLAUDE.md

Update the `action_duration` module description in `CLAUDE.md` line 109:
- Change "Finite or Indefinite" to "resolved runtime duration for active actions (always finite)"

### 10. Tests

**Unit test** (worldwake-sim): `DurationExpr::ActorDefendStance` resolves to `ActionDuration::Finite(defend_stance_ticks)` when actor has `CombatProfile`, fails when actor lacks `CombatProfile`.

**Unit test** (worldwake-systems): Defend action with `defend_stance_ticks: nz(5)` completes after 5 ticks.

**Golden test** (worldwake-ai): Agent defending against a hostile target that dies mid-defend naturally re-evaluates after defend expires and switches to a different goal (loot, self-care, or idle).

**Regression**: All existing combat golden tests must still pass. The `golden_reduce_danger_defensive_mitigation` test may need its tick budget adjusted if it relies on indefinite defend duration.

**Planner test**: Verify that `search_plan` produces a real cost estimate for defend (equal to `defend_stance_ticks`) instead of the previous 0-tick hack.

## Edge Cases

### Short durations (1–2 ticks)
An agent with `defend_stance_ticks: nz(1)` would re-evaluate every tick. This is valid — it means the agent is extremely cautious and constantly reassessing. The `FreelyInterruptible` flag on defend already allows interruption, so rapid cycling creates no new problems. Performance impact is negligible since the decision pipeline already runs per-tick for all agents.

### Long durations (100+ ticks)
An agent with very high `defend_stance_ticks` holds a long defensive stance. This is still finite — the agent will eventually re-evaluate. The `FreelyInterruptible` flag means the interrupt system can still force re-evaluation if a higher-priority goal appears. Long durations create the same emergence deadlock risk as `Indefinite`, but the guarantee that the action *eventually* expires prevents permanent deadlock.

### Missing `CombatProfile`
If an agent without a `CombatProfile` attempts to defend:
- `DurationExpr::ActorDefendStance::resolve_for()` returns `Err("actor {id} lacks combat profile")`
- Action start fails at the `start_gate` level
- `estimate_duration_from_beliefs()` returns `None`, so the planner cannot cost the action and will not include it in plans

This is correct behavior — an agent without combat capabilities should not be able to defend.

### Renewal boundary gap
When defend expires, there is a brief window (one decision cycle) where the agent is not in a defensive stance. This is intentional and creates emergence:
- Attackers can exploit the timing gap (realistic combat dynamics)
- The agent might decide *not* to re-defend (threat resolved, better priorities emerged)
- The `CombatStance::Defending` component is removed when the action commits, so the window is real and observable

## Information-Path Analysis (FND-01 Section H)

**Information path**: `defend_stance_ticks` is a static per-agent parameter on `CombatProfile`. It does not propagate through events or perception — it is read directly by the action framework at action start time (authoritative) and by the planner via `BeliefView` (belief-local). No new information paths are introduced.

**Positive-feedback analysis**: No new feedback loops introduced. The finite defend actually *breaks* a potential deadlock loop (defend → danger persists → defend again → forever) by forcing periodic re-evaluation. Each renewal cycle allows the agent to observe that the threat may have resolved.

**Concrete dampeners**: N/A — no amplifying loops exist in this change.

## Stored State vs. Derived

- **Stored**: `defend_stance_ticks` field on `CombatProfile` (authoritative per-agent parameter)
- **Derived**: Resolved `ActionDuration::Finite(n)` at action start time from `defend_stance_ticks` — transient, tracked by the scheduler's action instance
- **Removed**: `ActionDuration::Indefinite` variant — no longer representable in the type system
