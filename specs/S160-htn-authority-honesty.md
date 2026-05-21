# S160 — HTN Authority Honesty

**Status:** Draft
**Type:** Correctness/honesty fix (planner-local schema metadata + trace, plus an
escort action-payload field migration in `worldwake-sim`/`worldwake-systems`; no
new authoritative simulation state, no method-required goals)
**Priority:** Medium. Sequence after archived S158; independent of S159.
**Crates:** `worldwake-ai` (schema enum, trace, support-declared direct bounty
method, planner payload-override), `worldwake-sim` (`EscortToSafetyActionPayload` field type),
`worldwake-systems` (escort affordance enumeration, runtime heal resolution,
contention read).
**Foundations:** FND-20, FND-27, FND-28, FND-29, FND-31
**Extends:** `archive/specs/S156-htn-authority-honesty.md` (first iteration
stripped the `GoalSchema.methods` fossil, dead methods, and unenforced schema
fields, and made strategic fallback explicit/traced). This spec closes the
remaining honesty gaps the second iteration found: subgoal-authority labeling,
the renamed support-declared direct bounty method, and the `u32::MAX` escort
sentinel.

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
  `EscortToSafety` (1). Test `registry_builds_with_11_methods_without_dead_method_ids`
  (`registry.rs:73`) asserts the count.
- Before ticket S160HTNAUTHHON-003,
  `htn/methods.rs::fulfill_bounty_group_hunt` declared subgoals
  `[DeclareSupport (social signal), TravelTo(staging), Attack(target)]` while
  naming group coordination the world could not enforce. Ticket
  S160HTNAUTHHON-003 renamed it to
  `fulfill_bounty_support_declared_direct` and rewrote the local comment; there
  is still **no recruit/coordination action leaf**, and the actual confrontation
  remains a solo `Attack` after the support signal.
- `htn/method_schema.rs::SubgoalTemplate` has 8 variants (`AcquireCommodity`,
  `TravelTo`, `ObserveTarget`, `AskWitness`, `InspectArtifact`, `PerformAction`,
  `ResolveCoordination`, `ReturnTo`). `search/strategic.rs::template_to_stages`
  (`strategic.rs:541`) mechanically maps each to a `StrategicStage` (place list);
  **no validation** that a `PerformAction(op, payload)` resolves to a real
  `ActionDefId` or that the payload template binds. Subgoals are stage hints.
- The escort `intended_heal_action: ActionDefId` field is defined in
  `worldwake-sim/src/action_payload.rs:396` and built with the
  `ActionDefId(u32::MAX)` sentinel ("resolved at runtime" placeholder, FND-28
  fossil-seed risk) at **two** construction sites:
  - `goal_model.rs:962` — planner payload-override path (`build_payload_override`,
    `worldwake-ai`).
  - `escort_actions.rs:210` — affordance-enumeration path (`enumerate_escort_payloads`
    builds it via `build_escort_payload(.., ActionDefId(u32::MAX))`,
    `worldwake-systems`).
  The sentinel is overwritten at action start by
  `escort_actions.rs:401` (`payload.intended_heal_action = heal_action_id(context.action_defs)?`)
  and read at `escort_actions.rs:600` (`enqueue_for_contention`).

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
   Attach the authority label per subgoal (the per-subgoal granularity is what
   `RequiredActionLeaf` requires); the exact carrier — a wrapper struct around
   `SubgoalTemplate`, or pairing in `MethodSchema.subgoals` — is a ticket-time
   detail, but it must touch the `method_schema!` macro and all 11 method
   definitions so every current subgoal is labeled `StageHint` (honest
   classification of present behavior).

   **Both variants ship now, with a negative test as the present-tense
   consumer.** `RequiredActionLeaf` is defined but **unused** at landing, and a
   test must assert that no current method declares it. This mirrors the existing
   `test_relevant_ops_authority_is_hint_only_at_landing` precedent
   (`goal_schema.rs:1173`): the negative test is the live consumer that gives the
   variant present-tense meaning and guards against premature method-required
   labeling, satisfying `docs/spec-drafting-rules.md` rule 5 ("enforced
   declarations only") via the same pattern the codebase already accepts for
   `relevant_ops`. The *strategic-search enforcement* for `RequiredActionLeaf`
   (the search check + trace that proves a required leaf mapped to a real planned
   `ActionDef`) is deferred to the first real method-required method (a future
   spec). Defining both variants now is required for Deliverable 2's stage-hint-
   vs-enforced trace distinction to be non-vacuous; a single-variant enum would
   convey no distinction.
2. **Honest stage-hint traces** in `htn/selector.rs` / `search/strategic.rs` /
   `decision_trace.rs`: the method trace must distinguish stage-hint subgoals from
   enforced leaves so a reader cannot mistake "method selected" for "subgoal
   enforced." The natural carrier is a `MethodSubgoalAuthority` field on
   `SubgoalAttemptResult` (`decision_trace.rs:1269–1274`), populated alongside the
   existing `kind`/`outcome` fields when `MethodPlanAttemptTrace.subgoals_attempted`
   is built in `search/strategic.rs`. Extends the existing
   `MethodPlanAttemptTrace` contract documented in `docs/planner-contracts.md` §4
   — do not add a second trace subsystem.
3. **Resolved the fake group-hunt method (rename, option b).**
   S160HTNAUTHHON-003 renamed `fulfill_bounty_group_hunt` to
   `fulfill_bounty_support_declared_direct`, reflecting "declare support, then
   pursue directly" with no claim of enforced group coordination. This preserves
   the `DeclareSupport` social signal — a real world artifact (the
   `declare_support` action exists, `planner_ops.rs:44`/`131`) — while removing the
   misleading "group hunt" promise. Removal (option a) is rejected: the
   `DeclareSupport` stage is a lawful belief-backed step worth keeping.

   Renamed touch points (the registry **count** test is *unaffected* — a
   rename keeps 11 methods):
   - the method fn name;
   - the registry insert;
   - the selector test, now
     `support_declared_direct_selects_from_real_belief_preconditions`, and its
     `.expect(...)` assert message.
4. **Remove the `ActionDefId(u32::MAX)` escort sentinel (option A, cross-crate).**
   Change `EscortToSafetyActionPayload.intended_heal_action` from `ActionDefId` to
   `Option<ActionDefId>` (`None` until resolved at action start). This is the
   honest representation of the existing resolve-at-start flow.

   Option B (resolve the real heal `ActionDefId` at payload construction) is
   **rejected**: neither construction site has the `ActionDefRegistry` in scope —
   `enumerate_escort_payloads` receives only `_def: &ActionDef`
   (`escort_actions.rs:191`), and the planner `build_payload_override`
   (`goal_model.rs:950`) has no registry. The registry is only available at action
   start, which is exactly where resolution already happens.

   All sites the field-type change touches:
   - **Field definition:** `worldwake-sim/src/action_payload.rs:396` →
     `pub intended_heal_action: Option<ActionDefId>`.
   - **Construction (planner payload-override):** `goal_model.rs:962` → `None`.
   - **Construction (affordance enumeration):** `escort_actions.rs:210` /
     `build_escort_payload` (`escort_actions.rs:165–177`) → `None`.
   - **Runtime resolution:** `escort_actions.rs:401` →
     `payload.intended_heal_action = Some(heal_action_id(context.action_defs)?)`.
   - **Contention read:** `escort_actions.rs:600` (`enqueue_for_contention`) must
     handle `Option` (resolution precedes enqueue at action start; treat a `None`
     here as an internal error, not a silent skip).
   - **Test sample:** `action_payload.rs:696` (`sample_escort_to_safety_payload`)
     → `Some(ActionDefId(27))`.

   Add a test asserting no plan, action trace, or dispatch ever observes a
   placeholder/sentinel action ID (with `Option`, the pre-resolution state is
   `None`, so there is no sentinel `ActionDefId` to observe).
5. **`docs/planner-contracts.md` §4** — add the stage-hint-vs-required-leaf
   distinction to the HTN trace contract language (the section already documents
   `MethodPlanAttemptTrace` at lines 295–326).

## Authoritative-to-AI Impact Analysis

Deliverable 4 modifies affordance generation (`enumerate_escort_payloads`) and a
validation function (`validate_escort_payload`) and changes a planner-synthesized
payload (`build_payload_override`), so the CLAUDE.md checklist applies:

1. `get_affordances` — pass. Escort affordances are still enumerated; `None`
   replaces the sentinel.
2. `generate_candidates` — pass. Escort goal emission is unchanged.
3. `search_plan` — pass. The payload-override produces `None` instead of the
   sentinel.
4. `BestEffort` action start — **handle**: `escort_actions.rs:401` sets
   `Some(heal_action_id(...))`; the read at `escort_actions.rs:600` must handle
   `Option`.
5. `handle_plan_failure` — N/A. No precondition semantics change.
6. **Payload revalidation** — **handle**: both construction sites must agree
   (`None == None`) so `requested_affordance_matches` /
   `with_payload_override_validator` does not reject the escort step on a
   planner-vs-affordance payload mismatch.
7. Golden tests — must pass (`cargo test -p worldwake-ai`); behavior of the
   renamed method is identical to its prior behavior (solo pursuit after support
   declaration).

## FND-01 Section H Analysis

Honesty/cleanup change; introduces planner-local schema metadata and trace
fields, and migrates an existing escort action-payload field type. No new
authoritative simulation state, action, component, or feedback loop.

- **Information-path analysis:** Not applicable. Method selection already reads
  only belief-backed preconditions; this spec adds no new reads. (S158 governs the
  belief-source correctness of those reads.)
- **Positive-feedback analysis:** Not applicable. No amplifying loop.
- **Concrete dampeners:** Not applicable.
- **Stored-state vs. derived read-model list:** No new authoritative state.
  `MethodSubgoalAuthority` is a static schema label; `MethodPlanAttemptTrace`
  remains a transient debug read-model (per `docs/planner-contracts.md` §4, not
  serialized save/replay state). The escort field change is a type migration on an
  existing in-flight action payload (`ActionDefId` → `Option<ActionDefId>`), not a
  new authoritative fact; per FND-28 the migration replaces the sentinel outright
  rather than coexisting with it. No derived value is promoted to truth (FND-27).
- **Planner-formalism analysis:** The current behavior is **HTN method
  decomposition over existing affordances, with legal flat-GOAP fallback**. This
  spec labels that honestly and does not change it. No goal becomes
  method-required: the schema-contract burden (FND-20, `docs/spec-drafting-rules.md`
  HTN checklist) is unmet for all 11 methods.

### HTN Method Drafting Checklist (per spec-drafting-rules)

This spec changed the group-hunt method surface and adds authority labeling; it
adds no new pursuit pattern.

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
   labels make the *unenforced* status explicit. `RequiredActionLeaf` ships with a
   negative test as its present-tense consumer (mirroring the `relevant_ops`
   hint-only precedent at `goal_schema.rs:1173`); its strategic-search enforcement
   is deferred to the first method that needs it (Deliverable 1). The `u32::MAX`
   sentinel (an unenforced placeholder) is removed (Deliverable 4).
6. **Proof surface:** see below.

### Proof surface (FND-31)

- Negative test: no current method declares `RequiredActionLeaf` (guards against
  premature method-required), mirroring `test_relevant_ops_authority_is_hint_only_at_landing`.
- Trace test: method trace distinguishes stage-hint subgoals from enforced leaves;
  a selected method with only stage hints is not reported as having enforced its
  subgoals.
- Group-hunt resolution test: registry no longer exposes a method *named* group
  hunt without a real coordination leaf (the selector test was renamed to
  `support_declared_direct_selects_from_real_belief_preconditions`; the
  11-method count test is unchanged because option (b) is a rename).
- Sentinel test: no plan, action trace, or dispatch observes a placeholder action
  ID (pre-resolution state is `None`).
- Payload-revalidation test: the escort step survives revalidation with both
  construction sites producing `None` (no planner-vs-affordance payload mismatch).
- All existing `worldwake-ai` HTN goldens pass (behavior of the renamed method is
  identical to its prior behavior — solo pursuit after support declaration).
