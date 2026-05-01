# S129PLADIRFAC-002: Hygiene event tags and payloads

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new variants on `EventTag` and `DecisionEventPayload`
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md (D4)

## Problem

S129's causal chain ("wilderness relief → place dirtier → bad sleep there → travel decision") needs explicit event-log records so the chain is queryable per FND-29A and FND-30. Today the relevant decision-event surface (`crates/worldwake-core/src/event_tag.rs` + `decision_event_payload.rs`) carries no hygiene-domain tags. This ticket adds the two tags the spec mandates (`WasteCreated`, `WashFacilityUsed`) plus their payload structs so handlers in tickets 005/006/007 can emit them. `LatrineMaintained` is **not** added — the spec defers it under FND-28 until a `clean_latrine` action lands.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `EventTag` enum at `crates/worldwake-core/src/event_tag.rs` listed 41 current variants at intake. This ticket expanded it to 43 variants with `WasteCreated` and `WashFacilityUsed`. The enum derives `Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize`; new variants remain unit-style to satisfy the existing `Copy` bound.
2. `DecisionEventPayload` enum at `crates/worldwake-core/src/decision_event_payload.rs` carried 14 variants at intake. This ticket expanded it to 16 variants with `WasteCreated(WasteCreatedPayload)` and `WashFacilityUsed(WashFacilityUsedPayload)`. Payload struct precedent: `SleepEpisodeStartedPayload` and `SleepEpisodeEndedPayload` use named structs with `pub` fields including `EntityId`, `Tick`, and `Permille`. The new payload structs follow that precedent.
3. The shared abstraction boundary under audit is the `EventTag` ↔ `DecisionEventPayload` pairing — every routable event tag has at most one payload variant on the payload enum. New variants must be added to both surfaces in lockstep.
4. The deferred `LatrineMaintained` variant is intentionally not added (per spec Non-Goals + FND-28 row): no `clean_latrine` action exists today, so the variant would have no emitter. Adding a dead variant would violate FND-28.
5. Existing focused/unit coverage: grep `event_tag.rs`'s inline tests (if any) for the canonical variant list (at line 61 of the spec's earlier grep, an array of all variants is asserted). That array must be extended with the two new variants to satisfy the existing exhaustiveness test.

## Architecture Check

1. Adding new event tags as enum variants (rather than free-form string identifiers) preserves the typed-event-log contract — consumers like the observer binary, decision-trace machinery, and any future replay tool match on the variant exhaustively. A string-keyed event stream would defeat FND-29's debuggability mandate.
2. Payload structs as standalone named types (rather than tuple variants on `DecisionEventPayload`) keeps consumer code readable when destructuring — every field is named. Mirrors the `SleepEpisodeStartedPayload` precedent. No backward-compat shim: today there is no waste/wash payload; net-new addition.

## Verification Layers

1. New variants are reachable from `match` exhaustiveness sites → `cargo build` failure surface alone is the proof; if any cross-crate match site does not handle the new variants, the build breaks. Inventory those sites during implementation (grep `match.*EventTag` workspace-wide).
2. Save/load round-trips the new variants → save_load focused test seeding an event log with `WasteCreated` and `WashFacilityUsed` records carrying their payloads, then save → load → assert equal.

## What to Change

### 1. `crates/worldwake-core/src/event_tag.rs`

Add two new variants to the `EventTag` enum after the most recent existing entry (preserve the existing variant ordering convention):

```rust
WasteCreated,
WashFacilityUsed,
```

If the file's `#[cfg(test)]` block has an exhaustiveness array listing every variant (the spec reassessment grep noted line 61 has such a test), extend the array with the two new variants.

### 2. `crates/worldwake-core/src/decision_event_payload.rs`

Add two new variants to the `DecisionEventPayload` enum, each carrying a named-struct payload:

```rust
WasteCreated(WasteCreatedPayload),
WashFacilityUsed(WashFacilityUsedPayload),
```

Then declare the payload structs after the existing `SleepEpisodeStartedPayload` block:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WasteCreatedPayload {
    pub creator: EntityId,
    pub place: EntityId,
    pub waste_lot: EntityId,
    pub source: WasteSource,
    pub place_dirtiness_delta: Permille,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WasteSource {
    WildernessRelief,
    OvercapacityLatrine,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WashFacilityUsedPayload {
    pub user: EntityId,
    pub basin: EntityId,
    pub water_consumed: u16,
    pub agent_dirtiness_delta: Permille,
    pub basin_dirtiness_delta: Permille,
    pub partial: bool,
}
```

### 3. Exhaustiveness sites across the workspace

If any cross-crate `match` block exhaustively destructures `EventTag` or `DecisionEventPayload`, add the missing arms (most likely returning `()` or a no-op pass-through, depending on the consumer). Inventory during implementation; common candidates include the observer binary at `crates/worldwake-cli/src/bin/observer.rs` and decision-trace consumers in `crates/worldwake-ai/src/decision_trace.rs`.

### 4. `SAVE_FORMAT_VERSION` impact

The event log is part of the persisted save state, so this ticket bumped `SAVE_FORMAT_VERSION` from 56 to 57.

## Files to Touch

- `crates/worldwake-core/src/event_tag.rs` (modify)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify)
- Likely: `crates/worldwake-cli/src/bin/observer.rs` (modify — if observer matches `EventTag` exhaustively; grep `match.*EventTag` to confirm)
- Likely: `crates/worldwake-ai/src/decision_trace.rs` (modify — if decision-trace consumers match `DecisionEventPayload` exhaustively)
- Likely: `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` if event log is persisted; confirm during reassessment)

## Out of Scope

- Emitting the new event tags from any handler (deferred to tickets 005/006/007).
- `LatrineMaintained` variant — explicitly deferred per spec Non-Goals and FND-28 row.
- New observer feature design beyond rendering the added payload fields. The live exhaustive consumer sweep required updating the existing observer decision-history renderer and fixture coverage for the two new variants.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test `event_tag_includes_waste_created_and_wash_facility_used` in `event_tag.rs`'s test module — asserts the two new variants exist (compile-time check via match coverage).
2. New focused test in `decision_event_payload.rs`'s test module — round-trips `DecisionEventPayload::WasteCreated(WasteCreatedPayload { ... })` and `WashFacilityUsed(WashFacilityUsedPayload { ... })` through bincode serialize → deserialize and asserts equality.
3. Existing event-tag exhaustiveness test (if present at `event_tag.rs` test block line 61) extended to include the two new variants.
4. Existing suite: `cargo test -p worldwake-core` and any consumer crate whose match sites were updated.

### Invariants

1. `EventTag::WasteCreated` and `EventTag::WashFacilityUsed` exist as unit variants and are `Copy`-compatible with the rest of the enum.
2. `DecisionEventPayload::WasteCreated` carries exactly `WasteCreatedPayload`; `WashFacilityUsed` carries exactly `WashFacilityUsedPayload` — no other variant payloads collide with these names.
3. `LatrineMaintained` is **not** present in either enum — verified by grep returning zero matches across `crates/`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_tag.rs` — extend the inline test block with the two new variants.
2. `crates/worldwake-core/src/decision_event_payload.rs` — new round-trip test per payload struct.

### Commands

1. `cargo test -p worldwake-core event_tag`
2. `cargo test -p worldwake-core decision_event_payload`
3. `cargo build --workspace` (catches missing exhaustiveness arms)
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-30.

- Added `EventTag::WasteCreated` and `EventTag::WashFacilityUsed`.
- Added typed `DecisionEventPayload` variants and payload structs for waste creation and wash-facility use.
- Re-exported the new payload types from `worldwake-core`.
- Updated observer decision-history rendering and coverage for both hygiene payloads.
- Bumped `SAVE_FORMAT_VERSION` from 56 to 57 and extended save/load decision-event roundtrip coverage.
- Corrected `specs/IMPLEMENTATION-ORDER.md` so S129 no longer claims `LatrineMaintained` lands before a `clean_latrine` action exists.

## Deviations

- The live exhaustive consumer sweep found observer rendering and save/load fixtures as current-ticket fallout; both were updated in scope.
- `LatrineMaintained` remains deferred and absent from `crates/`.

## Verification Result

Passed:

1. `cargo test -p worldwake-core --lib event_tag::tests::event_tag_includes_all_required_variants -- --exact`
2. `cargo test -p worldwake-core --lib decision_event_payload::tests::hygiene_decision_payloads_roundtrip_through_bincode -- --exact`
3. `cargo test -p worldwake-core --lib event_tag`
4. `cargo test -p worldwake-core --lib decision_event_payload`
5. `cargo build --workspace`
6. `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_decision_event_payloads -- --exact`
7. `cargo test -p worldwake-core`
8. `cargo test -p worldwake-cli --bin observer tests::render_decision_history_section_covers_all_variants -- --exact`
9. `cargo test -p worldwake-sim --lib save_load`
10. `cargo test --workspace`
11. `cargo clippy --workspace --all-targets -- -D warnings`
12. `git diff --check`
13. `rg -n LatrineMaintained crates` returned no matches.

`cargo test --workspace` initially exposed owned fallout in the observer row-count fixture and save-format assertion; both were corrected and the command was rerun successfully.
