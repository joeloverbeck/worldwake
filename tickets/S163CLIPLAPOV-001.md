# S163CLIPLAPOV-001: POV-safe action-menu target labels

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — `worldwake-cli` presentation only
**Deps**: None (`archive/specs/S162-belief-view-source-gate-hardening.md` already landed)

## Problem

The CLI action menu (`actions`/`do`) is the player-facing play surface and builds
its candidate list through the lawful affordance path
(`PerAgentBeliefView::with_runtime_from_world` → `get_affordances`,
`crates/worldwake-cli/src/handlers/actions.rs:43-46`). But it then renders each
bound target's label with the omniscient `entity_display_name(sim.world(), *t)`
(`actions.rs:81`), which reads authoritative `World` truth (names, topology, item
lots, kinds, transit) for the target regardless of what the controlled agent
lawfully knows. For a remote bound target (e.g. an escort/travel destination known
only via belief) this surfaces world truth the player should not see — an FND-19
omniscient side channel in the one surface that is supposed to be lawful. This is
S163 Deliverable 1.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `handle_actions` exists at `crates/worldwake-cli/src/handlers/actions.rs:33`;
   the omniscient label call is `entity_display_name(sim.world(), *t)` at
   `actions.rs:81` inside the `bound_targets` map (`:78-83`). `actions.rs:10`
   imports exactly one display helper (`use crate::display::entity_display_name;`)
   and `:81` is its only call site in this file — removing the call lets the import
   be removed too. Existing inline tests `test_actions_lists_affordances:410` and
   `test_actions_stores_in_repl_state:425` assert only that `last_affordances` is
   non-empty; neither asserts label text, so this change does not break them.
2. Spec contract: S163 Deliverable 1
   (`specs/S163-cli-player-pov-boundary.md:111-123`). The label source must match
   the lawfulness of the affordance that surfaced the target — FND-14A permits
   reading a co-located entity's directly-perceivable physical label (item-lot
   commodity/quantity, workstation tag, kind) and public topology (place names);
   remote/social/relational facts require a belief.
3. Shared boundary under audit: the belief-view read surface
   `crates/worldwake-sim/src/per_agent_belief_view.rs`. It exposes `believed_entity`
   (`:268`), `last_seen_memory` (`:1431`), `believed_entities_at` (`:1066`), and
   `effective_place` (via `Deref`), but **no** display-name/label accessor. The POV
   resolver is therefore new CLI-side code built over the existing view; it does
   not add a belief-view trait method (no `worldwake-sim` change).
4. Information-path: today the same label fact has one transport path in the play
   surface — the direct omniscient `World` read. After this change the canonical
   (and only) play-surface path is the POV resolver, which reads world truth only
   for FND-14A-lawful co-located physical facts and public topology, and falls back
   to belief / a generic "unknown" token otherwise. No mixed-state coexistence
   remains: the `entity_display_name` call and its import are removed from
   `actions.rs` in-scope.
5. Adjacent contradiction classification: the wider REPL (`world`/`inspect`/
   `events`/`tick` traces) is also omniscient, but that is **future cleanup** owned
   by S163CLIPLAPOV-002 (debug-only marking, not POV-gating) per S163 Non-Goals —
   it is explicitly out of scope here.

## Architecture Check

1. A thin CLI-side resolver over the already-constructed `PerAgentBeliefView` is the
   minimal root-cause fix: the view is the canonical FND-14A split implementation
   (per CLAUDE.md), so routing labels through it makes label lawfulness match
   affordance lawfulness by construction, with no new belief-view surface and no
   authoritative-state change.
2. No backward-compatibility shim: the omniscient `entity_display_name` call and its
   `use` import are deleted from the play surface, not wrapped or aliased. A richer
   `pov_display.rs` / `CharacterPovView` is explicitly a Non-Goal (S163), so the
   resolver stays a small local helper, not a new module.

## Verification Layers

1. Label lawfulness for a co-located physical target (item lot / workstation) →
   focused unit test on the resolver asserting the FND-14A physical label is
   returned.
2. Label lawfulness for a remote/unknown target (not co-located, not in belief) →
   focused unit test asserting the resolver returns a believed label or the generic
   "unknown" token, never the authoritative `World` name.
3. Single-layer ticket: this is `worldwake-cli` presentation only — no decision
   trace, action trace, or event-log delta is involved (no authoritative mutation,
   no planner input). The proof surface is focused CLI unit tests over the resolver
   and `handle_actions`.

## What to Change

### 1. Add a POV-safe target-label resolver

In `crates/worldwake-cli/src/handlers/actions.rs` (or a small sibling helper in the
play-surface module), add a function that resolves a bound target's display label
from the controlled agent's POV, given the already-built `PerAgentBeliefView` and
the target `EntityId`:

- If the target is at the controlled agent's effective place (co-located),
  return the FND-14A directly-perceivable physical label — item-lot
  commodity/quantity, workstation tag, or kind — and public topology place names.
- Else, if the target is in the agent's belief (`believed_entity` /
  `last_seen_memory`), return the believed/last-seen label.
- Else, return a generic `"unknown"` token. Never read the authoritative `World`
  `Name` for an entity that is neither co-located nor believed.

Mechanism choice (a free helper in `actions.rs` vs. a tiny `pov` submodule) is at
implementer discretion; keep it minimal per the Non-Goal against a `pov_display.rs`
build-out.

### 2. Route the menu labels through the resolver

Replace the `entity_display_name(sim.world(), *t)` map at `actions.rs:78-83` with a
map over the new resolver, passing the `view` constructed at `actions.rs:44`. Remove
the now-unused `use crate::display::entity_display_name;` import at `actions.rs:10`.

## Files to Touch

- `crates/worldwake-cli/src/handlers/actions.rs` (modify)

## Out of Scope

- Marking the debug/observer console surfaces (`display.rs`, `control.rs`,
  `world_overview.rs`, `inspect.rs`, `events.rs`, `tick.rs`) debug-only and the
  play-surface boundary guard — S163CLIPLAPOV-002.
- The `handle_cancel` regression guard (D2) and the FND-19 symmetry test (D4) —
  S163CLIPLAPOV-003.
- Any belief-view trait/accessor change in `worldwake-sim`; the resolver is
  CLI-side and reads the existing view surface only.
- POV-gating any REPL command other than the action menu (S163 Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. Resolver returns the FND-14A physical label for a co-located item-lot /
   workstation target without reading the authoritative `Name`.
2. Resolver returns a believed/last-seen label for a believed-but-not-co-located
   target, and the generic `"unknown"` token for a target neither co-located nor
   believed — never the authoritative `World` name in the latter case.
3. `handle_actions` still produces a non-empty menu for the existing food scenario
   (`test_actions_lists_affordances`, `test_actions_stores_in_repl_state` continue
   to pass).
4. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. The play surface (`handle_actions`) never reads the authoritative `World` `Name`
   of a target the controlled agent does not lawfully perceive (FND-14A) or recall
   (belief). (FND-14, FND-14A, FND-19.)
2. No new authoritative state and no agent decision logic change: this ticket
   alters only how menu labels are derived.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/handlers/actions.rs` (inline `#[cfg(test)]`) — add
   focused tests for the resolver covering the co-located-physical, believed, and
   unknown cases, using the existing `human_with_food_scenario` /`observer_scenario`
   helpers plus a two-place scenario for the remote/unknown case.

### Commands

1. `cargo test -p worldwake-cli handlers::actions`
2. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. `scripts/verify.sh` (before PR push)
