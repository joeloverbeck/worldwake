# S108: Per-Action Binding Strictness

**Status**: COMPLETED

## Summary

Tighten `ActionRequestMode::BestEffort` substitution by attaching an explicit `BindingStrictness` classifier to every `ActionDef`. Today BestEffort is permissive-by-default: when the unified matcher finds no identity match, `resolve_affordance` (`crates/worldwake-sim/src/tick_step.rs:504`) synthesizes an `Affordance` directly from the raw requested targets, regardless of whether the step semantically requires a specific identity (accuse *this* suspect, transfer *this* item, loot *this* corpse, escort *this* subject). Classify each action on the spectrum `ExactIdentity → FungibleEquivalentCommodity → EquivalentWorkstationTagAtSamePlace → EquivalentRouteStep → AnyLegalTarget`, gate the BestEffort synthesis site in `resolve_affordance` through a dedicated `check_binding_strictness` helper, apply the same helper to the two best-effort-like fallbacks in `plan_revalidation.rs`, and refuse substitutions that would cross the strictness boundary. Landed note: the current dispatch gate rejects underbound or malformed `ExactIdentity` BestEffort requests at request resolution, while fully bound stale exact-identity requests still flow into authoritative start-time revalidation instead of dying at `resolve_affordance`.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Completed and ready for archival.

## Crates

- `worldwake-sim` — `BindingStrictness` enum, `ActionDef::binding_strictness`, `check_binding_strictness` helper, strictness gate in `resolve_affordance` (`tick_step.rs`) before the BestEffort synthesis fallback.
- `worldwake-ai` — `plan_revalidation.rs` keeps its existing same-target fallback revalidation helpers; reassessment showed they are not alternate-target substitution surfaces, so no blanket `check_binding_strictness` gate was added there. Decision trace records the strictness class at dispatch via `PlannedStepSummary`.
- `worldwake-systems` — classify each registered action at its `ActionDef` registration site (`accuse`, `loot`, `heal`, `escort_to_safety`, `pick_up`, `put_down`, `drop_item`, `steal`, `eat`, `drink`, `trade`, `travel`, `patrol`, `post_bounty`, `post_notice`, `claim_bounty`, `bribe`, `threaten`, `declare_support`, etc.).

## Dependencies

- None as a hard dependency. Builds on the existing unified legality path (`with_payload_override_validator`, `requested_affordance_matches` in `affordance_query.rs`).
- Soft coupling to S109 (Typed Discrepancy Taxonomy): `check_binding_strictness`'s `ExactIdentityRequired` result feeds S109's `Discrepancy::NoLegalBinding`. S108 is standalone-valuable and maps its refusals to the existing `BlockingFact::AssumptionFailed`; S109 later refines that mapping.
- Soft coupling to S110 (Decision History Events): the new strictness class should appear in the dispatch-time event payload once S110 lands.

## Design Goals

- Every socially or materially identity-bound action refuses BestEffort substitution that silently redirects to a different counterparty, item, or target.
- Strictness is declared at action-registration time, not inferred at dispatch. No per-handler ad-hoc checks.
- Strictness is **orthogonal to `TargetSpec`**: `TargetSpec` answers "which entities can be enumerated for this action?"; `BindingStrictness` answers "once the planner has bound a specific entity, may the dispatcher substitute?" Two actions with the same `TargetSpec` (e.g., `pick_up` and `loot`, both reaching items/corpses at actor's place) can carry different strictness.
- Affordance enumeration and dispatch consult the shared classifier directly. Planner revalidation still uses its existing same-target fallback helpers, which reassessment showed are not alternate-target substitution paths; the live failure boundary for stale fully bound exact-identity steps therefore remains "primary revalidation miss first, then authoritative dispatch/start-time validation if the same-target step still survives."
- Strictness drives the failure classification surface: an `ExactIdentity` request that cannot find the exact target records a different discrepancy (`NoLegalBinding`, via S109) than an `AnyLegalTarget` fallback that found nothing available (`SellerOutOfStock`).

## Non-Goals

- Revisiting the unified legality path more broadly. The per-affordance identity predicate `requested_affordance_matches` keeps its current signature and semantics; the strictness gate is a new helper invoked at the BestEffort-substitution sites.
- Full discrepancy taxonomy rework — covered separately by S109.
- Per-action whitelisting of substitute targets. Strictness is coarse-grained (5 classes) and covers the common cases; finer-grained rules wait for concrete scenarios.
- Removing `revalidate_exact_target_step` in this spec. That function's exact-identity semantics for all-`TargetSpec::SpecificEntity` action definitions already partially subset `ExactIdentity`; consolidation into the strictness classifier is a follow-up (see "Open Migration Work" below).

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-4 (Persistent Identity, Object Permanence, Explicit Transfer) | `ExactIdentity` strictness forbids silent retargeting of identity-bearing actions (accuse, loot, heal, escort). The same exact entity that appeared in the plan must still be the target at dispatch, or the step fails lawfully through revalidation. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | Binding strictness is declared alongside preconditions and cost, formalizing the binding contract of an action. |
| FND-14 (World State Is Not Belief State) | Planner-synthesized BestEffort requests originate from the agent's belief state. When the world diverges from belief (target gone, moved, or replaced), the failure must surface through revalidation, not silent substitution against authoritative world state. Strictness is the gate that forces that surfacing. |
| FND-20 (Resource-Bounded Practical Reasoning) | Declaring strictness at registration avoids hidden exception logic inside the planner or dispatcher. Every substitution decision is traceable to an authored classifier. |
| FND-21 (Intentions Are Revisable Commitments) | An `ExactIdentity` step whose target is gone must fail through the normal revalidation + replan path, not silently substitute a different entity. |
| FND-24 (Ownership, Custody, Access, Obligation, Jurisdiction Are Distinct) | Identity-bound actions (loot, accuse within jurisdiction, escort) enforce the target-identity contract that ownership/custody/jurisdiction rules depend on. |
| FND-26 (Systems Interact Through State) | `ActionDef::binding_strictness` is authoritative state that the dispatcher and revalidator both read; no hidden cross-system call or derived logic. |
| FND-29 (Debuggability Is a Product Feature) | The dispatch-time trace records the classifier, so "why did this BestEffort request refuse to substitute?" has a direct answer. |

## Deliverables

### D1: `BindingStrictness` enum

New type in `crates/worldwake-sim/src/action_def.rs` (alongside `ActionDef`):

```rust
/// How permissive BestEffort target substitution is for a given action.
/// Declared once on each ActionDef at registration. Orthogonal to TargetSpec:
/// TargetSpec governs enumeration, BindingStrictness governs post-binding
/// substitution.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BindingStrictness {
    /// The exact entity referenced in the request must be present and
    /// eligible. Substitution is forbidden. Example actions:
    /// accuse, loot, heal, escort_to_safety, bribe, threaten,
    /// press_force_claim, yield_force_claim, declare_support,
    /// queue_for_corpse_use, queue_for_care_target, claim_bounty.
    ExactIdentity,
    /// The target may be substituted with any entity of the same
    /// fungible commodity kind at the same place. Example actions:
    /// pick_up, eat, drink.
    FungibleEquivalentCommodity,
    /// The target may be substituted with any entity carrying the same
    /// WorkstationTag at the same place. Example actions:
    /// queue_for_facility_use, and the recipe-backed harvest/craft
    /// action defs when their workstation target is generic.
    EquivalentWorkstationTagAtSamePlace,
    /// The target may be substituted with any route edge that reaches
    /// the same destination place. Example actions: travel, patrol.
    EquivalentRouteStep,
    /// The target may be substituted with any legal target that
    /// satisfies the action's preconditions. Example actions:
    /// investigate, search_place, ask_witness, ask_about_person,
    /// report_missing, report_found, consult_record, tell,
    /// staff_market, stage_stock_for_sale, unstage_stock, store_stock,
    /// collect_display_stock, trade. Use sparingly and only when target
    /// identity is not part of the step's semantic commitment.
    AnyLegalTarget,
}
```

### D2: `ActionDef` extension

Add a non-`Option` `binding_strictness: BindingStrictness` field to the `ActionDef` record. Every existing action receives an explicit classification at its registration site — there is no `Default` impl on the field and no `Default` impl on `ActionDef` that would bypass authorial choice. The compiler enforces completeness.

Because `ActionDef` derives `Serialize, Deserialize` and participates in save/replay, the field uses `#[serde(default = "BindingStrictness::exact_identity_default")]` for deserialization compatibility with old saves; the default function returns `BindingStrictness::ExactIdentity` (the most conservative class — refuses substitution). This keeps construction-site completeness strict without breaking replay, and any old-save action whose true class was permissive will simply fail BestEffort substitution in a recoverable way until the save is re-emitted.

Authoritative classification for every currently-registered action:

| Action (registered name) | Registration site | Strictness |
|--------------------------|-------------------|------------|
| `accuse` | `justice_actions.rs:62` | `ExactIdentity` |
| `fine` | `justice_actions.rs` | `ExactIdentity` |
| `exile` | `justice_actions.rs` | `ExactIdentity` |
| `attack` | `combat.rs:402` | `ExactIdentity` |
| `defend` | `combat.rs:460` | `ExactIdentity` |
| `loot` | `combat.rs:496` | `ExactIdentity` |
| `bury` | `combat.rs:548` | `ExactIdentity` |
| `heal` | `combat.rs:816` | `ExactIdentity` |
| `queue_for_corpse_use` | `combat.rs:582` | `ExactIdentity` |
| `queue_for_care_target` | `combat.rs:856` | `ExactIdentity` |
| `pick_up` | `transport_actions.rs:58` | `FungibleEquivalentCommodity` |
| `put_down` | `transport_actions.rs:94` | `FungibleEquivalentCommodity` |
| `drop_item` | `transport_actions.rs:126` | `FungibleEquivalentCommodity` |
| `steal` | `transport_actions.rs:158` | `ExactIdentity` |
| `travel` | `travel_actions.rs:28` | `EquivalentRouteStep` |
| `patrol` | `patrol_actions.rs:28` | `EquivalentRouteStep` |
| `investigate` | `investigate_actions.rs:37` | `AnyLegalTarget` |
| `tell` | `tell_actions.rs:41` | `ExactIdentity` (the listener is the identity contract) |
| `post_bounty` | `artifact_actions.rs:85` | `ExactIdentity` (the posting place binds) |
| `post_notice` | `artifact_actions.rs:124` | `ExactIdentity` |
| `claim_bounty` | `artifact_actions.rs:163` | `ExactIdentity` |
| `ask_about_person` | `ask_about_person_actions.rs:49` | `ExactIdentity` (the subject binds) |
| `ask_witness` | `epistemic_actions.rs:47` | `ExactIdentity` |
| `escort_to_safety` | `escort_actions.rs:49` | `ExactIdentity` |
| `establish_camp` | `bandit_camp_actions.rs:36` | `ExactIdentity` (the place binds) |
| `trade` | `trade_actions.rs:39` | `ExactIdentity` (counterparty is the identity contract) |
| `staff_market` | `trade_actions.rs:1345` | `ExactIdentity` |
| `queue_for_facility_use` | `facility_queue_actions.rs:35` | `EquivalentWorkstationTagAtSamePlace` |
| `bribe` | `office_actions.rs:135` | `ExactIdentity` |
| `threaten` | `office_actions.rs:180` | `ExactIdentity` |
| `declare_support` | `office_actions.rs:225` | `ExactIdentity` |
| `press_force_claim` | `office_actions.rs:246` | `ExactIdentity` |
| `yield_force_claim` | `office_actions.rs:267` | `ExactIdentity` |
| `consult_record` | `consult_record_actions.rs:36` | `ExactIdentity` (the specific record binds) |
| `report_missing` | `report_actions.rs:60` | `ExactIdentity` (the subject binds) |
| `report_found` | `report_actions.rs:105` | `ExactIdentity` |
| `relieve_wilderness` | `needs_actions.rs:100` | `AnyLegalTarget` (no target) |
| `search_place` | `search_actions.rs:38` | `AnyLegalTarget` |
| `store_stock` | `stock_actions.rs:60` | `ExactIdentity` (the specific lot binds) |
| `collect_display_stock` | `stock_actions.rs:90` | `ExactIdentity` |
| `stage_stock_for_sale` | `stock_actions.rs:120` | `ExactIdentity` |
| `unstage_stock` | `stock_actions.rs:154` | `ExactIdentity` |
| `eat` | `needs_actions.rs` (`register_def`) | `FungibleEquivalentCommodity` |
| `drink` | `needs_actions.rs` (`register_def`) | `FungibleEquivalentCommodity` |
| `sleep` | `needs_actions.rs` (`register_def`) | `AnyLegalTarget` (no target) |
| `toilet` | `needs_actions.rs` (`register_def`) | `EquivalentWorkstationTagAtSamePlace` |
| `wash` | `needs_actions.rs` (`register_def`) | `EquivalentWorkstationTagAtSamePlace` |
| `Harvest Apples`, `Harvest Grain`, `Harvest Water`, `Bake Bread`, and other recipe-backed action defs | `production_actions.rs`, `action_registry.rs` | `EquivalentWorkstationTagAtSamePlace` |

Borderline classifications (`trade`, `tell`, `post_bounty`, `report_missing`, etc.) are explicit authorial calls because the identity contract is social, not enumerated. Edge cases are confirmed at ticket-implementation time with reference to the action's documented binding semantics.

### D3: `check_binding_strictness` helper and gate at `resolve_affordance`

Add a new helper in `crates/worldwake-sim/src/affordance_query.rs`:

```rust
/// Result of checking whether BestEffort substitution is allowed for a
/// given (action, request) pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrictnessGate {
    /// Substitution is allowed under the action's strictness class.
    /// The caller may proceed with BestEffort synthesis constrained to
    /// the matching substitution pool (fungible commodity / workstation
    /// tag / route destination / any legal target).
    SubstitutionAllowed(BindingStrictness),
    /// Substitution is forbidden for this action (ExactIdentity) and
    /// the exact target was not matched by any live affordance. The
    /// caller must reject the request and surface a typed failure.
    ExactIdentityRequired,
}

#[must_use]
pub fn check_binding_strictness(
    def: &ActionDef,
    mode: ActionRequestMode,
) -> StrictnessGate {
    match (mode, def.binding_strictness) {
        (ActionRequestMode::Strict, _) => StrictnessGate::SubstitutionAllowed(def.binding_strictness),
        (ActionRequestMode::BestEffort, BindingStrictness::ExactIdentity) => {
            StrictnessGate::ExactIdentityRequired
        }
        (ActionRequestMode::BestEffort, class) => StrictnessGate::SubstitutionAllowed(class),
    }
}
```

The signature and return type of `requested_affordance_matches` are unchanged. `check_binding_strictness` is a pure function of `ActionDef` and `ActionRequestMode`.

### D4: Dispatch enforcement in `resolve_affordance`

Modify `crates/worldwake-sim/src/tick_step.rs::resolve_affordance` (currently line 468–522). Today, when the identity-match enumeration produces no match and `mode == BestEffort`, the function synthesizes a fresh `Affordance` from `targets.to_vec()` at line 504–514. Insert the strictness gate before synthesis for requests that are not already fully bound and payload-valid:

```rust
let (mut affordance, binding) = match reproduced {
    Some(affordance) => (affordance, RequestBindingKind::ReproducedAffordance),
    None if mode == crate::ActionRequestMode::BestEffort => {
        match check_binding_strictness(def, mode) {
            StrictnessGate::ExactIdentityRequired => {
                return Err(RequestResolutionRejectionReason::ExactIdentityRequired);
            }
            StrictnessGate::SubstitutionAllowed(_class) => (
                crate::Affordance {
                    def_id,
                    actor,
                    bound_targets: targets.to_vec(),
                    payload_override: payload_override.clone(),
                    explanation: None,
                    contention_status: worldwake_core::ContentionStatus::Unmanaged,
                },
                RequestBindingKind::BestEffortFallback,
            ),
        }
    }
    None => return Err(RequestResolutionRejectionReason::NoMatchingAffordance),
};
```

Extend `RequestResolutionRejectionReason` with an `ExactIdentityRequired` variant. The caller (the `ProduceAction` arm of `input_action`) records the rejection in the request-resolution trace. In the landed contract, underbound or malformed BestEffort exact-identity requests stop there, while fully bound stale requests continue into the existing start-time validation path so the scheduler can reject the concrete target authoritatively. Pre-S109, both surfaces map into `BlockingFact::AssumptionFailed`; S109 refines the typed discrepancy.

### D5: Revalidation reassessment in `plan_revalidation.rs`

`crates/worldwake-ai/src/plan_revalidation.rs::revalidate_next_step` (line 14–49) has three paths: the primary `requested_affordance_matches` over enumerated affordances, `revalidate_best_effort_payload_override_step` (line 51–82), and `revalidate_exact_target_step` (line 84–118). Reassessment after T-002 showed the latter two are not alternate-target substitution paths: both operate on the step's already planned `targets`.

- `revalidate_best_effort_payload_override_step` validates a planner-synthesized payload override against the same planned targets. This is required for lawful exact-identity actor-place actions such as `post_notice`, where the payload remains anchored to the same posting place even when primary affordance enumeration does not surface a payload-bearing affordance.
- `revalidate_exact_target_step` synthesizes an affordance from the step's own planned targets and re-runs `requested_affordance_matches`; it does not substitute a different target.

As a result, no blanket `check_binding_strictness(def, ActionRequestMode::BestEffort)` gate was added to these helpers. `BindingStrictness` still governs dispatch-time substitution boundaries, but these planner-side helpers remain lawful same-target revalidation surfaces.

Partial overlap with current code:
- `accuse` uses `TargetSpec::SpecificEntity(EntityId { slot: 0, generation: 0 })` as a placeholder plus a custom `enumerate_accuse_targets` (`justice_actions.rs:27, 69-72`). `revalidate_exact_target_step` revalidates that same target rather than substituting a different one.
- `post_notice` uses `TargetSpec::ActorPlace` and an explicit payload override anchored to the same posting place. Gating `revalidate_best_effort_payload_override_step` by `ExactIdentityRequired` would incorrectly suppress lawful repeated `post_notice` commits.
- `loot` uses `TargetSpec::EntityAtActorPlace { kind: EntityKind::Agent }` (`combat.rs:505`). Neither planner-side fallback retargets it; the dispatch-side/request-shape handling from D4 remains the relevant S108 change for malformed or stale dispatch shapes there.

### D6: Decision trace field on `PlannedStepSummary`

Extend `crates/worldwake-ai/src/decision_trace.rs::PlannedStepSummary` (line 971) with an optional `binding_strictness: Option<BindingStrictness>` field showing the classifier that governed dispatch. Populated from the `ActionDef` at the moment the step is resolved into the trace. `PlannedStep` itself (`planner_ops.rs:814`) is NOT extended; the authoritative source is always `ActionDef::binding_strictness`, and the trace carries a snapshot for inspection.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Not applicable. Strictness is a static property of `ActionDef`, registered at startup. No agent belief or information flow is introduced.
2. **Positive-feedback analysis**: No amplifying loops. Strictness is a filter, not a feedback system.
3. **Concrete dampeners**: Not applicable (no loops).
4. **Stored state vs. derived read-model**: `BindingStrictness` is authoritative static metadata on each registered `ActionDef`. The `StrictnessGate` return is computed, not stored. The trace snapshot on `PlannedStepSummary` is a derived copy.

## SystemFn Integration

No new SystemFn. The strictness gate is a pure helper invoked inline from `resolve_affordance` (sim) and `revalidate_next_step` (ai).

## Component Registration

None. `BindingStrictness` attaches to `ActionDef` (registry data, not an ECS component).

## Cross-System Interactions

- **AI planner ↔ sim dispatch**: The planner emits `ActionRequest` with `ActionRequestMode::BestEffort`; the sim's `resolve_affordance` consults `check_binding_strictness` before synthesizing a fallback affordance. Under the landed contract, the gate rejects underbound or malformed exact-identity requests at request resolution and otherwise preserves fully bound stale requests for authoritative start-time validation. Planner-side same-target fallback revalidation remains unchanged because it does not substitute different targets. State-mediated through the registered `ActionDef`, not a direct call (FND-26).
- **S109 discrepancy taxonomy**: The new `RequestResolutionRejectionReason::ExactIdentityRequired` feeds into S109's `Discrepancy::NoLegalBinding` classification. S108 lands the enum, the gate, and the request-resolution/start-time validation surfaces; S109 refines the resulting failure typing.

## Profile-Driven Parameters

Not applicable. `BindingStrictness` is per-`ActionDef`, not per-agent. Scenario authors do not override strictness per agent — an action's identity contract is a property of the action, not the actor.

## Open Migration Work (follow-up, out of scope)

Once D3–D6 land, `revalidate_exact_target_step`'s all-`TargetSpec::SpecificEntity` precondition becomes a narrower subset of `BindingStrictness::ExactIdentity`. A follow-up spec should either fold `revalidate_exact_target_step` into the strictness gate or retain it only as a documented fast path with an explicit comment tying it to `ExactIdentity` semantics (FND-28: no backward compatibility in live authority paths).

## Validation and Falsification

### Unit tests (in `affordance_query.rs` and `tick_step.rs`)

1. `check_binding_strictness(def[ExactIdentity], BestEffort)` → `ExactIdentityRequired`.
2. `check_binding_strictness(def[FungibleEquivalentCommodity], BestEffort)` → `SubstitutionAllowed(FungibleEquivalentCommodity)`.
3. `check_binding_strictness(def[_], Strict)` → `SubstitutionAllowed(_)` for every class (Strict mode bypasses the gate).
4. `resolve_affordance` with an underbound or malformed BestEffort request against an `ExactIdentity` action → `RequestResolutionRejectionReason::ExactIdentityRequired`, no synthesized affordance.
5. `revalidate_best_effort_payload_override_step` remains lawful for same-target exact-identity payload actions such as `post_notice`.
6. `revalidate_exact_target_step` remains lawful for same-target exact-identity specific-entity steps such as `accuse`.
7. `resolve_affordance` with a BestEffort request against a `FungibleEquivalentCommodity` action whose specific item is gone but another of the same kind is present at the place → substitutes successfully.
8. `resolve_affordance` with a BestEffort request against an `EquivalentRouteStep` action and an alternate route edge to the same destination → substitutes.

### Integration tests

7. Existing `loot` golden (or the strongest live AI decision golden owner): capture the AI-selected corpse binding, then carry that stale binding through a BestEffort external `loot` request after the corpse moves. Confirm request resolution preserves the original corpse id and the action trace refuses start rather than silently rebinding to a different corpse.
8. Existing consume-pipeline golden (or the strongest live needs/decision golden owner): capture the AI-selected fungible `pick_up` lot for self-consumption, then carry that stale binding through a BestEffort external `pick_up` request after the lot moves. Confirm the request follows the non-exact path and the consume pipeline still reaches `eat` / `drink`.
9. Real decision-trace assertions: at least one exact-identity golden and one fungible consume-pipeline golden assert the AI-selected step's `binding_strictness` against the authoritative registry value.
10. No travel-route golden is required on the current branch: `travel` binds to destination place, so alternate-edge reuse is not a distinct AI-visible golden boundary without additional production-surface route identity.

### Regression guard

11. Each `ActionDef` registration site assigns an explicit `binding_strictness` value. Because the field is non-`Option`, `ActionDef` has no `Default` impl, and `#[serde(default = …)]` is applied only for deserialization, a new action whose literal-construction site omits `binding_strictness` fails compilation.
12. `ActionDef` bincode roundtrip test (extend the existing test at `action_def.rs:143`) round-trips the new field.

### Authoritative-to-AI Impact Rule (CLAUDE.md checklist)

Because this spec modifies BestEffort dispatch, tickets must verify the full AI decision cycle:

1. `get_affordances` — unaffected; affordance enumeration does not consult strictness.
2. `generate_candidates` — unaffected; candidate emission does not consult strictness.
3. `search_plan` — unaffected; the planner does not pre-filter on strictness (strictness only gates substitution at dispatch/revalidation).
4. `BestEffort` action start — core site. Underbound or malformed exact-identity requests are rejected at request resolution, while fully bound stale exact-identity requests continue into authoritative start-time validation; confirm the resulting failure still converts cleanly into `BlockingFact::AssumptionFailed` (pre-S109).
5. `handle_plan_failure` — confirm `BlockingFact::AssumptionFailed` routes through `failure_handling.rs` for re-plan.
6. Payload revalidation (`with_payload_override_validator`) — same-target exact-identity payload revalidation remains lawful; verify with the affected focused and golden proofs (`post_notice`, `post_bounty`, `accuse`).
7. Golden tests — run the relevant exact golden proving stale-corpse refusal, the consume-pipeline fungible fallback golden, and the lawful social loop (`post_notice`) that T-003 preserved.

## Outcome

Completed on 2026-04-19.

- Added authoritative `BindingStrictness` metadata to `worldwake_sim::ActionDef`, plus the pure `check_binding_strictness` helper and `StrictnessGate`.
- Classified the live action registry across `worldwake-systems`, preserved `ActionDef` serde/bincode compatibility, and updated exhaustive fixture construction across `worldwake-sim`, `worldwake-systems`, and `worldwake-ai`.
- Added `RequestResolutionRejectionReason::ExactIdentityRequired`, the strictness-aware request-resolution / dispatch behavior for underbound or malformed exact-identity BestEffort requests, and the AI-side `BlockingFact::AssumptionFailed` mapping.
- Corrected the drafted planner-side D5 contract during implementation: `plan_revalidation.rs` keeps its same-target fallback helpers because they do not perform alternate-target substitution on the live branch.
- Added `binding_strictness` to `PlannedStepSummary` and populated it from `ActionDef` at the real trace-construction boundary.
- Added hybrid golden end-to-end proof in `golden_ai_decisions.rs` for exact-identity stale-corpse refusal and fungible consume-pipeline fallback, then regenerated the golden inventory/docs.

### Deviations

1. D4 narrowed during implementation: fully bound stale `ExactIdentity` BestEffort requests are not rejected universally at request resolution. Under the landed contract, only underbound or malformed exact-identity requests are rejected there; fully bound stale requests still proceed to authoritative start-time validation.
2. D5 was corrected rather than implemented literally. `revalidate_best_effort_payload_override_step` and `revalidate_exact_target_step` remain lawful same-target revalidation helpers, so no blanket planner-side `check_binding_strictness` gate was added.
3. The end-to-end proof in Validation items 7–10 landed as a hybrid golden seam: the AI-selected binding is captured from decision trace, then carried through the narrowest lawful external BestEffort request because the live autonomous branch does not hold a stable stale-request window for those cases.
4. The drafted travel-route golden was not required on the live branch because `travel` binds to destination place rather than route-edge identity at the AI-visible proof surface.

### Verification Result

Passed across the ticket chain that delivered this spec:

1. `cargo test -p worldwake-sim`
2. Targeted `worldwake-ai` regressions for T-002, T-003, and T-004, including the same-target revalidation proofs and `decision_trace` coverage
3. `cargo test -p worldwake-ai golden_loot_refuses_substitute_corpse_after_remote_travel_commitment -- --exact`
4. `cargo test -p worldwake-ai golden_consume_pipeline_rebinds_pick_up_after_remote_lot_change -- --exact`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo test -p worldwake-ai`
7. `cargo build --workspace`
8. `cargo test --workspace`
9. `cargo clippy --workspace --all-targets -- -D warnings`
