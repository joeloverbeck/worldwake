**Status**: PENDING

# E16c: Institutional Beliefs & Record Consultation

## Epic Summary

Introduce first-class institutional records and explicit institutional belief state so agents can know offices, faction membership, support declarations, and later legal/jurisdiction facts through witness observation, reports, and record consultation rather than through authoritative relation reads hidden behind AI-facing helpers.

This spec is the architectural completion of the E14/E16 information boundary:

- authoritative institutional state remains in ordinary world relations/components
- institutional information propagates through events, records, and testimony
- agent planning reads stored institutional beliefs, not live institutional truth
- contradictory or stale institutional claims remain representable

This is not a patch for the current political AI seam. It is the replacement architecture that later politics, justice, guards, and institutional economy work should build on.

## Phase

Phase 3: Information & Politics

## Crates

`worldwake-core`
- institutional claim/record types
- belief-state types for institutional claims
- record entity classification and components

`worldwake-sim`
- belief/runtime institutional query traits
- consultation affordance payloads

`worldwake-systems`
- record consultation action handling
- institutional record mutation helpers used by office/faction/legal systems

`worldwake-ai`
- candidate generation / ranking / goal-model queries through institutional belief reads

## Dependencies

- E14 (belief stores, witness/report model, `PublicRecord` semantics)
- E15 (report/rumor propagation)
- E16 (offices, factions, support declarations, succession, political actions)

## Why This Exists

Current architecture still has one real weakness:

1. political AI now respects the belief boundary, but it still uses a narrow runtime seam for live institutional facts
2. office holding, support declarations, and faction membership are not yet first-class in agent belief state
3. no active spec yet defines how an agent learns institutional truth from local records
4. later epics such as E16b, E17, E19, and S05 all need institutional awareness that is traceable, local, and disputable

Without this spec, the codebase will drift toward one of two bad outcomes:

1. more ad hoc AI-facing helpers that leak authoritative relations through the boundary
2. multiple incompatible mini-models for politics, justice, guards, and institutional stock knowledge

This spec prevents both.

## Foundational Alignment

This spec exists to satisfy the following non-negotiable principles in [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md):

- Principle 7: information must travel locally through witnesses, reports, and records
- Principle 12: world state is not belief state
- Principle 13: knowledge is acquired locally and travels physically
- Principle 14: contradiction and stale knowledge are first-class
- Principle 16: memories, evidence, and records are world state
- Principle 21: roles, offices, and institutions are world state
- Principle 23: social artifacts are first-class
- Principle 24: systems interact through state, not through each other
- Principle 26: no backward compatibility layers
- Principle 27: debuggability is a product feature

## Design Goals

1. Institutional facts must be representable as world artifacts and belief artifacts, not as hidden AI shortcuts.
2. Agents must be able to learn institutional facts by:
   - witnessing visible institutional events
   - hearing reports
   - consulting a record at a location
3. Institutional beliefs must support:
   - unknown
   - stale
   - contradicted
   - superseded
4. Authoritative office/faction state must remain separate from belief state.
5. The architecture must extend naturally to E16b, E17, E19, S05, and later contracts/notices/warrants without inventing a new belief subsystem each time.
6. When this lands, live institutional truth must stop leaking through planner-only helper paths.

## Non-Goals

1. Implementing full forgery, record destruction, or counterfeiting in this spec
2. Replacing the ordinary authoritative office/faction/support relations as source of truth
3. Reworking non-institutional belief storage such as wounds, inventories, or route knowledge
4. Solving the entire loyalty architecture beyond public institutional claims

## Existing Infrastructure (Leveraged, Not Reimplemented)

| Infrastructure | Location | Usage in E16c |
|----------------|----------|---------------|
| `AgentBeliefStore` | E14 belief system | stores new institutional belief state alongside entity beliefs |
| `PerceptionSource` / Tell propagation | E14/E15 | continue to carry report and rumor provenance |
| `VisibilitySpec::PublicRecord` | E14 foundations alignment | defines records as consultable-at-location, not globally known |
| `office_holder / offices_held` | core relations | remains authoritative office-title state |
| `member_of / members_of` | core relations | remains authoritative faction membership state |
| `support_declarations` | core relations | remains authoritative public support state |
| `OfficeData`, `FactionData` | E16 core data | remain authoritative institutional entities |
| `EventTag::Political` and later legal tags | event tagging | visible institutional changes still propagate through ordinary events |
| `WorldTxn` | transactional boundary | record mutation and institutional-state mutation must remain atomic |

## Deliverables

### 1. Record Entities as First-Class World Artifacts

Add `EntityKind::Record`.

Records are ordinary world entities with identity, location, ownership/custody, and access conditions. A record may represent:

- office register
- succession notice
- faction roster
- public declaration notice
- later accusation, warrant, tax ledger, stock ledger, or appointment record

Add `RecordData`:

```rust
pub struct RecordData {
    pub title: String,
    pub record_kind: RecordKind,
    pub home_place: EntityId,
    pub issuer: Option<EntityId>,
    pub consultation_ticks: u32,
    pub max_entries_per_consult: u32,
}
```

```rust
pub enum RecordKind {
    OfficeRegister,
    FactionRoster,
    SupportLedger,
    JurisdictionNotice,
    LegalNotice,
}
```

`consultation_ticks` and `max_entries_per_consult` are explicit record-local policy, not hidden constants.

### 2. Typed Institutional Claims

Add a typed claim model for institutional facts. These claims can appear:

- inside record entries
- inside agent institutional beliefs
- inside visible event payloads where appropriate

```rust
pub enum InstitutionalClaim {
    OfficeHolder {
        office: EntityId,
        holder: Option<EntityId>,
        effective_tick: Tick,
    },
    OfficeController {
        office: EntityId,
        controller: Option<EntityId>,
        contested: bool,
        effective_tick: Tick,
    },
    FactionMembership {
        faction: EntityId,
        member: EntityId,
        active: bool,
        effective_tick: Tick,
    },
    SupportDeclaration {
        office: EntityId,
        supporter: EntityId,
        candidate: Option<EntityId>,
        effective_tick: Tick,
    },
}
```

This scope is intentionally narrow to current political/institutional needs, but the type is extensible for later claims such as accusations, warrants, licenses, treasuries, and appointments.

### 3. Record Entries, Not Global Truth Mirrors

Add an explicit record-entry container:

```rust
pub struct InstitutionalRecordEntry {
    pub entry_id: RecordEntryId,
    pub claim: InstitutionalClaim,
    pub recorded_tick: Tick,
    pub supersedes: Option<RecordEntryId>,
}
```

Add `InstitutionalRecord`:

```rust
pub struct InstitutionalRecord {
    pub entries: Vec<InstitutionalRecordEntry>,
}
```

Important rule:

- record entries are durable world state
- they are not regenerated each tick by scanning authoritative relations
- mutating systems append or supersede entries when institutional truth changes

This keeps records as world artifacts rather than caches that secretly mirror truth.

### 4. Institutional Belief State in `AgentBeliefStore`

Extend the belief store with institutional claims:

```rust
pub struct AgentBeliefStore {
    pub known_entities: BTreeMap<EntityId, BelievedEntityState>,
    pub social_observations: Vec<SocialObservation>,
    pub institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>,
}
```

```rust
pub enum InstitutionalBeliefKey {
    OfficeHolder { office: EntityId },
    OfficeController { office: EntityId },
    FactionMembership { faction: EntityId, member: EntityId },
    SupportDeclaration { office: EntityId, supporter: EntityId },
}
```

```rust
pub struct BelievedInstitutionalClaim {
    pub claim: InstitutionalClaim,
    pub source: InstitutionalKnowledgeSource,
    pub learned_tick: Tick,
    pub learned_at: Option<EntityId>,
}
```

```rust
pub enum InstitutionalKnowledgeSource {
    WitnessedEvent,
    Report { from: EntityId, chain_len: u8 },
    RecordConsultation { record: EntityId, entry_id: RecordEntryId },
    SelfDeclaration,
}
```

This keeps institutional beliefs explicit, source-traceable, and separate from generic entity snapshots.

### 5. Contradiction Is First-Class

Do not collapse multiple claims for the same institutional key into one silent winner at storage time.

Instead, the read model derives one of:

```rust
pub enum InstitutionalBeliefRead<T> {
    Unknown,
    Certain(T),
    Conflicted(Vec<T>),
}
```

Examples:

- office holder unknown
- certain that office `X` is vacant
- conflicted between `A` and `B` as holder because one is a rumor and one is a stale record

The planner may act only on `Certain(...)` for institutional decisions that require commitment. `Conflicted(...)` should suppress or defer institution-sensitive goals unless the goal is specifically about resolving the contradiction.

### 6. Consultation as an Explicit Action

Add `ConsultRecord`.

- **Domain**: `ActionDomain::Social`
- **Preconditions**:
  - actor and record co-located
  - record accessible under ordinary custody/access rules
  - actor alive and not in transit
- **Duration**:
  - `RecordData.consultation_ticks`
- **Effect on commit**:
  - read up to `RecordData.max_entries_per_consult` entries from the record
  - project those entries into the actor's `institutional_beliefs`
  - preserve provenance as `RecordConsultation { record, entry_id }`
- **Interruptibility**:
  - freely interruptible until commit

This is how `VisibilitySpec::PublicRecord` becomes concrete gameplay rather than a placeholder.

### 7. Institutional Event Projection

Visible institutional actions/events must project directly into institutional belief storage for same-place witnesses when the event content is institutionally legible.

Examples:

- office vacancy activation
- office installation
- support declaration
- later force-claim press/yield/control/install from E16b

This does not require record consultation if the agent witnessed the event. Witnessing and consulting are separate acquisition paths.

### 8. Record Mutation Ownership

The system that changes authoritative institutional truth must own record updates atomically through `WorldTxn`.

Examples:

- office installation appends or supersedes the office register entry
- support declaration appends or supersedes the support ledger entry
- faction membership change appends or supersedes the faction roster entry

Do not add a global reconciliation daemon that scans authoritative state and rewrites records afterward. That would demote records into caches and violate Principle 25.

### 9. Institutional Belief Query Surface

Replace the current narrow live institutional helper approach with a belief-derived read surface.

Suggested trait additions in `worldwake-sim`:

```rust
fn believed_office_holder(&self, office: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
fn believed_office_controller(&self, office: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
fn believed_membership(
    &self,
    faction: EntityId,
    member: EntityId,
) -> InstitutionalBeliefRead<bool>;
fn believed_support_declaration(
    &self,
    office: EntityId,
    supporter: EntityId,
) -> InstitutionalBeliefRead<Option<EntityId>>;
fn consultable_records_at(&self, place: EntityId) -> Vec<EntityId>;
```

Rules:

1. self-private state may still be known authoritatively where appropriate
2. public institutional facts must come from stored belief/record pathways
3. once this spec lands, AI modules must not read live office holder / faction membership / public support truth through direct helper shims

### 10. Record Consultation Profile

To avoid magic constants and support agent diversity, add:

```rust
pub struct RecordConsultationProfile {
    pub max_entries_per_action_bonus: u32,
    pub reread_patience_ticks: u32,
    pub contradiction_tolerance: Permille,
}
```

Use cases:

- diligent clerks reread large ledgers more effectively
- impatient agents avoid repeated consultations
- some agents act under mild contradiction more readily than others

`contradiction_tolerance` is an agent trait, not a world-truth score.

### 11. Migration Requirement

When this spec is implemented:

1. remove the current planner-only live institutional helper path for public office/faction/support facts
2. update E16 political AI to read institutional beliefs
3. update E16b/E17/E19 to create and consult records rather than inventing new knowledge shortcuts
4. do not preserve both architectures in parallel

## Component Registration

Add to `with_component_schema_entries!`:

| Component | Kind Predicate | Storage Field |
|-----------|---------------|---------------|
| `RecordData` | `kind == EntityKind::Record` | `record_data` |
| `InstitutionalRecord` | `kind == EntityKind::Record` | `institutional_record` |
| `RecordConsultationProfile` | `kind == EntityKind::Agent` | `record_consultation_profile` |

`AgentBeliefStore` grows `institutional_beliefs` as an extension of existing agent belief state.

## SystemFn Integration

### `worldwake-core`

- add `EntityKind::Record`
- add institutional claim / record entry / record data types
- extend `AgentBeliefStore`
- add `WorldTxn` helpers for:
  - appending institutional record entries
  - superseding record entries
  - projecting institutional claims into agent beliefs

### `worldwake-systems`

- add `consult_record` action handler
- update office/faction/support mutation sites to append/supersede record entries atomically
- later E16b/E17 systems reuse the same mutation helpers for control, accusation, warrant, and jurisdiction notices

### `worldwake-sim`

- add institutional belief query traits and read-model types
- expose consultable record affordances

### `worldwake-ai`

- candidate generation, ranking, and goal-model checks consume institutional belief reads
- contradictory institutional belief should suppress or defer institution-sensitive action rather than silently falling back to live truth

## Cross-System Interactions (Principle 12)

The interaction path must be state-mediated:

1. office/faction/legal systems mutate authoritative institutional state
2. those same mutations append or supersede durable record entries
3. visible events and record consultation project institutional claims into agent belief stores
4. Tell/report propagation moves those claims socially
5. AI reads institutional belief state only

No system calls another system's logic to "inform" agents out of band.

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path to Agent |
|-------------|--------|---------------|
| office vacancy | vacancy event or office register entry | same-place witness, report, or record consultation |
| office installation | installation event or office register entry | same-place witness, report, or record consultation |
| faction membership | faction roster entry or witnessed induction/removal event | direct witness, report, or roster consultation |
| support declaration | public declaration event or support ledger entry | same-place witness, report, or ledger consultation |
| office control/contest | E16b political event or later control ledger entry | witness, report, or consultation |

No remote institutional belief may appear without a witness, report, or consulted record.

### H.2 Positive-Feedback Analysis

**Loop 1: institutional knowledge -> coordinated action -> more institutional knowledge**
- Amplifier: once agents know who holds office, they can align support, opposition, trade, and law enforcement more coherently.

**Loop 2: records -> legitimacy -> more record consultation -> stronger legitimacy**
- Amplifier: visible records can make an institution easier to trust and therefore more often consulted.

### H.3 Concrete Dampeners

**Loop 1 dampeners**
- agents must physically witness, travel, or hear a report
- consultation takes time and occupies action slots
- records have limited entries per consult action
- contradictory beliefs can block decisive action
- stale records can be overtaken by newer witnessed events

**Loop 2 dampeners**
- records exist at places and can be inaccessible, missing, stale, or contested
- custody/access rules can prevent immediate consultation
- report chains degrade with distance and relays
- agents differ in contradiction tolerance and reread patience

### H.4 Stored State vs Derived

**Stored**
- authoritative office/faction/support/controller state
- record entities
- `RecordData`
- `InstitutionalRecord`
- `AgentBeliefStore.institutional_beliefs`
- `RecordConsultationProfile`

**Derived**
- `InstitutionalBeliefRead::{Unknown,Certain,Conflicted}`
- which claim currently appears most recent or reliable
- whether contradiction is tolerable enough for a specific agent/goal
- AI interpretation of institutional legitimacy or confidence

## Invariants Enforced

1. authoritative institutional truth and believed institutional truth remain separate
2. public institutional facts do not teleport into AI through live helper methods
3. records are durable world artifacts, not per-tick derived caches
4. contradictory institutional beliefs can coexist in agent memory
5. institutional knowledge remains traceable to witness, report, or record consultation
6. no backward-compatibility alias layer preserves both the old live-helper path and the new institutional-belief path

## Acceptance Criteria

1. office holding, faction membership, and support declaration can all exist as record entries and belief entries
2. an agent can learn institutional facts by consulting a co-located record
3. same-place witnesses of visible institutional events gain corresponding institutional beliefs without consulting a record
4. report propagation can transmit institutional claims with provenance
5. contradictory institutional claims are representable and queryable as conflict rather than being silently overwritten
6. AI political/legal/institutional logic can read institutional beliefs without depending on live authoritative office/faction/support queries
7. records are updated atomically when authoritative institutional truth changes
8. the older planner-only institutional helper seam is removed rather than preserved

## Tests

- [ ] consulting an office register creates institutional beliefs about office holder state
- [ ] consulting a faction roster creates institutional beliefs about faction membership
- [ ] consulting a support ledger creates institutional beliefs about public support declarations
- [ ] witnessing office installation creates an institutional belief without record consultation
- [ ] report propagation preserves institutional claim provenance and relay chain length
- [ ] contradictory office-holder claims remain stored as conflict rather than collapsing silently
- [ ] political AI suppresses office-claim behavior when office-holder belief is conflicted
- [ ] support-candidate planning reads believed support declarations rather than live declarations
- [ ] record updates append/supersede entries atomically with the authoritative mutation that caused them
- [ ] record location and access rules prevent remote/global consultation
- [ ] after migration, no AI module depends on live public institutional helper queries

## Cross-Spec Notes

- E16 political AI should migrate from the current narrow seam to institutional belief reads when this lands.
- E16b should use the same record and institutional-belief architecture for control, contest, and installation by force.
- E17 should build accusations, warrants, and jurisdiction notices on top of `EntityKind::Record` and `InstitutionalClaim`, not by inventing a second record model.
- E19 should treat contested offices, controllers, and legal notices as institutional beliefs acquired through witness/report/consultation.
- S05 should reuse the record architecture for stock ledgers and institutional audits rather than inventing bespoke merchant-only knowledge channels.

## Critical Files To Modify

| File | Change |
|------|--------|
| `crates/worldwake-core/src/entity.rs` | add `EntityKind::Record` |
| `crates/worldwake-core/src/component_tables.rs` | add record component storage |
| `crates/worldwake-core/src/component_schema.rs` | register `RecordData`, `InstitutionalRecord`, `RecordConsultationProfile` |
| `crates/worldwake-core/src/beliefs.rs` or belief-state module | extend `AgentBeliefStore` with institutional belief storage |
| `crates/worldwake-core/src/records.rs` or new module | add institutional claim and record-entry types |
| `crates/worldwake-core/src/world_txn.rs` | add transactional record-entry and institutional-belief mutation helpers |
| `crates/worldwake-sim/src/belief_view.rs` | replace live institutional helper path with institutional belief query surface |
| `crates/worldwake-sim/src/action_payload.rs` | add `ConsultRecordActionPayload` |
| `crates/worldwake-systems/src/office_actions.rs` | append/supersede support-declaration records on mutation |
| `crates/worldwake-systems/src/offices.rs` | append/supersede office register entries on vacancy/install transitions |
| `crates/worldwake-systems/src/` | add `consult_record` action handler module or integrate into record actions module |
| `crates/worldwake-ai/src/candidate_generation.rs` | migrate institutional reads to belief-derived queries |
| `crates/worldwake-ai/src/goal_model.rs` | migrate office satisfaction/progress checks to institutional beliefs |
| `crates/worldwake-ai/src/ranking.rs` | consume belief-derived institutional certainty/conflict |

## Spec References

- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principles 7, 12, 13, 14, 16, 21, 23, 24, 26, 27
- [E16b-force-legitimacy-and-jurisdiction-control.md](/home/joeloverbeck/projects/worldwake/specs/E16b-force-legitimacy-and-jurisdiction-control.md)
- [E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/specs/E17-crime-theft-justice.md)
- [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md)
