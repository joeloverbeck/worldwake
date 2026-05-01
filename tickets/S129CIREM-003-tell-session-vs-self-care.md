# S129CIREM-003: Listener-tell agent stuck in chronic dirtiness during long social runs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Likely — tell-listener motive vs. critical-need ranking interaction, or per-agent observed-basin belief decay during long social engagements
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md, prior tell/communication spec (search archive for `tell` / `communication_profile` originator)

## Problem

`golden_survival_tell::survival_tell_lands_row_five` reports `Listener Bea dirtiness exceeded authored critical pm(820) for 1197 consecutive ticks (max allowed: 700)`. Bea is at her starting place "Rill Camp" (`scenarios/survival-tell.ron:135`), which is `[Camp, Latrine]`-tagged and co-located with both `Camp Well` (water source) and `Camp Washbasin` (`scenarios/survival-tell.ron:247–249`). Bea's `dirtiness_weight: 760` is high (`scenarios/survival-tell.ron:150`) and her metabolism's `wilderness_relief_dirtiness_penalty: 100` is moderate. With the parent S129 CI commit's hygiene-multiplier floor in place she should not be starved by ranking saturation; yet she is in critical dirtiness for 83% of the 1440-tick run.

Bea's role is the listener half of a tell relationship with `Scout Una` (at North Orchard). Tell sessions require co-location, so one of {Una travels to Rill Camp, Bea travels to North Orchard} must happen for tells to occur. North Orchard has `[Field, Farm]` tags, no latrine, no wash basin — Bea wilderness-relieves there during tell sessions and accumulates dirtiness penalty per relief.

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Pre-S129 baseline**: parent commit `fa0dd620` passes `golden_survival_tell::survival_tell_lands_row_five`. Verified during S129 CI investigation. The 1197-tick stall is a regression introduced by the S129 changeset.
2. **Hygiene modifier is not the proximate cause**: with `HYGIENE_FACTOR_FLOOR = 700` and the post-commit belief-storage rework, Bea's wash motive at saturated state is at least `motive * 0.7 * 0.7 = motive * 0.49`. With `dirtiness_weight: 760` and dirtiness pressure at saturation (`pm(1000)`), base wash motive `= 1000 * 760 / 1000 = 760`. Floored multiplier gives `760 * 0.49 = ~372`. Drive escalation can amplify this further. The arithmetic does not explain a 1197-tick chronic stall; some structural barrier is preventing wash from being committed.
3. **Live tell mechanics**: Bea has `tell_profile: (max_tell_candidates: 2, max_relay_chain_len: 3, conversation_memory_capacity: 12, conversation_memory_retention_ticks: 96)` and `communication_profile: (alarm_acceptance: 950, testimony_acceptance: 950, gossip_acceptance: 650)` (`scenarios/survival-tell.ron:162–172`). With `social_weight: 250`, Bea has a non-zero social motive. But "social" alone should not override `dirtiness_weight: 760` once dirtiness is critical. Investigation must confirm whether tell-related goals (`GoalKind::ShareBelief { .. }`, `GoalKind::AskWitness { .. }`, etc.) somehow rank above wash for Bea.
4. **Live `GoalKind::ShareBelief` and listener-side goals**: search `crates/worldwake-ai/src/ranking.rs` for `ShareBelief`, `AskWitness`, and any `Listen` analog. Establish what the priority class and motive arithmetic are for the listener role. The reassessment must verify whether the S129 ranking changes affected these motives indirectly (they should not, but verify).
5. **Live perception update on Bea**: the S129 CI commit added `wash_basin_state` to `BelievedEntityState` and populated it in `build_observed_entity_snapshot`. Verify that perception running in this scenario actually populates Bea's basin belief. If perception is throttled during tell engagements, basin state may not refresh and remote-from-Rill-Camp ranking may use stale beliefs.
6. **First failure boundary**: there are two candidates. (a) Bea is at North Orchard receiving tells continuously and never travels back to Rill Camp despite critical dirtiness. (b) Bea is at Rill Camp but commits to a tell-receive plan that blocks wash for the full duration. Decision-trace dump on Bea at the tick where dirtiness first crosses critical will distinguish these.
7. **Mismatch + correction**: `survival_tell` was the survival-row-five landing (S121 / per-need contracts era). The contract `critical_run_limits.dirtiness: 700` (`scenarios/survival-tell.ron:24`) was set when the ranking arithmetic did not yet have S129's hygiene multipliers. Even with the parent commit's floor, the new ranking landscape *changes* what equilibrium Bea reaches. The original contract was authored against pre-S129 arithmetic and the bug here may legitimately be that tell engagements now compete with hygiene state in a way the contract did not anticipate. **The contract should not be relaxed without naming the substrate.**
8. **Heuristic Removal Discipline (precision-rules §12)**: a tempting fix is to add a "self-care override" that forces wash whenever dirtiness is critical. This is exactly the kind of heuristic FND-3 forbids — the substrate must be ranking arithmetic + concrete state, not a hardcoded carve-out for a need.
9. **Cumulative arithmetic (precision-rules §15)**: Bea's `dirtiness_rate: 1`, `dirtiness_critical: 820`, starting `dirtiness: 200`. First critical at tick `(820 - 200) / 1 = 620` from basal alone. Plus wilderness reliefs at +100 each. With `bladder_rate: 4` and starting `bladder: 240`, Bea wilderness-relieves every ~165 ticks at North Orchard if she is there. So Bea reaches critical dirtiness much earlier than tick 620 if she is doing tell sessions away from Rill Camp.
10. **Coverage gap (precision-rules §3)**: no focused golden currently exercises "agent in chronic critical state during sustained tell engagement". The `survival_tell` golden tests that the contract holds end-to-end, but does not isolate the listener-during-tell-engagement invariant. This ticket should add the focused proof.
11. **Branch symmetry (precision-rules §13)**: `Una`'s and `Bea`'s utility profiles differ (`hunger_weight`, `thirst_weight`, `social_weight`). A "both agents handle long tell engagements without chronic stall" expectation is not symmetric — Una has different needs. The ticket scope should be Bea-as-listener specifically, not "all agents in tell scenarios".

## Architecture Check

1. **Investigation precedes fix**: the failure shape is still ambiguous (sub-cases 6a and 6b above). Decision-trace dump on Bea is required before any code change.
2. **No special-case for self-care**: do not add a "force wash when critical dirtiness" override. Per FND-3, the ranking arithmetic must be the substrate. If wash is not winning, either the ranking math is wrong or there is a structural blocker (e.g. an active tell action that is not interruptible) holding Bea.
3. **No backwards-compatibility shim**: do not add a "if listener and chronically dirty, bail from tell" hook. The fix is to make wash motive correctly outrank tell when both are valid and the agent is in critical hygiene state.

## Verification Layers

1. **First-critical decision** -> decision-trace dump for Bea at the first tick `T` when her dirtiness crosses critical. What does she choose? If she chooses to continue a tell session, the ranking is letting tell beat critical wash motive — fix the arithmetic.
2. **Continuing-action interruption** -> action-trace dump showing whether Bea is committed to a long-running action (e.g. a multi-tick tell or pick-up) that is not being interrupted by the new critical wash motive. If so, the `Interruptibility::InterruptibleWithPenalty` setting on the offending action is the contract surface.
3. **Plan-finding failure** -> if wash *is* selected but no plan is found, `places_with_wash_access` and the snapshot wash-state surface are the boundary.
4. **Belief currency on Bea's basin** -> if Bea travels to North Orchard, her belief about Camp Washbasin should remain intact (parent commit added `BelievedEntityState::wash_basin_state`). Verify by inspecting Bea's `AgentBeliefStore` mid-run.

## What to Change

### 1. Investigation: dump Bea's decision and action traces

Add a one-shot diagnostic test that runs `survival_tell` and prints:

- For each tick from Bea's first critical-dirtiness tick to tick 1440: Bea's `effective_place`, current active action, current decision (selected goal), and any `StartFailed` events.
- Bea's `AgentBeliefStore` for `Camp Washbasin` at ~ tick 800 (mid-engagement).
- Bea's `DeprivationExposure.dirtiness_critical_ticks` at the same tick.

Use this to classify the failure into one of the candidate root causes.

### 2. Fix (driven by trace findings)

Choose exactly one based on the trace:

- **If Bea is committed to a long tell action that won't interrupt under critical hygiene**: lower the action's `Interruptibility` cost, or extend the interruption logic to honor critical-class need pressure. Make sure this doesn't break tell mechanics generally.
- **If tell motive is ranking above wash even at saturated dirtiness**: investigate why. The `social_weight: 250` is moderate; with `dirtiness_weight: 760` and dirtiness pressure 1000, dirtiness should win the priority class. If it doesn't, there is a ranking arithmetic bug to fix.
- **If wash plans are not found**: investigate whether Bea's belief about Camp Washbasin is stale or absent, leveraging the parent commit's `BelievedEntityState::wash_basin_state` storage.
- **If Bea never returns to Rill Camp**: check whether travel-back-to-camp is being modelled as a "wash" plan or a separate "travel home" pattern. The wash anchor is `OpportunityAnchor::Entity(basin)` (S129 §16); the planner should compose travel + wash. If composition fails, that is the search surface to investigate.

### 3. Targeted golden coverage

After the fix, add `listener_with_critical_dirtiness_breaks_off_tell_to_wash`: focused fixture with two agents, ongoing tell engagement, push the listener's dirtiness to critical, assert the listener commits a wash within the dirtiness tolerance window.

## Files to Touch

- `crates/worldwake-ai/tests/` (new diagnostic + new focused golden)
- `crates/worldwake-ai/src/ranking.rs` or `crates/worldwake-ai/src/agent_tick/` (only after trace points there)
- `crates/worldwake-sim/src/` (only if interruptibility / start-path logic needs change)

## Out of Scope

- Drive-escalation wash recurrence — separate ticket S129CIREM-001.
- Late-game stuck idle in baseline / contested / scattered — completed in `archive/tickets/S129CIREM-002-late-game-stuck-idle.md`.
- Patrol vs self-care — separate ticket S129CIREM-004.
- Tell mechanics rebalancing for non-listener agents — out of scope unless trace shows a shared root cause.
- Survival contract tuning (`max_authored_critical_run_ticks` / `critical_run_limits.dirtiness`) — should not be touched without first naming the architectural substrate the relaxation rests on.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_tell::survival_tell_lands_row_five` — Bea's `dirtiness_critical_ticks` <= 700 (the contract).
2. New focused golden `listener_with_critical_dirtiness_breaks_off_tell_to_wash` — passes.
3. Existing tell-related goldens continue to pass (search `cargo test -p worldwake-ai golden_*tell*` for the active inventory).
4. Existing suite: `cargo test --workspace`, `./scripts/verify.sh`.

### Invariants

1. **Critical hygiene preempts non-critical social motive**: when `dirtiness >= critical_threshold` and a believed wash basin is reachable, wash motive must outrank `ShareBelief` / `AskWitness` / other listener-side tell motives unless the tell is itself critical (alarm propagation).
2. **No structural blocking by long-running social actions**: tell-related actions either decline to start when the actor is in critical hygiene, or carry interruptibility that lets the critical-need transition trigger replan.
3. **No special-case override**: the fix is named in ranking arithmetic, candidate generation, or interruptibility — not a bare "if dirtiness >= critical, force wash" hook.

## Test Plan

### New/Modified Tests

1. New diagnostic test (one-shot) that dumps Bea's decision/action traces in the failure window. Disposable.
2. New focused golden `listener_with_critical_dirtiness_breaks_off_tell_to_wash`.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_survival_tell -- --ignored --test-threads=1`
2. `cargo test -p worldwake-ai --test '*tell*'` (existing tell-related goldens)
3. `cargo test --workspace`
4. `./scripts/verify.sh`
