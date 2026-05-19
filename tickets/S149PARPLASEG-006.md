# S149PARPLASEG-006: Information-barrier companion AskWitness synthesis

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — companion-intention synthesis for information barriers
**Deps**: S149PARPLASEG-004, S149PARPLASEG-005

## Problem

D7 makes ignorance an actionable plan outcome: when a plan terminal is `InformationBarrier { topic }`, the agenda manager spawns a companion `GoalKind::AskWitness` intention to acquire the missing fact. On commit, the agent's belief store updates and the suspended primary intention's `BeliefStatusChanged` resume condition fires.

## Assumption Reassessment (2026-05-20)

1. `GoalKind::AskWitness` is at `crates/worldwake-core/src/goal.rs:145` with payload `{ witness: EntityId, topic: TellTopic }` (NOT `{ topic, .. }` and NOT `InformationGapTopic`). The companion synthesis must supply a concrete `witness: EntityId` and pass the barrier's `TellTopic` through. `GoalKindDiscriminant::AskWitness` exists at goal.rs:213.
2. The companion intention is slot-typed `SlotKind::SocialMotive` (`crates/worldwake-core/src/slot_kind.rs`, S148). It is owned by the suspended primary intention; abandoning the primary cancels the companion.
3. Shared boundary under audit: the agenda-manager companion-spawn surface and the existing S139 testimony-acquisition path (`AskWitness` goal layer). Phase distinction: this is candidate/companion synthesis; the resume itself (the primary's `BeliefStatusChanged` firing) is owned by ticket 005's resume evaluation.
4. Live `GoalKind` under test: `AskWitness`. The witness is chosen from co-located or known agents the belief view exposes as plausible sources for the topic (belief-only, FND-14/FND-15). If no plausible witness is known, no companion is spawned (the primary stays suspended until another resume path or abandon fires).
5. AI regression layer: runtime `agent_tick`/agenda-manager; full action registries required for the E2E commit→belief-update→resume chain (covered by ticket 009 golden), but companion-spawn logic itself is unit-testable.

## Architecture Check

1. Reusing the S139 `AskWitness` substrate keeps information acquisition on the existing testimony path — no new sensing mechanism (FND-26, state-mediated). The companion is an ordinary lawful intention, not a privileged side channel.
2. Ownership (primary cancels companion) keeps the partial intention's revisability intact (FND-21) and avoids orphaned companion intentions.

## Verification Layers

1. `InformationBarrier` spawns a `SocialMotive`-slotted `AskWitness` companion with the barrier's `TellTopic` and a concrete witness → focused runtime test on the companion-spawn path.
2. Abandoning the primary cancels the companion → focused runtime test (primary abandoned → companion removed).
3. No-witness case → focused runtime test: no plausible witness known → no companion spawned, primary stays suspended.

## What to Change

### 1. Companion synthesis on `InformationBarrier`

In the agenda manager, when a suspended intention's terminal is `InformationBarrier { topic }`, select a witness from belief-view-known plausible sources and spawn a companion `GoalKind::AskWitness { witness, topic }` intention slot-typed `SlotKind::SocialMotive`, owned by the primary.

### 2. Ownership lifecycle

Wire the companion's cancellation to the primary's abandonment.

## Files to Touch

- `crates/worldwake-ai/src/agenda_manager.rs` (modify) — companion synthesis + ownership wiring
- `Likely: crates/worldwake-ai/src/candidate_generation.rs` (modify, if companion goals are emitted through the candidate path) — grep `AskWitness` emission sites to confirm

## Out of Scope

- The resume condition firing on commit (ticket 005's resume evaluation already handles `BeliefStatusChanged`).
- Coordination-barrier triggers (ticket 007).
- E2E information-barrier golden (ticket 009).

## Acceptance Criteria

### Tests That Must Pass

1. New: an `InformationBarrier { topic }` with a known plausible witness spawns a `SocialMotive`-slotted `AskWitness { witness, topic }` companion.
2. New: abandoning the primary cancels the companion.
3. New: no known plausible witness → no companion spawned, primary remains suspended.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Witness selection reads only the belief view — no authoritative roster query (FND-14/FND-15).
2. A companion never outlives its owning primary intention.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` (inline) — companion spawn / ownership / no-witness cases.

### Commands

1. `cargo test -p worldwake-ai`
2. `scripts/verify.sh`
