# E12: Combat, Wounds & Healing

**Status**: COMPLETED

## Epic Summary
Implement combat actions, a unified wound system, natural stabilization / healing, death with finality, and corpse handling. There is **no stored Health component**. Agent bodily condition is derived from `WoundList` plus current physiological state.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (actions and scheduler)
- E09 (HomeostaticNeeds, WoundList/Wound shared schema in `worldwake-core` — combat wounds and deprivation wounds use the same data model)
- E10 (InTransitOnEdge — used for transit checks in co-location preconditions)
- Shared body-harm schema in `worldwake-core` so combat wounds and deprivation wounds use the same data model

## Foundations Alignment Changes
This revision fixes three important gaps:

1. **Wounds are generalized bodily harm, not combat-only damage.** The same carrier must support deprivation harm from E09 and future non-combat injury.
2. **Wounds now stabilize and heal naturally.** Without this, the system lacks a physical dampener and fills the world with permanent one-way damage unless a medic intervenes.
3. **Bleeding progression must be per-wound state, not a hand-wavy global decrement.**

## Deliverables

### CombatProfile Component
Per-agent combat / bodily resilience parameters enabling Principle 11.

```rust
pub struct CombatProfile {
    pub wound_capacity: Permille,
    pub incapacitation_threshold: Permille,
    pub attack_skill: Permille,
    pub guard_skill: Permille,
    pub defend_bonus: Permille,            // added to guard_skill when Defend active
    pub natural_clot_resistance: Permille, // how quickly bleed_rate decreases per tick
    pub natural_recovery_rate: Permille,   // severity reduction per tick under recovery conditions
    pub unarmed_wound_severity: Permille,  // base wound from unarmed attacks
    pub unarmed_bleed_rate: Permille,      // bleed rate from unarmed attacks
    pub unarmed_attack_ticks: NonZeroU32,  // duration for unarmed attack action
}
```

`attack_skill` / `guard_skill` replace the earlier `base_attack` / `base_defense` wording. They are still numeric, but they are explicitly traits of the body / training rather than floating anonymous formula knobs.

### WoundList Component
`WoundList` is a `Vec<Wound>` stored on the agent entity. It already exists in `crates/worldwake-core/src/wounds.rs` (registered in E09). E12 extends the `Wound` struct with a bleeding field.

```rust
struct Wound {
    pub body_part: BodyPart,              // Head, Torso, LeftArm, RightArm, LeftLeg, RightLeg
    pub cause: WoundCause,
    pub severity: Permille,
    pub inflicted_at: Tick,
    pub bleed_rate_per_tick: Permille,    // 0 for non-bleeding wounds, 0 once clotted/treated
}
```

E09 deprivation wounds pass `Permille(0)` for `bleed_rate_per_tick`. Elapsed time is derived as `current_tick - inflicted_at`, so no `ticks_since_inflicted` field is stored.

### WoundCause Extension
`WoundCause` currently has `Deprivation(DeprivationKind)` (from E09 shared schema). E12 adds:
- `Combat { attacker: EntityId, weapon: CombatWeaponRef }`

New enum `CombatWeaponRef`:

```rust
pub enum CombatWeaponRef {
    Unarmed,
    Commodity(CommodityKind),     // Sword, Bow
}
```

All types derive `Copy + Clone + Eq + Ord + Hash + Serialize + Deserialize`.

### Weapon Commodities
Add to `CommodityKind` in `crates/worldwake-core/src/items.rs`:
- `Sword` -- trade_category: `TradeCategory::Weapon`, melee weapon
- `Bow` -- trade_category: `TradeCategory::Weapon`, ranged weapon (for Phase 2, still requires Place co-location)

Each weapon needs a `CombatWeaponProfile` (new struct in `worldwake-core`):

```rust
pub struct CombatWeaponProfile {
    pub base_wound_severity: Permille,
    pub base_bleed_rate: Permille,
    pub attack_duration_ticks: NonZeroU32,
}
```

Access via `CommodityKind::combat_weapon_profile() -> Option<CombatWeaponProfile>`.
Unarmed combat uses per-agent parameters from `CombatProfile`.

### No Stored Health Component
Authoritative checks are always derived:

- **Wound load** = sum of `wound.severity`
- **Incapacitated** = `wound_load >= incapacitation_threshold`
- **Dead** = `wound_load >= wound_capacity`

No `health`, `hit_points`, or duplicate aggregate score may be stored.

### Combat Actions

#### Attack
- Precondition:
  - attacker and target co-located at the same Place (neither in transit)
  - attacker alive
  - attacker not incapacitated
  - attacker has weapon or can fight unarmed
- Duration: derived from weapon profile or unarmed profile
- Effect:
  - resolve hit / guard outcome
  - append wound(s) to target
  - emit public combat event
- Visibility: public at the Place where combat occurs
- Witnesses: all co-located agents there

#### Defend
- Precondition: actor alive, not incapacitated, at a Place (not in transit)
- Effect: while active, agent's effective `guard_skill` is boosted by `CombatProfile.defend_bonus`. Hit resolution checks for active Defend action on target.
- Duration: `DurationExpr::Indefinite` -- runs until cancelled or interrupted
- Interruptibility: `FreelyInterruptible`
- Payload: `ActionPayload::None`
- Visibility: public at Place

### Hit Resolution
Deterministic given RNG state.

Inputs may include:
- weapon wound profile
- `attack_skill`
- `guard_skill`
- fatigue from E09
- existing wound penalties from `WoundList`

The output is not "health damage." It is one or more new `Wound` values appended to `WoundList`.

### Per-Wound Progression
Each wound progresses independently each tick.

- if `bleed_rate_per_tick > 0`, increase `severity` by that rate
- once clotting / treatment occurs, `bleed_rate_per_tick` becomes `0`
- elapsed time is derived as `current_tick - wound.inflicted_at`
- non-bleeding wounds can slowly recover if recovery conditions are met

This must be implemented per wound, not as a global health drain.

### Natural Stabilization & Recovery
Phase 2 must include low-fidelity natural recovery.

A wound may naturally improve when:
- it is not actively bleeding
- the agent is alive
- the agent is not in immediate combat
- the agent has at least minimally tolerable hunger / thirst / fatigue

Natural clotting reduces `bleed_rate_per_tick` based on elapsed time (`current_tick - wound.inflicted_at`) and `CombatProfile.natural_clot_resistance`. This is a physical world process (blood coagulation), not a numerical clamp. Higher `natural_clot_resistance` means faster natural clotting. Once `bleed_rate_per_tick` reaches zero, the wound transitions to the recovery phase where `severity` decreases at `CombatProfile.natural_recovery_rate` per tick under acceptable conditions.

Medicine accelerates and improves this process, but medicine is not the sole path to recovery.

This is the physical dampener for the wound-spiral loop.

### Wound Consequences (Cross-System via State)
Wound effects propagate through shared state.

- **Pain**: not stored; future systems may derive pain from `WoundList` as a read-model
- **Bleeding**: wound severity increases per wound
- **Movement restriction**: severe leg wounds restrict travel and pursuit
- **Action restriction**: severe arm wounds hinder weapon use / crafting
- **Incapacitation**: wound load beyond threshold blocks new actions
- **Fatigue interaction**: the combat system reads the `HomeostaticNeeds` component from world state (state-mediated per Principle 12)

### Healing Action
- **Heal**: agent treats wounded target
- Precondition:
  - healer and target co-located
  - target has wound(s)
  - healer has medicine / bandage
- Effect:
  - reduce `bleed_rate_per_tick` immediately or over early treatment ticks
  - reduce wound severity over treatment duration
  - consume medicine
- Duration: derived from medicine profile and wound severity
- Result: treated wound becomes more stable and heals faster

### Death
Triggered when wound load reaches `wound_capacity`.

When death triggers:
- attach `DeadAt(Tick)` component to the agent
- agent is NOT archived -- retains all components, remains in the world
- scheduler excludes agents with `DeadAt` from planning and action starts
- body persists with inventory (lootable), WoundList, CombatProfile, location
- emit death event with cause chain

New component:

```rust
pub struct DeadAt(pub Tick);
impl Component for DeadAt {}
```

Registered on `EntityKind::Agent`.

### Corpse Handling
Bodies remain in the world (have `DeadAt` but are not archived).

Actions on corpses:
- **Loot** -- transfer items from dead agent to looter
  - Precondition: co-located, target has `DeadAt`, looter alive and not incapacitated
  - Effect: transfer item ownership, subject to carry capacity
  - Duration: derived from item weight
  - Visibility: public at Place

Deferred: Bury, Discover/inspect. No corpse auto-cleanup in Phase 2.

## ActionPayload Extension
Add to `ActionPayload` in `action_payload.rs`:
- `Combat(CombatActionPayload)` -- for Attack action
- `Loot(LootActionPayload)` -- for Loot action

```rust
pub struct CombatActionPayload {
    pub target: EntityId,
    pub weapon: CombatWeaponRef,
}
pub struct LootActionPayload {
    pub target: EntityId,
}
```

## Constraint and Precondition Extensions
New `Constraint` variants (in `action_semantics.rs`):
- `ActorNotIncapacitated` -- wound load below incapacitation_threshold
- `ActorNotDead` -- no `DeadAt` component

New `Precondition` variants (in `action_semantics.rs`):
- `TargetAlive(u8)` -- target lacks `DeadAt`
- `TargetDead(u8)` -- target has `DeadAt` (for Loot)
- `TargetIsAgent(u8)` -- target is `EntityKind::Agent`

## DurationExpr Extensions
New variants in `action_semantics.rs`:
- `Indefinite` -- action runs until cancelled/interrupted (for Defend)
- `CombatWeapon` -- resolves to weapon's `attack_duration_ticks` from `CombatWeaponProfile`, falling back to actor's `CombatProfile.unarmed_attack_ticks` for unarmed

## Component Registration
Components to register in `component_schema.rs`:

- `WoundList` -- **already registered** on Agent (E09). E12 extends Wound struct only.
- `CombatProfile` -- **NEW**, register on `EntityKind::Agent`
- `DeadAt` -- **NEW**, register on `EntityKind::Agent`

## SystemFn Integration
- Implements the combat / wound handler in `SystemDispatch`
- Runs once per tick for active combat actions and wound progression
- Reads:
  - `WoundList`
  - `CombatProfile`
  - weapon profiles (via `CommodityKind::combat_weapon_profile()`)
  - `HomeostaticNeeds`
  - active actions (to check for Defend stance on targets)
  - `DeadAt` (to skip dead agents)
- Writes:
  - `WoundList`
  - `DeadAt` (attach on death)
  - combat events (attack outcome, wound inflicted)
  - death events (with cause chain)

E12 does **not** call E09 or E13 directly.

## Cross-System Interactions (Principle 12)
- **E09 -> E12**: physiology affects combat effectiveness and natural healing quality
- **E09 -> E12**: deprivation can append `WoundCause::Deprivation(...)` entries to `WoundList`
- **E12 -> E13**: AI derives pain / danger / treatment priorities from wounds and co-located threats
- **E12 -> future systems**: corpses and wounds become discoverable material evidence

## FND-01 Section H

### Information-Path Analysis
- Combat is local to the shared Place
- Witnesses are all co-located agents at that Place
- Wounds live on the wounded body
- Death events propagate through ordinary event visibility and later witness systems

### Positive-Feedback Analysis
- **Wounds -> weakness -> more wounds**
- **Bleeding -> more wound load -> less capacity to flee / heal**

### Concrete Dampeners
- **Wounds -> weakness loop dampeners**:
  - physical separation if one side flees
  - incapacitation removing the target from active exchange
  - natural stabilization / healing when combat stops
- **Bleeding loop dampeners**:
  - natural clotting: `bleed_rate_per_tick` decreases based on elapsed time and `CombatProfile.natural_clot_resistance` -- this models blood coagulation (a physical world process, not a numerical clamp)
  - bandaging / healing by co-located agents

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `WoundList`
- `CombatProfile`
- `DeadAt(Tick)`

**Derived (transient read-model)**:
- wound load
- pain pressure
- incapacitation
- death imminence
- current combat disadvantage from wounds

## Invariants Enforced
- 9.14: Death finality -- dead agents do not plan, act, trade, vote, or consume
- 9.5: Conservation -- corpse inventory persists; treatment consumes medicine
- Principle 3: no stored health component
- Principle 6: deterministic combat outcomes given RNG state
- Principle 8: wound loops have physical dampeners through clotting / recovery / separation
- Principle 11: per-agent combat profiles, no magic numbers
- Principle 12: cross-system interaction via shared state only, no direct system calls

## Tests
- [ ] T14: Dead agents generate no new plans or actions
- [ ] Combat resolves deterministically with same RNG state
- [ ] New wounds append to `WoundList` with correct cause and body_part
- [ ] Per-wound bleeding increases severity over time
- [ ] Treatment reduces bleeding and consumes medicine
- [ ] Non-bleeding wounds can naturally stabilize / recover under acceptable physiological conditions
- [ ] Deprivation wounds coexist with combat wounds in the same `WoundList`
- [ ] Death triggers when wound load reaches `wound_capacity`
- [ ] Incapacitation triggers when wound load reaches `incapacitation_threshold`
- [ ] Corpse retains inventory and location context
- [ ] Death event traces back to cause chain
- [ ] Cannot attack dead agents
- [ ] No stored `Health` component exists
- [ ] Different `CombatProfile` values produce different outcomes
- [ ] Durations derive from weapon / medicine profiles, not hardcoded constants
- [ ] `DeadAt` component is attached on death and scheduler excludes agents with it
- [ ] Scheduler excludes agents with `DeadAt` from planning and action starts
- [ ] Loot action transfers items from dead agent to looter
- [ ] Defend action boosts effective `guard_skill` by `defend_bonus`
- [ ] `Sword` and `Bow` exist in `CommodityKind` with `TradeCategory::Weapon`
- [ ] Natural clotting reduces `bleed_rate_per_tick` over time based on `natural_clot_resistance`
- [ ] Recovery only occurs when not bleeding and physiological conditions acceptable
- [ ] Dead agents stop accruing new physiological `BodyCostPerTick`; corpse load/inventory persistence remains enforced through the existing inventory/load model
- [ ] `DurationExpr::Indefinite` keeps Defend running until cancelled
- [ ] `CombatWeaponRef::Commodity(Sword)` produces different wound profile than `Unarmed`

## Acceptance Criteria
- Combat produces wounds, not hit-point subtraction
- Wounds are the unified bodily-harm carrier for combat and deprivation
- No stored health component exists
- Death is final and modeled via `DeadAt(Tick)` component (not archival)
- Corpses persist as interactive entities; only Loot is supported (Bury and Discover deferred)
- Wounds can stabilize and heal naturally; medicine accelerates recovery
- All combat outcomes remain traceable in the event log
- `Sword` and `Bow` exist as weapon commodities
- Defend is an ongoing (indefinite) action boosting `guard_skill`
- **Out of scope**: armor coverage/mitigation, route combat (combat only at Places)

## Route Combat Note
Route combat is deferred to a future epic. Combat in E12 only occurs where co-presence actually exists at a Place. E10's `InTransitOnEdge` is checked to ensure combatants are not in transit, but no combat occurs on routes.

## Spec References
- Section 3.4 (wounds as carriers of consequence)
- Section 4.5 (combat / injury / death, healing)
- Section 7.1 (material propagation: wounds, corpses)
- Section 9.14 (death finality)
- `docs/FOUNDATIONS.md` Principles 3, 6, 8, 11, 12

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Combat, wounds, healing, death, corpse persistence, and loot are implemented across `worldwake-core`, `worldwake-sim`, and `worldwake-systems`.
  - Scheduler-level E12 integration coverage was added in `crates/worldwake-systems/tests/e12_combat_integration.rs`.
  - This archived spec corrects the prior assumption that dead agents should continue accruing physiological `BodyCostPerTick`; death finality instead stops new physiological accrual while corpse load/inventory persistence remains modeled through state.
- Deviations from original plan:
  - Verification is split across focused combat tests, generic scheduler/death tests in `worldwake-sim`, and the final E12 integration file rather than one giant combat-only suite.
- Verification results:
  - `cargo test -p worldwake-systems --test e12_combat_integration`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
