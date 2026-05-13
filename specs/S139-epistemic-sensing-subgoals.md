# S139: Epistemic Sensing Subgoals — Ask Witness and Inspect Container

**Status**: Draft

## Summary

Worldwake's belief architecture treats provenance, freshness, contradiction, and refutation as first-class (S113 belief envelope, S122 frame assumptions, S109 typed discrepancies). What's missing is the agent-level *intent to learn*: today's planner inserts a verification step only as a side effect of frontier exploration (S130 `ExploreLocation` with `HypothesisKind`) or by stumbling into a contradiction during execution (S114 expectation mismatch). The agent has no first-class way to say "I will go ask the captain whether the caravan returned" or "I will open the chest to confirm the gold is there before riding to court."

The assessor proposes five new epistemic `GoalKind` variants: `VerifyBelief`, `ConsultRecord`, `AskWitness`, `ScoutPlace`, `InspectContainer`. Of these, S130 already provides hypothesis-driven exploration (covering `ScoutPlace`-class needs through `ExploreLocation { hypothesis }`), and `ConsultRecord` exists implicitly through existing record-reading actions in `consult_record_actions.rs`. The two highest-leverage additions are `AskWitness` (FOUNDATIONS Scenario G — false-rumor → wrongful-accusation chain requires agents to question witnesses about prior testimony) and `InspectContainer` (FOUNDATIONS Scenario C — stored-gold robbery report requires the owner to inspect the stash to discover mismatch).

S139 lands those two as discrete `GoalKind` variants, each with: a satisfaction predicate over the agent's belief envelope (the agent has a belief about the topic with confidence ≥ a threshold), a candidate-generation pass that anchors on perceived/recalled witnesses and containers, and an action handler that queries the witness/container and updates beliefs through the existing perception/testimony paths. The repair search (S137) can splice these as `RepairKind::InsertVerification` steps before a guard that depends on a low-confidence belief.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `GoalKind` (`crates/worldwake-core/src/goal.rs:62`) with `AskWitness { witness: EntityId, topic: TellTopic }` and `InspectContainer { container: EntityId, expectation_id: Option<ExpectationId> }`. Adds `EpistemicProfile { verification_threshold: Permille, witness_recency_preference: Permille }` (universal per-agent).
- `worldwake-ai` — extends `goal_dispatch_decl.rs` with declarations for the two new goal kinds. Extends `candidate_generation.rs` with epistemic emitters: `emit_ask_witness_candidates` (anchors on agents in the belief store with `TestifiedAbout` claims about a topic the agent's belief is stale/contradicted on) and `emit_inspect_container_candidates` (anchors on owned/accessible containers about which the agent holds an outstanding `Expectation` per S59 or a low-confidence belief). Extends `goal_policy.rs` with new family policy entries.
- `worldwake-systems` — new `ask_about_witness_actions.rs` (extends the existing `ask_about_person_actions.rs` pattern; the assessor's `AskWitness` is topic-anchored rather than person-anchored, so the new module is sibling to it). New `inspect_container_actions.rs` for the open-and-observe flow. Both produce `EffectSchema` declarations under S134's completed contract.
- `worldwake-cli` — `AgentDef.epistemic_profile` field; observer Section 4 (Goals) renders epistemic goal commits with the topic / container / expectation under inspection.

## Dependencies

- S130 (Survey Records and Frontier Disconfirmation) — completed. Provides hypothesis-driven `ExploreLocation`. S139 reuses `HypothesisKind` semantics where they overlap (e.g., `InspectContainer` with `expectation_id: None` falls back to a discovery-style observation).
- S113 (Belief Envelope) — completed. Provides `BeliefValue<T>.confidence` and `BeliefStatus`. Both are inputs to the satisfaction predicate.
- S114 (Plan Step Guards) — completed. `AskWitness` and `InspectContainer` produce belief-update effects that satisfy guards on later steps.
- S109 (Typed Discrepancy Taxonomy) — completed. `Discrepancy::WitnessUnreachable` and `Discrepancy::ContainerInaccessible` join the taxonomy.
- S110 (Decision History Events) — completed. Existing event tags carry the new goal commits.
- S59 (Expectation and Obligation Substrate) — completed. `InspectContainer.expectation_id` references existing `ExpectationId`; the inspection result resolves the expectation as fulfilled, mismatched, or unchanged.
- S137 (Plan Causal Links and Repair) — completed and archived at `archive/specs/S137-plan-causal-links-and-repair.md`. Soft dependency: `RepairKind::InsertVerification` splices these goals as repair steps. Order-independent.
- S138 (Affordance-to-Opportunity Compiler) — Phase 11 sibling. Soft dependency: opportunities anchored on perceived witnesses/containers feed the new emitters.

## Design Goals

1. **Discrete goal kinds, not action kinds.** Per the established `AcquireCommodity`/`Wash`/`Sleep` pattern, epistemic intents are first-class `GoalKind` variants with satisfaction predicates, not just one-step actions.
2. **Satisfaction predicate over belief envelope.** A `VerifyBelief`-style predicate cannot directly read world state; it asserts the agent now holds a belief on the named topic with confidence ≥ `verification_threshold`. The world's truth value is irrelevant to whether the goal is satisfied — what matters is that the agent has updated their belief.
3. **Witness anchoring over reachable testimony.** `AskWitness` candidates emit only when the agent's belief envelope contains a `TestifiedAbout` claim from the witness on the topic, OR the witness is in the agent's `Knows` graph for that topic. No global witness query.
4. **Container anchoring over access rights.** `InspectContainer` candidates emit only when the agent has a believed access right (owner, holder, controller, or office authority) on the container. No global container scan.
5. **Per-agent threshold for "verify before act."** `EpistemicProfile.verification_threshold` controls when a low-confidence belief triggers an epistemic detour. A high-courage / low-doubt agent might act on `confidence ≥ pm(400)`; a magistrate verifying testimony before issuing a warrant might require `confidence ≥ pm(800)`.
6. **No teleporting truth.** Both actions update the agent's belief envelope through the existing perception and testimony paths. They produce events that other agents could perceive (overhearing the question, witnessing the open chest), preserving locality.
7. **Determinism.** Both new goal kinds and their candidate emitters iterate `BTreeMap`-stable. Action outcomes are deterministic functions of belief state and witness/container state.
8. **No silent privilege.** Neither action bypasses contention (queue substrate for popular witnesses), locality (the witness must be co-located or reachable via travel), or legality (inspecting a chest you don't own raises legal exposure per S138 if the agent's `LawAbidingProfile` weighs it).

## Non-Goals

- **`VerifyBelief`, `ConsultRecord`, `ScoutPlace`.** Deferred. `VerifyBelief` is a meta-goal whose decomposition produces `AskWitness`/`InspectContainer`/`ConsultRecord` instances — the right shape after HTN methods (Phase 12) land. `ConsultRecord` is partially covered by existing `consult_record_actions`; promoting it to a `GoalKind` adds little until S140's artifact lifecycle differentiates "actionable" vs "reference-only" records. `ScoutPlace` overlaps with S130's hypothesis-driven `ExploreLocation`.
- **Forced honesty.** Witnesses can lie, refuse, or misremember. `AskWitness` produces a belief update, not a truth update. Lie modeling is in scope for cross-witness contradiction goldens (Scenario G).
- **Cross-room shouting.** Both actions require co-location (FND-7). Long-distance witness inquiry routes through travel.
- **Multi-witness fan-out.** The agent asks one witness per `AskWitness` commit. Multi-witness compare-and-contrast is the planner's job through repeated emission, not a single-goal aggregation.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Both actions update belief through perception and testimony — the same carriers ordinary world events use. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | Both goals are agent-level expressions of "I do not know enough; I will find out." Confidence threshold is per-agent. |
| FND-17 (Surprise Comes From Violated Expectation) | `InspectContainer.expectation_id` explicitly couples the inspection to an existing `Expectation`; mismatch updates belief and surfaces the canonical robbery-report chain. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Verifications are agent decisions under bounded budget; an agent in critical stress will skip them, an agent with attention slack will indulge them. |
| FND-21 (Intentions Are Revisable Commitments) | Verification goals can be suspended if more urgent goals arise. |
| FND-22 (Agent Diversity Through Concrete Variation) | `EpistemicProfile.verification_threshold` makes per-agent variation explicit: paranoid magistrate vs trusting villager. |
| FND-23 (Roles, Offices, and Institutions Are World State) | `InspectContainer` anchors on access right, which derives from ownership/office authority. Office-driven inspection (a steward auditing a treasury) emerges from this same predicate. |
| FND-29 (Debuggability Is a Product Feature) | The chain "agent suspected stash empty → committed `InspectContainer` → observed mismatch → reported" is a sequence of inspectable goal commits and events. |

## Deliverables

### `worldwake-core::goal::GoalKind` extension

```rust
pub enum GoalKind {
    // existing variants preserved (Eat, Drink, Wash, Sleep, AcquireCommodity, ...)
    AskWitness {
        witness: EntityId,
        topic: TellTopic,                      // existing enum
    },
    InspectContainer {
        container: EntityId,
        expectation_id: Option<ExpectationId>, // S59
    },
}
```

### `EpistemicProfile` (new universal component)

```rust
pub struct EpistemicProfile {
    pub verification_threshold: Permille,         // confidence floor below which verification fires
    pub witness_recency_preference: Permille,     // weighting per-tick freshness vs first-hand-distance
    pub container_inspection_cooldown: Tick,      // min ticks between repeat inspections of the same container
}

impl Default for EpistemicProfile { /* … */ }
```

### `goal_dispatch_decl.rs` extensions

Two new `GoalDispatchKey` declarations, each with:
- `relevant_ops: &'static [PlannerOpKind]` — `&[Travel, AskWitness]` and `&[Travel, InspectContainer]` respectively (`PlannerOpKind` extended).
- `family: GoalFamilyPolicy` — both classified under a new `GoalFamilyPolicy::EpistemicSensing` family with `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::CriticalSurvival)`.
- `progress_barrier: ProgressBarrier::AnyMatchingTarget` for `AskWitness`, `ProgressBarrier::ExactTarget` for `InspectContainer`.

### `candidate_generation.rs` emitters

```rust
fn emit_ask_witness_candidates(
    agent: EntityId,
    belief_view: &RuntimeBeliefView,
    epistemic: &EpistemicProfile,
    out: &mut CandidateBatch,
) { /* … */ }

fn emit_inspect_container_candidates(
    agent: EntityId,
    belief_view: &RuntimeBeliefView,
    expectations: &ExpectationStore,
    epistemic: &EpistemicProfile,
    out: &mut CandidateBatch,
) { /* … */ }
```

The `AskWitness` emitter triggers when:
- The agent's belief envelope contains a `TestifiedAbout` claim or the witness is co-located AND the topic appears in the agent's belief envelope at `confidence < verification_threshold` OR `status == BeliefStatus::Stale | Disputed | Contradicted`.

The `InspectContainer` emitter triggers when:
- The agent has a believed access right on the container AND (an expectation about the container exists with `state == Active` near-deadline, OR the agent's belief about the container's contents has `confidence < verification_threshold`).

### Action handlers

`crates/worldwake-systems/src/ask_about_witness_actions.rs` and `crates/worldwake-systems/src/inspect_container_actions.rs`. Each registers an `ActionDef` with `BindingStrictness::ExactIdentity` (the witness or container is the named target).

The `ask_about_witness` action handler:
- Co-location precondition.
- Witness availability precondition (existing `AvailableForConversation` predicate).
- Effect: emits a `TellEvent` with the witness as speaker and the agent as listener; updates the agent's belief envelope on the topic via the existing testimony path.

The `inspect_container` action handler:
- Co-location precondition.
- Believed access-right precondition.
- Effect: emits a perception event (existing `ObservedContainerContents`); resolves any matched `Expectation` as fulfilled (contents match) or mismatched (existing `ExpectationMismatch` event tag).

### Component registration

- `EpistemicProfile` — register on `EntityKind::Agent`, universal default. `register_component_schema()` in `crates/worldwake-core/src/component_schema.rs`.

### `AgentDef` scenario contract

```rust
pub struct AgentDef {
    // existing fields
    pub epistemic_profile: Option<EpistemicProfileDef>,
}
```

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Both actions add information paths the world already supports. `AskWitness` produces a `TellEvent` along the existing testimony channel — overheard by co-located observers, recorded in event log. `InspectContainer` produces a perception event along the existing perception channel — visible to co-located observers (someone watching the agent open the chest can perceive both the act and the contents).
2. **Positive-feedback analysis.** Potential loop: low-confidence belief triggers verification → verification produces a fresh belief → fresh belief triggers more action → action exposes more low-confidence beliefs → more verifications. **Concrete dampener:** `verification_threshold` is per-agent; once the agent's belief crosses the threshold, no further verification fires. `container_inspection_cooldown` prevents re-inspection of the same container within N ticks. Suppression by `EpistemicSensing` family at critical-survival stress halts the loop under pressure.
3. **Concrete dampeners.**
   - Per-agent `verification_threshold` — once met, no further verification.
   - `container_inspection_cooldown` — TTL on repeat inspection.
   - Family suppression at critical stress.
   - `LearnedOpportunityMemory` (S109) damps repeated witness inquiry that previously yielded nothing.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: `EpistemicProfile` (per-agent), the `TellEvent`/`ObservedContainerContents` events emitted by the actions, the resulting belief-store updates.
   - **Derived read-model**: candidate emission per tick (transient).

## SystemFn Integration

No new `SystemFn`. Action execution flows through the existing scheduler. Candidate emission flows through the existing `agent_tick` candidate-generation phase.

## Component Registration

- `EpistemicProfile` — universal, `EntityKind::Agent`, default-applied per `docs/spec-drafting-rules.md` Section 5.

## Cross-System Interactions

- **AI → Sim**: emits the same testimony / perception events the existing tell/inspect machinery uses.
- **Sim → Core**: the resulting events feed the belief store via existing per-agent perception and testimony paths.
- **Sim → AI**: belief updates surface to the next agent_tick through the existing belief-view facade.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

`EpistemicProfile` is the per-agent profile. All three fields are `Permille` or `Tick` typed. Two agents with identical beliefs on the same topic will trigger or skip verification differently because their thresholds differ.

## Validation and Falsification

- **Golden coverage**: new `golden_epistemic_sensing.rs` with five scenarios:
  1. Stale-belief about witness testimony → expects `AskWitness` commit, belief update, threshold crossed.
  2. Outstanding `Expectation` near deadline on owned chest → expects `InspectContainer` commit, expectation resolution.
  3. FOUNDATIONS Scenario C (stored gold robbery): owner-believes-gold-present → suspicious-cue → `InspectContainer` → mismatch → robbery report. End-to-end across the existing `golden_simulation_gaps.rs` substrate; expand into S139's golden suite.
  4. FOUNDATIONS Scenario G (false rumor): agent receives contradicting testimony → `AskWitness` chain across two witnesses → contradiction surfaces in belief envelope.
  5. Critical-survival suppression: hungry agent skips verification when self-care class crosses threshold.
- **No regression**: existing 1440-tick survival goldens unaffected — epistemic emitters fire only when the verification threshold is breached, which never happens at default profiles in survival-baseline.

## Risks

- **Witness-availability fan-out.** A scenario with many co-located witnesses could emit many `AskWitness` candidates per tick. Mitigation: emitters cap per-tick emissions to `K` (default 3) per topic, ranked by witness recency × testimony freshness.
- **Inspection-cooldown tuning.** Too-short cooldown allows spam; too-long blocks legitimate repeat checks. Mitigation: default 60 ticks (≈ 1 hour at the existing tick scale); golden 2 above locks the boundary.
- **Belief-update collision.** A witness can be asked about a topic the agent's belief envelope holds with high confidence from another source. Mitigation: emitter only fires below the verification threshold; satisfaction predicate respects the existing belief-merge rules (S113 envelope merge), not naive overwrite.
