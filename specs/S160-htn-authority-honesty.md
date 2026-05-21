# S160 — HTN Authority Honesty

**Status:** Draft
**Type:** Correctness/honesty fix (planner-local metadata + trace; no new
simulation state, no method-required goals)
**Priority:** Medium. Sequence after archived S158; independent of S159.
**Foundations:** FND-20, FND-29, FND-31
**Extends:** `archive/specs/S156-htn-authority-honesty.md` (first iteration
stripped the `GoalSchema.methods` fossil, dead methods, and unenforced schema
fields, and made strategic fallback explicit/traced). This spec closes the
remaining honesty gaps the second iteration found: subgoal-authority labeling,
the still-fake group-hunt method, and the `u32::MAX` escort sentinel.

## Problem Statement

### Motivation

The HTN layer names imply more enforcement than the live code provides. Method
schemas declare rich subgoal templates (`PerformAction`, `ResolveCoordination`,
`ReturnTo`, etc.), but `search/strategic.rs` converts them into *strategic stages*
with no enforcement that a declared subgoal maps to an actual planned
`ActionDef` leaf. This is acceptable as **method-guided strategic search**, but
the schema surface overpromises (FND-20: planner formalisms may encode search
control, not authority they do not enforce; FND-29: traces must prove what the
code actually constrained, not decorate intended behavior). Two concrete smells
also exist: a method that fakes coordination, and a sentinel placeholder action
ID that can rot into a fossil (FND-28).

### Evidence (verified against code on 2026-05-21)

- `htn/registry.rs` builds **11 methods** across `FulfillBounty` (3),
  `ProduceCommodity` (3), `RestockCommodity` (2), `InvestigateViolation` (2),
  `EscortToSafety` (1). Test `registry_builds_with_11_methods_…` asserts the count.
- `htn/methods.rs::fulfill_bounty_group_hunt` declares subgoals
  `[DeclareSupport (social signal), TravelTo(staging), Attack(target)]`. A code
  comment states *"Existing planner ops have no RecruitAlly leaf. DeclareSupport
  is the first-ship social signal for assembling a lawful group hunt."* There is
  **no recruit/coordination action leaf**; the actual confrontation is a solo
  `Attack`. The method name promises group coordination the world cannot enforce
  (FND-20, FND-31).
- `htn/method_schema.rs::SubgoalTemplate` has 8 variants including
  `PerformAction`, `ResolveCoordination`, `ReturnTo`. `search/strategic.rs::
  template_to_stages` mechanically maps each to a `StrategicStage` (place list);
  **no validation** that a `PerformAction(op, payload)` resolves to a real
  `ActionDefId` or that the payload template binds. Subgoals are stage hints.
- `goal_model.rs` (~L958–966) builds `EscortToSafetyActionPayload` with
  `intended_heal_action: ActionDefId(u32::MAX)` — a sentinel "resolved at runtime"
  placeholder (FND-28 fossil-seed risk).

### Key scoping decisions (brainstorm 2026-05-21)

- **No method becomes method-required.** The burden of proof (flat fallback
  semantically illegal, required leaves mapped to `ActionDef`, trace proves each
  leaf, golden fails if fallback bypasses the method, failure produces state) is
  not met by any current method. This spec only makes the *current* honesty
  explicit; it does not expand HTN authority.
- This is honesty + cleanup, not a new coordination system. A real group-hunt
  needs recruit/contract/grant artifacts (a future gameplay spec), out of scope.

## Deliverables

1. **`MethodSubgoalAuthority` enum** in `htn/method_schema.rs`:
   ```
   pub enum MethodSubgoalAuthority {
       /// Subgoal contributes strategic destinations, prerequisite commodities,
       /// or trace context. Not enforced as an ordinary ActionDef leaf.
       StageHint,
       /// Subgoal must correspond to at least one ordinary ActionDef-backed
       /// planned step, and the trace must prove selected/skipped/failed status.
       RequiredActionLeaf,
   }
   ```
   Every current subgoal is labeled `StageHint` (honest classification of present
   behavior). `RequiredActionLeaf` is defined but **unused** at landing — and a
   test must assert no current method declares it (negative test against premature
   method-required, mirroring `relevant_ops` hint-only and the FOUNDATIONS rule).
   Per `docs/spec-drafting-rules.md` rule 5 ("enforced declarations only"), the
   `RequiredActionLeaf` variant must ship **with** its enforcing consumer (a
   strategic-search check + trace) even though no method uses it yet; if shipping
   the consumer is deferred, the variant must be deferred with it. Decide at ticket
   time: either land variant+consumer together, or land only `StageHint` labeling
   now and defer `RequiredActionLeaf` to the first method that needs it.
2. **Honest stage-hint traces** in `htn/selector.rs` / `search/strategic.rs` /
   `decision_trace.rs`: the method trace must distinguish stage-hint subgoals from
   enforced leaves so a reader cannot mistake "method selected" for "subgoal
   enforced." Extends the existing `MethodPlanAttemptTrace` contract documented in
   `docs/planner-contracts.md` §4 — do not add a second trace subsystem.
3. **Resolve the fake group-hunt method.** Either:
   - a) Remove `fulfill_bounty_group_hunt` until real coordination artifacts
     exist; or
   - b) Rename it to its honest behavior (e.g. `fulfill_bounty_support_declared_
     direct`) reflecting "declare support, then pursue directly" with no claim of
     enforced group coordination.
   Recommendation: (b) — it preserves the `DeclareSupport` social signal (a real
   world artifact) while removing the misleading "group hunt" promise. Update the
   11-method registry test accordingly.
4. **Remove the `ActionDefId(u32::MAX)` sentinel.** Replace the escort
   `intended_heal_action` placeholder with an honest representation:
   `Option<ActionDefId>` (None until resolved) or resolve the real heal
   `ActionDefId` at payload construction. Add a test asserting no plan, action
   trace, or dispatch ever observes a placeholder/sentinel action ID.
5. **`docs/planner-contracts.md` §4** — add the stage-hint-vs-required-leaf
   distinction to the HTN trace contract language.

## FND-01 Section H Analysis

Honesty/cleanup change; introduces planner-local schema metadata and trace
fields, no new simulation state, action, component, or feedback loop.

- **Information-path analysis:** Not applicable. Method selection already reads
  only belief-backed preconditions; this spec adds no new reads. (S158 governs the
  belief-source correctness of those reads.)
- **Positive-feedback analysis:** Not applicable. No amplifying loop.
- **Concrete dampeners:** Not applicable.
- **Stored-state vs. derived read-model list:** No new authoritative state.
  `MethodSubgoalAuthority` is a static schema label; `MethodPlanAttemptTrace`
  remains a transient debug read-model (per `docs/planner-contracts.md` §4, not
  serialized save/replay state). No derived value is promoted to truth (FND-27).
- **Planner-formalism analysis:** The current behavior is **HTN method
  decomposition over existing affordances, with legal flat-GOAP fallback**. This
  spec labels that honestly and does not change it. No goal becomes
  method-required: the schema-contract burden (FND-20, `docs/spec-drafting-rules.md`
  HTN checklist) is unmet for all 11 methods.

### HTN Method Drafting Checklist (per spec-drafting-rules)

This spec **changes** the group-hunt method surface and adds authority labeling;
it adds no new pursuit pattern.

1. **Reusable pursuit pattern:** None added. Existing patterns (arm/travel/attack,
   acquire/craft/return, restock, witness/ledger investigation, escort) are
   re-labeled, not extended.
2. **Why flat GOAP is insufficient:** Unchanged — current methods provide
   multi-stage decomposition/search control. This spec does not assert new
   insufficiency; flat fallback stays legal for every method.
3. **Fallback policy:** Flat-GOAP fallback **remains allowed** for all methods. No
   method-required contract is introduced.
4. **Information reads:** No new reads. Group-hunt rename preserves the existing
   `DeclareSupport` / `AllyOrBountyOfficeAvailable` belief reads.
5. **Enforced declarations only:** This is the spec's core fix — `StageHint`
   labels make the *unenforced* status explicit, and the unused `RequiredActionLeaf`
   variant must ship with its enforcing consumer or be deferred (Deliverable 1).
   The `u32::MAX` sentinel (an unenforced placeholder) is removed (Deliverable 4).
6. **Proof surface:** see below.

### Proof surface (FND-31)

- Negative test: no current method declares `RequiredActionLeaf` (guards against
  premature method-required).
- Trace test: method trace distinguishes stage-hint subgoals from enforced leaves;
  a selected method with only stage hints is not reported as having enforced its
  subgoals.
- Group-hunt resolution test: registry no longer exposes a method *named* group
  hunt without a real coordination leaf (count/name assertions updated).
- Sentinel test: no plan, action trace, or dispatch observes a placeholder action
  ID.
- All existing `worldwake-ai` HTN goldens pass (behavior of the renamed method is
  identical to its prior behavior — solo pursuit after support declaration).
```
