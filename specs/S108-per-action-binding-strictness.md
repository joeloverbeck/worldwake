# S108: Per-Action Binding Strictness

## Summary

Tighten `ActionRequestMode::BestEffort` substitution by attaching an explicit `BindingStrictness` classifier to every `ActionDef`. Today BestEffort is permissive-by-default: a planner-synthesized request can fall back to any affordance that matches the action kind at the same place, even when the step semantically requires a specific identity (accuse *this* suspect, transfer *this* item, loot *this* corpse, escort *this* subject). Classify each action on the spectrum `ExactIdentity → FungibleEquivalentCommodity → EquivalentFacilityClassAtSamePlace → EquivalentRouteStep → AnyLegalTarget`, enforce the classifier in the unified legality path (`requested_affordance_matches`, `revalidate_next_step`, tick-step dispatch), and refuse BestEffort substitution that would cross the strictness boundary.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-sim` — `BindingStrictness` enum, `ActionDef::binding_strictness`, strictness enforcement in `requested_affordance_matches` and `step_tick` BestEffort fallback
- `worldwake-ai` — `plan_revalidation.rs` reads strictness when classifying revalidation failures; decision trace records the strictness class at dispatch
- `worldwake-systems` — classify each registered action at its `ActionDef` registration site (accuse, transfer, loot, escort, post-bounty, give, treat-wounds, consume, buy, use-workstation, travel, etc.)

## Dependencies

- None. Builds on the existing unified legality path (`with_payload_override_validator`, `requested_affordance_matches` in `affordance_query.rs`).

## Design Goals

- Every socially or materially identity-bound action refuses BestEffort substitution that silently redirects to a different counterparty, item, or target.
- Strictness is declared at action-registration time, not inferred at dispatch. No per-handler ad-hoc checks.
- Revalidation, affordance enumeration, and dispatch all consult the same strictness class. A step classified `ExactIdentity` behaves the same way in all three surfaces.
- Strictness drives the failure classification surface: an `ExactIdentity` request that cannot find the exact target records a different discrepancy (`TargetGone` / `NoLegalBinding`) than an `AnyLegalTarget` fallback that found nothing available (`SellerOutOfStock`).

## Non-Goals

- Revisiting the unified legality path (already in place via `requested_affordance_matches` + `with_payload_override_validator`). This spec adds a classifier on top, not a new validation surface.
- Full discrepancy taxonomy rework — covered separately by S109.
- Per-action whitelisting of substitute targets. Strictness is coarse-grained (5 classes) and covers the common cases; finer-grained rules wait for concrete scenarios.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-4 (Persistent Identity, Object Permanence, Explicit Transfer) | `ExactIdentity` strictness forbids silent retargeting of identity-bearing actions (accuse, transfer, loot). The same exact entity that appeared in the plan must still be the target at dispatch, or the step fails lawfully. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | Binding strictness is declared alongside preconditions and cost, formalizing the binding contract of an action. |
| FND-20 (Resource-Bounded Practical Reasoning) | Planner-synthesized BestEffort requests still receive the strictness filter, so agents do not accidentally accuse a different suspect or give bread to a different recipient because the planner emitted a fungible request. |
| FND-21 (Intentions Are Revisable Commitments) | An `ExactIdentity` step whose target is gone must fail through the normal revalidation + replan path, not silently substitute a different entity. |
| FND-24 (Ownership, Custody, Access, Obligation, Jurisdiction Are Distinct) | Identity-bound actions (loot, transfer, accuse within jurisdiction) enforce the target-identity contract that ownership/custody/jurisdiction rules depend on. |

## Deliverables

### D1: `BindingStrictness` enum

New type in `crates/worldwake-sim/src/action_def.rs` (or wherever `ActionDef` currently lives):

```rust
/// How permissive BestEffort target substitution is for a given action.
/// Declared once on each ActionDef at registration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BindingStrictness {
    /// The exact entity referenced in the request must be present and
    /// eligible. Substitution is forbidden. Example actions:
    /// accuse, transfer, give, loot, escort, treat-wounds, punish,
    /// tell-witness, consume-owned-specific-item.
    ExactIdentity,
    /// The target may be substituted with any entity of the same
    /// fungible commodity kind at the same place. Example actions:
    /// eat-bread (any bread stack), drink-water (any water container),
    /// consume-fungible-commodity.
    FungibleEquivalentCommodity,
    /// The target may be substituted with any entity that fills the
    /// same facility class (workstation, well, hearth) at the same
    /// place. Example actions: use-workstation, use-well, rest-at-hearth.
    EquivalentFacilityClassAtSamePlace,
    /// The target may be substituted with any route edge that reaches
    /// the same destination place. Example actions: travel.
    EquivalentRouteStep,
    /// The target may be substituted with any legal target that
    /// satisfies the action's preconditions. Example actions: explore,
    /// scout, wait. Use sparingly.
    AnyLegalTarget,
}
```

### D2: `ActionDef` extension

Add a `binding_strictness: BindingStrictness` field to the `ActionDef` registration record. Every existing action receives an explicit classification at its registration site — no default. The compiler enforces completeness.

Concrete classification table (to be confirmed at implementation time; representative assignments):

| Action | Strictness |
|--------|------------|
| Accuse | `ExactIdentity` |
| TransferItem / GiveItem | `ExactIdentity` |
| LootCorpse | `ExactIdentity` |
| EscortSubject | `ExactIdentity` |
| TreatWounds | `ExactIdentity` |
| TellWitness / ShareBelief (targeted) | `ExactIdentity` |
| PostBounty / PostNotice | `ExactIdentity` (the issuer binds; the topic may be fungible separately) |
| ConsumeOwnedCommodity | `FungibleEquivalentCommodity` |
| AcquireCommodity (purchase) | `FungibleEquivalentCommodity` |
| UseWorkstation (produce, craft) | `EquivalentFacilityClassAtSamePlace` |
| UseWell / RestAtHearth | `EquivalentFacilityClassAtSamePlace` |
| Travel | `EquivalentRouteStep` |
| Explore / Scout / Wait | `AnyLegalTarget` |

### D3: Strictness enforcement in `requested_affordance_matches`

Modify `crates/worldwake-sim/src/affordance_query.rs::requested_affordance_matches` to consult the action's `binding_strictness`:

```rust
pub fn requested_affordance_matches(
    world: &World,
    request: &ActionRequest,
    action_def: &ActionDef,
    agent: EntityId,
) -> MatchOutcome {
    // 1. Attempt exact-identity match against the request's bound targets.
    if let Some(affordance) = match_exact(world, request, action_def, agent) {
        return MatchOutcome::Exact(affordance);
    }
    // 2. If mode is BestEffort, consult strictness before falling back.
    match (request.mode, action_def.binding_strictness) {
        (ActionRequestMode::BestEffort, BindingStrictness::ExactIdentity) => {
            // ExactIdentity refuses BestEffort fallback. Return a typed
            // failure that the caller can classify as NoLegalBinding.
            MatchOutcome::ExactIdentityRequired
        }
        (ActionRequestMode::BestEffort, strictness) => {
            match_best_effort_within(world, request, action_def, agent, strictness)
        }
        (ActionRequestMode::Strict, _) => MatchOutcome::NoMatch,
    }
}
```

`match_best_effort_within` restricts the fallback pool to candidates that satisfy the strictness bound (same commodity / same facility class at same place / same route destination).

### D4: `tick_step` dispatch enforcement

`crates/worldwake-sim/src/tick_step.rs` already calls `requested_affordance_matches` around the BestEffort dispatch path. With D3 in place, the dispatch fallback (currently line ~504) receives `MatchOutcome::ExactIdentityRequired` for identity-bound actions whose target disappeared, converts it to a typed start-failure reason (see S109 for the full discrepancy taxonomy — pre-S109 it maps to the existing `BlockingFact::AssumptionFailed`), and does not synthesize a substitute affordance.

### D5: Revalidation classifier

`crates/worldwake-ai/src/plan_revalidation.rs::revalidate_next_step` already calls the unified matcher. With D3, revalidation automatically observes the same strictness boundary. Extend the revalidation outcome to record the strictness class that decided the outcome, so the decision trace and S109 discrepancy classifier can see "ExactIdentity required — target gone" vs "Fungible substitution failed — no stock."

### D6: Decision trace field

In `crates/worldwake-ai/src/decision_trace.rs`, extend the `PlannedStep`-level trace record with an optional `binding_strictness: Option<BindingStrictness>` field showing the classifier that governed dispatch. Populated from the `ActionDef` at the moment the step is resolved.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Not applicable. Strictness is a static property of `ActionDef`, registered at startup. No agent belief or information flow is introduced.
2. **Positive-feedback analysis**: No amplifying loops. Strictness is a filter, not a feedback system.
3. **Concrete dampeners**: Not applicable (no loops).
4. **Stored state vs. derived read-model**: `BindingStrictness` is authoritative static metadata on each registered `ActionDef`. No runtime-derived state is introduced. The dispatch outcome is computed, not stored.

## SystemFn Integration

No new SystemFn. The strictness filter runs inside the existing `requested_affordance_matches` called from `step_tick` and `revalidate_next_step`.

## Component Registration

None. `BindingStrictness` attaches to `ActionDef` (registry data, not an ECS component).

## Cross-System Interactions

- **AI planner ↔ sim dispatch**: The planner emits `ActionRequest` with `ActionRequestMode::BestEffort`; the sim's `requested_affordance_matches` filters substitution through the strictness class. State-mediated through the registered `ActionDef`, not a direct call.
- **S109 discrepancy taxonomy**: The `MatchOutcome::ExactIdentityRequired` return feeds into S109's `NoLegalBinding` / `TargetGone` classification. S108 lands the enum and enforcement; S109 lands the discrepancy typing.

## Profile-Driven Parameters

Not applicable. `BindingStrictness` is per-`ActionDef`, not per-agent. Scenario authors do not override strictness per agent — an action's identity contract is a property of the action, not the actor.

## Validation and Falsification

### Unit tests (in `affordance_query.rs`)

1. `ExactIdentity` + target gone + BestEffort mode → returns `ExactIdentityRequired`, never substitutes.
2. `FungibleEquivalentCommodity` + specific item gone + another of same kind present → substitutes successfully.
3. `EquivalentFacilityClassAtSamePlace` + workstation A occupied, workstation B of same class available → substitutes.
4. `EquivalentFacilityClassAtSamePlace` + facility at a different place → does NOT substitute (strictness requires same place).
5. `EquivalentRouteStep` + alternate edge to same destination → substitutes.
6. `AnyLegalTarget` + any eligible target → substitutes.

### Integration tests

7. Existing `accuse` golden: with a substituted suspect entity available in the place, confirm the planner does NOT substitute; the original target-gone step fails through revalidation.
8. Existing `eat` / `consume` golden: confirm fungible substitution still works.
9. Existing `travel` golden: confirm equivalent-route substitution works.

### Regression guard

10. Each `ActionDef` registration site has explicit `binding_strictness`. A new action without an assigned strictness fails compilation (no default).

## Outcome

To be filled in at completion.
