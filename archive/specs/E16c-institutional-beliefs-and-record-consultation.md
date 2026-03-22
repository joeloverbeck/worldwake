**Status**: ✅ COMPLETED

# E16c: Institutional Beliefs & Record Consultation

## Epic Summary

Introduce first-class institutional records and explicit institutional belief state so agents can know offices, faction membership, and support declarations through witness observation, reports, and record consultation rather than through authoritative relation reads hidden behind AI-facing helpers.

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
- E15c (conversation memory — `ToldBeliefMemory` / `HeardBeliefMemory` / `TellMemoryKey`)
- E16 (offices, factions, support declarations, succession, political actions)
- S12 (prerequisite-aware planning — `prerequisite_places()` for ConsultRecord at remote locations)

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
| `ToldBeliefMemory` / `HeardBeliefMemory` | E15c conversation memory | institutional claims from Tell flow through HeardBeliefMemory before projecting into institutional_beliefs |
| `TellMemoryKey` / resend suppression | E15c | prevents redundant institutional claim re-telling |
| `VisibilitySpec::PublicRecord` | E14 foundations alignment | defines records as consultable-at-location, not globally known |
| `PerceptionProfile` | E14 belief system | extended with consultation fields (institutional_memory_capacity, consultation_speed_factor, contradiction_tolerance) |
| `S12 prerequisite_places()` | S12 prerequisite-aware search | ConsultRecord at remote locations produces Travel+Consult prerequisite chains |
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

Add `RecordData` (absorbs record entries — no separate `InstitutionalRecord` component):

```rust
pub struct RecordData {
    pub record_kind: RecordKind,
    pub home_place: EntityId,
    pub issuer: EntityId,
    pub consultation_ticks: u32,
    pub max_entries_per_consult: u32,
    pub entries: Vec<InstitutionalRecordEntry>,
    pub next_entry_id: u64,
}
```

Methods on `RecordData`:
- `append_entry(claim, tick) -> RecordEntryId` — appends new entry, increments next_entry_id
- `supersede_entry(old_id, new_claim, tick) -> Result<RecordEntryId>` — appends entry with `supersedes: Some(old_id)`
- `entries_newest_first() -> impl Iterator` — reverse chronological iteration
- `active_entries() -> Vec<&InstitutionalRecordEntry>` — entries not superseded by a later entry

```rust
pub enum RecordKind {
    OfficeRegister,
    FactionRoster,
    SupportLedger,
    // E16b adds: JurisdictionNotice
    // E17 adds: LegalNotice
}
```

`consultation_ticks` and `max_entries_per_consult` are explicit record-local policy, not hidden constants. `JurisdictionNotice` and `LegalNotice` are deferred to E16b and E17 respectively — they are downstream concerns that E16c does not exercise. Adding them here would be forward-coupling to unvalidated requirements (YAGNI).

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
    // E16b adds: OfficeController { office, controller, contested, effective_tick }
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

`OfficeController` is deferred to E16b — it represents physical force control distinct from legitimate holding, which is an E16b concept. E16c delivers only the claims its acceptance criteria exercise.

This scope is intentionally narrow to current political/institutional needs, but the type is extensible for later claims such as controllers, accusations, warrants, licenses, treasuries, and appointments.

### 3. Record Entries, Not Global Truth Mirrors

Record entries live inside `RecordData` (no separate `InstitutionalRecord` component — entries are part of the record artifact):

```rust
pub struct InstitutionalRecordEntry {
    pub entry_id: RecordEntryId,
    pub claim: InstitutionalClaim,
    pub recorded_tick: Tick,
    pub supersedes: Option<RecordEntryId>,
}

pub struct RecordEntryId(pub u64);
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
    pub told_beliefs: BTreeMap<TellMemoryKey, ToldBeliefMemory>,       // E15c
    pub heard_beliefs: BTreeMap<TellMemoryKey, HeardBeliefMemory>,     // E15c
    pub institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>,  // E16c
}
```

```rust
pub enum InstitutionalBeliefKey {
    OfficeHolderOf { office: EntityId },
    // E16b adds: OfficeControllerOf { office: EntityId }
    FactionMembersOf { faction: EntityId },
    SupportFor { supporter: EntityId, office: EntityId },
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
- faction membership changes
- (E16b adds: force-claim press/yield/control/install)

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
// E16b adds: fn believed_office_controller(...)
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
fn believed_support_declarations_for_office(
    &self,
    office: EntityId,
) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)>;
fn consultable_records_at(&self, place: EntityId) -> Vec<EntityId>;
```

Rules:

1. self-private state may still be known authoritatively where appropriate
2. public institutional facts must come from stored belief/record pathways
3. once this spec lands, AI modules must not read live office holder / faction membership / public support truth through direct helper shims

### 10. PerceptionProfile Extension for Institutional Knowledge

To avoid magic constants and support agent diversity (Principle 20), extend the existing `PerceptionProfile` with institutional consultation parameters rather than adding a separate component:

```rust
// New fields on existing PerceptionProfile:
pub institutional_memory_capacity: u32,       // default: 20 — how many institutional beliefs the agent retains
pub consultation_speed_factor: Permille,      // default: Permille(500) — multiplier on record's consultation_ticks
pub contradiction_tolerance: Permille,        // default: Permille(300) — how readily the agent acts under contradiction
```

Use cases:

- diligent clerks read records more quickly (higher consultation_speed_factor)
- some agents retain more institutional knowledge than others
- some agents act under mild contradiction more readily than others

`contradiction_tolerance` is an agent trait, not a world-truth score. Consultation speed/capacity is a form of perception — how effectively an agent acquires information from records.

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

`RecordData` absorbs record entries (no separate `InstitutionalRecord` component — entries live inside `RecordData`). Consultation parameters are per-agent fields on `PerceptionProfile` (no separate `RecordConsultationProfile`). `AgentBeliefStore` grows `institutional_beliefs` as an extension of existing agent belief state.

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

## E15c Tell Integration (Reassessment Addition)

E15c (completed after this spec was originally drafted) added `ToldBeliefMemory` / `HeardBeliefMemory` with `TellMemoryKey` for social information propagation. Institutional claims transmitted via Tell must flow through this existing system:

1. When an agent Tells another about an institutional fact, the claim enters `HeardBeliefMemory` first (preserving provenance, chain length, and resend-suppression logic from E15c).
2. A projection step then writes the institutional claim into the listener's `institutional_beliefs` with `InstitutionalKnowledgeSource::Report { from: speaker, chain_len }`.
3. This unifies all social information flow through one channel — no parallel path that bypasses E15c's memory/resend-suppression.

Chain length degrades through relays: direct witness → Report(chain_len=1) → Report(chain_len=2) → etc.

## PlanningSnapshot Extension (Reassessment Addition)

The GOAP plan search operates on `PlanningSnapshot` (immutable read-only state). For the AI to reason about institutional beliefs during planning:

1. Extend `PlanningSnapshot` with `actor_institutional_beliefs: BTreeMap<InstitutionalBeliefKey, InstitutionalBeliefRead<...>>` — captured at snapshot build time from the actor's `AgentBeliefStore`.
2. Extend `PlanningState` with `institutional_belief_overrides: BTreeMap<InstitutionalBeliefKey, InstitutionalBeliefRead<...>>` — populated by hypothetical `ConsultRecord` transitions during search (follows the existing `support_declaration_overrides` pattern).
3. The `RuntimeBeliefView` impl on `PlanningState` reads from overrides first, then snapshot.

## S12 Prerequisite-Aware Planning Integration (Reassessment Addition)

S12 (completed after this spec was originally drafted) enables multi-hop travel-to-prerequisite plans. ConsultRecord integrates with this:

1. When candidate generation detects `Unknown` institutional beliefs, it keeps the ordinary political goal family (`ClaimOffice` / `SupportCandidateForOffice`) but only when consultable record evidence makes the branch actionable.
2. If the relevant record is at a remote location, the plan search produces a prerequisite chain inside the political plan: `Travel(to record place) → ConsultRecord → PoliticalAction`.
3. `ConsultRecord` is registered as a `PlannerOpKind` with `may_appear_mid_plan: true` — it serves as a prerequisite step in multi-step plans, not as a standalone ranked goal family.
4. `prerequisite_places()` for political goals returns the record's home place when the agent's institutional belief is `Unknown`.

## Phased Delivery (Reassessment Addition)

This spec is delivered in two phases:

### Phase B1: Record Infrastructure
- `EntityKind::Record`, `RecordData`, `RecordKind`
- `InstitutionalClaim`, `InstitutionalRecordEntry`, `RecordEntryId`
- `institutional_beliefs` field on `AgentBeliefStore` with capacity enforcement
- `PerceptionProfile` extension with consultation parameters
- `WorldTxn` record helpers (append, supersede, project belief)
- `ConsultRecord` action (co-location required, max entries per consult)
- Perception system projects institutional beliefs for witnesses of political events
- Tell action projects institutional claims through HeardBeliefMemory → institutional_beliefs
- Action handlers append record entries in same transaction as authoritative mutations

### Phase B2: AI Migration
- `InstitutionalBeliefRead` derivation helpers on `AgentBeliefStore`
- `PerAgentBeliefView` institutional methods backed by belief store (not live world)
- `PlanningSnapshot` + `PlanningState` institutional belief fields
- Candidate generation keeps ordinary political goal families, requires consultable-record evidence when beliefs are `Unknown`, and suppresses institution-sensitive action when beliefs are `Conflicted`
- `PlannerOpKind::ConsultRecord` integrates with S12 as a mid-plan prerequisite step under political goals
- Ranking continues to score only the surviving political goals after candidate-generation gating
- Shared start-failure recovery remains generic; stale political retries are prevented by refreshed institutional beliefs and candidate omission on the next tick rather than new `BlockingFact` variants
- Live helper seam removed (Principle 26)
- Golden tests updated + new institutional belief scenarios

Each phase is independently testable and reviewable. B1 validates infrastructure before B2 depends on it.

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path to Agent |
|-------------|--------|---------------|
| office vacancy | vacancy event or office register entry | same-place witness, report, or record consultation |
| office installation | installation event or office register entry | same-place witness, report, or record consultation |
| faction membership | faction roster entry or witnessed induction/removal event | direct witness, report, or roster consultation |
| support declaration | public declaration event or support ledger entry | same-place witness, report, or ledger consultation |
| office control/contest | (deferred to E16b) | (deferred to E16b) |

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
- authoritative office/faction/support state
- record entities with `RecordData` (including entries)
- `AgentBeliefStore.institutional_beliefs`
- `PerceptionProfile` consultation fields

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

### Phase B1 (Infrastructure)

| File | Change |
|------|--------|
| `crates/worldwake-core/src/institutional.rs` (NEW) | all institutional claim/record/belief types |
| `crates/worldwake-core/src/entity.rs` | add `EntityKind::Record` |
| `crates/worldwake-core/src/component_tables.rs` | add record component storage |
| `crates/worldwake-core/src/component_schema.rs` | register `RecordData` |
| `crates/worldwake-core/src/delta.rs` | add `ComponentKind::RecordData`, `ComponentValue::RecordData` |
| `crates/worldwake-core/src/canonical.rs` | add hashing for new component kind |
| `crates/worldwake-core/src/belief.rs` | extend `AgentBeliefStore` with `institutional_beliefs`, extend `PerceptionProfile` with consultation fields |
| `crates/worldwake-core/src/world.rs` | add `create_record()` method |
| `crates/worldwake-core/src/world_txn.rs` | add `create_record()`, `append_record_entry()`, `supersede_record_entry()`, `project_institutional_belief()` |
| `crates/worldwake-sim/src/action_payload.rs` | add `ConsultRecord(ConsultRecordActionPayload)` |
| `crates/worldwake-systems/src/consult_record_actions.rs` (NEW) | ConsultRecord action handler |
| `crates/worldwake-systems/src/perception.rs` | project institutional beliefs for witnesses of political events |
| `crates/worldwake-systems/src/tell_actions.rs` | project institutional claims via HeardBeliefMemory flow |
| `crates/worldwake-systems/src/office_actions.rs` | append support-declaration records on mutation |
| `crates/worldwake-systems/src/offices.rs` | append office register entries on vacancy/install |

### Phase B2 (AI Migration)

| File | Change |
|------|--------|
| `crates/worldwake-core/src/belief.rs` | add `believed_office_holder()`, `believed_factions_of()`, etc. derivation helpers |
| `crates/worldwake-sim/src/per_agent_belief_view.rs` | back institutional methods with belief store (replace `self.world.*` reads) |
| `crates/worldwake-ai/src/planning_snapshot.rs` | add `actor_institutional_beliefs` field |
| `crates/worldwake-ai/src/planning_state.rs` | add `institutional_belief_overrides`, implement institutional methods |
| `crates/worldwake-ai/src/candidate_generation.rs` | gate ordinary political goals on institutional belief reads; require consultable-record evidence when `Unknown`, suppress when `Conflicted` |
| `crates/worldwake-ai/src/goal_model.rs` | keep political goal families and integrate `ConsultRecord` as a prerequisite planning step |
| `crates/worldwake-ai/src/planner_ops.rs` | add `PlannerOpKind::ConsultRecord` |
| `crates/worldwake-ai/src/ranking.rs` | continue ranking surviving political goals after candidate-generation gating; no standalone consult-record ranking surface |
| `crates/worldwake-ai/src/search.rs` | register ConsultRecord planner op, S12 prerequisite chains |
| `crates/worldwake-ai/src/failure_handling.rs` | preserve shared start-failure recovery for stale political starts; no institutional-specific blocker taxonomy |
| `crates/worldwake-ai/tests/golden_offices.rs` | update existing + add institutional belief scenarios |

## Spec References

- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principles 7, 12, 13, 14, 16, 21, 23, 24, 26, 27
- [E16b-force-legitimacy-and-jurisdiction-control.md](/home/joeloverbeck/projects/worldwake/specs/E16b-force-legitimacy-and-jurisdiction-control.md)
- [E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/specs/E17-crime-theft-justice.md)
- [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md)

## Outcome

- Outcome amended: 2026-03-22
- Completion date: 2026-03-22
- What changed: institutional claim propagation, witness/report storage, record consultation, and AI-side institutional belief reads are all live. The remaining live office/faction/support helper seam was removed from the AI/runtime boundary, planner support baselines now come from institutional belief reads, office-register interpretation is centralized, and the remaining planner-local support accessor was renamed to `effective_support_declaration` to keep the planner overlay surface semantically distinct from deleted live institutional helpers.
- Deviations from original plan: the final cutover landed through targeted seam removal and focused/golden migration rather than a wholesale rewrite of every political golden around consultation. Existing explicit-belief goldens were kept where they already matched the architecture.
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
  - `python3 scripts/golden_inventory.py --write --check-docs`
