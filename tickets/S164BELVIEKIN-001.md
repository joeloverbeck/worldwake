# S164BELVIEKIN-001: Observed-kind belief carrier on LastSeenRecord

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `LastSeenRecord` carrier field (core), `LastSeenRecordDef` (cli), last-seen synthesis in the per-agent belief view (sim), save-format version bump (sim)
**Deps**: None

## Problem

The last-seen belief synthesis at `per_agent_belief_view.rs:1296` builds a
`BelievedEntityState` for a last-seen-only remote entity with
`believed_kind: self.world.entity_kind(*entity)` — a **live world read** for an
entity the actor has not co-located-observed this tick. Location and aliveness on
the same synthesized state are correctly frozen from the (stale-correct)
`LastSeenRecord`, but kind is pulled from authoritative truth. `LastSeenRecord`
(`expectation.rs:126-132`) stores `subject`, `place`, `observed_tick`, `source`,
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

1. `LastSeenRecord` (`crates/worldwake-core/src/expectation.rs:126-132`) derives
   `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize` and currently has no
   kind field. `EntityKind` (`crates/worldwake-core/src/entity.rs:7`) derives
   `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`,
   so `observed_kind: Option<EntityKind>` keeps the record `Copy` and serializable.
2. The synthesis lives at `crates/worldwake-sim/src/per_agent_belief_view.rs:1293-1304`
   inside `known_entity_beliefs`; only the last-seen-only branch leaks (the
   `known_entities` belief-store branch at `:1278` clones stored state, which already
   carries `believed_kind`, belief.rs:1735). Existing inline tests exercising this
   path: `effective_place_uses_last_seen_without_refreshing_remote_truth`
   (`per_agent_belief_view.rs:2948`) and
   `known_entity_beliefs_expose_only_actor_subjective_memory`
   (`per_agent_belief_view.rs:3030`) — both must be re-validated and updated to seed
   `observed_kind` rather than relying on the live read.
3. Cross-system boundary under audit: the belief/memory carrier contract
   (`LastSeenRecord` is the canonical last-seen memory; FND-7/FND-15). After this
   change, kind reaches a distant actor only via the record's `observed_kind`, never
   live world.
4. Mismatch + correction (serialization): the save format is **bincode**
   (`save_load.rs:87/140`), which is positional, so `#[serde(default)]` does not make
   pre-bump byte streams loadable. `SAVE_FORMAT_VERSION` is currently `99`
   (`save_load.rs:7`); adding a field to the serialized `LastSeenRecord` (carried in
   the `LastSeenMemory` component) is format-breaking and **requires a bump to 100**.
   Pre-bump saves are rejected by the version match at `save_load.rs:130` (FND-28 —
   no migration shim). `#[serde(default)]` is still applied to the `LastSeenRecordDef`
   field because RON scenario input is serde-self-describing and existing scenarios
   must keep deserializing without authoring the new field.
5. Construction sites for `LastSeenRecord` are all struct literals (no `..record`
   spread), so the compiler enforces population at every site. Runtime sites:
   `search_actions.rs:436`/`:474` (direct observation — read found entity's kind),
   the two testimony relays `report_actions.rs:784` and
   `ask_about_person_actions.rs:364` (propagate `observed_kind: record.observed_kind`
   so kind travels with the relayed memory through the hearsay chain, FND-15), and
   the scenario loader `scenario/mod.rs:1724` (map from the `Def`). Remaining literal
   sites are test fixtures (`per_agent_belief_view.rs:2560/2965`, `expectation.rs:294/409`,
   `save_load.rs:660` test helper, `delta.rs:579`, `candidate_generation.rs:8769`) and
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

## Verification Layers

1. Synthesis no longer reads live world → focused unit test on `known_entity_beliefs`:
   change authoritative `entity_kind` of a last-seen-only remote entity after the
   record is stored; assert the synthesized `believed_kind` equals the recorded
   `observed_kind`, not the new live kind.
2. Carrier travels through testimony → focused unit test on the relay path: relayed
   record retains the original `observed_kind` (`report_actions` / `ask_about_person`).
3. Save/load round-trip preserves the field → `save_load.rs` round-trip test asserts
   `restored_last_seen_memory.records[..].observed_kind` matches the saved value.
4. Single-layer-per-invariant: each invariant above maps to a distinct focused
   surface (synthesis unit test, relay unit test, save-load round-trip); no
   golden/E2E surface is needed here — the cross-system behavioral proof lands in
   ticket 005.

## What to Change

### 1. Add `observed_kind` to `LastSeenRecord`

In `crates/worldwake-core/src/expectation.rs`, add
`pub observed_kind: Option<EntityKind>` to `LastSeenRecord`. Update the doc comment
to note it is the kind observed at last-seen time (belief/memory carrier, may go
stale), not current world truth.

### 2. Add the matching `LastSeenRecordDef` field

In `crates/worldwake-cli/src/scenario/types.rs` (`LastSeenRecordDef`, line 402), add
`#[serde(default)] pub observed_kind: Option<EntityKind>`. `EntityKind` is a plain
enum (not an `EntityId` reference), so it deserializes directly — no `*Def` wrapper.
Map the field through in the scenario loader at `scenario/mod.rs:1724`.

### 3. Populate at runtime construction sites

- `search_actions.rs:436`/`:474`: set `observed_kind` from the found entity's kind at
  observation time (the actor is co-located/observing it).
- `report_actions.rs:784` and `ask_about_person_actions.rs:364`
  (`relay_last_seen_record`): propagate `observed_kind: record.observed_kind` so the
  kind travels with the relayed memory through the hearsay chain.
- Update all remaining literal construction sites (test fixtures, save-load helper,
  delta) to supply `observed_kind`; the compiler enforces this.

### 4. Switch the synthesis to read the carrier

In `per_agent_belief_view.rs:1296`, replace
`believed_kind: self.world.entity_kind(*entity)` with
`believed_kind: record.observed_kind`.

### 5. Bump the save format

In `crates/worldwake-sim/src/save_load.rs`, bump `SAVE_FORMAT_VERSION` 99 → 100.

## Files to Touch

- `crates/worldwake-core/src/expectation.rs` (modify — field + doc + test fixtures)
- `crates/worldwake-core/src/delta.rs` (modify — test fixture construction)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — `LastSeenRecordDef` field)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — loader mapping at `:1724`)
- `crates/worldwake-systems/src/search_actions.rs` (modify — `:436`/`:474`)
- `crates/worldwake-systems/src/report_actions.rs` (modify — relay at `:784`)
- `crates/worldwake-systems/src/ask_about_person_actions.rs` (modify — relay at `:364`)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — synthesis `:1296` + tests)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump + helper + round-trip test)

## Out of Scope

- The `entity_kind` accessor source-gate (ticket 002) — this ticket only fixes the
  `known_entity_beliefs` synthesis and lands the carrier field.
- The remote-kind-change adversarial golden (ticket 005).
- Any change to authoritative `world.entity_kind` semantics.

## Acceptance Criteria

### Tests That Must Pass

1. Synthesis reads the carrier: a last-seen-only remote entity whose authoritative
   kind changes after the record is stored keeps its recorded `observed_kind` in the
   synthesized `believed_kind`.
2. Relay propagation: a relayed last-seen record retains the original `observed_kind`.
3. Save/load round-trip preserves `observed_kind`.
4. Existing suite: `cargo test -p worldwake-sim` (including updated `:2948`/`:3030`).

### Invariants

1. The last-seen synthesis never reads `self.world.entity_kind` for a last-seen-only
   entity.
2. `observed_kind` is belief/memory carrier state — never promoted to authoritative
   world truth; authoritative `entity_kind` is unchanged.
3. Two live authoritative representations of the same fact do not coexist (FND-28):
   pre-bump saves are rejected, not migrated.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — new focused test that the
   synthesis reads `observed_kind`, not live world; update `:2948` and `:3030` to seed
   `observed_kind`.
2. `crates/worldwake-systems/src/report_actions.rs` (and/or
   `ask_about_person_actions.rs`) — relay-propagation test for `observed_kind`.
3. `crates/worldwake-sim/src/save_load.rs` — extend the round-trip test at `:1647` to
   assert `observed_kind`.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-core -p worldwake-systems -p worldwake-cli`
3. `./scripts/verify.sh`

Merge note: Ticket 001 bumps SAVE_FORMAT_VERSION 99→100; sibling tickets 002–005 add no serialized-state fields and deliberately avoid a second bump.
