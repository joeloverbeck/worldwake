# S140: Multi-Axis Artifact Lifecycle

**Status**: Draft

## Summary

`ArtifactHeader` (`crates/worldwake-core/src/social_artifact.rs:5`) and `ArtifactState` (`social_artifact.rs:55`) currently model artifacts (notices, bounties, accusations, sale listings) as `Active | Expired` with a single TTL governed by `ArtifactPostingProfile` (S97). FND-25A names five orthogonal axes — existence, visibility, legal effect, credibility, actionability — and warns against collapsing them into a single boolean. Today's collapsed shape forces problematic compromises:

- An expired bounty stops being visible the same tick its legal authorization to issue rewards lapses, even though it should remain in institutional memory as a closed case.
- A revoked warrant, exonerated accusation, or fulfilled contract has no representation distinct from "expired" — yet each carries different downstream consequences for inspectability, future references, and historical integrity.
- A rumor's credibility (Disputed, Refuted, Unknown) is not separable from its existence — a refuted rumor must still be inspectable as part of FOUNDATIONS Scenario G's exoneration chain.
- Posted notices have no separate "actionable" axis — the planner cannot tell "this notice exists and is visible but the underlying claim has been adjudicated and is no longer actionable."

S140 separates the five axes into distinct typed components. Existence becomes `ArtifactExistence::{Exists, Destroyed}`. Visibility becomes `ArtifactVisibility::{Hidden, Private, Posted, WidelyKnown}`. Legal effect becomes `ArtifactLegalEffect::{None, Active, Suspended, Expired, Revoked, Fulfilled}`. Credibility becomes `ArtifactCredibility::{Credible, Disputed, Refuted, Unknown}`. Actionability becomes `ArtifactActionability::{Actionable, AwaitingProof, Blocked, Closed}`. Each axis has explicit lawful transitions through declared world processes (expiry, fulfillment, revocation, supersession, adjudication) and per-axis observers. The existing `ArtifactPostingProfile` TTL becomes one specific transition (legal_effect: Active → Expired); other transitions emerge from explicit causal events.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — refactors `ArtifactHeader` to embed five typed axis fields. Adds `ArtifactExistence`, `ArtifactVisibility`, `ArtifactLegalEffect`, `ArtifactCredibility`, `ArtifactActionability` enums. `ArtifactState` (the current single-enum) becomes a deprecated alias removed in the same change per FND-28. `SAVE_FORMAT_VERSION` increments.
- `worldwake-systems` — `artifact_lifecycle.rs` extended with per-axis transition handlers. `crates/worldwake-systems/src/post_bounty_actions.rs`, `post_notice_actions.rs`, `accusation_actions.rs`, sale-listing handlers, and bounty-fulfillment paths emit per-axis transitions rather than the single `ArtifactState::Active → Expired` step.
- `worldwake-ai` — planner reads `actionability` not `existence` when evaluating "can I act on this artifact?" A revoked warrant still exists and is visible but is non-actionable. Decision-trace surfaces axis values for artifacts referenced by ranked candidates.
- `worldwake-cli` — observer Section 5 (Artifacts) renders per-axis state. Scenario `ArtifactDef` extended for authoring per-axis initial state.

## Dependencies

- S97 (Post Notice Artifact TTL) — completed. `ArtifactPostingProfile` continues to govern the `Active → Expired` legal-effect transition. S140 layers the other axes atop.
- S125 (Institutional Treasuries and Bounty Funding) — completed. Bounty fulfillment becomes the `legal_effect: Active → Fulfilled` transition with reward release through the existing treasury path.
- S110 (Decision History Events) — completed. Per-axis transitions emit through the existing event-log surface.
- S109 (Typed Discrepancy Taxonomy) — completed. `Discrepancy::ArtifactNotActionable` joins the taxonomy for planner-side filtering.

## Design Goals

1. **Five orthogonal axes.** Existence, visibility, legal effect, credibility, actionability vary independently. A revoked warrant: `Exists | Posted | Revoked | Credible | Closed`. An exonerated accusation: `Exists | Posted | None | Refuted | Closed`. An expired-but-still-posted bounty: `Exists | Posted | Expired | Credible | Closed`.
2. **Lawful transitions only.** Each axis has a typed `*Transition` enum naming the world events that move it. No axis can be mutated except through a declared transition.
3. **Per-axis history.** Every transition emits an `EventTag::ArtifactTransition` with the axis name, prior value, new value, cause-event reference. History is append-only (FND-29A).
4. **Visibility ≠ existence.** A hidden artifact still exists; a destroyed artifact has no remaining axes. `Destroyed` is terminal across all axes; the artifact's record persists in event-log history but the live artifact entity is dropped.
5. **Actionability is the planner's gate.** `actionability == Actionable` is the precondition for goal candidates anchored on the artifact. Other axes inform belief and ranking but do not gate planner action.
6. **Credibility decays through evidence.** Credibility transitions through `evidence_against`, `evidence_for`, `contradicting_testimony`, `confirmed_by_witness` — all observed event types. No silent decay.
7. **Backward-compat-free migration.** `ArtifactState` (single-enum) is removed in the same change. The old `Active | Expired` semantics map to specific axis combinations; the migration path is per-instance, not a long-running shim. Per FND-28.
8. **Determinism.** Per-axis transition handlers run in a fixed order within `artifact_lifecycle_system`. Cross-axis interactions (e.g., `legal_effect = Fulfilled` implying `actionability = Closed`) are computed via deterministic dispatch.

## Non-Goals

- **Per-artifact-class custom axes.** Five axes apply uniformly. Per-class custom state lives elsewhere (e.g., bounty reward fund stays on the bounty entity, not as an axis).
- **Multi-tenant credibility.** Credibility is the artifact's *aggregate* believed credibility per the issuing institution's ledger. Per-agent credibility belief lives in the per-agent belief envelope (S113), not on the artifact.
- **Auto-derivation of one axis from another.** The five axes are independent. `legal_effect = Expired` does not auto-set `actionability = Closed`; the closer transition is its own event.
- **Save-format compatibility.** Pre-S140 saves cannot be loaded by post-S140 binaries (FND-28). The save-format bump is real.
- **New event tag taxonomy.** A single `EventTag::ArtifactTransition` covers all per-axis transitions; the axis name is in the payload.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | All five axes are typed enums, never numeric "lifecycle scores." |
| FND-18 (Memory, Evidence, and Records Are World State) | Artifacts retain inspectable history across axes — exonerated accusations, revoked warrants, fulfilled bounties remain inspectable as world state. |
| FND-25 (Social Artifacts Are First-Class) | The shape S140 establishes is what FND-25 requires: artifacts as world entities with rich, multi-faceted state, not collapsed booleans. |
| FND-25A (Artifact Lifecycle, Visibility, and Actionability Are Distinct) | Direct compliance — five axes are the canonical FND-25A test articulated as code. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | `ArtifactState` is removed, not retained. The migration is per-instance, not a parallel-representation shim. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Each axis transition is an append-only event with cause attribution. |

## Deliverables

### `worldwake-core::social_artifact` refactor

```rust
pub struct ArtifactHeader {
    pub artifact_id: ArtifactId,
    pub kind: ArtifactKind,
    pub issued_at: Tick,
    pub issuer: EntityId,
    pub place: EntityId,
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
    Private { audience: SmallVec<EntityId, 4> },
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
    Disputed { disputed_at: Tick, contradicting: SmallVec<EntityId, 2> },
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

`ArtifactState` (the prior single-enum) is removed.

### Per-axis transition events

```rust
pub enum AxisName {
    Existence, Visibility, LegalEffect, Credibility, Actionability,
}

pub struct ArtifactTransitionPayload {
    pub artifact: ArtifactId,
    pub axis: AxisName,
    pub prior: ArtifactAxisValue,
    pub new: ArtifactAxisValue,
    pub cause_event: Option<EventId>,
    pub at: Tick,
}
```

Single `EventTag::ArtifactTransition` covers all five axes. Decoding inspects the axis name in the payload.

### `artifact_lifecycle_system` refactor

Per-axis transition handlers run in a fixed order:
1. `existence` (handles `Destroyed` terminal state — short-circuits the rest).
2. `legal_effect` (handles `Expired`, `Suspended`, `Revoked`, `Fulfilled` per the existing TTL/event paths).
3. `credibility` (handles `Disputed` ← contradicting testimony events, `Refuted` ← evidence-against events).
4. `visibility` (handles `Posted` ← post events, `Hidden` ← unstaging events, `WidelyKnown` ← rumor-saturation events).
5. `actionability` (handles `Closed` ← adjudication events, `Blocked` ← jurisdiction events, `AwaitingProof` ← proof-pending events).

Cross-axis effects flow through events, not through direct cross-axis writes (FND-26).

### Planner integration

`crates/worldwake-ai/src/candidate_generation.rs` checks `actionability == Actionable` when emitting candidates anchored on artifacts. The new `Discrepancy::ArtifactNotActionable { artifact, reason }` surfaces blocked candidates.

### Scenario authoring

`ArtifactDef` extends with five optional axis fields; defaults supply the historical `Active` shape for back-compat at the scenario-author layer. Per FND-28, no back-compat at the engine layer.

```rust
pub struct ArtifactDef {
    // existing fields
    pub existence: Option<ArtifactExistenceDef>,
    pub visibility: Option<ArtifactVisibilityDef>,
    pub legal_effect: Option<ArtifactLegalEffectDef>,
    pub credibility: Option<ArtifactCredibilityDef>,
    pub actionability: Option<ArtifactActionabilityDef>,
}
```

### Observer Section 5 (Artifacts)

Render per-axis state for every artifact referenced in the run, with axis-transition timeline:
```
Bounty B7 (issued tick 100, by office Watch, place TownSq)
  existence: Exists
  visibility: Posted (since t=102)
  legal_effect: Fulfilled (t=480, by Hunter Theron, evidence Wolf-Pelt)
  credibility: Credible
  actionability: Closed (t=480, cause: BountyFulfilled)
  axis history: 8 transitions
```

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Per-axis transitions emit through the existing event-log surface. Agents perceive transitions through the same posting/rumor/observation channels they perceive the original artifact (FND-7). No new cross-agent path.
2. **Positive-feedback analysis.** No amplifying loop. Each axis has a finite state set; transitions are deterministic functions of explicit world events.
3. **Concrete dampeners.** Not applicable — no amplification.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: `ArtifactHeader` with five typed axis fields, per-axis transition events in the event log.
   - **Derived read-model**: per-axis "current value" is read directly from `ArtifactHeader`; the axis-history view is a per-tick decoded scan over the event log (cached at observer-render time).

## SystemFn Integration

`artifact_lifecycle_system` (existing) is extended with per-axis transition handlers. No new `SystemFn`; the existing one's body grows.

## Component Registration

- `ArtifactHeader` (refactored) — already registered. Schema migration handled at the save-format bump.
- No new components.

## Cross-System Interactions

- **Sim ↔ Sim internal**: per-axis transition handlers read events from prior systems' output (e.g., adjudication events from `accusation_actions`).
- **Sim → AI**: planner reads `actionability` directly. `Discrepancy::ArtifactNotActionable` flows through the existing decision-trace surface.
- **Sim → CLI**: observer reads `ArtifactHeader` and `ArtifactTransition` events.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

Not applicable — artifact lifecycle is per-artifact-class state, not per-agent. Per-issuer policies (which institution suspends vs revokes a warrant) live on the issuer agent's existing components (e.g., `OfficeProcedureProfile` from existing institutional code).

## Validation and Falsification

- **Golden coverage**: new `golden_artifact_lifecycle.rs` with six scenarios:
  1. Bounty fulfilled → expects `legal_effect: Active → Fulfilled`, `actionability: Actionable → Closed`, treasury reward release (S125), all in correct order.
  2. Warrant revoked → expects `legal_effect: Active → Revoked`, `actionability: Actionable → Closed`, planner refusal of candidates anchored on it.
  3. False accusation exonerated → expects `credibility: Credible → Refuted`, `actionability: Actionable → Closed`, `existence: Exists` retained for audit.
  4. Expired-but-still-posted bounty → expects `legal_effect: Expired`, `visibility: Posted` retained, `actionability: Closed`.
  5. Suspended warrant under jurisdiction conflict → expects `legal_effect: Active → Suspended`, restoration on resolution.
  6. FOUNDATIONS Scenario G chain (false rumor → wrongful accusation → contested evidence → exoneration) end-to-end across S140's artifact axes.
- **Migration parity**: every pre-S140 committed scenario produces identical run-time behavior post-S140 (the default axis values for prior `ArtifactState::Active` map to `Exists | Posted | Active | Credible | Actionable`).
- **No-shim regression**: a grep guard asserts `ArtifactState` is removed from the codebase post-S140.

## Risks

- **Save-format break.** `SAVE_FORMAT_VERSION` increments; no shim. Mitigation: ticket-001 lands the migration logic for any committed save fixtures that need to round-trip.
- **Per-axis handler ordering.** Cross-axis effects must be deterministic. Mitigation: the five-stage ordering above is fixed and tested via golden 1; tie-breaking within an axis is by `BTreeMap`-stable iteration over transition events.
- **Author overhead.** Scenarios with rich artifact state will need per-axis declarations. Mitigation: scenario-author defaults preserve the historical `Active` shape; explicit axis state is opt-in for scenarios that exercise non-default lifecycle.
