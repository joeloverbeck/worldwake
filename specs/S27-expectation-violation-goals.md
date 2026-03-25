**Status**: PENDING

# S27: Expectation-Violation Goals

## Summary

Add a new goal source family to the AI candidate generation pipeline: when an agent observes a mismatch between prior belief and current perception, emit a reactive investigation goal. This implements FOUNDATIONS Principle 15 ("Surprise Comes From Violated Expectation") as a first-class goal driver. Violation-driven reporting to co-located agents emerges from the existing `ShareBelief` pipeline without a dedicated goal variant.

## Phase

Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 3)

## Crate

- `worldwake-core` (new `GoalKind` variant, `ViolationKind` enum, `ViolationMemory` component, `ViolationDispositionProfile` component, `SocialObservationKind::WitnessedAbsence` variant)
- `worldwake-ai` (candidate generation via `emit_expectation_violation_candidates()`, planner ops, goal policy, ranking, feasibility dispatch, knowledge-path traces)
- `worldwake-systems` (new `investigate` action definition and handler)

## Dependencies

- S22 (intention frames -- investigation goals create Generic-domain `IntentionFrame` entries with assumption monitoring and patience tracking, consistent with the domain-agnostic commitment architecture)
- S23 (refined blocked intents -- investigation failures need compound-keyed blockers so a failed investigation at Place A does not suppress investigation at Place B)
- S25 (feasibility sketching -- `InvestigateMissing` needs a `FeasibilityHint` dispatch entry for cheap pre-search ordering)
- S28 (knowledge-path traces -- violation candidates use `emit_candidate_with_trace()` with `KnowledgePath` showing belief-observation contradiction provenance)

## FOUNDATIONS Alignment

- **P15** (Surprise comes from violated expectation): This IS the implementation of P15. Agents notice anomalies relative to prior expectation, commitment, claim, count, or routine. The agent discovers mismatch between `BelievedEntityState` and current perception.
- **P1** (Maximal emergence through local causality): Violation-reactive goals produce emergent investigation and reporting chains without authored quest logic. An agent finds gold missing, investigates, reports to a co-located authority -- all from generic systems.
- **P2** (No ungrounded triggers or probabilities): All parameters (investigation duration, memory retention, motive weights) come from per-agent `ViolationDispositionProfile`, never from hardcoded constants.
- **P3** (Concrete state over abstract scores): Violations are concrete mismatches (entity expected at place, not found; commodity expected available, quantity zero), not abstract "surprise scores."
- **P7** (Locality of motion, interaction, and communication): Violation detection uses only local observation -- the agent must be co-located at the violation site to notice the mismatch. Reports travel physically via the existing `ShareBelief`/Tell action.
- **P8** (Every action has preconditions, duration, cost, and occupancy): Investigation has profile-driven duration, preconditions (co-location, not incapacitated), and occupies the agent. It is interruptible.
- **P9** (Outcomes are granular and leave aftermath): Investigation commits a `SocialObservation` record (`WitnessedAbsence`) to the agent's belief store, making the investigation result a concrete, shareable artifact.
- **P12** (World state is not belief state): Violations are detected from the agent's own belief store (`AgentBeliefStore.known_entities`) compared against fresh perception, never from world truth.
- **P20** (Agent diversity through concrete variation): `ViolationDispositionProfile` provides per-agent investigation duration, memory retention, and motive weights. Different agents investigate differently.

## Motivation

The current AI can correct stale beliefs through passive re-observation (golden Scenario D pattern). But correction is silent -- the agent just replans with updated beliefs. It does not:

- Investigate why the expected resource or entity is missing
- Report the anomaly to a co-located agent (authority, owner, ally)
- Proactively seek a replacement supply through violation-driven urgency

These reactive behaviors are exactly where FOUNDATIONS expects emergence. The canonical regression scenario C (stored gold -> empty stash -> discovery -> robbery report) requires violation-reactive goals. Today's architecture handles steps 1-6 of Scenario C (belief mismatch detection and belief update) but not step 7 (trigger search, accusation, reporting, or other reactive behavior).

### What exists today

- `AgentBeliefStore` (`worldwake-core/src/belief.rs`) stores `known_entities: BTreeMap<EntityId, BelievedEntityState>` where each `BelievedEntityState` tracks `last_known_place`, `last_known_inventory`, `workstation_tag`, `resource_source`, `alive`, `wounds`, `last_known_courage`, `observed_tick`, and `source`.
- `PerAgentBeliefView` (`worldwake-sim/src/per_agent_belief_view.rs`) provides `GoalBeliefView` methods including `known_entity_beliefs()` which returns all `(EntityId, BelievedEntityState)` pairs, `entities_at(place)` which returns currently observed entities at a place, and `is_dead(entity)`.
- Candidate generation (`worldwake-ai/src/candidate_generation.rs`) has six `emit_*` families: need, production, enterprise, combat, social, political. Each produces `GroundedGoal` entries keyed by `GoalKey`. Since S28, all emitters use `emit_candidate_with_trace()` with `EvidenceTrace` and `KnowledgePath` for belief provenance.
- `GoalKind` (`worldwake-core/src/goal.rs`) has 17 variants. `GoalKindTag` (`worldwake-ai/src/goal_model.rs`) mirrors these for planner dispatch. `GoalPriorityClass` has five levels: `Background`, `Low`, `Medium`, `High`, `Critical`.
- `ShareBelief { listener, subject }` already exists as a goal for proactive information sharing via the Tell action.
- `IntentionFrame` (S22) provides domain-agnostic multi-tick commitment tracking with assumption monitoring, patience limits, and exhaustion-to-blocker integration.
- `BlockedIntentMemory` (S23) uses compound-keyed `BlockerKey` (goal + place + target + action_def) for place-scoped blocker records.
- `FeasibilityHint` (S25) provides cheap pre-search ordering (Likely/Uncertain/Unlikely) with per-`GoalKindTag` dispatch.
- `DecisionContext` provides shared pressure state (`max_self_care_class`, `danger_class`) for suppression evaluation.

### What is missing

No candidate generation function compares prior beliefs against current perception to detect violations. No `GoalKind` variant represents investigating a missing entity. No memory prevents repeated violation goals for the same already-noticed mismatch. No per-agent profile governs investigation behavior.

## Design

### Violation Detection

Each tick, during candidate generation (not as a separate system), compare the agent's `known_entity_beliefs()` against current perception for entities at the agent's current location. A violation occurs when:

1. **Entity expected present, now absent**: Agent previously believed entity E was at place P (via `BelievedEntityState.last_known_place == Some(P)`). Agent is now at P. `GoalBeliefView::entities_at(P)` does not include E, and E is not in-transit. -> `ViolationKind::EntityMissing { entity: E, expected_place: P }`

2. **Supply expected available, now depleted**: Agent previously believed commodity C was available at place P (via `BelievedEntityState.last_known_inventory` for a source entity at P, quantity > 0). Agent is now at P and observed quantity is 0. -> `ViolationKind::SupplyDepleted { commodity: C, source: <source_entity>, place: P }`

3. **Entity expected alive, now dead**: Agent previously believed entity E was alive (`BelievedEntityState.alive == true`). Agent now observes E is dead (`GoalBeliefView::is_dead(E) == true`) at the same location. -> `ViolationKind::EntityDead { entity: E }`

Detection is strictly local (P7) -- the agent must be co-located with the violated expectation. Detection runs inside `emit_expectation_violation_candidates()`, not as a separate system pass, keeping the architecture consistent with the existing `emit_*` pattern.

#### Violation-to-Goal Mapping

Not all violation kinds emit investigation goals:

- `EntityMissing` -> emits `InvestigateMissing { place }` (the entity is gone; investigation may reveal what happened)
- `SupplyDepleted` -> emits `InvestigateMissing { place }` (the supply was expected available; investigation confirms depletion)
- `EntityDead` -> records in `ViolationMemory` ONLY; does NOT emit `InvestigateMissing` (the corpse IS the evidence; existing `BuryCorpse`, `ShareBelief`, and `EngageHostile` pipelines handle death reactions through normal candidate generation)

#### Detection Timing

Perception runs before candidate generation each tick. Perception updates beliefs for entities the agent currently OBSERVES. Stale beliefs (entity believed at place P but not observed at P) remain in `known_entities` with their old `observed_tick`. The detection function compares these stale beliefs against `entities_at(current_place)` from fresh perception. This timing is what makes violation detection work: the old belief persists until contradicted, and the fresh observation provides the contradiction.

#### Self-Caused Depletion

No explicit exclusion code is needed. When an agent consumes a resource through its own action, the action's commit handler updates the agent's belief store immediately (the agent observed itself consuming). The agent's belief about the resource already matches post-action reality. Since no belief-perception mismatch exists, no violation is detected. This is the natural consequence of the belief-first architecture (P12) -- not a special case.

**Exclusions** (requiring explicit checks):
- The agent has no prior belief about the entity at this place (no expectation = no surprise)
- The entity is in-transit on a travel edge (temporarily absent, not missing)

### New Types in worldwake-core

#### ViolationKind

```rust
/// A detected mismatch between prior belief and current local observation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ViolationKind {
    /// Agent believed entity was at this place; entity is absent on observation.
    EntityMissing {
        entity: EntityId,
        expected_place: EntityId,
    },
    /// Agent believed commodity was available at a source here; source is depleted.
    SupplyDepleted {
        commodity: CommodityKind,
        source: EntityId,
        place: EntityId,
    },
    /// Agent believed entity was alive; entity is now dead.
    EntityDead {
        entity: EntityId,
    },
}
```

Lives in a new `worldwake-core/src/violation.rs` module. Must derive `Clone`, `Debug`, `Eq`, `Ord`, `PartialEq`, `PartialOrd`, `Serialize`, `Deserialize` for deterministic storage in `BTreeMap`/`BTreeSet`.

#### ViolationMemory Component

```rust
/// Records detected violations to prevent repeated reactive goal generation
/// for the same already-noticed mismatch.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ViolationMemory {
    pub violations: Vec<RecordedViolation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordedViolation {
    pub kind: ViolationKind,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
}
```

Registered as an Agent-only component in `component_schema.rs`. Follows the same pattern as `BlockedIntentMemory`: a `Vec` with expiry-based retention and per-kind deduplication.

Methods:
- `is_recorded(&self, kind: &ViolationKind, current_tick: Tick) -> bool`: Returns true if an unexpired record exists for this violation kind.
- `record(&mut self, kind: ViolationKind, observed_tick: Tick, ttl: u32)`: Records a violation, replacing any existing entry for the same kind. TTL is in ticks.
- `expire(&mut self, current_tick: Tick)`: Removes expired entries.

The TTL value comes from the agent's `ViolationDispositionProfile.violation_memory_retention_ticks`. It is long enough to prevent thrashing but short enough that the agent will re-notice if the violation persists after investigating.

#### ViolationDispositionProfile Component

```rust
/// Per-agent parameters governing investigation behavior.
/// Enables agent diversity (P20) for violation response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ViolationDispositionProfile {
    /// Duration in ticks for the investigate action. Per-agent curiosity/thoroughness.
    pub investigation_duration_ticks: NonZeroU32,
    /// How many ticks before a recorded violation expires from memory.
    pub violation_memory_retention_ticks: u32,
    /// Base motive weight for investigation goals (like social_weight, care_weight).
    pub investigation_motive_weight: Permille,
    /// Additional motive when the agent owns the missing entity.
    pub ownership_motive_bonus: Permille,
}
```

Registered as an Agent-only component in `component_schema.rs`. Follows the same pattern as `TellProfile`, `IntentionDispositionProfile`, and `UtilityProfile`: a per-agent profile struct that governs domain-specific behavior.

Default baseline values (used when constructing prototype agents):
- `investigation_duration_ticks`: 3
- `violation_memory_retention_ticks`: 50
- `investigation_motive_weight`: Permille(500)
- `ownership_motive_bonus`: Permille(200)

Different agents can vary these parameters to produce diversity: a curious merchant might have higher investigation motive and longer duration; a busy laborer might have lower motive and shorter retention.

#### SocialObservationKind Extension

```rust
pub enum SocialObservationKind {
    // ... existing variants ...
    /// Agent confirmed the absence of an expected entity at a location through investigation.
    WitnessedAbsence,
}
```

New variant in `worldwake-core/src/belief.rs`. Records that the agent has confirmed (through deliberate investigation, not just passive observation) that an expected entity or resource is absent. The `subjects` field of `SocialObservation` carries `(missing_entity, place)`. This record is shareable via the Tell pipeline, making investigation results propagate through the social system.

### New GoalKind Variant

```rust
pub enum GoalKind {
    // ... existing 17 variants ...

    /// Investigate why an expected entity or resource is missing at a location.
    /// The agent travels to (or is already at) the place and spends time
    /// searching for evidence of what happened.
    InvestigateMissing {
        place: EntityId,
    },
}
```

**Design decision -- single new variant, not two**: The initial design considered both `InvestigateMissing` and `ReportAnomaly`. After reviewing the codebase, `ReportAnomaly` is unnecessary because:

1. The existing `ShareBelief { listener, subject }` goal already covers proactive information sharing via the Tell action. When an agent detects a violation, their updated belief state (entity missing, supply depleted) is exactly the kind of information `ShareBelief` is designed to propagate.
2. Adding a separate `ReportAnomaly` goal would duplicate the Tell action pipeline and require a parallel planner ops path for no additional emergent behavior.
3. The social candidate generation (`emit_social_candidates`) already evaluates what relayable information the agent has. After a violation updates the agent's beliefs, the social pipeline will naturally consider sharing that information with co-located listeners.

Therefore: S27 adds only `InvestigateMissing`. Violation-driven reporting emerges from the existing `ShareBelief` pipeline once the agent's beliefs are updated by the violation detection.

#### GoalKey Extraction

```rust
GoalKind::InvestigateMissing { place } => GoalKey {
    kind: GoalKind::InvestigateMissing { place },
    commodity: None,
    entity: None,
    place: Some(place),
},
```

This keys investigation goals by place, so the agent can have at most one investigation goal per place (preventing duplicates for multiple missing entities at the same location).

#### GoalKindTag Addition

```rust
pub enum GoalKindTag {
    // ... existing 17 variants ...
    InvestigateMissing,
}
```

### Candidate Generation

New function `emit_expectation_violation_candidates()` in `candidate_generation.rs`, called from `generate_candidates_with_travel_horizon()` alongside the existing six `emit_*` calls:

```rust
emit_expectation_violation_candidates(&mut candidates, &mut diagnostics, &ctx);
```

This is the seventh `emit_*` family. It follows the same `emit_candidate_with_trace()` pattern established by S28 for knowledge-path provenance.

Algorithm:
1. Get the agent's current place from `ctx.place`. If `None` (agent in transit), return early.
2. Read the agent's `ViolationDispositionProfile` via `ctx.view`. If absent, return early (agent has no investigation behavior).
3. Read the agent's `ViolationMemory` via `ctx.view`.
4. Get the agent's known entity beliefs via `ctx.view.known_entity_beliefs(ctx.agent)`.
5. Build the set of currently observed entities at the agent's place: `let observed = ctx.view.entities_at(current_place)`.
6. For each `(entity_id, believed_state)` in the belief store:
   a. Skip if `entity_id == ctx.agent` (self).
   b. If `believed_state.last_known_place == Some(current_place)`:
      - If entity is NOT in `observed` and NOT in-transit: detect `ViolationKind::EntityMissing { entity: entity_id, expected_place: current_place }`.
      - If entity IS in `observed` and `believed_state.alive == true` but `ctx.view.is_dead(entity_id)`: detect `ViolationKind::EntityDead { entity: entity_id }`. Record in ViolationMemory but do NOT emit a goal (existing pipelines handle death reactions).
   c. If `believed_state.resource_source.is_some()` and `believed_state.last_known_place == Some(current_place)`:
      - For each `(commodity, believed_qty)` in `believed_state.last_known_inventory` where `believed_qty > Quantity(0)`:
        - If `ctx.view.commodity_quantity(entity_id, commodity) == Quantity(0)`: detect `ViolationKind::SupplyDepleted { commodity, source: entity_id, place: current_place }`.
7. For each detected violation that should emit a goal (EntityMissing, SupplyDepleted):
   a. Check `ViolationMemory::is_recorded()` -- skip if already recorded and unexpired.
   b. Record the violation in `ViolationMemory` with TTL from `ViolationDispositionProfile.violation_memory_retention_ticks`.
   c. Check `BlockedIntentMemory::is_blocked_for_search()` for the corresponding `GoalKey` -- skip if blocked (S23 compound keying ensures this is place-scoped).
   d. Emit via `emit_candidate_with_trace()` with:
      - `GoalKind::InvestigateMissing { place: current_place }`
      - `Evidence { entities: {entity_id}, places: {current_place} }`
      - `EvidenceTrace` with `KnowledgePath` showing: "prior belief (entity at place, observed at tick T) contradicted by current local observation"

### Goal Policy

`InvestigateMissing` goal policy in `goal_policy.rs`:

- **Suppression**: `WhenStressedAtOrAbove(GoalPriorityClass::High)` -- investigation is suppressed when the agent has critical survival or danger needs. Consistent with enterprise and social goals.
- **Penalty interrupt eligibility**: `Never` -- investigation is a short action and does not warrant penalty interruption.
- **Free interrupt role**: `FreeInterruptRole::Normal` -- investigation follows standard interrupt rules. It is not reactive self-care (`Reactive`) nor opportunistic looting (`Opportunistic`).

### Ranking

In `ranking.rs`, `InvestigateMissing` goals receive:

- **Priority class**: `GoalPriorityClass::Low` -- below survival needs (Critical/High), below combat (High/Medium), above Background enterprise goals.
- **Motive score**: Derived from the agent's `ViolationDispositionProfile`:
  - Base motive: `profile.investigation_motive_weight.as_u32()`.
  - Ownership bonus: if the agent owns the missing entity (via `ctx.view.believed_owner_of(entity) == Some(agent)`), add `profile.ownership_motive_bonus.as_u32()`.
  - The evidence entities in the `GroundedGoal` identify which entities triggered the violation for ownership lookup.

### Feasibility Hint (S25 Integration)

Add `InvestigateMissing` dispatch to the feasibility hint system in `feasibility.rs`:

- **Likely**: Agent's `effective_place` matches the investigation place (already co-located, can start immediately).
- **Uncertain**: Agent is not at the investigation place (needs travel; feasibility depends on route).
- **Unlikely**: `BlockedIntentMemory` has an unexpired entry matching the `GoalKey` for this investigation (recent failure at this place).

### Planner Ops

`InvestigateMissing` maps to a new `PlannerOpKind::Investigate`. The planner op semantics:

- **Relevant action**: The `investigate` action definition (see Action Definitions below).
- **Terminal condition**: The agent is at the investigation place and the investigate action can start.
- **Barriers**: Travel to the investigation place is a progress barrier (same pattern as other place-targeted goals).
- **Goal satisfaction**: Satisfied after the investigate action completes (the agent's beliefs are updated and `SocialObservation` is recorded).
- **`may_appear_mid_plan`**: `false` -- investigate is always the terminal action.
- **`is_materialization_barrier`**: `false` -- investigation does not block materialization.
- **`transition_kind`**: `GoalModelFallback` -- uses `GoalKind::apply_planner_step()`.

### IntentionFrame Integration (S22)

When the AI selects `InvestigateMissing` as the active goal, the decision runtime creates an `IntentionFrame`:

- **Domain**: `IntentionDomain::Generic`
- **Assumptions**: `[FrameAssumption::NoCriticalThreat]` -- investigation is abandoned if the agent faces danger.
- **Patience**: From `IntentionDispositionProfile.domain_patience[IntentionDomainTag::Generic]` (the agent's general patience for non-specialized multi-tick goals).

If the frame exhausts (patience runs out due to stalls), the frame's exhaustion-to-blocker integration (S22) writes a `BlockedIntent` with `BlockingFact::PatienceExhausted`, preventing re-emission of the same investigation goal for the blocker TTL.

### Invalidation Domains (S24)

No new `DirtySet` domain is needed for violation detection. The `emit_expectation_violation_candidates()` function depends on:

- `BELIEF_ENTITIES` domain (stale beliefs about entity locations) -- already exists.
- `POSITION` domain (agent's current place) -- already exists.

When either domain is marked dirty, the AI pipeline re-runs candidate generation including violation detection. This is the existing behavior for all `emit_*` families.

### Action Definitions

New action definition `investigate` in `worldwake-systems`:

- **Domain**: `ActionDomain::Generic` (not specific to any system domain)
- **Duration**: Profile-driven via `ViolationDispositionProfile.investigation_duration_ticks`. The action handler reads the agent's profile at start time and sets duration accordingly. If the profile is absent, falls back to `NonZeroU32::new(3).unwrap()`.
- **Preconditions**: Agent must be at the investigation place. Agent must not be incapacitated.
- **Interruptibility**: `FreelyInterruptible` -- investigation can be interrupted without penalty by higher-priority goals.
- **Visibility**: `SamePlace` -- co-located agents can observe the investigation.
- **Causal event tags**: `BTreeSet::from([EventTag::Discovery])`
- **Handler on start**: No-op (standard start).
- **Handler on tick**: Continue (standard tick progression).
- **Handler on commit**:
  1. Record the investigation event in the event log with `EventTag::Discovery`.
  2. Update `ViolationMemory` to extend the TTL for the investigated violation (prevents immediate re-investigation). The new TTL comes from the agent's `ViolationDispositionProfile.violation_memory_retention_ticks`.
  3. Record a `SocialObservation { kind: WitnessedAbsence, subjects: (missing_entity, place), place: current_place, observed_tick: current_tick, source: PerceptionSource::DirectObservation }` in the agent's `AgentBeliefStore`. This makes the investigation result a concrete, shareable belief artifact (P9). Co-located agents or future Tell actions can propagate this information.
  4. Future extensions (E17 crime system) can add evidence discovery as investigation outcomes -- tracks, broken containers, witness identification. For S27, investigation confirms the absence and records the confirmed observation.
- **Handler on abort**: No-op (investigation interrupted, no artifact produced).

### Integration with Existing Social Pipeline

After violation detection updates the agent's beliefs, the existing `emit_social_candidates()` in `candidate_generation.rs` will naturally consider the updated belief state when evaluating what to share with co-located listeners. If the agent has a `TellProfile` and a co-located listener exists, the social pipeline may emit `ShareBelief` goals for the newly observed facts (entity missing, supply depleted). This achieves the "report anomaly" behavior without a dedicated goal variant.

After investigation completes, the `WitnessedAbsence` social observation provides additional shareable content. The `listener_aware_relayable_subjects()` function already filters subjects by what the listener does not yet know. A freshly observed violation (entity gone, supply depleted) qualifies as new information worth sharing.

## Tickets

### S27-001: Define ViolationKind, ViolationMemory, and ViolationDispositionProfile in worldwake-core

- Add `violation.rs` module to `worldwake-core` with `ViolationKind` enum and `RecordedViolation` struct.
- Add `ViolationMemory` component with `is_recorded()`, `record()`, `expire()` methods.
- Add `ViolationDispositionProfile` struct with `investigation_duration_ticks`, `violation_memory_retention_ticks`, `investigation_motive_weight`, `ownership_motive_bonus` fields.
- Add `SocialObservationKind::WitnessedAbsence` variant to `worldwake-core/src/belief.rs`.
- Register `ViolationMemory` and `ViolationDispositionProfile` as Agent-only components in `component_schema.rs` and `component_tables.rs`.
- Derive `Clone`, `Debug`, `Eq`, `Ord`, `PartialEq`, `PartialOrd`, `Serialize`, `Deserialize` on `ViolationKind`.
- Focused unit tests: recording, deduplication by kind, TTL expiry with profile-driven retention, profile default values.
- Verify: `cargo test -p worldwake-core`, `cargo clippy -p worldwake-core`.

### S27-002: Add InvestigateMissing GoalKind variant and planner support

- Add `GoalKind::InvestigateMissing { place: EntityId }` to `worldwake-core/src/goal.rs`.
- Update `GoalKey::from(GoalKind)` to extract `place` field.
- Add `GoalKindTag::InvestigateMissing` to `worldwake-ai/src/goal_model.rs`.
- Implement `GoalKindPlannerExt` for `InvestigateMissing`: `goal_kind_tag()`, `relevant_op_kinds()`, `relevant_observed_commodities()`, `is_satisfied()`, `goal_relevant_places()`, `prerequisite_places()`, `build_payload_override()`, `apply_planner_step()`, `is_progress_barrier()`, `matches_binding()`.
- Add `PlannerOpKind::Investigate` with semantics (`may_appear_mid_plan: false`, `is_materialization_barrier: false`, `transition_kind: GoalModelFallback`).
- Add goal policy for `InvestigateMissing` in `goal_policy.rs` (suppression: `WhenStressedAtOrAbove(High)`, penalty interrupt: `Never`, free interrupt: `FreeInterruptRole::Normal`).
- Add ranking logic for `InvestigateMissing` in `ranking.rs` (priority class: `Low`, motive from `ViolationDispositionProfile`).
- Add `FeasibilityHint` dispatch for `InvestigateMissing` in `feasibility.rs` (Likely if co-located, Uncertain if travel needed, Unlikely if blocked).
- Verify: `cargo build --workspace`, `cargo clippy --workspace`.

### S27-003: Implement emit_expectation_violation_candidates()

- New `emit_expectation_violation_candidates()` function in `candidate_generation.rs`.
- Detect `EntityMissing`, `SupplyDepleted`, `EntityDead` violations by comparing stale beliefs against current perception at agent's current location.
- `EntityMissing` and `SupplyDepleted` emit `InvestigateMissing` goals; `EntityDead` records in `ViolationMemory` only.
- Check `ViolationMemory` to skip already-recorded violations.
- Check `BlockedIntentMemory::is_blocked_for_search()` to skip blocked investigation goals.
- Use `emit_candidate_with_trace()` with `EvidenceTrace` and `KnowledgePath` showing belief-observation contradiction provenance (S28 integration).
- Read `ViolationDispositionProfile` for TTL when recording violations.
- Integrate into `generate_candidates_with_travel_horizon()` call chain as the seventh `emit_*` family.
- Focused unit tests: violation detection for each kind, ViolationMemory suppression, in-transit entity exclusion, EntityDead does not emit goal, profile-absent agent skipped.
- Verify: `cargo test -p worldwake-ai`, `cargo clippy -p worldwake-ai`.

### S27-004: Add investigate action definition and handler

- New `investigate` action definition in `worldwake-systems`.
- Duration: profile-driven via `ViolationDispositionProfile.investigation_duration_ticks`.
- Preconditions: agent at investigation place, not incapacitated.
- Interruptibility: `FreelyInterruptible`.
- Handler on commit: record `EventTag::Discovery` event, update `ViolationMemory` TTL, record `SocialObservation { kind: WitnessedAbsence, ... }` in agent's belief store.
- Register in `ActionDefRegistry` and `ActionHandlerRegistry`.
- Register affordance generation for investigate action in `affordance_query.rs`.
- Focused unit tests: action starts, completes, updates ViolationMemory, produces SocialObservation.
- Verify: `cargo test -p worldwake-systems`, `cargo clippy -p worldwake-systems`.

### S27-005: Golden test -- entity missing triggers investigation

- Scenario: Agent A believes entity E (an item lot or another agent) is at Place P (via prior observation seeded in belief store). E is moved to Place Q by an authoritative world mutation. A arrives at P, perception refresh does NOT update E's belief (E is not observed at P), candidate generation detects `EntityMissing`, emits `InvestigateMissing { place: P }`. A executes the investigate action. On completion, `SocialObservation(WitnessedAbsence)` is recorded in A's belief store.
- Proves: P15 (violated expectation triggers reactive goal), P7 (local observation only), P12 (belief vs world state separation), P9 (investigation produces shareable aftermath).
- Verification layer: golden E2E coverage -- agent's active action becomes `investigate`, ViolationMemory records the violation, SocialObservation is recorded on commit.
- Verify: `cargo test -p worldwake-ai --test golden_*`.

### S27-006: Golden test -- violation triggers ShareBelief to co-located listener

- Scenario: Agent A detects commodity depletion at a resource source. Co-located Agent B (with appropriate `TellProfile`) is present. After A's belief update from the violation, the social candidate pipeline (`emit_social_candidates`) emits `ShareBelief { listener: B, subject: <source_entity> }`. A executes Tell to B, propagating the violation information.
- Proves: P1 (emergent reporting chain from generic systems), P15 (violated expectation), P7 (local observation and physical information transfer).
- Verification layer: golden E2E coverage -- A's decision trace shows `ShareBelief` candidate generated after violation, B's belief store updated by Tell.
- Verify: `cargo test -p worldwake-ai --test golden_*`.

### S27-007: Workspace verification and documentation

- `cargo test --workspace` -- all pass.
- `cargo clippy --workspace` -- no new warnings.
- Update `docs/golden-e2e-coverage.md` with new violation-investigation and violation-report coverage entries.
- Update `docs/golden-e2e-scenarios.md` with scenario descriptions.

## FND-01 Section H Analysis

### Information-path analysis

Violations are detected through local perception: the agent must be co-located with the violation site (P7). The information path is:

1. **Prior observation** -> `AgentBeliefStore.known_entities` records `BelievedEntityState` with `last_known_place`, `last_known_inventory`, `alive`, etc.
2. **World change** -> Another agent or system moves/consumes/kills the entity through lawful state transitions.
3. **Agent arrives at location** -> `PerAgentBeliefView` perception refresh updates beliefs for observed entities. Entities NOT observed at the location retain their stale beliefs.
4. **Violation detection** -> `emit_expectation_violation_candidates()` compares stale belief (entity believed at P) against fresh observation (entity not in `entities_at(P)`).
5. **Investigation** -> Agent executes `investigate` action at the location, spending profile-driven time and confirming the absence. Commits `SocialObservation(WitnessedAbsence)` as shareable evidence.
6. **Report propagation** -> Existing `ShareBelief`/Tell pipeline physically transmits updated beliefs and investigation observations to co-located listeners.

No information travels without a physical carrier. The agent cannot detect violations at remote locations.

### Positive-feedback analysis

**Potential loop**: Violation -> investigation -> finding nothing -> frustration -> more investigation?

This loop does NOT amplify because:
- `ViolationMemory` records the violation after detection, preventing re-emission of the same `InvestigateMissing` goal for `violation_memory_retention_ticks` (profile-driven, default 50).
- The investigation action updates the agent's beliefs to match current reality. After investigation, the prior belief no longer mismatches perception, so no violation is detected even after ViolationMemory expires.
- The loop is: violation detected -> investigate -> beliefs updated -> no more violation. It terminates in one cycle.

**Potential loop**: Violation -> ShareBelief -> listener investigates -> shares back?

This loop does NOT amplify because:
- The listener's investigation updates their own beliefs to match reality. They now know the entity is gone -- sharing this back provides no new information.
- `ToldBeliefMemory` prevents re-telling the same fact to the same listener (E15c).

### Concrete dampeners

1. **ViolationMemory TTL (profile-driven)**: Prevents repeated violation goals for the same mismatch. The TTL comes from `ViolationDispositionProfile.violation_memory_retention_ticks`, not a hardcoded constant (P2). Physical analogy: the agent remembers they already noticed this problem.
2. **Belief update**: Once the agent's belief matches reality (entity confirmed absent), no further violation is detected. Physical analogy: you only notice something missing once; after that, you know it is gone.
3. **Investigation duration (profile-driven)**: The investigate action takes `investigation_duration_ticks` time (P8), preventing instant chain reactions. Per-agent variation (P20) means some agents investigate briefly, others thoroughly. Physical analogy: searching takes effort and occupies the agent.
4. **Goal suppression under stress**: Investigation is suppressed when the agent has High or Critical survival/danger needs (via `GoalFamilyPolicy.suppression`). Physical analogy: you do not investigate a missing bread loaf while being attacked.
5. **Low priority class**: Investigation ranks below survival, combat, and critical needs, ensuring it does not crowd out essential behavior.
6. **IntentionFrame exhaustion (S22)**: If the investigation stalls repeatedly (patience exhausted), the frame writes a `BlockedIntent` that prevents re-emission for the blocker TTL. Physical analogy: the agent gives up on a fruitless search.

### Stored state vs. derived read-model list

**Stored (authoritative)**:
- `ViolationMemory` component on Agent entities -- records which violations the agent has already noticed, with profile-driven TTL.
- `ViolationDispositionProfile` component on Agent entities -- per-agent investigation behavior parameters.
- `ViolationKind` values within `ViolationMemory.violations` -- the specific mismatch records.
- `SocialObservation(WitnessedAbsence)` entries in `AgentBeliefStore.social_observations` -- investigation results shareable via Tell.
- Event log entries for investigation actions (`EventTag::Discovery`) -- causal record of when and where the agent investigated.

**Derived (transient, recomputable)**:
- Violation detection result from `emit_expectation_violation_candidates()` -- derived each tick by comparing stale beliefs against current perception at current location.
- `GroundedGoal` candidates for `InvestigateMissing` -- derived from violations and filtered by ViolationMemory/BlockedIntentMemory.
- `FeasibilityHint` for `InvestigateMissing` -- derived from agent position and blocker memory.

## Verification

1. `cargo test --workspace` -- all existing and new tests pass.
2. `cargo clippy --workspace` -- no new warnings.
3. Golden test S27-005 proves: violation detection -> investigation -> belief update -> SocialObservation chain (P15, P7, P9, P12).
4. Golden test S27-006 proves: violation -> Tell -> information propagation chain (P1, P15, P7).
5. ViolationMemory prevents repeated goals for same mismatch (focused unit test in S27-001).
6. ViolationDispositionProfile drives all numeric parameters (focused unit test in S27-001, integration test in S27-004).
7. EntityDead records in ViolationMemory but does not emit InvestigateMissing (focused unit test in S27-003).
8. Investigation produces SocialObservation(WitnessedAbsence) on commit (focused unit test in S27-004).
9. `docs/golden-e2e-coverage.md` updated with new violation coverage entries.
