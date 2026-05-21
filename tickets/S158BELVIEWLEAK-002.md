# S158BELVIEWLEAK-002: Production-job & load/capacity leak closure

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PerAgentBeliefView` production-job and load/capacity accessors
**Deps**: None

## Problem

`PerAgentBeliefView::has_production_job`, `carry_capacity`, and `load_of_entity`
read current authoritative world state for any entity with no co-location/belief
gate. An agent can "know" a remote workstation became busy/free, or that a remote
target's encumbrance changed, without any perception — an FND-14 violation
inherited by the human CLI action menu (FND-19). S158 D1 (production + physical),
proven by S158 D4 goldens.

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Confirmed ungated reads in `crates/worldwake-sim/src/per_agent_belief_view.rs`:
   `has_production_job` (line 2207) → `self.world.has_component_production_job`;
   `carry_capacity` (1850) → `self.world.get_component_carry_capacity`;
   `load_of_entity` (1856) → `load_of_entity(self.world, entity)`. None gate on
   co-location or belief.
2. Source authority: `specs/S158-belief-view-remote-truth-leak-closure.md` D1
   (production + physical bullets). Remote production activity is belief-backed by
   the existing `EntityBeliefAspect::Activity`
   (`crates/worldwake-core/src/entity_belief_claim.rs:24,41`); no new aspect is
   introduced. Remote load/capacity falls back to belief or `None`.
3. Shared boundary under audit: the production-job and inventory/load accessor
   surface of `PerAgentBeliefView` consumed by `affordance_query.rs`
   (`TargetLacksProductionJob` precondition) and by failure handling. The gate
   predicate is co-location (FND-14A) or an existing belief entry; reuse the
   co-location predicate (`has_authoritative_local_visibility`) already used by
   `direct_container`.
4. Intended invariant: a remote production job starting/finishing unseen, or a
   remote target's load changing unseen, must NOT change the agent's candidates,
   plans, or affordance set until a lawful carrier arrives.
5. Live `GoalKind` under audit: `ProduceCommodity` / `RestockCommodity` (production
   path) and escort/trade load reasoning. Exact surface: `has_production_job`
   feeding the `TargetLacksProductionJob` affordance precondition (verify the
   current precondition name in `affordance_query.rs` during reassessment).
6. Intended verification layer: golden E2E in `belief_wall_trap.rs`; full action
   registries required (production).
13. Adjacent contradiction: a co-located workstation's busy/idle state IS
    physically observable (FND-14A), so co-located `has_production_job` reads are
    retained; only remote reads are gated. Required consequence, not a new bug.

## Architecture Check

1. Co-location-or-belief gating keeps the view a derived read-model; remote
   production activity is sourced from the existing `Activity` belief aspect, so
   no new stored state, no `Sourced<T>` (S158 defers that), and no behavior loss
   for lawfully-known facts.
2. No backward-compatibility shim: ungated `world.*` reads are replaced in place
   behind the gate; no parallel `believed_production_job` method (FND-28).

## Verification Layers

1. Remote job start/finish does not change candidates → decision trace.
2. Remote job start/finish does not change the affordance set (incl.
   `TargetLacksProductionJob`-driven affordances) → affordance fingerprint.
3. Remote load change does not alter route/trade/escort assumptions → decision
   trace / candidate assertion.
4. AI and Human control sources see identical lawful affordances → control-source
   swap fingerprint (pattern at line 598).
5. Co-located busy workstation still produces correct affordance (negative
   control) → affordance fingerprint.

## What to Change

### 1. Gate production-job and load/capacity accessors

In `crates/worldwake-sim/src/per_agent_belief_view.rs`:
- `has_production_job`: for a co-located entity return the observable busy/idle
  state; for a remote entity return the belief-backed value from
  `EntityBeliefAspect::Activity` if present, else `false`.
- `carry_capacity`, `load_of_entity`: for a co-located or directly-possessed
  entity return the observed value; for a remote entity return belief-backed value
  if present, else `None`.

### 2. Add production + physical goldens (failing-first)

In `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs`:
- `golden_belief_wall_trap_remote_production_job_unseen` — remote workstation
  starts/finishes a job unseen; assert no busy/free knowledge, unchanged
  `TargetLacksProductionJob`-driven affordances.
- `golden_belief_wall_trap_remote_load_change_unseen` — remote target's load
  changes unseen; assert planner does not adjust assumptions.
- Extend the control-source-swap fingerprint to these scenarios.
- Negative control: co-located busy workstation observable for both control
  sources.
Each new leak golden must fail against current `main` and pass after section 1.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (modify)

## Out of Scope

- Economic accessors (ticket 001), contention accessors (ticket 003).
- `can_control` / `believed_rights` (S158 Non-Goals).
- New `EntityBeliefAspect` variants (uses existing `Activity`).
- Doc updates (ticket 004).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_belief_wall_trap_remote_production_job_unseen` — no remote busy/free
   knowledge; `TargetLacksProductionJob`-driven affordances unchanged.
2. `golden_belief_wall_trap_remote_load_change_unseen` — no remote load knowledge.
3. Negative control: co-located busy workstation observable for AI and Human.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production-job/load/capacity accessor returns a remote entity's current
   world value; remote knowledge arrives only via belief carriers (FND-14).
2. AI and Human control sources produce identical lawful affordances (FND-19).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` — remote-job and
   remote-load goldens + negative controls + control-swap fingerprint extension;
   rationale: prove production/physical leaks closed without over-suppressing
   co-located observation.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai belief_wall_trap` (confirm names
   with `cargo test -p worldwake-ai --test golden_ai -- --list`)
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh`
