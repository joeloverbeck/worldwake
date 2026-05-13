# S139: Ask-Witness Goal Layer

**Status**: Draft

## Summary

Worldwake's belief architecture treats provenance, freshness, contradiction, and refutation as first-class (S113 belief envelope, S122 frame assumptions, S109 typed discrepancies). The action layer for asking a witness is already shipped — `crates/worldwake-systems/src/epistemic_actions.rs` registers `ask_witness` with a full `AskWitnessEffectSink` that imports the witness's beliefs into the actor's belief store carrying `PerceptionSource::Report { from: witness, chain_len: 1 }` provenance, and `crates/worldwake-sim/src/action_payload.rs:364` defines `AskWitnessPayload { target, topic_entity, topic_commodity }`. What's missing is the agent-level *intent to learn*: today the action can fire only as part of a plan whose goal already names it; there is no first-class `GoalKind` an agent can adopt that says "I will go ask the captain whether the caravan returned." The repair search (S137 `RepairKind::InsertVerification`) cannot splice an ask step before a guard that depends on a low-confidence belief because there is no goal-layer surface to attach it to.

S139 lands `GoalKind::AskWitness { witness, topic }` as a discrete `GoalKind` variant covering FOUNDATIONS Scenario G — false-rumor → wrongful-accusation chains require agents to question witnesses about prior testimony. The goal has a satisfaction predicate over the agent's belief envelope (the agent now holds a fresh belief about the topic sourced from this witness with confidence ≥ the per-agent `stale_evidence_barrier_threshold`), a candidate-generation pass that anchors on belief entries whose `PerceptionSource::Report` provenance names a co-located witness, and a `build_payload_override` that maps the goal's `TellTopic` to the existing `AskWitnessPayload` split-field shape. The existing universal `EpistemicDispositionProfile` (`crates/worldwake-core/src/epistemic.rs:23`) is the per-agent profile substrate — already seeded by `world.rs::create_agent`, already wired through belief-view accessors, already consumed by `ask_witness` and `ask_about_person` action handlers. S139 extends it with one new field (`witness_recency_preference`) rather than introducing a second epistemic-disposition component (FND-28 single-source-of-truth, spec-drafting-rules section 5f semantic-overlap discipline). No new action infrastructure is introduced — the existing `ask_witness` registration absorbs all action-layer work.

`InspectContainer`, `VerifyBelief`, `ConsultRecord`, and `ScoutPlace` (originally bundled in the assessor's proposal) are out of scope. `InspectContainer` specifically requires a new perception event tag, a new payload, a new effect step, and a "believed container access right" belief surface (FND-24 ownership/custody/access distinction); that substrate belongs in a future sibling spec.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `GoalKind` at `crates/worldwake-core/src/goal.rs:62` with `AskWitness { witness: EntityId, topic: TellTopic }`. Extends the existing universal `EpistemicDispositionProfile` at `crates/worldwake-core/src/epistemic.rs:23` with one new field `witness_recency_preference: Permille` (weighting per-tick freshness vs first-hand-distance), `#[serde(default)]`-annotated so existing RON scenarios deserialize unchanged. Reuses `stale_evidence_barrier_threshold` (already consumed by `crates/worldwake-ai/src/goal_model.rs:148` as a confidence/staleness gate) as the verification threshold; reuses `ask_memory_retention_ticks` (already consumed by `epistemic_actions.rs:466` and `ask_about_person_actions.rs:518` to gate re-ask) as the witness inquiry cooldown.
- `worldwake-sim` — extends the `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:270` with `entity_beliefs_sourced_from_witness(agent, witness)` — a new accessor that surfaces `BelievedEntityState` entries whose `source = PerceptionSource::Report { from: witness, .. }`. The existing `epistemic_disposition_profile(actor)` accessor at `belief_view.rs:548` (also exposed via `SocialBeliefView::epistemic_disposition_profile` at line 1171 and the `RuntimeBeliefView` impl at line 2020) is reused unchanged. Provides the `impl_goal_belief_view!` forwarding for the one new method.
- `worldwake-ai` — extends `GoalDispatchKey` at `crates/worldwake-ai/src/goal_dispatch_key.rs:6` with `AskWitness` (bumps `ALL` from `[Self; 40]` to `[Self; 41]` at line 50 and adds the `from_goal_kind` match arm at line 99). Adds a `DECL_ASK_WITNESS` `GoalDispatchDeclaration` at `crates/worldwake-ai/src/goal_dispatch_decl.rs` referencing a new `ASK_WITNESS_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::AskWitness]` and a new `EPISTEMIC_SENSING_POLICY: GoalFamilyPolicy` constant beside the existing family policy constants in `crates/worldwake-ai/src/goal_dispatch_decl.rs`. Implements all 11 `GoalKindPlannerExt` methods at `crates/worldwake-ai/src/goal_model.rs:40-82` for the new variant (reusing the existing private helpers `ask_witness_payload_matches_subject` at line 95 and `epistemic_subject_for_belief` at line 107). Adds `emit_ask_witness_candidates` in `crates/worldwake-ai/src/candidate_generation.rs`. Adds `GoalPriorityClass` assignment and the `motive_score` contribution in `crates/worldwake-ai/src/ranking.rs`.
- `worldwake-cli` — no `AgentDef` plumbing change — `AgentDef.epistemic_disposition: Option<EpistemicDispositionProfile>` already exists at `crates/worldwake-cli/src/scenario/types.rs:603`, and `spawn_agent()` already applies it. The new `witness_recency_preference` field flows through automatically because (a) `EpistemicDispositionProfile` derives `Serialize`/`Deserialize`, (b) the new field is `#[serde(default)]`-annotated, and (c) the existing `unwrap_or_default()` / `set_component_epistemic_disposition_profile` plumbing carries the extended struct unchanged. Observer rendering of `GoalKind::AskWitness` commits routes through the existing decision-trace path (Section 3b Decision History in `crates/worldwake-cli/src/bin/observer.rs`); no new observer section is introduced.

## Dependencies

- S130 (Survey Records and Frontier Disconfirmation) — completed and archived at `archive/specs/S130-survey-records-frontier-disconfirmation.md`. Provides hypothesis-driven `ExploreLocation` and establishes the pattern of belief-anchored goal emission this spec mirrors.
- S113 (Belief Envelope) — completed and archived at `archive/specs/S113-belief-envelope.md`. Provides `BelievedEntityState`, `PerceptionSource`, and confidence semantics. Inputs to the satisfaction predicate.
- S114 (Plan Step Guards) — completed and archived at `archive/specs/S114-plan-step-guards.md`. The new goal produces a belief-update effect that satisfies guards on later plan steps.
- S109 (Typed Discrepancy Taxonomy) — completed and archived at `archive/specs/S109-typed-discrepancy-taxonomy.md`. Provides `LearnedOpportunityMemory` (queried via `GoalBeliefView::learned_opportunity_memory` at `crates/worldwake-sim/src/belief_view.rs:319`) and the dampening pathway used by ranking.
- S110 (Decision History Events) — completed and archived at `archive/specs/S110-decision-history-events.md`. Existing event tags carry the new goal commits without modification.
- S134 (Canonical Effect Schema) — active at `specs/S134-canonical-effect-schema.md` and implemented. The existing `EffectStep::AskWitness` at `crates/worldwake-sim/src/effect_schema.rs:185` and its sink dispatch at line 942 are the action-layer surface S139 reuses verbatim.
- S137 (Plan Causal Links and Repair) — completed and archived at `archive/specs/S137-plan-causal-links-and-repair.md`. Soft dependency: `RepairKind::InsertVerification` splices the new goal as a repair step before a low-confidence-belief guard.
- S138 (Affordance-to-Opportunity Compiler) — completed and archived at `archive/specs/S138-opportunity-compiler.md`. The emitter scans the agent's belief envelope directly rather than consuming S138's opportunity records (the opportunity surface does not currently model per-witness testimony topics).
- S59 (Expectation and Obligation Substrate) — completed and archived at `archive/specs/S59-expectation-obligation-substrate.md`. Not directly consumed by S139 (with `InspectContainer` deferred, `expectation_id` linkage is no longer needed in this spec).

## Design Goals

1. **Discrete goal kind, not action kind.** Per the established `AcquireCommodity`/`Wash`/`Sleep` pattern, the epistemic intent is a first-class `GoalKind` variant with a satisfaction predicate, not just a one-step action. The action exists already; this spec adds the goal-layer surface that decides when the action should fire.
2. **Satisfaction predicate over belief envelope.** The predicate asserts the agent now holds a belief about the topic sourced from this witness (or with confidence ≥ `stale_evidence_barrier_threshold` if the freshness pathway is more appropriate for the variant). The world's truth value is irrelevant to whether the goal is satisfied — what matters is that the agent has updated their belief through the testimony path.
3. **Witness anchoring over reachable testimony.** `AskWitness` candidates emit only when the agent's belief envelope contains a `BelievedEntityState` whose `source = PerceptionSource::Report { from: witness, .. }` for a co-located witness, OR the witness is co-located AND the topic appears in the agent's belief envelope at confidence below `stale_evidence_barrier_threshold` (the cold-start case where no prior report exists but the witness is physically accessible). No global witness query — the emitter respects FND-15 by surfacing only witnesses the agent has lawful information about.
4. **Per-agent threshold for "verify before act."** The existing `EpistemicDispositionProfile.stale_evidence_barrier_threshold` (currently `pm(400)` default at `epistemic.rs:36`) controls when a low-confidence belief triggers an epistemic detour. A high-courage / low-doubt agent might act on `confidence ≥ pm(400)`; a magistrate verifying testimony before issuing a warrant might require `confidence ≥ pm(800)`. The reassessment found that this lever is already wired through `goal_model.rs:148`; S139 extends its semantic use from barrier insertion to emitter gating.
5. **No teleporting truth.** The action updates the agent's belief envelope through the shipped `apply_ask_witness_commit` path at `crates/worldwake-systems/src/epistemic_actions.rs:405-471`, which calls `import_entity_snapshot()` with `PerceptionSource::Report { from: target, chain_len: 1 }` — the testimony provenance FND-15 requires. The action's `EventTags = Social + Discovery` (epistemic_actions.rs:62) already make the act observable to co-located third parties.
6. **Determinism.** The new goal kind and its candidate emitter iterate `BTreeMap`-stable. The satisfaction predicate is a deterministic function of belief state.
7. **No silent privilege.** The action's existing preconditions (`ActorAlive`, `TargetExists(0)`, `TargetAtActorPlace(0)`, `TargetKind(Agent)`, `TargetAlive(0)` plus the incapacitation check at `epistemic_actions.rs:282-284`) already enforce locality and target legality. The goal layer adds no bypass.
8. **No new action infrastructure.** All action-layer machinery — `ActionDef`, `AskWitnessPayload`, `EffectStep::AskWitness`, `AskWitnessEffectSink`, `AskWitnessMemory` cooldown — exists and is exercised by tests. S139 introduces zero new files in `worldwake-systems/`.
9. **Single-source-of-truth for epistemic disposition.** S139 extends `EpistemicDispositionProfile` rather than introducing a parallel `EpistemicProfile` component. Two epistemic-disposition components on every agent would violate FND-28 and the spec-drafting-rules section 5f semantic-overlap discipline.

## Non-Goals

- **`InspectContainer`.** Deferred to a future sibling spec. It requires net-new action machinery (`PlannerOpKind::InspectContainer`, `InspectContainerPayload`, `EffectStep::InspectContainer`, a new sink), a new `ObservedContainerContents` perception event tag, AND a "believed container access right" belief surface that respects FND-24's ownership/custody/access/jurisdiction distinction. That substrate is comparable in scope to S139 itself and warrants its own design.
- **`VerifyBelief`, `ConsultRecord`, `ScoutPlace`.** Deferred. `VerifyBelief` is a meta-goal whose decomposition produces `AskWitness`-class instances — the right shape after HTN methods (Phase 12) land. `ConsultRecord` is partially covered by existing `consult_record_actions`; promoting it to a `GoalKind` adds little until S140's artifact lifecycle differentiates "actionable" vs "reference-only" records. `ScoutPlace` overlaps with S130's hypothesis-driven `ExploreLocation`.
- **A new `EpistemicProfile` component.** The existing `EpistemicDispositionProfile` is the per-agent substrate. Two epistemic-related universal components would create dual truth (FND-28) and unacknowledged semantic overlap (spec-drafting-rules 5f).
- **`TellTopic::SocialObservation` and `TellTopic::InstitutionalClaim` topics.** The initial `GoalKind::AskWitness` payload override accepts only `TellTopic::EntityBelief { subject }` and maps it to `AskWitnessPayload { target: witness, topic_entity: Some(subject), topic_commodity: None }`. The `SocialObservation` and `InstitutionalClaim` variants of `TellTopic` do not have a clean projection onto the current `AskWitnessPayload` split-field shape; their handling is deferred.
- **Forced honesty.** Witnesses can lie, refuse, or misremember. `AskWitness` produces a belief update sourced from the witness, not a truth update. Lie modeling is exercised by Scenario G goldens.
- **Cross-room shouting.** The action requires co-location (FND-7). Long-distance witness inquiry routes through travel.
- **Multi-witness fan-out.** The agent asks one witness per `AskWitness` commit. Multi-witness compare-and-contrast is the planner's job through repeated emission, not a single-goal aggregation.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-7 (Locality of Motion, Interaction, and Communication) | The action's existing co-location precondition (`TargetAtActorPlace(0)`) is preserved. Long-distance witness inquiry routes through travel; no remote query. |
| FND-14 (World State Is Not Belief State) | The satisfaction predicate reads belief state only. The world's truth about the topic does not affect goal satisfaction. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Belief import carries `PerceptionSource::Report { from: witness, chain_len: 1 }` provenance, the same testimony carrier ordinary tell events use. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | The goal is the agent-level expression of "I do not know enough; I will find out." Confidence threshold is per-agent. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Verifications are agent decisions under bounded budget; suppression at high stress (via `EPISTEMIC_SENSING_POLICY`) prevents an agent in critical survival load from indulging them. |
| FND-21 (Intentions Are Revisable Commitments) | The verification goal can be suspended if more urgent goals arise. |
| FND-22 (Agent Diversity Through Concrete Variation) | The extended `EpistemicDispositionProfile.stale_evidence_barrier_threshold` and new `witness_recency_preference` together make per-agent variation explicit: paranoid magistrate vs trusting villager. |
| FND-28 (No Backward Compatibility) | S139 extends the existing `EpistemicDispositionProfile` rather than introducing a parallel `EpistemicProfile` component (single source of truth). `AskWitnessPayload`'s split-field shape is preserved because it models a *filter* abstraction (open-quantified over an entity or commodity), distinct from `TellTopic`'s *tagged-union* abstraction at the goal layer — the `build_payload_override` translation captures the abstraction-level transition explicitly, not a duplicate representation. |
| FND-29 (Debuggability Is a Product Feature) | The chain "agent received stale rumor → committed `AskWitness` → witness's belief imported with provenance → confidence threshold crossed → new plan adopted" is a sequence of inspectable goal commits, decision events, and `PerceptionSource::Report` provenance entries. |

## Deliverables

### D1. `worldwake-core::goal::GoalKind` extension

```rust
pub enum GoalKind {
    // existing variants preserved (Eat, Drink, Wash, Sleep, AcquireCommodity, ...)
    AskWitness {
        witness: EntityId,
        topic: TellTopic,  // initial scope: TellTopic::EntityBelief only
    },
}
```

The variant's fields are `Copy` (verified: `EntityId` Copy at `ids.rs`, `TellTopic` Copy at `belief.rs:1736`), so the existing `#[derive(Copy)]` on `GoalKind` at `goal.rs:61` is preserved.

### D2. Extend `EpistemicDispositionProfile`

In `crates/worldwake-core/src/epistemic.rs:23`, add one new field to the existing universal component:

```rust
pub struct EpistemicDispositionProfile {
    pub stale_evidence_barrier_threshold: Permille,
    pub witness_query_duration_ticks: NonZeroU32,
    pub ask_memory_retention_ticks: u32,
    #[serde(default = "default_witness_recency_preference")]
    pub witness_recency_preference: Permille,  // NEW: weighting per-tick freshness vs first-hand-distance
}

fn default_witness_recency_preference() -> Permille { Permille::new_unchecked(500) }

impl Default for EpistemicDispositionProfile {
    fn default() -> Self {
        Self {
            stale_evidence_barrier_threshold: Permille::new_unchecked(400),
            witness_query_duration_ticks: NonZeroU32::new(2).unwrap(),
            ask_memory_retention_ticks: 12,
            witness_recency_preference: default_witness_recency_preference(),
        }
    }
}
```

Update all explicit-construction sites that enumerate every field (no `..Default::default()` spread) to include `witness_recency_preference`. Verified implementation fallout for ticket 002 covered 11 pre-existing explicit sites:

- `crates/worldwake-systems/src/ask_about_person_actions.rs`
- `crates/worldwake-systems/src/epistemic_actions.rs`
- `crates/worldwake-sim/src/action_semantics.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-core/src/world.rs` (two fixtures)
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-core/src/epistemic.rs` (existing `Default`-mirroring fixture; verify it stays in sync)

RON scenario files in `scenarios/` (3 sites: `survival-ask-consult.ron`, `cli-evaluation.ron`, `final-integration.ron`) do NOT need editing because the new field is `#[serde(default)]`.

The component is already registered in `component_schema.rs:2119` and seeded in `world.rs::create_agent` at line 200 — no new registration work. The existing belief-view accessor `epistemic_disposition_profile(actor)` at `belief_view.rs:548`, `1171`, `2020` already surfaces the extended struct to the AI crate.

### D3. `GoalDispatchKey` extension

In `crates/worldwake-ai/src/goal_dispatch_key.rs`:
- Add `AskWitness` variant to the enum at line 6.
- Bump `ALL: [Self; 40]` at line 50 to `[Self; 41]` and add `Self::AskWitness` to the array.
- Add the `from_goal_kind` match arm at line 99: `GoalKind::AskWitness { .. } => Self::AskWitness`.

### D4. `GoalDispatchDeclaration` for `AskWitness`

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`:

```rust
const ASK_WITNESS_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::AskWitness];
const ASK_WITNESS_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::AskWitness];

const DECL_ASK_WITNESS: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AskWitness",
    provenance_family: Some(RankedGoalProvenanceFamily::EpistemicSensing),
    relevant_ops: ASK_WITNESS_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: EPISTEMIC_SENSING_POLICY,
    progress_barrier_ops: ASK_WITNESS_BARRIER,
};
```

The `invalidation_strategy`, `feasibility_strategy`, and `frontier_exhaustion_strategy` choices match the existing ask-witness action semantics and the closest testimony-path declaration.

### D5. `EPISTEMIC_SENSING_POLICY` constant

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`, alongside `SELF_CARE_POLICY`, `ENTERPRISE_POLICY`, `SOCIAL_POLICY`, `SHARE_BELIEF_TESTIMONY_POLICY`:

```rust
const EPISTEMIC_SENSING_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical),
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Normal,
};
```

`GoalFamilyPolicy` is a struct, not an enum (`goal_policy.rs:71-75`); the family is created by defining the constant, not by adding an enum variant. `GoalPriorityClass::Critical` is the actual top variant in `ranking.rs:1989-1995`. `goal_policy.rs` keeps the shared policy types, evaluation helper, and focused policy tests; the concrete dispatch-family constants live with the declaration table.

### D6. `GoalKindPlannerExt` implementations

In `crates/worldwake-ai/src/goal_model.rs` (trait at lines 40-82), add match arms for `GoalKind::AskWitness { witness, topic }` to all 11 trait methods:

1. `ranked_goal_provenance_family` → `Some(RankedGoalProvenanceFamily::EpistemicSensing)`.
2. `relevant_op_kinds` → `&[PlannerOpKind::Travel, PlannerOpKind::AskWitness]`.
3. `target_commodity` → `None` (no commodity target).
4. `relevant_observed_commodities` → `None`.
5. `build_payload_override` → constructs `AskWitnessPayload { target: witness, topic_entity, topic_commodity }` from `topic` per the mapping below. Registered with `with_payload_override_validator` because `AskWitnessPayload` is planner-synthesized (not affordance-derived). The validator is the existing `ask_witness_payload_matches_subject` helper at `goal_model.rs:95`.
6. `is_progress_barrier` → `true` when the step's op is `PlannerOpKind::AskWitness` and the binding matches the goal's witness.
7. `is_satisfied` → reads `view.entity_beliefs_sourced_from_witness(agent, witness)` (new `GoalBeliefView` accessor — see D8) and returns `true` when a belief on `topic`'s subject exists with `source = PerceptionSource::Report { from: witness, .. }` whose observed tick is within the freshness window derived from `EpistemicDispositionProfile.witness_recency_preference` and the live `BeliefConfidencePolicy.staleness_penalty_per_tick`, OR the belief's confidence ≥ `stale_evidence_barrier_threshold`. Ticket 007 replaced the stale `TODO(S139EPISENSUB-002)` satisfaction placeholder with this concrete freshness branch before the golden ticket.
8. `goal_relevant_places` → `[view.effective_place(witness)]` (uses the existing accessor — co-location is the only relevant place).
9. `prerequisite_places` → same as `goal_relevant_places`.
10. `matches_binding` → `authoritative_targets.contains(&witness)` for `PlannerOpKind::AskWitness`; standard travel-binding for `PlannerOpKind::Travel`.
11. `candidate_is_available` → `true` when the agent's belief envelope holds a topic entry whose confidence is below `stale_evidence_barrier_threshold` AND the witness-cooldown gate (driven by `ask_memory_retention_ticks` via the existing `AskWitnessMemory` substrate) is not active.

`build_payload_override` mapping (initial scope):
- `TellTopic::EntityBelief { subject }` → `AskWitnessPayload { target: witness, topic_entity: Some(subject), topic_commodity: None }`.
- `TellTopic::SocialObservation { .. }` / `TellTopic::InstitutionalClaim { .. }` → return `GoalPayloadOverrideError::UnsupportedTopic` (deferred, per Non-Goals).

### D7. `emit_ask_witness_candidates`

In `crates/worldwake-ai/src/candidate_generation.rs`, add an emitter following the per-target enumeration pattern from `emit_engage_hostile_goals` (lines 2486-2565):

```rust
fn emit_ask_witness_candidates(
    candidates: &mut Vec<GoalOffer>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) { /* … */ }
```

The emitter fires when:
1. The agent's `EpistemicDispositionProfile.stale_evidence_barrier_threshold` gates apply (read through the existing `epistemic_disposition_profile` belief-view accessor) AND
2. The agent's belief envelope contains a `BelievedEntityState` for some subject whose `source = PerceptionSource::Report { from: witness, .. }` for a co-located witness AND that belief's confidence is below `stale_evidence_barrier_threshold`, OR a co-located witness is named in the belief envelope as a known agent AND the agent's belief about the topic subject has confidence below `stale_evidence_barrier_threshold` (cold-start case) AND
3. The `AskWitnessMemoryKey { counterparty: witness, topic_entity, topic_commodity }` cooldown window has elapsed (`current_tick - asked_tick ≥ ask_memory_retention_ticks`) per the existing `AskWitnessMemory` substrate at `belief.rs:1763` (the same gate consumed by `epistemic_actions.rs:466` and `ask_about_person_actions.rs:518`).

Emission is capped at `K = 3` per topic per tick, ranked by witness recency × testimony freshness (weighted by `witness_recency_preference`), to dampen fan-out when many co-located witnesses match (see Risks). Each emission produces one `GoalKind::AskWitness { witness, topic }` with `OpportunityAnchor::Entity(witness)`.

The emitter is wired into the agent_tick candidate-generation phase at the same call-site tier as `emit_social_candidates`.

### D8. `GoalBeliefView` accessor for testimony-sourced beliefs

In `crates/worldwake-sim/src/belief_view.rs` (trait at line 270), add:

```rust
fn entity_beliefs_sourced_from_witness(
    &self,
    agent: EntityId,
    witness: EntityId,
) -> Vec<(EntityId, BelievedEntityState)>;
```

Backing `RuntimeBeliefView` impl scans `AgentBeliefStore.known_entities` (BTreeMap, deterministic iteration) and filters entries whose `source` matches `PerceptionSource::Report { from, .. }` with `from == witness`. `impl_goal_belief_view!` macro forwards.

The existing `epistemic_disposition_profile(actor)` accessor at lines 548, 1171, 2020 already surfaces the extended `EpistemicDispositionProfile` to the AI crate — no new profile accessor is required.

### D9. Ranking integration

In `crates/worldwake-ai/src/ranking.rs`, add `GoalKind::AskWitness` to the `GoalPriorityClass` assignment. Initial class: `Low` (epistemic detours are dispensable under stress). `motive_score` contribution combines:
- Base salience proportional to the gap between current belief confidence and `EpistemicDispositionProfile.stale_evidence_barrier_threshold`.
- Recency bonus proportional to staleness of the belief being verified, weighted by `EpistemicDispositionProfile.witness_recency_preference`.
- Damping via the existing `LearnedOpportunityMemory` pathway (queried through `GoalBeliefView::learned_opportunity_memory` at `belief_view.rs:319`) so repeated fruitless asks are progressively de-prioritized.

The motive_score formula must be expressible in `Permille` arithmetic (no floats per CLAUDE.md determinism invariant).

### D10. `PlannerOpKind::AskWitness` classification audit

In `crates/worldwake-ai/src/planner_ops.rs`, `PlannerOpKind::AskWitness` already exists at line 47 and is classified at line 136 (`(ActionDomain::Epistemic, "ask_witness") => Some(PlannerOpKind::AskWitness)`). No code changes required here; this deliverable is the verification step in the ticket.

### D11. Observer integration and profile-doc regeneration

Existing decision-trace rendering in `crates/worldwake-cli/src/bin/observer.rs` (Section 3b Decision History) already renders `GoalKind` commits through `DecisionEventPayload`. Adding `GoalKind::AskWitness` to the trace surface requires no new section; verify the variant appears in the existing payload-summary path.

Regenerate `docs/profiles/all-profiles.md` via `python3 scripts/profile_docs.py --write` after D2 lands so the auto-generated profile doc reflects the new `witness_recency_preference` field.

## FND-01 Section H — Causal Hooks Declaration

1. **Specific missing downstream consequence motivating the system.** Without a `GoalKind::AskWitness`, agents cannot adopt the intent to verify a stale or contradicted belief by asking a co-located witness. The repair search (`S137 RepairKind::InsertVerification`) has nowhere to splice an ask step. FOUNDATIONS Scenario G (false-rumor → wrongful-accusation) is unreachable from current goal-layer surfaces. The action layer exists (S134's `EffectStep::AskWitness`) but cannot fire absent a planner intent that names it. Existing `GoalKind` variants do not produce this consequence: `ShareBelief` is outbound testimony, `ExploreLocation` is place-anchored, `InvestigateViolation` is record-anchored.
2. **Concrete entities, relations, records introduced.** One new field on existing `EpistemicDispositionProfile`. No new records, no new relations, no new components. Reuses `AskWitnessMemory`/`AskWitnessMemoryKey` (already in core at `belief.rs:1763`) for cooldown tracking.
3. **Actions or world processes mutating them.** `EpistemicDispositionProfile` is scenario-authored at agent spawn (immutable thereafter; future specs may introduce experience-driven adaptation). `AskWitnessMemory` is mutated by the existing `apply_ask_witness_commit` at `epistemic_actions.rs:460-466`.
4. **Information produced, how it travels, who can observe it.** The action emits `EventTags = Social + Discovery` (epistemic_actions.rs:62), observable by co-located third parties through the existing visibility surface. Belief import carries `PerceptionSource::Report { from: witness, chain_len: 1 }` provenance to the actor. No new information path is created; this spec only adds the intent surface that triggers the existing flow.
5. **Quantities conserved, transferred, transformed.** None. The action transfers belief content, not material quantities; `apply_ask_witness_commit`'s import path is already conservation-neutral.
6. **Scarce capacities, exclusive affordances, reservations, queues, claims.** Witness attention is implicitly exclusive: the action's existing `EntityAtActorPlace { kind: EntityKind::Agent }` target spec at `epistemic_actions.rs:51-53` plus `BindingStrictness::ExactIdentity` at line 65 prevents another agent from simultaneously asking the same witness on the same tick (one of them queues via the scheduler).
7. **Partial failures, degraded states, aftermath.** The witness may have no belief on the topic — the existing sink imports nothing; the agent's belief is unchanged but `AskWitnessMemory.asked_tick` updates, triggering the cooldown. The witness may be incapacitated (handled by `is_authoritatively_incapacitated` check at `epistemic_actions.rs:282-284`). The witness may relocate during travel before arrival — the standard travel-step revalidation handles this through existing plan-step expectations (S114).
8. **Positive feedback loops amplified.** Potential loop: low-confidence belief triggers verification → verification produces a fresh belief → fresh belief triggers more action → action exposes more low-confidence beliefs → more verifications.
9. **Physical dampeners limiting loops.**
   - `EpistemicDispositionProfile.stale_evidence_barrier_threshold` — per-agent; once met, no further verification fires for that topic.
   - `EpistemicDispositionProfile.ask_memory_retention_ticks` — TTL on repeat inquiry of the same `(witness, topic)` pair, enforced by the existing `AskWitnessMemory` substrate.
   - `EPISTEMIC_SENSING_POLICY.suppression = WhenStressedAtOrAbove(GoalPriorityClass::Critical)` halts emission under critical-survival load.
   - `LearnedOpportunityMemory` (S109) damps repeated fruitless witness inquiries through the existing ranking-side pathway at `belief_view.rs:319`.
   - Per-tick emission cap `K = 3` per topic prevents fan-out spikes when many co-located witnesses match.
10. **Agent-local learned, memory, habit, trust updates.** `AskWitnessMemory` (existing) captures asked-tick per `(counterparty, topic_entity, topic_commodity)`. `LearnedOpportunityMemory` (existing) progressively damps fruitless asks. Authoritative state: both memories. Summary: the emitter's per-tick decision (transient).
11. **How agents become wrong, how they correct, provenance/freshness markers.** Imported beliefs carry `PerceptionSource::Report { from, chain_len }` provenance with attached confidence (computed via `BeliefConfidencePolicy` at `belief.rs:2489-2498`). Stale or contradicted beliefs surface through the existing `BeliefStatus::Stale | Disputed | Contradicted` axis at `belief_view.rs:41-47`. A witness who lies produces a belief with normal Report provenance whose later contradiction surfaces through perception of conflicting evidence — Scenario G goldens exercise this path.
12. **Lifecycle states, transitions, visibility, legality, actionability differences.** `AskWitnessMemory` entries do not expire; they are referenced by cooldown checks. `EpistemicDispositionProfile` is configured at spawn and persists for agent lifetime. No new lifecycle is introduced.
13. **Temporal/spatial resolution, scheduling regime, tie-breaking.** Same as the existing `ask_witness` action: per-tick scheduling, co-location precondition, deterministic `BTreeMap` iteration in candidate generation, scheduler arbitration when multiple agents ask the same witness.
14. **Boundary conditions, external drivers, off-map interfaces.** None. The new goal kind operates entirely on local beliefs and co-located witnesses; no off-map interaction.
15. **Derived views, caches, optimizations.** Candidate emission is transient (per-tick derived view). `LearnedOpportunityMemory` is authoritative cached state that survives ticks. No new caches.
16. **Causal records, event identities, provenance links emitted.** No new event tags. Existing `Social + Discovery` event tags + decision-history payloads emitted by the action carry the causal trace. `PerceptionSource::Report` on imported beliefs carries the knowledge-path link to the witness.
17. **Target patterns, invariants, regression cases, falsification checks.** See Validation and Falsification.
18. **What must survive save/load, replay, offscreen compression.** Extended `EpistemicDispositionProfile` (existing serde derives; new field has `#[serde(default)]` for omitted-field serde inputs such as authored RON profile snippets). `AskWitnessMemory` (already saved). `GoalKind::AskWitness` variant in active goal commits (already part of save format via existing `GoalKind` derive). `SAVE_FORMAT_VERSION` bumps from 83 → 84 because the bincode-backed serialized layout of `EpistemicDispositionProfile` changes; full pre-bump save files remain rejected by the save header.

## SystemFn Integration

No new `SystemFn`. Action execution flows through the existing scheduler at `crates/worldwake-sim/src/scheduler.rs`. Candidate emission flows through the existing `agent_tick` candidate-generation phase at `crates/worldwake-ai/src/agent_tick/`.

## Component Registration

- `EpistemicDispositionProfile` — already universal, already registered on `EntityKind::Agent` at `crates/worldwake-core/src/component_schema.rs:2119`, already seeded in `world.rs::create_agent`. No registration change.

## Cross-System Interactions

- **AI → Sim**: The goal layer commits a `GoalKind::AskWitness`; the planner produces an action plan that the existing `ask_witness` action handler executes. No new direct call.
- **Sim → Core**: The action's effect sink imports the witness's beliefs into the actor's `AgentBeliefStore` via the existing `import_entity_snapshot()` path.
- **Sim → AI**: Belief updates surface to the next agent_tick through the existing belief-view facade; the satisfaction predicate reads them through `GoalBeliefView::entity_beliefs_sourced_from_witness`.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

`EpistemicDispositionProfile` is the per-agent profile. Existing fields (`stale_evidence_barrier_threshold: Permille`, `witness_query_duration_ticks: NonZeroU32`, `ask_memory_retention_ticks: u32`) are reused; one new field (`witness_recency_preference: Permille`) is added. All fields use `Permille` or `Tick`/`u32` types. Two agents with identical beliefs on the same topic will trigger or skip verification differently because their thresholds differ.

## Authoritative-to-AI Impact Analysis

The spec adds a new `GoalKind` variant and a new candidate emitter. Per CLAUDE.md's Authoritative-to-AI Impact Rule, the 7-checklist applies:

1. **`get_affordances`** — N/A. No new affordances. The existing `ask_witness` action's affordance generation is unchanged.
2. **`generate_candidates`** — affected. New `emit_ask_witness_candidates` wired into the agent_tick candidate-generation phase. The emitter respects locality (co-located witnesses only) and confidence thresholds (per-agent).
3. **`search_plan`** — affected. The new `ASK_WITNESS_BARRIER` plus `relevant_op_kinds = [Travel, AskWitness]` define plan shape. Terminal ordering matches the existing `ShareBelief` pattern (travel-then-terminal).
4. **`BestEffort` action start** — N/A. The new GoalKind reuses the existing `ask_witness` action handler unchanged.
5. **`handle_plan_failure`** — affected. Failure of the `AskWitness` step (witness moved, witness incapacitated, target-place no longer reachable) must trigger correct replan. Verified by Validation Scenario 6 below.
6. **Payload revalidation** — affected. `build_payload_override` synthesizes `AskWitnessPayload` from the goal's `(witness, topic)` pair. The handler must register `with_payload_override_validator` pointing at the existing `ask_witness_payload_matches_subject` helper at `goal_model.rs:95` so `plan_revalidation.rs` accepts the synthesized payload at step start.
7. **Golden tests** — required. New `golden_epistemic_sensing.rs` covers the six scenarios listed below. Existing 1440-tick survival goldens must remain green (the emitter fires only when `stale_evidence_barrier_threshold` is breached, which does not occur in survival-baseline scenarios with default profiles).

## Validation and Falsification

- **Golden coverage**: new `crates/worldwake-ai/tests/golden_epistemic_sensing.rs` with six scenarios:
  1. **Stale-belief verification**: agent holds a belief about subject X imported with `PerceptionSource::Report { from: witness_a, chain_len: 1 }` whose confidence has decayed below `stale_evidence_barrier_threshold`; witness_a is co-located. Expected: `AskWitness { witness: witness_a, topic: TellTopic::EntityBelief { subject: X } }` commit, action executes, belief refreshed with updated tick, threshold crossed.
  2. **Cold-start ask**: agent has no prior belief about subject X but has a low-confidence belief acquired from rumor (`PerceptionSource::Rumor`); a co-located known witness has a belief about X. Expected: emitter fires, ask commits, witness's belief imports with Report provenance.
  3. **FOUNDATIONS Scenario G chain**: agent receives testimony A about subject X from witness_a, later receives contradicting testimony B from witness_b. Expected: belief status transitions to `Disputed`; emitter fires for follow-up asks; contradiction surfaces in the belief envelope without omniscient correction.
  4. **Critical-survival suppression**: hungry agent at `GoalPriorityClass::Critical` stress holds a low-confidence belief about subject X with a co-located witness. Expected: emitter does not fire (suppressed by `EPISTEMIC_SENSING_POLICY.suppression`), agent prioritizes self-care.
  5. **Cooldown gate**: agent asks witness W about topic T at tick `t0`; at tick `t0 + (ask_memory_retention_ticks - 1)`, the belief envelope still shows low confidence. Expected: emitter does NOT fire (cooldown active); at tick `t0 + ask_memory_retention_ticks`, emitter fires again.
  6. **Plan-failure replan**: agent commits `AskWitness`, travels to witness's last-known place; witness has relocated. Expected: travel-step revalidation fails, plan replan re-runs candidate generation with updated belief about witness's location.

- **No regression**: existing 1440-tick survival goldens unaffected — the emitter fires only when `stale_evidence_barrier_threshold` is breached, which does not happen at default profiles in survival-baseline.

## Risks

- **Witness-availability fan-out.** A scenario with many co-located witnesses could emit many `AskWitness` candidates per tick. Mitigation: emitter caps per-tick emissions at `K = 3` per topic, ranked by witness recency × testimony freshness (weighted by `witness_recency_preference`).
- **Topic-shape impedance mismatch.** `TellTopic` is a tagged-union; `AskWitnessPayload`'s `topic_entity`/`topic_commodity` is a filter shape. Initial scope handles only `TellTopic::EntityBelief`. Mitigation: `build_payload_override` returns `GoalPayloadOverrideError::UnsupportedTopic` for `SocialObservation` / `InstitutionalClaim` variants until a follow-up spec widens `AskWitnessPayload` or introduces parallel payloads.
- **Belief-update collision.** A witness can be asked about a topic the agent's belief envelope holds with high confidence from another source. Mitigation: emitter only fires below `stale_evidence_barrier_threshold`; satisfaction predicate respects the existing belief-merge rules (S113 envelope merge), not naive overwrite.
- **Cooldown-tuning sensitivity.** Too-short retention allows spam; too-long blocks legitimate repeat checks. Mitigation: default `ask_memory_retention_ticks = 12` (existing value); golden Scenario 5 locks the boundary.
- **Field-addition save-format bump.** Extending `EpistemicDispositionProfile` requires `SAVE_FORMAT_VERSION` 83 → 84 because the bincode-backed serialized layout changes. Mitigation: the new field is `#[serde(default)]` for omitted-field authored/self-describing serde inputs, while full pre-bump save files remain rejected by the save header.
