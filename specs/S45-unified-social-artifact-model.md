# S45: Unified Social Artifact Model

## Summary

Introduce a first-class social artifact substrate that gives bounties, warrants, contracts, notices, debts, and obligations the same structural treatment currently reserved for institutional records (offices, factions, crime registers). Today social processes beyond crime/justice/politics are absent: there is no way to post a bounty, issue a warrant, file a debt, publish a notice, or create a contract as a world entity. FOUNDATIONS IV.25 is explicit: "There is no special quest system. There are only world entities and records that people create, discover, believe, dispute, ignore, accept, or fulfill."

The fix introduces a `SocialArtifact` component family with shared identity, custody, location, authenticity, lifecycle, and claim linkage — then implements the first two concrete artifact types (Bounty and Notice) to prove the substrate before expanding.

## Source

Derived from the ChatGPT architecture review (`brainstorming/improvements-to-ai-architecture.md`, Issues #6 and Feature 1) validated against the actual codebase. Confirmed: only 4 `RecordKind` values exist (OfficeRegister, FactionRoster, SupportLedger, CrimeRegister). No bounties, warrants, contracts, debts, notices, or obligations as world entities. The existing `RecordData` pattern (entries with supersession, issuer, home place, consultation) provides a partial model but is institution-bound and lacks general artifact semantics.

Note: `EntityKind::Contract` and `EntityKind::Rumor` currently exist as empty placeholder variants — no components are registered on them and no systems reference them. This spec consolidates social artifacts under a unified `EntityKind::SocialArtifact` with typed content via `ArtifactKind`, and removes the empty Contract/Rumor variants per Principle 28.

## Phase

Phase 5: Architectural Substrates

## Crates

- `worldwake-core` (new `SocialArtifact`, `ArtifactKind`, `ArtifactState`, `BountyTerms`, `NoticeContent` types; new `EntityKind::SocialArtifact`; removal of empty `EntityKind::Contract` and `EntityKind::Rumor`)
- `worldwake-sim` (artifact lifecycle actions; affordance generation for artifact interaction)
- `worldwake-systems` (artifact-domain action handlers: post bounty, claim bounty, post notice)
- `worldwake-ai` (artifact-aware candidate generation; bounty-pursuit goals)

## Dependencies

- S44 (Generalized Contention) is beneficial for bounty claim competition but not blocking. Claim contention can use the substrate once available. (Archived: `archive/specs/S44-generalized-contention-substrate.md`)
- E16c (institutional records) is completed — the existing `RecordData` pattern informs but does not constrain this spec. (Archived: `archive/specs/E16c-institutional-beliefs-and-record-consultation.md`)
- E17 (crime/justice) is completed — bounties and warrants extend the justice chain. (Archived: `archive/specs/E17-crime-theft-justice.md`)

## FOUNDATIONS Alignment

- **Principle 25, Social Artifacts Are First-Class**: "A bounty is a public offer or institutional order with an issuer, conditions, reward source, proof requirements, place of posting, expiration, and possible claimants. A rumor is a transmitted claim with a source and credibility. A robbery report is both a record and a social act. A debt can pressure future behavior even when no coin moves right now. If these are only UI abstractions or hidden controller state, emergence dies."
- **Principle 4, Persistent Identity, Object Permanence, and Explicit Transfer**: Social artifacts have stable identity. A bounty exists at a place, was issued by someone, can be read, claimed, expired, or destroyed.
- **Principle 7, Locality**: Agents learn about artifacts by co-located perception (reading a posted notice) or social transmission (being told about a bounty). No global bounty board.
- **Principle 18, Memory, Evidence, and Records Are World State**: Artifacts are not UI abstractions — they exist as entities/components that can be created, copied, destroyed, forged, or contested.
- **Principle 28, No Backward Compatibility**: Empty `EntityKind::Contract` and `EntityKind::Rumor` variants are removed and consolidated under the unified `SocialArtifact` kind with `ArtifactKind` discriminant. No backward compatibility shims.
- **Canonical Scenario A**: Beast attack → report → bounty → hunt → reward. This requires bounties as world entities.

## Design Goals

1. **Entity-based, not record-based**: Social artifacts are full entities with `EntityId`, components, and relations — not entries in a record ledger. This gives them placement, custody, identity, and component-driven behavior.
2. **Shared substrate, typed content**: All social artifacts share common lifecycle semantics (issuer, custodian, location, state, expiration) but carry type-specific content (bounty terms, notice text, debt amount).
3. **Physical presence**: Artifacts exist at places. A bounty must be posted somewhere. A notice is nailed to a board. Agents discover artifacts by being at the posting place and perceiving them.
4. **Phased introduction**: This spec implements Bounty and Notice. Warrants, contracts, debts are deferred to follow-up specs that reuse the same substrate.
5. **No quest pipeline**: Bounties are not quests. They are world entities with conditions, reward sources, and claimant competition. The AI pursues them through generic goal planning, not a dedicated bounty-pursuit pipeline.

## Deliverables

### 1. `EntityKind::SocialArtifact` and Contract/Rumor removal (`worldwake-core`)

The current `EntityKind` enum (11 variants):

```rust
pub enum EntityKind {
    Agent,
    ItemLot,
    UniqueItem,
    Container,
    Facility,
    Place,
    Faction,
    Office,
    Contract,   // REMOVE — empty placeholder, zero consumers
    Rumor,      // REMOVE — empty placeholder, zero consumers
    Record,
}
```

After this spec (11 variants — net zero change):

```rust
pub enum EntityKind {
    Agent,
    ItemLot,
    UniqueItem,
    Container,
    Facility,
    Place,
    Faction,
    Office,
    Record,
    SocialArtifact,  // NEW — unified kind for bounties, notices, and future artifact types
}
```

Contract and Rumor are removed per Principle 28. They have no components registered in `component_schema.rs`, no systems reference them, and no entities are allocated with these kinds. Future contract and rumor implementations will use `SocialArtifact` with `ArtifactKind::Contract` / `ArtifactKind::Rumor`.

Social artifacts are allocated through the standard generational allocator and participate in the full entity lifecycle (placement, ownership, archival).

### 2. `ArtifactHeader` component (`worldwake-core`)

Shared metadata for all social artifacts:

```rust
/// Common metadata shared by all social artifact types.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactHeader {
    /// What kind of artifact this is.
    pub kind: ArtifactKind,
    /// Who created/issued this artifact.
    pub issuer: EntityId,
    /// Which institution (office/faction) authorized it, if any.
    pub issuing_authority: Option<EntityId>,
    /// Tick when the artifact was created.
    pub created_at: Tick,
    /// Tick when the artifact expires and becomes inactive.
    /// `None` means no expiration.
    pub expires_at: Option<Tick>,
    /// Current lifecycle state.
    pub state: ArtifactState,
    /// Jurisdiction — which office/faction's domain this applies to.
    /// `None` for personal/private artifacts.
    pub jurisdiction: Option<EntityId>,
}

impl Component for ArtifactHeader {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// Public offer for completing a task, with reward.
    Bounty,
    /// Public or semi-public informational posting.
    Notice,
    // Future: Warrant, Contract, Debt, Obligation, Rumor
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactState {
    /// Active and discoverable.
    Active,
    /// Fulfilled — conditions were met.
    Fulfilled,
    /// Expired — past expiration tick.
    Expired,
    /// Withdrawn — issuer cancelled before fulfillment.
    Withdrawn,
    /// Destroyed — physical artifact removed from world.
    Destroyed,
}
```

### 3. `BountyTerms` component (`worldwake-core`)

```rust
/// Terms of a bounty — what must be done and what reward is offered.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BountyTerms {
    /// What the bounty requires. Extensible as more target types appear.
    pub target: BountyTarget,
    /// What proof the claimant must provide.
    pub proof_requirement: ProofRequirement,
    /// Reward commodity and quantity.
    pub reward_commodity: CommodityKind,
    pub reward_quantity: Quantity,
    /// Where the reward comes from.
    pub reward_source: RewardSource,
    /// Where claims should be presented.
    pub claim_place: EntityId,
}

impl Component for BountyTerms {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BountyTarget {
    /// Kill or drive off a specific entity.
    EliminateEntity { target: EntityId },
    /// Deliver a commodity to a place.
    DeliverCommodity {
        commodity: CommodityKind,
        quantity: Quantity,
        destination: EntityId,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProofRequirement {
    /// Physical evidence (corpse, item) must be presented.
    PhysicalEvidence,
    /// Testimony from a credible witness is sufficient.
    WitnessTestimony,
    /// Self-report by the claimant (lowest bar).
    SelfReport,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RewardSource {
    /// Reward paid from an institutional treasury (office/faction entity).
    InstitutionalTreasury { treasury_entity: EntityId },
    /// Reward paid by the issuer personally.
    PersonalFunds { issuer: EntityId },
    /// Reward reserved as a specific item lot.
    ReservedLot { lot: EntityId },
}
```

### 4. `NoticeContent` component (`worldwake-core`)

```rust
/// Content of a public notice.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NoticeContent {
    /// What the notice is about.
    pub topic: NoticeTopic,
}

impl Component for NoticeContent {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NoticeTopic {
    /// Warning about a threat on a route or at a place.
    ThreatWarning { place: EntityId },
    /// Announcement of an office vacancy.
    OfficeVacancy { office: EntityId },
    /// Announcement of a commodity shortage.
    CommodityShortage { commodity: CommodityKind, place: EntityId },
    /// General institutional announcement.
    Institutional { claim: InstitutionalClaim },
}
```

### 5. Component schema registration

Register on `EntityKind::SocialArtifact`:
- `ArtifactHeader` (required)
- `BountyTerms` (for Bounty kind)
- `NoticeContent` (for Notice kind)
- Standard placement components (the artifact exists at a physical place)

### 6. Artifact lifecycle actions (`worldwake-systems`)

All artifact actions use `ActionDomain::Social`.

**PostBounty action**:
- Preconditions: issuer is an office holder or has personal funds; co-located with posting place.
- Duration: 1-2 ticks (the act of posting).
- Effects: Creates a `SocialArtifact` entity with `ArtifactHeader` + `BountyTerms`, placed at the posting location.
- Event: Emits a bounty-posted event with witness data.

**ClaimBounty action**:
- Preconditions: claimant co-located with `claim_place`; holds required proof; bounty is Active.
- Duration: 1-2 ticks (presenting the claim).
- Effects: Validates proof, transfers reward from source to claimant, sets bounty state to Fulfilled.
- Contention: Bounty claims go through `ContentionQueue` (S44 substrate). A `ContentionPolicy` with `max_waiters: Some(0)` (race mode) is attached to the bounty entity at creation — first valid claimant to start the action wins. Subsequent claimants receive a structured `QueueFull` rejection and can replan.
- Partial failures:
  - Treasury depleted between posting and claim: ClaimBounty commit checks treasury balance. If insufficient, the claim fails with a clear "treasury depleted" error; bounty remains Active but the claimant receives no reward. The bounty may be withdrawn by the issuer or expire naturally.
  - Proof contested: If `ProofRequirement::WitnessTestimony` is required but no qualifying witness testimony exists in the claimant's belief store, claim is rejected with "insufficient proof."
  - Claimant leaves claim_place mid-action: Action aborts; bounty remains Active.
- Event: Emits bounty-claimed event.

**PostNotice action**:
- Preconditions: issuer co-located with posting place.
- Duration: 1 tick.
- Effects: Creates a `SocialArtifact` entity with `ArtifactHeader` + `NoticeContent`.

**ReadNotice/ReadBounty** — handled through standard perception: co-located agents with the artifact entity perceive its content during the perception system pass, similar to how agents perceive other entities. The artifact's content becomes a belief (e.g., "there is a bounty for killing the wolf, reward 10 food, posted at the town square").

### 7. Artifact perception integration

The perception system already iterates all co-located entities generically via `world.entities_effectively_at(place)` (`perception.rs:214`). No entity-kind-specific iteration logic is needed.

The new work is an **artifact-specific belief creation handler** within the perception system that:
1. When an agent perceives an entity with `ArtifactHeader`, reads the header and type-specific content component (`BountyTerms` or `NoticeContent`).
2. Populates a `BelievedArtifactState` field on the perceived entity's `BelievedEntityState`.
3. For notices, also internalizes notice content as an institutional belief or entity belief depending on topic.

```rust
/// Believed state of a perceived social artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedArtifactState {
    /// What kind of artifact this is.
    pub kind: ArtifactKind,
    /// Believed lifecycle state at time of observation.
    pub state: ArtifactState,
    /// Who issued it.
    pub issuer: EntityId,
    /// When it expires (if known).
    pub expires_at: Option<Tick>,
    /// For bounties: believed target and reward.
    pub bounty_terms: Option<BelievedBountyTerms>,
    /// For notices: believed topic.
    pub notice_topic: Option<NoticeTopic>,
    /// Tick when this artifact state was observed.
    pub observed_tick: Tick,
}

/// Simplified bounty terms as perceived by an agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedBountyTerms {
    pub target: BountyTarget,
    pub reward_commodity: CommodityKind,
    pub reward_quantity: Quantity,
    pub claim_place: EntityId,
}
```

This is added as `pub believed_artifact: Option<BelievedArtifactState>` on `BelievedEntityState`, following the existing pattern of `believed_contention: Option<BelievedContentionState>`.

Agents who perceived a bounty/notice can share that knowledge via Tell (using existing social transmission). Source degradation applies normally. An agent who hears about a bounty second-hand holds a believed artifact state with lower confidence than one who read it directly.

### 8. AI integration — bounty pursuit goals

Add `GoalKind::FulfillBounty { bounty: EntityId }` to the goal enum. Note: `GoalKind` derives `Copy` — all fields must be Copy types (`EntityId` is Copy).

- **Candidate generation**: Add `emit_bounty_candidates()` in `candidate_generation.rs`. When an agent believes an Active bounty exists (via `believed_artifact` on a `BelievedEntityState`) and the agent can potentially fulfill the terms (has combat ability for EliminateEntity, has goods for DeliverCommodity), emit a bounty-pursuit candidate.
- **Ranking**: Bounty pursuit ranked by `enterprise_weight` × reward value, competing with other enterprise goals.
- **Planning**: GOAP search for: Travel to target → accomplish target action → Travel to claim place → ClaimBounty.
- **Invalidation**: Invalidated when believed bounty state changes to Fulfilled, Expired, or Withdrawn. If the bounty becomes Fulfilled between plan formation and ClaimBounty execution (e.g., another agent claimed it first), the action fails at precondition check and `handle_plan_failure` triggers replanning — the agent abandons the fulfilled bounty and considers other goals.

### 9. Artifact expiration system

A per-tick `artifact_lifecycle_system()` registered as `SystemId::ArtifactLifecycle` that:
1. Checks all Active artifacts against current tick.
2. Transitions expired artifacts to `ArtifactState::Expired` at the **beginning** of the tick in which `current_tick >= expires_at`. This ensures no actions can target an expired artifact within the expiration tick.
3. Emits expiration events for perception (co-located agents perceive the state change).

System ordering: runs after action domain systems and before Perception, alongside Contention.

### 10. Golden tests

**Scenario A: Bounty lifecycle (Canonical Scenario A fragment)**
- Office holder posts bounty for eliminating a hostile entity at the town square.
- Agent perceives bounty at town square.
- Agent travels to target, eliminates it.
- Agent travels to claim place with evidence (corpse entity or trophy).
- Agent claims bounty; reward transfers from treasury to agent.
- Bounty state becomes Fulfilled.

**Scenario B: Bounty expiration**
- Bounty posted with `expires_at: Tick(50)`.
- No one claims it by tick 50.
- Artifact transitions to Expired.
- Agent arriving at tick 51 perceives expired bounty and does not pursue.

**Scenario C: Notice discovery**
- Office posts ThreatWarning notice at town square.
- Agent travels to town square, perceives notice.
- Agent internalizes threat warning as belief, adjusts route planning.

All scenarios with deterministic replay companions.

## Section H — FOUNDATIONS Analysis

### H.1 Information-path analysis

Social artifacts follow strict locality:
1. **Creation**: Issuer (agent or office) creates artifact at a physical place via an explicit action.
2. **Discovery**: Co-located agents perceive the artifact through the standard perception system. No global artifact registry is queried by agents.
3. **Propagation**: Agents who perceived a bounty/notice can share that knowledge via Tell (using existing social transmission). Source degradation applies normally.
4. **Claim**: Claimant must be co-located with the claim place. Proof must be physically present.

No information teleportation. No global bounty board. Agents in remote places do not know about local bounties until information reaches them through lawful carriers.

### H.2 Positive-feedback analysis

**Potential loop: bounty → hunt → success → reward → wealth → more bounties posted**
This is an intended economic cycle. The dampeners are:

1. **Treasury depletion**: Bounty rewards come from real treasuries. Posting bounties depletes the treasury. An office that posts too many bounties runs out of funds.
2. **Supply exhaustion**: If all the wolves are killed, no more wolf bounties are justified (the threat is gone).
3. **Claimant competition**: Multiple agents competing for the same bounty means only one collects the reward. Others waste effort. (Enhanced by S44 contention.)
4. **Travel time and risk**: Pursuing a bounty costs time and exposure to danger.

**Potential loop: notices → fear → avoidance → more notices about empty areas**
Dampened by: notices expire, new information contradicts stale notices, agents with different risk tolerances ignore warnings.

### H.3 Concrete dampeners

| Loop | Dampener |
|------|----------|
| Bounty economy spiral | Treasury depletion (finite reward source), target depletion (finite threats), claimant competition (one winner) |
| Notice fear cascade | Expiration, contradiction by fresh observation, agent diversity in risk tolerance |
| Artifact entity accumulation | Expiration → Destroyed lifecycle, entity archival/purge for inactive artifacts |

### H.4 Stored state vs. derived read-model list

| Item | Classification |
|------|---------------|
| `ArtifactHeader` on artifact entity | **Stored authoritative state** |
| `BountyTerms` on artifact entity | **Stored authoritative state** |
| `NoticeContent` on artifact entity | **Stored authoritative state** |
| `ArtifactState` transitions | **Stored authoritative state** (lifecycle mutations via explicit actions or expiration system) |
| Agent's `BelievedArtifactState` in belief store | **Derived belief** — perceived snapshot, may be stale |
| Bounty pursuit candidate in AI pipeline | **Derived** — generated per-tick from beliefs, ephemeral |

### H.5 Contention and exclusive affordances

Bounty claims are a contested exclusive affordance — only one claimant should collect the reward for a given bounty.

**Resolution mechanism**: At bounty creation, a `ContentionPolicy` is attached to the `SocialArtifact` entity with `max_waiters: Some(0)` (race mode). This means:
- First valid claimant to start the ClaimBounty action receives the grant.
- Subsequent claimants receive `ContentionError::QueueFull` — a structured rejection they can replan from.
- No waiting queue: agents who lose the race immediately replan (choose a different bounty, return to other goals).

**Tie-breaking**: Same-tick ties resolved by the contention substrate's ordinal-based ordering (deterministic, seeded).

PostBounty and PostNotice are not contested — any eligible agent can post independently.

### H.6 Partial failures and aftermath

| Failure | Aftermath |
|---------|-----------|
| ClaimBounty: treasury depleted | Claim fails at commit. Bounty remains Active. Claimant wastes time but gains no reward. Bounty may be withdrawn or expire. |
| ClaimBounty: proof insufficient | Claim rejected at precondition. Claimant must acquire valid proof. Bounty remains Active. |
| ClaimBounty: claimant leaves mid-action | Action aborts. Bounty remains Active. Contention grant released. |
| ClaimBounty: bounty already Fulfilled | Precondition fails. Claimant replans via `handle_plan_failure`. |
| ClaimBounty: bounty Expired during travel | Precondition fails. Agent perceives expired state on arrival. |
| PostBounty: issuer lacks funds/authority | Precondition fails. No artifact created. |

Every failure produces new state — no silent dead ends. Failed claims leave the agent at the claim place with updated beliefs, triggering replanning.

### H.7 Belief staleness and correction

Agents can become wrong about artifacts in several ways:
1. **Stale Active belief**: Agent believes bounty is Active, but it was claimed or expired. Correction: agent returns to posting place and perceives the updated state, or hears via Tell from another agent who perceived the change.
2. **Unknown bounty**: Agent never visited the posting place and has no belief about the bounty. No correction needed — ignorance is the default state.
3. **Second-hand staleness**: Agent heard about a bounty via Tell chain. The heard belief carries lower confidence and standard staleness decay. Agent may choose to verify by visiting the posting place before investing in pursuit.

**Provenance markers**: `BelievedArtifactState.observed_tick` and the `PerceptionSource` on the parent `BelievedEntityState` provide freshness and source-chain metadata. Agents with higher `stale_evidence_barrier_threshold` (from `EpistemicDispositionProfile`) are more likely to verify stale bounty beliefs before acting on them.

### H.8 Temporal resolution

- **Expiration**: `artifact_lifecycle_system()` runs at the beginning of each tick, before action domain systems. An artifact with `expires_at == current_tick` transitions to Expired before any actions can target it in that tick. This prevents race conditions between expiration and last-moment claims.
- **Creation visibility**: A newly posted artifact is placed at the posting location during the PostBounty/PostNotice action commit. It becomes perceivable by co-located agents on the next perception system pass (same tick, since Perception runs after action domain systems).
- **Claim finality**: ClaimBounty commit sets `ArtifactState::Fulfilled` atomically. The contention race (H.5) prevents multiple simultaneous claims.

## Cross-System Interactions (Principle 12)

- **Office/faction system** writes artifact creation events when authorized agents post bounties/notices.
- **Perception system** reads artifact entities at same place → writes to agent belief stores (via `BelievedArtifactState`).
- **AI candidate generation** reads believed artifacts from beliefs → emits pursuit candidates.
- **Action system** reads artifact state and proof conditions → validates claims.
- **Artifact lifecycle system** reads current tick → transitions expired artifacts.
- **Contention system** manages bounty claim exclusivity via race-mode `ContentionPolicy`.
- **Tell system** carries artifact knowledge through standard social transmission.
- **Economy** — reward transfer uses existing commodity transfer mechanisms (`transfer_selected_lots`).

No system invokes another's privileged behavior. All interaction through state and event log.

## Future Extensions (not in this spec)

| Artifact Type | ArtifactKind variant | Description | Prerequisite |
|---------------|---------------------|-------------|--------------|
| Warrant | `Warrant` | Institutional order to apprehend/detain a named agent | Extended rights model |
| Contract | `Contract` | Two-party agreement with obligations and penalties | Obligation substrate |
| Debt | `Debt` | Owed quantity from debtor to creditor with repayment terms | Obligation substrate |
| Patrol Order | `PatrolOrder` | Standing duty assignment with route and schedule | E19 guard patrol |
| Trade License | `TradeLicense` | Permission to trade at a specific market | Market infrastructure |
| Rumor | `Rumor` | Persistent social claim with source and credibility | Social transmission |

All future types reuse `ArtifactHeader` + type-specific content components. The empty `EntityKind::Contract` and `EntityKind::Rumor` variants are removed by this spec; future implementations use `EntityKind::SocialArtifact` with the appropriate `ArtifactKind`.

## Migration Path

1. Remove `EntityKind::Contract` and `EntityKind::Rumor` (zero consumers; update `ALL_ENTITY_KINDS` test constant).
2. Add `EntityKind::SocialArtifact` to the entity kind enum.
3. Add `ArtifactHeader`, `ArtifactKind`, `ArtifactState` to `worldwake-core`.
4. Add `BountyTerms`, `BountyTarget`, `ProofRequirement`, `RewardSource` to `worldwake-core`.
5. Add `NoticeContent`, `NoticeTopic` to `worldwake-core`.
6. Add `BelievedArtifactState`, `BelievedBountyTerms` to `worldwake-core` belief module.
7. Register components in schema for `EntityKind::SocialArtifact`.
8. Add `SystemId::ArtifactLifecycle` and `artifact_lifecycle_system()` to `worldwake-systems`.
9. Add PostBounty, ClaimBounty, PostNotice action handlers using `ActionDomain::Social`.
10. Extend perception for artifact entities (artifact-specific belief creation handler).
11. Add `GoalKind::FulfillBounty` with candidate generation and planning support.
12. Write golden tests.
13. Bump `SAVE_FORMAT_VERSION` from 17 to 18.

## Verification

- `cargo test --workspace` passes — no existing behavior changed.
- Golden test A proves end-to-end bounty lifecycle (post → perceive → pursue → claim → reward).
- Golden test B proves expiration lifecycle.
- Golden test C proves notice-driven belief acquisition.
- Save/load round-trip preserves all artifact entities and components.
- Conservation invariants hold — bounty rewards transfer from real sources.
- `cargo clippy --workspace` clean.
