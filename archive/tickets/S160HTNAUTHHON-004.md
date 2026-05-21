# S160HTNAUTHHON-004: Remove escort u32::MAX sentinel

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — escort action payload (`worldwake-sim`), escort affordance/runtime (`worldwake-systems`), planner payload-override (`worldwake-ai`), save format (`worldwake-sim`)
**Deps**: None

## Problem

`EscortToSafetyActionPayload.intended_heal_action` is an `ActionDefId` built with the
`ActionDefId(u32::MAX)` sentinel ("resolved at runtime" placeholder) at two
construction sites, then overwritten at action start with the real heal action id.
A sentinel `ActionDefId` that can leak into a plan, trace, or dispatch is a
fossil-seed risk (FND-28: no dead abstractions or placeholders in live authority
paths). This ticket replaces the sentinel with an honest `Option<ActionDefId>`
(`None` until resolved at action start), which is the truthful representation of the
existing resolve-at-start flow.

## Assumption Reassessment (2026-05-21)

1. `worldwake-sim/src/action_payload.rs:393` defines `EscortToSafetyActionPayload`
   with `pub intended_heal_action: ActionDefId` (line 396); it derives `Clone, Debug,
   Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. It is a variant of
   `ActionPayload` (line 34).
2. The `u32::MAX` sentinel is built at **two** construction sites:
   `worldwake-ai/src/goal_model.rs:962` (planner `build_payload_override`) and
   `worldwake-systems/src/escort_actions.rs:210` (affordance enumeration via
   `build_escort_payload`, lines 165–177). The runtime resolution overwrites it at
   `escort_actions.rs:401` (`payload.intended_heal_action = heal_action_id(context.action_defs)?`)
   and the value is read at `escort_actions.rs:600` (`enqueue_for_contention`).
   `heal_action_id` (`escort_actions.rs:179`) requires the `ActionDefRegistry`.
3. Option B (resolve the real id at construction) is **rejected**: neither
   construction site has the `ActionDefRegistry` in scope — `enumerate_escort_payloads`
   (`escort_actions.rs:191`) receives only `_def: &ActionDef`, and the planner
   `build_payload_override` (`goal_model.rs:950`) has no registry. The registry is
   only available at action start, exactly where resolution already happens. So
   option A (`Option<ActionDefId>`, `None` until resolved) is the correct shape.
4. **Serialization (load-bearing)**: `ActionPayload` lives in `ActionInstance`,
   stored in the scheduler's `active_actions: BTreeMap<ActionInstanceId,
   ActionInstance>` (`scheduler.rs:87`), which is part of serialized
   `SimulationState`. Changing `intended_heal_action` from `ActionDefId` to
   `Option<ActionDefId>` changes the serialized representation, so
   `SAVE_FORMAT_VERSION` (currently 96, `save_load.rs:7`) must bump to 97, and the
   version-assert tests at `save_load.rs:1369` and `save_load.rs:1380` must update.
5. Mixed-layer ticket — shared boundary under audit: the
   `EscortToSafetyActionPayload` shape consumed by the planner payload-override (ai),
   the affordance enumerator + start handler + contention enqueue (systems), and the
   serialized save format (sim). The first failure boundary if mishandled is
   compile-time (the field type change is atomic across all sites).
6. Existing test sample: `action_payload.rs:686` `sample_escort_to_safety_payload`
   constructs `intended_heal_action: ActionDefId(27)` (line 696) and the trait
   assertion at line 811 — update to `Some(ActionDefId(27))`. Escort coverage in
   `worldwake-systems` tests (`escort_actions.rs` test module) and any escort golden
   exercise the resolution path.

## Architecture Check

1. `Option<ActionDefId>` (`None` until resolved) is the honest representation of the
   existing resolve-at-start flow: there is no real heal id at planning/enumeration
   time, so the payload should say so rather than carry a sentinel that can leak.
   This removes the FND-28 fossil-seed without inventing a registry-threading path
   that the construction sites cannot support.
2. No backward-compatibility shim: the field type changes outright and all
   construction/resolution/read sites are migrated in one ticket (the change is
   atomic — intermediate states would not compile). The save format bumps rather
   than carrying a compatibility decoder (FND-28: compatibility only at boundaries,
   normalized into the current model).

## Verification Result

1. Passed `cargo test -p worldwake-systems escort` — six escort tests ran. The
   affordance enumeration test asserts pre-resolution `intended_heal_action: None`;
   the commit/handoff test asserts action start resolves it to `Some(heal_id)` and
   the care contention queue receives that real action id.
2. Passed `cargo test -p worldwake-sim -p worldwake-ai` — sim save/load, action
   payload bincode, and AI planner/golden package tests passed against the migrated
   payload shape. This first run also exposed an unused `ActionDefId` import after
   sentinel removal.
3. Passed `cargo test -p worldwake-ai` after the import cleanup — refreshed the AI
   package proof for the final planner override source state.
4. Passed `./scripts/verify.sh` — fmt check, workspace tests, cleanup scripts,
   workspace clippy, all-target clippy with `-D warnings`, and scenario coverage
   all exited cleanly.

## Landed Changes

### 1. Changed the field type (sim)

`action_payload.rs` now stores
`EscortToSafetyActionPayload.intended_heal_action: Option<ActionDefId>`. The action
payload sample uses `Some(ActionDefId(27))`, preserving the bincode/trait proof for
the resolved shape.

### 2. Construction sites set `None` (ai + systems)

- `goal_model.rs` planner payload override now emits `intended_heal_action: None`.
- `escort_actions.rs::build_escort_payload` no longer accepts a heal action id and
  affordance enumeration now emits `None`.

### 3. Runtime resolution and read (systems)

`start_escort_to_safety` resolves the real heal action id and stores
`Some(heal_action_id(...))`. The care contention handoff unwraps the resolved id and
records an internal error if that stage is reached while the value is still `None`.

### 4. Bumped the save format (sim)

`SAVE_FORMAT_VERSION` is now 97, with version assertion tests updated to the S160
escort sentinel removal.

### 5. Added sentinel-absence and resolution assertions

`escort_affordance_enumerates_non_adjacent_reachable_destination` asserts
pre-resolution `None`; `escort_to_safety_commit_moves_both_entities_and_queues_care_handoff`
asserts action start resolves `Some(heal_id)` before the final care queue handoff.

## Files Touched

- `crates/worldwake-sim/src/action_payload.rs` (modify — field type + test sample)
- `crates/worldwake-ai/src/goal_model.rs` (modify — planner payload-override → None)
- `crates/worldwake-systems/src/escort_actions.rs` (modify — enumeration, resolution, read)
- `crates/worldwake-sim/src/save_load.rs` (modify — SAVE_FORMAT_VERSION 96→97 + version-assert tests)

## Out of Scope

- Any change to escort preconditions, route planning, or contention semantics beyond
  the `Option` handling.
- The HTN authority labeling / trace / rename — tickets 001–003.
- Adding a `Tag → Source` or compatibility decoder for old save streams — the format
  bumps; no shim (FND-28).

## Acceptance Criteria

### Tests Passed

1. Passed `cargo test -p worldwake-systems escort`.
2. Passed `cargo test -p worldwake-sim -p worldwake-ai`.
3. Passed `cargo test -p worldwake-ai` after final import cleanup.
4. Passed `./scripts/verify.sh`.

### Invariants

1. No plan, action trace, or dispatch ever observes a placeholder/sentinel
   `ActionDefId` for `intended_heal_action`.
2. `intended_heal_action` is `Some` by the time it is read at contention enqueue;
   a `None` at that point is an internal error, never a silent skip.
3. Two live authoritative representations of the same fact do not coexist (FND-28):
   the sentinel path is removed, not aliased.

## Test Evidence

### Test Surfaces Updated

1. `crates/worldwake-systems/src/escort_actions.rs` (test module) — sentinel-absence
   on enumeration + resolution-to-`Some` at start + contention read handling.
2. `crates/worldwake-sim/src/action_payload.rs` — update `sample_escort_to_safety_payload`
   and the trait assertion to the `Option` shape.
3. `crates/worldwake-sim/src/save_load.rs` — version-assert tests updated to 97.

### Commands Run

1. Passed `cargo test -p worldwake-systems escort`.
2. Passed `cargo test -p worldwake-sim -p worldwake-ai`.
3. Passed `cargo test -p worldwake-ai`.
4. Passed `./scripts/verify.sh`.

Merge note: Ticket 004 bumps SAVE_FORMAT_VERSION 96→97 (changing the serialized `EscortToSafetyActionPayload.intended_heal_action` shape); no sibling ticket touches serialized state, so this is the only bump.

## Outcome

Completion date: 2026-05-21

The escort payload sentinel is removed. Planning and affordance construction now
represent the unresolved heal-action reference as `None`, action start resolves it
to `Some(heal_id)`, and the final care contention handoff treats unresolved `None`
as an internal error rather than a silent skip. The serialized action payload shape
changed, so the save format version bumped from 96 to 97.

Deviation from original plan: no separate new test function was needed; the existing
escort-focused tests were the stronger local proof seam and now assert both
pre-resolution absence and start-time resolution. The `worldwake-ai` package proof
surfaced a now-unused import, which was removed as verification hygiene.

Verification: focused, affected-package, and final wrapper gates passed as recorded
above.
