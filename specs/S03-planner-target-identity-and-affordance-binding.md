**Status**: PENDING

# Planner Target Identity and Affordance Binding

## Summary
Design exact target-aware planner matching for Worldwake so a grounded goal can require the planner to act on the specific corpse, sale lot, facility, report, or evidence object that caused the goal to exist, rather than matching any affordance from the same broad action family.

This spec introduces goal-aware affordance binding rules in the planning stack. It keeps the current canonical goal identity model, but adds explicit matching semantics so:
- `LootCorpse { corpse }` plans against that corpse
- `BuryCorpse { corpse, burial_site }` plans against that corpse and that grave plot
- future sale-lot trade plans bind to the exact listed lot
- future audit / investigation plans bind to the exact facility, record, or evidence target that motivated them

This is intentionally forward-looking and must not be scheduled ahead of the active phase gates in [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) without explicit reprioritization.

## Why This Exists
Current planning is mostly clean, but one architectural weakness remains:
- candidate generation produces grounded goals from concrete local evidence
- goal identity stores canonical commodity/entity/place fields
- search classifies actions by planner-op family
- but affordance matching can still accept the right action family without proving it is targeting the same concrete object that motivated the goal

That is tolerable for broad self-care or open-ended acquisition goals, but it becomes incorrect for object-specific goals.

This weakness will grow as the simulation becomes more concrete:
- corpse interactions need exact corpse and burial-site binding
- sale-lot trading needs exact lot binding
- storage/display logistics need exact facility/container binding
- crime and investigation need exact evidence/report/location binding
- institutional work will need exact office/record binding

If this is not fixed as shared architecture, every new system will be tempted to add one-off payload checks, hidden aliases, or family-specific search hacks. That would violate:
- Principle 3: concrete state over abstract scores
- Principle 7: locality of interaction and information
- Principle 24: systems interact through state, not through each other
- Principle 26: no backward compatibility
- Principle 27: debuggability is a product feature

The clean architecture is:
- candidate generation creates a grounded goal from concrete evidence
- goal semantics declare whether the goal is flexible or exact about each target slot
- search filters affordances through those binding semantics before successor construction
- hypothetical transitions preserve or transform bindings explicitly
- blocker handling and debugging can report exactly which concrete target failed

## Phase
Phase 3: Information & Politics, Step 10 (parallel after E14)

## Crates
- `worldwake-ai`
- `worldwake-sim`
- `worldwake-systems`
- `worldwake-core`

## Dependencies
- E14

## Design Goals
1. Preserve the current canonical goal identity model while making exact target matching explicit.
2. Distinguish goals that are intentionally flexible from goals that are object-specific.
3. Keep all matching state-derived and deterministic.
4. Avoid payload-only hacks, ad hoc special cases, or fallback aliases.
5. Make failure surfaces and blocker memory point to the exact object that failed.
6. Support future exact-binding scenarios without redesigning search again.
7. Do not add any omniscient planner shortcut that bypasses belief/evidence grounding.

## Non-Goals
- Replacing the planner with valuation-only logic
- Expanding goal identity into a full action-instance identity
- Adding backward-compatibility shims for loose matching once exact matching is implemented
- Introducing a global query service that resolves “best target” outside candidate generation

## Deliverables

### 1. Explicit Goal Binding Policy
Each goal family must declare how strictly it binds planner targets.

Add shared planning semantics equivalent to:

```rust
enum GoalBindingPolicy {
    Flexible,
    ExactEntity {
        entity: EntityId,
    },
    ExactPlace {
        place: EntityId,
    },
    ExactEntityAndPlace {
        entity: EntityId,
        place: EntityId,
    },
    ExactTargetTuple,
}
```

The implementation does not need to use this exact type name, but the semantics must exist explicitly in code.

Interpretation:
- `Flexible`: any affordance in the legal action family may satisfy the goal if the resulting state satisfies goal semantics
- `ExactEntity`: the acted-on authoritative entity must be the canonical entity from the goal
- `ExactPlace`: the acted-on authoritative place/facility must be the canonical place from the goal
- `ExactEntityAndPlace` / `ExactTargetTuple`: both bindings must match together

This policy belongs in goal semantics, not in one-off search branches.

### 2. Goal Families Must Declare Binding Intent
Goal semantics must classify current and future goals intentionally.

Initial target-binding expectations:
- `ConsumeOwnedCommodity`: `Flexible`
- `AcquireCommodity { SelfConsume | Restock | RecipeInput | Treatment }`: `Flexible`
- `Sleep`: `Flexible`
- `Relieve`: `Flexible`
- `Wash`: `Flexible`
- `EngageHostile { target }`: exact hostile target
- `ReduceDanger`: `Flexible`
- `Heal { target }`: exact target
- `ProduceCommodity { recipe_id }`: `Flexible`
- `SellCommodity`: currently flexible by commodity, but this spec must allow later evolution to exact facility or listed lot
- `RestockCommodity`: flexible today, but must support later exact facility binding
- `MoveCargo { commodity, destination }`: exact destination, flexible lot identity
- `LootCorpse { corpse }`: exact corpse
- `BuryCorpse { corpse, burial_site }`: exact corpse plus exact burial site

Future drafts must be able to tighten binding policy without redesigning search.

### 3. Search Must Filter Affordances Before Successor Construction
Search currently filters mostly by planner-op family and only later relies on payload/transition semantics. That is too late for exact-object goals.

New rule:
- after planner-op family filtering
- before successor construction
- the planner must ask goal semantics whether the affordance targets and payload are a legal binding for the current grounded goal

Equivalent interface:

```rust
trait GoalKindPlannerExt {
    fn matches_affordance_binding(
        &self,
        authoritative_targets: &[EntityId],
        payload: Option<&ActionPayload>,
    ) -> bool;
}
```

Requirements:
- matching must use authoritative entity ids when available
- payload-derived identities may refine matching, but must not replace authoritative target checks when the targets exist
- wrong-target affordances must be discarded, not explored and rejected later

### 4. Candidate Generation Must Keep Exact Evidence Separate From Incidental Evidence
`GroundedGoal` currently carries evidence entity/place sets. That remains useful for ranking and debugging, but exact bindings must not be inferred from arbitrary evidence membership.

New rule:
- canonical target fields in `GoalKey` define the exact target identity
- extra evidence entities/places remain supporting evidence only
- search binding must read canonical target identity, not “any evidence entity”

This keeps ranking/debug evidence broad while keeping planner bindings precise.

### 5. Planner-Only Candidates and Hypothetical Transitions Must Preserve Binding Semantics
Any planner-only candidate or hypothetical transition must preserve exact target meaning.

Examples:
- `BuryCorpse` may not opportunistically retarget from one corpse to another just because both are valid local corpses
- future sale-lot trade may not retarget from one listed lot to another during search expansion
- `MoveCargo` may remain lot-flexible but destination-exact

If a transition legitimately changes the target identity, that must happen by:
- completing a materialization barrier
- invalidating the current plan
- replanning from fresh grounded evidence

Not by silently changing targets mid-plan.

### 6. Failure Handling Must Report Exact Binding Failures
When an exact-bound plan step fails, blocker reporting must preserve the exact failed target relation.

Requirements:
- distinguish “the target entity disappeared” from “an alternative object exists”
- distinguish “wrong facility” from “facility unavailable”
- keep blocked-intent memory keyed by the grounded goal’s canonical identity, not by the looser action family

This is essential for:
- debuggability
- stable blocker expiry
- later investigative systems that reason from exact failed targets

### 7. Affordance Enumeration Must Stay Deterministic and Concrete
Exact binding must not introduce nondeterministic “best match” selection inside affordance enumeration.

Rules:
- affordance enumeration still returns all legal local affordances in deterministic order
- goal-aware binding selects from those affordances deterministically
- no heuristic “closest good enough corpse/facility/lot” selector should exist outside candidate generation

This prevents hidden retargeting logic from moving into another layer.

### 8. Debug Surface Must Expose Binding Decisions
Planner/debug output for exact-bound goals must answer:
- what exact target was required
- which affordances were rejected for wrong binding
- which affordance matched
- whether failure came from target disappearance, wrong target, or resource absence

This is a direct Principle 27 requirement.

## Architecture Notes

### Why Not Make Every Goal Exact?
Because some goals are intentionally open-ended:
- hunger relief should allow any legal local food source
- generic procurement should allow any valid acquisition path
- `ReduceDanger` should choose among several legal mitigation paths

Forcing exact binding everywhere would overfit the planner to candidate-generation contingencies and make the system less extensible.

### Why Not Encode Full Target Tuples Into GoalKey?
Because `GoalKey` is serving two jobs already:
- stable goal identity for blocked memory / selection
- canonical target summary

Inflating it into full action-instance identity would overcouple planning to one action path and make flexible goals harder to express. The cleaner split is:
- `GoalKey`: canonical identity
- binding policy: tells search how strict canonical identity is

### Why Not Leave This to Payload Validation?
Because payload validation happens too late and is action-specific. The planner should not explore wrong-object affordances in the first place. Shared target binding belongs in planner semantics, not in scattered payload validators.

## Component Registration
No new authoritative world components are required by this spec.

If implementation introduces helper enums/structs such as goal-binding policy or affordance-match diagnostics, they should live in planning/AI layers and must not become authoritative world state.

## SystemFn Integration

### `worldwake-core`
- no new authoritative components required
- preserve existing canonical goal identity fields on `GoalKind` / `GoalKey`
- do not add compatibility aliases to encode looser matching

### `worldwake-sim`
- affordance enumeration remains deterministic and complete
- payload validators continue to verify action legality, but no longer shoulder planner-wide exact-target semantics alone
- future affordance/belief helpers for listed lots, audit targets, or evidence targets must expose concrete authoritative ids so planner binding can use them directly

### `worldwake-systems`
- continue emitting concrete affordances and payloads
- object-specific actions (`loot`, `bury`, future `staff_market`, future `audit_stock`, future `collect_evidence`) must expose authoritative targets explicitly
- do not add system-local “nearest valid target” fallback behavior to compensate for loose planner matching

### `worldwake-ai`
- extend goal semantics to declare binding policy per goal family
- add goal-aware affordance binding checks in search before successor construction
- keep hypothetical transition semantics exact-target preserving
- update failure handling/debug surfaces to report exact binding failures
- ensure blocked-intent memory continues to operate on canonical goal identity

## Cross-System Interactions (Principle 12)
- E10 transport supplies exact facility/container targets for cargo and stock handling
- E11 trade drafts supply exact listed-lot targets for commodity exchange
- E12 corpse actions supply exact corpse and burial-site targets
- E14 beliefs limit which concrete objects are even known to the planner
- E17 crime/investigation will depend on exact evidence/report/facility targets

All of these remain state-mediated:
- goal grounding reads concrete believed entities/places
- affordances expose concrete authoritative targets
- search matches those targets deterministically
- systems commit ordinary world-state changes

No system-to-system command path is introduced.

## Testing Requirements
- goal semantics tests for binding policy per exact-bound goal family
- search tests proving wrong-target affordances are rejected before successor construction
- regression test for `BuryCorpse` exact corpse + exact burial-site binding
- regression test for `LootCorpse` exact corpse binding
- future trade tests proving exact listed-lot targeting once merchant-selling draft lands
- blocker/debug tests proving exact target failure is surfaced distinctly from generic no-path/no-input failure

## Acceptance Criteria
- exact-bound goals cannot silently retarget to a sibling affordance of the same family
- flexible goals remain flexible
- planner determinism is preserved
- blocker memory and debugging refer to the correct concrete target
- future drafts for listed-lot trade, stock storage, and audits can reuse the same planner-binding architecture without introducing new special cases

## FND-01 Section H

### Information-Path Analysis
- Exact target binding only applies to objects already present in the agent’s grounded goal evidence.
- The planner still cannot query raw world state for “better” unseen alternatives.
- Future sale-lot, corpse, audit, and evidence interactions remain local because affordances expose only locally knowable concrete objects.
- No information path is added that would let agents know about hidden objects merely because the action family exists.

### Positive-Feedback Analysis
- Loose target matching can create planner thrash and accidental success loops by allowing a goal for one object to be satisfied by another.
- Exact binding dampens that loop by forcing replanning when the grounded object disappears or changes.
- Flexible goals still allow adaptive behavior where the foundations want it, so the planner does not become brittle.

### Concrete Dampeners
- exact bindings break when the object leaves, is consumed, is buried, is stolen, or is no longer local
- replanning from fresh evidence is the only legal retarget path
- deterministic affordance ordering prevents hidden oscillation from nondeterministic tie-breaking
- blocked-intent memory on canonical target identity prevents immediate repeated retries against the same failed object

### Stored-vs-Derived State
Stored authoritative state:
- entities
- places
- ownership/custody/containment relations
- action affordances derived from authoritative state at tick time
- goal canonical identity fields

Derived / cached state:
- grounded goal evidence sets
- goal binding policy
- affordance-binding match decisions
- blocker classifications for failed exact-bound steps

No derived binding result may become authoritative truth.

## Recommended Sequencing
When this spec is scheduled, it should be implemented before or alongside:
- listed-lot trade from [S04-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/S04-merchant-selling-market-presence.md)
- facility stock/display targeting from [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md)
- exact evidence targeting in E17 crime/theft/justice follow-on work

Otherwise those efforts will likely reintroduce one-off exact-target hacks.
