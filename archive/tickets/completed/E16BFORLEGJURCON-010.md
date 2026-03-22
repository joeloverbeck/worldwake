# E16BFORLEGJURCON-010: Implement force-office vacancy and challenger grace windows

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — force-office control state machine (systems), force control continuity state (core), political trace surfaces (sim)
**Deps**: E16BFORLEGJURCON-005, E16BFORLEGJURCON-006

## Problem

`OfficeForceProfile` already models three concrete timing parameters, but only `uncontested_hold_ticks` currently affects behavior. `vacancy_claim_grace_ticks` and `challenger_presence_grace_ticks` are live authoritative data with no semantics, which is an architectural contradiction: the simulation stores force-law policy it does not actually honor. This ticket should finish those two grace mechanisms in the authoritative force-control system without reintroducing hidden heuristics or placeholder shortcuts.

## Assumption Reassessment (2026-03-22)

1. `OfficeForceProfile` in [`crates/worldwake-core/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/offices.rs) already contains `uncontested_hold_ticks`, `vacancy_claim_grace_ticks`, and `challenger_presence_grace_ticks`, while the force control system in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) currently only consults `uncontested_hold_ticks`. The other two fields are unused substrate today.
2. Current focused coverage in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) proves controller establishment, contest clearing, departure/death cleanup, installation, and trace shape through tests such as `force_control_establishes_controller_before_installation`, `force_control_contest_clears_controller_and_sets_contested_state`, and `force_control_installation_requires_uncontested_hold_and_clears_claims`. No focused test currently covers vacancy claim grace or challenger presence grace. I verified this with `cargo test -p worldwake-systems -- --list`.
3. The live closure boundary for this ticket is authoritative force-office control resolution after vacancy activation: `contests_office` / `office_controller` / `OfficeForceState` mutate before any later `office_holder` installation. This is not a support-declaration or visible-vacancy-loss ticket.
4. This is not a planner-surface rewrite ticket, but it is AI-impacting under the repo's Authoritative-To-AI Impact Rule. `GoalKind::ClaimOffice`, `get_affordances()`, `generate_candidates()`, `search_plan()`, and runtime failure handling already rely on the current authoritative force-control timing, and existing AI/golden coverage in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs), [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), [`crates/worldwake-ai/src/search.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs), and [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) assumes the current controller-establish / contest timing. Scope remains authoritative first, but verification must include the downstream AI pipeline.
5. Ordering matters at the authoritative world-state layer, not strict tick-separation by itself. The intended divergence is driven by delayed system resolution using explicit grace windows: a sole claimant does not immediately become controller during `vacancy_claim_grace_ticks`, and a challenger does not immediately dissolve control during `challenger_presence_grace_ticks`.
6. This ticket weakens no heuristic without substrate. It adds behavior for two already-stored concrete policy fields, using explicit vacancy time plus explicit challenger-presence tracking. The current “immediate controller establish / immediate contest” behavior is the shortcut; this ticket replaces it with stored, causal state.
7. Not a stale-request or start-failure ticket. The first live boundary is the politics system tick in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs), not action start or request resolution.
8. Current force control only runs once the office has no living recognized holder because `evaluate_office_succession()` exits early on `living_holder(world, office)`. Therefore `vacancy_claim_grace_ticks` applies to post-vacancy force control, not to coups against a still-living recognized holder. That scope should remain explicit.
9. `E16BFORLEGJURCON-006` now owns canonical `InstitutionalClaim::ForceControl` metadata emission and belief propagation. This ticket should shape authoritative transition state so `-006` can project it cleanly, but it should not duplicate belief-layer work here.
10. Mismatch corrected: existing golden coverage already exercises force-control timing, even though it does not yet isolate the two new grace windows. In particular, [`golden_force_claim_ai_installation`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L2558) and [`golden_contested_force_claim_resolves_after_yield`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L2850) will shift once `vacancy_claim_grace_ticks` and `challenger_presence_grace_ticks` gain semantics. This ticket therefore needs at least targeted golden updates or additions rather than declaring goldens out of scope.
11. Mismatch corrected: the original E16b spec text names both grace fields, but the implemented `-005` state machine intentionally shipped only `uncontested_hold_ticks`. This ticket exists to close that remaining architectural gap.
12. Concrete arithmetic under current code:
    `vacancy_age = tick - vacancy_since`
    `control_hold = tick - control_since + 1`
    `challenger_presence = tick - challenger_since + 1`
    The new semantics should remain integer-based, deterministic, and profile-driven. No float thresholds or wall-clock assumptions are acceptable.

## Architecture Check

1. The clean design is to treat both grace windows as authoritative temporal continuity over concrete local state:
   - vacancy claim grace uses the already-stored `OfficeData.vacancy_since`
   - challenger presence grace uses an explicit challenger-presence timestamp in `OfficeForceState`
   This is better than implicit timer math in locals because downstream traces, debugging, and future systems can observe the same stored facts.
2. No backward-compatibility aliasing or parallel force paths. The same canonical force-control system should absorb the grace semantics rather than adding wrappers, compatibility flags, or parallel timing paths elsewhere.
3. The cleaner long-term shape is still concrete timestamp continuity, not abstract "pending stability" flags or derived score buckets. Extending `OfficeForceState` with one explicit challenger-presence timestamp is more robust than encoding these windows as opaque booleans because traces, record updates, save/load, and future institutional reactions can all read the same authoritative substrate.

## Verification Layers

1. Vacancy claim grace delays controller establishment -> authoritative world state (`office_controller` remains `None`, `OfficeForceState` reflects pending grace) + focused `worldwake-systems` test
2. Challenger presence grace delays contested transition -> authoritative world state (`office_controller` remains current controller during grace) + focused `worldwake-systems` test
3. Challenger persistence through grace clears controller and marks contested -> authoritative world state + focused `worldwake-systems` test
4. Challenger departure before grace expiry preserves controller continuity and clears pending challenger continuity -> authoritative world state + focused `worldwake-systems` test
5. Political trace reflects pending grace states distinctly from active control/contest -> focused `worldwake-sim` trace assertions
6. `ClaimOffice` affordances, candidate generation, and plan search still converge on `PressForceClaim` after the authoritative timing change -> existing `worldwake-ai` unit coverage plus any updated targeted golden assertion
7. Installation gate still uses `uncontested_hold_ticks` after grace windows complete -> authoritative world state + focused `worldwake-systems` test

## What to Change

### 1. Add explicit challenger-grace continuity to `OfficeForceState`

Extend `OfficeForceState` with the minimum additional timestamp needed to model challenger persistence concretely. Recommended shape:

```rust
pub struct OfficeForceState {
    pub control_since: Option<Tick>,
    pub challenged_since: Option<Tick>,
    pub contested_since: Option<Tick>,
    pub last_uncontested_tick: Option<Tick>,
}
```

Guidelines:
- `office_controller` remains the single authoritative controller identity surface.
- The new field tracks only temporal continuity of an active challenger presence.
- Do not duplicate controller identity in the component.

### 2. Implement `vacancy_claim_grace_ticks` in the force-control system

For vacant force offices with exactly one present eligible claimant and no current controller:
- if `tick - vacancy_since < vacancy_claim_grace_ticks`, do not establish controller yet
- keep `office_controller == None`
- preserve/record enough state to show the office is still in a pending vacancy-grace phase
- once the vacancy claim grace elapses and the same conditions still hold, establish controller and start `control_since`

This keeps “first uncontested controller counts as established control” grounded in the actual vacancy age rather than an invisible shortcut.

### 3. Implement `challenger_presence_grace_ticks` in the force-control system

When a force office has a current controller and one or more non-controller present eligible challengers:
- start `challenged_since` on first challenger presence
- keep the current controller during the challenger grace window
- if all challengers leave before the grace elapses, clear `challenged_since` and preserve controller continuity
- if challenger presence persists through `challenger_presence_grace_ticks`, clear `office_controller`, clear uncontested continuity, and set `contested_since`

Recommended rule:
- the earliest still-live present challenger starts the grace window
- additional challengers do not reset the clock while challenge pressure remains continuous
- if challenge pressure fully disappears and later returns, the clock restarts

### 4. Extend political trace outcomes for pending grace states

Add explicit trace outcomes in [`crates/worldwake-sim/src/politics_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) for the two new authoritative pending phases, for example:
- vacancy claim grace pending
- challenger grace pending under an existing controller

These are important for debugging and later golden coverage. They should be distinct from:
- `ForceControllerEstablished`
- `ForceControllerMaintained`
- `ForceContested`

### 5. Keep installation semantics unchanged after grace completion

Do not rewrite the `uncontested_hold_ticks` recognition mechanism. After controller establishment legitimately begins, installation should still require the configured uncontested hold window and the existing claim cleanup path from `E16BFORLEGJURCON-005`.

## Files to Touch

- `crates/worldwake-core/src/offices.rs` (modify — extend `OfficeForceState` if needed)
- `crates/worldwake-core/src/component_tables.rs` (modify — component tests/fixtures if state shape changes)
- `crates/worldwake-core/src/world.rs` (modify — component roundtrip/query tests if state shape changes)
- `crates/worldwake-core/src/world_txn.rs` (modify — component delta tests if state shape changes)
- `crates/worldwake-systems/src/offices.rs` (modify — implement both grace windows and focused tests)
- `crates/worldwake-sim/src/politics_trace.rs` (modify — add pending grace outcomes if trace surface changes)

## Out of Scope

- `InstitutionalClaim::ForceControl` belief propagation — `E16BFORLEGJURCON-006`
- new AI affordance kinds, planner ops, or alternate political goal families — `E16BFORLEGJURCON-007/008`
- Guard/public-order reactions to prolonged contests — deferred to E19
- Any float-based smoothing, hidden caps, or non-local “stability” scores

## Acceptance Criteria

### Tests That Must Pass

1. Sole claimant does not become controller before `vacancy_claim_grace_ticks` elapses
2. Sole claimant becomes controller once vacancy claim grace elapses and conditions still hold
3. New challenger does not immediately clear controller before `challenger_presence_grace_ticks` elapses
4. Persistent challenger clears controller and marks contested once challenger grace elapses
5. Challenger departure before grace expiry clears pending challenge state and preserves controller continuity
6. Installation still requires `uncontested_hold_ticks` after controller establishment
7. Existing affected AI suite: `cargo test -p worldwake-ai golden_force_claim_ai_installation`
8. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. `office_controller` remains the only authoritative controller identity source
2. All grace timing remains deterministic, integer-based, and profile-driven
3. No force-office behavior falls back to `OfficeData.succession_period_ticks`
4. Physical presence remains required for both control and challenge

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` — add focused tests for vacancy-claim grace, challenger-grace persistence, challenger-grace expiry, and challenger withdrawal
2. `crates/worldwake-sim/src/politics_trace.rs` — add trace-summary tests if new pending grace outcomes are introduced
3. `crates/worldwake-core/src/world_txn.rs` and related core component test modules — update component/delta roundtrip coverage if `OfficeForceState` shape changes
4. `crates/worldwake-ai/tests/golden_offices.rs` — update or add targeted force-law golden coverage if the new authoritative grace windows shift controller-establish / contest timing as expected

### Commands

1. `cargo test -p worldwake-systems force_control_vacancy_claim_grace_delays_controller_establishment`
2. `cargo test -p worldwake-systems force_control_challenger_presence_grace_delays_contest`
3. `cargo test -p worldwake-systems`
4. `cargo test -p worldwake-ai golden_force_claim_ai_installation`
5. `cargo test -p worldwake-ai golden_contested_force_claim_resolves_after_yield`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completed: 2026-03-22
- What changed:
  Added `challenged_since` to `OfficeForceState`, implemented `vacancy_claim_grace_ticks` and `challenger_presence_grace_ticks` in the authoritative force-office state machine, and added explicit politics-trace outcomes for the two pending grace phases.
- Deviations from original plan:
  No new AI golden scenarios were needed. Existing force-law goldens already covered the downstream pipeline and continued to pass once the authoritative grace semantics were added.
- Verification results:
  `cargo test -p worldwake-systems force_control_vacancy_claim_grace_delays_controller_establishment`
  `cargo test -p worldwake-systems force_control_challenger_presence_grace_delays_contest`
  `cargo test -p worldwake-systems`
  `cargo test -p worldwake-ai golden_force_claim_ai_installation`
  `cargo test -p worldwake-ai golden_contested_force_claim_resolves_after_yield`
  `cargo test -p worldwake-ai`
  `cargo clippy --workspace --all-targets -- -D warnings`
  `cargo test --workspace`
