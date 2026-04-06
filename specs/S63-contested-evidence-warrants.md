# S63: Contested Evidence and Warrants

## Summary

Extend the justice system (E17) with warrants, detention, case records, alibi, evidence contest, and wrongful-accusation correction. Currently justice is a linear accuse → verdict → punish pipeline with no mechanism for contested evidence, wrongful accusation, or institutional correction after new evidence arrives. This spec adds the "world can be socially wrong" layer — institutions act on incomplete or false evidence, and later correction propagates unevenly.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (warrant, case, alibi, exoneration types)
- `worldwake-systems` (warrant, detention, evidence-contest actions)
- `worldwake-ai` (institutional goal generation for warrants, evidence gathering)

## Dependencies

- E17 (crime/theft/justice) — completed
- S45 (social artifacts) — completed
- S52 (evidence aftermath) — completed
- S59 (expectation-obligation substrate) — overdue expectations can trigger suspicion that feeds into wrongful accusation

## Design Goals

- Warrants are social artifacts with issuer, target, basis, jurisdiction, and expiry — not invisible system flags
- Detention is a concrete state: the detained entity is held at a place, cannot leave, and occupies institutional resources
- Evidence is contestable: conflicting witness claims, alibi records, physical evidence vs testimony
- Correction appends new records (exoneration, case revision) — it never overwrites history (P29A)
- Different offices may update at different times, creating institutional lag and contradiction (P16)

## Non-Goals

- Trial system with formal proceedings, jury, or courtroom — deferred
- Evidence forging or planting — deferred
- Appeals process or multi-level jurisdiction — deferred
- Witness protection or intimidation — deferred
- Lawyer/advocate agents — deferred

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State) | Warrants, case records, and alibi are stored state, not derived from event queries |
| P4 (Persistent Identity) | Each warrant, case, and alibi has stable identity and lifecycle |
| P7 (Locality) | Warrants must be physically distributed to jurisdictions. Correction propagates through carriers |
| P14 (World ≠ Belief) | An agent may be accused based on false beliefs; truth and institutional knowledge diverge |
| P16 (Contradiction First-Class) | Conflicting evidence, split institutional response, and delayed correction are core features |
| P18 (Records Are World State) | Warrants, alibis, case records, and exoneration records are inspectable world state |
| P25 (Social Artifacts) | Warrants are social artifacts with issuer, conditions, jurisdiction, and lifecycle |
| P25A (Artifact Lifecycle) | Warrants can be active, served, expired, or revoked — distinct states |
| P29A (Append-Only History) | Exoneration appends new records; the original accusation and warrant remain in history |

## Deliverables

### 1. Warrant Types

```rust
/// A warrant issued by an office for the detention or search of a subject.
/// Added to InstitutionalClaim enum.
InstitutionalClaim::Warrant {
    /// The office issuing the warrant.
    issuing_office: EntityId,
    /// The subject of the warrant.
    subject: EntityId,
    /// What the warrant authorizes.
    warrant_kind: WarrantKind,
    /// The accusation or case this warrant is based on.
    basis: RecordEntryId,
    effective_tick: Tick,
    /// Expiry tick. None = indefinite.
    expires_tick: Option<Tick>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WarrantKind {
    /// Arrest and bring before the issuing office.
    Arrest,
    /// Search the subject's belongings or premises.
    Search,
    /// Detain at a specified location pending investigation.
    Detention,
}
```

### 2. Case Record

```rust
/// A formal case record tracking an investigation from accusation to resolution.
/// Stored in RecordData (same append-only pattern as existing CrimeRegister).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaseRecord {
    pub case_id: CaseId,
    /// The accused entity.
    pub accused: EntityId,
    /// The accusing entity or institution.
    pub accuser: EntityId,
    /// Jurisdiction office handling the case.
    pub jurisdiction: EntityId,
    /// Evidence entries associated with this case.
    pub evidence_entries: Vec<CaseEvidence>,
    /// Current case state.
    pub state: CaseState,
    pub opened_tick: Tick,
    pub last_updated_tick: Tick,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CaseId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaseEvidence {
    /// What kind of evidence this is.
    pub kind: CaseEvidenceKind,
    /// Who provided this evidence.
    pub source: EntityId,
    /// When this evidence was recorded.
    pub recorded_tick: Tick,
    /// How credible the institution considers this evidence.
    pub credibility: Permille,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CaseEvidenceKind {
    /// Witness testimony (may be true or false).
    WitnessTestimony { witness: EntityId, claim: String },
    /// Physical evidence from a scene.
    PhysicalEvidence { evidence_id: EvidenceEntryId, place: EntityId },
    /// Alibi — someone claims the accused was elsewhere.
    Alibi { alibi_witness: EntityId, claimed_place: EntityId, claimed_tick: Tick },
    /// Institutional record (prior convictions, character reference).
    InstitutionalRecord { record_entry: RecordEntryId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CaseState {
    /// Under investigation — gathering evidence.
    Open,
    /// Warrant issued, awaiting subject detention.
    WarrantIssued,
    /// Subject detained, evidence being reviewed.
    SubjectDetained,
    /// Verdict reached.
    Resolved { verdict: CaseVerdict },
    /// Case closed without resolution (insufficient evidence, subject fled, etc).
    Closed { reason: CaseClosureReason },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CaseVerdict {
    Guilty { punishment: PunishmentKind },
    Exonerated,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CaseClosureReason {
    InsufficientEvidence,
    SubjectFled,
    SubjectDead,
    Superseded { by_case: CaseId },
    Withdrawn,
}
```

### 3. Detention State

```rust
/// Marks an agent as currently detained by an institution.
/// Registered on EntityKind::Agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetentionState {
    /// The office holding the agent.
    pub holding_office: EntityId,
    /// The place where the agent is detained.
    pub detention_place: EntityId,
    /// The case this detention is associated with.
    pub case_id: CaseId,
    pub since_tick: Tick,
}
```

When an agent has `DetentionState`, their action space is restricted: no travel, no trade, limited social interaction. The detained agent can still speak (to provide testimony or alibi) but cannot leave the detention place.

### 4. New Actions

#### `issue_warrant`
- **Preconditions**: Actor holds an office with jurisdiction. A case record exists with sufficient basis for the warrant kind. No existing active warrant for the same subject and case.
- **Duration**: Short (institutional action).
- **Effect**: Creates `InstitutionalClaim::Warrant` on the jurisdiction's record. Warrant is a social artifact posted at the office — agents must observe or be told about it.
- **Domain**: `ActionDomain::Social`

#### `serve_warrant`
- **Preconditions**: Actor is a patrol/guard agent. Actor has observed or been told about the warrant. Actor is co-located with the warrant subject.
- **Duration**: Medium (apprehension).
- **Effect**: If subject cooperates: apply `DetentionState`, move to office. If subject resists: combat. Creates event record.
- **Domain**: `ActionDomain::Social`

#### `detain`
- **Preconditions**: Actor holds an office. Subject is co-located. Warrant or office authority justifies detention.
- **Duration**: Short.
- **Effect**: Applies `DetentionState` to subject. Updates case record to `SubjectDetained`.
- **Domain**: `ActionDomain::Social`

#### `release`
- **Preconditions**: Actor holds the office that issued the detention. Case resolved or insufficient basis for continued detention.
- **Duration**: Short.
- **Effect**: Removes `DetentionState`. Subject regains full action space. Case updated.
- **Domain**: `ActionDomain::Social`

#### `present_evidence`
- **Preconditions**: Actor has evidence relevant to an open case. Actor is co-located with the jurisdiction office-holder.
- **Duration**: Short (social/epistemic action).
- **Effect**: Adds `CaseEvidence` to the case record. Office-holder updates beliefs about the case. May trigger case state change.
- **Domain**: `ActionDomain::Epistemic`

#### `contest_evidence`
- **Preconditions**: Actor has contradicting evidence or testimony. An open case exists where the actor is the accused or has relevant knowledge.
- **Duration**: Short.
- **Effect**: Adds contradicting `CaseEvidence` to the case. May reduce credibility of prior evidence. Office-holder re-evaluates.
- **Domain**: `ActionDomain::Epistemic`

#### `record_alibi`
- **Preconditions**: Actor witnessed the accused at a different place during the time of the alleged crime.
- **Duration**: Short.
- **Effect**: Adds `CaseEvidenceKind::Alibi` to the case record.
- **Domain**: `ActionDomain::Epistemic`

#### `revise_case`
- **Preconditions**: Actor holds jurisdiction. New evidence changes the weight of the case significantly.
- **Duration**: Short.
- **Effect**: Updates case state. May issue exoneration, change verdict, or close case. Appends revision record — does not overwrite history.
- **Domain**: `ActionDomain::Social`

### 5. Goal Kinds

```rust
GoalKind::IssueWarrant { subject: EntityId, case_id: CaseId }
GoalKind::ServeWarrant { subject: EntityId, warrant_entry: RecordEntryId }
GoalKind::PresentEvidence { case_id: CaseId }
GoalKind::ContestAccusation { case_id: CaseId }
GoalKind::RecordAlibi { accused: EntityId, case_id: CaseId }
```

**Candidate generation**: Office-holders with open cases and sufficient evidence generate `IssueWarrant`. Patrol agents who observe warrant subjects generate `ServeWarrant`. Witnesses with relevant observations generate `PresentEvidence`. The accused generates `ContestAccusation` if they believe they are innocent (belief about own actions). Witnesses of the accused's whereabouts generate `RecordAlibi`.

### 6. Institutional Lag and Uneven Correction

When a case is revised or a verdict changed at one office:
- The revision is recorded at that office's jurisdiction only
- Other offices with copies of the warrant or accusation are NOT automatically updated
- Correction propagates through the same channels as the original: messenger, notice board, tell
- An agent exonerated at one office may still be pursued under an outdated warrant from another
- This creates the "one office updates, another does not" pattern demanded by canonical regression G

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: The current justice system has no mechanism for contested truth, wrongful accusation, or institutional correction. Canonical regression G cannot be produced.

2. **New entities/relations/records**: `InstitutionalClaim::Warrant`, `CaseRecord` (in RecordData), `DetentionState` (component on Agent), `CaseEvidence`, `CaseId`.

3. **Actions that mutate them**: `issue_warrant`, `serve_warrant`, `detain`, `release`, `present_evidence`, `contest_evidence`, `record_alibi`, `revise_case`.

4. **Information production and travel**: Warrants are posted at offices — agents must observe them. Corrections propagate through tell/notice/observation. No instant broadcast.

5. **Conserved quantities**: None directly. Warrants and case records are informational state.

6. **Scarce capacities and contention**: Detention occupies a place (detained agent held there). Office-holder time is occupied by case management. Multiple cases compete for office-holder attention.

7. **Partial failures and aftermath**: False accusation → wrongful detention → later exoneration that does not undo damage. Warrant expires before service. Subject flees before detention. Evidence lost or decayed.

8. **Positive feedback loops**: Accusation → detention → missed obligations → more suspicion. Dampener: case expiry, evidence requirements, office-holder capacity limits, alibi presentation.

9. **Physical dampeners**: Office-holder time limits case processing speed. Evidence decay reduces available proof over time. Warrant expiry prevents indefinite pursuit. Subject death closes cases.

10. **Agent learning**: Office-holders update case beliefs from new evidence. Guards learn warrant targets from observation. Accused learns of accusation from social channels.

11. **How agents can be wrong**: False testimony leads to wrongful accusation. Stale warrant pursued after exoneration. Alibi from unreliable witness dismissed. Evidence misattributed.

12. **Lifecycle states**: Warrant: Active → Served → Expired → Revoked. CaseState: Open → WarrantIssued → SubjectDetained → Resolved/Closed. DetentionState: applied → removed.

13. **Temporal resolution**: Case processing is action-driven (office-holder decides when to act). Warrant expiry is tick-based. Detention is indefinite until release action.

14. **Boundary conditions**: Off-map subjects cannot be pursued. Warrants have jurisdiction limits. Cases involving boundary arrivals (S62 refugees) create interesting jurisdictional questions.

15. **Derived views**: None. All case and warrant state is authoritative.

16. **Causal records**: All case state changes logged. Warrant issuance, service, and revocation logged. Evidence presentation logged with source and credibility.

17. **Target patterns**: False testimony → warrant → suspect detained → alibi arrives → one office updates, another does not. Conflicting witnesses → office acts on incomplete evidence → correction propagates unevenly.

18. **Save/load and replay**: All components are standard ECS. Case records use existing RecordData append-only pattern. Deterministic.

## SystemFn Integration

No new system tick function. Warrant expiry can be checked during the existing institutional maintenance pass. Case processing is entirely action-driven by office-holders.

`DetentionState` restricts the agent's action space — the action precondition system already checks component state, so adding a `has_detention_state → cannot travel` check integrates into existing precondition infrastructure.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `DetentionState` | Agent | Role-specific | `None` — only detained agents |

`DetentionState` is runtime-generated state (applied by `detain` action, removed by `release`), not scenario-configured. Exempt from `AgentDef` requirements per spec-drafting-rules section 5.

`CaseRecord` is stored in `RecordData` on the jurisdiction office entity, using the existing record infrastructure.

Warrant is stored as `InstitutionalClaim::Warrant` in the existing claim system.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Crime/Justice (E17) | Extends existing accuse → verdict pipeline with intermediate warrant/detention/case steps | State-mediated |
| Evidence (S52) | Physical scene evidence feeds into case evidence bundles | State-mediated |
| Social artifacts (S45) | Warrants are social artifacts with the standard lifecycle | State-mediated |
| Perception (E14) | Warrant observation at offices, detention observation by co-located agents | State-mediated |
| Patrol (E19) | Patrol agents observe and serve warrants | State-mediated |
| Expectations (S59) | Overdue expectations can trigger suspicion → accusation → warrant chain | State-mediated |
| Records (E16c) | Case records use existing `RecordData` supersession pattern | State-mediated |

## Profile-Driven Parameters

`CaseEvidence.credibility` is set by the office-holder based on evidence kind, source reliability, and corroboration — not a fixed constant. Office-holders with higher `CognitiveProfile` (S53) may weigh evidence more carefully.

Warrant `expires_tick` is set by the issuing office based on jurisdiction policy (scenario-configurable per office).

No new per-agent profile component needed. Detention tolerance, evidence evaluation, and case management behavior emerge from existing `CognitiveProfile`, `DriveThresholds`, and office authority.
