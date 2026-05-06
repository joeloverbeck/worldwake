# S140: Multi-Axis Artifact Lifecycle

**Status**: COMPLETED

## Summary

`ArtifactHeader` (`crates/worldwake-core/src/social_artifact.rs:5`) and `ArtifactState` (`social_artifact.rs:55`) currently model artifacts (notices and bounties; accusations are tracked via the justice subsystem and do not live on `ArtifactHeader` today) as a single discriminator with five flat variants — `Active`, `Fulfilled`, `Expired`, `Withdrawn`, `Destroyed`. FND-25A names five orthogonal axes — existence, visibility, legal effect, credibility, actionability — and warns against collapsing them into a single discriminator. Today's flattened shape conflates concerns that FND-25A says must vary independently:

- An expired bounty stops being visible the same tick its legal authorization to issue rewards lapses, even though FND-25A explicitly requires it to remain inspectable as a closed case.
- A revoked warrant, exonerated accusation, or fulfilled contract collapses into one of the existing flat variants without preserving the distinction. The current `Withdrawn` does not separate "withdrawn by issuer" from "revoked by adjudication" from "superseded by later evidence", and `Fulfilled` carries no provenance about *who* fulfilled it.
- A rumor's credibility (Disputed, Refuted, Unknown) has no representation at all in `ArtifactState` — a refuted rumor must still be inspectable as part of FOUNDATIONS Scenario G's exoneration chain, and today the only options are "alive" or one of the closed variants.
- Posted notices have no separate "actionable" axis — the planner cannot tell "this notice exists and is visible but the underlying claim has been adjudicated and is no longer actionable." Today, only `FulfillBounty` candidate generation reads `ArtifactState` (`crates/worldwake-ai/src/candidate_generation.rs:650`); all other artifact-anchored goals (`Accuse`, `PunishAccused`, `PostBounty`, `PostNotice`) ignore artifact state entirely.

S140 separates the five axes into distinct typed fields on `ArtifactHeader`. Existence becomes `ArtifactExistence::{Exists, Destroyed}`. Visibility becomes `ArtifactVisibility::{Hidden, Private, Posted, WidelyKnown}`. Legal effect becomes `ArtifactLegalEffect::{None, Active, Suspended, Expired, Revoked, Fulfilled}`. Credibility becomes `ArtifactCredibility::{Credible, Disputed, Refuted, Unknown}`. Actionability becomes `ArtifactActionability::{Actionable, AwaitingProof, Blocked, Closed}`. Each axis has explicit lawful transitions through declared world processes (expiry, fulfillment, revocation, supersession, adjudication) and per-axis observers. The existing `ArtifactPostingProfile` TTL becomes one specific transition (legal_effect: Active → Expired); other transitions emerge from explicit causal events. The currently-used flat `ArtifactState` variants map onto specific axis combinations as documented in the Migration Map below; `ArtifactState` itself is removed in the same change per FND-28.

Sale listings are explicitly out of scope for this spec. They are a separate substrate (`SaleListing` component in `crates/worldwake-systems/src/trade.rs` and `stock_actions.rs`) that does not use `ArtifactHeader` or `ArtifactState`. If sale listings are later promoted to artifact status, that is a sibling spec, not a hidden deliverable here.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — COMPLETED

## Crates

- `worldwake-core` — refactors `ArtifactHeader` to embed five typed axis fields. Adds `ArtifactExistence`, `ArtifactVisibility`, `ArtifactLegalEffect`, `ArtifactCredibility`, `ArtifactActionability` enums plus the supporting payload enums `DestructionCause`, `SuspensionReason`, `RevocationReason`, `ProofKind`, `BlockerReason`, `CloseCause`. Adds `AxisName`, `ArtifactAxisValue`, and `ArtifactTransitionPayload`. Adds `Discrepancy::ArtifactNotActionable`. `ArtifactState` (the prior single-enum) is removed. `BelievedArtifactState` (carried by `EntityBeliefClaim::ArtifactState` at `crates/worldwake-core/src/entity_belief_claim.rs:42`) is migrated to a per-axis representation. `SAVE_FORMAT_VERSION` increments from 70 across the S140 migration; the persisted transition-payload carrier added in S140ARTLIFAXE-002 bumps the live format to 72, the persisted artifact-actionability discrepancy shape in S140ARTLIFAXE-003 bumps the live format to 73, and the artifact-addressed `InstitutionalClaim::ArtifactCredibilityRefutation` carrier added in S140ARTLIFAXE-008 bumps the live format to 74.
- `worldwake-sim` — `EventTag::ArtifactTransition` added. Decode and replay paths updated.
- `worldwake-systems` — `crates/worldwake-systems/src/artifact_lifecycle.rs` extended with per-axis transition handlers. `crates/worldwake-systems/src/artifact_actions.rs` (which contains the existing `register_post_bounty_action`, `register_post_notice_action`, `register_withdraw_bounty_action`, `register_claim_bounty_action` registrations and their commit handlers) emits per-axis transitions in place of the flat-state writes at lines 1193, 1293, 1382, 1497. Justice-subsystem accusation handling in `crates/worldwake-systems/src/justice_actions.rs` participates only if and to the extent that accusations are landed as artifacts in this spec — the spec scopes that explicitly under "Accusation participation" below.
- `worldwake-ai` — planner reads `actionability` not `existence` when evaluating "can I act on this artifact?" A revoked warrant still exists and is visible but is non-actionable. Decision-trace surfaces axis values for artifacts referenced by ranked candidates. The single existing artifact-state gate (`candidate_generation.rs:650`) is migrated; no new gates are introduced for goal kinds that did not previously filter on `ArtifactState`.
- `worldwake-cli` — observer adds a new section ("Section 11 — Artifact Lifecycle") rendering per-axis state. `NoticeDef` is renamed and unified into `ArtifactDef` so future artifact classes (bounties, accusations) can be authored uniformly; the unified Def carries optional per-axis initial state.

## Dependencies

- S97 (Post Notice Artifact TTL) — completed. `ArtifactPostingProfile` continues to govern the `Active → Expired` legal-effect transition. S140 layers the other axes atop.
- S125 (Institutional Treasuries and Bounty Funding) — completed. Bounty fulfillment becomes the `legal_effect: Active → Fulfilled` transition with reward release through the existing treasury path (see `commit_claim_bounty` at `artifact_actions.rs:1434`).
- S110 (Decision History Events) — completed. Per-axis transitions emit through the existing event-log surface.
- S109 (Typed Discrepancy Taxonomy) — completed. `Discrepancy::ArtifactNotActionable` joins the taxonomy for planner-side filtering.

## Design Goals

1. **Five orthogonal axes.** Existence, visibility, legal effect, credibility, actionability vary independently. A revoked warrant: `Exists | Posted | Revoked | Credible | Closed`. An exonerated accusation: `Exists | Posted | None | Refuted | Closed`. An expired-but-still-posted bounty: `Exists | Posted | Expired | Credible | Closed`.
2. **Lawful transitions only.** Each axis has typed payload variants naming the world events that move it. No axis can be mutated except through a declared transition.
3. **Per-axis history.** Every transition emits an `EventTag::ArtifactTransition` carrying the axis name, prior value, new value, and cause-event reference. History is append-only (FND-29A).
4. **Visibility ≠ existence.** A hidden artifact still exists; a destroyed artifact has no remaining axes. `Destroyed` is terminal across all axes; the artifact's record persists in event-log history but the live artifact entity is dropped.
5. **Actionability is the planner's gate.** `actionability == Actionable` is the precondition for goal candidates anchored on the artifact. Other axes inform belief and ranking but do not gate planner action. The current single gate at `candidate_generation.rs:650` becomes the actionability gate; goal kinds that did not previously filter on artifact state continue not to filter (see "Planner gate scope" deliverable for the explicit enumeration).
6. **Credibility decays through evidence.** Credibility transitions through `evidence_against`, `evidence_for`, `contradicting_testimony`, `confirmed_by_witness` — all observed event types. No silent decay.
7. **No backward-compat shim.** `ArtifactState` (single-enum) is removed in the same change. The old variants map to specific axis combinations per the Migration Map below; the migration path is per-instance, not a long-running shim. Per FND-28.
8. **Cross-axis effects flow through events, not direct writes.** When `legal_effect = Fulfilled`, the actionability transition to `Closed` is its own subsequent event emitted by the lifecycle handler that processed fulfillment, not a synchronous cross-axis write. This preserves FND-26 (systems interact through state) and Non-Goal 3 below (no auto-derivation). Per-axis handlers within `artifact_lifecycle_system` run in a fixed order so the resulting event sequence is deterministic; the determinism comes from explicit handler ordering plus `BTreeMap`-stable iteration, not from cross-axis reads.

## Non-Goals

- **Per-artifact-class custom axes.** Five axes apply uniformly. Per-class custom state lives elsewhere (e.g., bounty reward fund stays on the bounty entity, not as an axis).
- **Multi-tenant credibility.** Credibility is the artifact's *aggregate* believed credibility per the issuing institution's ledger. Per-agent credibility belief lives in the per-agent belief envelope, not on the artifact.
- **Auto-derivation of one axis from another.** The five axes are independent. `legal_effect = Expired` does not auto-set `actionability = Closed`; the closer transition is its own emitted event handled by the lifecycle handler that processed the expiry.
- **Save-format compatibility.** Pre-S140 saves cannot be loaded by post-S140 binaries (FND-28). The save-format bump is real. (This is distinct from scenario-author defaults — see the Validation section.)
- **New event tag taxonomy.** A single `EventTag::ArtifactTransition` covers all per-axis transitions; the axis name is in the payload.
- **Sale listings as artifacts.** `SaleListing` is a separate component substrate and is not migrated to the artifact taxonomy by this spec.
- **Promoting accusations to artifacts.** Accusations live on the justice subsystem (`justice_actions.rs`). Whether they should be migrated onto `ArtifactHeader` is a future spec; S140 only restructures artifacts that already use `ArtifactHeader` (notices and bounties). The exonerated-accusation example in Design Goal 1 illustrates the *shape* the unified axes can express, not a deliverable.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | All five axes are typed enums, never numeric "lifecycle scores." |
| FND-18 (Memory, Evidence, and Records Are World State) | Artifacts retain inspectable history across axes — exonerated accusations, revoked warrants, fulfilled bounties remain inspectable as world state. |
| FND-25 (Social Artifacts Are First-Class) | Authoring through a single unified `ArtifactDef` reflects FND-25's "there are only world entities and records" framing — one taxonomy for the artifact category. |
| FND-25A (Artifact Lifecycle, Visibility, and Actionability Are Distinct) | Direct compliance — five axes are the canonical FND-25A test articulated as code. |
| FND-26 (Systems Interact Through State) | Cross-axis effects flow through emitted `ArtifactTransition` events processed by axis handlers in fixed order, not through synchronous cross-axis writes. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | `ArtifactState` is removed, not retained. The migration is per-instance, not a parallel-representation shim. `BelievedArtifactState` is migrated alongside. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Each axis transition is an append-only event with cause attribution. |

## Migration Map (Existing `ArtifactState` → Axis Combinations)

| Current `ArtifactState` | Existence | Visibility (preserved from prior posting) | Legal Effect | Credibility | Actionability |
|-------------------------|-----------|--------------------------------------------|--------------|-------------|---------------|
| `Active` | `Exists` | preserved | `Active { expires_at: header.expires_at }` | `Credible` | `Actionable` |
| `Fulfilled` | `Exists` | preserved | `Fulfilled { fulfilled_at, by, evidence }` | `Credible` | `Closed { cause: BountyFulfilled }` |
| `Expired` | `Exists` | preserved | `Expired { expired_at }` | `Credible` | `Closed { cause: LegalEffectExpired }` |
| `Withdrawn` | `Exists` | preserved | `Revoked { revoked_at, by, reason: IssuerWithdrawal }` | `Credible` | `Closed { cause: Revoked }` |
| `Destroyed` | `Destroyed { destroyed_at, cause }` | (irrelevant; terminal) | (irrelevant; terminal) | (irrelevant; terminal) | (irrelevant; terminal) |

The pre-existing `ArtifactHeader` fields `expires_at`, `issuing_authority`, and `jurisdiction` are **preserved** on the refactored header (they are heavily consumed by `artifact_actions.rs`, e.g., line 455 for `issuing_authority` and line 460 for `jurisdiction`). They are not subsumed into axes; the axes carry only the lifecycle dimensions FND-25A names. `created_at` is preserved (renamed to `issued_at` would constitute gratuitous rename — keep `created_at`). No new top-level `place` field is added; posting place is already carried by `ArtifactPostingContext`. No new `ArtifactId` type is introduced; artifacts continue to be identified by `EntityId`.

## Deliverables

### D1. `worldwake-core::social_artifact` refactor

Refactor `ArtifactHeader` to retain its existing identity/jurisdiction fields and replace the single `state: ArtifactState` field with five typed axis fields:

```rust
pub struct ArtifactHeader {
    pub kind: ArtifactKind,
    pub issuer: EntityId,
    pub issuing_authority: Option<EntityId>,
    pub created_at: Tick,
    pub expires_at: Option<Tick>,
    pub jurisdiction: Option<EntityId>,
    // S140: five orthogonal lifecycle axes replacing `state: ArtifactState`.
    pub existence: ArtifactExistence,
    pub visibility: ArtifactVisibility,
    pub legal_effect: ArtifactLegalEffect,
    pub credibility: ArtifactCredibility,
    pub actionability: ArtifactActionability,
}

pub enum ArtifactExistence {
    Exists,
    Destroyed { destroyed_at: Tick, cause: DestructionCause },
}

pub enum ArtifactVisibility {
    Hidden,
    Private { audience: BTreeSet<EntityId> },
    Posted { place: EntityId },
    WidelyKnown,
}

pub enum ArtifactLegalEffect {
    None,
    Active { expires_at: Option<Tick> },
    Suspended { reason: SuspensionReason, suspended_at: Tick },
    Expired { expired_at: Tick },
    Revoked { revoked_at: Tick, by: EntityId, reason: RevocationReason },
    Fulfilled { fulfilled_at: Tick, by: EntityId, evidence: EntityId },
}

pub enum ArtifactCredibility {
    Credible,
    Disputed { disputed_at: Tick, contradicting: BTreeSet<EntityId> },
    Refuted { refuted_at: Tick, evidence: EntityId },
    Unknown,
}

pub enum ArtifactActionability {
    Actionable,
    AwaitingProof { required_proof: ProofKind },
    Blocked { reason: BlockerReason, since: Tick },
    Closed { closed_at: Tick, cause: CloseCause },
}
```

`ArtifactState` is removed. The supporting payload enums are defined as `Copy`-compatible value types so `ArtifactHeader` retains its existing `Copy` derive — except for `ArtifactVisibility::Private` and `ArtifactCredibility::Disputed`, which carry `BTreeSet<EntityId>` and therefore force the parent enum (and `ArtifactHeader`) to drop `Copy` in favor of `Clone`. All consumers that today copy `ArtifactHeader` are migrated to clone it; the migration is part of this deliverable's scope.

`BTreeSet<EntityId>` (rather than `SmallVec<EntityId, N>`) is the chosen collection type. This satisfies the CLAUDE.md determinism invariant (`BTreeMap`/`BTreeSet` only in authoritative state), avoids adding `smallvec` to `worldwake-core`'s minimal dependency set (currently `serde, bincode, blake3`), and matches the existing pattern across the workspace.

`DestructionCause`, `SuspensionReason`, `RevocationReason`, `ProofKind`, `BlockerReason`, and `CloseCause` are defined as small `Copy` enums in the same module. Initial variants:

- `DestructionCause::{Adjudication, IssuerDestroyed, Superseded, Decay}`
- `SuspensionReason::{JurisdictionDispute, EvidenceWithheld, ProcessReview}`
- `RevocationReason::{IssuerWithdrawal, Adjudication, SupersededByLater}`
- `ProofKind::{PhysicalEvidence, WitnessTestimony, SelfReport}` (mirrors existing `ProofRequirement` semantics; `ProofRequirement` is renamed to `ProofKind` if and only if the rename is mechanically clean across all call sites — otherwise the new `ProofKind` is the actionability-axis type and `ProofRequirement` remains the bounty-terms type, with a documented note about why two near-identical enums coexist)
- `BlockerReason::{LegalEffectExpired, LegalEffectRevoked, JurisdictionConflict, AwaitingAdjudication, BountyFulfilled, Adjudicated, Refuted}`
- `CloseCause::{BountyFulfilled, LegalEffectExpired, Revoked, Adjudicated, Refuted}`

### D2. Per-axis transition events

Add `EventTag::ArtifactTransition` with payload:

```rust
pub enum AxisName {
    Existence, Visibility, LegalEffect, Credibility, Actionability,
}

pub enum ArtifactAxisValue {
    Existence(ArtifactExistence),
    Visibility(ArtifactVisibility),
    LegalEffect(ArtifactLegalEffect),
    Credibility(ArtifactCredibility),
    Actionability(ArtifactActionability),
}

pub struct ArtifactTransitionPayload {
    pub artifact: EntityId,
    pub axis: AxisName,
    pub prior: ArtifactAxisValue,
    pub new: ArtifactAxisValue,
    pub cause_event: Option<EventId>,
    pub at: Tick,
}
```

A single `EventTag::ArtifactTransition` covers all five axes. Decoding inspects the `axis` field. The payload's clone cost is acceptable because transitions are infrequent (per-artifact, not per-tick).

### D3. `artifact_lifecycle_system` refactor

`crates/worldwake-systems/src/artifact_lifecycle.rs` (currently lines 8-62) is extended with per-axis transition handlers running in a fixed order:

1. `existence` (handles `Destroyed` terminal state — short-circuits the rest).
2. `legal_effect` (handles `Expired` ← TTL, `Revoked` ← withdrawal/revocation transition events, `Fulfilled` ← bounty-fulfillment events from `commit_claim_bounty` at `artifact_actions.rs:1434`, and source-event-backed `Suspended` / restoration from `InstitutionalClaim::ForceControl { contested }` record events wired by S140ARTLIFAXE-007).
3. `credibility` (S140ARTLIFAXE-008 wires source-event-backed `Refuted` transitions from artifact-addressed `InstitutionalClaim::ArtifactCredibilityRefutation` record events. Source-event-backed `Disputed` ← contradicting-testimony events and the full S63 case/alibi/exoneration workflow remain future-owned outside this bounded carrier.)
4. `visibility` (handles `Posted` ← post events, `Hidden` ← unstaging events, `WidelyKnown` ← rumor-saturation events).
5. `actionability` (handles `Closed` ← downstream consequence of legal_effect or credibility transitions emitted by stages 2-3 in this same tick, `Blocked` ← jurisdiction-conflict events, `AwaitingProof` ← proof-pending events).

Cross-axis effects flow through emitted events read by later handlers in the same tick (FND-26). The handler-order ensures fulfillment and expiry events are observed by the actionability handler in the same tick, so `Fulfilled → Closed` is a deterministic sequence even though it is two separate transitions. The current TTL expiry path (line 43, `header.state = ArtifactState::Expired`) becomes the legal-effect handler's `Active → Expired` write; the existing treasury-encumbrance release at line 56 is preserved.

### D4. Workspace-wide `ArtifactState` migration

Every reference to `ArtifactState` (~125 sites across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, `worldwake-cli`, plus test fixtures and goldens — see `crates/worldwake-systems/src/artifact_lifecycle.rs`, `crates/worldwake-systems/src/artifact_actions.rs`, `crates/worldwake-systems/src/perception.rs`, `crates/worldwake-core/src/belief.rs`, `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/exhaustion.rs`, `crates/worldwake-ai/src/goal_model.rs`, `crates/worldwake-ai/src/ranking.rs`, `crates/worldwake-ai/src/route_threat.rs`, `crates/worldwake-cli/src/scenario/mod.rs:983,1751`, plus `golden_offices.rs:2249` and `golden_survival_justice.rs:329`) is migrated to read from the appropriate axis. Equality checks of the form `header.state == ArtifactState::Active` become `header.actionability == ArtifactActionability::Actionable` (or whichever axis is semantically correct for that site, per the Migration Map). State-mutating writes become per-axis transition emissions. The migration is not a long-running shim per FND-28; either the call site is rewritten in this deliverable or it is deleted.

This deliverable may be split across multiple tickets along crate boundaries (one ticket per crate) by `/spec-to-tickets`; the deliverable's scope is the full set, not a partial pass.

### D5. `BelievedArtifactState` belief-snapshot migration

`BelievedArtifactState` (carried by `EntityBeliefClaim::ArtifactState` at `crates/worldwake-core/src/entity_belief_claim.rs:42`, populated by perception in `crates/worldwake-systems/src/perception.rs` at lines 855, 6293, 6342, 6383, 6474 and in `crates/worldwake-ai/src/route_threat.rs`, `crates/worldwake-systems/src/artifact_actions.rs:3247`, and `crates/worldwake-ai/src/candidate_generation.rs:7905`) is migrated to a per-axis representation. The default migration projects the five axes into `BelievedArtifactState` so per-agent belief retains lifecycle fidelity. Concretely, `BelievedArtifactState` mirrors the public axis fields of `ArtifactHeader` — agents who observed the artifact carry the same five axis values they could have read directly. Subsequent transitions update the believed copy through perception of `ArtifactTransition` events; FND-15 provenance metadata (acquisition tick, source) is attached per transition observation. The variant `EntityBeliefClaim::ArtifactState(Option<BelievedArtifactState>)` is renamed to `EntityBeliefClaim::Artifact(Option<BelievedArtifactState>)` to avoid the dead `ArtifactState` symbol surviving in the variant name.

### D6. Planner gate scope

In `crates/worldwake-ai/src/candidate_generation.rs`, the single existing artifact-state gate at line 650 (today: `artifact.kind != Bounty || artifact.state != ArtifactState::Active`) is rewritten as `artifact.kind != Bounty || artifact.actionability != ArtifactActionability::Actionable`. The change is a direct substitution, not a generalization. Goal kinds that did not previously filter on `ArtifactState` (`Accuse`, `PunishAccused`, `PostBounty`, `PostNotice`) continue not to filter — broadening the gate is a separate, scoped decision and is out of S140's scope.

When the gate rejects a candidate, emission records `Discrepancy::ArtifactNotActionable { artifact: EntityId, reason: BlockerReason }` through the side-effect-free candidate-generation pending-record path and read-phase `DiscrepancyMemory` persistence. `BlockerReason` is `Copy` so `Discrepancy`'s `Copy` derive is preserved. The decision-trace surface in `crates/worldwake-ai/src/decision_trace.rs` is extended to render the five axis values for any artifact referenced by a ranked candidate so the rejection cause is locally inspectable.

### D7. Scenario authoring — unified `ArtifactDef`

`NoticeDef` (`crates/worldwake-cli/src/scenario/types.rs:144-154`) is renamed to `ArtifactDef` and extended with optional axis-state fields and a payload sum type that preserves type safety across artifact classes:

```rust
pub struct ArtifactDef {
    pub kind: ArtifactKindDef, // discriminator
    pub issuer: String,
    pub location: String,
    pub issuing_authority: Option<String>,
    pub expires_at: Option<u64>,
    pub jurisdiction: Option<String>,
    pub payload: ArtifactPayloadDef,
    // Per-axis initial state. Defaults reflect the historical `ArtifactState::Active` shape
    // (`Exists | Posted | Active | Credible | Actionable`), so existing scenarios do not need
    // to declare axis state.
    pub existence: Option<ArtifactExistenceDef>,
    pub visibility: Option<ArtifactVisibilityDef>,
    pub legal_effect: Option<ArtifactLegalEffectDef>,
    pub credibility: Option<ArtifactCredibilityDef>,
    pub actionability: Option<ArtifactActionabilityDef>,
}

pub enum ArtifactPayloadDef {
    Notice(NoticeTopicDef),
    Bounty(BountyTermsDef),
    // future: Accusation(AccusationDef) when accusations are landed as artifacts
}
```

`ScenarioDef.notices: Vec<NoticeDef>` (`scenario/types.rs:39`) is renamed to `ScenarioDef.artifacts: Vec<ArtifactDef>`. Zero current `.ron` scenarios author this field today (verified by `grep "notices:" scenarios/*.ron` returning no matches), so RON migration cost is restricted to scenario tests in `crates/worldwake-cli/src/scenario/mod.rs` and the `test_spawn_notice_artifact_from_scenario` test at line 1687. `spawn_notice` (lines 955-987) is renamed to `spawn_artifact` and dispatches by `ArtifactKindDef` to construct the appropriate kind-specific terms component (`BountyTerms` for bounties, `NoticeContent` for notices) alongside the unified `ArtifactHeader`. Per FND-28, no back-compat alias is retained at the engine layer; the scenario-authoring boundary preserves "if you author the historical shape, you get the historical behavior" through optional-field defaults, which is a normalization at the boundary (FND-13), not a parallel representation.

### D8. Observer Section 11 — Artifact Lifecycle

Add a new section to `crates/worldwake-cli/src/bin/observer.rs` after the existing Section 10 ("Critical Window Forensics") optional emission block. Sections 1-10 are currently in use; Section 11 is the appropriate identifier. The new section iterates artifacts referenced in the run and renders per-axis state plus an axis-transition timeline:

```
## Section 11 — Artifact Lifecycle

Bounty B7 (issued tick 100, by office Watch, place TownSq)
  existence: Exists
  visibility: Posted (since t=102)
  legal_effect: Fulfilled (t=480, by Hunter Theron, evidence Wolf-Pelt)
  credibility: Credible
  actionability: Closed (t=480, cause: BountyFulfilled)
  axis history: 8 transitions
```

The section header text and section number are committed to. Section 5 ("Raw Event Sample") and other existing sections are not renumbered.

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis (FND-30 #4).** Per-axis transitions emit through the existing event-log surface as `EventTag::ArtifactTransition`. Agents perceive transitions through the same posting/rumor/observation channels they perceive the original artifact (FND-7). No new cross-agent path is introduced.
2. **Positive-feedback analysis (FND-30 #8).** No amplifying loop. Each axis has a finite state set; transitions are deterministic functions of explicit world events.
3. **Concrete dampeners (FND-30 #9).** Not applicable — no amplification.
4. **Stored state vs derived read-model list (FND-30 #15).**
   - **Stored authoritative state**: `ArtifactHeader` with five typed axis fields, per-axis `ArtifactTransition` events in the event log, `BelievedArtifactState` per-agent belief mirror.
   - **Derived read-model**: per-axis "current value" is read directly from `ArtifactHeader`; the axis-history view is a per-tick decoded scan over the event log (cached at observer-render time).
5. **Quantities, source/sink (FND-30 #5).** No new quantities are introduced. Treasury reward release on the `legal_effect: Fulfilled` transition continues to use the existing `release_bounty_reward` path (`artifact_actions.rs:1494`); reservation release on `Revoked` or `Expired` likewise uses the existing path. No source/sink accounting is reorganized.
6. **Provenance, freshness, and source-chain markers (FND-30 #11).** Each `ArtifactTransition` event carries `cause_event: Option<EventId>` linking it to the proximate world event (e.g., a `commit_claim_bounty` action commit). Belief observations of transitions inherit FND-15 provenance metadata (acquisition tick, source). Credibility transitions specifically carry the contradicting/refuting evidence entity, so an exonerating chain is reconstructable from the event log alone.
7. **Lifecycle states and transitions (FND-30 #12).** Direct compliance — this is the spec's central concern. Each axis enumerates its states and transitions, and the lifecycle handlers in D3 enumerate the events that move them.
8. **Temporal resolution and tie-breaking (FND-30 #13).** Per-axis transition handlers run in a fixed order within `artifact_lifecycle_system` (existence, legal_effect, credibility, visibility, actionability). Within an axis, ties are broken by `BTreeMap`-stable iteration over transition events. This makes the per-tick transition sequence deterministic across replays.
9. **Causal records and event identities (FND-30 #16).** `EventTag::ArtifactTransition` events carry `(artifact, axis, prior, new, cause_event, at)` so post-hoc inspection can reconstruct both the causal path (cause_event chain) and the knowledge path (perception observations of the transition). FND-29A append-only history is preserved.
10. **Save/load and replay (FND-30 #18).** `SAVE_FORMAT_VERSION` increments from 70 across S140; S140ARTLIFAXE-002 further bumps the live format to 72 when `ArtifactTransitionPayload` becomes persisted event-log payload data, S140ARTLIFAXE-003 bumps the live format to 73 when `Discrepancy::ArtifactNotActionable` and closed-cause `BlockerReason` variants become persisted, and S140ARTLIFAXE-008 bumps the live format to 74 when the artifact-addressed `InstitutionalClaim::ArtifactCredibilityRefutation` record carrier becomes persisted. `BelievedArtifactState` is migrated alongside `ArtifactState`. No save-format shim is provided (FND-28); pre-S140 saves cannot be loaded by post-S140 binaries. Replay determinism depends on the per-axis handler ordering documented in #8.

Declarations 1, 2, 3, 6, 7, 10, 14, 17 are addressed implicitly by the spec body or are not relevant to this system extension (S140 introduces no new actions, no new scarce capacities, no new boundary processes, no new agent-local learning, and no new validation falsifiers beyond those already enumerated in the Validation section).

## SystemFn Integration

`artifact_lifecycle_system` (existing) is extended with per-axis transition handlers. No new `SystemFn`; the existing system function's body grows.

## Component Registration

- `ArtifactHeader` (refactored) — already registered. Schema migration handled at the save-format bump.
- No new components.

## Cross-System Interactions

- **Sim ↔ Sim internal**: per-axis transition handlers read events from prior axis-handler stages within the same tick (e.g., the actionability handler reads `legal_effect: Fulfilled` events emitted earlier in the tick by the legal-effect handler). All reads go through the event log, not direct cross-handler calls.
- **Sim → AI**: planner reads `actionability` directly from `ArtifactHeader` for the migrated gate at `candidate_generation.rs:650`. `Discrepancy::ArtifactNotActionable` flows through candidate-generation pending records, read-phase `DiscrepancyMemory` persistence, and the existing decision-trace surface.
- **Sim → CLI**: observer reads `ArtifactHeader` and decoded `ArtifactTransition` events for Section 11.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

Not applicable — artifact lifecycle is per-artifact-class state, not per-agent. Per-issuer policies (which institution suspends vs revokes a warrant) live on the issuer agent's existing components (e.g., institutional procedure components from existing institutional code). `ArtifactPostingProfile` (S97) continues to govern the `Active → Expired` legal-effect transition.

## Validation and Falsification

- **Golden coverage**: new `golden_artifact_lifecycle.rs` with five scenarios:
  1. Bounty fulfilled → expects `legal_effect: Active → Fulfilled`, `actionability: Actionable → Closed`, treasury reward release (S125), all in correct order.
  2. Warrant revoked → expects `legal_effect: Active → Revoked`, `actionability: Actionable → Closed`, planner refusal of candidates anchored on it via `Discrepancy::ArtifactNotActionable`.
  3. Expired-but-still-posted bounty → expects `legal_effect: Expired`, `visibility: Posted` retained, `actionability: Closed`.
  4. Suspended legal effect under jurisdiction conflict → expects `legal_effect: Active → Suspended`, restoration on resolution. S140ARTLIFAXE-007 wires this branch to source `InstitutionalClaim::ForceControl { contested }` record events and updates the golden fixture to prove the source-backed path.
  5. False rumor refuted via contradicting evidence → expects `credibility: Credible → Refuted`, `actionability: Actionable → Closed`, `existence: Exists` retained for audit. S140ARTLIFAXE-008 replaces the explicit transition fixture with a source-event-backed path through artifact-addressed `InstitutionalClaim::ArtifactCredibilityRefutation` record events; full S63 case/evidence adjudication remains outside this bounded carrier.
- **Scenario-author migration parity**: every committed `.ron` scenario produces identical run-time behavior post-S140. Today, no scenario authors `notices:`, so this check reduces to confirming the renamed `artifacts:` field's defaults reproduce the historical `ArtifactState::Active` shape (`Exists | Posted | Active | Credible | Actionable`) when no axis fields are declared. This is a boundary normalization (FND-13), not engine-layer back-compat.
- **Save-state migration**: distinct from scenario-author parity. Pre-S140 saves are explicitly not loadable post-S140; the `SAVE_FORMAT_VERSION` bump from 70, the later live bump to 72 for persisted transition payloads, the live bump to 73 for the persisted artifact-actionability discrepancy shape, and the live bump to 74 for the persisted artifact-credibility-refutation record carrier are verified by the existing version-check path (`crates/worldwake-sim/src/save_load.rs`).
- **No-shim regression**: a grep guard asserts that the exact symbol `ArtifactState` appears nowhere in `crates/` post-S140 (excluding `BelievedArtifactState` which is renamed via D5 and carries no surviving `ArtifactState` substring after the variant rename). The guard greps the exact word boundary `\bArtifactState\b`.

## Risks

- **Migration breadth.** The 125-site `ArtifactState` reference set spans every layer of the workspace plus tests. Mitigation: D4 commits to the migration as scope; `/spec-to-tickets` is expected to split D4 across crate-bounded tickets so each can be implemented and reviewed independently.
- **Save-format break.** `SAVE_FORMAT_VERSION` increments from 70 across S140 and reaches 74 once transition payloads, artifact-actionability discrepancy records, and the artifact-credibility-refutation record carrier are persisted; no shim. Mitigation: any committed save fixtures that need to round-trip get fresh-generation tickets alongside the migration.
- **Per-axis handler ordering.** Cross-axis effects must be deterministic across replays. Mitigation: the five-stage ordering above is fixed and tested via golden 1; tie-breaking within an axis is by `BTreeMap`-stable iteration over transition events.
- **`Copy` removal on `ArtifactHeader`.** `BTreeSet<EntityId>` payload on `Private` and `Disputed` forces dropping `ArtifactHeader`'s `Copy` derive. Mitigation: D1 includes the consumer-migration audit; the existing 1:1 ratio of struct copies to clones is small enough to migrate inline.
- **Scenario rename ergonomics.** Renaming `notices:` to `artifacts:` plus `NoticeDef` to `ArtifactDef` will affect any downstream scenario tooling. Mitigation: zero `.ron` files use the field today, so the rename is an engine-side rename without scenario-authoring fallout.

## Outcome

Completed on 2026-05-06.

- Implemented across archived tickets `S140ARTLIFAXE-001` through `S140ARTLIFAXE-008`.
- Replaced the flat `ArtifactState` model with five typed artifact lifecycle axes on `ArtifactHeader`, migrated belief/read surfaces, scenario authoring, observer rendering, planner actionability gating, transition payload persistence, and generated golden documentation.
- Added append-only `ArtifactTransition` provenance with deterministic lifecycle-stage ordering and source-backed legal-effect suspension/restoration through `InstitutionalClaim::ForceControl { contested }` record events.
- Added source-backed credibility refutation through the bounded artifact-addressed `InstitutionalClaim::ArtifactCredibilityRefutation { artifact, evidence, effective_tick }` record carrier, then moved Scenario 392 from explicit transition injection to that source-backed path.
- Bumped `SAVE_FORMAT_VERSION` through the S140 chain to 74 for the persisted artifact lifecycle, discrepancy, transition-payload, and institutional-claim shapes.

Deviations:

- Accusations were not promoted to `ArtifactHeader`; S140 remains scoped to artifacts already using the social-artifact substrate.
- The full S63 case/alibi/exoneration workflow and source-event-backed `Disputed` credibility branch remain outside this completed S140 slice. The landed refutation path is intentionally the minimal artifact-addressed institutional record carrier required for lifecycle provenance.
- FOUNDATIONS Scenario G is not claimed as an end-to-end justice/witness chain by S140 alone; S140 supplies the artifact lifecycle axis substrate and source-backed refutation seam.

Verification results:

- `cargo test -p worldwake-core --lib institutional`
- `cargo test -p worldwake-systems --lib artifact_lifecycle`
- `cargo test -p worldwake-ai --test golden_artifact_lifecycle`
- `cargo test -p worldwake-sim --lib save_load`
- `cargo test -p worldwake-sim --lib institutional_knowledge_trace`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `./scripts/verify.sh`
