# S139EPISENSUB-006: Golden coverage for AskWitness and observer trace audit

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: No — adds new test file and exercises the existing observer decision-trace surface
**Deps**: archive/tickets/S139EPISENSUB-001.md, archive/tickets/S139EPISENSUB-002.md, tickets/S139EPISENSUB-003.md, tickets/S139EPISENSUB-004.md, tickets/S139EPISENSUB-005.md, tickets/S139EPISENSUB-007.md

## Problem

The full S139 pipeline (foundation, profile extension, dispatch, emitter, ranking) must be exercised end-to-end before merge. This ticket adds `crates/worldwake-ai/tests/golden_epistemic_sensing.rs` with six scenarios proving the contract, audits the existing observer decision-trace surface to confirm `GoalKind::AskWitness` commits render, and confirms the existing 1440-tick survival goldens remain green (no regression from the new emitter).

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Worldwake's golden-test conventions are documented in `docs/golden-e2e-testing.md` (canonical guide per `tickets/README.md`). Existing precedent goldens for goal-emission scenarios: `golden_share_belief.rs` (closest structural analog — both `ShareBelief` and `AskWitness` are testimony-path goals routing through co-located agents). Verify the file's structure during implementation.
2. The six scenarios encoded in `specs/S139-epistemic-sensing-subgoals.md` "Validation and Falsification" section: stale-belief verification, cold-start ask, FOUNDATIONS Scenario G chain, critical-survival suppression, cooldown gate, plan-failure replan. Each scenario is one `#[test]` function.
3. Shared abstraction boundary under audit: end-to-end behavior of the AskWitness goal layer — candidate emission, ranking, plan search, action execution, belief import, satisfaction predicate, decision-trace surfacing. The intended invariant is "agent autonomously verifies a stale belief through testimony when co-located with the original witness, respecting per-agent thresholds, cooldowns, and stress suppression."
4. Live `GoalKind` under test: `GoalKind::AskWitness` (landed by tickets 001-005, with satisfaction freshness completed by ticket 007). Live operator surface: `PlannerOpKind::Travel` + `PlannerOpKind::AskWitness`. Existing `EffectStep::AskWitness` and `apply_ask_witness_commit` are the action-layer endpoints; these are unchanged by S139.
5. Scenario isolation (precision-rules Rule 8): for each scenario, name what's intentionally excluded from setup. Scenario 1 (stale-belief verification) excludes competing self-care drives (no hunger pressure). Scenario 4 (critical-survival suppression) requires hunger pressure to trigger `GoalPriorityClass::Critical`; verify the existing survival-suppression fixtures in `golden_survival_*.rs` for the canonical pressure setup. Scenarios 3 and 6 may involve multiple witnesses / travel; isolate scenario contracts to avoid cross-contamination.
6. Coverage classification (precision-rules Rule 3): all six scenarios are *golden E2E* coverage. Targeted-unit coverage for the same paths landed in tickets 001 and 004; this ticket's role is end-to-end integration verification.
7. Authoritative-to-AI Impact (CLAUDE.md): all 7 checklist points are covered by tickets 001-005, ticket 007, and this ticket. Scenario 6 specifically exercises `handle_plan_failure` after travel-step revalidation fails (witness relocated).
8. Existing 1440-tick survival goldens (`golden_survival_baseline.rs`, `golden_survival_contested.rs`, etc.) must remain green. The new emitter fires only when `stale_evidence_barrier_threshold` is breached — verify with default profiles that no survival scenario triggers the threshold.

## Architecture Check

1. The golden tests exercise the live pipeline end-to-end through `SpawnedSimulation` and the standard tick loop. No mocking, no shortcuts. FND-29 debuggability: each scenario's expected chain is documented in the test header (belief setup → emission tick → commit tick → action tick → satisfaction tick).
2. No backwards-compat shims. The goldens assert the new contract directly; they do not preserve any pre-S139 behavior.
3. The observer-audit half of this ticket is verification, not new code: confirm `crates/worldwake-cli/src/bin/observer.rs` Section 3b (Decision History) renders the new variant. If the existing trace surface uses an exhaustive match, ticket 001's `GoalKindPlannerExt` integration plus this ticket's verification close the loop. If the trace requires a new arm, that's a scope-extending finding flagged here.

## Verification Layers

1. Scenario 1 (stale-belief verification) — emission tick visible in decision trace; commit tick visible in action trace; belief refresh visible in event-log delta (new `PerceptionSource::Report` provenance entry); satisfaction predicate flips visible in decision trace. Four distinct proof surfaces, one per layer (per precision-rules Rule 5).
2. Scenario 4 (critical-survival suppression) — emitter gate rejection visible in candidate-generation diagnostics (decision trace); no `AskWitness` candidate emitted; self-care goal proceeds. Decision trace is the primary proof surface (per precision-rules Rule 6 decision-trace preference for AI reasoning / suppression).
3. Scenario 5 (cooldown gate) — cooldown-active diagnostic visible at tick `t0 + cooldown - 1`; emission resumes at tick `t0 + cooldown`. Decision trace + emitter diagnostics.
4. Scenario 6 (plan-failure replan) — travel-step revalidation failure in action trace; replan trigger in decision trace; new candidate set on replan tick. Three layers per precision-rules Rule 5.
5. Authoritative ordering (per precision-rules Rule 4): event-log delta for belief refresh — assert exact ordering `(tick, sequence_in_tick)` per the action trace contract.

## What to Change

### 1. Create `golden_epistemic_sensing.rs`

`crates/worldwake-ai/tests/golden_epistemic_sensing.rs`. Six test functions, one per scenario from the spec's Validation section. Use the harness pattern from `golden_share_belief.rs` (the closest analog).

```rust
#[test]
fn golden_ask_witness_refreshes_stale_belief() {
    // Scenario 1: Setup an agent with a stale Report-sourced belief about subject X
    // from witness W; co-locate them; advance ticks; assert AskWitness emission, commit,
    // belief refresh, and satisfaction.
}

#[test]
fn golden_ask_witness_cold_start_emission() {
    // Scenario 2: Agent has only a Rumor-provenance low-confidence belief about X;
    // co-locate with witness W who has direct-observation belief; emit AskWitness;
    // assert Report-provenance belief lands.
}

#[test]
fn golden_ask_witness_scenario_g_contradicting_testimony() {
    // Scenario 3: Witness_a tells testimony A about X; witness_b tells testimony B about X.
    // Belief envelope transitions to Disputed; emitter fires for follow-up asks; assert
    // disputed status surfaces in trace.
}

#[test]
fn golden_ask_witness_critical_survival_suppression() {
    // Scenario 4: Hungry agent at Critical priority class; co-located witness; low-confidence belief.
    // Assert emitter gate rejects via EPISTEMIC_SENSING_POLICY suppression; self-care goal proceeds.
}

#[test]
fn golden_ask_witness_cooldown_gate() {
    // Scenario 5: Ask witness W about T at t0; advance to t0 + cooldown - 1 (cooldown active);
    // assert no AskWitness emission. Advance to t0 + cooldown; assert emission resumes.
}

#[test]
fn golden_ask_witness_plan_failure_replan() {
    // Scenario 6: Commit AskWitness; travel to witness's last-known place; witness has relocated.
    // Assert travel-step revalidation failure in action trace; replan in decision trace.
}
```

Each test follows the existing golden pattern: `spawn_scenario(def)` → standard tick loop → assertions on decision trace, action trace, event-log, and final world state. Use `DecisionTraceSink` and `ActionTraceSink` per `crates/worldwake-cli/src/bin/observer.rs` precedent.

### 2. Audit observer decision-trace surface

In `crates/worldwake-cli/src/bin/observer.rs`, confirm Section 3b (Decision History) renders `GoalKind::AskWitness` commits. The existing `DecisionEventPayload` matches over `GoalKind` — verify either:
- (a) the variant is automatically rendered by an existing trait-based or derive-based serialization, OR
- (b) the match is exhaustive and ticket 001's `GoalKindPlannerExt` integration covered it, OR
- (c) a new render arm is needed (scope-extending — flag explicitly in this ticket's What to Change update if it triggers).

The audit is a Read + grep operation; if (c) triggers, add the render arm in this ticket.

### 3. Confirm no regression in survival goldens

Run the existing 1440-tick survival goldens (`golden_survival_baseline`, `golden_survival_contested`, `golden_survival_items_decay`, `golden_survival_justice`, `golden_survival_patrol`, `golden_survival_production`, `golden_survival_tell`, `golden_survival_ask_consult`) and confirm they pass unchanged. The new emitter fires only when `stale_evidence_barrier_threshold` is breached, which does not occur at default profiles in survival-baseline. Document any survival-golden touch in the test plan.

## Files to Touch

- `crates/worldwake-ai/tests/golden_epistemic_sensing.rs` (new — 6 test functions)
- Likely: `crates/worldwake-cli/src/bin/observer.rs` (modify only if observer-audit finding (c) triggers — render arm for `GoalKind::AskWitness` if not automatically covered)
- `docs/generated/golden-e2e-inventory.md` (regenerated by `python3 scripts/golden_inventory.py --write --check-docs` after the new golden file lands)
- `docs/generated/golden-coverage-matrix.md` (regenerated by the same script)
- `docs/generated/golden-scenario-index.md` (regenerated by the same script)
- `docs/generated/golden-scenario-details/epistemic-sensing.md` (new — auto-generated by the inventory script)

## Out of Scope

- Calibration of motive_score magnitudes — handled in ticket 005 if Scenario rank-ordering surfaces imbalance.
- `TellTopic::SocialObservation` / `InstitutionalClaim` topic shapes — deferred per ticket 001's `build_payload_override`.
- New `InspectContainer` scenarios — deferred per S139 Non-Goals (future sibling spec).
- Multi-tick FOUNDATIONS Scenario G chain involving institutional response (warrant issuance) — Scenario 3 covers the belief-side contradiction; institutional response is downstream of S140.

## Acceptance Criteria

### Tests That Must Pass

1. All six new golden tests in `golden_epistemic_sensing.rs` pass.
2. Decision trace for Scenario 1 contains exactly one `AskWitness` candidate emission event followed by one `AskWitness` commit event before the satisfaction tick.
3. Action trace for Scenario 1 shows the `ask_witness` action's start → commit lifecycle with `(tick, sequence_in_tick)` ordering as specified in the test.
4. Event-log delta for Scenario 1 contains the new `BelievedEntityState` write with `PerceptionSource::Report { from: witness_a, chain_len: 1 }`.
5. Scenario 4's decision trace contains an `EPISTEMIC_SENSING_POLICY` suppression diagnostic; no `AskWitness` commit event.
6. Scenario 5's emitter diagnostics show `cooldown_active` rejection at the pre-elapsed tick.
7. Scenario 6's action trace shows `Travel` step revalidation failure followed by replan in decision trace.
8. Existing suite: `cargo test --workspace` passes (specifically including all `golden_survival_*` tests).

### Invariants

1. New emitter fires only when `stale_evidence_barrier_threshold` is breached at default `EpistemicDispositionProfile::default()` settings — verified by the unchanged survival goldens.
2. Decision-trace and action-trace surfaces remain the canonical proof for AI behavior — no behavior is asserted only through event-log inspection (per precision-rules Rule 6).
3. Each golden scenario isolates one intended contract (per precision-rules Rule 8); cross-scenario contamination would surface as flaky tests.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_epistemic_sensing.rs` (new) — 6 test functions per the six scenarios in the spec.

### Commands

1. `cargo test -p worldwake-ai --test golden_epistemic_sensing` — targeted run for the new goldens.
2. `cargo test -p worldwake-ai --test golden_survival_baseline --test golden_survival_contested --test golden_survival_items_decay --test golden_survival_justice --test golden_survival_patrol --test golden_survival_production --test golden_survival_tell --test golden_survival_ask_consult` — regression check on existing 1440-tick goldens.
3. `python3 scripts/golden_inventory.py --write --check-docs` — regenerate the docs/generated/* artifacts after the new golden file lands.
4. `cargo test --workspace` — full suite.
5. `./scripts/verify.sh` — full pre-PR gate.
