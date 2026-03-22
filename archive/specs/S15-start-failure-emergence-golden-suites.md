**Status**: COMPLETED

# S15: Start-Failure Emergence Golden E2E Suites

## Summary

Add cross-system golden E2E suites that prove S08's start-failure architecture as an emergent world contract rather than a heal-only regression. Current golden coverage proves one important S08 case: `TreatWounds` can lawfully reach `StartFailed` when wounds disappear before start, and AI carries the failure into blocked-intent memory instead of crashing. That is necessary, but it is not sufficient.

S08 changed shared authority/runtime behavior in four places:

1. BestEffort start failures now include lawful authoritative start rejection.
2. `Scheduler` now records structured `ActionStartFailure`.
3. AI consumes those failures through normal failure handling and blocked-intent reconciliation.
4. action traces now expose `StartFailed` as a first-class lifecycle event.

The current golden suite proves those surfaces only through the care domain. We still do not have end-to-end proof that the same contract holds when the failed action belongs to ordinary resource contention, trade opportunity drift, or political opportunity drift. Without that coverage, the golden docs currently over-compress S08 into "the heal race was fixed," which understates the architectural change.

This spec adds distinct golden suites where a lawful start failure in one system becomes the cause of a new downstream chain in another system, proving Principle 1 (maximal emergence), Principle 19 (revisable commitments), and Principle 24 (systems interact through state, not each other).

## Why This Exists

### Current coverage is too narrow

`docs/golden-e2e-scenarios.md` and `docs/golden-e2e-coverage.md` currently list only the care pre-start wound-disappearance regression as explicit S08 golden proof. Search of the active golden tests shows `StartFailed` assertions only in `golden_care.rs`.

That leaves three real gaps:

1. no golden proves a non-care BestEffort start failure through the shared action-framework path
2. no golden proves the next-tick AI reconciliation surface (`action_start_failures` -> blocker / plan clearing / alternative plan) outside care
3. no golden proves that start-failure recovery can itself become the hinge of a longer cross-system emergent chain

### Existing tests are related, but do not close the gap

- `golden_resource_exhaustion_race` proves same-tick contention no longer crashes the world, but it does not assert `StartFailed`, next-tick AI failure reconciliation, or a distinctive downstream recovery branch.
- the segmented supply-chain tests proved S08 trace value during development, but they are not active `golden_*` suites and the full combined chain is currently blocked on S10 trade pricing, so they do not provide present golden proof for S08.
- the existing political and social goldens prove locality and emergence, but not post-selection lawful start rejection and recovery.

## Phase

Phase 3: Information & Politics

## Crate

`worldwake-ai` for the golden suites and any small harness helpers

## Dependencies

- S08 completed
- E14 completed
- E15 / E15c completed
- E16d completed
- S07 completed

No new production architecture is required. This is golden coverage and harness-only work unless a currently hidden engine defect is revealed during implementation.

## Design Goals

1. Prove the shared S08 start-failure contract outside the care domain.
2. Use action traces and decision traces at the correct layers rather than inferring failures from missing downstream state.
3. Make each scenario produce a longer consequential chain after the start failure instead of stopping at "the action failed."
4. Avoid scenarios blocked on S10 pricing or other unfinished architecture.
5. Keep setups lawful and locality-respecting. The failure must arise from another agent or authoritative world change, not from test-side omniscient mutation shortcuts.

## Scenario Inventory

### Scenario 26: Contested Harvest Start Failure -> Remote Recovery

**File**: `crates/worldwake-ai/tests/golden_production.rs`

**Systems exercised**: Needs, Production, Travel, AI failure handling, action tracing, decision tracing, conservation

**Setup**:

- Two hungry agents start co-located beside a finite local orchard that can satisfy only one immediate harvest.
- A second orchard exists at a distant place and remains reachable through the normal travel graph.
- Both agents have lawful local knowledge of the nearby source and the remote fallback source.
- The local source should be set up so both agents can lawfully choose the local harvest from the same snapshot, but only one can actually start successfully.

**Emergent behavior proven**:

- both agents converge on the same locally rational harvest opportunity
- one harvest starts and commits
- the losing agent records `StartFailed` on the queued harvest start through the shared BestEffort path
- the next AI tick converts that structured start failure into ordinary blocker/plan-clearing behavior rather than crash, livelock, or silent disappearance
- the losing agent then replans toward the distant orchard, travels there, harvests, and eats

**Why this matters**:

This proves that S08 did not merely fix the care race. The same authority-to-AI handoff must hold for ordinary contested production, and the failed local attempt must become the cause of a new travel-and-survival chain rather than dead code.

**Assertion surface**:

1. action trace: the losing agent has a `StartFailed` harvest event before the remote recovery chain begins
2. decision trace on the next tick: the losing agent records the start failure in the planning trace and does not retain the stale failed step
3. authoritative state: the losing agent later reaches the remote orchard, harvests, and reduces hunger
4. conservation: authoritative commodity totals remain bounded by explicit source stock

**Distinct from existing Scenario 3d**:

Scenario 3d proves contention/exhaustion/conservation. Scenario 26 specifically proves the S08 trace + AI reconciliation contract and requires an alternative downstream branch after the failure.

### Scenario 27: Local Trade Opportunity Vanishes -> Production Fallback

**File**: `crates/worldwake-ai/tests/golden_trade.rs` or `golden_emergent.rs`

**Systems exercised**: Needs, Trade, Travel, Production, AI failure handling, action tracing, decision tracing

**Setup**:

- Two hungry buyers and one local seller start co-located.
- The seller has exactly one unit of edible stock and normal 1:1 trade remains sufficient, so this scenario is not blocked on S10 pricing.
- A remote orchard or equivalent non-trade fallback food source exists.
- Both buyers have lawful local belief about the seller's merchandise and the remote fallback.

**Emergent behavior proven**:

- both buyers can lawfully generate local `AcquireCommodity` plans that resolve through trade
- one buyer completes the trade first and consumes the only local stock
- the second buyer's queued trade start records `StartFailed` or authoritative start rejection through the shared S08 path because the local trade opportunity no longer exists
- the second buyer does not loop forever on the dead local trade branch
- instead, the second buyer replans to the distant production fallback, travels, acquires food, and eats

**Why this matters**:

This is the economic analogue of the care race. The local market opportunity changes between planning and start because another lawful agent acts first. The simulation must treat that as world drift, not engine failure, and the AI must recover into a new cross-system chain.

**Assertion surface**:

1. action trace: the losing buyer records `StartFailed` on the local trade attempt
2. decision trace next tick: the start failure is visible and the stale local trade step is not retained as current plan
3. authoritative outcome: the losing buyer later completes the distant acquire/eat fallback
4. negative loop check: no repeated failed local trade start cycle after the stock is gone

**Distinct from existing Scenario 2b**:

Scenario 2b proves successful buyer-driven trade. Scenario 27 proves lawful loss of a local trade opportunity, structured recovery, and downstream shift into production/travel.

### Scenario 28: Remote Office Claim Race -> Graceful Political Loss

**File**: `crates/worldwake-ai/tests/golden_emergent.rs`

**Systems exercised**: Social Tell, political candidate generation, Travel, office succession/installation, AI failure handling, action tracing, decision tracing

**Setup**:

- A vacant remote office exists under the ordinary support-law path.
- Two ambitious agents learn of the vacancy through lawful information channels already used in the social/political goldens.
- Both have reason to travel and attempt the same office claim.
- One claimant can arrive and install first through the ordinary `ClaimOffice` -> `DeclareSupport` path.

**Emergent behavior proven**:

- both claimants generate lawful political intent from local belief state, not from hidden authority reads
- the first claimant reaches the office and installs
- the second claimant's queued political action then fails at authoritative start because the office opportunity has been lawfully consumed
- the failure becomes a structured `StartFailed` event and next-tick plan reconciliation, not a crash and not a stale permanent political loop
- after the office is occupied, the losing claimant no longer generates a fresh claim path for that same office unless and until world state changes again

**Why this matters**:

This is the political proof of Principle 19. Intent is not entitlement. Planning to claim an office does not reserve it. Another actor can legitimately take the opportunity first, and the late claimant must lose gracefully through the same S08 recovery path.

**Assertion surface**:

1. action trace: losing claimant records `StartFailed` on the post-travel political action
2. decision trace next tick: start failure is visible and `ClaimOffice` for that occupied office disappears from generated/selectable candidates
3. authoritative state: the winner becomes office holder through the normal succession path
4. negative contract: no stale repeated claim-start attempts against the now-occupied office

**Distinct from existing political suites**:

Current office/emergent suites prove claim success, locality, social propagation, suppression, and force succession. They do not prove lawful post-selection loss of a political opportunity.

## Trace Guidance For These Suites

Each S15 scenario should intentionally use both trace layers:

- **Action traces** prove the start-failure lifecycle fact (`StartFailed`) and any required same-tick ordering.
- **Decision traces** prove that the next AI tick consumed the structured failure and moved the agent off the stale branch.

Do not infer S08 recovery solely from missing downstream commits. These suites exist specifically to prove the handoff between the authoritative start-failure record and the AI reconciliation layer.

## Deliverables

### S15-001: Scenario 26 in `golden_production.rs`

Add:

- `golden_contested_harvest_start_failure_recovers_via_remote_fallback`
- `golden_contested_harvest_start_failure_recovers_via_remote_fallback_replays_deterministically`

### S15-002: Scenario 27 in `golden_trade.rs` or `golden_emergent.rs`

Add:

- `golden_local_trade_start_failure_recovers_via_production_fallback`
- `golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically`

### S15-003: Scenario 28 in `golden_emergent.rs`

Add:

- `golden_remote_office_claim_start_failure_loses_gracefully`
- `golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically`

### S15-004: Update `docs/golden-e2e-coverage.md`

Required updates:

- add the new tests to the file layout counts
- add new cross-system chains for production, trade, and politics
- stop treating S08 as represented only by the care race
- replace stale S02c backlog wording that still references the archived traceability spec as the blocker when the active blocker is now S10 pricing

### S15-005: Update `docs/golden-e2e-scenarios.md`

Required updates:

- add scenario entries for S26-S28
- explain why each is distinct from the pre-existing success-path scenario it extends
- document the action-trace and decision-trace contract for each

### S15-006: Update `docs/golden-e2e-testing.md`

Add explicit guidance for S08-style coverage:

- when the contract is "lawful start rejection is recoverable," use `StartFailed` action traces plus next-tick decision-trace failure handling
- do not treat "no later commit happened" as sufficient evidence of start-failure reconciliation

## Non-Goals

1. No new production action framework or scheduler changes as primary scope.
2. No scenarios blocked on S10 bilateral pricing.
3. No test hacks that mutate authoritative state omnisciently after planning when the same drift can be produced by another lawful agent.
4. No collapsing these suites into focused unit tests; the point is cross-system emergence.

## Risks

1. Some candidate setups may accidentally prove only precondition invalidation rather than true post-selection world drift. The chosen setup must ensure the relevant branch was lawful when selected.
2. Trade scenarios can accidentally drift into S10 pricing failure rather than S08 start-failure recovery. Keep the price path unquestionably acceptable so stock exhaustion, not valuation, is the hinge.
3. Political scenarios can become brittle if they assert incidental tick counts instead of action lifecycle ordering. Use trace ordering and authoritative office-holder state.

## SystemFn Integration

No new production `SystemFn` registrations are part of S15.

The suites must exercise the existing registered systems through the real harness:

- needs / metabolism
- production / transport
- trade
- travel
- social Tell
- office succession and political actions

If a harness helper is added, it must only compose existing setup paths. It must not create a test-only execution path that bypasses the real system registry.

## Component Registration

No new components are introduced by S15.

## Outcome

- Completion date: 2026-03-20
- What actually changed:
  - Added production, trade, and political start-failure goldens proving the shared S08 `StartFailed` contract outside the care domain.
  - Added deterministic replay companions for all three new S15 scenario families.
  - Extended political trace ergonomics with direct office availability phases so the political start-failure golden can assert closure directly instead of reconstructing it from lower-level facts.
  - Updated the golden documentation tickets called out in this spec, so S08 is no longer represented as care-only coverage.
- Deviations from original plan:
  - The political trace enhancement landed as `OfficeAvailabilityPhase`, a broader authoritative trace vocabulary than the original claimability-only framing, because the shared trace also needs to read cleanly for force-law offices.
  - The political proof remained centered on the existing `declare_support` start boundary; no scheduler or request-resolution architecture changes were required.
- Verification results:
  - Passed `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback`
  - Passed `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback`
  - Passed `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully`
  - Passed `cargo test -p worldwake-ai`

The scenarios should reuse existing components and records, including as needed:

- `ResourceSource`
- `PerceptionProfile`
- `MerchandiseProfile`
- `TradeDispositionProfile`
- `DemandMemory`
- office/faction/support data already introduced by E16/E16d

Any helper additions must use ordinary component registration and transaction paths already present in the harness.

## FND-01 Section H Analysis

### Information-Path Analysis

These suites rely on existing lawful information paths:

- Scenario 26: both agents know the local and remote food sources through perception or seeded lawful beliefs
- Scenario 27: buyers know about the seller through co-location/perception and know the fallback source through ordinary belief setup
- Scenario 28: office opportunity knowledge arrives through the same social/locality mechanisms already proven by S13/S14

No suite may smuggle the decisive state change through omniscient test mutation. The opportunity loss must come from another agent lawfully acting first.

### Positive-Feedback Analysis

The main risk is retry spam:

- failed local harvest -> retry local harvest forever
- failed local trade -> retry same dead seller forever
- failed office claim -> retry against already occupied office forever

These suites should prove that S08's stored failure handoff participates in existing dampeners rather than creating loops.

### Concrete Dampeners

The relevant dampeners are already part of the architecture and should be visible in behavior:

- blocked-intent memory with TTL
- blocker clearing / plan invalidation
- finite inventories and source stock
- travel time to remote fallbacks
- office occupancy removing the original political opportunity

### Stored Vs Derived State

Stored state involved:

- `ActionStartFailure` records on `Scheduler`
- blocked-intent memory / runtime plan state
- inventories, source quantities, office holder relations, beliefs, and locations

Derived state involved:

- action-trace summaries of `StartFailed`
- decision-trace summaries of start-failure handling
- any per-test merged timeline helpers

No new cache or abstract truth layer should be introduced.

## Verification

Per scenario:

1. `cargo test -p worldwake-ai --test <targeted_test_binary> <test_name>`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

After implementation:

5. verify `docs/golden-e2e-coverage.md`, `docs/golden-e2e-scenarios.md`, and `docs/golden-e2e-testing.md` all reflect the new suites and updated S08 coverage story
