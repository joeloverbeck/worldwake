**Status**: ✅ COMPLETED

# Planner Target Identity and Affordance Binding

## Summary
Design exact target-aware planner matching for Worldwake so a grounded goal can require the planner to act on the specific corpse, hostile, listener, office, or evidence object that caused the goal to exist, rather than matching any affordance from the same broad action family.

This spec adds a `matches_binding` method to `GoalKindPlannerExt` that dispatches on `PlannerOpKind` to distinguish auxiliary ops (Travel, Trade, Harvest, Craft, etc.) from terminal ops (Attack, Loot, Heal, Tell, DeclareSupport, Bury) — only terminal ops check target identity against the goal's canonical fields.

This is intentionally forward-looking and must not be scheduled ahead of the active phase gates in [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) without explicit reprioritization.

## Why This Exists
Current planning is mostly clean, but one architectural weakness remains:
- candidate generation produces grounded goals from concrete local evidence
- goal identity stores canonical commodity/entity/place fields via `GoalKey`
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
- `GoalKind` variant fields already encode the canonical target identity (extracted into `GoalKey`)
- `matches_binding()` on `GoalKindPlannerExt` tells search whether a candidate's authoritative targets satisfy the goal's target requirement for a given op kind
- search filters candidates through binding before successor construction
- hypothetical transitions preserve bindings by reading target identity from `GoalKind`, not from candidate targets
- blocker handling and debugging can report exactly which concrete target failed

## Phase
Phase 3: Information & Politics, Step 10 (parallel after E14)

## Crates
- `worldwake-ai`

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
- Introducing a global query service that resolves "best target" outside candidate generation
- Adding a separate `GoalBindingPolicy` enum — `GoalKey` already encodes whether targets exist; a parallel enum adds sync burden without value

## Deliverables

### 1. Add `matches_binding()` to `GoalKindPlannerExt`

Add a method to the existing `GoalKindPlannerExt` trait implementation:

```rust
impl GoalKindPlannerExt for GoalKind {
    fn matches_binding(
        &self,
        authoritative_targets: &[EntityId],
        op_kind: PlannerOpKind,
    ) -> bool;
}
```

**Dispatch logic**:

1. **Auxiliary ops always pass** — `Travel`, `Trade`, `Harvest`, `Craft`, `QueueForFacilityUse`, `MoveCargo`, `Consume`, `Sleep`, `Relieve`, `Wash`, `Defend`, `Bribe`, `Threaten` return `true` unconditionally. These ops serve the goal but do not act on the goal's canonical target entity.

2. **Terminal ops check goal-specific target identity** — `Attack`, `Loot`, `Heal`, `Tell`, `DeclareSupport`, `Bury` must verify that `authoritative_targets` contains the entity required by the goal's variant fields.

3. **Empty `authoritative_targets` skip binding** — planner-only synthetic candidates (e.g., hypothetical put-down of materialized items) have empty `authoritative_targets` and bypass binding checks.

**Rationale**: A separate `GoalBindingPolicy` enum is not needed. `GoalKey` already extracts canonical `entity: Option<EntityId>` and `place: Option<EntityId>` from every `GoalKind` variant. The `matches_binding` method reads those same variant fields directly, avoiding a parallel enum that must stay in sync.

### 2. Goal Families — Binding Classification

All 17 `GoalKind` variants classified by binding behavior:

| GoalKind | Binding | Terminal Op | Target Field(s) |
|---|---|---|---|
| `ConsumeOwnedCommodity { commodity }` | Flexible | `Consume` | — |
| `AcquireCommodity { commodity, purpose }` | Flexible | varies | — |
| `Sleep` | Flexible | `Sleep` | — |
| `Relieve` | Flexible | `Relieve` | — |
| `Wash` | Flexible | `Wash` | — |
| `EngageHostile { target }` | Exact entity | `Attack` | `target` |
| `ReduceDanger` | Flexible | `Defend` | — |
| `Heal { target }` | Exact entity | `Heal` | `target` |
| `ProduceCommodity { recipe_id }` | Flexible | `Craft` | — |
| `SellCommodity { commodity }` | Flexible | `Trade` | — |
| `RestockCommodity { commodity }` | Flexible | `Trade` | — |
| `MoveCargo { commodity, destination }` | Exact destination | `MoveCargo` | `destination` (place only) |
| `LootCorpse { corpse }` | Exact entity | `Loot` | `corpse` |
| `BuryCorpse { corpse, burial_site }` | Exact entity+place | `Bury` | `corpse`, `burial_site` |
| `ShareBelief { listener, subject }` | Exact entity | `Tell` | `listener` |
| `ClaimOffice { office }` | Exact entity | `DeclareSupport` | `office` (via payload) |
| `SupportCandidateForOffice { office, candidate }` | Exact entity | `DeclareSupport` | `office` (via payload) |

**Notes on specific variants**:

- **`BuryCorpse`**: The `GoalKind` variant exists but no action def or handler exists yet. Binding semantics are documented here for future implementation. Tests cannot cover this variant until the action def is registered.

- **`ClaimOffice` / `SupportCandidateForOffice`**: The `declare_support` action def has `targets: Vec::new()`, so affordances have empty `bound_targets`. The payload is built entirely from the goal's fields via `build_declare_support_payload_override()`. Because `authoritative_targets` is empty, the binding check falls through to the "empty targets skip binding" rule. This is correct: the payload override validator already ensures the payload references the right office and candidate. Document this edge case explicitly.

- **`MoveCargo`**: Binding is on the destination place, not on lot identity. Lots are flexible.

Future drafts must be able to tighten binding policy (e.g., exact facility for `SellCommodity`) without redesigning search.

### 3. Search Must Filter Candidates via `matches_binding()` Before Successor Construction

In `search_candidates()` (search.rs), insert a `.retain()` call **after** the existing facility-use blocked filter:

```rust
candidates.retain(|candidate| !candidate_uses_blocked_facility_use(candidate, &node.state, registry));

// NEW: reject candidates whose authoritative targets violate goal binding
candidates.retain(|candidate| {
    let Some(semantics) = semantics_table.get(&candidate.def_id) else {
        return true;
    };
    goal.kind.matches_binding(&candidate.authoritative_targets, semantics.op_kind)
});
```

**Requirements**:
- Matching must use `authoritative_targets` (from `Affordance.bound_targets`)
- Wrong-target affordances must be discarded before successor construction, not explored and rejected later
- Candidates with empty `authoritative_targets` (planner-only synthetic candidates) skip binding checks — this is handled inside `matches_binding()` returning `true` for empty targets

**Op-kind dispatch rationale**: The same goal may involve multiple relevant ops across a multi-step plan. For example, `LootCorpse { corpse: X }` may require Travel → Loot. The Travel step targets a destination place, not the corpse — so Travel is auxiliary and must not be checked against the corpse entity. Only the terminal Loot op checks that its target matches `corpse: X`.

### 4. Candidate Generation Keeps Exact Evidence Separate From Incidental Evidence (Verified)

Verified correct in current implementation. `GoalKey` extracts canonical `entity`/`place` from `GoalKind` variant fields. `evidence_entities`/`evidence_places` in `GroundedGoal` remain separate supporting data used for ranking and debugging only.

Search binding reads canonical target identity from `GoalKind` variant fields, not from "any evidence entity". No changes needed.

### 5. Hypothetical Transitions Already Preserve Binding (Verified)

`apply_planner_step` reads target identity from `GoalKind` variant fields, not from candidate targets. Binding is preserved by construction. No code changes needed.

For example, `BuryCorpse` may not opportunistically retarget from one corpse to another just because both are valid local corpses. If a transition legitimately changes the target identity, that must happen by:
- completing a materialization barrier
- invalidating the current plan
- replanning from fresh grounded evidence

Not by silently changing targets mid-plan.

### 6. Failure Handling Already Target-Specific (Verified)

`BlockedIntent` records `goal_key: GoalKey` which includes `entity` and `place` extracted from `GoalKind`. For example, `LootCorpse { corpse: X }` and `LootCorpse { corpse: Y }` produce different `GoalKey` values. Already correct; no changes needed.

This means:
- "the target entity disappeared" is distinguishable from "an alternative object exists" (different `GoalKey`)
- blocked-intent memory is keyed by the grounded goal's canonical identity, not by the looser action family
- blocker expiry operates on the correct concrete target

### 7. Affordance Enumeration Must Stay Deterministic and Concrete

Exact binding must not introduce nondeterministic "best match" selection inside affordance enumeration.

Rules:
- affordance enumeration still returns all legal local affordances in deterministic order
- goal-aware binding selects from those affordances deterministically via `.retain()`
- no heuristic "closest good enough corpse/facility/lot" selector should exist outside candidate generation

This prevents hidden retargeting logic from moving into another layer. No changes needed; this deliverable documents an invariant.

### 8. BindingRejection Trace Struct

Add a concrete trace type for binding rejections:

```rust
pub struct BindingRejection {
    pub def_id: ActionDefId,
    pub rejected_target: EntityId,
    pub required_target: EntityId,
}
```

Integration with the decision trace system:

- Add `binding_rejections: Vec<BindingRejection>` to `PlanAttemptTrace`
- Populate from the `.retain()` filter: when `matches_binding()` returns `false`, record the rejection
- Propagate through `plan_search_result_to_trace()` in agent_tick.rs

The debug surface must answer:
- what exact target was required by the goal
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

### Why Not a Separate `GoalBindingPolicy` Enum?
`GoalKey` already encodes canonical `entity` and `place` from every `GoalKind` variant. Adding a parallel `GoalBindingPolicy` enum that re-declares "this goal has an entity target" duplicates information that `GoalKind` variant fields already express. The `matches_binding()` method reads those fields directly, keeping the source of truth in one place.

### Why Not Leave This to Payload Validation?
Because payload validation happens too late and is action-specific. The planner should not explore wrong-object affordances in the first place. Shared target binding belongs in planner semantics, not in scattered payload validators.

### Op-Kind Dispatch Rationale
`matches_binding` takes `op_kind` because the same goal involves multiple action types across a multi-step plan. A `LootCorpse { corpse: X }` plan may include Travel → Loot steps. Travel's targets are destination places, not corpse entities. Only terminal/core ops that directly act on the goal's target entity need binding verification. Auxiliary ops that serve the goal indirectly (navigation, resource gathering, facility queuing) always pass.

### DeclareSupport Edge Case
The `declare_support` action def has `targets: Vec::new()`, so affordances produced for it have empty `bound_targets`. The payload is constructed entirely from the goal's fields via `build_declare_support_payload_override()`. Because `authoritative_targets` is empty for these candidates, `matches_binding()` returns `true` (empty targets bypass). This is correct behavior: the payload override validator (`validate_declare_support_payload_override`) already ensures the payload references the correct office and candidate. The binding check would be redundant and would require special-casing to extract targets from payloads instead of from `bound_targets`.

### Planner-Only Candidate Bypass
Synthetic candidates generated by `planner_only_candidates()` (e.g., hypothetical put-down of items that haven't been picked up yet) have empty `authoritative_targets` (see `SearchCandidate` construction in `search_candidate_from_planner`). These skip binding checks because they represent hypothetical future actions, not concrete affordances bound to specific entities.

## Component Registration
No new authoritative world components are required by this spec.

`BindingRejection` and `matches_binding()` logic live in the `worldwake-ai` planning layer and must not become authoritative world state.

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
- do not add system-local "nearest valid target" fallback behavior to compensate for loose planner matching

### `worldwake-ai`
- add `matches_binding()` to `GoalKindPlannerExt` with `PlannerOpKind` dispatch
- add `.retain()` binding filter in `search_candidates()` after the facility-use blocked filter
- add `BindingRejection` trace struct to `PlanAttemptTrace`
- hypothetical transitions already correct (verified)
- failure handling already correct (verified)
- blocked-intent memory already operates on canonical goal identity (verified)

## Cross-System Interactions (Principle 12)
- E10 transport supplies exact facility/container targets for cargo and stock handling
- E11 trade drafts supply exact listed-lot targets for commodity exchange
- E12 corpse actions supply exact corpse targets (burial-site action def pending)
- E14 beliefs limit which concrete objects are even known to the planner
- E15b social actions supply listener targets for belief sharing
- E16 office actions supply office/candidate targets via payload (empty `bound_targets`)
- E17 crime/investigation will depend on exact evidence/report/facility targets

All of these remain state-mediated:
- goal grounding reads concrete believed entities/places
- affordances expose concrete authoritative targets (or payloads for targetless action defs)
- search matches those targets deterministically
- systems commit ordinary world-state changes

No system-to-system command path is introduced.

## Testing Requirements

### Unit Tests for `matches_binding()`
Per-variant tests covering:
- **Match**: `LootCorpse { corpse: X }` with `authoritative_targets: [X]` and `op_kind: Loot` → `true`
- **Mismatch**: `LootCorpse { corpse: X }` with `authoritative_targets: [Y]` and `op_kind: Loot` → `false`
- **Auxiliary bypass**: `LootCorpse { corpse: X }` with `authoritative_targets: [destination]` and `op_kind: Travel` → `true`
- **Empty targets bypass**: `LootCorpse { corpse: X }` with `authoritative_targets: []` and `op_kind: Loot` → `true`
- **Flexible goal**: `Sleep` with any `authoritative_targets` and any `op_kind` → `true`

Repeat for each exact-bound variant: `EngageHostile`, `Heal`, `ShareBelief`, `ClaimOffice`, `SupportCandidateForOffice`.

### Search Integration Tests
- **Two corpses at same place**: Agent with `LootCorpse { corpse: X }` goal at a place with corpses X and Y. Search must only produce Loot candidates targeting X, not Y.
- **Two hostiles at same place**: Agent with `EngageHostile { target: A }` must only produce Attack candidates targeting A.
- **Flexible goal unaffected**: Agent with `Sleep` goal must accept any available sleep affordance regardless of targets.

### Binding Rejection Trace Tests
- Verify `BindingRejection` entries appear in `PlanAttemptTrace` when wrong-target candidates are filtered
- Verify trace includes `def_id`, `rejected_target`, and `required_target`
- Verify `dump_agent()` output includes binding rejection information

### Not Tested (Documented for Future)
- `BuryCorpse` regression test — no action def or handler exists yet. When the `bury` action is implemented, add exact corpse + exact burial-site binding tests.

## Acceptance Criteria
- exact-bound goals cannot silently retarget to a sibling affordance of the same family
- flexible goals remain flexible
- planner determinism is preserved
- blocker memory and debugging refer to the correct concrete target
- binding rejection traces expose which candidates were rejected and why
- `DeclareSupport` with empty `bound_targets` continues to work via payload override
- planner-only synthetic candidates with empty `authoritative_targets` bypass binding
- future drafts for listed-lot trade, stock storage, and audits can reuse the same `matches_binding` architecture without introducing new special cases

## FND-01 Section H

### Information-Path Analysis
- Exact target binding only applies to objects already present in the agent's grounded goal evidence.
- The planner still cannot query raw world state for "better" unseen alternatives.
- Future sale-lot, corpse, audit, and evidence interactions remain local because affordances expose only locally knowable concrete objects.
- No information path is added that would let agents know about hidden objects merely because the action family exists.
- `DeclareSupport` targets flow through payload overrides built from goal fields, not through affordance `bound_targets`. This is consistent with information locality because the goal fields were derived from local observations during candidate generation.

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
- goal canonical identity fields (`GoalKind` variant fields → `GoalKey`)

Derived / transient state:
- grounded goal evidence sets
- `matches_binding()` decisions (computed per candidate per tick)
- `BindingRejection` trace entries (opt-in diagnostic, never authoritative)
- affordance-binding match results
- blocker classifications for failed exact-bound steps

No derived binding result may become authoritative truth.

## Recommended Sequencing
When this spec is scheduled, it should be implemented before or alongside:
- listed-lot trade from [S04-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/S04-merchant-selling-market-presence.md)
- facility stock/display targeting from [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md)
- exact evidence targeting in E17 crime/theft/justice follow-on work

Otherwise those efforts will likely reintroduce one-off exact-target hacks.

## Outcome

- **Completion date**: 2026-03-17
- **What changed**: Implemented across 4 tickets (S03PLATARIDE-001 through -004). Added `matches_binding()` to `GoalKindPlannerExt` in `goal_model.rs` with auxiliary-pass/terminal-check dispatch. Added `.retain()` binding filter in `search_candidates()` in `search.rs`. Added `BindingRejection` trace struct and `binding_rejections` field on `PlanAttemptTrace` in `decision_trace.rs`, wired through `search_plan` and `agent_tick.rs` trace propagation. Added unit tests for all 17 `GoalKind` variants' binding behavior and 5 search integration tests covering two-corpses, two-hostiles, flexible-goal, rejection-trace, and empty-targets-bypass scenarios.
- **Deviations**: `BindingRejection` uses `rejected_targets: Vec<EntityId>` (plural) and `required_target: Option<EntityId>` rather than the spec's singular `rejected_target: EntityId`, to handle multi-target action defs uniformly. Search integration tests placed in `search.rs` test module (co-located with `TestBeliefView` infrastructure) rather than `agent_tick.rs`.
- **Verification**: All unit and integration tests pass. `cargo test --workspace` and `cargo clippy --workspace` clean. Deliverables 1-3, 8 implemented; deliverables 4-7 verified as already correct (no changes needed).
