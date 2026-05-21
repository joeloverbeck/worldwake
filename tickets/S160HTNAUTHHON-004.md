# S160HTNAUTHHON-004: Remove escort u32::MAX sentinel

**Status**: PENDING
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

## Verification Layers

1. No plan, action trace, or dispatch observes a placeholder/sentinel action id ->
   focused test asserting the pre-resolution payload is `None` (there is no sentinel
   `ActionDefId` to observe).
2. Resolution at action start sets `Some(heal_action_id(...))` -> action-trace /
   focused authoritative runtime test over the escort start handler
   (`escort_actions.rs:401`); the contention enqueue read (`:600`) treats `None` at
   that point as an internal error, not a silent skip.
3. Payload revalidation accepts the escort step -> focused runtime
   request-resolution coverage: both construction sites produce `None`, so
   `requested_affordance_matches` / `with_payload_override_validator` sees `None ==
   None` and does not reject the step.
4. Save/replay round-trips the new shape -> save_load focused test at the bumped
   `SAVE_FORMAT_VERSION` (97).

## What to Change

### 1. Change the field type (sim)

`action_payload.rs:396` — `pub intended_heal_action: Option<ActionDefId>`. Update the
test sample at `:696` to `Some(ActionDefId(27))`.

### 2. Construction sites set `None` (ai + systems)

- `goal_model.rs:962` (planner payload-override) — `intended_heal_action: None`.
- `escort_actions.rs:210` / `build_escort_payload` (`:165–177`) — change the
  `intended_heal_action` parameter/literal so enumeration produces `None`.

### 3. Runtime resolution and read (systems)

- `escort_actions.rs:401` — `payload.intended_heal_action = Some(heal_action_id(context.action_defs)?);`
- `escort_actions.rs:600` (`enqueue_for_contention`) — handle `Option`; resolution
  precedes enqueue, so treat a `None` here as an internal error
  (`ActionError::InternalError`), not a silent skip.

### 4. Bump the save format (sim)

`save_load.rs:7` — `SAVE_FORMAT_VERSION` 96 → 97. Update the version-assert tests at
`save_load.rs:1369` and `:1380` to 97.

### 5. Add the sentinel-absence test

Assert that no constructed escort payload (enumeration or planner override) carries
a placeholder action id — the pre-resolution value is `None`.

## Files to Touch

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

### Tests That Must Pass

1. New focused test: pre-resolution escort payloads carry `None` (no sentinel id).
2. New/updated runtime test: action start resolves `intended_heal_action` to
   `Some(heal_action_id(...))`; the escort step survives payload revalidation.
3. Updated save_load version-assert tests pass at `SAVE_FORMAT_VERSION` 97.
4. Existing suite: `cargo test -p worldwake-sim -p worldwake-systems -p worldwake-ai`

### Invariants

1. No plan, action trace, or dispatch ever observes a placeholder/sentinel
   `ActionDefId` for `intended_heal_action`.
2. `intended_heal_action` is `Some` by the time it is read at contention enqueue;
   a `None` at that point is an internal error, never a silent skip.
3. Two live authoritative representations of the same fact do not coexist (FND-28):
   the sentinel path is removed, not aliased.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/escort_actions.rs` (test module) — sentinel-absence
   on enumeration + resolution-to-`Some` at start + contention read handling.
2. `crates/worldwake-sim/src/action_payload.rs` — update `sample_escort_to_safety_payload`
   and the trait assertion to the `Option` shape.
3. `crates/worldwake-sim/src/save_load.rs` — version-assert tests updated to 97.

### Commands

1. `cargo test -p worldwake-systems escort`
2. `cargo test -p worldwake-sim -p worldwake-ai`
3. `./scripts/verify.sh`

Merge note: Ticket 004 bumps SAVE_FORMAT_VERSION 96→97 (changing the serialized `EscortToSafetyActionPayload.intended_heal_action` shape); no sibling ticket touches serialized state, so this is the only bump.
