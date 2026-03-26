# E17CRITHEJUS-003: Extend institutional claims with Accusation, Verdict, CrimeRegister

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — core institutional type extensions plus cross-crate institutional-knowledge consumers
**Deps**: E17CRITHEJUS-001, E17CRITHEJUS-002, E17CRITHEJUS-017

## Problem

No institutional record type exists for crimes. The accusation and punishment system requires `RecordKind::CrimeRegister` plus crime-specific `InstitutionalClaim` variants that can survive record consultation, institutional Tell transport, belief storage, ranking, and trace formatting without introducing duplicate information paths or placeholder-only compile fixes.

## Assumption Reassessment (2026-03-26)

1. `RecordKind` in [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs) still contains only `OfficeRegister`, `FactionRoster`, and `SupportLedger`. `InstitutionalClaim` still contains only `OfficeHolder`, `FactionMembership`, `SupportDeclaration`, and `ForceControl`.
2. `ViolationId` already exists in [crates/worldwake-core/src/violation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/violation.rs), and `PunishmentKind` already exists in [crates/worldwake-core/src/crime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/crime.rs). The original ticket dependency text was stale.
3. This is not a core-only enum-extension ticket. `InstitutionalClaim` now travels through first-class institutional Tell topics after E17CRITHEJUS-017, so every claim variant must map cleanly across the shared abstraction boundary: `InstitutionalClaim` -> `InstitutionalBeliefKey` / Tell-topic lane -> record consultation -> relay ordering -> trace formatting. Relevant live consumers include [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs), [crates/worldwake-systems/src/consult_record_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/consult_record_actions.rs), [crates/worldwake-systems/src/tell_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs), [crates/worldwake-sim/src/institutional_knowledge_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/institutional_knowledge_trace.rs), [crates/worldwake-sim/src/social_relay.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/social_relay.rs), [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs), and [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).
4. The exact shared data contract under audit is the institutional-record knowledge path for crime cases: `RecordData.entries[*].claim` -> `institutional_belief_key(claim)` -> `AgentBeliefStore.institutional_beliefs` / Tell topic memory lanes -> AI/runtime consumers reading known institutional beliefs. A compile-only enum extension would leave this path architecturally incomplete.
5. The live institutional-belief system already assumes every `InstitutionalClaim` belongs to a deterministic memory lane. `institutional_tell_topic_key()`, `institutional_claim_same_memory_lane()`, `current_institutional_belief_topics()`, and multiple `institutional_belief_key()` helpers are exhaustive today. New crime claims therefore need real lane semantics, not placeholder `_ =>` fallbacks.
6. The clean lane model is case-oriented: accusation and verdict knowledge should be keyed by the concrete case identity `{ accused, violation_id }`, so consultation and Tell treat “this theft case” as the memory lane while still preserving append-only record entries inside `RecordData`.
7. The original ticket proposed `InstitutionalClaim::Verdict { ..., supersedes_accusation: RecordEntryId }`. That duplicates supersession facts already stored canonically in `InstitutionalRecordEntry.supersedes` and creates two lawful transport paths for the same relation. Per the repo’s information-path rules and P3/P16/P24, the canonical supersession path must remain the record entry wrapper, not a duplicate field inside the claim payload.
8. Because of item 7, the verdict payload should carry only crime-domain facts: `accused`, `violation_id`, `punishment`, and `effective_tick`. The “which accusation did this resolve?” fact should continue to live solely in `InstitutionalRecordEntry.supersedes` and be asserted through record append/supersede tests.
9. Focused live test inventory exists and was verified. `cargo test -p worldwake-core institutional -- --list` currently exposes the institutional unit tests in [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs), and `cargo test -p worldwake-core violation -- --list` confirms `ViolationId`/`SuspectedTheft` already ship from core.
10. Existing downstream behavior does not yet consume crime claims semantically. `worldwake-ai/src/institutional_queries.rs` and office-specific readers are intentionally office-register-specific. This ticket should not invent early crime-planning logic, but it must leave the shared institutional knowledge substrate able to store, relay, and inspect crime claims cleanly for later tickets.
11. Corrected mismatch: the original “update downstream match arms” wording understated the work. The needed downstream changes are not placeholders; they are concrete lane mapping, equality/ranking/formatting updates, and trace summaries that keep institutional knowledge coherent once crime claims exist.
12. Adjacent contradiction surfaced during reassessment: the active E17 spec section still duplicates accusation supersession inside the verdict payload. That spec-family discrepancy should be reconciled in planning material, but this ticket can still implement the cleaner canonical path in code and record the deviation explicitly.

## Architecture Check

1. Reusing `RecordData` for `CrimeRegister` is the right architecture. Crime records are institutional memory, not a new storage primitive.
2. Extending `InstitutionalClaim` is better than inventing a parallel crime-record payload enum because accusations and verdicts need to travel through the same consultation/Tell/belief infrastructure as other institutional artifacts.
3. A concrete crime-case memory lane is better than generic placeholder match arms. Without a deterministic lane, consultation and Tell dedup become undefined for crime claims, and later accusation/punishment AI would have to retrofit meaning onto an unstable substrate.
4. Keeping accusation-resolution identity only in `InstitutionalRecordEntry.supersedes` is cleaner than duplicating `supersedes_accusation` inside `InstitutionalClaim::Verdict`. One fact should have one canonical transport path.
5. No backwards-compatibility aliasing. Extend the existing institutional architecture directly and update all exhaustive consumers to the new canonical shape.

## Verification Layers

1. Core type surface -> focused `institutional.rs` serde and append/supersede tests prove the new record kind and claims serialize and preserve append-only semantics.
2. Institutional memory-lane semantics -> focused `belief.rs` tests prove accusation/verdict claims dedup and compare by case lane/content rather than falling through placeholder logic.
3. Record consultation projection -> focused `consult_record_actions.rs` and/or `institutional_knowledge_trace.rs` coverage proves crime claims can be projected into agent institutional belief memory with deterministic keys.
4. Tell ordering / conversational transport stability -> focused `social_relay.rs`, `tell_actions.rs`, or `ranking.rs` tests prove new crime claims do not break institutional-topic ordering or memory-lane equality.
5. Cross-workspace exhaustiveness -> `cargo build --workspace` proves all claim/record consumers compile on the new canonical surface.

## What to Change

### 1. Add `CrimeRegister` to `RecordKind`

Extend `RecordKind` in [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs) with:

```rust
CrimeRegister,
```

Update any exhaustive ordering/formatting/tests that assume the old closed set.

### 2. Add `Accusation` and `Verdict` to `InstitutionalClaim`

Extend `InstitutionalClaim` in [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs) with:

```rust
Accusation {
    accuser: EntityId,
    accused: EntityId,
    violation_id: ViolationId,
    effective_tick: Tick,
},
Verdict {
    accused: EntityId,
    violation_id: ViolationId,
    punishment: PunishmentKind,
    effective_tick: Tick,
},
```

Do not duplicate the supersession link inside `Verdict`; use `InstitutionalRecordEntry.supersedes` as the single canonical link from verdict entry to accusation entry.

### 3. Extend institutional belief-key and Tell-lane semantics for crime cases

Update the institutional knowledge substrate so accusation and verdict claims map to a deterministic crime-case lane keyed by `{ accused, violation_id }`. This includes the helpers in:

- [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs)
- [crates/worldwake-systems/src/consult_record_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/consult_record_actions.rs)
- [crates/worldwake-systems/src/tell_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs)
- [crates/worldwake-sim/src/institutional_knowledge_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/institutional_knowledge_trace.rs)

The outcome should be deterministic storage, relay, and trace behavior for crime claims even before later tickets add accusation/punishment AI.

### 4. Update downstream ordering, formatting, and exhaustive consumers

Add the real new variants to downstream exhaustive matches, especially:

- [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
- [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
- [crates/worldwake-sim/src/social_relay.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/social_relay.rs)

These updates should preserve deterministic ordering and legible trace output. Do not add `_` catchalls.

## Files to Touch

- [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs)
- [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs)
- [crates/worldwake-systems/src/consult_record_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/consult_record_actions.rs)
- [crates/worldwake-systems/src/tell_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs)
- [crates/worldwake-sim/src/institutional_knowledge_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/institutional_knowledge_trace.rs)
- [crates/worldwake-sim/src/social_relay.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/social_relay.rs)
- [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
- [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
- Focused test modules in the same files as needed

## Out of Scope

- Creating `CrimeRegister` entities in world setup or golden harnesses
- Accuse/Fine/Exile action handlers
- AI candidate generation or planner semantics for accusation/punishment goals
- Spec-wide planning-document cleanup outside this ticket file

## Acceptance Criteria

### Tests That Must Pass

1. `InstitutionalClaim::Accusation` serde round-trip preserves all fields.
2. `InstitutionalClaim::Verdict` serde round-trip preserves `accused`, `violation_id`, both `PunishmentKind` variants, and `effective_tick`.
3. `RecordData` of kind `CrimeRegister` can be constructed and appended to.
4. A verdict entry can supersede an accusation entry using `RecordData::supersede_entry()`, and the supersession link lives only in `InstitutionalRecordEntry.supersedes`.
5. Institutional belief/Tell lane helpers treat accusation and verdict claims for the same `{ accused, violation_id }` case deterministically.
6. Existing suite: `cargo test -p worldwake-core institutional`
7. Existing suite: `cargo test -p worldwake-sim institutional_knowledge_trace`
8. Existing suite: `cargo test -p worldwake-systems consult_record`
9. Existing suite: `cargo test -p worldwake-systems tell`
10. Existing suite: `cargo test -p worldwake-ai ranking`
11. Existing suite: `cargo build --workspace`
12. Existing suite: `cargo clippy --workspace`

### Invariants

1. `InstitutionalClaim` remains `Copy + Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Serialize + Deserialize`.
2. `RecordKind` remains deterministic and serializable.
3. Append-only record semantics are preserved; accusation resolution is modeled by entry supersession, not in-place mutation.
4. Crime-case identity has a single canonical information path in belief/Tell memory lanes: `{ accused, violation_id }`.
5. Existing office/faction/support record behavior remains unaffected.

## Test Plan

### New/Modified Tests

1. [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs) — add serde round-trip coverage for `Accusation` and `Verdict`, plus append/supersede assertions for `CrimeRegister`. Rationale: prove the new record/claim surface and canonical supersession path at the owning layer.
2. [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) — add institutional memory-lane equality tests for accusation/verdict case identity. Rationale: crime claims must enter the shared Tell/belief substrate with deterministic lane semantics.
3. [crates/worldwake-sim/src/institutional_knowledge_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/institutional_knowledge_trace.rs) and/or [crates/worldwake-systems/src/consult_record_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/consult_record_actions.rs) — add focused projection coverage for consulted crime-register entries. Rationale: record consultation is the authoritative bridge from record history into agent institutional belief memory.
4. [crates/worldwake-sim/src/social_relay.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/social_relay.rs) or [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) — update deterministic claim-order tests if needed. Rationale: new claim variants must not destabilize institutional Tell prioritization.

### Commands

1. `cargo test -p worldwake-core institutional`
2. `cargo test -p worldwake-core belief`
3. `cargo test -p worldwake-sim institutional_knowledge_trace`
4. `cargo test -p worldwake-systems consult_record`
5. `cargo test -p worldwake-systems tell`
6. `cargo test -p worldwake-ai ranking`
7. `cargo build --workspace`
8. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-26
- What actually changed:
  - Added `RecordKind::CrimeRegister`, `InstitutionalClaim::Accusation`, and `InstitutionalClaim::Verdict`.
  - Extended institutional memory-lane and Tell/consultation helpers so accusation and verdict knowledge share a deterministic crime-case lane keyed by `{ accused, violation_id }`.
  - Updated downstream institutional-claim ordering and trace formatting in sim/AI consumers so the new variants integrate cleanly with the existing institutional-knowledge substrate.
  - Added focused tests in core and sim covering serde, append/supersede semantics, crime-case memory-lane equality, and consulted crime-case trace summaries.
- Deviations from original plan:
  - The implemented verdict payload does not include `supersedes_accusation`. That relationship already exists canonically on `InstitutionalRecordEntry.supersedes`, and duplicating it inside the claim would create a second lawful transport path for the same fact.
  - The ticket scope expanded from “enum extensions plus compile fixes” to include the real institutional-belief/Tell lane work required by the current post-E17CRITHEJUS-017 architecture.
- Verification:
  - `cargo test -p worldwake-core institutional`
  - `cargo test -p worldwake-core belief`
  - `cargo test -p worldwake-sim institutional_knowledge_trace`
  - `cargo test -p worldwake-systems consult_record`
  - `cargo test -p worldwake-systems tell`
  - `cargo test -p worldwake-ai ranking`
  - `cargo build --workspace`
  - `cargo clippy --workspace`
