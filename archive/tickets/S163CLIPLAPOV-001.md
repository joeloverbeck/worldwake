# S163CLIPLAPOV-001: POV-safe action-menu target labels

**Status**: COMPLETED
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
lawfully knows. Before this ticket, a remote bound target (e.g. an escort/travel
destination known only via belief) could surface world truth the player could not
lawfully know — an FND-19 omniscient side channel in the one surface that is
supposed to be lawful. This was S163 Deliverable 1.

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
   by the now-archived `archive/tickets/S163CLIPLAPOV-002.md` (debug-only marking,
   not POV-gating) per S163 Non-Goals — it is explicitly out of scope here.

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

## Verified Layers

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

## Landed Changes

### 1. Added a POV-safe target-label resolver

In `crates/worldwake-cli/src/handlers/actions.rs`, this ticket added
`pov_target_label`, a local helper that resolves a bound target's display label
from the controlled agent's POV using the already-built `PerAgentBeliefView`, the
controlled actor, and the target `EntityId`:

- If the target is at the controlled agent's effective place (co-located),
  it returns the FND-14A directly-perceivable physical label: item-lot
  commodity/quantity, workstation tag, resource-source commodity, or entity kind.
- If the target is a place, it returns the public topology place name.
- If the target is known through belief or last-seen memory, it returns a
  belief-backed / last-seen label such as `Agent last seen at Market`.
- Otherwise, it returns the generic `"unknown"` token. It does not read the
  authoritative `Name` for an entity that is neither co-located nor remembered.

The implementation stayed as a free helper in `actions.rs`, not a new
`pov_display.rs` module or cross-crate belief-view API.

### 2. Routed the menu labels through the resolver

The `handle_actions` target-label map now calls `pov_target_label(&view,
sim.world(), entity, *t)`. The old `entity_display_name` import and call were
removed from the play-surface action menu.

## Landed Files

- `crates/worldwake-cli/src/handlers/actions.rs` (modified)

## Out of Scope

- Marking the debug/observer console surfaces (`display.rs`, `control.rs`,
  `world_overview.rs`, `inspect.rs`, `events.rs`, `tick.rs`) debug-only and the
  play-surface boundary guard — `archive/tickets/S163CLIPLAPOV-002.md`.
- The `handle_cancel` regression guard (D2) and the FND-19 symmetry test (D4) —
  S163CLIPLAPOV-003.
- Any belief-view trait/accessor change in `worldwake-sim`; the resolver is
  CLI-side and reads the existing view surface only.
- POV-gating any REPL command other than the action menu (S163 Non-Goals).

## Acceptance Result

### Passed Criteria

1. `pov_target_label_uses_local_physical_item_label` proves the resolver returns
   the FND-14A physical item-lot label for a co-located target without using the
   target's authoritative `Name`.
2. `pov_target_label_uses_last_seen_label_without_remote_name` and
   `pov_target_label_hides_unknown_remote_name` prove remembered remote targets
   use a last-seen label while unknown remote targets render `"unknown"` and do
   not expose the authoritative remote name.
3. `test_actions_lists_affordances` and `test_actions_stores_in_repl_state`
   continue to pass with the menu routed through the POV resolver.
4. Existing suite passed: `cargo test -p worldwake-cli`.

### Preserved Invariants

1. The play surface (`handle_actions`) never reads the authoritative `World` `Name`
   of a target the controlled agent does not lawfully perceive (FND-14A) or
   remember (belief / last-seen memory). (FND-14, FND-14A, FND-19.)
2. No authoritative state and no agent decision logic changed; this ticket altered
   only how menu labels are derived.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/handlers/actions.rs` (inline `#[cfg(test)]`) — added
   focused resolver tests covering co-located physical labels, remembered remote
   labels, and unknown remote labels.

### Commands Run

1. `cargo test -p worldwake-cli handlers::actions`
2. `cargo test -p worldwake-cli`
3. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-22.

- Replaced the play-surface action-menu target label path with a local
  POV-safe resolver over `PerAgentBeliefView`.
- Preserved lawful public topology names and same-tick local physical labels,
  while rendering remembered remote targets from belief/last-seen memory and
  unknown remote targets as `"unknown"`.
- Added focused regression tests proving local physical labels, last-seen labels,
  and unknown remote targets do not expose remote authoritative names.

## Deviations

- The landed believed/last-seen label is intentionally descriptive
  (`Agent last seen at Market`) rather than a personal name. The live belief
  surface does not store a display-name field for remembered entities, and this
  ticket did not add a new belief-view API.
- `scripts/verify.sh` was not run for this first ticket iteration; the ticket
  surface was verified with the focused handler tests, full `worldwake-cli` crate
  tests, and the ticket's CI-shaped `worldwake-cli` clippy gate. The harness will
  run `./scripts/verify.sh` before final branch push if the full S163 family lands.

## Verification Result

- Passed `cargo test -p worldwake-cli handlers::actions`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
