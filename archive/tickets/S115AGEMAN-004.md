# S115AGEMAN-004: D4A classify_rejection + S112 carve-out removal

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds `classify_rejection` plus `RejectionLifecycle`, removes the two S112 committed-goal carve-outs in `planning.rs`, and routes committed probe rejections through classifier-driven lifecycle handling in the live planner seam.
**Deps**: [archive/tickets/S115AGEMAN-003](../archive/tickets/S115AGEMAN-003.md)

## Problem

The S112 feasibility probe returns `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy }` with nine `Discrepancy` variants, but the current planner treats all rejections uniformly. Two structurally-different cases are indistinguishable today: goals that are *already satisfied* (e.g., `MoveCargo` when the agent is at destination) get conflated with goals that are *structurally infeasible*. `build_candidate_plans` papers over this at two sites (`planning.rs:400-427` and `planning.rs:875-892`) with `!is_committed` / `if … == committed_goal { continue; }` carve-outs that keep the committed opportunity alive without explaining why. S115 replaces that ambiguity with an authoritative classifier (`classify_rejection`) that maps each rejection to `Satisfied` / `InfeasibleUntil { trigger }` / `Dead`, plus a pre-check that detects satisfied post-conditions regardless of the rejection reason. After this ticket, the carve-outs are gone and the classifier is the single decoder of rejection → lifecycle.

## Outcome

Implemented `classify_rejection` in `crates/worldwake-ai/src/agenda_manager.rs` as the authoritative rejection-to-lifecycle decoder for committed feasibility rejections. The live integration point today is `crates/worldwake-ai/src/agent_tick/planning.rs`: committed rejected plans are classified before selection, satisfied goals are parked in the committed portfolio entry with `AgendaPhase::Suspended`, retryable goals are parked with `AgendaPhase::Pending` plus a `RevivalTrigger`, and dead goals are removed after recording `DiscrepancyMemory`.

This removes both S112 carve-outs from `planning.rs`. `cargo_satisfaction_at_destination_while_carrying` now passes because `MoveCargo` satisfaction is detected through the classifier path rather than by keeping rejected committed opportunities alive through special-case filtering.

## Deviations

1. The classifier signature is `classify_rejection(actor, probe_verdict, offer, beliefs, tick, revive_cooldown_ticks)`, not the narrower ticket sketch, because route and goal-satisfaction checks in `GoalBeliefView` are actor-scoped and dead-goal discrepancy entries need the current tick.
2. The current live seam does not yet invoke `tick_agenda`; ticket 005 still owns the dormant agenda-system wiring. As a result, this ticket parks satisfied and pending lifecycle states in `agenda_state.committed` by phase, rather than moving entries through the fuller pending/suspended agenda maps described in the earlier draft.
3. `GoalKindPlannerExt::is_satisfied` was not the practical seam for the satisfied pre-check. The implemented `MoveCargo` short-circuit uses belief-view-accessible post-conditions directly in `agenda_manager.rs`.

## Verification Result

Passed:

1. `cargo test -p worldwake-ai agenda_manager::tests -- --list`
2. `cargo test -p worldwake-ai agent_tick::tests::cargo_satisfaction_at_destination_while_carrying -- --exact`
3. `cargo test -p worldwake-ai --test golden_portfolio_planning portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal -- --exact`
4. `cargo test -p worldwake-ai --test golden_planner_pathology`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo fmt --all`

## Assumption Reassessment (2026-04-22)

1. `FeasibilityVerdict` at `crates/worldwake-ai/src/agent_tick/portfolio.rs:29-32` is `pub(crate)` — `classify_rejection` must live in `worldwake-ai` (spec confirms). Consumption is within the crate, so `pub(crate)` remains appropriate.
2. `Discrepancy` at `crates/worldwake-core/src/discrepancy.rs:6-25` has 9 variants: `BeliefStale`, `BeliefContradicted`, `ImproperPlanningState`, `MissingObservation`, `NoLegalBinding`, `NoWillingCounterparty`, `RouteUnknown`, `SearchBudgetExhausted`, `PartialExecutionDrift`. The classifier must be exhaustive (Rust `match` forces this, which is the desired compile-time guarantee).
3. The shared boundary under audit is the return type of `classify_rejection` — `RejectionLifecycle { Satisfied, InfeasibleUntil { trigger: RevivalTrigger }, Dead }`. This type is a new enum local to `agenda_manager`, not a core type, because downstream consumers are ai-internal.
4. Existing S112 carve-outs at the start of the ticket were:
   - `crates/worldwake-ai/src/agent_tick/planning.rs:409-427` — `rejected_opportunities` filter uses `!is_committed` predicate to keep the committed opportunity in `search_order` even when the probe rejects it.
   - `crates/worldwake-ai/src/agent_tick/planning.rs:875-892` — `rejected_by_goal` tracking uses `if slot.ranked.grounded.key == committed_goal { continue; }` to skip the committed goal from rejection bookkeeping.
   Both carve-outs were deleted. In the live implementation, `classify_rejection` now runs from `agent_tick/planning.rs` before plan selection and applies the correct lifecycle transition for the committed goal (park as `Suspended` via Satisfied pre-check, park as `Pending` via `InfeasibleUntil`, or drop via `Dead`).
5. `cargo_satisfaction_at_destination_while_carrying` (`crates/worldwake-ai/src/agent_tick/tests.rs:4710`) and `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` (`crates/worldwake-ai/tests/golden_portfolio_planning.rs:210`) are the two load-bearing tests protecting the S112 behavior. Post-ticket both must pass via the new classifier path, NOT via the carve-outs.
6. Ticket 003 landed the pure `agenda_manager` module plus the blocker-key substrate, but it did not yet wire D4A rejection routing. This ticket owns both the pure `classify_rejection` function and the caller-side `Dead`-branch `DiscrepancyMemory::record` integration. Because full `tick_agenda` execution is still ticket 005 territory, the live routing point is `agent_tick/planning.rs`.
7. Satisfied-goal detection is intentionally narrow in this implementation. `MoveCargo` now short-circuits to `Satisfied` when the actor is already at the destination while carrying the commodity, or when destination restock demand is already closed. Other goal kinds still fall through to discrepancy-table classification unless they gain equally belief-local post-condition checks in a later ticket.

## Architecture Check

1. `classify_rejection(actor, probe_verdict, offer, beliefs, tick, revive_cooldown_ticks)` remains deterministic and side-effect-free: it computes a `RejectionLifecycle` from the probe verdict plus belief-local context, but does not mutate agenda state or discrepancy memory itself. Placing it in `agenda_manager.rs` keeps lifecycle decoding co-located with the broader agenda lifecycle machinery.
2. Removing both carve-outs eliminates fossilized conditional branches in `build_candidate_plans`. Post-ticket, the committed opportunity follows the same rejection routing as any other candidate — the agenda manager absorbed the special case. This is the architectural cleanup FND-28 mandates.
3. Exhaustive match on `Discrepancy` gives compile-time protection against variant additions (the compiler will force future variants to pick a lifecycle row). No default `_ =>` catch-all, per spec D4A table explicit mapping.

## Verification Layers

1. Classifier correctness per variant — unit tests, one per `Discrepancy` variant, asserting the mapped `RejectionLifecycle` matches the spec D4A table.
2. Satisfied pre-check — unit test: construct `offer` with `MoveCargo { commodity, destination }`, configure `MockGoalBeliefView` to report agent-at-destination-with-commodity; assert `RejectionLifecycle::Satisfied` regardless of the synthetic `Discrepancy` variant fed to the probe verdict.
3. Integration: rejected committed opportunity no longer gets a carve-out — `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` passes via classifier → Suspended (Satisfied), not via `!is_committed` retention. Verified by grepping the test for references to the old `rejected_opportunities` carve-out and confirming the new path is taken (decision trace shows lifecycle transition, not ad-hoc search_order retention).
4. Integration: portfolio rejection test — `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` passes via classifier mapping the infeasible commitment to Dead (dropped) rather than pinning it.
5. Carve-out removal — grep post-ticket for `!is_committed` and `== committed_goal` in `planning.rs`: zero matches in the removed regions.

## What to Change

### 1. Define `RejectionLifecycle` in `agenda_manager.rs`

```rust
pub enum RejectionLifecycle {
    Satisfied,
    InfeasibleUntil { trigger: RevivalTrigger },
    Dead,
}
```

### 2. Implement `classify_rejection`

```rust
pub(crate) fn classify_rejection(
    probe_verdict: &FeasibilityVerdict,
    offer: &GoalOffer,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
    revive_cooldown_ticks: u32,
) -> RejectionLifecycle {
    debug_assert!(matches!(probe_verdict, FeasibilityVerdict::RejectedBeforeSearch { .. }));

    // Satisfied pre-check: if the goal's post-conditions are already true in beliefs,
    // short-circuit to Satisfied regardless of the rejection reason.
    if goal_post_conditions_already_satisfied(&offer.key, beliefs) {
        return RejectionLifecycle::Satisfied;
    }

    let reason = match probe_verdict {
        FeasibilityVerdict::RejectedBeforeSearch { reason } => *reason,
        FeasibilityVerdict::Plausible => unreachable!("caller precondition"),
    };

    match reason {
        Discrepancy::MissingObservation => {
            // Commodity-bound goals → CommodityAvailable; else TargetPresent.
            if let Some(kind) = offer_commodity(offer) {
                let place = offer.anchor.place().expect("commodity goal needs place");
                RejectionLifecycle::InfeasibleUntil {
                    trigger: RevivalTrigger::CommodityAvailable { place, kind, min: Quantity(1) },
                }
            } else {
                let target = offer.anchor.entity().expect("target goal needs entity");
                let place = offer.anchor.place().unwrap_or(target);
                RejectionLifecycle::InfeasibleUntil {
                    trigger: RevivalTrigger::TargetPresent { target, place },
                }
            }
        }
        Discrepancy::RouteUnknown => RejectionLifecycle::InfeasibleUntil {
            trigger: RevivalTrigger::RouteLearned {
                from: beliefs.effective_place(offer_agent(offer)).expect("agent place known"),
                to: offer.anchor.place().expect("target place needed"),
            },
        },
        Discrepancy::PartialExecutionDrift => RejectionLifecycle::InfeasibleUntil {
            trigger: drift_trigger_for_offer(offer, tick, revive_cooldown_ticks),
        },
        Discrepancy::BeliefStale | Discrepancy::SearchBudgetExhausted => {
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TickElapsed {
                    at_tick: Tick(tick.0 + u64::from(revive_cooldown_ticks)),
                },
            }
        }
        Discrepancy::NoWillingCounterparty => {
            let counterparty = offer.anchor.entity().expect("counterparty anchor");
            let place = offer.anchor.place().unwrap_or(counterparty);
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::CounterpartyAvailable { counterparty, place },
            }
        }
        Discrepancy::BeliefContradicted | Discrepancy::NoLegalBinding | Discrepancy::ImproperPlanningState => {
            RejectionLifecycle::Dead
        }
    }
}
```

Helpers `goal_post_conditions_already_satisfied`, `offer_commodity`, `offer_agent`, `drift_trigger_for_offer` are small private functions — each inspects specific `GoalKind` variants.

### 3. Remove S112 carve-outs

At `crates/worldwake-ai/src/agent_tick/planning.rs:409-427`:

```rust
let rejected_opportunities: BTreeSet<OpportunityKey> = portfolio
    .slots
    .values()
    .filter_map(|slot| {
        matches!(
            slot.feasibility,
            FeasibilityVerdict::RejectedBeforeSearch { .. }
        )
        .then_some(OpportunityKey {
            goal_key: slot.ranked.grounded.key,  // <- will be `slot.ranked.offer.key` after ticket 002
            anchor: slot.ranked.grounded.anchor, // <- ditto
        })
    })
    .collect();
```

Remove the `is_committed` local and the `!is_committed &&` guard. Delete the 10-line justification comment (lines 395-408) — the carve-out no longer exists, so the explanation is dead text.

At `crates/worldwake-ai/src/agent_tick/planning.rs:875-892`:

Remove the `if slot.ranked.grounded.key == committed_goal { continue; }` guard at the top of the `for slot in portfolio.slots.values()` loop. Pre-classifier, committed goals now flow through the same rejection-bookkeeping path as any other goal (if the classifier demoted them to Pending/Suspended, they're not in the slots; if they're still in the slots and rejected, they're a real rejection).

### 4. Wire classifier invocation in tick_agenda

Update ticket 003's `demote_to_pending_or_suspended` to route probe-rejected losers through `classify_rejection`:

- `RejectionLifecycle::Satisfied` → move to `suspended` with `KillCondition::External`, `revival_trigger: None`.
- `RejectionLifecycle::InfeasibleUntil { trigger }` → move to `pending` with `revival_trigger: Some(trigger)`, `kill_condition: KillCondition::External` (unless the caller has richer context).
- `RejectionLifecycle::Dead` → drop entirely; caller emits `GoalAbandoned` and writes `DiscrepancyMemory::record` with synthesized `BlockerKey`.

Note: ticket 003 already defined the demote helper; this ticket extends its signature to accept the probe verdict per candidate and call `classify_rejection` internally.

## Files to Touch

- `crates/worldwake-ai/src/agenda_manager.rs` (modify — add `classify_rejection`, `RejectionLifecycle`, helpers)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — remove both carve-outs)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — pass mutable `DiscrepancyMemory` into planner-side lifecycle routing)

## Out of Scope

- `agenda_tick_system` SystemFn wiring (ticket 005)
- S74 margin-based switch logic (ticket 005)
- New unit/integration tests beyond what's listed in Acceptance Criteria (ticket 006 bundles broader lifecycle tests)
- Golden agenda scenario (ticket 007)
- Changes to `FeasibilityVerdict` or `Discrepancy` enum shape — this ticket consumes them as-is
- Changes to the probe itself (`feasibility_probe::probe`) — classifier reads its output, does not modify probe behavior

## Acceptance Criteria

### Tests That Must Pass

1. `agenda_manager` unit coverage exists for all 9 `Discrepancy` variants plus the satisfied `MoveCargo` pre-check.
2. `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` passes without the `!is_committed` carve-out (verified by grep after deletion).
3. `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` passes.
4. `cargo test -p worldwake-ai --test golden_planner_pathology` passes.
5. `cargo test -p worldwake-ai` passes.
6. `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. `classify_rejection` is exhaustive over all nine `Discrepancy` variants at compile time (`match` without `_`).
2. `classify_rejection` is pure: no mutation of `AgendaState`, `DiscrepancyMemory`, beliefs, or world.
3. Zero references to `!is_committed` or `== committed_goal` in `planning.rs` after the ticket (grep-verified).
4. The committed opportunity path through `build_candidate_plans` is no longer special-cased; it follows the same rejected-opportunity filter as every other slot.
5. Satisfied pre-check fires regardless of the `Discrepancy` variant supplied — goal post-conditions being true in beliefs always short-circuits to `RejectionLifecycle::Satisfied`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` now covers all 9 discrepancy-to-lifecycle mappings plus the satisfied `MoveCargo` pre-check.
2. `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` now asserts the committed entry is preserved in `AgendaPhase::Suspended`, proving the classifier path replaced the old carve-out behavior.
3. Existing golden coverage in `golden_portfolio_planning` and `golden_planner_pathology` exercises the integrated planner behavior after carve-out removal.

### Commands

1. `cargo test -p worldwake-ai agenda_manager::tests -- --list`
2. `cargo test -p worldwake-ai agent_tick::tests::cargo_satisfaction_at_destination_while_carrying -- --exact`
3. `cargo test -p worldwake-ai --test golden_portfolio_planning portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal -- --exact`
4. `cargo test -p worldwake-ai --test golden_planner_pathology`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo fmt --all`
