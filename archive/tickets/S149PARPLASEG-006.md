# S149PARPLASEG-006: Information-barrier companion AskWitness synthesis

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — companion-intention synthesis for information barriers; agenda-origin save shape
**Deps**: archive/tickets/S149PARPLASEG-004.md, archive/tickets/S149PARPLASEG-005.md, archive/tickets/S149PARPLASEG-010.md

## Problem

D7 makes ignorance an actionable plan outcome: when a plan terminal is `InformationBarrier { topic }`, the agenda manager spawns a companion `GoalKind::AskWitness` intention to acquire the missing fact. On commit, the agent's belief store updates and the suspended primary intention's `BeliefStatusChanged` resume condition fires.

## Assumption Reassessment (2026-05-20)

1. `GoalKind::AskWitness` is at `crates/worldwake-core/src/goal.rs:145` with payload `{ witness: EntityId, topic: TellTopic }` (NOT `{ topic, .. }` and NOT `InformationGapTopic`). The companion synthesis must supply a concrete `witness: EntityId` and pass the barrier's `TellTopic` through. `GoalKindDiscriminant::AskWitness` exists at goal.rs:213.
2. The companion intention is slot-typed `SlotKind::SocialMotive` (`crates/worldwake-core/src/slot_kind.rs`, S148). It is owned by the suspended primary intention; abandoning the primary cancels the companion.
3. Shared boundary under audit: the agenda-manager companion-spawn surface and the existing S139 testimony-acquisition path (`AskWitness` goal layer). Phase distinction: this is candidate/companion synthesis; the resume-condition evaluation is owned by ticket 005, while executable segment writing/re-entry is owned by ticket 010.
4. Live `GoalKind` under test: `AskWitness`. The witness is chosen from co-located or known agents the belief view exposes as plausible sources for the topic (belief-only, FND-14/FND-15). If no plausible witness is known, no companion is spawned (the primary stays suspended until another resume path or abandon fires).
5. AI regression layer: runtime `agent_tick`/agenda-manager; full action registries required for the E2E commit→belief-update→resume chain (covered by ticket 009 golden), but companion-spawn logic itself is unit-testable.
6. Live `AgendaEntry` has no separate slot or owner field. The narrowest durable representation is a serialized `AgendaOrigin::Companion { primary, slot }` variant, with `slot: SlotKind::SocialMotive`; because agenda state is nested under `AgentDecisionRuntime`, this changes the current save payload and requires a `SAVE_FORMAT_VERSION` bump with rejection of version 92 at the existing save-header boundary.

## Architecture Check

1. Reusing the S139 `AskWitness` substrate keeps information acquisition on the existing testimony path — no new sensing mechanism (FND-26, state-mediated). The companion is an ordinary lawful intention, not a privileged side channel.
2. Ownership (primary cancels companion) keeps the partial intention's revisability intact (FND-21) and avoids orphaned companion intentions.

## Verified Layers

1. `InformationBarrier` spawns a `SocialMotive`-slotted `AskWitness` companion with the barrier's `TellTopic` and a concrete witness → focused runtime test on the companion-spawn path.
2. Abandoning the primary cancels the companion → focused runtime test (primary abandoned → companion removed).
3. No-witness case → focused runtime test: no plausible witness known → no companion spawned, primary stays suspended.
4. Agenda-origin save-shape break → focused save-version tests prove current format 93 and version 92 rejection.

## Landed Changes

### 1. Companion synthesis on `InformationBarrier`

In the agenda manager, when a suspended intention's terminal is `InformationBarrier { topic }`, the code selects a witness from belief-view-known plausible sources and spawns a companion `GoalKind::AskWitness { witness, topic }` intention. The companion records slot and owner as `AgendaOrigin::Companion { primary, slot: SlotKind::SocialMotive }`.

### 2. Ownership lifecycle

The companion's cancellation is wired to primary abandonment and primary kill handling.

## Landed Files

- `crates/worldwake-ai/src/agenda_manager.rs` (modify) — companion synthesis + ownership wiring
- `crates/worldwake-ai/src/agenda_types.rs` (modify) — companion agenda origin
- `crates/worldwake-ai/src/lib.rs` (modify) — export the companion synthesis helper
- `crates/worldwake-sim/src/save_load.rs` (modify) — current save-format version bump/rejection proof
- `specs/S149-partial-plan-segments-and-typed-terminals.md` (modify) — save-version and companion-origin truth-sync

## Out of Scope

- The resume condition evaluation on commit (ticket 005 handles `BeliefStatusChanged`).
- Executable segment writing and tactical re-entry (ticket 010).
- Coordination-barrier triggers (ticket 007).
- E2E information-barrier golden (ticket 009).

## Acceptance Result

### Focused Tests

1. Added: an `InformationBarrier { topic }` with a known plausible witness spawns a `SocialMotive`-slotted `AskWitness { witness, topic }` companion.
2. Added: `AgendaOrigin::Companion { primary, slot: SlotKind::SocialMotive }` round-trips through agenda-state serialization.
3. Added: abandoning the primary cancels the companion.
4. Added: no known plausible witness -> no companion spawned, primary remains suspended.
5. Passed: `cargo test -p worldwake-ai`

### Invariants

1. Witness selection reads only the belief view — no authoritative roster query (FND-14/FND-15).
2. A companion never outlives its owning primary intention.

## Outcome

Completion date: 2026-05-20.

S149 D7 is landed for the agenda-manager companion-spawn layer. Information barriers now synthesize ordinary `AskWitness` companion intentions through the belief-view surface, owned by the suspended primary partial intention. The serialized agenda-origin shape is explicit and covered by the current save-format bump from version 92 to 93.

## Deviations

The ticket's reassessed scope remained narrow. The E2E commit -> belief update -> resume chain stays with S149PARPLASEG-009, and executable segment re-entry stays with the already archived S149PARPLASEG-010.

## Verification Result

1. Passed: `cargo fmt --all`
2. Passed: `cargo test -p worldwake-ai --lib agenda_manager::tests::information_barrier_spawns_social_motive_ask_witness_companion -- --exact`
3. Passed: `cargo test -p worldwake-ai --lib agenda_manager::tests::information_barrier`
4. Passed: `cargo test -p worldwake-ai --lib agenda_manager::tests::abandoning_information_barrier_primary_cancels_companion -- --exact`
5. Passed: `cargo test -p worldwake-ai --lib agenda_types::tests::lifecycle_enums_roundtrip_through_bincode -- --exact`
6. Passed: `cargo test -p worldwake-sim --lib save_load::tests::save_format_version_is_93_after_s149_companion_origin_landing -- --exact`
7. Passed: `cargo test -p worldwake-sim --lib save_load::tests::load_rejects_pre_s149_companion_origin_version_92_without_migration_shim -- --exact`
8. Passed: `cargo test -p worldwake-ai`
9. Passed: `cargo test -p worldwake-sim`
10. Passed: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
11. Passed: `./scripts/verify.sh`

## Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` (inline) — companion spawn / ownership / no-witness cases.
2. `crates/worldwake-ai/src/agenda_types.rs` (inline) — companion origin bincode roundtrip.
3. `crates/worldwake-sim/src/save_load.rs` (inline) — current version 93 and version 92 rejection.
