# S59: Expectation and Obligation Substrate

## Summary

Add expectation records that let agents track commitments, duties, and anticipated arrivals, then detect overdue states and launch search/rescue behavior. Currently agents detect missing entities retroactively through `ViolationKind::EntityMissing` when they happen to visit an expected location. This spec adds proactive expectation tracking — "X should be at Y by tick Z" — so that overdue states drive search, rescue, and report cascades without omniscient detection.

## Phase

Phase 7: Consequence Carriers

## Status

**Status**: COMPLETED

## Crates

- `worldwake-core` (expectation types, components)
- `worldwake-sim` (system registration, GoalBeliefView extension)
- `worldwake-systems` (search/report actions, overdue detection system)
- `worldwake-ai` (search goal generation, candidate generation for search actions, PlannerOpKind integration)

## Dependencies

- E14 (perception & belief) — completed
- S27 (expectation-violation goals) — completed
- S52 (evidence aftermath) — completed
- S54 (entity belief claims) — completed

## Design Goals

- Agents track time-bounded expectations about other entities: "courier should arrive at market by tick 300"
- Global clock maintenance marks expectations overdue when their time window expires; later missing-person confirmation and response remain locality-sensitive
- Search and rescue emerge from expectation violation, not from a dedicated mission system
- Last-seen records propagate through existing social channels (tell, observation, rumor)
- Reuses `EntityBeliefClaim`, `ViolationKind`, `SceneEvidence`, and care actions

## Non-Goals

- Formal contract or employment system — deferred to S64 (debt/obligations)
- Search party coordination (multi-agent joint search) — emergent from individual search intents
- Omniscient missing-person registry — strictly forbidden (P7)
- Automated periodic check-in system — agents check expectations when they are at the right place at the right time

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State) | Expectations are stored records with subject, place, time window, and basis — not derived scores |
| P4 (Persistent Identity) | Expectation records have stable identity and lifecycle (created, active, overdue, resolved, expired) |
| P5 (Carriers of Consequence) | Expectations create downstream search, accusation, grief, institutional response |
| P7 (Locality) | Global clock maintenance may mark an expectation overdue, but missing-person confirmation, search, and reporting still rely on owner-local observation, travel, and communication rather than omniscient lookup |
| P8 (Preconditions and Duration) | Search actions have travel cost, duration, and occupy the searcher |
| P10 (Aftermath) | Failed searches produce evidence (searched-and-found-nothing), successful searches produce rescue or recovery |
| P14 (World ≠ Belief) | Expectations are belief-state (what an agent expects), not world truth |
| P15 (Knowledge Travels) | Last-seen records have provenance and travel through tell/observation, not telepathy |
| P17 (Violated Expectation) | Directly satisfies — expectation records are the substrate for detecting violated expectations |
| P18 (Records Are World State) | Expectation and last-seen records are inspectable, transmittable world state |
| P22 (Agent Diversity) | Grace periods, memory capacity, and search urgency vary per agent through profile parameters |
| P26 (Systems Interact Through State) | All cross-system interactions are state-mediated (see Cross-System Interactions table) |

## Deliverables

### 1. Expectation Record Types

```rust
/// Unique identifier for an expectation record.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ExpectationId(pub u64);

/// Why an expectation exists.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExpectationBasis {
    /// Agent was assigned a duty (patrol, delivery, escort).
    DutyAssignment { office: EntityId },
    /// Agent promised to deliver goods or arrive.
    DeliveryCommitment { commodity: CommodityKind, quantity: Quantity },
    /// Household member expected home by routine.
    RoutineReturn,
    /// Escort obligation — subject is expected to accompany a charge.
    EscortObligation { charge: EntityId },
    /// General social expectation (friend said they'd visit, etc).
    SocialPromise,
}

/// Lifecycle state of an expectation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExpectationState {
    /// Within the expected time window.
    Active,
    /// Past the deadline; owner has not yet observed resolution.
    Overdue,
    /// Resolved: subject found safe, found dead, or expectation fulfilled.
    Resolved { outcome: ExpectationOutcome },
    /// Owner gave up or expectation became irrelevant.
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExpectationOutcome {
    Fulfilled,
    FoundSafe { at_place: EntityId },
    FoundWounded { at_place: EntityId },
    FoundDead { at_place: EntityId },
    NotFound,
    /// Subject returned on their own before owner searched.
    ReturnedLate,
}

/// A time-bounded expectation that a subject will be at a place.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpectationRecord {
    pub id: ExpectationId,
    pub owner: EntityId,
    pub subject: EntityId,
    pub expected_place: EntityId,
    pub deadline_tick: Tick,
    /// How many ticks past the deadline before the owner considers it overdue.
    pub grace_ticks: u64,
    pub basis: ExpectationBasis,
    pub state: ExpectationState,
    pub created_tick: Tick,
}
```

### 2. Last-Seen Record

```rust
/// A record of when and where an entity was last seen, with provenance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LastSeenRecord {
    pub subject: EntityId,
    pub place: EntityId,
    pub observed_tick: Tick,
    /// Who observed or reported this.
    pub source: EntityId,
    /// Was this a direct observation or hearsay?
    pub provenance: LastSeenProvenance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LastSeenProvenance {
    /// Observer directly saw the subject at the place.
    DirectObservation,
    /// Heard from another agent (with source chain depth).
    Hearsay { original_observer: EntityId, chain_depth: u8 },
}
```

### 3. Components

```rust
/// Per-agent store of active expectations about other entities.
/// Registered on EntityKind::Agent. Universal profile.
pub struct ExpectationStore {
    pub records: BTreeMap<ExpectationId, ExpectationRecord>,
    next_expectation_id: ExpectationId,
}

/// Per-agent store of last-seen records for known entities.
/// Registered on EntityKind::Agent. Universal profile.
pub struct LastSeenMemory {
    /// Keyed by subject entity. Stores the most recent sighting.
    pub records: BTreeMap<EntityId, LastSeenRecord>,
    /// Maximum number of tracked entities (memory bounded).
    pub capacity: u16,
}
```

### 4. Search Outcome Types

```rust
/// Result of a search action at a specific place.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SearchResult {
    /// Found the target alive.
    FoundAlive { entity: EntityId, condition: SearchCondition },
    /// Found the target dead.
    FoundDead { entity: EntityId },
    /// Found evidence (tracks, blood, belongings) but not the person.
    FoundEvidence { evidence_kinds: Vec<EvidenceKind> },
    /// Found nothing relevant.
    NothingFound,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SearchCondition {
    Healthy,
    Wounded,
    Unconscious,
}
```

### 5. New Actions

All actions registered through the standard `ActionDef` + `ActionHandler` pattern in `worldwake-systems`, following the existing registration pattern in `action_registry.rs:register_all_actions()`.

#### `report_missing`
- **Preconditions**: Actor has an overdue `ExpectationRecord`. Actor is at a place with an office that has jurisdiction (or any co-located agent for informal reports).
- **Duration**: Short (communication action).
- **Effect**: Creates a `ViolationKind::EntityMissing` through the existing violation framework. If reported to an office, creates an institutional record. Updates the expectation state.
- **Domain**: `ActionDomain::Social`

#### `ask_about_person`
- **Preconditions**: Actor is co-located with another agent. Actor has an overdue expectation for the missing subject, and the action payload binds `{ target, subject }` directly.
- **Duration**: Short (conversation action).
- **Effect**: Target agent checks their `LastSeenMemory` for the subject. The actor always records the query in existing `AskWitnessMemory`. If the target has a positive record, the actor updates their own `LastSeenMemory` through direct hearsay transfer.
- **Domain**: `ActionDomain::Epistemic`

#### `search_place`
- **Preconditions**: Actor is at the place to search. The action binds the missing subject directly from overdue-expectation search context and/or planner goal binding, not from a stored `SearchTarget` carrier.
- **Duration**: Medium (investigation action, similar to existing investigate).
- **Effect**: Checks for the target entity at the place. Checks `SceneEvidence` for relevant traces. Produces `SearchResult`. Updates `LastSeenMemory` and `ExpectationRecord`.
- **Domain**: `ActionDomain::Epistemic`

#### `escort_to_safety`
- **Preconditions**: Actor is co-located with a wounded or incapacitated entity. A safe destination exists in actor's beliefs.
- **Duration**: Travel action. The escortee is treated as a co-located dependent: both actor and escortee travel the same edge simultaneously, with the escortee's movement governed by the actor's travel action (similar to how carried items move with their holder). The escortee does not independently occupy a travel slot — they are bound to the escort action for its duration.
- **Effect**: Moves both actor and escortee to destination. On arrival, hands off to care system via the existing `queue_for_care_target` action pattern.
- **Domain**: `ActionDomain::Care`

#### `report_found`
- **Preconditions**: Actor has resolved a search (found alive, found dead). Actor is at a place with interested parties (expectation owner, office).
- **Duration**: Short (communication action).
- **Effect**: Updates institutional records. Notifies expectation owner through existing Tell channels. If found dead, triggers corpse handling cascade.
- **Domain**: `ActionDomain::Social`

### 6. Goal Kinds and Candidate Generation

New `GoalKind` variants:

```rust
GoalKind::SearchForMissing {
    subject: EntityId,
    last_seen: Option<EntityId>,
}
GoalKind::ReportMissing {
    subject: EntityId,
    to_office: Option<EntityId>,
}
GoalKind::EscortToSafety {
    subject: EntityId,
    destination: EntityId,
}
```

Each new variant requires a corresponding `GoalKey` entry in `GoalKey::from()` (`goal.rs`).

**Candidate generation**: When an agent's `ExpectationStore` contains an overdue record, `generate_candidates` emits `SearchForMissing` and `ReportMissing` goals via a new `emit_search_candidates()` function called from `generate_candidates_with_travel_horizon()`. Priority scales with the overdue duration and the relationship to the subject (duty-based expectations rank higher than social promises).

### 7. Overdue Detection SystemFn

A system function `check_overdue_expectations` registered as `SystemId::ExpectationCheck`:

- For each agent with an `ExpectationStore`, scan active records.
- If `current_tick > deadline_tick + grace_ticks` and state is `Active`, transition to `Overdue`.
- This is a global per-tick maintenance operation over stored expectation records. It does not check whether the subject is actually at the expected place.
- Locality-sensitive downstream behavior remains separate: the owner still needs to observe (or fail to observe) the subject, travel, ask, search, or report through normal world channels to generate violations and follow-on behavior.

### 8. PlannerOpKind Integration

New `PlannerOpKind` variants in `crates/worldwake-ai/src/planner_ops.rs`:

```rust
PlannerOpKind::SearchPlace,
PlannerOpKind::AskAboutPerson,
PlannerOpKind::ReportMissing,
PlannerOpKind::EscortToSafety,
PlannerOpKind::ReportFound,
```

Classification in `classify_action_def()`:

| Domain | Action name | PlannerOpKind |
|--------|-------------|---------------|
| `ActionDomain::Epistemic` | `"search_place"` | `PlannerOpKind::SearchPlace` |
| `ActionDomain::Epistemic` | `"ask_about_person"` | `PlannerOpKind::AskAboutPerson` |
| `ActionDomain::Social` | `"report_missing"` | `PlannerOpKind::ReportMissing` |
| `ActionDomain::Care` | `"escort_to_safety"` | `PlannerOpKind::EscortToSafety` |
| `ActionDomain::Social` | `"report_found"` | `PlannerOpKind::ReportFound` |

Each variant needs planner semantics entries in `build_semantics_table()` defining precondition/effect modeling for the GOAP search.

### 9. GoalBeliefView Extension

New methods on `GoalBeliefView` trait (`crates/worldwake-sim/src/belief_view.rs`):

```rust
fn expectation_store(&self, agent: EntityId) -> Option<ExpectationStore> {
    let _ = agent;
    None
}
fn last_seen_memory(&self, agent: EntityId) -> Option<LastSeenMemory> {
    let _ = agent;
    None
}
```

These methods are required for `emit_search_candidates()` in candidate generation to read the agent's expectation and last-seen state.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence addressed**: Agents currently have no proactive mechanism to mark that someone should have arrived but has not. The global overdue-maintenance slice marks the record past due by clock, while later locality-sensitive search/report behavior still requires physically grounded observation, travel, and communication.

2. **New entities/relations/records**: `ExpectationRecord`, `LastSeenRecord`, `ExpectationStore` (component), `LastSeenMemory` (component), `SearchResult`.

3. **Actions that mutate them**: `report_missing` (creates violation + institutional record), `ask_about_person` (shares/updates LastSeenMemory), `search_place` (produces SearchResult, updates ExpectationRecord), `escort_to_safety` (moves entities), `report_found` (resolves expectation, updates records).

4. **Information production and travel**: Overdue state begins as an authoritative clock transition on the owner's stored expectation record. Last-seen records propagate through direct observation and dedicated `ask_about_person` hearsay transfer; later report/search aftermath still travels through ordinary local communication and observation rather than global confirmation. Search results are local to the searcher and shared through later runtime/report actions. Missing-person confirmation is not global.

5. **Conserved quantities**: None directly. Expectations are informational records, not physical goods.

6. **Scarce capacities and contention**: Search occupies the searcher (body + time). Multiple agents may search for the same person — no exclusive claim. If the subject is found by one searcher, others discover this through observation or tell, not instant notification.

7. **Partial failures and aftermath**: Search finds nothing → updates memory, may trigger re-search at different location. Search finds evidence but not person → partial information, new leads. Search finds dead body → triggers corpse handling, grief, institutional notification. Escort fails mid-route → both entities exposed to route dangers.

8. **Positive feedback loops**: More overdue expectations → more search goals → more travel → more exposure to danger → potentially more missing persons. Dampener: finite agent count, search takes time and occupies the searcher, agents have competing needs (hunger, rest) that constrain search effort.

9. **Physical dampeners**: Agent fatigue, travel time, competing homeostatic needs, limited memory capacity in `LastSeenMemory`, expectation expiry.

10. **Agent learning**: `LastSeenMemory` records accumulate from observation and hearsay. Agents update route danger beliefs from search encounters. No new learning types beyond existing belief and memory infrastructure.

11. **How agents can be wrong**: Stale last-seen records (subject moved since sighting). Hearsay corruption (chain_depth degrades reliability). False reports of sighting. Subject returns safely after search begins (wasted effort is aftermath, not error).

12. **Lifecycle states**: ExpectationRecord: Active → Overdue → Resolved/Expired. LastSeenRecord: created, updated (newer sighting replaces older), evicted (capacity limit).

13. **Temporal resolution**: Global overdue maintenance runs once per tick as `SystemId::ExpectationCheck` after Perception in the canonical system order. Search actions have explicit tick durations. Grace period is tick-denominated (`u64`).

14. **Boundary conditions**: Expectations about entities that left the simulation boundary are resolved as `NotFound` after extended search. No special boundary logic needed — the entity is simply absent from all searched places.

15. **Derived views**: None. All expectation and last-seen state is authoritative.

16. **Causal records**: Search events are logged with searcher, place, tick, and result. Expectation state transitions are recorded through the normal authoritative state-delta event path. `report_missing` events are logged with reporter and office.

17. **Target patterns and regression cases**: Courier overdue → employer searches → finds wounded → escorts to safety. Merchant fails to return → stale rumor misdirects search → correction → re-search. Guard expected at checkpoint → missing → patrol gap discovered → institutional response.

18. **Save/load and replay**: `ExpectationStore` and `LastSeenMemory` are standard ECS components — survive save/load. All tick-based deadlines are deterministic.

## SystemFn Integration

`check_overdue_expectations` is registered as `SystemId::ExpectationCheck` and runs in the canonical system order after `Perception` and before `EvidenceDecay`.

**Ordering rationale**: The system runs after Perception so same-tick belief updates finish before later systems and AI consume the newly overdue state. It runs before EvidenceDecay so overdue state is established before later cleanup, and before goal generation (which occurs in the AI tick after all systems) so newly overdue expectations can immediately feed later search/report candidate logic.

A new `SystemId::ExpectationCheck` variant is added to the `define_system_ids!` macro in `system_manifest.rs`, and inserted into `SystemManifest::canonical()` between `Perception` and `EvidenceDecay`.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `ExpectationStore` | Agent | Universal | `Default` — empty records map, `next_expectation_id: ExpectationId(0)` |
| `LastSeenMemory` | Agent | Universal | `Default` — all agents can remember where they last saw others; `capacity: 20` |

Both components added to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` as `Option<T>` fields with `unwrap_or_default()` in `spawn_agent()` — always applied with defaults, scenario-overridable.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Perception (E14) | Observation updates `LastSeenMemory`; later search/report behavior consumes both perception results and overdue state | State-mediated |
| Violation goals (S27) | `report_missing` creates `ViolationKind::EntityMissing` through existing violation framework | State-mediated |
| Evidence (S52) | `search_place` reads `SceneEvidence` at the searched location for relevant traces | State-mediated |
| Care (E12) | `escort_to_safety` hands off wounded entity to existing care queue via `queue_for_care_target` | State-mediated |
| Justice (E17) | `report_missing` to an office creates institutional record through existing crime register | State-mediated |
| Social (Tell) | Later `report_found` communication can reuse existing Tell/social channels; `ask_about_person` already transfers positive last-seen information through its dedicated typed action path | State-mediated |
| Corpse handling | Finding a dead body during search triggers existing corpse observation and handling cascade | State-mediated |

## Profile-Driven Parameters

`LastSeenMemory.capacity` is per-agent (scenario-configurable). Higher capacity for institutional agents (guards, magistrates) who need to track more people.

`ExpectationRecord.grace_ticks` is per-expectation, set at creation time based on the basis (patrol check-ins have shorter grace than social promises).

## Outcome

- **Completion date**: 2026-04-07
- **What changed**: All 9 deliverables implemented across 17 tickets (S59EXPOBLSUB-001 through S59EXPOBLSUB-017). Types (`ExpectationRecord`, `LastSeenRecord`, `SearchResult`, etc.) in `worldwake-core`. Components (`ExpectationStore`, `LastSeenMemory`) registered as universal on Agent. `ExpectationCheck` system registered as `SystemId::ExpectationCheck` after Perception. Five new actions (`search_place`, `ask_about_person`, `report_missing`, `escort_to_safety`, `report_found`) with handlers in `worldwake-systems`. Three new `GoalKind` variants (`SearchForMissing`, `ReportMissing`, `EscortToSafety`, `ReportFound`) with candidate generation (`emit_search_candidates`, `emit_escort_candidates`, `emit_report_found_candidates`). Five `PlannerOpKind` variants with planner semantics. `GoalBeliefView` extended with `expectation_store()` and `last_seen_memory()`.
- **Deviations**: None significant. All deliverables landed as specified.
- **Verification**: 5 golden E2E scenarios (Scenarios 120–125) in `golden_expectation.rs` with deterministic replay companions. Golden gap analysis (S67, archived) confirmed comprehensive coverage. All `cargo test --workspace` and `cargo clippy` passing.
