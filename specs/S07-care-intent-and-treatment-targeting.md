**Status**: READY

# Care Intent and Treatment Targeting

## Summary
Replace the current split care model with one patient-anchored care goal family.

Today the architecture splits care across:
- `GoalKind::AcquireCommodity { purpose: Treatment }`, which knows the commodity but not the patient
- `GoalKind::Heal { target }`, which knows the patient but not the acquisition intent

That split is the wrong architecture. It causes one goal family to carry the resource need and another to carry the patient identity, which then leaks into candidate generation, ranking, search, and action legality.

The clean design is:
- one top-level care goal family anchored to a concrete patient: `GoalKind::TreatWounds { patient: EntityId }`
- planner semantics that let that goal acquire treatment commodities, produce them, move toward the patient, and apply treatment as needed
- no separate top-level generic treatment-acquisition goal
- no self-target prohibition for treatment if the action is otherwise legal
- belief-driven candidate generation using E14's `AgentBeliefStore` and `PerceptionSource`

## Why This Exists
The current architecture has four linked faults:

1. `AcquireCommodity(Treatment)` has no patient identity.
2. `Heal { target }` has patient identity but sits beside a separate treatment-acquisition goal instead of owning that procurement path.
3. Ranking for `AcquireCommodity(Treatment)` uses `treatment_pain()` which takes the max of actor's own pain and local patient pain — wrong for both self-care and third-party care, and does not use per-agent care sensitivity.
4. Combat treatment currently forbids self-targeting (`combat.rs:790-796`), so self-wounds and treatment procurement do not compose into a lawful self-care loop.

Those are not isolated bugs. They are one design problem:
- the architecture models treatment supply and treatment intent as separate goal families even though the simulation reason is "this patient needs care."

That violates the direction set by:
- Principle 3: concrete state over abstract scores
- Principle 8: actions have concrete preconditions, duration, cost, and occupancy
- Principle 18: practical reasoning must be explainable from belief and priorities
- Principle 20: agent diversity should come from concrete variation
- Principle 26: no backward compatibility
- Principle 27: debuggability is a product feature
- Principle 28: system specs must declare concrete causal hooks

## Why This Is A Spec, Not A Ticket
This work crosses architectural boundaries:
- `worldwake-core` goal identity and utility profile semantics
- `worldwake-ai` candidate generation, ranking, goal semantics, search, and diagnostics
- `worldwake-sim` action handler self-target enum
- `worldwake-systems` treatment action legality

It is too large and too structural for a ticket-first patch. The correct ownership is a formal spec.

## Phase
Phase 3: Information & Politics hardening. All dependencies are complete.

## Crates
- `worldwake-core`
- `worldwake-ai`
- `worldwake-sim`
- `worldwake-systems`

## Completed Dependencies
| Dependency | Status | What it provides |
|-----------|--------|-----------------|
| `archive/specs/E14-perception-beliefs.md` | Complete | `AgentBeliefStore`, `BelievedEntityState` with `wounds` field, `PerceptionSource` enum (`DirectObservation`, `Report`, `Rumor`, `Inference`), `PerAgentBeliefView` |
| `S02-goal-decision-policy-unification.md` | Complete | `GoalFamilyPolicy` in `goal_policy.rs`, exhaustive match over `GoalKind` variants, `SuppressionRule`/`PenaltyInterruptEligibility`/`FreeInterruptRole` |
| `S03-planner-target-identity-and-affordance-binding.md` | Complete | `matches_binding()` on `GoalKindPlannerExt` with exact patient binding for `Heal` — reusable for `TreatWounds` |

## Design Goals
1. Make care intent patient-anchored, not commodity-anchored.
2. Remove the split between treatment procurement and treatment application as top-level goal families.
3. Support both self-care and third-party care under one lawful model.
4. Keep all reasoning grounded in concrete belief state via E14's `AgentBeliefStore`.
5. Preserve deterministic planning and ranking behavior.
6. Support future non-medicine treatment methods without another redesign.
7. Remove obsolete goal variants and logic instead of layering wrappers beside them.

## Non-Goals
- Implementing full medical institutions, triage queues, hospitals, or diagnosis
- Replacing the entire planner
- Implementing non-medicine treatment methods (architecture must be extensible to them)
- Adding omniscient patient-discovery shortcuts
- Introducing backward-compatibility aliases for old goal variants
- Adding an `InvestigateReport` goal kind (future extension, not this spec)

## Resolved Design Questions

These were open in the original draft. User decisions recorded here:

| Question | Decision | Rationale |
|----------|----------|-----------|
| Should care thresholds reuse `DriveThresholds.pain` or become their own surface? | Reuse existing pain `ThresholdBand` | Avoids new profile surface; pain thresholds already calibrated for wound urgency |
| Should future non-medicine treatment methods join this goal family? | Architecture extensible, implement medicine only | Future methods add new `ActionDomain::Care` action defs and planner ops; `TreatWounds` goal and satisfaction remain stable |
| After E14, should indirect reports create care intent? | Only `PerceptionSource::DirectObservation` triggers `TreatWounds` | Respects Principle 7 (locality). Agents must independently travel to patient; direct observation upon arrival triggers care. No `InvestigateReport` goal in this spec. |
| How should ranking distinguish self-care from care-for-others? | New `care_weight: Permille` field in `UtilityProfile` | `pain_weight` governs self-pain urgency; `care_weight` governs third-party care urgency. Per-agent diversity via profile parameters (Principle 20). |

## Deliverables

### D01. Replace split care goals with `TreatWounds { patient: EntityId }`

**Files**: `crates/worldwake-core/src/goal.rs`

Remove:
- `GoalKind::Heal { target: EntityId }`
- `CommodityPurpose::Treatment` variant

Add:
```rust
GoalKind::TreatWounds { patient: EntityId }
```

Goal naming uses `TreatWounds` (not `Heal`) to disambiguate from `PlannerOpKind::Heal` which names the action op, not the goal.

Update `GoalKey::from(GoalKind)`:
- `GoalKind::TreatWounds { patient }` → `GoalKey { entity: Some(patient), commodity: None, place: None }`

Migration rule:
- do not preserve `CommodityPurpose::Treatment` as a parallel top-level path
- do not preserve `Heal { target }` beside the new goal family
- update all callers and tests directly

### D02. Care goal owns its supply path

**Files**: `crates/worldwake-ai/src/goal_model.rs`

`TREAT_WOUNDS_OPS`: `[Travel, Heal, Trade, QueueForFacilityUse, Craft]` (same op set as current `HEAL_OPS`).

The planner treats medicine acquisition/production as subordinate steps within the `TreatWounds` goal. No separate top-level treatment-acquisition goal exists.

### D03. Patient identity exact-bound (leveraging S03)

**Files**: `crates/worldwake-ai/src/goal_model.rs`

`matches_binding()` for `TreatWounds { patient }`:
- Terminal `Heal` op: `authoritative_targets.contains(patient)` — must match exact patient
- Auxiliary ops (Travel, Trade, QueueForFacilityUse, Craft): always pass

Identical structure to current `Heal` binding — rename to `TreatWounds`.

### D04. Treatment commodity capability remains item-defined

No new files. `CommodityKind::spec().treatment_profile` stays source of truth.

Future extensibility: new treatment methods add new `ActionDomain::Care` action defs that classify to new `PlannerOpKind` entries, and `TREAT_WOUNDS_OPS` expands. The `TreatWounds` goal family and satisfaction condition remain stable.

### D05. Self-care must be lawful

**Files**:
- `crates/worldwake-systems/src/combat.rs` (lines 790-796): Remove `target == instance.actor` check in `validate_heal_context()`
- `crates/worldwake-sim/src/action_handler.rs` (line 250): Remove `SelfTargetActionKind::Heal` variant, leaving single-variant enum with just `Attack`

Keep all other constraints in `validate_heal_context()`: Medicine required, co-location, actor alive/not incapacitated, target has wounds.

### D06. Add `care_weight` to UtilityProfile; split ranking semantics

**Files**:
- `crates/worldwake-core/src/utility_profile.rs`: Add `care_weight: Permille` field, default `Permille(200)` (low baseline — most agents prioritize self over others)
- `crates/worldwake-ai/src/ranking.rs`: New ranking logic for `TreatWounds`

Ranking for `TreatWounds { patient }`:
- **Self-care** (`patient == agent`):
  - Priority: `classify_band(self_pain, &thresholds.pain)`
  - Motive: `score_product(pain_weight, self_pain)`
- **Other-care** (`patient != agent`):
  - Priority: `classify_band(patient_pain, &thresholds.pain)`
  - Motive: `score_product(care_weight, patient_pain)`

Remove from `ranking.rs`:
- `treatment_pain()` function (lines 373-390)
- `treatment_score()` function (lines 392-403)
- `AcquireCommodity { purpose: Treatment }` branches in `priority_class()` (lines 152-159) and `motive_score()` (lines 254-257)

### D07. Belief-driven candidate generation

**Files**: `crates/worldwake-ai/src/candidate_generation.rs`

Replace `emit_heal_goals()` (lines 536-558) + `emit_treatment_candidates()` (lines 560-605) with single `emit_care_goals()`.

Logic:
1. **Self-care**: If agent believes self wounded (`view.has_wounds(agent)`), emit `TreatWounds { patient: agent }`.
2. **Third-party care**: Iterate `view.known_entity_beliefs(agent)`. For each entity with non-empty `wounds`:
   - If `source == PerceptionSource::DirectObservation`: emit `TreatWounds { patient }`
   - If `source` is `Report`/`Rumor`/`Inference`: **skip** (investigation-first; agent must independently travel and observe)

**No medicine gate**: Care intent forms even without medicine. The planner handles supply acquisition through Trade/Craft/Harvest ops. The current `emit_heal_goals()` early-returns when agent has zero medicine — this is removed.

Remove:
- `emit_heal_goals()` function
- `emit_treatment_candidates()` function
- `local_heal_targets()` function (which excluded self via `filter(|target| *target != agent)`)

Update `emit_combat_candidates()` (line 138-148) to call `emit_care_goals()` instead of `emit_treatment_candidates()` + `emit_heal_goals()`.

### D08. Patient-semantic goal satisfaction

**Files**: `crates/worldwake-ai/src/goal_model.rs`

`TreatWounds { patient }` satisfied when `pain_summary(patient) == Some(Permille(0))`.

Identical to current `Heal` satisfaction — NOT satisfied by merely holding medicine.

### D09. Patient-centric failure handling

**Files**: `crates/worldwake-ai/src/failure_handling.rs`

Update `GoalKind::Heal { .. }` match arm (line 457) to `GoalKind::TreatWounds { .. }`.

Blocker messages reference patient identity (not generic "treatment acquisition failed").
`BlockingFact::TargetGone` resolution checks patient aliveness.

### D10. GoalFamilyPolicy for TreatWounds (leveraging S02)

**Files**: `crates/worldwake-ai/src/goal_policy.rs`

Replace `GoalKind::Heal { .. }` match arm (lines 143-147) with:
```rust
GoalKind::TreatWounds { .. } => GoalFamilyPolicy {
    suppression: SuppressionRule::Never,
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Reactive,
},
```

Identical policy to current `Heal`.

### D11. Direct-observation gate for third-party care

Implemented as part of D07's `emit_care_goals()` logic.

Only `PerceptionSource::DirectObservation` in `BelievedEntityState.source` triggers `TreatWounds` for other patients. `Report`, `Rumor`, `Inference` sources do NOT produce care goals.

No new `InvestigateReport` goal kind. The agent must independently travel to the patient's location; direct observation upon arrival triggers care. Document this as a future extension point.

### D12. Remove obsolete code paths

Across all crates, remove:
- `CommodityPurpose::Treatment` variant from `goal.rs`
- `SelfTargetActionKind::Heal` from `action_handler.rs`
- `emit_treatment_candidates()`, `emit_heal_goals()`, `local_heal_targets()` from `candidate_generation.rs`
- `treatment_pain()`, `treatment_score()` from `ranking.rs`
- All `GoalKind::Heal` match arms across the codebase (replaced by `TreatWounds`)
- All `AcquireCommodity { purpose: Treatment }` ranking branches

Update `GoalKindTag` enum in `goal_model.rs`:
- Remove `Heal` variant
- Add `TreatWounds` variant

## Key Architectural Decisions

**A. Goal naming**: `TreatWounds` (not `Heal`) to disambiguate from `PlannerOpKind::Heal` which names the action op, not the goal.

**B. `care_weight` vs `pain_weight` split**: `pain_weight` governs self-pain urgency. `care_weight` governs third-party care urgency. Per-agent diversity via profile parameters (Principle 20), not hardcoded altruism tiers.

**C. No medicine gate on care goal emission**: Current `emit_heal_goals()` early-returns when agent has zero medicine (`candidate_generation.rs:537-543`). This is wrong — the care intent should form regardless. The planner handles supply acquisition through Trade/Craft/Harvest ops in `TREAT_WOUNDS_OPS`.

**D. Direct-observation gate**: Only `PerceptionSource::DirectObservation` triggers care goals for others. This respects Principle 7 (locality) and prevents "psychic healing" from stale rumors.

**E. Treatment method extensibility**: Future methods (bandage, rest, etc.) add new `ActionDomain::Care` entries and planner ops. `TreatWounds` goal family and satisfaction condition remain stable.

## Component Registration
Authoritative world-state changes:
- `UtilityProfile` gains `care_weight: Permille` field

Disallowed:
- global treatment routing service state
- compatibility shim components preserving both old and new care models

## SystemFn Integration

### `worldwake-core`
- Replace `GoalKind::Heal { target }` with `GoalKind::TreatWounds { patient: EntityId }`
- Remove `CommodityPurpose::Treatment` variant
- Update `GoalKey::from()` to extract `entity: Some(patient)` for `TreatWounds`
- Add `care_weight: Permille` to `UtilityProfile`

### `worldwake-ai`
- `goal_model.rs`: `GoalKindTag::TreatWounds`, ops, satisfaction, binding, places, hypothetical outcome
- `goal_policy.rs`: `TreatWounds` policy entry (Never/Never/Reactive)
- `candidate_generation.rs`: Single `emit_care_goals()` with belief-driven generation and direct-observation gate
- `ranking.rs`: Self/other split using `pain_weight`/`care_weight`, remove `treatment_pain`/`treatment_score`
- `failure_handling.rs`: `Heal` → `TreatWounds` match arms with patient-centric blocker messages

### `worldwake-sim`
- `action_handler.rs`: Remove `SelfTargetActionKind::Heal` variant
- Preserve deterministic affordance, duration, and binding semantics for treatment actions

### `worldwake-systems`
- `combat.rs`: Remove self-target prohibition in `validate_heal_context()` (lines 790-796)
- Preserve co-location, inventory, wound, and actor-state constraints
- Keep treatment effects derived from treatment commodity profiles and wound data

## Cross-System Interactions (Principle 12)
- Combat and deprivation create concrete wounds (stored in component tables)
- E14 passive perception exposes wounded patients to agents through `AgentBeliefStore` with `PerceptionSource`
- AI emits and ranks patient-anchored care goals from those beliefs (direct-observation gate for third-party)
- Planner chooses lawful means: acquiring medicine, producing it, traveling, or treating
- Treatment action mutates wound state and commodity stock via `WorldTxn`

No direct system-to-system calls are introduced. Care remains state-mediated.

## FND-01 Section H

### H1. Information-Path Analysis
- **Self-care**: Agent's own wounds always known (self-authoritative via `has_wounds(agent)` on belief view). Emits `TreatWounds { patient: agent }`.
- **Local third-party care**: Wounded patient directly observed at same place via E14 passive perception → `BelievedEntityState { wounds: [...], source: DirectObservation }` stored in `AgentBeliefStore.known_entities` → `emit_care_goals()` iterates `known_entity_beliefs(agent)`, checks `source == DirectObservation`, emits `TreatWounds { patient }`.
- **Indirect path**: Agent A tells Agent C about wounded Agent B via E15b social telling. C's belief records `source: Report { from: A, chain_len: 1 }`. C does NOT emit `TreatWounds`. C must independently travel to B's believed location. Upon arrival, E14 passive perception fires, belief updates to `DirectObservation`, next tick `emit_care_goals()` emits care goal.
- **Treatment supply**: Visible lots, reachable sellers, known recipes all belief-traced through `AgentBeliefStore.known_entities[entity].last_known_inventory` and `view.known_recipes(agent)`.

### H2. Positive-Feedback Analysis
- Successful treatment increases survival → more agents can perform care (positive loop).
- No care-weight amplification — `care_weight` is static profile parameter, not updated by care outcomes.

### H3. Concrete Dampeners
- **Medicine consumption**: Finite supply limits treatment rate (concrete commodity depletion per treatment action)
- **Action occupancy**: Actor cannot do other things while treating (action framework single-action constraint)
- **Travel time and locality**: Topology graph distance limits who can be treated (place graph travel edges)
- **Belief staleness**: `enforce_capacity()` evicts old beliefs past `memory_retention_ticks` (prevents indefinite stale care intent)
- **Actor incapacitation**: Cannot treat while incapacitated (checked in `validate_heal_context()`)
- **Per-agent `care_weight` diversity**: Spreads responsibility rather than funneling all care through highest-altruism agent

### H4. Stored State vs Derived
**Stored authoritative state**:
- Wounds on agents (component tables)
- Treatment-capable commodity definitions (`CommodityKind::spec().treatment_profile`)
- Inventory and location state (relation tables)
- Action state and actor constraints (scheduler)
- `care_weight` in `UtilityProfile` (component tables)
- `AgentBeliefStore` with wound snapshots and `PerceptionSource` (component tables)

**Derived read-models**:
- Care-goal candidate generation (`emit_care_goals()` output)
- Care urgency ranking (`priority_class()` / `motive_score()` for `TreatWounds`)
- Patient-aware plan decomposition (planner search output)
- Care explanations and blocker classifications (failure handling output)
- `pain_summary()` / `derive_pain_pressure()` computations

No derived care summary may become the source of truth over wounds, inventory, or patient identity.

## Invariants
- There is one top-level care goal family anchored to a patient: `TreatWounds { patient }`
- Top-level generic treatment-acquisition goals (`AcquireCommodity { purpose: Treatment }`) do not exist
- Care plans preserve exact patient identity throughout planning via `matches_binding()`
- Self-treatment is lawful when ordinary action constraints allow it
- Treatment capability comes only from concrete commodity definitions
- All ranking and explanation for care are patient-aware
- No backward-compatibility aliases preserve the old split care model
- Only `DirectObservation` triggers third-party care goals

## Tests

### Core Tests
- [ ] `GoalKind::TreatWounds` satisfies value bounds (Clone, Eq, Ord, Serialize, Deserialize)
- [ ] `GoalKey::from(TreatWounds { patient })` extracts `entity: Some(patient)`, `commodity: None`, `place: None`
- [ ] `CommodityPurpose` does not contain `Treatment` (compile-time — exhaustive match)
- [ ] `UtilityProfile` with `care_weight` defaults correctly and roundtrips through bincode

### Systems Tests
- [ ] Self-treatment lawful when actor has wounds + Medicine + same place
- [ ] Self-treatment still requires all constraints (Medicine, alive, not incapacitated, target has wounds)
- [ ] Third-party treatment regression passes (existing heal tests adapted)
- [ ] Attack self-target still forbidden (`SelfTargetActionKind::Attack` remains)

### AI Candidate Generation Tests
- [ ] `emit_care_goals` emits `TreatWounds { patient: self }` when agent believes self wounded — even without medicine
- [ ] `emit_care_goals` emits `TreatWounds { patient: other }` when other is wounded via `DirectObservation`
- [ ] `emit_care_goals` does NOT emit `TreatWounds` for `Report`/`Rumor`/`Inference` sources
- [ ] No `AcquireCommodity { purpose: Treatment }` goals emitted anywhere

### AI Ranking Tests
- [ ] Self-`TreatWounds` uses `pain_weight` for motive score
- [ ] Other-`TreatWounds` uses `care_weight` for motive score
- [ ] Agent with high `care_weight` + low `pain_weight` prioritizes other-care over self-care
- [ ] Agent with high `pain_weight` + low `care_weight` prioritizes self-care over other-care

### AI Goal Model Tests
- [ ] `TreatWounds` ops include `[Travel, Heal, Trade, QueueForFacilityUse, Craft]`
- [ ] `TreatWounds` satisfied when `pain_summary(patient) == Some(Permille(0))`
- [ ] `matches_binding` exact-bound for terminal `Heal` op, pass for auxiliaries

### AI Failure Handling Tests
- [ ] `TreatWounds` blocker messages reference patient identity
- [ ] `BlockingFact::TargetGone` resolution checks patient aliveness

### Golden Tests
- [ ] Wounded agent self-treats (with and without initial medicine)
- [ ] Healer treats directly-observed wounded patient
- [ ] Indirect wound report (via telling) does NOT trigger care goal — agent must travel and observe
- [ ] Deterministic replay holds for all care scenarios
- [ ] Care goal invalidates when patient heals before treatment arrives

## Acceptance Criteria
- The split between `AcquireCommodity(Treatment)` and `Heal` is removed
- One patient-anchored care goal family `TreatWounds { patient }` owns treatment procurement and application
- Self-treatment and third-party care both work under one lawful action model
- Ranking uses `pain_weight` for self-care and `care_weight` for other-care
- Candidate generation uses E14 `AgentBeliefStore` with `DirectObservation` gate
- The resulting architecture absorbs belief-driven care without another redesign
- No `CommodityPurpose::Treatment`, `GoalKind::Heal`, or `SelfTargetActionKind::Heal` remain

## Suggested Implementation Sequence
1. `worldwake-core`: Add `TreatWounds` to `GoalKind`, remove `Heal`, remove `CommodityPurpose::Treatment`, add `care_weight` to `UtilityProfile`
2. `worldwake-sim`: Remove `SelfTargetActionKind::Heal`
3. `worldwake-systems`: Remove self-target prohibition in `validate_heal_context()` (`combat.rs:790-796`)
4. `worldwake-ai` `goal_model.rs`: `TreatWounds` ops, satisfaction, binding, places, hypothetical outcome; replace `GoalKindTag::Heal` with `GoalKindTag::TreatWounds`
5. `worldwake-ai` `goal_policy.rs`: `TreatWounds` policy entry (identical to current `Heal`)
6. `worldwake-ai` `candidate_generation.rs`: Replace `emit_heal_goals` + `emit_treatment_candidates` with `emit_care_goals` using belief/observation gate
7. `worldwake-ai` `ranking.rs`: Self/other split with `pain_weight`/`care_weight`; remove `treatment_pain`/`treatment_score`
8. `worldwake-ai` `failure_handling.rs`: `Heal` → `TreatWounds` match arms
9. Test migration and new tests
10. Golden test validation: `cargo test --workspace`

## Critical Files

| File | Change Category |
|------|----------------|
| `crates/worldwake-core/src/goal.rs` | Add `TreatWounds`, remove `Heal`, remove `CommodityPurpose::Treatment`, update `GoalKey` |
| `crates/worldwake-core/src/utility_profile.rs` | Add `care_weight: Permille` field |
| `crates/worldwake-sim/src/action_handler.rs` | Remove `SelfTargetActionKind::Heal` |
| `crates/worldwake-systems/src/combat.rs` | Remove self-target prohibition in `validate_heal_context()` |
| `crates/worldwake-ai/src/candidate_generation.rs` | Replace `emit_heal_goals` + `emit_treatment_candidates` with `emit_care_goals` |
| `crates/worldwake-ai/src/ranking.rs` | Self/other ranking split, remove `treatment_pain`/`treatment_score` |
| `crates/worldwake-ai/src/goal_model.rs` | `TreatWounds` ops, satisfaction, binding, places, hypothetical outcome |
| `crates/worldwake-ai/src/goal_policy.rs` | `TreatWounds` policy entry |
| `crates/worldwake-ai/src/failure_handling.rs` | `Heal` → `TreatWounds` match arms |

## Future Extension Points
- **Non-medicine treatment**: New `ActionDomain::Care` action defs (bandage, rest) with corresponding `PlannerOpKind` entries added to `TREAT_WOUNDS_OPS`
- **Investigation goal**: Dedicated `InvestigateReport` goal kind that triggers travel to a reported-wounded patient's location, leading to direct observation and then care
- **Triage priority**: Multiple concurrent `TreatWounds` goals ranked by patient severity and agent relationship
- **Role-based care**: Specialized healer roles via profile weights and equipment requirements
