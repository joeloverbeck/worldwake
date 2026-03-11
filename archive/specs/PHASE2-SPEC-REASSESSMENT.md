# Phase 2 Spec Reassessment: E09-E13

**Status**: COMPLETED

This memo records the structural corrections made while re-evaluating Phase 2 against the foundational principles.

## Overall Judgment

The original Phase 2 set was strong in intent but not yet strict enough in several places about:
- concrete state over abstract scores
- local information flow
- physical carriers for trade / production / route interaction
- avoiding future-epic leakage into Phase 2 AI
- making survival pressures materially consequential

The weakest areas were:

1. **E13 overreached its phase** by naming goals whose supporting systems do not exist yet.
2. **E11 described emergent pricing at a slogan level, not an implementable causal mechanism.**
3. **E10 still allowed effectively magical harvest and abstract facility capacity.**
4. **E09 lacked hard deprivation consequences and mixed decision-layer motivation into action legality.**
5. **E09 / E12 / E13 shared ownership of pain, fear, wounds, and thresholds in inconsistent ways.**

## Cross-Epic Corrections

### 1. Remove stored fear from Phase 2
Stored fear was the wrong abstraction.  
Phase 2 now derives danger from believed threats, current attackers, co-located hostiles, and current wounds.

Result:
- E09 no longer owns fear
- E13 derives danger pressure
- E14 can later replace omniscient belief access without rewriting the AI layer

### 2. Move thresholds out of sole E13 ownership
The original specs had E09 action logic and urgency behavior depending on thresholds “owned by E13,” even though E13 comes later in the implementation order.

Result:
- `DriveThresholds` becomes shared Phase 2 schema used by both E09 and E13
- thresholds are now **per drive**, not one generic ladder for everything

### 3. Unify bodily harm through wounds
If starvation, dehydration, and combat all matter, they should converge on the same consequence carrier.

Result:
- E09 can add deprivation wounds
- E12 owns wound progression / recovery logic
- E13 derives pain from `WoundList`
- no duplicate “health” or “condition” score is needed

### 4. Ground production in real source stock and real work sites
The original production spec still let “harvest” create goods from a tagged place with no concrete stock.

Result:
- `ResourceSource` / `YieldBuffer`
- reservable workstation entities
- persistent WIP jobs
- no abstract facility slot count

### 5. Make route presence concrete in Phase 2
The original specs already knew route danger could not be lawful without route presence, but they left travel in a too-abstract state.

Result:
- `InTransitOnEdge` becomes part of E10
- future ambush / escort / witness logic now has a lawful physical carrier

### 6. Replace static goal catalog selection with grounded candidate generation
The original E13 goal catalog mixed present and future systems and risked a wish-list AI.

Result:
- candidate goals must emerge from concrete believed evidence
- only Phase 2-grounded goals remain
- plans are parameterized, not bare action IDs

### 7. Add a real dampener for AI replan loops
“Needs will eventually change” was not good enough as the sole dampener.

Result:
- `BlockedIntent` / failure memory records the concrete reason an attempt failed
- agents avoid instantly retrying the same blocked target until something changes

## Dependency / Order Correction

One important order issue existed in the original plan:

- **E11 cannot fully satisfy its own acceptance criteria without E10**, because physical restock depends on physical procurement / transport.
- The original “E09-E12 fully parallel” statement also hid the fact that body-harm and thresholds need shared schema up front.

The revised implementation order file addresses this.

## What Stayed Intact

The reassessment keeps the original high-level Phase 2 goal:
- autonomous survival
- physical logistics
- local trade
- bodily harm with finality
- unified agent decision making

It does **not** add later-phase politics, rumor, faction, office, or camp systems into Phase 2.

## Output Files
The revised package includes:
- corrected E09
- corrected E10
- corrected E11
- corrected E12
- corrected E13
- corrected implementation order

## Outcome
- Completion date: 2026-03-11
- What actually changed: the reassessment corrections were carried into the implemented Phase 2 stack, including shared thresholds and wound ownership, concrete production and transit carriers, grounded trade/restock assumptions, and the E13 decision-architecture scope and planner corrections.
- Deviations from original plan: this memo remained a corrective archival record rather than becoming a living active spec once the E09-E13 implementation work landed.
- Verification results: the implemented E13 surface was verified with `cargo test -p worldwake-ai` on 2026-03-11; the related Phase 2 correction targets are also reflected in the archived completed E10, E11, and E12 specs plus the current implementation-order document.
