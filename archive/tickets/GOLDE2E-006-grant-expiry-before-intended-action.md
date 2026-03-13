# GOLDE2E-006: Grant Expiry Before Intended Action

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None required — the existing queue/grant transition runtime already handles grant-loss recovery
**Deps**: None (`facility_queue_system()` already expires stale grants in `crates/worldwake-systems/src/facility_queue.rs`)

## Problem

Scenario 9 shows grants being promoted and used promptly, and scenario 9b already proves patience-based queue abandonment. The remaining gap is narrower: what happens when a grant is already promoted, but the actor reprioritizes into a different urgent action before consuming that grant? The grant should expire authoritatively, the AI runtime should notice the lost grant through its normal facility transition bookkeeping, and the original facility-use goal should recover through standard replanning once it becomes the best local option again.

## Report Reference

Backlog item **P-NEW-5** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4).

## Assumption Reassessment (2026-03-13)

1. `GrantedFacilityUse::expires_at` exists and `expire_stale_grant()` runs in `facility_queue_system`.
2. `QueueGrantExpired` event tag exists in `worldwake-core/src/event_tag.rs`.
3. The AI runtime already tracks queued facility intents and facility queue/grant transitions in `crates/worldwake-ai/src/agent_tick.rs`; this ticket should verify that grant expiry uses that existing path rather than introducing a new alias path.
4. The golden harness can already set short grant expiry windows via `ExclusiveFacilityPolicy::grant_hold_ticks`.
5. Because `facility_queue_system()` promotes only ready queue heads, the scenario should be framed as "grant expires after promotion while the actor is diverted by a higher-priority local need", not as a missing queue-aware planning path.

## Architecture Check

1. Grant expiry should flow through the same runtime dirtiness and replanning path already used for queue/grant transitions and patience-based abandonment.
2. No special-case grant-renewal logic and no compatibility aliasing. The facility queue system clears the grant; the AI notices the lost grant on the next decision cycle and replans from current local state.
3. The test should only require eventual re-entry into the original facility-use path after the interrupting need is resolved. It should not force the agent to pursue the original goal while a higher-priority need remains legitimately active.

## Engine-First Mandate

If implementing this e2e suite reveals that post-grant-expiry replanning, re-queue behavior, or the interaction between grant expiry events and the AI decision runtime is incomplete or architecturally unsound, do not patch around it. Extend the existing queue-transition/runtime architecture so grant loss and patience expiry continue to share one coherent path. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_production.rs`

**Setup**: Agent at a facility with exclusive policy and very short grant expiry (likely 1-2 ticks). Configure the agent so hunger initially drives queueing for the exclusive facility, then an already-local competing need crosses into a higher-priority band before the next tick's action request. That detour should be satisfiable without the facility so the grant can expire unused.

**Assertions**:
- Agent queues for facility, receives grant.
- A competing need causes a local detour before the exclusive action starts.
- Grant expires (`QueueGrantExpired` event emitted).
- After the detour resolves, the agent re-queues or otherwise replans back into the original facility-use path and eventually completes the hunger-driven goal.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `reports/golden-e2e-coverage-analysis.md` (modify after implementation)
- Engine files only if the new test exposes a real grant-loss runtime gap

## Out of Scope

- Grant renewal mechanics
- Multiple concurrent grant expirations
- Patience timeout (separate ticket GOLDE2E-005)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_grant_expiry_before_intended_action` — agent's grant expires, agent re-queues or replans
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`
4. Full workspace lint: `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual grant manipulation after setup
2. At most one active grant per facility at any time
3. Conservation holds throughout
4. Grant expiry recovery reuses the existing queue/grant transition runtime path rather than adding a second grant-specific control flow

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-5 from the Part 3 backlog
- Update Part 4 summary statistics
- Record any engine/runtime architectural change only if the test proves one was necessary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_grant_expiry_before_intended_action` — proves grant expiry recovery

### Commands

1. `cargo test -p worldwake-ai golden_grant_expiry`
2. `cargo test -p worldwake-ai golden_`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-13
- What changed:
  - Reassessed the ticket assumptions against the current queue/grant architecture and narrowed the scope to the real runtime seam: grant expiry after promotion during a higher-priority local detour.
  - Added `golden_grant_expiry_before_intended_action` in `crates/worldwake-ai/tests/golden_production.rs`.
  - Updated `reports/golden-e2e-coverage-analysis.md` to record the new proven scenario, remove backlog item `P-NEW-5`, and refresh the current coverage counts.
- Deviations from original plan:
  - No engine/runtime code change was required. The existing facility queue + AI runtime architecture already handled grant-loss recovery correctly.
  - The final proof does not require observing a visible second waiting-state frame, because queue re-entry and re-promotion can happen in the same tick under the current scheduler. The durable proof is a second real promotion plus eventual hunger relief.
- Verification results:
  - `cargo test -p worldwake-ai golden_grant_expiry_before_intended_action`
  - `cargo test -p worldwake-ai golden_`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
