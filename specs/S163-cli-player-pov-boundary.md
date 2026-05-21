# S163 — CLI Player-POV Boundary

**Status:** DRAFT
**Type:** Correctness fix (FND-19 agent symmetry: stop the player-facing CLI path
from surfacing omniscient world facts; mark the omniscient observer/debug surfaces
as such). No new authoritative simulation state, system, component, action, or
feedback loop.
**Priority:** Medium. Sequence after
`archive/specs/S162-belief-view-source-gate-hardening.md` — the player menu
inherits the belief view, so the view must be lawful first. Independent of the
FRAMECAUSEVT ticket.
**Crates:** `worldwake-cli` (`handlers/actions.rs`, `display.rs`, `handlers/control.rs`).
**Foundations:** FND-14, FND-14A, FND-19
**Source:** `reports/ai-architecture-consolidation-third-iteration.md` §10/§5
(triage `docs/triage/2026-05-21-ai-architecture-consolidation-third-iteration-triage.md`).

## Problem Statement

### Motivation

FND-19: "Outside explicit debug, authoring, or replay tools, the interface may
surface only what the currently controlled agent could lawfully perceive, infer,
remember, or obtain from records and testimony. UI convenience must not become an
omniscient side channel."

The CLI action menu is the player-facing prototype and is built correctly through
`PerAgentBeliefView` + `get_affordances` (the right skeleton for AI/player
symmetry). But two seams in the *player* path read authoritative `World` directly,
and the omniscient observer/debug helpers carry no marker preventing a future
player UI from importing them. Now that archived S162 makes the belief view
lawful, these CLI seams are the remaining FND-19 leaks in the play interface.

### Evidence (verified against code on 2026-05-21)

- **`crates/worldwake-cli/src/handlers/actions.rs:81`** — `handle_actions` builds
  the menu via the lawful affordance path (`:43-46`,
  `PerAgentBeliefView::with_runtime_from_world` → `get_affordances`), but then
  renders bound-target names with the **omniscient** `entity_display_name(sim.world(),
  *t)`. The affordances are belief-filtered; their display names are resolved from
  world truth. A player could read names of entities the affordance machinery never
  meant to expose by that channel.
- **`crates/worldwake-cli/src/handlers/actions.rs:160`** — `handle_cancel`
  iterates `sim.scheduler().active_actions()` (all global active actions) rather
  than the controlled agent's own actions.
- **`crates/worldwake-cli/src/display.rs`** — `entity_display_name` (`:37-61`),
  `resolve_entity` (`:71-110`), `format_location` (`:148-161`) read `World`
  directly (names, topology, item lots, kinds, transit). Module doc says "pure
  read-only" but does **not** mark the module observer/debug-only.
- **`crates/worldwake-cli/src/handlers/control.rs`** — `handle_switch` (`:52-115`)
  is a meta operation (resolves any named entity, checks aliveness, mutates
  `ControlSource`, prints current location) built on the omniscient `display.rs`
  helpers. No module-level debug/meta marker.

### Key scoping decisions (brainstorm 2026-05-21)

- **`display.rs` / `control.rs` are legitimate observer/debug surfaces** — they are
  not bugs to be rewritten now. The fix is to (a) **mark** them debug/observer-only
  so a future normal-play UI cannot silently inherit omniscience, and (b) stop the
  *player* path (`actions.rs`) from routing player-visible output through them.
- **No full `pov_display.rs` build-out and no `CharacterPovView` capability layer**
  in this spec. That is speculative future-UI scope (the report's own §10
  recommendation set). The minimal root-cause fix is a POV-safe name/label resolver
  for the player path plus the debug-only markers. A richer POV display module is a
  future spec if/when a real player UI is built.
- This spec changes **no authoritative state and no agent decision logic**; it is
  purely about what the *human interface* surfaces. `switch`/`observe` remain
  omniscient meta-control (correct for a debug tool), just explicitly labeled.

## Deliverables

1. **POV-safe target labels in the player menu** (`actions.rs:78-82`) — resolve
   bound-target display labels through the belief view (believed name / last-seen
   label / a generic "unknown" token), not `entity_display_name(sim.world(), ..)`.
   The label source must match the lawfulness of the affordance that surfaced the
   target. Mechanism (a belief-view label accessor vs. a thin POV resolver) is a
   ticket-time detail, but it must not read authoritative `World` names for
   entities the controlled agent does not lawfully know.

2. **Scope `handle_cancel` to the controlled agent** (`actions.rs:153-183`) — cancel
   only actions belonging to the controlled entity, not all global active actions.
   The player must not be able to enumerate or cancel other agents' in-flight
   actions through this command.

3. **Mark `display.rs` and `control.rs` as observer/debug-only** — add a
   module-level doc comment stating these surfaces read authoritative world truth
   and are for observer/debug/replay tooling only; normal player-facing UI must not
   depend on them. Pair the doc with an enforceable guard (ticket-time choice: a
   module-boundary test asserting the player-facing handlers do not call
   `entity_display_name`/`resolve_entity`/`format_location` for player-visible
   output, or an equivalent lint/test). The guard must fail if the player path
   regains an omniscient display dependency.

4. **FND-19 player/AI symmetry test** — assert that, for the same controlled
   entity with the same belief state, the player action menu and the AI affordance
   set are identical, and that the menu's *labels* expose no fact absent from the
   actor's belief. Combined with archived S162's belief-wall goldens, this proves the play
   interface adds no omniscient side channel. (`switch`/`observe` are explicitly
   excluded as debug/meta and tested separately as such.)

## Authoritative-to-AI Impact Analysis

This spec does not touch authoritative validation, affordance enumeration, or agent
decision logic — only CLI presentation and the cancel-scoping. The CLAUDE.md
authoritative-to-AI checklist is therefore **not applicable** (no `validate_*`,
`can_exercise_control`, precondition, or planner-input change). The one behavioral
change, `handle_cancel` scoping, affects only which input the human may enqueue; it
must still produce a lawful `InputKind` for the controlled entity's own action.

## FND-01 Section H Analysis

Correctness/UI-boundary fix; no new authoritative state, system, component, action,
or feedback loop. Per `docs/spec-drafting-rules.md`, mandatory headers with
applicability:

- **Information-path analysis:** This spec *removes* an unlawful UI information path
  (omniscient world names/locations surfaced to the human) and routes player-visible
  labels through the lawful belief view. No new path introduced.
- **Positive-feedback analysis:** Not applicable. No loop.
- **Concrete dampeners:** Not applicable.
- **Stored-state vs. derived read-model list:** No new authoritative state. Display
  output is a transient derived view; this spec narrows the player path's view to
  belief-backed labels. `display.rs`/`control.rs` remain derived observer views,
  now explicitly scoped to debug/observer use (FND-27, FND-29).
- **Planner-formalism analysis:** Not applicable; no planner change.

### Systemic-Validation Analysis (FND-31)

Cross-surface feature (belief view → CLI). Negative illegal-path case the feature
must **not** produce: a player reading, through the action menu or cancel command,
any entity name/location/in-flight-action that the controlled agent does not
lawfully know. Feature-scoped checks: the player/AI symmetry test (Deliverable 4),
the module-boundary guard (Deliverable 3), and focused tests on `handle_actions`
labels and `handle_cancel` scoping. Depends on archived
`archive/specs/S162-belief-view-source-gate-hardening.md` (the belief view must
be lawful for the symmetry test to be meaningful).
