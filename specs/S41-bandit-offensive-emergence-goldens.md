# S41: Bandit Offensive Emergence — Golden E2E Coverage

**Status**: DRAFT

## Summary

Three golden E2E suites covering the offensive/economic half of E18 bandit dynamics. The existing T22 suite proves the *defensive* chain (external attack → camp destruction → regrouping → re-establishment). Reassessment against the live code shows Suite 2's route-danger and social-transport substrate already exists, while Suite 1's scarcity-driven raid motive and Suite 3's wound-only raid dampening still require architecture alignment before the final intended goldens can truthfully claim those behaviors.

## Motivation

E18 implemented a rich emergent system — bandit camps where raids, looting, route danger, and merchant adaptation all arise from interacting subsystems without scripted triggers. T22 (2 tests) validates only the destruction/regrouping path. The following E18 behaviors have zero golden coverage:

| Behavior | Systems Involved | Why It Matters |
|----------|-----------------|----------------|
| Bandit-initiated raids (`RaidTarget` goal) | AI candidate gen, Combat, Loot | Core offensive behavior — bandits never raid in any golden test |
| Supply pressure driving raid motivation | Needs, AI ranking, Combat | Intended offensive emergence, but the live `RaidTarget` ranking path no longer uses hunger/enterprise motive directly |
| Merchant route adaptation from raid beliefs | Beliefs, Enterprise, Travel, Trade | Downstream economic impact from information locality |
| Wound accumulation dampening raid frequency | Combat, Wounds, AI pressure | Physical dampener (FND-10) — never golden-tested for any system |

## Crate

`worldwake-ai` (golden tests in `crates/worldwake-ai/tests/`)

## Dependencies

All completed:
- E18 (bandit dynamics — `RaidTarget`, `EstablishBanditCamp`, `RegroupWithFaction`, `BanditCamp`, `BanditFactionPolicy`)
- E12 (combat — `CombatProfile`, `WoundList`, attack/loot actions)
- E13 (decision architecture — GOAP planner, pressure system, candidate generation)
- E14/E15 (beliefs, perception — danger beliefs, `BelievedActivity`)
- E11 (trade — `MerchandiseProfile`, `DemandMemory`, enterprise signals)

---

## Foundational Alignment

| Principle | How These Suites Validate Compliance |
|-----------|-------------------------------------|
| FND-1 (Maximal Emergence) | Raids emerge from hunger pressure + co-located targets, not scripted raid schedules or encounter probability |
| FND-2 (No Ungrounded Triggers) | No `raidChance` or `encounterRate` — raid candidate emission requires concrete co-location of bandit and non-faction target |
| FND-3 (Concrete State Over Abstract Scores) | Route danger assessed from agent beliefs about observed threats, never stored on edges |
| FND-7 (Locality) | Merchant learns about raids through perception or Tell, not global query. Information travels by physical carrier with delay |
| FND-10 (Physical Dampeners) | Intended target: wound accumulation should limit raid frequency through concrete physical state, not numeric cooldowns or caps |
| FND-12 (World State != Belief State) | Merchant acts on stale or propagated beliefs, not omniscient knowledge of bandit positions |
| FND-17 (Agent Symmetry) | Bandits use the same attack action, combat system, and AI planner as all other agents |
| FND-24 (Systems Interact Through State) | Intended chain: needs scarcity and combat aftermath should interact through shared state rather than direct system calls |
| FND-25 (Derived Summaries Are Caches) | Route danger estimates are planning heuristics derived from beliefs, never authoritative stored state |

---

## Deliverables

### Suite 1: Pressure-Driven Raid Emergence (Scenario 47)

**File**: `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (extend existing file)

**Tests**: `golden_pressure_driven_raid_emergence` + `golden_pressure_driven_raid_emergence_replays_deterministically`

#### Setup

- 3-place topology: BanditCamp, RoadJunction, SafeVillage
- 3 bandit agents at BanditCamp with:
  - `BanditFactionPolicy` on their faction
  - `CombatProfile` (moderate — can win but sustain wounds)
  - `HomeostaticNeeds` with elevated hunger (hunger ≥ 700 permille)
  - `MetabolismProfile` with non-zero `hunger_rate` so hunger continues rising
  - `PerceptionProfile` for observation
  - `UtilityProfile` with non-zero `danger_weight` and low `courage`
- Active `BanditCamp` component on the camp place with faction-owned supply container
- Camp supply container nearly empty (1 Bread remaining — insufficient for 3 agents)
- 1 non-faction traveler agent at BanditCamp (or arriving via travel) with:
  - Food items in personal inventory (Apple × 4)
  - `CombatProfile` (weak — will likely lose)
  - `PerceptionProfile`
- `ResourceSource` at BanditCamp (OrchardRow with Apple) to verify bandits prefer raiding when co-located target exists vs. harvesting when alone

#### Proves

1. **Candidate generation**: At least one bandit generates `GoalKind::RaidTarget { target: traveler }` as a goal candidate when the non-faction traveler is co-located. Verified via decision traces: `candidates.generated` contains `RaidTarget`.
2. **Goal selection**: Under the live code, the raid goal only ranks non-zero when the raider has a lawful hostility/danger surface for the target. The eventual golden must either prove that live path explicitly or land a follow-up architecture change that restores proactive scarcity-driven raid motive before claiming hunger-selected raids.
3. **Combat execution**: The raid resolves through the standard `attack` action lifecycle — Started → Committed. Verified via action traces: `ActionTraceKind::Committed` for `"attack"` by a bandit targeting the traveler.
4. **Post-combat loot**: After combat victory, the surviving bandit generates `LootCorpse` goal and executes loot action. Verified via action traces: `"loot"` committed by a bandit.
5. **Conservation**: Total commodity quantity (Apple + Bread) is conserved across the raid-loot chain. Verified via `verify_live_lot_conservation()`.
6. **No scripted trigger**: No raid occurs before the traveler arrives (bandits do other things — harvest, consume). Verified via decision traces in pre-arrival ticks showing non-raid goal selections.

#### Chain

```
camp supplies low → bandit remains motivated to seek supplies →
traveler arrives at camp → perception observes non-faction agent →
candidate_generation emits RaidTarget → live ranking currently requires a lawful hostility/danger surface for RaidTarget to rank non-zero →
plan search finds Attack plan →
attack action starts → combat resolves → loot action → supplies acquired
```

**Causal depth**: 5 (supply pressure / target availability / hostility surface -> raid selection -> combat -> loot -> supply state change)

**Systems**: Needs, AI (candidate gen + ranking + search), Combat, Corpse/Loot, Conservation

#### Metadata Annotation

```
// Scenario 47: Pressure-Driven Raid Emergence
//
// Systems: Needs, AI, Combat, Loot, Conservation
// GoalKinds: RaidTarget, LootCorpse, ConsumeOwnedCommodity
// ActionDomains: Combat, Corpse, Needs
// Places: BanditCamp, RoadJunction, SafeVillage
// Principles: 1, 5, 17, 24, Maximal Emergence, Conservation
```

#### FND-01 Section H Analysis

**Information-path**: Bandit perceives traveler through direct observation (co-located). Hunger state is internal authoritative component. No global queries involved.

**Positive-feedback loop**: Successful raid → more supplies → sustained camp → more bandits healthy → more raids. This is the intended amplifying loop, but the live ranking substrate for `RaidTarget` should be checked against that claim before implementation.

**Concrete dampener**: Intended target is combat wounds. Each raid exposes the raiding bandit to injury. Reassessment shows the live code does not currently let wounds alone dampen raids once hostiles disappear, so Suite 3 remains an explicit engine-gap item.

**Stored state vs. derived**: `HomeostaticNeeds.hunger` (stored), `WoundList` (stored), `BanditCamp.supplies` (stored container). Raid target eligibility is derived from co-location + faction membership queries at candidate-generation time.

---

### Suite 2: Raid-Belief Economic Cascade (Scenario 48)

**File**: `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (extend existing file)

**Tests**: `golden_raid_belief_economic_cascade` + `golden_raid_belief_economic_cascade_replays_deterministically`

#### Setup

- 4-place topology: Market, DangerousRoad, BanditCamp, SafeRoute (longer path from Market to the same supplier destination, e.g., RemoteFarm)
- Market → DangerousRoad: 1 tick; DangerousRoad → BanditCamp: 1 tick
- Market → SafeRoute → RemoteFarm: 4 ticks (longer but avoids bandits)
- DangerousRoad → RemoteFarm: 1 tick (through the danger zone)
- 2 bandit agents at DangerousRoad with:
  - Active `BanditCamp` on BanditCamp place, faction policy
  - `CombatProfile`, `PerceptionProfile`, elevated hunger
- 1 witness agent at DangerousRoad (non-faction, non-merchant) who will observe the raid
- 1 merchant agent at Market with:
  - `MerchandiseProfile` with `sale_kinds` including Apple
  - `DemandMemory` showing demand for Apple at Market
  - Restock source beliefs pointing to RemoteFarm (reachable via DangerousRoad or SafeRoute)
  - `PerceptionProfile`, `UtilityProfile` with non-zero `danger_weight`
  - NO initial danger beliefs about DangerousRoad
- 1 traveler-victim at DangerousRoad for bandits to raid
- ResourceSource at RemoteFarm producing Apple

#### Proves

1. **Raid occurs**: Bandits raid the co-located traveler. Verified via action traces: `"attack"` committed by a bandit at DangerousRoad.
2. **Witness forms danger belief**: The witness agent, co-located during the raid, forms a `BelievedActivity { action_domain: ActionDomain::Combat }` belief about a bandit. Verified via belief store inspection after the raid tick.
3. **Belief propagation to merchant**: The witness travels to Market and executes `Tell` to the merchant, propagating the danger belief. Alternatively, the merchant travels through DangerousRoad and directly observes aftermath. Verified via action traces: `"tell"` committed with `listener: merchant` and `subject: bandit`, OR merchant belief store contains danger belief after traversal.
4. **Merchant route adaptation**: After acquiring the danger belief, the merchant's next restock plan avoids DangerousRoad — selecting the longer SafeRoute instead. This depends on the already-live route-threat substrate in `route_threat.rs` / `planning_snapshot.rs`, which T22 already partially proves for traveler rerouting.
5. **No omniscient rerouting**: Before acquiring the danger belief, the merchant selects the shorter DangerousRoad route (if tested in the initial tick). Verified via decision traces in pre-belief ticks.

#### Chain

```
bandits raid traveler at DangerousRoad → witness perceives combat →
witness travels to Market → witness Tells merchant about danger →
merchant acquires danger belief about DangerousRoad →
merchant's next restock plan avoids DangerousRoad →
merchant selects longer SafeRoute for trade trip
```

**Causal depth**: 5 (raid → perception → belief propagation → route replanning → economic behavior change)

**Systems**: Combat/Raid, Perception, Beliefs, Social Tell, Enterprise, Travel, AI

#### Metadata Annotation

```
// Scenario 48: Raid-Belief Economic Cascade
//
// Systems: Combat, Perception, Beliefs, Social Tell, Enterprise, Travel, AI
// GoalKinds: RaidTarget, ShareBelief, RestockCommodity
// ActionDomains: Combat, Social, Travel, Production
// Places: Market, DangerousRoad, BanditCamp, SafeRoute, RemoteFarm
// Principles: 3, 7, 12, 25, Maximal Emergence
```

#### FND-01 Section H Analysis

**Information-path**: Raid → witness perceives at same place → witness travels physically to Market → Tell action at Market → merchant receives belief through social Tell. Every hop is traceable. No global broadcast. This reuses the existing generic `ShareBelief` / `TellTopic::EntityBelief` transport path rather than a combat-only exception.

**Positive-feedback loop**: None in this suite's scope. The merchant's avoidance does not amplify the raid frequency.

**Concrete dampener**: N/A for this suite — the dampener (route avoidance reducing bandit target availability) is a cross-suite property. Within this suite, the chain is linear (cause → effect), not cyclic.

**Stored state vs. derived**: `BelievedActivity` in merchant's belief store (stored after Tell). Route danger cost in planner is derived from beliefs at planning time, never stored as authoritative edge weight. `MerchandiseProfile.sale_kinds` (stored), `DemandMemory` (stored), restock gap (derived).

---

### Suite 3: Wound-Dampened Raid Spiral (Scenario 49)

**File**: `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (extend existing file)

**Tests**: `golden_wound_dampened_raid_spiral` + `golden_wound_dampened_raid_spiral_replays_deterministically`

#### Setup

- 2-place topology: BanditCamp, RoadJunction
- 2 bandit agents at BanditCamp with:
  - `BanditFactionPolicy` on faction
  - `CombatProfile` (can win but always sustain wounds — moderate skill, low guard)
  - `HomeostaticNeeds` with chronic hunger (hunger_rate non-zero, limited food supply)
  - `MetabolismProfile` with non-zero hunger_rate
  - `PerceptionProfile`
  - `UtilityProfile` with `danger_weight` ≥ 700 permille and low `courage` (≤ 200 permille)
  - NO healing items or medicine available
- Active `BanditCamp` component with minimal supplies
- A sequence of 2-3 non-faction targets arriving at BanditCamp on successive ticks (seeded via `ControlSource::None` → `ControlSource::Ai` activation at specific ticks, or travel arrivals)
- Targets have weak `CombatProfile` (bandits win but take hits)
- No wound recovery mechanism — no medicine, no healer, no `TreatWounds` capability

#### Proves

1. **Current gap**: Under the live code, a victorious bandit with lingering wounds but no current attackers / visible hostiles does not gain `danger_pressure` from those wounds alone.
2. **Current gap**: `derive_pain_pressure()` only feeds `TreatWounds`; it does not currently suppress or demote `RaidTarget`.
3. **Current gap**: `RaidTarget` is never stress-suppressed in `goal_policy.rs`, and `emit_raid_target_goals()` only gates on `danger_pressure >= thresholds.danger.high()`.
4. **Required follow-up**: Before this suite is implemented, the engine needs one canonical architecture for wound-driven raid deterrence. The fix should not be documented here as a one-off candidate-generation patch; it should align the shared combat/raid motive substrate so the same rule is visible to ranking, interruption, and diagnostics.
5. **Dampening remains the intended invariant**: when that follow-up lands, the suite should prove the behavior change traces back to concrete `WoundList` state rather than cooldowns or abstract fatigue counters.

#### Chain

```
wound-producing raids → accumulated concrete injury →
shared combat/raid deterrence substrate changes the selected behavior →
bandit stops raiding despite continued opportunity
```

**Causal depth**: 4 (repeated raids → wound accumulation → deterrence signal → behavioral suppression)

**Systems**: Combat (repeated), Wounds (accumulation), AI (shared combat/raid motive or deterrence substrate), Feedback dampening

#### Metadata Annotation

```
// Scenario 49: Wound-Dampened Raid Spiral
//
// Systems: Combat, Wounds, AI, Needs, Feedback Dampening
// GoalKinds: RaidTarget, ReduceDanger, ConsumeOwnedCommodity
// ActionDomains: Combat, Needs
// Places: BanditCamp, RoadJunction
// Principles: 8, Concrete State Over Abstract Scores, Feedback Dampening, Maximal Emergence
```

#### FND-01 Section H Analysis

**Information-path**: Wound state is an internal authoritative component on the bandit agent. The future deterrence mechanism must read that local state canonically, with no cross-agent information transfer needed.

**Positive-feedback loop identified**: Successful raid → loot → sustained hunger satisfaction → more raids → more wounds → MORE raids (if wounds don't accumulate). This is the amplifying loop that the dampener must break.

**Concrete dampener**: Wound accumulation remains the intended physical dampener. Each combat produces wounds via the combat system. Reassessment showed the live code does not yet connect wound-only state to raid deterrence once hostiles disappear, so the canonical fix still needs to be designed and implemented before this suite can assert the final behavior.

**Stored state vs. derived**: `WoundList` (stored — authoritative wound records). Pain/danger permille (derived from wound_list at ranking time). Priority class ordering (derived from pressure permille thresholds). Raid target eligibility (derived from co-location). No derived value is stored as authoritative state.

---

## Test File Organization

All three suites extend `golden_t22_bandit_camp_destruction.rs`, keeping bandit-related golden tests co-located. The shared topology builders and helper functions from T22 (e.g., `bandit_profile()`, `default_perception_profile()`, `connect()`) can be reused. Each suite defines its own topology variant and scenario-specific setup.

## Assertion Strategy

Per `docs/golden-e2e-testing.md` assertion hierarchy:

| Suite | Primary Assertion Surface | Secondary |
|-------|--------------------------|-----------|
| S47 (Raid Emergence) | Action traces (raid started/committed) | Decision traces (candidate emission), authoritative state (conservation) |
| S48 (Economic Cascade) | Decision traces (route selection before/after belief) | Action traces (tell committed), belief store inspection |
| S49 (Wound Dampening) | Future shared deterrence proof surface after engine alignment | Authoritative state (wound list growth), action traces (raid count) |

## Verification

1. `cargo test -p worldwake-ai golden_pressure_driven_raid` — Suite 1 passes
2. `cargo test -p worldwake-ai golden_raid_belief_economic` — Suite 2 passes
3. `cargo test -p worldwake-ai golden_wound_dampened` — Suite 3 passes
4. `cargo test -p worldwake-ai` — all existing golden tests still pass (no regressions)
5. `cargo clippy --workspace` — no warnings
6. `python3 scripts/golden_inventory.py --write --check-docs` — generated docs include Scenarios 47-49
7. Each suite exercises ≥3 systems with causal depth ≥3

## Coverage Impact

After implementation, E18-related golden coverage will include:

| Chain | Suite | Tests |
|-------|-------|-------|
| Destruction → regrouping → re-establishment | T22 (existing) | 2 |
| Hunger → raid → loot → resupply | S47 (new) | 2 |
| Raid → witness → belief → merchant reroute | S48 (new) | 2 |
| Repeated raids → wounds → raid suppression | S49 (new) | 2 |
| **Total** | | **8** |

New GoalKind coverage: `RaidTarget` (currently 0 scenarios), `EstablishBanditCamp` (currently only in T22 as secondary).

New principle coverage: FND-10 (Physical Dampeners) gets its first explicit golden validation via S49.
