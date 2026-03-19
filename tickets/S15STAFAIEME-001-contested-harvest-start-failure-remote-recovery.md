# S15STAFAIEME-001: Contested Harvest Start Failure Recovers Via Remote Fallback

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None expected; golden/harness scope only unless a hidden runtime defect is exposed
**Deps**: `specs/S15-start-failure-emergence-golden-suites.md`, S08 start-failure architecture, E14/E15/E15c/E16d/S07 as cited by the spec

## Problem

S08 changed the shared start-failure contract for all autonomous actions, but active golden proof only covers the care-domain wound-disappearance race. We still lack a production-domain golden that proves a lawful contested harvest can fail at authoritative start, surface as `StartFailed`, reconcile through the next AI tick, and cause a longer recovery chain rather than a dead end.

## Assumption Reassessment (2026-03-19)

1. Current golden production coverage includes `golden_resource_exhaustion_race` and replay coverage in [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs), but that scenario only proves contention/exhaustion/conservation. It does not assert `ActionTraceKind::StartFailed`, per-agent `planning.action_start_failures`, or downstream remote fallback after the failed local attempt.
2. Existing explicit S08 golden proof is confined to `golden_care_pre_start_wound_disappearance_records_blocker` in [crates/worldwake-ai/tests/golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs), which checks both `ActionTraceKind::StartFailed` and next-tick decision-trace consumption of `action_start_failures`.
3. The target gap is missing golden E2E coverage, not missing focused/unit coverage. Focused/runtime coverage already exists for the failure pipeline in [crates/worldwake-ai/src/agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs), [crates/worldwake-ai/src/failure_handling.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs), and [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs). Full action registries are required because the contract spans production, travel, needs, tracing, and authoritative start rejection.
4. The ordering contract here is mixed-layer: same-snapshot symmetric harvest selection followed by authoritative action-lifecycle divergence at start time, then next-tick AI reconciliation. The divergence is caused by delayed authoritative system resolution of a scarce local source, not by differing motive score or priority class.
5. The scenario must isolate one intended fallback branch. Lawful competing affordances such as starting with owned food, an alternate nearby seller, or another local unowned food lot should be excluded from setup so the losing agent's post-failure branch is specifically "travel to remote orchard -> harvest -> eat".
6. Current docs already describe the intended gap in [specs/S15-start-failure-emergence-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S15-start-failure-emergence-golden-suites.md). No contradictory active ticket exists because `tickets/` currently contains only the template and README.
7. Scope correction: if implementation reveals that the current production-domain path cannot produce a lawful start failure without production/runtime fixes, stop and revise the ticket scope before landing engine changes. Do not silently expand this ticket into a production-architecture change.

## Architecture Check

1. A dedicated production golden is cleaner than broadening the care golden because it proves S08 through an ordinary harvest path, keeping the causal chain legible and domain-specific.
2. No backward-compatibility shims or special-case scheduler paths should be added. The scenario must run through the existing shared BestEffort autonomous action path.

## Verification Layers

1. Both agents lawfully generated/select the local harvest branch from the same local knowledge snapshot -> decision trace in the golden scenario.
2. The losing harvest attempt fails at authoritative start through the shared S08 path -> action trace `StartFailed` plus scheduler `action_start_failures`.
3. The next AI tick consumes the structured failure and drops the stale failed branch -> decision trace `planning.action_start_failures` and plan-history assertions on the losing agent.
4. Recovery proceeds through distant travel, harvest, and hunger relief -> authoritative world state and action trace commits for travel/harvest/eat.
5. Commodity totals remain bounded by explicit source stock -> authoritative world-state conservation checks, not delayed event-log inference.

## What to Change

### 1. Add the production golden scenario

Add `golden_contested_harvest_start_failure_recovers_via_remote_fallback` and `golden_contested_harvest_start_failure_recovers_via_remote_fallback_replays_deterministically` to [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs).

Use a finite local orchard that can satisfy exactly one immediate harvest start and a distant fallback orchard reachable through the normal topology. Assert:

- one agent commits the local harvest chain
- the losing agent records `StartFailed` on harvest before remote recovery begins
- the next AI tick shows the structured start failure and no stale retained failed step
- the losing agent later travels, harvests remotely, and eats

### 2. Add only minimal shared harness support if required

If the scenario setup becomes repetitive, add narrowly scoped helper composition to [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs). Helpers must only compose existing world setup and tracing paths; they must not create a test-only shortcut around real system registration or authoritative start handling.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, only if needed)

## Out of Scope

- `crates/worldwake-ai/tests/golden_trade.rs`
- `crates/worldwake-ai/tests/golden_emergent.rs`
- production scheduler, affordance, or action-handler redesign
- S10 pricing work
- replacing trace assertions with weaker "no later commit happened" inference

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback_replays_deterministically -- --exact`
3. Existing guardrail: `cargo test -p worldwake-ai golden_resource_exhaustion_race -- --exact`
4. Existing guardrail: `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
5. Owning binary: `cargo test -p worldwake-ai --test golden_production`

### Invariants

1. Belief-only planning remains intact: the losing agent's fallback branch must arise from lawful beliefs and observation, not omniscient test-side mutation.
2. Start failure remains a recoverable shared runtime contract: no crash, no stuck stale plan, no special production-only recovery path.
3. Commodity conservation remains true: authoritative apple totals never exceed the combined stock explicitly seeded into the local and remote orchards.
4. Systems remain state-mediated: production, travel, needs, and AI interact through authoritative state and traces, not direct cross-system calls.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` — add the contested-harvest start-failure golden and deterministic replay companion.
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` — optional minimal helper reuse if scenario setup would otherwise duplicate existing harness composition.

### Commands

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai --test golden_production`
