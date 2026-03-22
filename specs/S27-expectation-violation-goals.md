**Status**: PENDING

# S27: Expectation-Violation Goals

## Summary

Add a new goal source family to the AI candidate generation pipeline: when an agent observes a mismatch between prior belief and current perception, emit reactive goals (investigate missing entity, report anomaly to co-located agent). This implements FOUNDATIONS Principle 15 ("Surprise Comes From Violated Expectation") as a first-class goal driver.

## Phase

Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 3)

## Crate

- `worldwake-core` (new `GoalKind` variants, `ViolationKind` enum, `ViolationMemory` component)
- `worldwake-ai` (candidate generation via `emit_expectation_violation_candidates()`, planner ops, goal policy, ranking)

## Dependencies

- S22 (intention frames -- provide the "expected" baseline for violation detection; `IntentionFrame.assumptions` captures what the agent expected to find)
- S23 (refined blocked intents -- investigation failures need compound-keyed blockers so a failed investigation at Place A does not suppress investigation at Place B)

## FOUNDATIONS Alignment

- **P15** (Surprise comes from violated expectation): This IS the implementation of P15. Agents notice anomalies relative to prior expectation, commitment, claim, count, or routine. The agent discovers mismatch between `BelievedEntityState` and current perception.
- **P1** (Maximal emergence through local causality): Violation-reactive goals produce emergent investigation and reporting chains without authored quest logic. An agent finds gold missing, investigates, reports to a co-located authority -- all from generic systems.
- **P7** (Locality of motion, interaction, and communication): Violation detection uses only local observation -- the agent must be co-located at the violation site to notice the mismatch. Reports travel physically via the existing `ShareBelief`/Tell action.
- **P12** (World state is not belief state): Violations are detected from the agent's own belief store (`AgentBeliefStore.known_entities`) compared against fresh perception, never from world truth.
- **P3** (Concrete state over abstract scores): Violations are concrete mismatches (entity expected at place, not found; commodity expected available, quantity zero), not abstract "surprise scores."

## Motivation

The current AI can correct stale beliefs through passive re-observation (golden Scenario D pattern). But correction is silent -- the agent just replans with updated beliefs. It does not:

- Investigate why the expected resource or entity is missing
- Report the anomaly to a co-located agent (authority, owner, ally)
- Proactively seek a replacement supply through violation-driven urgency

These reactive behaviors are exactly where FOUNDATIONS expects emergence. The canonical regression scenario C (stored gold -> empty stash -> discovery -> robbery report) requires violation-reactive goals. Today's architecture handles steps 1-6 of Scenario C (belief mismatch detection and belief update) but not step 7 (trigger search, accusation, reporting, or other reactive behavior).

### What exists today

- `AgentBeliefStore` (`worldwake-core/src/belief.rs`) stores `known_entities: BTreeMap<EntityId, BelievedEntityState>` where each `BelievedEntityState` tracks `last_known_place`, `last_known_inventory`, `alive`, `observed_tick`, and `source`.
- `PerAgentBeliefView` (`worldwake-sim/src/per_agent_belief_view.rs`) provides `GoalBeliefView` methods including `known_entity_beliefs()` which returns all `(EntityId, BelievedEntityState)` pairs.
- Candidate generation (`worldwake-ai/src/candidate_generation.rs`) has six `emit_*` families: need, production, enterprise, combat, social, political. Each produces `GroundedGoal` entries keyed by `GoalKey`.
- `GoalKind` (`worldwake-core/src/goal.rs`) has 17 variants. `GoalKindTag` (`worldwake-ai/src/goal_model.rs`) mirrors these for planner dispatch. `GoalPriorityClass` has five levels: `Background`, `Low`, `Medium`, `High`, `Critical`.
- `ShareBelief { listener, subject }` already exists as a goal for proactive information sharing via the Tell action. ReportAnomaly can reuse this mechanism rather than creating a parallel Tell path.

### What is missing

No candidate generation function compares prior beliefs against current perception to detect violations. No `GoalKind` variant represents investigating a missing entity or reporting an observed anomaly. No memory prevents repeated violation goals for the same already-noticed mismatch.

## Design

### Violation Detection

Each tick, during candidate generation (not as a separate system), compare the agent's `known_entity_beliefs()` against current perception for entities at the agent's current location. A violation occurs when:

1. **Entity expected present, now absent**: Agent previously believed entity E was at place P (via `BelievedEntityState.last_known_place == Some(P)`). Agent is now at P. `GoalBeliefView::entities_at(P)` does not include E, and E is not in-transit. -> `ViolationKind::EntityMissing { entity: E, expected_place: P }`

2. **Supply expected available, now depleted**: Agent previously believed commodity C was available at place P (via `BelievedEntityState.last_known_inventory` for a source entity at P, quantity > 0). Agent is now at P and observed quantity is 0. -> `ViolationKind::SupplyDepleted { commodity: C, source: <source_entity>, place: P }`

3. **Entity expected alive, now dead**: Agent previously believed entity E was alive (`BelievedEntityState.alive == true`). Agent now observes E is dead (`GoalBeliefView::is_dead(E) == true`) at the same location. -> `ViolationKind::EntityDead { entity: E }`

Detection is strictly local (P7) -- the agent must be co-located with the violated expectation. Detection runs inside `emit_expectation_violation_candidates()`, not as a separate system pass, keeping the architecture consistent with the existing `emit_*` pattern.

**Exclusions**: The following are NOT violations:
- The agent itself consumed the resource (self-caused depletion is expected)
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

Default TTL: 50 ticks. This is long enough to prevent thrashing but short enough that the agent will re-notice if the violation persists after investigating.

### New GoalKind Variants

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
GoalKind::InvestigateMissing { place } => (None, None, Some(place)),
```

This keys investigation goals by place, so the agent can have at most one investigation goal per place (preventing duplicates for multiple missing entities at the same location).

#### GoalKindTag Addition

```rust
pub enum GoalKindTag {
    // ... existing variants ...
    InvestigateMissing,
}
```

### Candidate Generation

New function `emit_expectation_violation_candidates()` in `candidate_generation.rs`, called from `generate_candidates_with_travel_horizon()` alongside the existing six `emit_*` calls:

```rust
emit_expectation_violation_candidates(&mut candidates, &mut diagnostics, &ctx);
```

Algorithm:
1. Get the agent's current place from `ctx.place`. If `None` (agent in transit), return early.
2. Get the agent's known entity beliefs via `ctx.view.known_entity_beliefs(ctx.agent)`.
3. For each `(entity_id, believed_state)` in the belief store:
   a. If `believed_state.last_known_place == Some(current_place)`:
      - Check if entity is currently observed at `current_place` via `ctx.view.entities_at(current_place)`.
      - If entity is NOT present and NOT the agent itself: detect `EntityMissing`.
      - If entity IS present and `believed_state.alive == true` but `ctx.view.is_dead(entity_id)`: detect `EntityDead`.
   b. If entity has `resource_source.is_some()` and `believed_state.last_known_place == Some(current_place)`:
      - Check current commodity quantity at the source.
      - If previously believed quantity > 0 and current quantity == 0: detect `SupplyDepleted`.
4. For each detected violation:
   a. Check `ViolationMemory::is_recorded()` -- skip if already recorded and unexpired.
   b. Check `BlockedIntentMemory::is_blocked()` for the corresponding `GoalKey` -- skip if blocked (S23 compound keying ensures this is place-scoped).
   c. Emit `GroundedGoal` with `GoalKind::InvestigateMissing { place: current_place }`, evidence entities from the violation, evidence places from the violation site.

### Goal Policy

`InvestigateMissing` goal policy in `goal_policy.rs`:

- **Suppression**: `WhenStressedAtOrAbove(GoalPriorityClass::High)` -- investigation is suppressed when the agent has critical survival or danger needs. Consistent with enterprise and social goals.
- **Penalty interrupt eligibility**: `Never` -- investigation is a short action and does not warrant penalty interruption.
- **Free interrupt role**: `true` -- investigation can be freely interrupted by higher-priority goals without penalty.

### Ranking

In `ranking.rs`, `InvestigateMissing` goals receive:

- **Priority class**: `GoalPriorityClass::Low` -- below survival needs (Critical/High), below combat (High/Medium), above Background enterprise goals.
- **Motive score**: Derived from the number of violations detected at the place and the relationship to missing entities. Base motive: `500`. Scaled up by ownership relationship to missing entities (agent owns the missing thing: `+200`). This is a concrete per-agent utility weight, not an abstract score.

### Planner Ops

`InvestigateMissing` maps to a new `PlannerOpKind::Investigate`. The planner op semantics:

- **Relevant action**: A new `investigate` action definition (see Action Definitions below).
- **Terminal condition**: The agent is at the investigation place and the investigate action can start.
- **Barriers**: Travel to the investigation place is a progress barrier (same pattern as other place-targeted goals).
- **Goal satisfaction**: Satisfied after the investigate action completes (the agent's beliefs are updated).

### Action Definitions

New action definition `investigate` in `worldwake-systems`:

- **Domain**: `ActionDomain::Generic` (not specific to any system domain)
- **Duration**: `DurationExpr::Fixed(3)` -- 3 ticks of searching. Short enough to not be a major commitment, long enough to have cost (P8).
- **Preconditions**: Agent must be at the investigation place. Agent must not be incapacitated.
- **Handler on completion**:
  1. Record the investigation in the event log with `EventTag::Investigation`.
  2. Update `ViolationMemory` to mark the violation as investigated (extends TTL to prevent immediate re-investigation).
  3. The agent's belief store is already updated by normal perception refresh (co-located observation). The investigation action provides additional time for the agent to process and notice further details.
  4. Future extensions (E17 crime system) can add evidence discovery as investigation outcomes -- tracks, broken containers, witness identification. For S27, investigation confirms the absence and updates the agent's beliefs with high-confidence fresh observation.

### Integration with Existing Social Pipeline

After violation detection updates the agent's beliefs, the existing `emit_social_candidates()` in `candidate_generation.rs` will naturally consider the updated belief state when evaluating what to share with co-located listeners. If the agent has a `TellProfile` and a co-located listener exists, the social pipeline may emit `ShareBelief` goals for the newly observed facts (entity missing, supply depleted). This achieves the "report anomaly" behavior without a dedicated goal variant.

The `listener_aware_relayable_subjects()` function already filters subjects by what the listener does not yet know. A freshly observed violation (entity gone, supply depleted) qualifies as new information worth sharing.

## Tickets

### S27-001: Define ViolationKind and ViolationMemory in worldwake-core

- Add `violation.rs` module to `worldwake-core` with `ViolationKind` enum and `RecordedViolation` struct.
- Add `ViolationMemory` component with `is_recorded()`, `record()`, `expire()` methods.
- Register `ViolationMemory` as an Agent-only component in `component_schema.rs` and `component_tables.rs`.
- Derive `Clone`, `Debug`, `Eq`, `Ord`, `PartialEq`, `PartialOrd`, `Serialize`, `Deserialize` on `ViolationKind`.
- Focused unit tests: recording, deduplication by kind, TTL expiry.
- Verify: `cargo test -p worldwake-core`, `cargo clippy -p worldwake-core`.

### S27-002: Add InvestigateMissing GoalKind variant and planner support

- Add `GoalKind::InvestigateMissing { place: EntityId }` to `worldwake-core/src/goal.rs`.
- Update `GoalKey::from(GoalKind)` to extract `place` field.
- Add `GoalKindTag::InvestigateMissing` to `worldwake-ai/src/goal_model.rs`.
- Implement `GoalKindPlannerExt` for `InvestigateMissing`: `goal_kind_tag()`, `relevant_op_kinds()`, `is_satisfied()`, `goal_relevant_places()`, `build_payload_override()`, `apply_planner_step()`, `is_progress_barrier()`.
- Add `PlannerOpKind::Investigate` with semantics (barriers, terminal, mid-plan viability).
- Add goal policy for `InvestigateMissing` in `goal_policy.rs` (suppression: `WhenStressedAtOrAbove(High)`, penalty interrupt: `Never`).
- Add ranking logic for `InvestigateMissing` in `ranking.rs` (priority class: `Low`, base motive: `500`).
- Verify: `cargo build --workspace`, `cargo clippy --workspace`.

### S27-003: Implement emit_expectation_violation_candidates()

- New `emit_expectation_violation_candidates()` function in `candidate_generation.rs`.
- Detect `EntityMissing`, `SupplyDepleted`, `EntityDead` violations by comparing prior beliefs to current perception at agent's current location.
- Check `ViolationMemory` to skip already-recorded violations.
- Check `BlockedIntentMemory` to skip blocked investigation goals.
- Emit `GroundedGoal` with `GoalKind::InvestigateMissing { place }` for detected violations.
- Integrate into `generate_candidates_with_travel_horizon()` call chain.
- Focused unit tests: violation detection for each kind, ViolationMemory suppression, self-caused depletion exclusion, in-transit entity exclusion.
- Verify: `cargo test -p worldwake-ai`, `cargo clippy -p worldwake-ai`.

### S27-004: Add investigate action definition and handler

- New `investigate` action definition in `worldwake-systems`.
- Duration: `DurationExpr::Fixed(3)`.
- Preconditions: agent at investigation place, not incapacitated.
- Handler: on completion, record investigation event in event log, update `ViolationMemory` TTL.
- Register in `ActionDefRegistry` and `ActionHandlerRegistry`.
- Register affordance generation for investigate action in `affordance_query.rs`.
- Focused unit tests: action starts, completes, updates ViolationMemory.
- Verify: `cargo test -p worldwake-systems`, `cargo clippy -p worldwake-systems`.

### S27-005: Golden test -- entity missing triggers investigation

- Scenario: Agent A believes entity E (an item lot or another agent) is at Place P (via prior observation seeded in belief store). E is moved to Place Q by an authoritative world mutation. A arrives at P, perception refresh updates beliefs, candidate generation detects `EntityMissing`, emits `InvestigateMissing { place: P }`. A executes the investigate action.
- Proves: P15 (violated expectation triggers reactive goal), P7 (local observation only), P12 (belief vs world state separation).
- Verification layer: golden E2E coverage -- agent's active action becomes `investigate`, ViolationMemory records the violation.
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

1. **Prior observation** -> `AgentBeliefStore.known_entities` records `BelievedEntityState` with `last_known_place`, `last_known_inventory`, `alive`.
2. **World change** -> Another agent or system moves/consumes/kills the entity through lawful state transitions.
3. **Agent arrives at location** -> `PerAgentBeliefView` perception refresh updates beliefs from local observation.
4. **Violation detection** -> `emit_expectation_violation_candidates()` compares prior belief against fresh perception.
5. **Investigation** -> Agent executes `investigate` action at the location, spending time and confirming the absence.
6. **Report propagation** -> Existing `ShareBelief`/Tell pipeline physically transmits updated beliefs to co-located listeners.

No information travels without a physical carrier. The agent cannot detect violations at remote locations.

### Positive-feedback analysis

**Potential loop**: Violation -> investigation -> finding nothing -> frustration -> more investigation?

This loop does NOT amplify because:
- `ViolationMemory` records the violation after detection, preventing re-emission of the same `InvestigateMissing` goal for 50 ticks.
- The investigation action updates the agent's beliefs to match current reality. After investigation, the prior belief no longer mismatches perception, so no violation is detected even after ViolationMemory expires.
- The loop is: violation detected -> investigate -> beliefs updated -> no more violation. It terminates in one cycle.

**Potential loop**: Violation -> ShareBelief -> listener investigates -> shares back?

This loop does NOT amplify because:
- The listener's investigation updates their own beliefs to match reality. They now know the entity is gone -- sharing this back provides no new information.
- `ToldBeliefMemory` prevents re-telling the same fact to the same listener.

### Concrete dampeners

1. **ViolationMemory TTL (50 ticks)**: Prevents repeated violation goals for the same mismatch. Physical analogy: the agent remembers they already noticed this problem.
2. **Belief update**: Once the agent's belief matches reality (entity confirmed absent), no further violation is detected. Physical analogy: you only notice something missing once; after that, you know it is gone.
3. **Investigation duration (3 ticks)**: The investigate action takes time (P8), preventing instant chain reactions. Physical analogy: searching takes effort and occupies the agent.
4. **Goal suppression under stress**: Investigation is suppressed when the agent has High or Critical survival/danger needs. Physical analogy: you do not investigate a missing bread loaf while being attacked.
5. **Low priority class**: Investigation ranks below survival, combat, and critical needs, ensuring it does not crowd out essential behavior.

### Stored state vs. derived read-model list

**Stored (authoritative)**:
- `ViolationMemory` component on Agent entities -- records which violations the agent has already noticed, with TTL.
- `ViolationKind` values within `ViolationMemory.violations` -- the specific mismatch records.
- Event log entries for investigation actions -- causal record of when and where the agent investigated.

**Derived (transient, recomputable)**:
- Violation detection result from `emit_expectation_violation_candidates()` -- derived each tick by comparing `known_entity_beliefs()` against current perception at current location.
- `GroundedGoal` candidates for `InvestigateMissing` -- derived from violations and filtered by ViolationMemory/BlockedIntentMemory.

## Verification

1. `cargo test --workspace` -- all existing and new tests pass.
2. `cargo clippy --workspace` -- no new warnings.
3. Golden test S27-005 proves: violation detection -> investigation -> belief update chain (P15, P7, P12).
4. Golden test S27-006 proves: violation -> Tell -> information propagation chain (P1, P15, P7).
5. ViolationMemory prevents repeated goals for same mismatch (focused unit test in S27-001).
6. Self-caused depletion excluded from violation detection (focused unit test in S27-003).
7. `docs/golden-e2e-coverage.md` updated with new violation coverage entries.
