# E16: Offices, Succession & Factions

## Epic Summary

Implement offices with succession laws (support and force), factions with loyalty, coercion/bribery actions, public order as an extensible Permille aggregator, and belief-mediated vacancy awareness — all built on existing E14/E15/E15b infrastructure.

## Phase

Phase 3: Information & Politics

## Crate

`worldwake-systems` (succession system, action handlers)
`worldwake-core` (new components, enums, relation storage, goal kinds)
`worldwake-ai` (new planner ops, goal model mapping, candidate generation)

## Dependencies

- E14 (beliefs needed for loyalty decisions and succession awareness)
- E15 (rumor propagation for vacancy news spreading via Tell)
- E15b (social AI goals — ShareBelief, SocialObservation tracking)
- E12 (combat system for Force succession resolution)
- E07 (action framework for Bribe, Threaten, DeclareSupport actions)

## Dependency Note

Institutional producers need a way to own the goods they produce through ordinary world relations. Workshop/source output claim policy should be established before office- or faction-owned facilities are expected to accumulate assets cleanly. See [DRAFT-production-output-ownership-claims.md](/home/joeloverbeck/projects/worldwake/specs/DRAFT-production-output-ownership-claims.md).

## Existing Infrastructure (Leveraged, Not Reimplemented)

The following already exist in the codebase and MUST be reused:

| Infrastructure | Location | Usage in E16 |
|----------------|----------|--------------|
| `EntityKind::Office` | `entity.rs` | Office entity classification |
| `EntityKind::Faction` | `entity.rs` | Faction entity classification |
| `office_holder / offices_held` (1:1) | `relations.rs` | Who holds which office |
| `member_of / members_of` (N:N) | `relations.rs` | Faction membership |
| `loyal_to / loyalty_from` (N:N, weighted `Permille`) | `relations.rs` | Agent loyalty scores |
| `hostile_to / hostility_from` (N:N) | `relations.rs` | Faction/agent hostility |
| `EventTag::Political` | `event_tag.rs` | Event classification for office/succession events |
| `EventTag::Social` | `event_tag.rs` | Event classification for bribe/threaten events |
| `ActionDomain::Social` | `action_domain.rs` | Domain for bribe/threaten/declare_support actions |
| `ActionDomain::Combat` | `action_domain.rs` | Domain for force succession resolution |
| Belief system (E14) | `worldwake-ai` | Belief-mediated vacancy awareness |
| Tell action (E15) | `worldwake-systems` | Vacancy news propagation |
| SocialObservation (E15b) | `worldwake-ai` | Bystander witnessing of bribe/threaten |
| GOAP planner | `worldwake-ai` | Goal/plan search for ClaimOffice, SupportCandidate |
| Combat system (E12) | `worldwake-systems` | Force succession resolution |

## Deliverables

### 1. New Components

#### OfficeData (component for `EntityKind::Office`)

```rust
pub struct OfficeData {
    pub title: String,                          // "Village Ruler", "Guard Captain"
    pub jurisdiction: EntityId,                 // Place this office governs
    pub succession_law: SuccessionLaw,          // Support or Force
    pub eligibility_rules: Vec<EligibilityRule>,
    pub succession_period_ticks: u64,           // Duration of support-gathering phase
    pub vacancy_since: Option<Tick>,            // None if filled, Some(tick) if vacant
}
```

#### FactionData (component for `EntityKind::Faction`)

```rust
pub struct FactionData {
    pub name: String,
    pub purpose: FactionPurpose,  // Political, Military, Trade, Religious
}
```

### 2. New Enums

```rust
pub enum SuccessionLaw {
    Support,  // Most supported candidate after timed period
    Force,    // Whoever can take and hold by combat
}

pub enum EligibilityRule {
    FactionMember(EntityId),  // Must be member of this faction
}

pub enum FactionPurpose {
    Political,
    Military,
    Trade,
    Religious,
}
```

**Design rationale**: `SuccessionLaw` is limited to `Support` and `Force` — two complementary paths (political and military). `Hereditary` and `Appointment` from the original spec are removed because blood relations and appointment mechanics are not in the codebase. `EligibilityRule` uses faction membership as the primary filter, based on existing `member_of` relations.

### 3. New Relation: Support Declarations

Stored in `RelationTables` (not a component):

```rust
// (supporter, office) -> candidate
// BTreeMap for determinism
pub support_declarations: BTreeMap<(EntityId, EntityId), EntityId>,
```

This is a public declaration separate from loyalty. An agent can be loyal to one candidate but publicly declare support for another (e.g., under coercion). This separation enables coerced support, secret loyalty — richer emergence.

**Reused relations** (no new definitions needed):
- `office_holder / offices_held` — who holds which office (1:1)
- `member_of / members_of` — faction membership (N:N)
- `loyal_to / loyalty_from` — weighted loyalty with Permille (N:N)
- `hostile_to / hostility_from` — faction/agent hostility (N:N)

### 4. UtilityProfile Extension

Add `courage: Permille` to `UtilityProfile` in `crates/worldwake-core/src/utility_profile.rs`:

```rust
pub struct UtilityProfile {
    // ... existing fields ...
    pub courage: Permille,
}
```

- Default: `Permille(500)` (moderate courage)
- Low courage (100-300): easily intimidated, yields to weaker threats
- High courage (700-900): resists even strong threats, may fight back
- Enables agent diversity per Principle 20 — two agents facing the same threat may react differently

### 5. New Actions

All actions use `ActionDomain::Social` and follow existing `ActionDef` / `ActionHandler` patterns.

#### Bribe Action

- **Domain**: `ActionDomain::Social`
- **Preconditions**: Actor and target co-located, actor possesses transferable goods, target is alive
- **Duration**: 2 ticks
- **Body cost**: Low (social exertion)
- **Effect on commit**:
  - Goods transferred from actor to target (via possession change — conservation invariant enforced)
  - Target's `loyal_to` actor increases by an amount proportional to goods value
  - Emit BribeEvent with `SamePlace` visibility (witnesses see it happening)
- **Social observation**: Bystanders record `WitnessedObligation` between actor and target
- **Interruptibility**: Freely interruptible (both parties walk away, goods not yet transferred)
- **Payload**: `BribeActionPayload { target: EntityId, offered_commodity: CommodityKind, offered_quantity: Quantity }`

#### Threaten Action

- **Domain**: `ActionDomain::Social`
- **Preconditions**: Actor and target co-located, actor has `CombatProfile` (implicit threat of force), target is alive
- **Duration**: 1 tick
- **Body cost**: Low
- **Yield/Resist logic**: Compare actor's combat advantage (`wound_capacity + attack_skill_base`) against target's `courage: Permille` from `UtilityProfile`. If actor's advantage exceeds target's courage threshold, target yields. Otherwise, target resists.
- **Effect on commit (yield)**: Target's `loyal_to` actor increases significantly. Emit `ThreatenEvent(Yielded)` with `SamePlace` visibility.
- **Effect on commit (resist)**: No loyalty change. May set `hostile_to` between actor and target. Emit `ThreatenEvent(Resisted)` with `SamePlace` visibility.
- **Social observation**: Bystanders record `WitnessedConflict`
- **Risk**: Witnesses may decrease their own loyalty to the threatener (seeing intimidation)
- **Interruptibility**: Not interruptible (too fast)
- **Payload**: `ThreatenActionPayload { target: EntityId }`

#### DeclareSupport Action

- **Domain**: `ActionDomain::Social`
- **Preconditions**: Agent at office jurisdiction place, office is vacant and in succession period, agent knows of vacancy (belief-mediated — agent must believe the office holder is dead/absent)
- **Duration**: 1 tick
- **Body cost**: None
- **Effect on commit**:
  - Sets `support_declarations[(agent, office)] = candidate`
  - Overwrites any previous declaration for same office
  - Emit `DeclareSupportEvent` with `SamePlace` visibility
- **Social observation**: Bystanders record `WitnessedCooperation` between declarer and candidate
- **Interruptibility**: Not interruptible (instant public statement)
- **Payload**: `DeclareSupportActionPayload { office: EntityId, candidate: EntityId }`

### 6. New AI Goals

#### `GoalKind::ClaimOffice { office: EntityId }`

- **Generated when**: Agent believes office is vacant AND agent is eligible (faction membership check)
- **Priority class**: Medium (below survival/danger, above social)
- **Motive**: Based on agent's `enterprise_weight` from `UtilityProfile` (ambition)
- **Planner ops**: Travel, Bribe, Threaten, DeclareSupport
- **Plan sketch (Support law)**: Travel to jurisdiction -> Bribe/Threaten potential supporters -> DeclareSupport(self) -> Wait
- **Plan sketch (Force law)**: Travel to jurisdiction -> Attack current competitors -> Occupy

#### `GoalKind::SupportCandidateForOffice { office: EntityId, candidate: EntityId }`

- **Generated when**: Agent believes office is vacant AND agent has `loyal_to` weight above threshold for an eligible candidate
- **Priority class**: Low (above idle social, below enterprise)
- **Motive**: Based on `social_weight * loyal_to` strength to candidate
- **Planner ops**: Travel, DeclareSupport
- **Plan sketch**: Travel to jurisdiction -> DeclareSupport(candidate)

### 7. New PlannerOpKinds

- `PlannerOpKind::Bribe` — classified from bribe action (`ActionDomain::Social`, name `"bribe"`)
- `PlannerOpKind::Threaten` — classified from threaten action (`ActionDomain::Social`, name `"threaten"`)
- `PlannerOpKind::DeclareSupport` — classified from declare_support action (`ActionDomain::Social`, name `"declare_support"`)

### 8. Succession System (per-tick)

`succession_system(world, txn, tick)` — runs each tick in `SystemManifest`:

```
For each Office entity with OfficeData:
  IF holder is alive -> continue (office is stable)
  IF holder is dead AND vacancy_since is None:
    -> Set vacancy_since = current_tick
    -> Clear office_holder relation
    -> Emit VacancyEvent (visibility: SamePlace at jurisdiction)
  IF vacancy_since is Some(start_tick):
    Match succession_law:
      Support:
        IF current_tick - start_tick >= succession_period_ticks:
          -> Count support_declarations for this office
          -> Candidate with most declarations wins
          -> If tie: extend period by succession_period_ticks / 2
          -> If winner: set office_holder, clear vacancy_since,
             clear support_declarations for this office,
             emit InstallationEvent
          -> If no declarations: extend period
      Force:
        -> Minimum closed-world implementation: if exactly one eligible claimant
           is present at jurisdiction after the succession period, install them
        -> Richer control / contest / hold-by-force legitimacy is deferred to
           E16b-force-legitimacy-and-jurisdiction-control.md
        -> Combatants at jurisdiction resolve naturally via existing combat system
```

**Key invariant**: Office uniqueness (9.13) — succession MUST NOT produce two simultaneous holders. The system sets the new holder atomically via `WorldTxn`.

### 9. Public Order (Extensible Permille Aggregator)

`public_order(place, world) -> Permille` — **pure derived function, NEVER stored** (Principle 3):

```rust
pub fn public_order(place: EntityId, world: &World) -> Permille {
    let mut order = Permille(750); // Baseline: moderate order

    // E16 factor: office vacancy
    for office in offices_with_jurisdiction(place, world) {
        if office_is_vacant(office, world) {
            order = order.saturating_sub(Permille(200)); // Vacancy destabilizes
        }
    }

    // E16 factor: faction conflict
    let hostile_faction_pairs = count_hostile_faction_pairs_at(place, world);
    order = order.saturating_sub(Permille(100) * hostile_faction_pairs);

    // Extension point: E17 will add crime_factor(place, world)
    // Extension point: E19 will add guard_factor(place, world)

    order.clamp(Permille(0), Permille(1000))
}
```

**Notes for E17 spec**: Add crime factor to `public_order()` — recent crimes at place reduce order.
**Notes for E19 spec**: Add guard factor to `public_order()` — guard presence at place increases order.

**FOUNDATIONS compliance**: Uses `Permille` (not `f32`), is a derived function (not stored state), and does NOT depend on E17 crime rate or E19 guard presence — those are future extension points, not forward dependencies.

### 10. Belief-Mediated Vacancy Awareness

Vacancy propagation follows the established E14/E15 information pipeline:

1. **Office holder dies** -> death event with `SamePlace` visibility at the place where death occurs
2. **Co-located agents perceive** the death (subject to `observation_fidelity`)
3. **Agents update beliefs** about the holder's alive status -> `BelievedEntityState.alive = false`
4. **Discovery events** fire when agents who believed the holder was alive observe the death -> `MismatchKind::AliveStatusChanged`
5. **News spreads via Tell** — agents with `GoalKind::ShareBelief` propagate the death/vacancy news through the topology
6. **Remote agents learn** of vacancy through rumor chains (source degradation applies)
7. **Goal generation** checks agent beliefs about office holder status, NOT world state

This means:
- Remote agents only learn of vacancies after information travels physically
- Agent diversity in `TellProfile` (`max_relay_chain_len`, `acceptance_fidelity`) creates varied awareness timing
- Agents may act on stale beliefs (believing office still held when it's vacant, or vice versa)

### 11. Action Payloads

Add to `ActionPayload` enum in `crates/worldwake-sim/src/action_payload.rs`:

```rust
Bribe(BribeActionPayload),
Threaten(ThreatenActionPayload),
DeclareSupport(DeclareSupportActionPayload),
```

```rust
pub struct BribeActionPayload {
    pub target: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
}

pub struct ThreatenActionPayload {
    pub target: EntityId,
}

pub struct DeclareSupportActionPayload {
    pub office: EntityId,
    pub candidate: EntityId,
}
```

## Component Registration

Add to `with_component_schema_entries!` macro in `component_schema.rs`:

| Component | Kind Predicate | Storage Field |
|-----------|---------------|---------------|
| `OfficeData` | `kind == EntityKind::Office` | `office_data` |
| `FactionData` | `kind == EntityKind::Faction` | `faction_data` |

## SystemFn Integration

Add `succession_system()` to `SystemManifest` in `system_manifest.rs`:
- Runs after needs system, before AI decision
- Domain: Political (uses `EventTag::Political`)
- Reads: `OfficeData`, `office_holder`, `support_declarations`, entity alive status
- Writes: `office_holder`, `vacancy_since`, events (`VacancyEvent`, `InstallationEvent`)

## FND-01 Section H Analysis

### H.1 Information-Path Analysis

| Information | Source | Path to Agent |
|-------------|--------|---------------|
| Office vacancy | Holder death event | SamePlace perception -> belief update -> Tell to others |
| Candidate eligibility | OfficeData + member_of | Public structure (eligibility rules) + agent's own membership |
| Support declarations | DeclareSupport event | SamePlace perception (witnesses see public declaration) |
| Bribery occurrence | Bribe event | SamePlace perception -> WitnessedObligation social observation |
| Coercion occurrence | Threaten event | SamePlace perception -> WitnessedConflict social observation |
| Succession result | InstallationEvent | SamePlace at jurisdiction -> Tell propagation |

All paths require co-location or physical carrier. No information teleports. (Principle 7: Locality)

### H.2 Positive-Feedback Analysis

**Loop 1 — Bandwagon Effect**: More supporters -> perceived legitimacy -> more supporters declaring for the same candidate.
- Amplifier: Social observations (`WitnessedCooperation`) make undecided agents more likely to back the leading candidate.

**Loop 2 — Bribery Spiral**: Office holder -> access to resources -> bribery -> loyalty -> support -> office retention.
- Amplifier: Incumbent advantage through resource access.

### H.3 Concrete Dampeners

**Loop 1 dampeners (Bandwagon)**:
- **Timed succession period**: Finite window limits bandwagon acceleration — the amplifying loop has a fixed number of ticks to operate
- **Agent diversity (Principle 20)**: Different loyalty weights, faction loyalties, and courage values prevent unanimous convergence — some agents have strong pre-existing loyalties that resist bandwagon pressure
- **Information locality (Principle 7)**: Remote agents can't observe the bandwagon; they decide based on partial/stale information — the feedback loop only operates within the perception radius of co-located agents
- **Faction membership**: Eligibility restrictions fragment the candidate pool — factional allegiances create competing bandwagons that counteract each other

**Loop 2 dampeners (Bribery Spiral)**:
- **Resource exhaustion**: Bribery consumes actual goods (conservation invariant enforced) — the incumbent's resource pool is finite and depletes with each bribe
- **Social cost**: Bystanders witness bribery (`WitnessedObligation`), may decrease their loyalty to briber — the act of bribery undermines the briber's standing with non-bribed observers
- **Competing demands**: Goods used for bribery can't be used for eating, trading, or other needs (opportunity cost via action occupancy) — the agent's homeostatic needs compete for the same resources
- **Threat of Force**: Competitors can use Force succession or combat to challenge an entrenched briber — military power provides an alternative path that bypasses accumulated loyalty

### H.4 Stored State vs Derived

**Stored (authoritative)**:
- `OfficeData` component (title, succession_law, eligibility, vacancy_since)
- `FactionData` component (name, purpose)
- `office_holder` relation (1:1) — already exists
- `member_of` relation (N:N) — already exists
- `loyal_to` relation (N:N weighted Permille) — already exists
- `hostile_to` relation (N:N) — already exists
- `support_declarations` relation ((agent, office) -> candidate) — new
- `courage` field in `UtilityProfile` component — new

**Derived (recomputed on query, never stored)**:
- `public_order(place)` — computed from office/faction state
- Eligibility check — computed from eligibility rules + member_of relation
- Support count per candidate — computed from support_declarations
- Candidate ranking — computed from support counts
- Goal generation (ClaimOffice, SupportCandidate) — computed from beliefs

## Invariants Enforced

- 9.13: Office uniqueness — each office has at most one holder at a time
- Conservation: goods used in bribery are transferred, not created/destroyed
- Belief-only planning: goal generation uses beliefs, never world state (Principle 10)
- Determinism: all storage uses BTreeMap/BTreeSet, all values use Permille (no floats)
- Information locality: vacancy news propagates physically through place graph (Principle 7)

## Tests

- [ ] T11: Office uniqueness — succession cannot produce two simultaneous holders
- [ ] Vacancy event emitted on holder death
- [ ] `vacancy_since` set correctly on holder death
- [ ] Support succession: candidate with most declarations wins after timed period
- [ ] Support succession: tied vote extends period
- [ ] Force succession: agent who defeats others and occupies location wins
- [ ] Eligibility: only faction members can claim office with `FactionMember` rule
- [ ] Bribe action: goods transfer + loyalty increase
- [ ] Bribe action: conservation invariant — goods transferred, not destroyed
- [ ] Bribe visibility: bystanders witness obligation
- [ ] Threaten action: yield when actor combat advantage exceeds target courage
- [ ] Threaten action: resist when target courage exceeds actor combat advantage
- [ ] Threaten action: hostility set on resist
- [ ] Threaten visibility: bystanders witness conflict
- [ ] DeclareSupport: overwrites previous declaration for same office
- [ ] DeclareSupport: requires agent at jurisdiction and vacancy
- [ ] DeclareSupport: belief-mediated — agent must believe office is vacant
- [ ] Public order: decreases during vacancy, uses Permille not f32
- [ ] Public order: decreases with hostile faction pairs
- [ ] Public order: is derived (never stored), recomputed each query
- [ ] Belief-mediated vacancy: remote agents don't know about vacancy until Tell propagates
- [ ] AI goal generation: ClaimOffice generated only for eligible agents who believe office is vacant
- [ ] AI goal generation: SupportCandidate generated based on loyalty to eligible candidate
- [ ] No scripted succession (spec section 8)
- [ ] Faction entity creation with FactionData component
- [ ] Member_of relation correctly tracks faction membership (reuses existing relation)
- [ ] Courage field enables agent diversity in threaten response

## Acceptance Criteria

- Offices with explicit succession laws (Support and Force)
- Factions as full entities with `FactionData` component
- Bribe, Threaten, and DeclareSupport as full ActionDefs with handlers
- Public order as extensible Permille-valued derived function (never stored)
- Belief-mediated vacancy awareness (no information teleportation)
- AI goals for claiming offices and supporting candidates via GOAP planner
- All existing relations reused (no duplication of member_of, loyal_to, etc.)
- No scripted succession — outcomes emerge from agent actions
- Section H analysis complete (information paths, feedback loops, dampeners, stored vs derived)

## Cross-Spec Notes

**For E17 spec**: Extend `public_order()` with crime factor — recent crimes at place reduce order.

**For E19 spec**: Extend `public_order()` with guard factor — guard presence at place increases order.

**For E16b spec**: Replace the minimal E16 force branch with explicit claim, control, contest, and uncontested-hold state. Do not preserve both paths long-term.

**Implementation order**: E16 remains at Step 10 in `IMPLEMENTATION-ORDER.md`. E16b now follows as the explicit force-legitimacy extension for contested control; public order's E17/E19 factors remain extension points, not dependencies.

## Critical Files to Modify

| File | Change |
|------|--------|
| `crates/worldwake-core/src/utility_profile.rs` | Add `courage: Permille` field |
| `crates/worldwake-core/src/entity.rs` | Already has Office/Faction kinds — no change |
| `crates/worldwake-core/src/component_tables.rs` | Add `office_data` and `faction_data` storage fields |
| `crates/worldwake-core/src/component_schema.rs` | Add OfficeData, FactionData entries |
| `crates/worldwake-core/src/relations.rs` | Add `support_declarations` storage |
| `crates/worldwake-core/src/goal.rs` | Add `ClaimOffice`, `SupportCandidateForOffice` variants |
| `crates/worldwake-core/src/event_tag.rs` | Already has Political and Social — no change needed |
| `crates/worldwake-sim/src/action_payload.rs` | Add Bribe, Threaten, DeclareSupport payloads |
| `crates/worldwake-sim/src/action_def_registry.rs` | Register new actions |
| `crates/worldwake-sim/src/system_manifest.rs` | Add `succession_system` |
| `crates/worldwake-systems/src/` | New modules: `offices.rs` (succession system), `office_actions.rs` (bribe, threaten, declare_support handlers) |
| `crates/worldwake-ai/src/planner_ops.rs` | Add Bribe, Threaten, DeclareSupport ops |
| `crates/worldwake-ai/src/goal_model.rs` | Add ClaimOffice, SupportCandidateForOffice mapping |
| `crates/worldwake-ai/src/candidate_generation.rs` | Add office/faction goal candidate emission |

## Spec References

- Section 3.9 (social institutions as first-class systems)
- Section 4.5 (office succession, faction loyalty/support)
- Section 7.4 (institutional propagation: vacancy, legitimacy, loyalty, enforcement)
- Section 9.13 (office uniqueness)
- Section 8 (no leader replacement cutscene)
- `docs/FOUNDATIONS.md` Principles 3, 7, 8, 10, 12, 20
