# E18: Bandit Camp Dynamics

## Epic Summary

Implement bandit camps as places with faction membership, supply containers, raid behavior, emergent destruction consequences, survivor regrouping through rally-point beliefs, and belief-based route danger assessment. All bandit behavior emerges from existing AI pressure/goal systems — no abstract morale scores, no stored danger values, no centralized patrol routes.

## Phase

Phase 4: Group Adaptation, CLI & Verification

## Crate

`worldwake-core` (components, profiles), `worldwake-systems` (actions, handlers), `worldwake-ai` (goal generation extensions)

## Dependencies

- E16 (faction system — MemberOf relations, FactionData, LoyalTo, HostileTo) — completed
- E12 (combat — CombatProfile, WoundList, Attack/Loot actions) — completed
- E13 (decision architecture — GOAP planner, pressure system, enterprise signals) — completed
- E14/E15 (beliefs, perception, Tell action) — completed for belief-based route safety

---

## Foundational Alignment

This spec was designed against `docs/FOUNDATIONS.md`. Key principle compliance:

| Principle | How This Spec Complies |
|-----------|----------------------|
| FND-1 (Maximal Emergence) | Bandit behavior emerges from pressure-driven AI goals, not scripted patrol routes or authored raid triggers |
| FND-2 (No Ungrounded Triggers) | No `raidChance`, `spawnRate`, or `encounterProbability`. Raids happen because a bandit is at a location, sees a target, and has combat capability |
| FND-3 (Concrete State Over Abstract Scores) | No morale score. No danger score on edges. Behavior emerges from wounds, hunger, supplies, blocked intents. Route safety comes from agent beliefs about actual threats |
| FND-4 (Persistent Identity) | Bandits have stable identity. Dead bandits stay dead. Supplies are conserved. Camp destruction leaves concrete aftermath |
| FND-7 (Locality) | Regrouping uses rally-point beliefs learned through observation, not global faction knowledge. Route danger assessed through local beliefs, not omniscient queries |
| FND-8 (Preconditions, Duration, Cost) | EstablishCamp has duration, material cost, minimum member count, and is interruptible. Raid uses combat duration/cost |
| FND-9 (Granular Outcomes) | Raids produce wounds, deaths, scattered cargo, witnesses, beliefs, blocked intents — not binary success/fail |
| FND-10 (Physical Dampeners) | Four concrete dampeners limit raid success spirals (see Section H) |
| FND-12 (World State != Belief State) | Agents assess route danger through beliefs, not by reading world state. Stale beliefs about camp locations persist until corrected |
| FND-17 (Agent Symmetry) | Bandits use the same action framework, combat system, and AI planner as all other agents |
| FND-24 (Systems Interact Through State) | No cross-system calls. Combat creates wounds → needs react → AI replans. All through state |
| FND-25 (Derived Summaries Are Caches) | Route threat estimates are derived queries for AI heuristic use, never stored as authoritative state |

---

## Deliverables

### 1. BanditCamp Component (`worldwake-core`)

```rust
/// Marks a Place entity as a bandit camp. Minimal stored state;
/// membership tracked via MemberOf relations to the camp's faction,
/// combat capability via per-agent CombatProfile, survival state
/// via WoundList and HomeostaticNeeds.
pub struct BanditCamp {
    /// Container entity holding the camp's communal supplies.
    pub supplies: EntityId,
}
```

- Attached to Place entities (EntityKind::Place with PlaceTag::Camp)
- The camp's faction is a separate Faction entity; members join via `MemberOf` relation
- No `members` field — use `world.members_of(faction_id)` to query
- No `morale` field — behavior emerges from concrete agent state (FND-3)
- No `preferred_raid_routes` — patrol targets emerge from individual agent goals (FND-1)

### 2. BanditFactionPolicy Component (`worldwake-core`)

```rust
/// Faction-scoped policy controlling regrouping, establishment, and abandonment thresholds.
/// All thresholds are Permille (0–1000) to comply with spec drafting rules.
pub struct BanditFactionPolicy {
    /// Minimum living faction members needed to establish a new camp.
    pub min_regroup_count: u8,
    /// Ticks required to establish a new camp via EstablishCamp action.
    pub establishment_duration_ticks: NonZeroU32,
    /// Grace period before an empty camp is considered abandoned.
    pub abandonment_grace_ticks: NonZeroU32,
    /// Wound-load threshold (as fraction of capacity) above which
    /// a bandit prioritizes fleeing over fighting. Per-agent courage
    /// in UtilityProfile modulates this further.
    pub flee_wound_threshold: Permille,
    /// Known rally place where faction members should regroup after
    /// camp destruction. Members learn this by observing faction policy
    /// while co-located with an active camp (belief, not guaranteed knowledge).
    pub rally_place: Option<EntityId>,
}
```

- Profile-driven parameters replace all magic numbers (FND-2)
- `flee_wound_threshold` interacts with existing `UtilityProfile.courage` — agents with high courage may fight past this threshold
- `rally_place` is faction policy rather than place state, but members still learn it through lawful local observation while at an active camp

### 3. Faction Setup

Bandit camps use the existing faction infrastructure from E16:

- **Faction entity**: `FactionData { name: "Forest Bandits", purpose: FactionPurpose::Military }`
- **Membership**: `MemberOf` relation from each bandit agent to the faction entity
- **Loyalty**: `LoyalTo { strength: Permille }` from each bandit to the faction
- **Hostility**: `HostileTo` relation from bandits to non-faction agents encountered during raids

No new relation types needed. The existing `members_of(faction_id)` query returns all current members.

### 4. Raid Action (`worldwake-systems`)

**ActionDef: Raid**

| Field | Value |
|-------|-------|
| Domain | `ActionDomain::Combat` |
| Actor constraints | `ActorAlive`, `ActorHasControl`, `ActorNotInTransit` |
| Target spec | `EntityAtActorPlace` (non-faction agent) |
| Preconditions | `TargetAlive`, `TargetAtActorPlace`, actor has `MemberOf` to a faction with a `BanditCamp`, target is NOT in same faction |
| Duration | From actor's `CombatProfile` (same as Attack) |
| Body cost | Same as Attack |
| Interruptibility | `FreelyInterruptible` (can disengage and flee) |
| Commit conditions | `TargetAtActorPlace` (target hasn't fled) |
| Visibility | `SamePlace` — all co-located agents witness the raid |
| Event tags | `Combat`, `Transfer` |
| Payload | `RaidActionPayload { target: EntityId, weapon: CombatWeaponRef }` |

**Handler behavior:**
- `commit_raid`: Resolves combat (delegates to existing wound-application logic from Attack). On victory (target dead or incapacitated): emits combat event, Loot affordance becomes available for the raider. On defeat (raider wounded/fled): emits combat event, `BlockedIntentMemory` records `CombatTooRisky` for this target/location.
- Witnesses at the same place form beliefs about the attack. These beliefs propagate via Tell action to other agents they encounter.

**Distinction from Attack:** Raid is semantically a bandit-initiated combat for the purpose of acquiring goods. The AI uses this distinction for goal generation — bandits generate `RaidTraveler` goal candidates (see AI section), not generic `EngageHostile`. Mechanically, combat resolution is identical to Attack.

### 5. EstablishCamp Action (`worldwake-systems`)

**ActionDef: EstablishCamp**

| Field | Value |
|-------|-------|
| Domain | `ActionDomain::Generic` |
| Actor constraints | `ActorAlive`, `ActorHasControl`, `ActorNotInTransit` |
| Target spec | None (acts on current place) |
| Preconditions | Actor's place has `PlaceTag::Camp` or `PlaceTag::Forest`; actor has `MemberOf` to a bandit faction; at least `BanditFactionPolicy.min_regroup_count` living faction members at same place; actor possesses minimum supplies (food commodity) |
| Duration | `BanditFactionPolicy.establishment_duration_ticks` |
| Body cost | Moderate (hunger, fatigue increase) |
| Interruptibility | `InterruptibleWithPenalty` (disruption by attack wastes progress) |
| Commit conditions | Same place requirements still met; minimum members still present |
| Visibility | `SamePlace` |
| Event tags | `WorldMutation` |

**Handler behavior:**
- `commit_establish_camp`: Creates `BanditCamp` component on the current Place entity. Creates a new Container entity for camp supplies. Transfers actor's carried supplies into camp container. Emits camp-establishment event.
- If the place already has a `BanditCamp` component (e.g., reoccupying an abandoned camp), the action reuses the existing camp rather than creating a duplicate.

### 6. Camp Destruction (Emergent, No New System)

Camp destruction is **not** a special system — it emerges from existing combat and AI:

1. Attackers engage bandits at camp using Attack/Raid actions
2. Combat produces wounds, deaths, incapacitation via existing combat system
3. Surviving bandits with high danger pressure generate `ReduceDanger` goals → flee via Travel
4. When no living faction members remain at the camp place, the camp is effectively abandoned

**Camp abandonment detection**: A lightweight per-tick check in the bandit camp system (new system function `bandit_camp_system()` registered in SystemManifest):
- For each place with `BanditCamp` component: query `members_of(faction)` for living members at that place
- If zero living members present for `BanditFactionPolicy.abandonment_grace_ticks`: remove `BanditCamp` component, emit `CampAbandoned` event
- Camp supplies container remains at the place — lootable by anyone
- Faction entity is NOT archived (surviving members still reference it)
- Grace period prevents premature abandonment if all members are briefly away (e.g., on a raid)

**Aftermath:**
- Supplies at abandoned camp are lootable (existing Loot/PickUp actions)
- Dead bandits have `DeadAt` component — bodies and possessions persist (FND-4, FND-9)
- Witnesses to the battle carry beliefs about the outcome

### 7. Survivor Behavior (Emergent from AI)

Survivors behave autonomously through the existing pressure-based AI decision system. No special "survivor mode" code:

| Concrete State | AI Response |
|---------------|-------------|
| High wound load | `ReduceDanger` goal → flee via Travel (existing) |
| Hunger/thirst | `ConsumeCommodity` goal → eat/drink from carried supplies (existing) |
| `CombatTooRisky` blocked intent | Avoids re-engaging at that location for expiration period (existing) |
| `DangerTooHigh` blocked intent | Avoids locations with perceived threats (existing) |
| Rally-point belief | `RegroupWithFaction` goal → Travel to rally place (new goal kind) |
| Fatal wounds | `DeadAt` component → permanently removed from decision cycle (existing) |

**Key invariants:**
- Survivors retain injuries, inventory, loyalties — no state reset (FND-4)
- No respawn: dead bandits stay dead, no new bandits spawned (brainstorming spec Section 8)
- No teleportation: regrouping requires physical travel (brainstorming spec Section 9.10)

### 8. Regrouping via Rally-Point Beliefs

**Information path** (FND-7 compliance):

1. While at an active camp, bandit agents can lawfully observe their faction's `BanditFactionPolicy.rally_place` → forms a belief: "if camp falls, regroup at [rally place]"
2. When camp is destroyed (high danger, members flee), each surviving bandit independently checks their belief about the rally point
3. Survivors who hold this belief generate `RegroupWithFaction { faction }` goal → plan search finds Travel to rally place
4. Survivors who never observed the rally point (e.g., newly joined, absent when it was set) do NOT know where to go — they act on their own survival goals only
5. At the rally place, if `min_regroup_count` living faction members gather, `EstablishCamp` affordance appears
6. The whole process uses normal Travel action — duration, interruption, and route exposure apply

**New goal kind** (`worldwake-ai`):
- `GoalKind::RegroupWithFaction { faction: EntityId }` — drives travel to the agent's believed rally point for that faction
- Priority: Below immediate survival (ReduceDanger, ConsumeCommodity) but above enterprise goals
- Suppression: Suppressed when danger is Critical (survival first)
- Maps to planner ops: `Travel`

### 9. Route Safety Through Beliefs (No Stored Danger)

Route safety is assessed through agent beliefs, not stored values on edges (FND-3, FND-25):

**How agents learn about dangerous routes:**
1. **Direct observation**: Agent at a place witnesses a bandit attack → forms belief "bandits present at [place]"
2. **Testimony**: Witness uses Tell action to share attack belief with other agents → belief spreads with source attribution and credibility
3. **Evidence**: Agent observes corpses, abandoned cargo at a place → infers danger (requires perception system from E14)
4. **Absence of evidence**: Over time, beliefs about danger at a location age and lose confidence if no new evidence arrives

**How agents use danger beliefs for route planning:**
- AI planner's route selection heuristic considers agent's beliefs about hostile presence near edge endpoints
- `route_threat_estimate(agent, edge)` — derived query (never stored) that checks agent's beliefs about threats at edge endpoints
- Merchants with beliefs about dangerous routes may: take longer safe routes, delay travel, seek guards (E19 interaction)

**After camp destruction:**
- No new attacks on former patrol routes → no new witness reports
- Existing beliefs about danger age out → route perceived as safer over time
- Merchants who never received reports continue using routes normally (FND-14: ignorance is first-class)

**After new camp established:**
- Bandits resume raiding from new location → new attacks → new witnesses
- New beliefs propagate → affected routes perceived as dangerous
- The causal chain is fully traceable: camp → bandits at location → raid → witnesses → beliefs → route avoidance

---

## Section H: FND-01 Analysis

### H.1 Information-Path Analysis

| Information | Source | Path to Agent | Latency |
|-------------|--------|--------------|---------|
| "Bandits are at location X" | Direct observation (co-location) | Immediate perception | 0 ticks |
| "Route Y was attacked" | Witness testimony | Tell action at shared location | Travel time to shared place + Tell |
| "Camp at Z was destroyed" | Direct observation or testimony | Same as above | Variable |
| "Rally point is at W" | Observation of `BanditFactionPolicy` while at an active camp | Immediate (co-located with camp) | 0 ticks |
| "Route Y is safe again" | Absence of new attack reports | Belief aging/decay | Configured belief freshness period |

No information arrives at an agent without a traceable physical path. Rally-point knowledge requires prior co-location with the camp.

### H.2 Positive-Feedback Analysis

**Amplifying loop identified:** Successful raids → more supplies → better-fed bandits → more raids → more supplies

This is the primary positive feedback loop. Successful raiding increases camp supplies, which keeps bandits well-fed and combat-ready, enabling further raids.

### H.3 Concrete Dampeners

Four physical dampeners limit the raid success spiral:

1. **Combat risk**: Every raid exposes bandits to wounds and death. Even successful raids may wound attackers. Wounded bandits have reduced combat effectiveness and may prioritize healing over raiding. Fatal wounds permanently remove raiders (no respawn).

2. **Traveler route avoidance**: Witnesses spread beliefs about dangerous routes. Merchants and travelers adjust route planning based on these beliefs, reducing the number of targets on bandit-patrolled routes. Fewer targets → fewer successful raids → supply pressure.

3. **Guard response** (E19 interaction): Reports of bandit attacks may trigger guard patrols on affected routes. Guards increase combat risk for bandits, creating a direct counter-force. This dampener strengthens as attack reports accumulate at settlements with guard institutions.

4. **Supply consumption**: Camp members consume supplies each tick through the existing needs system (hunger, thirst). A camp with many members depletes supplies faster, creating pressure even during successful periods. Supplies are conserved — they cannot appear from nowhere (FND-4).

All dampeners are physical world processes, not numeric caps (FND-10).

### H.4 Stored State vs. Derived

**Authoritative stored state:**
- `BanditCamp` component on Place entities (supplies container ref)
- `BanditFactionPolicy` component on Faction entities (thresholds, abandonment grace, rally place)
- `MemberOf` relations (bandit → faction)
- `LoyalTo` relations (bandit → faction, with strength)
- `HostileTo` relations (bandit → targets)
- Per-agent: `CombatProfile`, `UtilityProfile`, `WoundList`, `HomeostaticNeeds`, `BlockedIntentMemory`

**Derived (never stored as authoritative):**
- Route threat estimates (computed from agent beliefs for AI heuristic)
- Camp member count (derived from `members_of(faction)` query)
- Camp combat strength (derived from member CombatProfiles and WoundLists)
- Camp supply level (derived from container contents query)

---

## Principle 28 Declarations

### 28.1 Entities, Relations, and Records Introduced

| Entity/Component | Kind | Purpose |
|-----------------|------|---------|
| `BanditCamp` | Component on Place | Marks a place as a bandit camp, references supply container |
| `BanditFactionPolicy` | Component on Faction | Faction regroup/establishment/abandonment policy and rally point |
| Supply container | EntityKind::Container | Holds camp's communal supplies |
| Bandit faction | EntityKind::Faction | `FactionData { purpose: Military }` |

Relations used (all existing): `MemberOf`, `LoyalTo`, `HostileTo`, `LocatedIn`, `ContainedBy`, `PossessedBy`.

### 28.2 Actions and World Processes That Mutate Them

| Mutation | Action/Process |
|----------|---------------|
| BanditCamp created | `EstablishCamp` action commit |
| BanditCamp removed | `bandit_camp_system()` abandonment check |
| Supply container contents changed | `PickUp`, `PutDown`, `Loot` actions; needs consumption |
| Faction membership changed | Agent death (implicit — dead agents don't participate) |
| HostileTo created | Raid action commit (attacker → target) |

### 28.3 Information Production and Travel

- Raid events visible to `SamePlace` witnesses → witnesses carry beliefs → Tell action spreads to other agents at shared locations
- Camp establishment events visible to `SamePlace` → witnesses know camp location
- Rally-point knowledge acquired through observation while co-located with camp
- No information teleports — all paths require co-location or physical carrier

### 28.4 Conserved Quantities

- **Supplies**: Conserved. Enter camp via PickUp/PutDown or loot. Leave via consumption (needs system) or theft/loot. No generation except through explicit production/harvest actions.
- **Bandit agents**: Conserved. No spawn. No despawn. Created only during world setup. Removed only through death (DeadAt component).
- **Weapons/equipment**: Conserved through existing item conservation system.

### 28.5 Scarce Capacities and Contention

- **Camp establishment**: Only one EstablishCamp action can succeed at a place (once BanditCamp component exists, the commit condition prevents duplicates)
- **Raid targets**: Multiple bandits may target the same traveler — resolved through combat action ordering within the tick (existing combat system handles multi-combatant scenarios)
- **Rally-point gathering**: No explicit reservation — bandits simply travel there. First to arrive wait; EstablishCamp requires minimum count present simultaneously

### 28.6 Partial Failures and Aftermath

| Action | Partial Failure | Aftermath |
|--------|----------------|-----------|
| Raid | Raider wounded but target escapes | Raider has wounds, `CombatTooRisky` blocked intent, target has beliefs about attacker |
| Raid | Raider killed | Body + possessions remain at location, target may loot, witnesses report |
| EstablishCamp | Interrupted by attack | Progress lost, members scatter, supplies still carried |
| Regrouping | Not enough survivors reach rally point | Individuals survive independently, no new camp formed |
| Regrouping | Rally point occupied by hostiles | Bandits encounter danger, may flee further or fight |

### 28.7-28.8 Feedback Loops and Dampeners

See Section H.2 and H.3 above.

### 28.9 Derived Views and Optimizations

- `route_threat_estimate(agent, edge)` — AI heuristic derived from agent beliefs. May be cached per planning cycle but must be invalidated when beliefs change. Deletable and recomputable (FND-25).

### 28.10 How Agents Can Be Wrong

- Agent believes camp still exists after it was destroyed (stale belief until new evidence arrives)
- Agent believes route is safe when bandits have relocated there (no witness reports yet)
- Agent believes rally point is safe when it has been occupied by enemies
- Agent's belief about faction member locations is stale (members may have died or moved)
- Correction requires new local evidence — observation, testimony, or inference from physical clues

### 28.11 Save/Load, Replay, and Compression Survival

All authoritative state (`BanditCamp`, `BanditFactionPolicy`, faction relations, agent state) is component/relation data already handled by the existing save/load and replay systems. No new serialization requirements beyond registering the two components in `component_tables` and `component_schema`.

---

## System Integration

### New System Function

```rust
/// Checks for abandoned bandit camps and removes BanditCamp component.
/// Registered in SystemManifest between Combat and FacilityQueue.
pub fn bandit_camp_system(world: &mut World, event_log: &mut EventLog, tick: Tick) {
    // For each place with BanditCamp:
    //   Query members_of(faction) for living members located at this place
    //   If zero present and grace period expired: remove BanditCamp, emit CampAbandoned
}
```

**Tick execution order**: Needs → Production → Trade → Combat → **BanditCamp** → FacilityQueue → Politics → Perception

Placement after Combat ensures that combat deaths are processed before abandonment checks. Placement before Perception ensures that camp abandonment events are visible to observers in the same tick.

### Component Registration

- `BanditCamp`: allowed on `EntityKind::Place`
- `BanditFactionPolicy`: allowed on `EntityKind::Faction`

### AI Extensions (`worldwake-ai`)

**New goal kind:**
- `GoalKind::RegroupWithFaction { faction: EntityId }`
- Relevant planner ops: `Travel`
- Suppression: `WhenStressedAtOrAbove(Critical)` — survival comes first
- Priority class: Below immediate survival, above enterprise

**Candidate generation extension:**
- When agent has `MemberOf` to a faction AND holds a rally-point belief AND is not at the rally place AND no `BanditCamp` exists for that faction → generate `RegroupWithFaction` candidate
- When agent has `MemberOf` to a bandit faction AND is at a place with non-faction agents → generate raid goal candidates (using existing `EngageHostile` or new `RaidTarget` variant)

**Enterprise signal pattern for raids:**
- Analogous to merchant restock signals: bandits assess "opportunity" based on perceived non-faction agents at their location
- High opportunity (travelers with cargo) + low danger (no guards) → high-priority raid goal
- This uses the existing enterprise signal infrastructure from E13

---

## Invariants Enforced

- **No bandit respawn** (brainstorming spec Section 8): Dead bandits have `DeadAt`. No system creates new bandit agents.
- **No teleportation** (Section 9.10): Regrouping uses Travel action with full duration and route exposure.
- **Dead bandits stay dead** (Section 9.14): `DeadAt` component permanently removes from AI decision cycle.
- **No abstract morale** (FND-3): Behavior emerges from wounds, hunger, supplies, blocked intents.
- **No stored danger values** (FND-3, FND-25): Route safety assessed through beliefs.
- **Conservation** (FND-4): Supplies conserved. No item generation without explicit source.
- **Locality** (FND-7): Rally-point knowledge requires prior observation. No global queries.

---

## Tests

### T22: Bandit Camp Destruction Chain (Golden Test)
- **Setup**: Bandit camp with 3+ members, supplies, raid history on nearby routes. Merchants with beliefs about route danger.
- **Action**: External force (guards, adventurers) attacks and defeats camp defenders.
- **Expected**:
  - Survivors flee (Travel action) based on danger pressure — they do not despawn
  - Surviving members retain injuries, inventory, and faction membership
  - Dead members have `DeadAt` — bodies and possessions persist at camp
  - Survivors with rally-point belief travel to rally place
  - If enough survivors gather at rally place: `EstablishCamp` action fires
  - Route beliefs age: former patrol routes perceived as safer over time
  - New camp location triggers new raids → new beliefs about new dangerous routes
  - Merchants with updated beliefs adjust route planning
- **Pass threshold** (from brainstorming T22): Within 5 in-world days, route safety and at least one downstream economic behavior must change because of the diaspora.
- **Causal depth**: >= 4 across >= 3 subsystems

### Additional Tests

- [ ] Survivors retain injuries, inventory, faction membership after camp destruction
- [ ] No respawn: destroyed camp does not regenerate members; member count only decreases
- [ ] Regrouping requires physical travel to rally point (no teleportation)
- [ ] Bandits without rally-point belief do not navigate to rally place
- [ ] Dead bandits do not participate in regrouping or generate goals
- [ ] EstablishCamp requires minimum member count at suitable location
- [ ] EstablishCamp can be interrupted by attack
- [ ] Camp supply container is lootable after abandonment
- [ ] Raid action creates witnesses who form beliefs about the attack
- [ ] Beliefs about route danger age out when no new attacks occur
- [ ] `bandit_camp_system` respects grace period before marking camp abandoned
- [ ] Multiple bandits raiding simultaneously resolves through existing combat ordering

---

## Acceptance Criteria

- Bandit camps as real places with faction membership and supply containers
- Destruction has persistent consequences — no state reset, no respawn
- Survivors behave autonomously through existing AI pressure/goal system
- Route danger assessed through agent beliefs, not stored scores
- Regrouping requires physical travel to a rally point learned through observation
- All behavior traceable through the causal chain (FND-27: debuggability)
- No f32 anywhere — Permille for all [0,1] ranges
- No magic numbers — all thresholds profile-driven

## Spec References

- `docs/FOUNDATIONS.md`: Principles 1-4, 7-10, 12, 17, 24-25, 27-28
- `brainstorming/emergent-prototype-spec.md`: Section 1 (exemplar 3), Section 4.5, Section 7.1, Section 8, Section 9.10, Section 9.14, T22
- `docs/precision-rules.md`: Rules 1-4, 7-8
