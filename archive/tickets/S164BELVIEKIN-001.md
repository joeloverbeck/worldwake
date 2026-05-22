# S164BELVIEKIN-001: Observed-kind belief carrier on LastSeenRecord

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `LastSeenRecord` carrier field (core), `LastSeenRecordDef` (cli), last-seen synthesis in the per-agent belief view (sim), save-format version bump (sim)
**Deps**: None

## Problem

Before this ticket, the last-seen belief synthesis in `per_agent_belief_view.rs` built a
`BelievedEntityState` for a last-seen-only remote entity with
`believed_kind: self.world.entity_kind(*entity)` — a **live world read** for an
entity the actor has not co-located-observed this tick. Location and aliveness on
the same synthesized state are correctly frozen from the (stale-correct)
`LastSeenRecord`, but kind is pulled from authoritative truth. `LastSeenRecord`
(`expectation.rs`) stores `subject`, `place`, `observed_tick`, `source`,
`provenance` — it has no observed kind, so the synthesis has nothing belief-local
to read and reaches for live world. This violates FND-7/FND-14/FND-15: a remote
entity that changes kind with no carrier would have its new kind "known" with no
perception, testimony, record, or memory path.

This ticket adds the belief/memory carrier — an `observed_kind` recorded at
observation time and propagated through testimony — and switches the synthesis to
read it. It is the foundation for ticket 002 (`entity_kind` accessor source-gate),
whose remote branch reads this field for last-seen-only entities.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `LastSeenRecord` (`crates/worldwake-core/src/expectation.rs`) derives
   `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize` and currently has no
   kind field. `EntityKind` (`crates/worldwake-core/src/entity.rs`) derives
   `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`,
   so `observed_kind: Option<EntityKind>` keeps the record `Copy` and serializable.
2. The synthesis lives in `crates/worldwake-sim/src/per_agent_belief_view.rs`
   inside `known_entity_beliefs`; only the last-seen-only branch leaks (the
   `known_entities` belief-store branch clones stored state, which already
   carries `believed_kind` from `belief.rs`). Existing inline tests exercising this
   path: `effective_place_uses_last_seen_without_refreshing_remote_truth`
   (`per_agent_belief_view.rs`) and
   `known_entity_beliefs_expose_only_actor_subjective_memory`
   (`per_agent_belief_view.rs`) — both were re-validated and updated to seed
   `observed_kind` rather than relying on the live read.
3. Cross-system boundary under audit: the belief/memory carrier contract
   (`LastSeenRecord` is the canonical last-seen memory; FND-7/FND-15). After this
   change, kind reaches a distant actor only via the record's `observed_kind`, never
   live world.
4. Mismatch + correction (serialization): the save format is **bincode**
   (`save_load.rs`), which is positional, so `#[serde(default)]` does not make
   pre-bump byte streams loadable. `SAVE_FORMAT_VERSION` was `99` before this ticket
   (`save_load.rs`); adding a field to the serialized `LastSeenRecord` (carried in
   the `LastSeenMemory` component) is format-breaking and required a bump to 100.
   Pre-bump saves are rejected by the version match in `save_load.rs` (FND-28 —
   no migration shim). `#[serde(default)]` is still applied to the `LastSeenRecordDef`
   field because RON scenario input is serde-self-describing and existing scenarios
   must keep deserializing without authoring the new field.
5. Construction sites for `LastSeenRecord` are all struct literals (no `..record`
   spread), so the compiler enforces population at every site. Runtime sites:
   `search_actions.rs` (direct observation — read found entity's kind),
   the two testimony relays in `report_actions.rs` and
   `ask_about_person_actions.rs` (propagate `observed_kind: record.observed_kind`
   so kind travels with the relayed memory through the hearsay chain, FND-15), and
   the scenario loader `scenario/mod.rs` (map from the `Def`). Remaining literal
   sites are test fixtures in `per_agent_belief_view.rs`, `expectation.rs`,
   `save_load.rs`, `delta.rs`, and `candidate_generation.rs`, and
   are updated mechanically. Runtime save/load round-trips the field automatically
   via the bincode derive once the version is bumped.

## Architecture Check

1. The carrier mechanism preserves kind-at-observation (FND-15 fidelity) rather than
   discarding it — the cleaner of the two options the spec considered. The kind is
   recorded once at observation and travels with the record exactly like `place` and
   `observed_tick`, so no live read is ever needed for a remote entity.
2. No backward-compatibility shim: the save-format version is bumped and pre-bump
   saves are rejected (FND-28). `#[serde(default)]` on the `Def` is forward-compat for
   serde-self-describing scenario input, not a live-authority compatibility layer.

## Verified Layers

1. Synthesis no longer reads live world → focused unit test on `known_entity_beliefs`:
   change authoritative `entity_kind` of a last-seen-only remote entity after the
   record is stored; assert the synthesized `believed_kind` equals the recorded
   `observed_kind`, not the changed live kind.
2. Carrier travels through testimony → focused unit test on the relay path: relayed
   record retains the original `observed_kind` (`report_actions` / `ask_about_person`).
3. Save/load round-trip preserves the field → `save_load.rs` round-trip test asserts
   `restored_last_seen_memory.records[..].observed_kind` matches the saved value.
4. Single-layer-per-invariant: each invariant above maps to a distinct focused
   surface (synthesis unit test, relay unit test, save-load round-trip); no
   golden/E2E surface is needed here — the cross-system behavioral proof lands in
   ticket 005.

## Landed Changes

### 1. Added `observed_kind` to `LastSeenRecord`

In `crates/worldwake-core/src/expectation.rs`, `LastSeenRecord` now carries
`pub observed_kind: Option<EntityKind>`. The doc comment records this as
last-seen belief/memory carrier state that may go stale, not current world truth.

### 2. Added the matching `LastSeenRecordDef` field

In `crates/worldwake-cli/src/scenario/types.rs`, `LastSeenRecordDef` now carries
`#[serde(default)] pub observed_kind: Option<EntityKind>`, and the scenario loader
maps it into runtime `LastSeenRecord` construction. `EntityKind` remains a plain enum
(not an `EntityId` reference), so no `*Def` wrapper was needed.

### 3. Populated runtime construction sites

- `search_actions.rs`: direct search observations set `observed_kind` from the found
  entity's kind at observation time.
- `report_actions.rs` and `ask_about_person_actions.rs`
  (`relay_last_seen_record`) propagate `observed_kind: record.observed_kind` so the
  kind travels with the relayed memory through the hearsay chain.
- Remaining literal construction sites (test fixtures, save-load helper, delta, AI
  test helper, CLI fixture) now supply `observed_kind`.

### 4. Switched synthesis to the carrier

In `per_agent_belief_view.rs`, the last-seen-only branch now synthesizes
`BelievedEntityState.believed_kind` from `record.observed_kind`, not
`self.world.entity_kind(*entity)`.

### 5. Bumped the save format

In `crates/worldwake-sim/src/save_load.rs`, `SAVE_FORMAT_VERSION` is now 100.

## Landed Files

- `crates/worldwake-core/src/expectation.rs` (modify — field + doc + test fixtures)
- `crates/worldwake-core/src/delta.rs` (modify — test fixture construction)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — `LastSeenRecordDef` field)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — loader mapping)
- `crates/worldwake-systems/src/search_actions.rs` (modify — direct observation records)
- `crates/worldwake-systems/src/report_actions.rs` (modify — relay)
- `crates/worldwake-systems/src/ask_about_person_actions.rs` (modify — relay)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — synthesis + tests)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump + helper + round-trip test)

## Out of Scope

- The `entity_kind` accessor source-gate (ticket 002) — this ticket only fixes the
  `known_entity_beliefs` synthesis and lands the carrier field.
- The remote-kind-change adversarial golden (ticket 005).
- Any change to authoritative `world.entity_kind` semantics.

## Acceptance Result

### Verified Acceptance

1. Synthesis reads the carrier: `known_entity_beliefs_synthesize_last_seen_kind_from_record_not_live_world`
   proves a last-seen-only remote entity keeps its recorded `observed_kind` in the
   synthesized `believed_kind`.
2. Relay propagation: report and ask-about-person relay tests prove relayed last-seen
   records retain the original `observed_kind`.
3. Save/load round-trip preserves `observed_kind` in
   `save_to_bytes_roundtrip_preserves_full_nondefault_state`.
4. Existing affected suites passed: `cargo test -p worldwake-sim`,
   `cargo test -p worldwake-core -p worldwake-systems -p worldwake-cli`, and
   `cargo test -p worldwake-ai`.

### Invariants

1. The last-seen synthesis never reads `self.world.entity_kind` for a last-seen-only
   entity.
2. `observed_kind` is belief/memory carrier state — never promoted to authoritative
   world truth; authoritative `entity_kind` is unchanged.
3. Two live authoritative representations of the same fact do not coexist (FND-28):
   pre-bump saves are rejected, not migrated.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — added a focused test proving
   last-seen synthesis reads `observed_kind`, not live world; updated existing fixtures
   to seed `observed_kind`.
2. `crates/worldwake-systems/src/report_actions.rs` and
   `crates/worldwake-systems/src/ask_about_person_actions.rs` — existing relay tests
   now assert `observed_kind` propagation through hearsay records.
3. `crates/worldwake-sim/src/save_load.rs` — extended the full nondefault state
   round-trip assertion to include `observed_kind`.

### Commands Run

1. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view`.
2. Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`.
3. Passed `cargo test -p worldwake-systems --lib report_actions::tests::report_found_commit_relays_last_seen_and_resolves_listener_state -- --exact`.
4. Passed `cargo test -p worldwake-systems --lib ask_about_person_actions::tests::ask_about_person_commit_transfers_last_seen_with_hearsay_provenance -- --exact`.
5. Passed `cargo test -p worldwake-core -p worldwake-systems -p worldwake-cli`.
6. Passed `cargo test -p worldwake-sim`.
7. Passed `cargo test -p worldwake-ai`.
8. Passed `./scripts/verify.sh`.

Merge note: Ticket 001 bumps SAVE_FORMAT_VERSION 99→100; sibling tickets 002–005 add no serialized-state fields and deliberately avoid a second bump.

## Outcome

Completed on 2026-05-22.

- Added `observed_kind: Option<EntityKind>` as the belief/memory carrier on
  `LastSeenRecord` and scenario `LastSeenRecordDef`.
- Populated direct observation records from the observed entity kind, propagated the
  carrier through report and ask-about-person hearsay relays, and updated all explicit
  construction sites.
- Switched last-seen-only `known_entity_beliefs` synthesis from a live
  `world.entity_kind` read to `record.observed_kind`.
- Bumped the current bincode save format from 99 to 100; older versions remain
  rejected by the existing version gate.
- Updated CLI/AI/test fixtures to seed the new carrier where the intended stale kind
  is known.

## Deviations

- The CLI action-menu fixture also needed `observed_kind: Some(EntityKind::Agent)` to
  keep its existing last-seen POV label expectation truthful. This was shared-shape
  fixture fallout from the carrier addition, not new CLI behavior.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`.
- Passed `cargo test -p worldwake-systems --lib report_actions::tests::report_found_commit_relays_last_seen_and_resolves_listener_state -- --exact`.
- Passed `cargo test -p worldwake-systems --lib ask_about_person_actions::tests::ask_about_person_commit_transfers_last_seen_with_hearsay_provenance -- --exact`.
- Passed `cargo test -p worldwake-core -p worldwake-systems -p worldwake-cli`.
- Passed `cargo test -p worldwake-sim`.
- Passed `cargo test -p worldwake-ai`.
- Passed `./scripts/verify.sh`.
