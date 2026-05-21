# S158BELVIEWLEAK-002: Production-job & load/capacity leak closure

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PerAgentBeliefView` production-job and load/capacity accessors
**Deps**: None

## Problem

Before this ticket, `PerAgentBeliefView::has_production_job`, `carry_capacity`,
and `load_of_entity` read authoritative world state for any entity with no
co-location/belief gate. An agent could "know" a remote workstation became
busy/free, or that a remote target's encumbrance changed, without any perception
— an FND-14 violation inherited by the human CLI action menu (FND-19). This
ticket closed the S158 D1 production + physical slice with focused
`belief_wall_trap` goldens.

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

## Verified Layers

1. Remote job start does not leak through `has_production_job` → focused
   `belief_wall_trap` golden accessor assertion.
2. Remote production activity remains available when backed by the existing
   `EntityBeliefAspect::Activity` carrier → focused `belief_wall_trap` golden
   assertion.
3. Remote load/capacity does not leak through `carry_capacity` or
   `load_of_entity` → focused `belief_wall_trap` golden accessor assertions.
4. Co-located busy workstation and co-located load remain observable
   (FND-14A negative controls) → focused `belief_wall_trap` golden assertions.

## Landed Changes

### 1. Gated production-job and load/capacity accessors

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, this ticket replaced the
ungated authoritative reads:
- `has_production_job`: self/co-located entities read the observable busy/idle
  state; remote entities return `true` only from the existing
  `EntityBeliefAspect::Activity` carrier when it records `ActionDomain::Production`.
- `carry_capacity`, `load_of_entity`: self, co-located, or directly possessed
  entities read the observable physical value; remote entities return `None`
  because no load/capacity belief aspect exists in the current model.

### 2. Added production + physical goldens

In `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs`, this ticket added:
- `golden_belief_wall_trap_remote_production_job_unseen` — proves a live remote
  production job is reachable in authoritative state but hidden from the remote
  belief view until an explicit activity belief exists.
- `golden_belief_wall_trap_remote_load_change_unseen` — proves live remote
  capacity/load are reachable in authoritative state but hidden from the remote
  belief view.
- Negative controls for co-located production-job and load observation.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (modify)

## Out of Scope

- Economic accessors (ticket 001), contention accessors (ticket 003).
- `can_control` / `believed_rights` (S158 Non-Goals).
- New `EntityBeliefAspect` variants (uses existing `Activity`).
- Doc updates (ticket 004).

## Acceptance Result

### Tests Passed

1. `golden_belief_wall_trap_remote_production_job_unseen` — no remote busy/free
   knowledge from live world truth; explicit activity belief restores the
   believed remote production-job state.
2. `golden_belief_wall_trap_remote_load_change_unseen` — no remote load knowledge.
3. Negative controls: co-located busy workstation and co-located load remain
   observable.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production-job/load/capacity accessor returns a remote entity's current
   world value; remote knowledge arrives only via belief carriers (FND-14).
2. AI and Human control sources produce identical lawful affordances (FND-19).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` — remote-job and
   remote-load goldens + negative controls; rationale: prove production/physical
   leaks closed without over-suppressing co-located observation.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_ai -- --list`
2. `cargo test -p worldwake-ai --test golden_ai scenarios::belief_wall_trap::golden_belief_wall_trap_remote_production_job_unseen -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai scenarios::belief_wall_trap::golden_belief_wall_trap_remote_load_change_unseen -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai belief_wall_trap`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-21.

- Closed the production-job leak by limiting `has_production_job` to self/local
  authoritative observation or the existing remote `Activity` belief carrier.
- Closed the physical load/capacity leak by limiting `carry_capacity` and
  `load_of_entity` to self/local/direct-possession observation. Remote
  load/capacity now returns unknown (`None`) because the current belief model has
  no load/capacity aspect and this ticket intentionally added no state.
- Added focused S158 belief-wall golden coverage for remote production-job and
  remote load/capacity truth leaks, plus co-located negative controls.

## Deviations

- The drafted wording allowed a remote load/capacity belief-backed value "if
  present"; live reassessment found no existing load/capacity belief aspect, so
  the landed S158 slice returns `None` for remote load/capacity instead of adding
  new belief state.
- The control-source-swap fingerprint was not extended as a separate assertion
  for these two new goldens. The existing `belief_wall_trap` control-source
  fingerprint remained green, and the new tests directly exercise the shared
  belief-view accessors used by both AI and human affordance enumeration.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai -- --list`
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::belief_wall_trap::golden_belief_wall_trap_remote_production_job_unseen -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::belief_wall_trap::golden_belief_wall_trap_remote_load_change_unseen -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai belief_wall_trap`
- Passed `cargo test -p worldwake-ai`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
