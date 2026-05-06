# S140ARTLIFAXE-002: artifact_lifecycle_system 5-stage refactor + event-driven cross-axis cascades

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `artifact_lifecycle_system` per-axis stage refactor; replaces ticket 001's placeholder cross-axis writes with `EventTag::ArtifactTransition` emissions read by later handler stages within the same tick
**Deps**: archive/tickets/S140ARTLIFAXE-001.md

## Problem

Ticket 001 lands the per-axis fields on `ArtifactHeader` and migrates all consumers, but it uses placeholder direct cross-axis writes at action commit handlers (e.g., `header.legal_effect = Fulfilled` followed immediately by `header.actionability = Closed`). The spec requires cross-axis effects to flow through emitted events read by later stages of `artifact_lifecycle_system`, not through synchronous cross-axis writes (FND-26: systems interact through state, not direct calls; spec Design Goal 8 + Non-Goal 3). This ticket refactors `artifact_lifecycle_system` from its current single-pass loop into five fixed-order stages (existence → legal_effect → credibility → visibility → actionability) and replaces 001's placeholder cross-axis writes with event-driven cascades that the actionability stage observes within the same tick.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. After ticket 001 lands, `artifact_lifecycle_system` (`crates/worldwake-systems/src/artifact_lifecycle.rs:8-62` pre-001) is a single-pass loop that handles only TTL expiry. Existing tests `artifact_lifecycle_system_expires_active_artifact_at_expiration_tick:194`, `artifact_lifecycle_system_leaves_nonexpiring_artifact_active:222`, `artifact_lifecycle_system_does_not_expire_before_expiration_tick:250`, `bounty_ttl_expiry_releases_encumbrance:278` exercise this surface; they were updated by 001 to assert axis values, and this ticket extends them to assert that fulfillment/expiry transitions emit `EventTag::ArtifactTransition` events. Action-commit-side tests in `artifact_actions.rs` (`claim_bounty_transfers_reward_and_fulfills_bounty:2670`, `withdraw_bounty_releases_encumbrance_without_transfer:2841`, etc., per ticket 001's assumption-reassessment item 1) are also extended.
2. Spec deliverable D3 names the 5 stages and the cross-axis observation pattern. Spec Design Goal 8 mandates event-driven cross-axis flow; spec Non-Goal 3 ("no auto-derivation") rules out direct cross-axis writes from any handler.
3. **Cross-system shared abstraction boundary**: The boundary under audit is the contract between action commit handlers (which write the proximate axis directly and emit the transition event) and `artifact_lifecycle_system`'s actionability stage (which reads emitted events from earlier-stage handlers within the same tick and writes the cascaded axis). Both sides live in `worldwake-systems`, but the contract itself is the event-log surface — the action handler does not call the lifecycle handler; the event log mediates.
4. **Information-path refactor stance**: For the same fact "this bounty was fulfilled and is therefore closed", the pre-002 path was direct cross-axis writes at the action commit handler (placeholder from 001). The canonical post-002 path is: action handler writes legal_effect + emits transition event → lifecycle_system stage 5 (actionability) reads the event and writes actionability + emits its own transition event. The placeholder direct-write is removed in this ticket; there is no temporary mixed-state coexistence after 002.
7. **Ordering layer**: The cross-axis cascade depends on action lifecycle ordering (action commit emits the legal_effect event) followed by event-log ordering within the same tick (actionability stage observes events emitted earlier in the tick). The spec's Risk 2 calls this out as the determinism contract. Verify that the existing system schedule places `artifact_lifecycle_system` after action commit within the tick — pin the schedule via `grep -rn "artifact_lifecycle_system" crates/worldwake-sim/` during implementation.
8. **Heuristic-removal discipline**: Removing 001's placeholder cross-axis writes does not remove a heuristic; it removes a transient compile-bridge. The substrate the placeholder stood in for is the lifecycle_system's actionability stage, which this ticket introduces.
13. **Adjacent contradictions**: If implementation discovers that `artifact_lifecycle_system` runs *before* action commit in the current schedule (so emitted events are not visible until next tick), classify the discrepancy as a required consequence of D3 rather than a separate ticket — adjust the schedule or use an in-tick event-buffer pattern; document the choice in the ticket completion notes.

## Architecture Check

1. The 5-stage handler is a single subsystem with internal phases — the alternative (one handler per axis as separate `SystemFn` registrations) would require cross-system dependencies between the new `SystemFn`s, violating FND-26 more deeply than the current design. Internal stages within one handler share authoritative state via the event log, which is the same mediation pattern but co-located in one module.
2. Event-driven cross-axis cascade is FND-26-compliant by construction. Stage 5 (actionability) reads events from the same tick's stage-2 (legal_effect) emissions; nothing reads private state of another stage.
3. Fixed stage order (existence → legal_effect → credibility → visibility → actionability) is documented inline in the handler and is the deterministic contract. Within a stage, `BTreeMap`-stable iteration breaks ties.

## Verification Layers

1. Stage ordering and per-axis transition emission → action trace + event-log delta. Each axis transition emits an `EventTag::ArtifactTransition` event; the trace surface proves both ordering (existence < legal_effect < credibility < visibility < actionability) and content (axis name, prior, new, cause_event).
2. Cross-axis cascade (e.g., legal_effect Fulfilled → actionability Closed) → event-log delta showing two `ArtifactTransition` events in the same tick with the actionability event's `cause_event` pointing at the legal_effect event.
3. TTL-expiry behavior unchanged → existing inline tests in `artifact_lifecycle.rs` pass with cascade assertions added.
4. Removal of 001's placeholder direct cross-axis writes → grep for the `// S140-001 placeholder, replaced by S140ARTLIFAXE-002` comment marker; zero matches post-002.
5. Action-commit handler integration → existing `claim_bounty_*` tests in `artifact_actions.rs` pass; new assertion verifies the actionability transition is emitted by the lifecycle handler in the same tick, not by the action commit handler.

## What to Change

### 1. Refactor `artifact_lifecycle_system` into 5 ordered stages

Replace the current single-pass loop in `crates/worldwake-systems/src/artifact_lifecycle.rs:8-62` (post-001 line numbers may differ) with a function body that runs 5 named stages in fixed order:

1. **existence** — short-circuit handler for `Destroyed` terminal state. If an artifact's `existence` is `Destroyed`, skip remaining stages for that artifact.
2. **legal_effect** — TTL expiry (the existing logic, restructured); also reads suspension events, revocation events, fulfillment-cause events from the same tick. The proximate write happens here OR at the action handler that observed the cause; this stage handles automatic transitions like TTL expiry.
3. **credibility** — observes contradicting-testimony events (writes `Disputed`) and evidence-against events (writes `Refuted`).
4. **visibility** — observes post events (writes `Posted`), unstaging events (writes `Hidden`), rumor-saturation events (writes `WidelyKnown`).
5. **actionability** — reads `ArtifactTransition` events emitted by stages 2 and 3 in this same tick. On observing `legal_effect: Fulfilled/Expired/Revoked` it writes `actionability: Closed { closed_at, cause: <derived from legal_effect cause> }`. On observing `credibility: Refuted` it also writes `Closed { cause: Refuted }`. Also handles jurisdiction-conflict events (writes `Blocked`) and proof-pending events (writes `AwaitingProof`).

Each stage emits `EventTag::ArtifactTransition` for any axis it writes, with `prior` / `new` / `cause_event` populated.

### 2. Replace ticket 001's placeholder cross-axis writes at action commit handlers

In `crates/worldwake-systems/src/artifact_actions.rs`, locate the `// S140-001 placeholder, replaced by S140ARTLIFAXE-002` comments at the 4 mutation sites (lines 1193, 1293, 1382, 1497 pre-001). At each site:

- Keep the proximate axis write (e.g., `header.legal_effect = Fulfilled`) — this is the immediate causal fact the action handler knows.
- Add an `ArtifactTransition` event emission for the proximate axis change, with the action's `EventId` as `cause_event`.
- Remove the placeholder cascaded axis write (e.g., the `header.actionability = Closed` line); the actionability stage of `artifact_lifecycle_system` will observe the just-emitted event within this same tick and write the cascade.

### 3. Verify same-tick handler ordering

Pin the schedule order: action commit handlers must run before `artifact_lifecycle_system` within the tick so emitted events are visible to stage 5. Grep for `artifact_lifecycle_system` registration in `crates/worldwake-sim/src/` and confirm; adjust order if needed (or add an in-tick event-buffer pass if scheduling cannot place lifecycle after commit).

### 4. Update existing tests + add new cascade tests

Update inline tests in `artifact_lifecycle.rs` and `artifact_actions.rs` to assert (a) the proximate axis transition event was emitted, and (b) within the same tick, the cascaded actionability transition was emitted by the lifecycle handler with `cause_event` pointing back at the proximate event.

## Files to Touch

- `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify — 5-stage handler refactor + cascade observation)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify — replace 4 placeholder cross-axis-write sites with proximate-axis-write + event emission)
- Likely: `crates/worldwake-sim/src/scheduler.rs` or `crates/worldwake-sim/src/system_dispatch.rs` (modify — confirm or adjust handler order); pin during implementation via `grep -rn "artifact_lifecycle_system" crates/worldwake-sim/`

## Out of Scope

- New axis enums, new event-tag variant, transition payload type — landed by S140ARTLIFAXE-001.
- `Discrepancy::ArtifactNotActionable` and decision-trace axis surfacing — covered by S140ARTLIFAXE-003.
- Observer rendering of transition history — covered by S140ARTLIFAXE-005.
- E2E goldens — covered by S140ARTLIFAXE-006.
- New transition trigger sources outside the existing TTL/fulfill/withdraw/destroy paths (e.g., suspension by jurisdiction conflict, refutation by evidence-against) are scaffolded as stage handlers in this ticket but their event-source emitters are landed when their domains arrive — this ticket is the cascade plumbing, not new lifecycle triggers beyond what already exists.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems --lib artifact_lifecycle` — TTL-expiry test asserts both legal_effect transition event AND cascaded actionability transition event in the same tick.
2. `cargo test -p worldwake-systems --lib artifact_actions claim_bounty_transfers_reward_and_fulfills_bounty` — emits legal_effect Fulfilled event from commit handler; lifecycle handler emits actionability Closed cascade in same tick.
3. `cargo test -p worldwake-systems --lib artifact_actions withdraw_bounty_releases_encumbrance_without_transfer` — emits legal_effect Revoked event; lifecycle handler emits actionability Closed cascade.
4. New test: cascade event ordering — within a single tick, the actionability transition's `cause_event` field references the legal_effect transition's event ID.
5. Existing suite: `cargo test --workspace`.

### Invariants

1. The 4 placeholder comment markers (`// S140-001 placeholder, replaced by S140ARTLIFAXE-002`) are removed from `artifact_actions.rs`. Verified by grep.
2. No action commit handler writes to the `actionability` axis directly post-002 (only stages of `artifact_lifecycle_system` write to actionability).
3. Per-axis stage order in `artifact_lifecycle_system`: existence → legal_effect → credibility → visibility → actionability. Documented inline.
4. Every axis mutation emits an `EventTag::ArtifactTransition` event with `(artifact, axis, prior, new, cause_event, at)` populated.
5. Existing TTL-expiry behavior is preserved: a bounty whose `expires_at` is reached transitions `legal_effect: Active → Expired` and `actionability: Actionable → Closed { cause: LegalEffectExpired }` in the same tick.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify) — add a `legal_effect_expired_cascades_to_actionability_closed_in_same_tick` test asserting both transition events emit and the actionability event's `cause_event` references the legal_effect event.
2. `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify) — extend `artifact_lifecycle_system_expires_active_artifact_at_expiration_tick` to assert the cascade event is emitted alongside the existing TTL behavior.
3. `crates/worldwake-systems/src/artifact_actions.rs` (modify) — extend `claim_bounty_transfers_reward_and_fulfills_bounty` to assert legal_effect transition event from commit handler + actionability cascade from lifecycle stage 5.
4. `crates/worldwake-systems/src/artifact_actions.rs` (modify) — extend `withdraw_bounty_releases_encumbrance_without_transfer` similarly for the Revoked path.

### Commands

1. `cargo test -p worldwake-systems --lib artifact_lifecycle artifact_actions -- --nocapture`
2. `cargo test --workspace`
3. `scripts/verify.sh`
