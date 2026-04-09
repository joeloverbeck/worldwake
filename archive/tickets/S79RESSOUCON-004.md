# S79RESSOUCON-004: Define lawful water-source harvest contract for S79

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — production recipe bootstrap, scenario authoring contract, possibly workstation schema
**Deps**: specs/S79-resource-source-consumption-affordances.md, tickets/S79RESSOUCON-001.md, tickets/S79RESSOUCON-002.md, archive/tickets/S79RESSOUCON-003.md

## Problem

S79 and S79RESSOUCON-003 both assume an agent can lawfully execute `harvest water -> drink`, but the live production substrate still has no canonical `Harvest Water` recipe and no explicit workstation contract for water sources. Ticket 001 completed the scenario-side recipe/bootstrap and facility-attachment path for existing harvestable recipes, but it also exposed that water remains an unowned production contract rather than a wired runtime path.

## Assumption Reassessment (2026-04-09)

1. The canonical production recipe bootstrap added in `crates/worldwake-systems/src/action_registry.rs` contained only `Harvest Apples`, `Harvest Grain`, and `Bake Bread`. There was no live `Harvest Water` recipe.
2. `specs/S79-resource-source-consumption-affordances.md` and `archive/tickets/S79RESSOUCON-003.md` both explicitly referenced a `Harvest Water` scenario and expected a water-source facility to support harvest-to-drink behavior.
3. Shared boundary under audit: canonical production recipe registry -> harvest action registration -> scenario authoring of facility-backed resource sources -> golden setup parity.
4. The live harvest action contract in `crates/worldwake-systems/src/production_actions.rs` requires one facility target carrying both `WorkstationMarker` and `ResourceSource`, and the recipe must declare the required workstation tag.
5. Reassessment result: there was no existing workstation tag that lawfully meant “harvestable water source.” Reusing `WashBasin` or `FieldPlot` would collapse unrelated facilities into one tag, so the canonical fix required an explicit `Well` contract.

## Architecture Check

1. This follow-up should establish one explicit water-source production contract rather than special-casing drink-from-source behavior. That keeps S79 aligned with explicit transfer and shared harvest semantics.
2. No backward-compatibility shims: the end state should choose the canonical water-source substrate and update scenario/spec/golden surfaces to match it.

## Verification Layers

1. Canonical water harvest recipe exists -> focused unit/runtime proof on the canonical recipe registry
2. Water-source facility is authorable through the lawful scenario/world shape -> authoritative world-state proof
3. `harvest water -> drink` becomes planner-visible and executable -> follow-up golden E2E proof that extends the completed apple proof surface from `archive/tickets/S79RESSOUCON-003.md`

## What to Change

### 1. Reassess and choose the canonical water-source workstation contract

Choose the canonical workstation contract for water harvest. If no existing tag lawfully represents a harvestable water source, add the explicit facility/workstation shape required by the live harvest action contract.

### 2. Wire the chosen water harvest contract through the runtime

Add the canonical `Harvest Water` recipe and the required runtime/scenario/bootstrap support so the production registry, action registration, authored world shape, and golden harness setup all agree on the same water-source substrate.

### 3. Reconcile S79 / archived S79RESSOUCON-003 proof surfaces

Update the active S79 spec and any new golden follow-up assumptions if the lawful water-source contract differs from the current draft narrative.

## Files to Touch

- `specs/S79-resource-source-consumption-affordances.md` (modify — if reassessment changes the lawful water-source contract)
- `crates/worldwake-core/src/production.rs` (modify — add explicit water-source workstation tag)
- `crates/worldwake-systems/src/action_registry.rs` (modify — add canonical water recipe if lawful)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — focused scenario proof for `Well`-backed water harvest bootstrap)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — keep golden recipe setup aligned with canonical runtime recipes)
- `crates/worldwake-ai/tests/golden_perception_exposure.rs` (modify — remove stale ad-hoc `FieldPlot` water-source setup)
- `scenarios/cli-evaluation.ron` (modify — attach authored water source to a named `Well` facility)

## Out of Scope

- Reverting ticket 001's completed apple/grain/bread scenario bootstrap
- Planner hypothetical harvest effects already owned by ticket 002
- Golden implementation itself unless this ticket is intentionally merged with 003

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof that the canonical runtime recipe registry includes `Harvest Water` on `WorkstationTag::Well`
2. Focused proof that the authored water-source facility shape can attach `ResourceSource { commodity: Water, ... }` to a `Well` and register `harvest:Harvest Water`
3. Existing suite: `cargo test -p worldwake-cli` or narrower affected-crate commands determined by reassessment

### Invariants

1. Water consumption remains an explicit transfer path, not a direct drink-from-global-source shortcut
2. The canonical water-source contract is shared consistently across recipe bootstrap, action registration, scenario authoring, and golden setup

## Test Plan

### New/Modified Tests

1. Modify focused recipe/bootstrap tests in the owning crate once the water contract is chosen
2. Create or modify the water-variant golden follow-up after the runtime contract is lawful; the completed apple proof in `archive/tickets/S79RESSOUCON-003.md` remains the existing owner for the apple branch

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Added the canonical `Well` workstation contract and `Harvest Water` recipe to the live production substrate.
- Updated the shipped CLI evaluation scenario to attach Thornwall Village water to a named well facility.
- Aligned focused golden/test setup so the repository now has one lawful authored path for facility-backed water harvest.
- The end-to-end water/drink golden remains deferred to the post-004 follow-up path already described in the S79/S81 planning material.

## Verification Result

- Passed `cargo test -p worldwake-cli`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
