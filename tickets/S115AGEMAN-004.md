# S115AGEMAN-004: D4A classify_rejection + S112 carve-out removal

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds `classify_rejection` function mapping all nine `Discrepancy` variants to agenda lifecycle transitions; removes two `committed_goal` / `!is_committed` carve-outs in `build_candidate_plans`.
**Deps**: [archive/tickets/S115AGEMAN-003](../archive/tickets/S115AGEMAN-003.md)

## Problem

The S112 feasibility probe returns `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy }` with nine `Discrepancy` variants, but the current planner treats all rejections uniformly. Two structurally-different cases are indistinguishable today: goals that are *already satisfied* (e.g., `MoveCargo` when the agent is at destination) get conflated with goals that are *structurally infeasible*. `build_candidate_plans` papers over this at two sites (`planning.rs:400-427` and `planning.rs:875-892`) with `!is_committed` / `if … == committed_goal { continue; }` carve-outs that keep the committed opportunity alive without explaining why. S115 replaces that ambiguity with an authoritative classifier (`classify_rejection`) that maps each rejection to `Satisfied` / `InfeasibleUntil { trigger }` / `Dead`, plus a pre-check that detects satisfied post-conditions regardless of the rejection reason. After this ticket, the carve-outs are gone and the classifier is the single decoder of rejection → lifecycle.

## Assumption Reassessment (2026-04-22)

1. `FeasibilityVerdict` at `crates/worldwake-ai/src/agent_tick/portfolio.rs:29-32` is `pub(crate)` — `classify_rejection` must live in `worldwake-ai` (spec confirms). Consumption is within the crate, so `pub(crate)` remains appropriate.
2. `Discrepancy` at `crates/worldwake-core/src/discrepancy.rs:6-25` has 9 variants: `BeliefStale`, `BeliefContradicted`, `ImproperPlanningState`, `MissingObservation`, `NoLegalBinding`, `NoWillingCounterparty`, `RouteUnknown`, `SearchBudgetExhausted`, `PartialExecutionDrift`. The classifier must be exhaustive (Rust `match` forces this, which is the desired compile-time guarantee).
3. The shared boundary under audit is the return type of `classify_rejection` — `RejectionLifecycle { Satisfied, InfeasibleUntil { trigger: RevivalTrigger }, Dead }`. This type is a new enum local to `agenda_manager`, not a core type, because downstream consumers are ai-internal.
4. Existing S112 carve-outs:
   - `crates/worldwake-ai/src/agent_tick/planning.rs:409-427` — `rejected_opportunities` filter uses `!is_committed` predicate to keep the committed opportunity in `search_order` even when the probe rejects it.
   - `crates/worldwake-ai/src/agent_tick/planning.rs:875-892` — `rejected_by_goal` tracking uses `if slot.ranked.grounded.key == committed_goal { continue; }` to skip the committed goal from rejection bookkeeping.
   Both carve-outs can be deleted once `classify_rejection` runs before `build_candidate_plans` and produces the correct lifecycle transition for the committed goal (demote to Suspended via Satisfied pre-check, demote to Pending via InfeasibleUntil, or drop via Dead).
5. `cargo_satisfaction_at_destination_while_carrying` (`crates/worldwake-ai/src/agent_tick/tests.rs:4710`) and `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` (`crates/worldwake-ai/tests/golden_portfolio_planning.rs:210`) are the two load-bearing tests protecting the S112 behavior. Post-ticket both must pass via the new classifier path, NOT via the carve-outs.
6. Ticket 003 landed the pure `agenda_manager` module plus the `blocker_key_from`-style identity substrate, but it did not yet wire D4A rejection routing. This ticket therefore owns both the pure `classify_rejection` function and the caller-side `Dead`-branch `DiscrepancyMemory::record` integration that uses the synthesized blocker key while keeping the classifier itself side-effect-free.
7. Satisfied-goal detection: for each `GoalKind` variant that has natural post-conditions readable from beliefs (e.g., `MoveCargo { commodity, destination }` — agent at destination with commodity possessed), the pre-check reads the belief store and short-circuits to `Satisfied`. For `GoalKind` variants without trivially-readable post-conditions (e.g., `Sleep`, which is a duration-bearing action), the pre-check returns no and variant-table classification applies. Per-variant satisfaction helpers reuse existing `is_satisfied` logic on `GoalKindPlannerExt` at `crates/worldwake-ai/src/goal_model.rs` — this ticket wires the probe path to query the same invariant.

## Architecture Check

1. `classify_rejection(probe_verdict, offer, beliefs)` is a deterministic pure function over three inputs. It produces a `RejectionLifecycle` without mutation. Placing it in `agenda_manager.rs` alongside `tick_agenda` keeps lifecycle decoding and lifecycle application co-located (FND-26: systems interact through state; here the state is the `RejectionLifecycle` output).
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
- `crates/worldwake-ai/src/goal_model.rs` (modify — possibly extend `GoalKindPlannerExt::is_satisfied` callers or add new helper; depends on whether current `is_satisfied` is callable from the belief-view-only context classify_rejection has)

## Out of Scope

- `agenda_tick_system` SystemFn wiring (ticket 005)
- S74 margin-based switch logic (ticket 005)
- New unit/integration tests beyond what's listed in Acceptance Criteria (ticket 006 bundles broader lifecycle tests)
- Golden agenda scenario (ticket 007)
- Changes to `FeasibilityVerdict` or `Discrepancy` enum shape — this ticket consumes them as-is
- Changes to the probe itself (`feasibility_probe::probe`) — classifier reads its output, does not modify probe behavior

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- classify_rejection` — 9 per-variant tests + satisfied pre-check + post-condition spot check.
2. `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` passes without the `!is_committed` carve-out (verified by grep after deletion).
3. `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` passes.
4. `cargo test -p worldwake-ai -- golden_planner_pathology` passes.
5. Existing suite: `cargo test --workspace` passes.

### Invariants

1. `classify_rejection` is exhaustive over all nine `Discrepancy` variants at compile time (`match` without `_`).
2. `classify_rejection` is pure: no mutation of `AgendaState`, `DiscrepancyMemory`, beliefs, or world.
3. Zero references to `!is_committed` or `== committed_goal` in `planning.rs` after the ticket (grep-verified).
4. The committed opportunity path through `build_candidate_plans` is no longer special-cased; it follows the same rejected-opportunity filter as every other slot.
5. Satisfied pre-check fires regardless of the `Discrepancy` variant supplied — goal post-conditions being true in beliefs always short-circuits to `RejectionLifecycle::Satisfied`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` (modify `#[cfg(test)]`) — 9 variant tests + 1 satisfied-pre-check test. Each asserts `classify_rejection` returns the spec-table-correct `RejectionLifecycle`.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` (modify) — if existing `build_candidate_plans` inline tests reference the removed carve-outs, update them to the new path (probably just grep-and-delete the tests that assert the carve-out behavior explicitly).
3. `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` (modify) — already migrated in ticket 002 to read `AgendaState.committed`; this ticket confirms the new path via the classifier is the mechanism keeping it pinned.
4. No new integration or golden tests — the existing golden coverage (`golden_portfolio_planning`, `golden_planner_pathology`) exercises the integrated behavior.

### Commands

1. `cargo test -p worldwake-ai -- agenda_manager classify_rejection`
2. `cargo test -p worldwake-ai -- cargo_satisfaction_at_destination`
3. `cargo test -p worldwake-ai -- golden_portfolio_planning`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `./scripts/verify.sh`
