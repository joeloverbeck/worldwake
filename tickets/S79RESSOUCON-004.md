# S79RESSOUCON-004: Define lawful water-source harvest contract for S79

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — production recipe bootstrap, scenario authoring contract, possibly workstation schema
**Deps**: specs/S79-resource-source-consumption-affordances.md, tickets/S79RESSOUCON-001.md, tickets/S79RESSOUCON-002.md, archive/tickets/S79RESSOUCON-003.md

## Problem

S79 and S79RESSOUCON-003 both assume an agent can lawfully execute `harvest water -> drink`, but the live production substrate still has no canonical `Harvest Water` recipe and no explicit workstation contract for water sources. Ticket 001 completed the scenario-side recipe/bootstrap and facility-attachment path for existing harvestable recipes, but it also exposed that water remains an unowned production contract rather than a wired runtime path.

## Assumption Reassessment (2026-04-09)

1. The canonical production recipe bootstrap added in `crates/worldwake-systems/src/action_registry.rs` currently contains only `Harvest Apples`, `Harvest Grain`, and `Bake Bread`. There is no live `Harvest Water` recipe.
2. `specs/S79-resource-source-consumption-affordances.md` and `tickets/S79RESSOUCON-003.md` both explicitly reference a `Harvest Water` scenario and expect a water-source facility to support harvest-to-drink behavior.
3. Shared boundary under audit: canonical production recipe registry -> harvest action registration -> scenario authoring of facility-backed resource sources.
4. The live harvest action contract in `crates/worldwake-systems/src/production_actions.rs` requires one facility target carrying both `WorkstationMarker` and `ResourceSource`, and the recipe must declare the required workstation tag.
5. Newly exposed adjacent contradiction: water-source consumption is not blocked by `KnownRecipes` wiring anymore; it is blocked by the absence of a lawful production recipe/workstation contract for water. This is a separate bug/follow-up, not part of ticket 001's now-completed scenario bootstrap slice.

## Architecture Check

1. This follow-up should establish one explicit water-source production contract rather than special-casing drink-from-source behavior. That keeps S79 aligned with explicit transfer and shared harvest semantics.
2. No backward-compatibility shims: the end state should choose the canonical water-source substrate and update scenario/spec/golden surfaces to match it.

## Verification Layers

1. Canonical water harvest recipe exists -> focused unit/runtime proof on the canonical recipe registry
2. Water-source facility is authorable through the lawful scenario/world shape -> authoritative world-state proof
3. `harvest water -> drink` becomes planner-visible and executable -> follow-up golden E2E proof that extends the completed apple proof surface from `archive/tickets/S79RESSOUCON-003.md`

## What to Change

### 1. Reassess and choose the canonical water-source workstation contract

Determine whether water harvest should use an existing workstation tag (if one already lawfully represents the source) or whether S79 needs an explicit new facility/workstation shape for water sources.

### 2. Wire the chosen water harvest contract through the runtime

Add the canonical `Harvest Water` recipe and any required scenario/bootstrap support so the production registry, action registration, and authored world shape all agree.

### 3. Reconcile S79 / archived S79RESSOUCON-003 proof surfaces

Update the active S79 spec and any new golden follow-up assumptions if the lawful water-source contract differs from the current draft narrative.

## Files to Touch

- `specs/S79-resource-source-consumption-affordances.md` (modify — if reassessment changes the lawful water-source contract)
- `archive/tickets/S79RESSOUCON-003.md` (modify — only if the archived apple-proof handoff needs factual amendment after the water contract lands)
- `crates/worldwake-systems/src/action_registry.rs` (modify — add canonical water recipe if lawful)
- Additional production/scenario files as required by the chosen workstation contract

## Out of Scope

- Reverting ticket 001's completed apple/grain/bread scenario bootstrap
- Planner hypothetical harvest effects already owned by ticket 002
- Golden implementation itself unless this ticket is intentionally merged with 003

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof that the canonical runtime recipe registry includes the lawful water harvest recipe
2. Focused proof that the authored water-source facility shape can satisfy harvest action registration/preconditions
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
