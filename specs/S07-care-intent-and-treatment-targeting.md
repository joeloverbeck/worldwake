**Status**: DRAFT

# Care Intent and Treatment Targeting

## Summary
Replace the current split care model with one patient-anchored care goal family.

Today the architecture splits care across:
- `GoalKind::AcquireCommodity { purpose: Treatment }`, which knows the commodity but not the patient
- `GoalKind::Heal { target }`, which knows the patient but not the acquisition intent

That split is not the long-term architecture we want. It causes one goal family to carry the resource need and another to carry the patient identity, which then leaks into candidate generation, ranking, search, and action legality.

The clean design is:
- one top-level care goal family anchored to a concrete patient
- planner semantics that let that goal acquire treatment commodities, produce them, move toward the patient, and apply treatment as needed
- no separate top-level generic treatment-acquisition goal
- no self-target prohibition for treatment if the action is otherwise legal

This spec proposes that redesign.

## Why This Exists
The current architecture has four linked faults:

1. `AcquireCommodity(Treatment)` has no patient identity.
2. `Heal { target }` has patient identity but currently sits beside a separate treatment-acquisition goal instead of owning that procurement path.
3. Ranking for `AcquireCommodity(Treatment)` uses the actor's own pain pressure, which is wrong for third-party care and misleading for future belief-driven care.
4. Combat treatment currently forbids self-targeting, so self-wounds and treatment procurement do not compose into a lawful self-care loop.

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
- `worldwake-systems` treatment action legality
- future `E14` belief integration for how agents know a patient is wounded

It is too large and too structural for a ticket-first patch. The correct ownership is a formal spec.

## Phase
Phase 3: Information & Politics hardening, after `E14`, and before further care-domain expansion relies on the current split model.

Recommended placement:
- Step 10, parallel with `S02` and `S03`, after `E14`

## Crates
- `worldwake-core`
- `worldwake-ai`
- `worldwake-sim`
- `worldwake-systems`

## Dependencies
- `E14-perception-beliefs.md`
- `S02-goal-decision-policy-unification.md`
- `S03-planner-target-identity-and-affordance-binding.md`

`S02` and `S03` are architectural siblings. This spec should align with them, and may share implementation surfaces with them, but owns care-domain modeling specifically.

## Design Goals
1. Make care intent patient-anchored, not commodity-anchored.
2. Remove the split between treatment procurement and treatment application as top-level goal families.
3. Support both self-care and third-party care under one lawful model.
4. Keep all reasoning grounded in concrete world state and belief-facing reads.
5. Preserve deterministic planning and ranking behavior.
6. Support future belief-mediated care, role-based care, and relationship-sensitive care without another redesign.
7. Remove obsolete goal variants and logic instead of layering wrappers beside them.

## Non-Goals
- Implementing full medical institutions, triage queues, hospitals, or diagnosis
- Replacing the entire planner
- Modeling non-commodity treatment methods in this spec
- Adding omniscient patient-discovery shortcuts
- Introducing backward-compatibility aliases for old goal variants

## Deliverables

### 1. Replace Split Care Goals With One Patient-Anchored Goal
Remove the top-level split between:
- `GoalKind::AcquireCommodity { commodity, purpose: Treatment }`
- `GoalKind::Heal { target }`

Replace them with one goal family equivalent to:

```rust
GoalKind::TreatWounds { patient: EntityId }
```

The exact name may differ, but the semantics must be:
- the goal is about a concrete patient
- the planner may acquire or produce treatment supplies as part of satisfying that goal
- the goal remains anchored to the same patient throughout planning and execution

Migration rule:
- do not preserve `CommodityPurpose::Treatment` as a parallel top-level path
- do not preserve `Heal { target }` beside the new goal family
- update all callers and tests directly

### 2. Care Goal Owns Its Supply Path
The care goal must own the entire treatment chain:
- if treatment commodity is already controlled, plan can proceed directly to treatment
- if treatment commodity is nearby, sold, or reachable, plan may acquire it
- if treatment commodity can be produced from known recipes, plan may produce it
- if the patient is not local, plan may travel if and only if belief/action rules later allow that care path

This means the planner must treat treatment commodity acquisition as a subordinate means of satisfying `TreatWounds { patient }`, not as a separate top-level intention.

### 3. Care Goal Must Be Exact About Patient Identity
The patient identity is canonical goal identity.

Requirements:
- `GoalKey` canonical entity for care is the patient
- search and affordance binding must preserve exact patient identity
- acquiring medicine for one patient must not silently retarget to another patient mid-plan
- if the intended patient disappears, dies, moves away, or no longer needs treatment, the plan must invalidate and replan from fresh evidence

This reuses the target-binding direction of `S03`, but care ownership lives here.

### 4. Treatment Commodity Capability Remains Item-Defined
Treatment capability must continue to come from concrete commodity definitions:
- `CommodityKind::spec().treatment_profile`

Do not add:
- a second treatment-capability registry
- recipe-only treatment tagging
- hardcoded care-only aliases

The item catalog remains the source of truth for which commodities can reduce wounds.

### 5. Self-Care Must Be Lawful
If an agent has wounds and treatment supply, self-care must be lawful when physical constraints permit it.

Required action-model change:
- remove the current self-target prohibition for the wound-treatment action

The action should be renamed to match the new concrete semantics if needed, for example:

```rust
ActionDef { name: "treat_wounds", ... }
```

The exact action name may differ, but the action must:
- allow `patient == actor`
- continue to require treatment commodity, co-location, action duration, and ordinary actor constraints
- remain interruptible and state-mediated

This is a direct agent-symmetry requirement. Human-controlled and AI-controlled agents must follow the same treatment legality.

### 6. Ranking Must Use Patient-Aware Care Semantics
Care urgency must derive from the patient, not from a generic treatment commodity purpose.

Required ranking direction:
- self-care urgency uses the actor's own wound pressure
- third-party care urgency uses the patient's wound pressure
- actor danger can still promote or suppress care decisions through shared decision policy

To support agent diversity, this spec recommends splitting current motivation semantics into:
- self pain sensitivity
- care-for-others sensitivity

Equivalent profile direction:

```rust
struct UtilityProfile {
    pain_weight: Permille,
    care_weight: Permille,
    ...
}
```

The exact field shape may differ, but the architecture must support:
- agents who strongly prioritize their own pain
- agents who strongly prioritize helping others
- agents who do both

Do not hardcode altruism tiers in ranking logic. Use per-agent parameters.

### 7. Candidate Generation Must Emit Care, Not Generic Treatment Procurement
Candidate generation must stop emitting top-level treatment-acquisition goals.

Instead it must emit patient-anchored care goals from concrete believed evidence:
- self observed as wounded
- co-located wounded patient perceived directly
- later, after `E14`, wounded patients learned through witness/report/record paths that remain locally knowable

Evidence for the care goal should include:
- the patient entity
- the place of the patient when locally known
- any currently known local or reachable treatment supply evidence that motivated the immediate plan path

But canonical identity remains the patient, not the supply.

### 8. Goal Satisfaction Must Be Care-Semantic, Not Inventory-Semantic
`TreatWounds { patient }` must be satisfied by patient state, not by merely holding medicine.

Satisfaction should be defined in one care-semantic place in the goal model. The exact threshold may be tuned through profile-driven or threshold-driven semantics, but the rule must remain patient-based.

Allowed forms:
- patient has no active wounds
- patient pain/wound severity falls below the configured care threshold

Disallowed form:
- actor merely acquired medicine, so the top-level care goal is considered satisfied

### 9. Failure Handling and Debugging Must Stay Patient-Centric
Blocked-intent and explanation surfaces must report care failures in patient terms.

Examples:
- no reachable treatment supply for patient X
- patient X moved away
- patient X no longer has wounds
- treatment action invalid because actor became incapacitated

Do not collapse these into a generic "treatment acquisition failed" reason once care is patient-anchored.

### 10. Future Belief Integration Hook
After `E14`, care candidate generation and ranking must be driven from belief-traceable patient knowledge, not from omniscient wound scans.

This spec therefore requires that the care goal architecture work with:
- stale patient state
- contradictory reports
- missed observations
- local records or testimony

The care model must not assume omniscient patient truth in its long-term design.

## Component Registration
Authoritative world-state changes may include updates to existing profile components, but this spec should not add a new "care manager" component or any global treatment singleton.

Allowed authoritative additions:
- extending an existing per-agent profile type with care-related weights or thresholds if needed

Disallowed:
- global treatment routing service state
- compatibility shim components preserving both old and new care models

## SystemFn Integration

### `worldwake-core`
- replace split treatment goal variants with one patient-anchored care goal family
- update `GoalKey` canonical identity accordingly
- extend per-agent profile data if separate `care_weight` or care thresholds are adopted
- remove obsolete treatment-purpose goal identity paths

### `worldwake-ai`
- emit patient-anchored care goals in candidate generation
- rank care goals from patient-aware pressure and actor-specific care preferences
- let search satisfy care goals through medicine acquisition, production, travel, and treatment actions as lawful means
- update goal explanation and blocker reporting to remain patient-centric
- remove generic top-level treatment acquisition logic

### `worldwake-sim`
- preserve deterministic affordance, duration, and binding semantics for treatment actions
- ensure action semantics and planner affordance matching support self-target treatment lawfully

### `worldwake-systems`
- rename and/or update the treatment action definition to reflect concrete wound treatment semantics
- remove self-target prohibition
- preserve ordinary co-location, inventory, wound, and actor-state constraints
- keep treatment effects derived from treatment commodity profiles and wound data

## Cross-System Interactions (Principle 12)
- combat and deprivation create concrete wounds
- belief/perception exposes wounded patients to agents through lawful information paths
- AI emits and ranks patient-anchored care goals from those beliefs
- planner chooses lawful means such as acquiring medicine, producing it, traveling, or treating
- treatment action mutates wound state and commodity stock

No direct system-to-system calls are introduced. Care remains state-mediated.

## FND-01 Section H

### H1. Information-Path Analysis
- self-care path: agent experiences wounds directly -> own belief/memory contains wound evidence -> emits `TreatWounds { self }`
- local third-party care path: wounded patient is directly perceived at the same place -> belief/memory records patient condition -> emits `TreatWounds { patient }`
- later indirect path after `E14`: witness/report/record conveys patient state -> belief store updates -> care goal may form if the patient remains locally actionable under those beliefs
- treatment supply path remains concrete: visible local lots, reachable sellers, known recipes, and resource sources

### H2. Positive-Feedback Analysis
- untreated wounds can create more danger and more future wounds if patients remain vulnerable
- better care capability can increase survival, which increases the number of agents who can later perform care
- role-sensitive care could create care concentration loops if one healer keeps being selected for every case

### H3. Concrete Dampeners
- treatment consumes concrete medicine
- treatment occupies actor time and attention
- actor incapacitation and danger can prevent care
- travel time and locality limit who can be treated
- belief staleness after `E14` prevents perfect care routing
- per-agent care preferences and physical constraints spread responsibility rather than relying on an invisible dispatcher

### H4. Stored State vs Derived Read-Models
Stored authoritative state:
- wounds on agents
- treatment-capable commodity definitions
- inventory and location state
- action state and actor constraints
- per-agent utility/profile fields if care-specific weights are added

Derived read-models:
- care-goal candidate generation
- care urgency/ranking
- patient-aware plan decomposition
- care explanations and blocker classifications

No derived care summary may become the source of truth over wounds, inventory, or patient identity.

## Invariants
- there is one top-level care goal family anchored to a patient
- top-level generic treatment-acquisition goals do not exist beside it
- care plans preserve exact patient identity throughout planning
- self-treatment is lawful when ordinary action constraints allow it
- treatment capability comes only from concrete commodity definitions
- all ranking and explanation for care are patient-aware
- no backward-compatibility aliases preserve the old split care model

## Tests
- [ ] Goal identity tests prove the care goal canonical entity is the patient
- [ ] Candidate-generation tests emit care for wounded self and wounded others from lawful evidence
- [ ] Candidate-generation tests do not emit obsolete top-level treatment-acquisition goals
- [ ] Ranking tests distinguish self pain weighting from other-care weighting
- [ ] Search tests prove a care goal can satisfy through treatment supply acquisition before treatment application
- [ ] Search tests preserve exact patient identity while acquiring medicine
- [ ] Action tests prove self-treatment is lawful and deterministic when the actor holds medicine
- [ ] Golden tests prove:
  - wounded self can acquire/apply medicine when lawful
  - healer can acquire/apply medicine for another patient
  - deterministic replay holds for both
- [ ] Blocker/explanation tests report patient-centric failure reasons

## Acceptance Criteria
- the split between `AcquireCommodity(Treatment)` and `Heal` is removed
- one patient-anchored care goal family owns treatment procurement and application
- self-treatment and third-party care both work under one lawful action model
- ranking and explanation become patient-aware instead of generic-treatment-aware
- the resulting architecture can absorb `E14` belief isolation without another care redesign

## Suggested Implementation Sequence
1. replace split care goal identity in `worldwake-core`
2. update goal semantics and planner decomposition in `worldwake-ai`
3. update ranking and candidate generation to emit/rank patient-anchored care goals
4. rename/update treatment action semantics in `worldwake-systems` and remove self-target prohibition
5. migrate tests and remove obsolete code paths
6. add patient-centric explanation/blocker coverage
7. validate with focused and workspace-wide tests

## Relationship to Existing Specs
- depends on `E14` because long-term care intent must be belief-driven
- complements `S02` by giving the decision-policy layer one coherent care goal family instead of split special cases
- complements `S03` because care is an exact patient-bound planning problem
- should land before later role-based or companion care work expands the current care model

## Open Questions
1. Should care thresholds reuse `DriveThresholds.pain`, or should care willingness become its own profile/threshold surface?
2. Should future non-medicine treatment methods join this same care goal family as alternative means, or should they wait for a later medical-system spec?
3. After `E14`, should indirect reports of wounded patients create only investigation/travel intent first, or may they directly emit care intent when the patient location is believed strongly enough?
