# S110DECHISEVE-002: EventTag variants, DecisionEventPayload types, and EventPayload decision_payload field

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `EventTag` (worldwake-core) extended with 11 variants; new `DecisionEventPayload` module in core; `EventPayload` gains `decision_payload: Option<DecisionEventPayload>` field; `SAVE_FORMAT_VERSION` bump
**Deps**: archive/tickets/S110DECHISEVE-001.md (`MaterializationTag` must live in `worldwake-core` before `ExpectationMismatchPayload` can reference it)

## Problem

S110's causal spine is a set of new typed events that record every agent decision (commit / reject / adopt / invalidate / mismatch / repair / replan / blocker-record). Per FND-29A, the event log is the authoritative append-only record of causal history, so these events must live on the main log — not in the optional `DecisionTraceSink`. This ticket introduces the schema: 11 new unit `EventTag` variants, the `DecisionEventPayload` sum enum with all component payload structs, and the `decision_payload: Option<DecisionEventPayload>` field on `EventPayload`. Emission wiring lands in ticket 004; this ticket lands the types and the format bump so emitters have a target to build against.

## Assumption Reassessment (2026-04-20)

1. `EventTag` lives at `crates/worldwake-core/src/event_tag.rs:7`, derives `Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash + Serialize + Deserialize`, and currently carries 26 unit variants. Tests `event_tag_includes_all_required_variants`, `event_tag_order_is_declaration_stable`, and `event_tag_bincode_roundtrip_covers_every_variant` (in the same file's `#[cfg(test)]` block) verify variant count, declaration-order stability, and bincode round-trip. Adding 11 unit variants preserves all derives; the tests must be updated to cover the new variants (count check, `ALL_EVENT_TAGS` array, round-trip loop).
2. `EventPayload` is defined at `crates/worldwake-core/src/event_record.rs:46` with fields `tick, cause, actor_id, action_name, target_ids, evidence, place_id, state_deltas, observed_entities, visibility, witness_data, tags`. Adding `decision_payload: Option<DecisionEventPayload>` preserves existing derives (`Clone, Debug, Eq, PartialEq, Serialize, Deserialize`). Workspace-wide grep found 57 direct `EventPayload { … }` construction sites; each needs `decision_payload: None` added (or the field populated in the three ticket-004 emission paths).
3. Shared abstraction boundary under audit: the wire-format shape of `EventPayload`. After this ticket, the shape is serialized differently (one additional field), so `SAVE_FORMAT_VERSION` must bump from 33 to 34 (`crates/worldwake-sim/src/save_load.rs:6`) per FND-28 — old saves are not decodable. This is the spec's Non-Goal (no backwards-compatible decoding).
5. Not a planner-driven ticket — type introduction only, no goal-family or affordance surface touched in this scope.
15. `decision_history_alternatives` on `CognitiveProfile` (ticket 003) is not referenced by this ticket's types; the cap is a runtime truncation concern, not a payload-shape concern. The `GoalCommittedPayload::rejected_alternatives: Vec<RejectedAlternativeSummary>` field has no compile-time bound — enforcement happens at emission time in ticket 004.

## Architecture Check

1. Payload-on-field (`decision_payload: Option<DecisionEventPayload>`) rather than payload-inside-variant preserves `EventTag`'s classifier role. `EventTag` stays `Copy + Ord + Hash` so index maps (`BTreeMap<EventTag, Vec<EventId>>` on `EventLog`) continue to work without change. Events that carry no decision payload (action lifecycle, world mutation, trade) keep `Option::None` and pay only one byte of tag overhead per record.
2. No backwards-compat shim — `SAVE_FORMAT_VERSION` bumps atomically and old saves fail to load with `SaveError::VersionMismatch`. No migration layer, no dual-decode path. FND-28 applies.
3. All new payload types live in `worldwake-core` so they transitively reference only core types (`EntityId`, `GoalKey`, `BlockerKey`, `BlockingFact`, `Discrepancy`, `BeliefClaimKey`, `MaterializationTag`, `SuspensionReason`, `ActionDefId`, `Tick`). Crate-layering invariant preserved; no ai-internal types leak into the event schema.

## Verification Layers

1. Type and derive correctness → compile-time check; workspace must build.
2. Bincode round-trip invariance for `EventPayload` with `decision_payload: Some(…)` across every `DecisionEventPayload` variant → unit tests in `event_record.rs` and `decision_event_payload.rs` `#[cfg(test)]` blocks.
3. `EventTag` enum-stability contract (declaration order, `ALL_EVENT_TAGS` length, round-trip) → existing tests in `event_tag.rs` extended to include the new variants.
4. `SAVE_FORMAT_VERSION` bump correctness → existing `save_load.rs` tests (`save_load_roundtrip`, version-mismatch tests) must continue to pass with the new version constant; no dual-version decode path exists.
6. Single-layer ticket (type/schema only) — no decision-trace, action-trace, or belief-view mapping applies until emission lands in ticket 004.

## What to Change

### 1. Extend `EventTag` with 11 new unit variants

In `crates/worldwake-core/src/event_tag.rs`, append the following variants (order matters — appended to preserve `Ord` stability of existing variants):

```rust
pub enum EventTag {
    // ... existing 26 variants unchanged ...
    GoalOffered,
    GoalSuppressed,
    GoalCommitted,
    GoalSuspended,
    GoalAbandoned,
    PlanAdopted,
    PlanInvalidated,
    ExpectationMismatch,
    RepairApplied,
    ReplanTriggered,
    BlockerRecorded,
}
```

Update the `ALL_EVENT_TAGS` constant to length 38 (was 27 — current file shows 26 variants + 1 index offset; verify exact length at implementation time), add the 11 new variants to the array in declaration order, and update `event_tag_includes_all_required_variants` (the `assert_eq!(ALL_EVENT_TAGS.len(), 27)` becomes the new count). The round-trip test iterates `ALL_EVENT_TAGS` and covers the new variants automatically once they're in the array.

### 2. Define `DecisionEventPayload` and all component payload types

Create `crates/worldwake-core/src/decision_event_payload.rs`. All types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. None require `Copy` (they are held by value in `EventPayload::decision_payload`, not in any hot-path Copy position).

Sum enum:

```rust
pub enum DecisionEventPayload {
    GoalOffered(GoalOfferedPayload),
    GoalSuppressed(GoalSuppressedPayload),
    GoalCommitted(GoalCommittedPayload),
    GoalSuspended(GoalSuspendedPayload),
    GoalAbandoned(GoalAbandonedPayload),
    PlanAdopted(PlanAdoptedPayload),
    PlanInvalidated(PlanInvalidatedPayload),
    ExpectationMismatch(ExpectationMismatchPayload),
    RepairApplied(RepairAppliedPayload),
    ReplanTriggered(ReplanTriggeredPayload),
    BlockerRecorded(BlockerRecordedPayload),
}
```

Component structs and helper enums as specified in S110 D2. Each payload struct carries the fields the spec defines. The core-crate types referenced — `EntityId`, `GoalKey`, `BlockerKey`, `BlockingFact`, `Discrepancy`, `BeliefClaimKey`, `MaterializationTag` (delivered by `archive/tickets/S110DECHISEVE-001.md`), `SuspensionReason`, `ActionDefId`, `Tick` — all live in core after that dependency lands.

Helper enums (`EmitterTag`, `EvidenceKindTag`, `GoalRejectionReason`, `PlanInvalidationReason`, `RepairKind`, `ReplanReason`) are all `Copy + Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Hash + Serialize + Deserialize`. `EvidenceSummary` holds `BTreeMap<EvidenceKindTag, u16>` and derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` only. `RejectedAlternativeSummary` derives the same set.

Register the module in `crates/worldwake-core/src/lib.rs` and re-export `DecisionEventPayload` and every component payload/enum type alongside existing core re-exports.

### 3. Add `decision_payload` field to `EventPayload`

In `crates/worldwake-core/src/event_record.rs`, modify `EventPayload`:

```rust
pub struct EventPayload {
    pub tick: Tick,
    pub cause: CauseRef,
    pub actor_id: Option<EntityId>,
    pub action_name: Option<String>,
    pub target_ids: Vec<EntityId>,
    pub evidence: Vec<EvidenceRef>,
    pub place_id: Option<EntityId>,
    pub state_deltas: Vec<StateDelta>,
    pub observed_entities: BTreeMap<EntityId, ObservedEntitySnapshot>,
    pub visibility: VisibilitySpec,
    pub witness_data: WitnessData,
    pub tags: BTreeSet<EventTag>,
    pub decision_payload: Option<DecisionEventPayload>,
}
```

The field is appended to keep existing field order stable. Serde wire-format will include it as the trailing field. Add `import` of `DecisionEventPayload` at the top of `event_record.rs`.

### 4. Update every `EventPayload` construction site

Workspace grep identifies 57 direct `EventPayload { … }` construction sites across `crates/worldwake-core`, `crates/worldwake-sim`, `crates/worldwake-systems`, and `crates/worldwake-ai`. Each must add `decision_payload: None` as the trailing field. This is mechanical. No existing emission site is a decision event — every non-decision emitter passes `None`.

The three emitter paths that will later populate this field (candidate_generation, ranking, agent_tick/planning — covered in ticket 004) continue to pass `None` in this ticket and are rewired in 004.

### 5. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, bump `pub const SAVE_FORMAT_VERSION: u32 = 33;` to `34`. Existing save/load tests that assert the version constant or exercise version-mismatch paths (e.g., `version_mismatch_returns_error`) continue to work — they reference the constant symbolically.

### 6. Unit tests for round-trip invariance

In `crates/worldwake-core/src/event_record.rs` `#[cfg(test)]` block, add a test that constructs an `EventPayload` with `decision_payload: Some(DecisionEventPayload::GoalCommitted(GoalCommittedPayload { … }))` populated with representative data, bincode-serializes, deserializes, and asserts equality.

In `crates/worldwake-core/src/decision_event_payload.rs` `#[cfg(test)]` block, add a per-variant round-trip test that instantiates each `DecisionEventPayload` variant (11 total) with representative field values and confirms bincode round-trip.

## Files to Touch

- `crates/worldwake-core/src/event_tag.rs` (modify — 11 new variants, update `ALL_EVENT_TAGS` and count assertion)
- `crates/worldwake-core/src/decision_event_payload.rs` (new — sum enum and all component types)
- `crates/worldwake-core/src/event_record.rs` (modify — add `decision_payload` field, import, and round-trip test)
- `crates/worldwake-core/src/lib.rs` (modify — module declaration and re-exports)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` bump)
- All 57 `EventPayload { … }` construction sites across the workspace (modify — append `decision_payload: None`). Non-exhaustive list: `crates/worldwake-sim/src/tick_step.rs`, `crates/worldwake-sim/src/action_execution.rs`, `crates/worldwake-sim/src/action_termination.rs`, `crates/worldwake-systems/src/*_actions.rs`, `crates/worldwake-systems/src/combat.rs`, `crates/worldwake-systems/src/evidence_decay.rs`, `crates/worldwake-systems/src/item_decay.rs`, `crates/worldwake-core/src/event_record.rs` (tests), and all in-tree test-setup helpers. Implementer must grep-verify the full list at implementation time.

## Out of Scope

- Emission of any new event variants at runtime. This ticket ships the schema only; all 57 existing construction sites continue to produce the same events as before with `decision_payload: None`. Ticket 004 wires emission.
- `decision_history_alternatives` on `CognitiveProfile`. Ticket 003 adds that field; the truncation it governs is applied at emission time in ticket 004.
- Observer rendering of new events. Ticket 006 adds the "Decision History" section.
- Replay-invariance test. Ticket 005 adds the explicit decision-event replay check.
- Save-state migration from version 33 to 34. FND-28 applies — old saves fail to load with `SaveError::VersionMismatch`.

## Acceptance Criteria

### Tests That Must Pass

1. `event_tag_satisfies_required_traits` still compiles — new variants preserve `Copy + Ord + Hash + Serialize + Deserialize`.
2. `event_tag_includes_all_required_variants` asserts the new total count (existing 26 + 11 new = 37; implementer verifies exact current count at implementation time and updates both the array and the `assert_eq!(ALL_EVENT_TAGS.len(), N)`).
3. `event_tag_order_is_declaration_stable` continues to pass.
4. `event_tag_bincode_roundtrip_covers_every_variant` continues to pass with the new variants in `ALL_EVENT_TAGS`.
5. New test `decision_event_payload_variants_roundtrip_through_bincode` — each of the 11 `DecisionEventPayload` variants round-trips.
6. New test `event_payload_with_decision_payload_roundtrips_through_bincode` — `EventPayload` with `decision_payload: Some(…)` round-trips.
7. Existing `pending_event_roundtrips_through_bincode_with_ordered_deltas` and all `event_record.rs` round-trip tests continue to pass (with the trailing `decision_payload: None` added to the test fixtures).
8. `cargo test --workspace` passes.
9. `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. `EventTag` remains `Copy + Ord + Hash` — new variants are unit-shaped.
2. Exactly one `DecisionEventPayload` definition exists (in `worldwake-core`); no duplicate or parallel decision-payload type in sim/systems/ai.
3. Every `EventPayload` construction site populates `decision_payload` explicitly (no field-init-shorthand can omit it because `EventPayload` has no `Default` impl).
4. `SAVE_FORMAT_VERSION` is exactly 34 after this ticket; old version-33 saves are rejected with `SaveError::VersionMismatch`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs` (`#[cfg(test)]`) — `decision_event_payload_variants_roundtrip_through_bincode` exercises all 11 variants.
2. `crates/worldwake-core/src/event_record.rs` (`#[cfg(test)]`) — `event_payload_with_decision_payload_roundtrips_through_bincode` with at least `GoalCommitted` payload populated.
3. `crates/worldwake-core/src/event_tag.rs` — update existing `ALL_EVENT_TAGS` constant, length assertion, and variant list.
4. Update every existing `EventPayload` round-trip and construction test to pass `decision_payload: None` as the trailing field (in-place modification, not new tests).

### Commands

1. `cargo test -p worldwake-core` — fastest feedback loop on the type changes.
2. `cargo test -p worldwake-sim save_load` — confirms the version bump is coherent with save/load tests.
3. `cargo test --workspace` — confirms no consumer construction site is missed.
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-04-20
- Added [decision_event_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/decision_event_payload.rs) with the `DecisionEventPayload` sum enum plus all S110 component payload structs and helper enums in `worldwake-core`, then exported the schema from [lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/lib.rs).
- Extended [event_tag.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/event_tag.rs) with the 11 decision-history `EventTag` variants and updated the stable variant list/count tests to cover them.
- Extended [event_record.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/event_record.rs) so `EventPayload` now carries `decision_payload: Option<DecisionEventPayload>`, and exposed that field through `EventView::decision_payload()` so the shared event-view abstraction can actually read the new schema.
- Updated every direct `EventPayload { ... }` construction site found in the workspace to set `decision_payload: None`, preserving current runtime behavior until the emission wiring lands in ticket 004.
- Bumped [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) `SAVE_FORMAT_VERSION` from `33` to `34` so the serialized event shape change is explicit and old saves fail fast with version mismatch.
- Landed the required bincode round-trip coverage for the new decision payload schema and for `EventPayload` carrying a decision payload.
- Deviation from the original ticket wording: `PlanInvalidationReason` could not honestly derive `Hash` on the live branch because the nested core types it carries (`BeliefClaimKey`, `Discrepancy`, `GoalKey`) do not currently implement `Hash`. The enum still derives `Copy + Clone + Eq + Ord + Serialize + Deserialize`, and no current consumer requires hashing.

## Verification Result

- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-sim save_load`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
