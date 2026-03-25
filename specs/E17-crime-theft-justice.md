# E17: Crime, Theft & Justice

## Summary

Implement theft as an explicit action, crime evidence as concrete world state, an accusation system backed by institutional records, and Fine/Exile punishment actions. Crime discovery builds on S27's expectation-violation pipeline: when an owner visits the place where they believed their item was, `EntityMissing` fires, investigation confirms suspected theft, and the existing `ShareBelief`/Tell pipeline propagates the suspicion to authorities. This is the final epic before the Phase 3 gate.

## Phase

Phase 3: Information & Politics (Step 13)

## Crate

- `worldwake-core` (new types: profiles, violation extension, institutional claim extension, goal kinds)
- `worldwake-systems` (steal action, accuse action, fine/exile actions, investigate commit extension)
- `worldwake-ai` (theft and justice candidate generation, planner ops, goal policy, ranking, feasibility)

## Dependencies

- E15 (social transmission -- `ShareBelief`/Tell for crime report propagation, `VisibilitySpec::Hidden` for unwitnessed events)
- S01 (production output ownership -- `can_exercise_control()`, `believed_owner_of()`, ownership-gated pickup)
- S03 (planner target identity & affordance binding -- `matches_binding()` for exact-bound theft/accusation goals)
- E16c (institutional beliefs & records -- `RecordData`, `InstitutionalClaim`, append-only record architecture)
- S27 (expectation-violation goals -- `ViolationKind::EntityMissing`, `InvestigateViolation`, `ViolationMemory`, `ViolationDispositionProfile`)

## Dependency Note

S27 delivers belief-vs-observation violation detection and reactive investigation goals. E17 extends the investigation commit handler: when investigation confirms that a missing entity was owned by the investigator, the outcome is recorded as `SuspectedTheft` rather than generic `WitnessedAbsence`. This bridges the gap between S27's generic mismatch detection and crime-specific institutional response.

S01 ensures that production output has explicit ownership. Without S01, taking an unowned item is lawful `pick_up` and theft has no meaning. E17's Steal action is architecturally the complement of S01's ownership-gated pickup: `pick_up` works when `can_exercise_control()` is true; Steal works when it is false.

E16c provides the institutional record architecture. Accusations and verdicts follow the same append-only, supersede-aware `RecordData` pattern as `OfficeRegister`, `FactionRoster`, and `SupportLedger`.

## FOUNDATIONS Alignment

- **P1** (Maximal emergence through local causality): Theft, discovery, accusation, and punishment all arise from interacting systems without authored quest logic. The canonical regression scenario C (stored gold -> empty stash -> discovery -> robbery report) is directly realized through general-purpose systems.
- **P2** (No ungrounded triggers or probabilities): All durations, tolerances, and motive weights come from per-agent profiles (`TheftDispositionProfile`, `JusticeDispositionProfile`). No `crimeChance`, `stealthScore`, or `evidenceWeight` constants.
- **P3** (Concrete state over abstract scores): No abstract evidence weight scoring system. Evidence IS concrete world state: an agent who witnessed the theft event (in their `AgentBeliefStore`), an ownership mismatch discovered through investigation (`SuspectedTheft` in `ViolationMemory`), an item observed in the thief's possession. Accusation requires the accuser to hold specific concrete evidence.
- **P4** (Persistent identity, object permanence, explicit transfer): Steal explicitly transfers possession without transferring ownership. Conservation maintained. The stolen item continues to exist with unchanged identity.
- **P7** (Locality of motion, interaction, and communication): Theft witnessing requires co-location plus perception. Crime discovery requires the owner to physically visit the location and observe the mismatch. Accusation requires physical travel to the CrimeRegister's `home_place`. No remote detection.
- **P8** (Every action has preconditions, duration, cost, and occupancy): Steal has preconditions (co-location, item owned by other, capacity), profile-driven duration, occupies the agent, and is interruptible. Fine and Exile require institutional authority and co-location with the accused.
- **P9** (Outcomes are granular and leave aftermath): Steal creates: possession transfer, event log entry with `EventTag::Crime`, `VisibilitySpec::Hidden`. A failed or interrupted theft produces no transfer but may generate noise (co-located agents may have perceived the attempt). Discovery creates `SocialObservation(SuspectedTheft)`. Accusation creates an institutional record entry. Exile creates `hostile_to` relation and removes faction membership.
- **P10** (Every positive feedback loop needs a physical dampener): See FND-01 Section H.
- **P12** (World state is not belief state): The thief's identity is unknown to the victim until concrete evidence emerges through lawful channels. The accuser's evidence is validated from their `AgentBeliefStore`, not from world truth. Wrong accusations are possible.
- **P13** (Knowledge acquired locally and travels physically): Theft knowledge travels through: (a) direct witness observation at co-location, (b) owner's violation detection at crime scene, (c) `ShareBelief`/Tell chain from witness to owner or authority.
- **P14** (Ignorance, uncertainty, contradiction are first-class): Owner may suspect theft but not know the thief (`suspect: None`). Multiple agents may hold conflicting accounts. Wrong accusations are possible if evidence is circumstantial.
- **P15** (Surprise comes from violated expectation): S27's `EntityMissing` violation fires when the owner visits the stash and observes a mismatch between their belief ("my gold is here") and reality ("no gold"). E17 extends the investigation outcome with `SuspectedTheft` when ownership mismatch is confirmed.
- **P16** (Memory, evidence, records are world state): Accusations are `InstitutionalClaim` entries in a `CrimeRegister` `RecordData` entity. Witness memories are `SocialObservation` entries in `AgentBeliefStore`. All are inspectable, shareable, and persistent.
- **P17** (Agent symmetry): Human-controlled agents can steal, be stolen from, accuse, and be punished identically to AI-controlled agents. `ControlSource` changes nothing.
- **P20** (Agent diversity through concrete variation): `TheftDispositionProfile` and `JusticeDispositionProfile` create per-agent variation in theft duration, risk tolerance, accusation motive, and fine severity.
- **P21** (Roles, offices, institutions are world state): Punishment actions (Fine, Exile) require institutional authority -- the punisher must hold an office with jurisdiction. A vacant office means no one can punish.
- **P22** (Ownership, custody, access, obligation, jurisdiction are distinct): Steal transfers possession, not ownership. This is the core mechanism. The system correctly models "the gold is the guild's, but the thief has it."
- **P23** (Social artifacts are first-class): Accusations are institutional records with issuer, accused, evidence references, place of filing, and supersede chain. They can be consulted, contested, and superseded.
- **P24** (Systems interact through state, not through each other): The crime-to-punishment chain propagates through state mutations and event history: steal mutates possession relations -> violation detection reads belief state -> investigation mutates violation memory -> accusation mutates institutional records -> punishment mutates relations and possession. No direct cross-system calls.
- **P27** (Debuggability is a product feature): All crime evidence is inspectable world state. Decision traces show why agents steal, accuse, or punish. Action traces show steal/accuse/fine/exile execution and outcomes. "Why was this agent accused?" is answerable from the CrimeRegister entries and the accuser's belief store.
- **P28** (Every new system spec must declare its causal hooks): See detailed design below and FND-01 Section H.

## Motivation

### What exists today

- **S27 violation detection**: `ViolationKind::EntityMissing` fires when an agent visits a place where they expected an entity. `InvestigateViolation { violation_id, place }` goal triggers reactive investigation. Investigation commits `SocialObservation(WitnessedAbsence)` confirming the absence. `ViolationMemory` stores recorded violations with TTL expiry. `ViolationDispositionProfile` provides per-agent investigation parameters.
- **S01 ownership gating**: `can_exercise_control(actor, entity)` gates `pick_up`. Items owned by others cannot be lawfully picked up. `believed_owner_of()` provides the belief-side ownership query for AI planning. `ProductionOutputOwnershipPolicy` assigns ownership at harvest/craft time.
- **E15 social transmission**: `ShareBelief`/Tell propagates observations between co-located agents. `VisibilitySpec::Hidden` makes events invisible to non-participants. The perception system evaluates co-located agents against event visibility using `PerceptionProfile`.
- **E16c institutional records**: `RecordData` with `InstitutionalClaim` entries and supersedes chains. `RecordKind` has three values (`OfficeRegister`, `FactionRoster`, `SupportLedger`). Records are append-only institutional memory at specific places.
- **`EventTag::Crime`**: Already exists in the `EventTag` enum but is currently unused.
- **Existing action domains**: `ActionDomain` has `Generic`, `Needs`, `Production`, `Trade`, `Social`, `Travel`, `Transport`, `Combat`, `Care`, `Corpse`.

### What is missing

- No action allows taking items owned by others (Steal). The only way to acquire owned items is lawful `pick_up` (requires `can_exercise_control`) or `trade`.
- No mechanism connects `EntityMissing` violations to suspected theft. Investigation confirms absence but does not distinguish "item depleted" from "item stolen."
- No accusation action or institutional crime record. There is no way for an agent to formally accuse another.
- No punishment actions (Fine, Exile). Institutional authority cannot impose consequences for crime.
- No per-agent profiles for theft or justice behavior. Crime-related agent diversity does not exist.
- No crime-specific AI candidate generation. Agents cannot form theft, accusation, or punishment goals.

## Design

### New Types in worldwake-core

#### TheftDispositionProfile

```rust
/// Per-agent parameters governing theft behavior.
/// Only agents with this component consider theft goals.
/// Enables agent diversity (P20) for crime disposition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TheftDispositionProfile {
    /// Duration in ticks for the steal action. Per-agent deftness/caution.
    pub steal_duration_ticks: NonZeroU32,
    /// Base motive weight for theft goals. Higher = more inclined to steal.
    pub theft_motive_weight: Permille,
    /// Motive penalty per co-located observer (risk aversion).
    /// Subtracted from motive per co-located non-self agent.
    /// Permille(0) = no risk aversion (brazen thief).
    pub witness_risk_penalty: Permille,
}
```

Lives in a new `worldwake-core/src/crime.rs` module. Registered as an Agent-only component in `component_schema.rs`. Most agents will NOT have this component -- only agents who may consider theft.

#### JusticeDispositionProfile

```rust
/// Per-agent parameters governing accusation and punishment behavior.
/// Only agents with this component consider justice goals.
/// Enables agent diversity (P20) for justice disposition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JusticeDispositionProfile {
    /// Motive weight for accusation goals when the agent has evidence.
    pub accusation_motive_weight: Permille,
    /// Fine amount as fraction of the stolen commodity quantity.
    /// Permille(500) = fine equals 50% of the stolen quantity.
    pub fine_severity: Permille,
}
```

Same module, same registration pattern. Agents with this profile (merchants, guards, office holders) will generate accusation and punishment goals.

#### PunishmentKind

```rust
/// The kind of punishment imposed by institutional authority.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PunishmentKind {
    /// Transfer commodity from convicted agent to faction/office treasury.
    Fine {
        commodity: CommodityKind,
        amount: Quantity,
    },
    /// Remove agent from faction and mark as hostile.
    Exile {
        from_faction: EntityId,
    },
}
```

Same module.

#### ViolationKind Extension

```rust
pub enum ViolationKind {
    // ... existing EntityMissing, SupplyDepleted, EntityDead ...

    /// Investigation revealed ownership mismatch: agent's owned item is
    /// missing and suspected taken by another agent.
    SuspectedTheft {
        missing_entity: EntityId,
        expected_place: EntityId,
        /// Suspect, if known. Some(entity) when accuser witnessed the theft
        /// event or observed the stolen item in someone else's possession.
        /// None when the owner knows something was stolen but not by whom.
        suspect: Option<EntityId>,
    },
}
```

Extends the existing `ViolationKind` enum in `violation.rs`.

`SuspectedTheft` is created when an `InvestigateViolation` resolves and the investigating agent owned the missing entity (via `believed_owner_of`). The `suspect` field starts as `None` and is updated to `Some(entity)` when the owner gains evidence identifying a specific agent:
- A witness shared the theft event via Tell (the witness observed the `Hidden` theft event at co-location).
- The owner or any agent observed the stolen item in the thief's possession (normal perception).

#### SocialObservationKind Extension

```rust
pub enum SocialObservationKind {
    // ... existing WitnessedCooperation, WitnessedConflict,
    //     WitnessedObligation, WitnessedTelling, CoPresence, WitnessedAbsence ...

    /// Investigation confirmed ownership mismatch -- suspected theft.
    SuspectedTheft,
}
```

Extends the existing enum in `belief.rs`. `SocialObservationKind` is only the family tag; the concrete payload must live in typed `SocialObservationDetail`, not overloaded tuple slots. Sharing this evidence through Tell is a separate extension, not something the existing entity-belief Tell pipeline can already do.

#### RecordKind Extension

```rust
pub enum RecordKind {
    // ... existing OfficeRegister, FactionRoster, SupportLedger ...

    /// Crime register at a jurisdictional office. Stores accusations and verdicts.
    CrimeRegister,
}
```

Extends the existing enum in `institutional.rs`.

#### InstitutionalClaim Extension

```rust
pub enum InstitutionalClaim {
    // ... existing OfficeHolder, FactionMembership, SupportDeclaration, ForceControl ...

    /// Formal accusation of theft filed at a jurisdictional office.
    Accusation {
        accuser: EntityId,
        accused: EntityId,
        violation_id: ViolationId,
        effective_tick: Tick,
    },
    /// Punishment verdict recorded after accusation resolution.
    Verdict {
        accused: EntityId,
        punishment: PunishmentKind,
        effective_tick: Tick,
        /// The accusation entry this verdict resolves.
        supersedes_accusation: RecordEntryId,
    },
}
```

Extends the existing enum in `institutional.rs`. Follows the same append-only, supersede-aware pattern as existing institutional claims.

#### GoalKind Extensions

```rust
pub enum GoalKind {
    // ... existing 19 variants ...

    /// Steal an item owned by another agent.
    StealItem {
        target_item: EntityId,
    },
    /// Formally accuse a suspect of theft at the jurisdictional office.
    Accuse {
        accused: EntityId,
        violation_id: ViolationId,
    },
    /// Punish an accused agent (requires institutional authority and co-location).
    PunishAccused {
        accused: EntityId,
        punishment: PunishmentKind,
    },
}
```

`StealItem` takes a specific target item because the planner needs to know what to steal for precondition checking, travel planning, and possession transfer. Follows the same pattern as `LootCorpse { corpse }`.

`Accuse` and `PunishAccused` are separate goals because they have distinct preconditions: accusation requires evidence plus travel to the office; punishment requires institutional authority plus co-location with the accused. Separating them allows the AI to plan multi-step justice chains.

### Steal Action

- **Name**: `steal`
- **Domain**: `ActionDomain::Transport` (fundamentally a transport operation that bypasses ownership gating)
- **Actor constraints**: `ActorAlive`, `ActorNotIncapacitated`
- **Target**: `TargetSpec::SpecificEntity` (the target `ItemLot`)
- **Preconditions** (belief-level, checked by affordance generation):
  - Actor and target at same place
  - Target is `EntityKind::ItemLot`
  - Target has an owner other than the actor
  - Actor cannot exercise control over the target (if they can, `pick_up` is the lawful path)
  - Target is not currently possessed by another agent (stealing from someone's hands is robbery -- a combat action, out of scope)
  - Target is not reserved
  - Actor has remaining load capacity for the item's weight
- **Duration**: `DurationExpr::ProfileDriven` -- reads `TheftDispositionProfile.steal_duration_ticks`. Multi-tick, interruptible.
- **Interruptibility**: `Interruptibility::FreelyInterruptible`
- **Visibility**: `VisibilitySpec::Hidden`
- **Causal event tags**: `BTreeSet::from([EventTag::Crime, EventTag::Transfer])`
- **Payload**: `ActionPayload::None` (target item identity is in `instance.targets`)

**Handler on start**: Validate all preconditions authoritatively. Re-verify that actor cannot exercise control over the item (S01 check). If any precondition fails, return `StartFailed` for the existing S08 best-effort recovery path.

**Handler on tick**: Standard tick progression toward `steal_duration_ticks`.

**Handler on commit**:
1. Transfer possession: `txn.set_possessor(target_item, actor)`. Ownership relation remains unchanged.
2. Update placement: item moves to actor's direct possession (same relation mutation as `pick_up` commit, but without the `can_exercise_control` gate).
3. Emit event with `EventTag::Crime`, `VisibilitySpec::Hidden`, `WitnessData` containing actor as sole direct participant.
4. The perception system (running in the same tick, after action processing) evaluates whether any co-located agent witnesses this `Hidden` event based on their `PerceptionProfile`.

**Handler on abort**: No-op. An interrupted theft produces no possession transfer. (P9: partial failure = "nothing happened yet.")

**Architectural note**: Steal shares validation logic with `transport_actions::pick_up` but must NOT call `can_exercise_control()` as a precondition. The key difference: `pick_up` requires `can_exercise_control() == true`; `steal` requires `can_exercise_control() == false` and the actor has a `TheftDispositionProfile`.

### Crime Discovery Pipeline (S27 Integration)

The existing S27 pipeline handles most of crime discovery without modification:

1. **Owner visits stash**: Owner arrives at the place where they believed their item was. Perception refresh does NOT show the item. S27's `ViolationKind::EntityMissing` violation fires from the belief-observation mismatch.
2. **Owner investigates**: S27's `InvestigateViolation { violation_id, place }` goal triggers investigation.
3. **Investigation commit (EXTENDED by E17)**: The existing investigate handler commits `SocialObservation(WitnessedAbsence)`. E17 extends this commit path: if the investigating agent owned the missing entity (`believed_owner_of(missing_entity) == Some(investigating_agent)`), the handler ALSO:
   - Records `ViolationKind::SuspectedTheft { missing_entity, expected_place, suspect: None }` in the agent's `ViolationMemory` (with TTL from `ViolationDispositionProfile.violation_memory_retention_ticks`).
   - Records `SocialObservation(SuspectedTheft)` in the agent's `AgentBeliefStore` with typed theft detail carrying `missing_entity`, `expected_place`, and `suspect: None`.
   - The `suspect` field is `None` at this point -- the owner knows something was stolen but not by whom.

**Suspect identification** occurs through concrete evidence arriving via lawful channels:

- **Direct witness**: If another agent was co-located during the theft and the perception system resolved the `Hidden` event to them, that witness knows the thief's identity. Via `ShareBelief`/Tell (E15), this information can reach the owner or authorities. When the owner receives this testimony and it matches their `SuspectedTheft` violation, the `suspect` field is updated.
- **Possession observation**: If the owner or any agent observes the stolen item in another agent's possession (normal co-location perception -- the item is visibly carried), the observer can identify the suspect. This updates the `SuspectedTheft` violation's `suspect` field.

**No abstract stealth model**: Detection is entirely handled by the existing perception system applied to `VisibilitySpec::Hidden` events. The perception system already uses `PerceptionProfile` to determine if co-located agents notice events. No separate stealth score or stealth-vs-awareness comparison is introduced.

### Accusation System

#### Accuse Action

- **Name**: `accuse`
- **Domain**: `ActionDomain::Social`
- **Preconditions** (belief-level):
  - Actor at the same place as a `CrimeRegister` record entity (the jurisdictional office's crime register)
  - Actor has concrete evidence: (a) witnessed the theft event (crime-tagged event observation in their `AgentBeliefStore`), OR (b) completed `InvestigateViolation` that resulted in `SuspectedTheft` with a known `suspect`, OR (c) observed the stolen item in the accused's possession
  - The accused entity is believed alive
  - No existing unresolved accusation against the same accused for the same violation exists in the `CrimeRegister` (checked via record consultation)
- **Duration**: `DurationExpr::Fixed(NonZeroU32::new(1).unwrap())` -- filing an accusation is a brief administrative act
- **Visibility**: `VisibilitySpec::SamePlace` -- the accusation is a public act at the office
- **Causal event tags**: `BTreeSet::from([EventTag::Social, EventTag::Crime])`

**Handler on commit**:
1. Append `InstitutionalClaim::Accusation { accuser, accused, violation_id, effective_tick }` to the `CrimeRegister` `RecordData` entity at the current place.
2. Emit event with `EventTag::Crime` and `VisibilitySpec::SamePlace`.

**Evidence validation** (authoritative): The handler verifies that the accuser's `AgentBeliefStore` contains at least one of: (a) typed theft evidence identifying the accused, (b) a witnessed crime event where the perpetrator matches the accused, (c) a belief that the stolen item is in the accused's possession. This check uses the accuser's belief store (P12). Wrong accusations are possible if the accuser's evidence is flawed.

### Punishment Actions

#### Fine Action

- **Name**: `fine`
- **Domain**: `ActionDomain::Social`
- **Preconditions** (belief-level):
  - Actor holds an office with jurisdiction over the place where the CrimeRegister exists (institutional authority via `office_holder` or `office_controller` relation)
  - An unresolved `Accusation` entry exists in the `CrimeRegister` against the target
  - Actor and accused at same place (accused must be physically present)
  - Accused possesses sufficient commodity to pay the fine
- **Duration**: `DurationExpr::Fixed(NonZeroU32::new(1).unwrap())`
- **Visibility**: `VisibilitySpec::SamePlace`
- **Causal event tags**: `BTreeSet::from([EventTag::Social, EventTag::Crime, EventTag::Transfer])`

**Handler on commit**:
1. Calculate fine amount: `JusticeDispositionProfile.fine_severity` applied to the stolen commodity kind and quantity from the accusation's referenced violation.
2. Transfer commodity lots from accused to the faction/office treasury entity. Conservation is maintained -- this is an explicit transfer, not destruction.
3. Supersede the `Accusation` entry with a `Verdict { accused, punishment: Fine { commodity, amount }, effective_tick, supersedes_accusation }` entry in the `CrimeRegister`.
4. Emit event.

#### Exile Action

- **Name**: `exile`
- **Domain**: `ActionDomain::Social`
- **Preconditions** (belief-level):
  - Actor holds an office with jurisdiction (same check as Fine)
  - An unresolved `Accusation` entry exists in the `CrimeRegister` against the target
  - Accused is a member of a faction the office controls
- **Duration**: `DurationExpr::Fixed(NonZeroU32::new(1).unwrap())`
- **Visibility**: `VisibilitySpec::SamePlace`
- **Causal event tags**: `BTreeSet::from([EventTag::Social, EventTag::Crime, EventTag::Political])`

**Handler on commit**:
1. Remove accused from faction membership (mutate `member_of` relation).
2. Add `hostile_to(faction, accused)` relation -- the faction is now hostile to the exile.
3. Supersede the `Accusation` entry with a `Verdict { accused, punishment: Exile { from_faction }, effective_tick, supersedes_accusation }`.
4. Emit event.

### AI Integration

#### Theft Candidate Generation

New function `emit_theft_candidates()` in `candidate_generation.rs` (new `emit_*` family):

**Guard**: Only runs if the agent has a `TheftDispositionProfile` component. Agents without this profile never consider theft.

**Algorithm**:
1. Query co-located `ItemLot` entities at the agent's current place.
2. For each item lot: check `believed_owner_of(item) != Some(self)` AND `can_exercise_control(self, item) == false` in the belief view.
3. Filter: item not currently possessed by another agent, item not reserved, agent has load capacity.
4. Calculate motive: `theft_motive_weight - (witness_risk_penalty * co_located_agent_count)`. If motive <= 0, skip.
5. Emit `GroundedGoal` with `GoalKind::StealItem { target_item }`.

**Goal priority class**: `GoalPriorityClass::Low` (opportunistic, below survival/combat/danger).

**Suppression**: Suppressed when `WhenStressedAtOrAbove(GoalPriorityClass::Medium)`. Do not steal under moderate or higher stress.

**Knowledge path**: `KnowledgePath::DirectObservation` for the item lot's location and ownership.

#### Justice Candidate Generation

New function `emit_justice_candidates()` in `candidate_generation.rs` (new `emit_*` family):

**Guard**: Only runs if the agent has a `JusticeDispositionProfile` component.

**Algorithm for accusation candidates**:
1. Scan agent's `ViolationMemory` for `ViolationKind::SuspectedTheft` entries with `suspect: Some(entity)`.
2. For each such violation: check that no existing accusation has been filed (requires knowledge of the nearest `CrimeRegister` contents -- via prior record consultation or belief).
3. Emit `GroundedGoal` with `GoalKind::Accuse { accused: suspect, violation_id }`.

**Algorithm for punishment candidates**:
1. Scan agent's known `CrimeRegister` entries (from prior consultation observations in `AgentBeliefStore`) for unresolved `Accusation` entries.
2. For each: check if the agent holds institutional authority (office holder/controller with jurisdiction).
3. Determine punishment kind: if the accused has commodities, prefer `Fine`; otherwise `Exile`.
4. Emit `GroundedGoal` with `GoalKind::PunishAccused { accused, punishment }`.

**Goal priority class**: `GoalPriorityClass::Low`.

**Motive**: From `JusticeDispositionProfile.accusation_motive_weight`.

#### Planner Ops

New `PlannerOpKind` variants:

- `Steal` -- maps to steal action. Terminal for `StealItem` goal. Barriers: item no longer at place, item now possessed by another, actor at wrong place.
- `Accuse` -- maps to accuse action. Terminal for `Accuse` goal. Barriers: not at CrimeRegister, accusation already filed.
- `Fine` -- maps to fine action. Terminal for `PunishAccused { punishment: Fine { .. } }` goal. Barriers: no unresolved accusation, accused not present.
- `Exile` -- maps to exile action. Terminal for `PunishAccused { punishment: Exile { .. } }` goal. Barriers: no unresolved accusation, accused not a faction member.

#### GoalKindTag Extensions

```rust
pub enum GoalKindTag {
    // ... existing 19 tags ...
    StealItemTag,
    AccuseTag,
    PunishAccusedTag,
}
```

#### Goal Policy

- `StealItem`: family = `Crime`. Suppression = `WhenStressedAtOrAbove(Medium)`. Not a critical survival goal. Not a reactive goal. Free-interrupt role = false.
- `Accuse`: family = `Justice`. Suppression = `WhenStressedAtOrAbove(Medium)`. Not a critical survival goal. Not reactive. Free-interrupt = false.
- `PunishAccused`: family = `Justice`. Same suppression. Not critical. Not reactive. Free-interrupt = false.

#### Feasibility Hints

- `StealItem`: `Likely` if agent co-located with target item and has capacity. `Uncertain` if item is at a known remote place. `Unlikely` if no known target items.
- `Accuse`: `Likely` if agent has `SuspectedTheft` with known suspect and knows a `CrimeRegister` location. `Uncertain` otherwise.
- `PunishAccused`: `Likely` if agent has institutional authority and knows of unresolved accusations. `Uncertain` otherwise.

#### Binding (S03)

- `StealItem { target_item }`: exact-bound. `matches_binding()` rejects affordances targeting a different item entity.
- `Accuse { accused, violation_id }`: exact-bound on `accused`. Rejects affordances targeting a different accused.
- `PunishAccused { accused, punishment }`: exact-bound on `accused`. Rejects affordances targeting a different accused.

### CrimeRegister Record Entity

A `RecordData` entity of kind `RecordKind::CrimeRegister` must exist at each jurisdiction that can process accusations. This follows the same pattern as existing institutional records:

- `record_kind: RecordKind::CrimeRegister`
- `home_place`: the place where accusations are filed (typically the office's seat)
- `entries: Vec<InstitutionalRecordEntry>` with `InstitutionalClaim::Accusation` and `InstitutionalClaim::Verdict` entries

CrimeRegisters are created as part of world setup (same as `OfficeRegister`, `FactionRoster`, `SupportLedger` entities). Each office that has law-enforcement jurisdiction gets a CrimeRegister at its seat.

### Integration with Existing Systems

#### Perception System (E14)

No changes needed. The perception system already handles `VisibilitySpec::Hidden` events by evaluating co-located agents' `PerceptionProfile`. Theft events with `Hidden` visibility will be evaluated by the existing perception pipeline.

#### ShareBelief/Tell (E15)

Changes needed. The live Tell path only relays entity-belief subjects, not `SocialObservation` payloads. Witness-driven theft testimony therefore requires an explicit follow-up extension that adds typed social-evidence conversation topics. Do not assume `SocialObservation(SuspectedTheft)` is already relayable through the existing Tell action.

#### Ownership (S01)

No changes needed. `can_exercise_control()` already provides the gate that distinguishes lawful `pick_up` from theft. The steal action checks `can_exercise_control() == false`.

#### Violation Detection (S27)

Minimal extension: the investigate commit handler gains a conditional branch that checks ownership of the missing entity and records `SuspectedTheft` when applicable. This is a targeted extension of the existing handler, not a new system.

## Tickets

### E17-001: Core crime types in worldwake-core

Add `TheftDispositionProfile`, `JusticeDispositionProfile`, and `PunishmentKind` to a new `worldwake-core/src/crime.rs` module. Register both profiles as Agent-only components in `component_schema.rs` and `component_tables.rs`. Export from `lib.rs`. All types must derive `Clone`, `Debug`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`.

**Tests**: construction, serde round-trip, component registration for each entity kind.

**Verify**: `cargo test -p worldwake-core`, `cargo clippy -p worldwake-core`.

### E17-002: Extend ViolationKind with SuspectedTheft

Add `ViolationKind::SuspectedTheft { missing_entity, expected_place, suspect }` to `violation.rs`. Add `SocialObservationKind::SuspectedTheft` to `belief.rs`. Update any exhaustive matches on these enums. The concrete belief payload for theft evidence should be handled by typed `SocialObservationDetail`, not by tuple overloading.

**Tests**: serde round-trip for new variant, `ViolationMemory` recording of `SuspectedTheft`, `Ord` stability.

**Verify**: `cargo test -p worldwake-core`, `cargo clippy -p worldwake-core`.

### E17-003: Extend institutional claims with Accusation, Verdict, CrimeRegister

Add `InstitutionalClaim::Accusation { accuser, accused, violation_id, effective_tick }` and `InstitutionalClaim::Verdict { accused, punishment, effective_tick, supersedes_accusation }` to `institutional.rs`. Add `RecordKind::CrimeRegister`. Update exhaustive matches.

**Tests**: serde round-trip, append/supersede with new claim types, `RecordData` construction with `CrimeRegister`.

**Verify**: `cargo test -p worldwake-core`, `cargo clippy -p worldwake-core`.

### E17-004: Add GoalKind variants and GoalKey extraction

Add `GoalKind::StealItem { target_item }`, `GoalKind::Accuse { accused, violation_id }`, `GoalKind::PunishAccused { accused, punishment }` to `goal.rs`. Update `GoalKey::from(GoalKind)` for each variant: `StealItem` extracts `target_item` as entity, `Accuse` extracts `accused`, `PunishAccused` extracts `accused`.

**Tests**: `GoalKey` extraction, serde round-trip, identity.

**Verify**: `cargo build --workspace`, `cargo clippy --workspace`.

### E17-005: Planner support for new goal kinds

In `worldwake-ai`:
- Add `GoalKindTag::StealItemTag`, `GoalKindTag::AccuseTag`, `GoalKindTag::PunishAccusedTag` to `goal_model.rs`.
- Add `PlannerOpKind::Steal`, `PlannerOpKind::Accuse`, `PlannerOpKind::Fine`, `PlannerOpKind::Exile` to `planner_ops.rs`. Implement `PlannerOpSemantics` for each.
- Implement `GoalKindPlannerExt` for each new goal kind (terminal operator, `matches_binding()`).
- Add goal policy for each in `goal_policy.rs` (family, suppression, interrupt class).
- Add ranking logic in `ranking.rs` (`GoalPriorityClass::Low` for all three).
- Add `FeasibilityHint` dispatch for each.

**Tests**: focused unit tests for binding acceptance/rejection, goal policy evaluation, ranking ordering.

**Verify**: `cargo build --workspace`, `cargo clippy --workspace`.

### E17-006: Implement steal action in worldwake-systems

New `steal_actions.rs` module. `register_steal_action()` following the `register_investigate_action()` pattern. Action definition with `VisibilitySpec::Hidden`, `EventTag::Crime`, profile-driven duration from `TheftDispositionProfile.steal_duration_ticks`.

- Start handler: validate preconditions authoritatively (co-location, item owned by other, `can_exercise_control == false`, item not possessed by another, item not reserved, load capacity).
- Tick handler: standard duration progression.
- Commit handler: transfer possession via `set_possessor()`, emit event with Hidden visibility.
- Abort handler: no-op.
- Register in `register_all_actions()`.

**Tests**: steal transfers possession not ownership, conservation maintained, Hidden visibility on event, abort produces no transfer, start-fail when actor can exercise control.

**Verify**: `cargo test -p worldwake-systems`, `cargo clippy -p worldwake-systems`.

### E17-007: Extend investigate commit with SuspectedTheft detection

Modify `investigate_actions.rs` commit handler. After existing `WitnessedAbsence` observation recording, add: if the investigating agent owned the missing entity (check `believed_owner_of()` on the agent's belief view), then:
1. Record `ViolationKind::SuspectedTheft { missing_entity, expected_place, suspect: None }` in `ViolationMemory`.
2. Record `SocialObservation(SuspectedTheft)` in `AgentBeliefStore` using typed theft detail.

**Tests**: owner investigating their own missing item produces `SuspectedTheft`; non-owner investigating does NOT produce `SuspectedTheft`; `SuspectedTheft` is recorded in both `ViolationMemory` and `AgentBeliefStore`.

**Verify**: `cargo test -p worldwake-systems`, `cargo clippy -p worldwake-systems`.

### E17-008: Implement accuse action

New `justice_actions.rs` module in `worldwake-systems`. `register_accuse_action()`.

- Preconditions: co-located with `CrimeRegister`, evidence in accuser's belief store, accused alive, no duplicate accusation.
- Commit: append `InstitutionalClaim::Accusation` to `CrimeRegister` `RecordData`.

**Tests**: accusation creates record entry, duplicate accusation rejected, accusation without evidence rejected, accusation with evidence from witness testimony accepted.

**Verify**: `cargo test -p worldwake-systems`, `cargo clippy -p worldwake-systems`.

### E17-009: Implement fine and exile actions

Add `register_fine_action()` and `register_exile_action()` to `justice_actions.rs`.

- Fine: transfer commodity from accused to treasury, supersede `Accusation` with `Verdict`, conservation maintained.
- Exile: remove `member_of` relation, add `hostile_to` relation, supersede `Accusation` with `Verdict`.

**Tests**: fine transfers commodity (conservation check), exile removes membership and adds hostility, both supersede the accusation entry, fine fails when accused has insufficient commodity.

**Verify**: `cargo test -p worldwake-systems`, `cargo clippy -p worldwake-systems`.

### E17-010: Implement emit_theft_candidates()

New function in `candidate_generation.rs`. Only for agents with `TheftDispositionProfile`. Scan co-located item lots owned by others. Apply witness risk penalty per co-located observer. Use `emit_candidate_with_trace()` with knowledge-path provenance.

**Tests**: candidate generated for stealable co-located item, no candidate when agent lacks profile, no candidate when motive reduced to zero by witnesses, no candidate for unowned items, no candidate for items agent can exercise control over.

**Verify**: `cargo test -p worldwake-ai`, `cargo clippy -p worldwake-ai`.

### E17-011: Implement emit_justice_candidates()

New function in `candidate_generation.rs`. Only for agents with `JusticeDispositionProfile`. Scan `ViolationMemory` for `SuspectedTheft` with known suspect -> emit `Accuse`. Scan known `CrimeRegister` for unresolved accusations where agent has authority -> emit `PunishAccused`.

**Tests**: accusation candidate generated when suspect known, no candidate when suspect unknown, punishment candidate generated when agent has authority and unresolved accusation exists, no candidate without authority.

**Verify**: `cargo test -p worldwake-ai`, `cargo clippy -p worldwake-ai`.

### E17-012: Golden test -- theft creates EntityMissing violation for owner

Scenario: Agent A (with `TheftDispositionProfile`) steals item from Place P. Item is owned by Agent B (not present at P). B later arrives at P. S27's `EntityMissing` violation fires. B investigates. Investigation commit detects ownership mismatch and records `SuspectedTheft`.

Proves: P15 (violated expectation from theft), P7 (local discovery only), P3 (concrete evidence), P12 (belief-state separation).

**Verify**: `cargo test -p worldwake-ai --test golden_*`.

### E17-013: Golden test -- witnessed theft enables accusation chain

Scenario: Agent A steals at Place P. Agent C (witness with `PerceptionProfile`) is co-located and witnesses the `Hidden` event. C travels to authority and shares via Tell. Authority (office holder with `JusticeDispositionProfile`) travels to `CrimeRegister` and files accusation. Authority fines A when co-located.

Proves: P1 (emergent justice chain), P7 (witness co-location), P16 (accusation as institutional record), P22 (jurisdiction requirement).

**Verify**: `cargo test -p worldwake-ai --test golden_*`.

### E17-014: Workspace verification and documentation

- `cargo test --workspace` -- all pass.
- `cargo clippy --workspace` -- no new warnings.
- Update golden coverage docs.
- Update `specs/IMPLEMENTATION-ORDER.md` to mark E17 dependencies as current.

## FND-01 Section H Analysis

### 1. Information-path analysis

How does crime information reach agents who act on it? Trace for each step:

1. **Theft occurs**: Steal action commits at Place P with `VisibilitySpec::Hidden`. Only the thief (sole direct participant) knows. No other agent has knowledge of the theft.
2. **Witness observation (if any)**: The perception system (same tick, after action commit) evaluates co-located agents at P against the `Hidden` event. If an agent at P has `PerceptionProfile` and passes perception evaluation, they observe the event and record it in their `AgentBeliefStore` as a crime-tagged event observation.
3. **Owner visits stash**: Owner B arrives at P at some future tick. Perception refresh does NOT show the stolen item at P. B's prior belief says the item was at P (via `BelievedEntityState.last_known_place`). S27's `ViolationKind::EntityMissing` violation fires from the belief-observation mismatch.
4. **Owner investigates**: B's `InvestigateViolation` goal triggers. B spends `investigation_duration_ticks` at P (from `ViolationDispositionProfile`). On commit, B confirms the absence AND detects the ownership mismatch -> records `SuspectedTheft { suspect: None }` in `ViolationMemory` and `SocialObservation(SuspectedTheft)` in `AgentBeliefStore`.
5. **Witness shares**: If witness C observed the theft (step 2), C's `ShareBelief` candidate generation fires for co-located listeners. C physically travels to B or to an authority and Tells the crime observation. This updates the listener's belief store with the thief's identity.
6. **Suspect identified by possession**: Alternatively, if any agent observes the stolen item in A's possession (normal co-location perception at any place), they can identify the suspect. The observer's `SuspectedTheft` entry gets `suspect: Some(A)`.
7. **Accusation filed**: The agent with evidence (B, C, or authority) travels to the `CrimeRegister`'s `home_place` and files an `Accuse` action.
8. **Punishment administered**: The office holder travels to the accused's location and administers `Fine` or `Exile` while co-located.

Every step requires co-location or physical travel. No information teleports. The path is fully traceable (P27). A theft at an empty location remains unknown until someone visits (step 3) or the thief is observed elsewhere with the item (step 6).

### 2. Positive-feedback analysis

**Loop 1: Theft -> deprivation -> more theft?**

If an agent steals food because they are hungry, eating the food satisfies hunger, reducing theft motive. The loop is: hunger -> theft -> consume -> satisfaction -> no more theft. Self-terminating.

BUT: if theft victims become hungry due to lost supplies, they might steal from others. Chain: A steals from B -> B becomes hungry -> B steals from C -> cascade.

**Loop 2: Crime -> punishment -> exile -> desperation -> more crime?**

If an exiled agent has no faction support and cannot trade, they may resort to more theft. Chain: theft -> accusation -> exile -> deprivation -> more theft. This is an amplifying loop.

**Loop 3 (dampening): Crime -> witness reporting -> accusation -> punishment -> deterrence**

More theft -> more witnesses -> more accusations -> more punishments -> fewer agents willing to steal (risk penalty from co-located agents). This is a NEGATIVE feedback loop that naturally stabilizes.

### 3. Concrete dampeners

**For Loop 1** (theft -> deprivation cascade):
- **Homeostatic need satisfaction**: The stolen food is consumed, satisfying the thief's need. Conservation ensures the food is gone after consumption -- the loop terminates when needs are met.
- **Theft duration + occupancy (P8)**: Stealing takes `steal_duration_ticks`. During that time, the thief cannot eat, sleep, or do anything else. More theft = less time for other survival activities.
- **Witness risk penalty**: `TheftDispositionProfile.witness_risk_penalty` reduces theft motive per co-located agent. High-traffic areas become harder to steal from. Physical analogy: crowded spaces deter theft.
- **Item scarcity (conservation)**: Each theft depletes available items at the location. Fewer items = fewer theft targets. Conservation is the ultimate physical limit.

**For Loop 2** (exile -> desperation -> more crime):
- **Physical distance**: Exiled agents have `hostile_to` relation with the faction, which means faction-controlled areas are dangerous (guards from E19 will pursue hostile agents). The exile must find supplies far from faction territory.
- **Item scarcity**: Same conservation dampener as Loop 1.
- **Agent mortality**: Desperate agents under deprivation may die from deprivation wounds (needs system), removing them from the loop permanently.

**For Loop 3** (dampening loop -- no additional dampener needed):
- This is inherently stabilizing. More crime leads to more punishment, which deters crime. The risk penalty in `TheftDispositionProfile.witness_risk_penalty` is the mechanism by which deterrence operates: more agents present = lower theft motive.

### 4. Stored state vs. derived read-model list

**Stored (authoritative)**:
- `TheftDispositionProfile` component on Agent entities
- `JusticeDispositionProfile` component on Agent entities
- `ViolationKind::SuspectedTheft` entries in `ViolationMemory` component
- `SocialObservation(SuspectedTheft)` entries in `AgentBeliefStore`
- `RecordData` entity of `RecordKind::CrimeRegister` at jurisdictional places
- `InstitutionalClaim::Accusation` entries in CrimeRegister
- `InstitutionalClaim::Verdict` entries in CrimeRegister
- Possession relations (after theft) via `possessed_by`
- Ownership relations (unchanged by theft) via `owned_by`
- `hostile_to` relations (after exile)
- `member_of` relations (removed after exile)
- Event log entries with `EventTag::Crime`

**Derived (transient, recomputable)**:
- Theft candidate goals from `emit_theft_candidates()` -- derived each tick from co-located stealable items and profile
- Justice candidate goals from `emit_justice_candidates()` -- derived each tick from `ViolationMemory` and `CrimeRegister` knowledge
- `FeasibilityHint` for `StealItem`/`Accuse`/`PunishAccused` -- derived from agent position and evidence state
- Witness risk penalty computation -- derived from co-located agent count
- Fine amount computation -- derived from `JusticeDispositionProfile.fine_severity` and commodity quantity

## Phase 3 Gate

After E17, verify:
- [ ] Information propagates through explicit channels (witnesses, rumors, records)
- [ ] Offices transfer through succession
- [ ] Crimes discovered through defined pathways (witness observation, inventory violation, possession sighting)
- [ ] No omniscient NPCs (theft at empty location remains unknown until discovery)
- [ ] Causal chains from crime -> discovery -> accusation -> punishment are fully traceable

## Invariants Enforced

- P15 / 9.17: Traceable discovery -- no immediate global accusation after theft
- P12 / 9.11: Crime awareness through information channels only
- P4: Conservation maintained through all transfers (steal, fine)
- P22: Ownership and possession remain distinct throughout

## Spec References

- Section 4.5 (crime and theft)
- Section 7.3 (informational propagation: suspicion, discovery delays)
- Section 8 (no global omniscience for NPCs)
- Section 9.17 (traceable discovery)
- FOUNDATIONS Principles 1-4, 7-10, 12-17, 20-24, 27-28
