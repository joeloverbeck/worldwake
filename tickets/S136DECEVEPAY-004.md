# S136DECEVEPAY-004: Populate decisive_* fields at failure-path emission sites

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — failure-path emission sites in `worldwake-ai::agent_tick::execution`, `observation`, and `mod`
**Deps**: archive/tickets/S136DECEVEPAY-001.md

## Problem

Spec D6 (decisive_* slice): the four failure-path tags (`BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`, `SourceExpectationFailure`) gain `decisive_beliefs`, `decisive_records`, `decisive_world_observations` via ticket 001. These default to empty Vec at every construction site after ticket 001. This ticket wires real failure-input data into these fields at each emission site so the always-on causal history names the load-bearing facts behind every failure decision (FND-29A) — without new computation, since each emission site already has the failed-claim/observation set as a function input.

## Assumption Reassessment (2026-05-06)

1. The four failure-path emission sites (verified during reassessment):
   - `crates/worldwake-ai/src/agent_tick/execution.rs:448, 503` — `BlockerRecorded`. The typed `Discrepancy` payload is already a function input at these sites; its variants (`MissingObservation`, `BeliefStale`, `BeliefContradicted`, `SourceInvalidated`, etc. per `crates/worldwake-core/src/discrepancy.rs:8`) carry the contradicted-claim set the populator reads from.
   - `crates/worldwake-ai/src/agent_tick/execution.rs:140, 222` and `crates/worldwake-ai/src/agent_tick/mod.rs:497` — `ReplanTriggered`. The `ReplanReason` payload (`decision_event_payload.rs:362`'s reason field) names the stale-belief / contradicted-claim trigger.
   - `crates/worldwake-ai/src/agent_tick/observation.rs:123` — `ExpectationMismatch`. The `expected_materializations` and `mismatch_detail` inputs are already in scope at this emission.
   - `crates/worldwake-ai/src/agent_tick/mod.rs:621` — `SourceExpectationFailure`. The `cause: ExpectationFailureCauseTag` and source-attribution data are already in scope.
2. `BeliefRef`, `RecordRef`, `ObservationRef` types added by ticket 001 carry typed addresses (entity ID + claim key + tick / aspect). The populators construct them from each failure site's already-in-scope inputs without new belief queries — preserves spec design goal #2 ("no new computation").
3. Each output Vec is capped at `cognitive.decision_history_alternatives` (precedent: existing `rejected_alternatives` cap in `build_rejected_alternatives` at `planning.rs:991`).
4. Existing tests covering these emission sites: failure-path emissions are exercised indirectly by golden tests under `crates/worldwake-ai/tests/golden_*.rs`. Verify at implementation time which goldens currently assert payload shape vs. tag-only — none today assert `decisive_*` (the field didn't exist before ticket 001). Ticket 006 adds explicit golden coverage; this ticket's verification is focused-unit per emission site.
5. Boundary under audit: each failure site's typed-input → `decisive_*` projection. The mapping is mechanical because the typed inputs already discriminate which beliefs/records/observations are load-bearing — the populator reads, not infers.

## Architecture Check

1. The populators are derived projections — no new authoritative state (FND-3, FND-27). The data already flows through the typed `Discrepancy` / `ReplanReason` / `mismatch_detail` / `cause` payloads that the failure sites consume.
2. No new SystemFn, no new event tag (additive payload only — spec design goal #6).
3. The cap-driven Vec discipline mirrors `rejected_alternatives`'s existing per-agent cap, preserving the determinism contract (`BTreeMap`-stable iteration — FND-9).
4. The classifier is mechanical — every `decisive_*` entry traces to a specific function input on the failing site. No heuristic, no inference, no new belief query.

## Verification Layers

1. Per-site populator correctness → focused unit per emission site asserting the expected `decisive_*` content for a specific failure fixture.
2. Cap enforcement → focused unit per site asserting the Vec is bounded by `cognitive.decision_history_alternatives`.
3. Cross-site coherence → golden coverage in ticket 006 (deferred to that ticket; this ticket covers focused-only).

## What to Change

### 1. `BlockerRecorded` populator

At `crates/worldwake-ai/src/agent_tick/execution.rs:448` and `:503`, derive `decisive_beliefs`, `decisive_records`, `decisive_world_observations` from the `Discrepancy` payload's contradicted-claim and observation-input set.

Match each `Discrepancy` variant: `BeliefStale` / `BeliefContradicted` → push the contradicted `BeliefClaimKey` + tick + status onto `decisive_beliefs`. `MissingObservation` → push the missing observation's typed address onto `decisive_world_observations`. `SourceInvalidated` → push the source record onto `decisive_records`. Other variants populate the slot most semantically aligned (verify discrepancy-variant coverage at implementation time against `crates/worldwake-core/src/discrepancy.rs`).

Cap each Vec at `cognitive.decision_history_alternatives`.

### 2. `ReplanTriggered` populator

At `execution.rs:140`, `execution.rs:222`, and `mod.rs:497`, derive the three Vecs from the `ReplanReason` payload. Each `ReplanReason` variant names what triggered the replan; map to the corresponding decisive_* slot (e.g., `BeliefUpdate { claim_key }` → `decisive_beliefs.push(...)`).

### 3. `ExpectationMismatch` populator

At `observation.rs:123`, populate `decisive_world_observations` from the `mismatch_detail` input (which names the post-arrival observation that contradicted the expectation). Populate `decisive_beliefs` from any contradicted belief claims that participated in `expected_materializations`. `decisive_records` is typically empty here (records aren't a primary input to expectation-mismatch detection) — leave empty Vec unless implementation discovers a record-bearing path.

### 4. `SourceExpectationFailure` populator

At `mod.rs:621`, populate `decisive_*` from the `cause: ExpectationFailureCauseTag` input plus the source-attribution data already on the payload (e.g., `source: SourceKeyPayload`). `assumptions` remains empty Vec (spec D4 — no active-plan frame at this site).

### 5. Shared converter helpers

Where multiple emission sites map similar typed inputs to `decisive_*` (e.g., `BeliefClaimKey + tick + status` → `BeliefRef`), factor out small free-function helpers in a new module `crates/worldwake-ai/src/agent_tick/decisive_evidence.rs` or inline within the existing agent_tick submodule depending on call-site clustering. Keep helpers free of new dependencies — pure projections from already-in-scope state. Create the new module only if shared logic actually emerges; otherwise inline at each emission site.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — `BlockerRecorded` and `ReplanTriggered` populators)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — `ExpectationMismatch` populator)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — `ReplanTriggered:497` and `SourceExpectationFailure:621` populators)
- Likely: `crates/worldwake-ai/src/agent_tick/decisive_evidence.rs` (new — shared converters; create only if shared logic emerges, otherwise inline at sites)

## Out of Scope

- Promoting decisive classification to success-path tags (`GoalCommitted`, `PlanAdopted`) — Non-Goal in spec, deferred to follow-on.
- `assumptions` population (ticket 002).
- `rejection_dimension` population (ticket 003).
- Adding new `Discrepancy` or `ReplanReason` variants (uses existing taxonomy).
- Observer Section 3 rendering (ticket 005).
- Golden coverage (ticket 006).

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit per failure tag (`BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`, `SourceExpectationFailure`) asserting `decisive_*` carries the expected typed addresses for a specific failure fixture.
2. New focused unit per failure tag asserting Vec cap enforcement at `cognitive.decision_history_alternatives`.
3. Existing agent_tick suite passes: `cargo test -p worldwake-ai agent_tick::`.

### Invariants

1. Every `decisive_*` entry traces to a specific function-input field on the failing emission site — no heuristic classification, no derived inference (FND-29A).
2. Vecs are bounded by `cognitive.decision_history_alternatives`.
3. No new authoritative state.
4. No new belief queries — populators read from data already in scope.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/execution.rs::tests` — new focused units for `BlockerRecorded` (per `Discrepancy` variant slot) and `ReplanTriggered` populators.
2. `crates/worldwake-ai/src/agent_tick/observation.rs::tests` — new focused unit for `ExpectationMismatch` populator covering `mismatch_detail` and `expected_materializations` slots.
3. `crates/worldwake-ai/src/agent_tick/mod.rs::tests` (or sibling) — new focused unit for `SourceExpectationFailure` populator.

### Commands

1. `cargo test -p worldwake-ai agent_tick::execution`
2. `cargo test -p worldwake-ai agent_tick::observation`
3. `cargo test -p worldwake-ai agent_tick`
4. `cargo test -p worldwake-ai`
5. `./scripts/verify.sh`
