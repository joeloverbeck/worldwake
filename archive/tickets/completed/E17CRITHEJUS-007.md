# E17CRITHEJUS-007: Extend investigate commit with SuspectedTheft detection

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — targeted extension of existing investigate handler
**Deps**: E17CRITHEJUS-002 (landed `ViolationKind::SuspectedTheft`; live code already includes typed `SocialObservationDetail::SuspectedTheft`)

## Problem

When an owner investigates a missing entity (S27's `InvestigateViolation`), the handler currently records `WitnessedAbsence` unconditionally. It cannot distinguish "item depleted/moved" from "my owned item was stolen." E17 needs the investigate commit to detect the ownership mismatch and record `SuspectedTheft` when the investigator owned the missing entity.

## Assumption Reassessment (2026-03-26)

1. Shared boundary under audit: [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs)`::commit_investigate` is the S27 aftermath bridge from a resolved `ViolationMemory` record into local `AgentBeliefStore` social evidence. That is the exact place where crime-specific interpretation must be added if investigation is meant to upgrade a generic missing-entity mismatch into theft suspicion.
2. Live code already records only [`SocialObservationDetail::WitnessedAbsence`](../crates/worldwake-core/src/belief.rs) from investigate commit, then resolves the selected violation record in [`ViolationMemory`](../crates/worldwake-core/src/violation.rs). There is no current ownership-aware branch in the handler.
3. `ViolationKind::SuspectedTheft` is already live in [`crates/worldwake-core/src/violation.rs`](../crates/worldwake-core/src/violation.rs), and typed `SocialObservationDetail::SuspectedTheft { missing_entity, expected_place, suspect }` is already live in [`crates/worldwake-core/src/belief.rs`](../crates/worldwake-core/src/belief.rs). The prior dependency on `E17CRITHEJUS-015` is stale for this ticket's implementation scope.
4. `RuntimeBeliefView::believed_owner_of()` is already available, and `PerAgentBeliefView::from_world(actor, txn)` can be constructed inside systems at commit time. The correct ownership check is therefore belief-scoped, not a direct parallel read from authoritative ownership tables.
5. Existing focused coverage in [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs) already proves generic investigate behavior: commit records `WitnessedAbsence`, same-place violations stay distinct, and pre-recorded `SuspectedTheft` does not re-enter the generic investigate affordance path. This ticket should extend those focused tests rather than creating a new file.
6. Existing golden coverage in [`crates/worldwake-ai/tests/golden_emergent.rs`](../crates/worldwake-ai/tests/golden_emergent.rs) currently proves the baseline `EntityMissing -> InvestigateViolation -> WitnessedAbsence` pipeline, not owner-specific theft interpretation. That gap is real but does not block this ticket's focused-layer implementation.
7. Mismatch: the older ticket text still talks as if typed theft evidence does not exist yet. That is no longer true. Corrected scope is narrower and cleaner: add the missing owner-aware branch in investigate commit, store typed theft evidence there, and leave relay/accusation consumption to later tickets.
8. Information-path note: after this change there are still two lawful local aftermath artifacts from the same investigation for owners, not two competing transport paths. `WitnessedAbsence` remains the generic observation artifact; `SuspectedTheft` becomes the canonical crime interpretation artifact derived locally at the investigate boundary. No duplicate path is removed in scope.
9. N/A — no heuristic removal or ranking arithmetic change.
10. N/A — no stale-request or contested-affordance ordering claim.
11. Adjacent contradiction check: if future tickets need suspect promotion from `None` to `Some(thief)`, that remains separate evidence-ingestion work. This ticket should not invent a new alias path or retroactive mutation shortcut to cover that downstream gap.
12. Current architecture assessment: the proposed change is better than the status quo because it keeps theft inference at the only clean local causal boundary that already knows "I investigated my own missing thing here." Deferring that interpretation to accusation or AI candidate generation would force later systems to reconstruct a past local inference from weaker artifacts.

## Architecture Check

1. The cleanest design is a targeted extension of the existing investigate commit, because that is where the system already turns a pending violation into persistent local aftermath. Adding ownership-aware interpretation there preserves locality, keeps the evidence concrete, and avoids teaching later systems to infer theft from an under-specified `WitnessedAbsence` artifact.
2. Recording both `WitnessedAbsence` and `SuspectedTheft` is architecturally better than replacing absence with theft. The world still needs the generic anomaly artifact, while the crime pipeline needs the owner-specific interpretation. Those are distinct facts, not aliases.
3. The ownership check should use the actor's runtime belief view, not direct authoritative ownership lookup. That keeps the inference on the same belief-scoped side of the boundary as the originating violation and avoids an architectural leak from world truth into local investigation interpretation.
4. No backwards-compatibility shim, tuple overloading, or alias path should be added. If later consumers need stronger evidence, they should consume the typed theft detail directly.

## Verification Layers

1. Investigate commit emits generic absence aftermath -> focused runtime test in [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs)
2. Owner-only crime interpretation is added at the same commit boundary -> focused runtime test in [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs)
3. `ViolationMemory` keeps the original `EntityMissing` record resolved and also gains a new unresolved `SuspectedTheft` record with retention -> focused runtime test inspecting authoritative component state
4. Theft evidence stores `suspect: None` at investigation time -> focused runtime test inspecting `AgentBeliefStore` detail and recorded violation payload
5. Generic investigate affordance remains closed to `ViolationKind::SuspectedTheft` -> existing focused tests in [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs)

## What to Change

### 1. Extend investigate commit handler in `investigate_actions.rs`

After the existing `WitnessedAbsence` observation recording, evaluate whether the investigated missing entity is believed to be owned by the investigating actor. If so, record additive theft suspicion artifacts:

```rust
let belief = PerAgentBeliefView::from_world(instance.actor, txn);
if belief.believed_owner_of(subject) == Some(instance.actor) {
    store.record_social_observation(SocialObservation {
        detail: SocialObservationDetail::SuspectedTheft {
            missing_entity: subject,
            expected_place: place,
            suspect: None,
        },
        place,
        observed_tick: txn.tick(),
        source: PerceptionSource::DirectObservation,
    });

    let suspected_violation = ViolationKind::SuspectedTheft {
        missing_entity: subject,
        expected_place: place,
        suspect: None,
    };
    memory.record(
        suspected_violation,
        txn.tick(),
        profile.violation_memory_retention_ticks,
    );
}
```

Exact control flow must match the live handler:
- preserve the current `WitnessedAbsence` write
- preserve resolution of the selected original violation id
- add a new `SuspectedTheft` record rather than mutating the existing `EntityMissing` record in place
- use the same retention policy surface already used by investigate resolution

## Files to Touch

- `crates/worldwake-systems/src/investigate_actions.rs` (modify existing handler and focused tests)

## Out of Scope

- Steal action (E17CRITHEJUS-006)
- Suspect identification via witness Tell or possession observation
- Updating `SuspectedTheft.suspect` from `None` to `Some(thief)` when evidence arrives (requires follow-up crime-evidence relay/consumption work; see `E17CRITHEJUS-016`)
- Accuse/Fine/Exile actions (E17CRITHEJUS-008/009)
- AI candidate generation consuming `SuspectedTheft` (E17CRITHEJUS-010/011)
- Modifying `ViolationKind` or `SocialObservationDetail` shape in core (already landed)
- Replacing `WitnessedAbsence` with theft-specific aftermath; this ticket is additive, not substitutive

## Acceptance Criteria

### Tests That Must Pass

1. Owner investigating their missing item: `ViolationMemory` contains `SuspectedTheft { missing_entity, expected_place, suspect: None }`
2. Owner investigating: `AgentBeliefStore` contains typed theft evidence observation
3. Non-owner investigating same missing item: `ViolationMemory` does NOT contain `SuspectedTheft`
4. Non-owner investigating: still records `WitnessedAbsence` as before
5. Owner investigating: BOTH `WitnessedAbsence` and `SuspectedTheft` are recorded (SuspectedTheft is additive)
6. Owner investigating: the original `EntityMissing` violation record still resolves normally; theft suspicion is an added record, not an in-place kind swap
7. `SuspectedTheft.suspect` is `None` (thief identity not yet known at investigation time)
8. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Existing `WitnessedAbsence` behavior completely preserved (additive change only)
2. `SuspectedTheft` only recorded when `believed_owner_of(missing) == Some(investigator)` (P12 — uses belief, not world truth)
3. `suspect: None` at investigation time (P14 — ignorance is first-class)
4. The investigated `EntityMissing` record remains the resolved historical cause of the action; `SuspectedTheft` is a new local aftermath artifact
5. This ticket records concrete typed theft evidence locally; relay of that evidence through Tell is handled by later crime-evidence work

## Tests

### New/Modified Tests

1. [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs) — modify the existing generic investigate commit test to keep proving `WitnessedAbsence` plus original violation resolution, so the owner-aware branch cannot regress the baseline.
Rationale: this is the core non-crime contract the ticket must preserve.
2. [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs) — add a focused owner-path test that proves one investigate commit yields both `WitnessedAbsence` and typed `SuspectedTheft`, records `ViolationKind::SuspectedTheft { suspect: None }`, and still resolves the original `EntityMissing`.
Rationale: this is the exact new S27 -> E17 bridge behavior.
3. [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs) — add a focused non-owner-path test proving the same missing entity investigation does not synthesize theft suspicion.
Rationale: this protects the ownership gate and prevents crime false positives from generic absence handling.

### Commands

1. `cargo test -p worldwake-systems investigate_action_commits_witnessed_absence_and_extends_violation_memory`
2. `cargo test -p worldwake-systems owner_investigating_missing_owned_entity_records_suspected_theft`
3. `cargo test -p worldwake-systems non_owner_investigating_missing_entity_does_not_record_suspected_theft`
4. `cargo test -p worldwake-systems`
5. `cargo clippy -p worldwake-systems -- -D warnings`
6. `cargo test -p worldwake-ai golden_entity_missing_triggers_investigation`

## Outcome

- Completed: 2026-03-26
- What actually changed:
  - Extended [`crates/worldwake-systems/src/investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs) so investigate commit now checks ownership through `PerAgentBeliefView::from_world(actor, txn)` and, for owner-investigated missing entities, records additive typed `SocialObservationDetail::SuspectedTheft { suspect: None }`.
  - The same owner path now records a new unresolved `ViolationKind::SuspectedTheft` entry while still resolving the original `ViolationKind::EntityMissing` record.
  - Added focused owner and non-owner investigate tests in the existing investigate test module.
- Deviations from original plan:
  - No core enum/detail work was needed; typed theft detail was already live in `worldwake-core`, so the stale dependency on `E17CRITHEJUS-015` did not apply to implementation.
  - No new test file or AI-side implementation was required; the change remained a targeted investigate-handler extension plus focused coverage.
- Verification results:
  - `cargo test -p worldwake-systems investigate_action_commits_witnessed_absence_and_extends_violation_memory` ✅
  - `cargo test -p worldwake-systems owner_investigating_missing_owned_entity_records_suspected_theft` ✅
  - `cargo test -p worldwake-systems non_owner_investigating_missing_entity_does_not_record_suspected_theft` ✅
  - `cargo test -p worldwake-systems` ✅
  - `cargo clippy -p worldwake-systems -- -D warnings` ✅
  - `cargo test -p worldwake-ai golden_entity_missing_triggers_investigation` ✅
