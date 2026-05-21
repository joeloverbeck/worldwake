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
**Crates:** `worldwake-cli` — play surface `handlers/actions.rs`; debug-marker
targets `display.rs`, `handlers/control.rs`, `handlers/world_overview.rs`,
`handlers/inspect.rs`, `handlers/events.rs`, `handlers/tick.rs`.
**Foundations:** FND-14, FND-14A, FND-19
**Source:** `reports/ai-architecture-consolidation-third-iteration.md` §10/§5
(triage `docs/triage/2026-05-21-ai-architecture-consolidation-third-iteration-triage.md`).

## Problem Statement

### Motivation

FND-19: "Outside explicit debug, authoring, or replay tools, the interface may
surface only what the currently controlled agent could lawfully perceive, infer,
remember, or obtain from records and testimony. UI convenience must not become an
omniscient side channel."

The CLI action menu (`actions`/`do`) is the player-facing *play surface* and is
built correctly through `PerAgentBeliefView` + `get_affordances` (the right
skeleton for AI/player symmetry). But its rendered target labels still read
authoritative `World` directly, and the CLI's many omniscient observer/debug
commands carry no marker preventing a future player UI from importing them. Now
that archived S162 makes the belief view lawful, the one remaining FND-19 leak in
the *play surface* is the menu's label resolution; everything else omniscient in
the REPL is debug/observer console and needs marking, not POV-gating, in this
spec. (`handle_cancel` was previously suspected to leak but already scopes to the
controlled entity — see Evidence and Deliverable 2.)

### Evidence (verified against code on 2026-05-21)

- **`crates/worldwake-cli/src/handlers/actions.rs:81`** — `handle_actions` builds
  the menu via the lawful affordance path (`:43-46`,
  `PerAgentBeliefView::with_runtime_from_world` → `get_affordances`), but then
  renders bound-target names with the **omniscient** `entity_display_name(sim.world(),
  *t)`. The affordances are belief-filtered; their display names are resolved from
  world truth. A player could read names of entities the affordance machinery never
  meant to expose by that channel.
- **`crates/worldwake-cli/src/handlers/actions.rs:160`** — `handle_cancel`
  iterates `sim.scheduler().active_actions()` but already `.find`s only the
  controlled entity's own action (`instance.actor == entity`, `:160-165`) and
  enqueues `CancelAction { actor: entity, .. }`. It has filtered by actor since
  the function was created (commit `f3697cc9`, E21CLIHUMCON-008); a player cannot
  enumerate or cancel another agent's in-flight action, and the command prints no
  entity names. The scoping is **already correct** — what is missing is a
  regression guard locking it (see Deliverable 2).
- **`crates/worldwake-cli/src/display.rs`** — `entity_display_name` (`:37-61`),
  `resolve_entity` (`:71-110`), `format_location` (`:148-161`) read `World`
  directly (names, topology, item lots, kinds, transit). Module doc says "pure
  read-only" but does **not** mark the module observer/debug-only.
- **`crates/worldwake-cli/src/handlers/control.rs`** — `handle_switch` (`:52-115`)
  is a meta operation (resolves any named entity, checks aliveness, mutates
  `ControlSource`, prints current location) built on the omniscient `display.rs`
  helpers. No module-level debug/meta marker.
- **The wider REPL is omniscient too.** `dispatch_command`
  (`crates/worldwake-cli/src/handlers/mod.rs:32-71`) routes a large set of
  human-reachable read commands that read authoritative `World` directly and are
  gated by neither belief nor control mode: `world`/`places`/`agents`/`goods`
  (`world_overview.rs`), `inspect`/`relations`/`look`/`inventory`/`needs`
  (`inspect.rs`, e.g. owner/holder/members/office/hostility at `:591-624`),
  `events`/`event`/`trace` (`events.rs`), and `tick`'s action-trace output naming
  all agents (`tick.rs:80-89`). These are *not* part of the lawful play surface —
  they are the CLI's debug/observer console (the source report classifies them as
  such and recommends a future `DebugWorldView`/`ObserverUi` capability, §10 /
  lines 180, 365). This spec marks them debug-only so a future normal-play UI
  cannot silently inherit them; POV-gating them is explicitly out of scope (see
  Non-Goals).

### Key scoping decisions (brainstorm 2026-05-21)

- **The play surface is exactly the action menu** (`actions`/`do`, plus
  self-scoped `status`/`look`/`inventory`/`needs` about the controlled agent).
  Everything else routed by `dispatch_command` is debug/observer console:
  `display.rs`, `control.rs` (`switch`/`observe`), `world_overview.rs`
  (`world`/`places`/`agents`/`goods`), `inspect.rs` (`inspect`/`relations`),
  `events.rs` (`events`/`event`/`trace`), and `tick.rs`'s action-trace output.
  These are legitimate observer/debug surfaces — not bugs to be rewritten now. The
  fix is to (a) **mark** them debug/observer-only so a future normal-play UI cannot
  silently inherit omniscience, and (b) stop the play surface (`actions.rs`) from
  routing player-visible output through them.
- This spec changes **no authoritative state and no agent decision logic**; it is
  purely about what the *human interface* surfaces. `switch`/`observe` remain
  omniscient meta-control (correct for a debug tool), just explicitly labeled.

### Non-Goals

- **POV-gating the debug/observer console commands.** `world`/`places`/`agents`/
  `goods`, `inspect`/`relations`, `events`/`event`/`trace`, `switch`/`observe`, and
  `tick`'s trace output stay omniscient by design; this spec only **marks** them
  debug-only (Deliverable 3) so a future normal-play UI cannot inherit them
  silently. Routing them through the belief view is future-UI work, gated on a real
  player UI existing.
- **No full `pov_display.rs` build-out and no `CharacterPovView` capability layer.**
  That is speculative future-UI scope (the report's own §10 recommendation set).
  The minimal root-cause fix is a POV-safe label resolver for the play surface
  (Deliverable 1) plus the debug-only markers (Deliverable 3). A richer POV display
  module — and a dedicated `DebugWorldView`/`ObserverUi` capability for the console
  commands — is a future spec if/when a real player UI is built.

## Deliverables

1. **POV-safe target labels in the player menu** (`actions.rs:78-82`) — resolve
   bound-target display labels through the belief view (believed name / last-seen
   label / a generic "unknown" token), not `entity_display_name(sim.world(), ..)`.
   The label source must match the lawfulness of the affordance that surfaced the
   target: a directly-perceivable physical label of a co-located entity (item-lot
   "5× Grain", workstation tag) is lawful under FND-14A even without a stored
   belief entry, while remote, social, or relational facts require a belief.
   Mechanism (a belief-view label accessor vs. a thin POV resolver) is a
   ticket-time detail — note that no belief-view *name/label* accessor exists today
   (`per_agent_belief_view.rs` exposes `believed_*` / `last_seen_memory`, not a
   display-name method), so a thin POV resolver built over the belief view is the
   likely path. It must not read authoritative `World` names for entities the
   controlled agent does not lawfully perceive or recall.

2. **Regression guard for `handle_cancel` scoping** (`actions.rs:153-183`) — the
   handler already scopes to the controlled entity (`.find(|(_, instance)|
   instance.actor == entity)`, `:160-165`), so no behavioral change is needed. Add
   a focused test that locks this: with another agent's action active and the
   controlled agent idle, `handle_cancel` enqueues nothing and references no other
   agent's action. The guard fails if a future change reverts to global
   enumeration.

3. **Mark the debug/observer console surfaces as such** — add module-level doc
   comments to `display.rs`, `control.rs`, `world_overview.rs`, `inspect.rs`,
   `events.rs`, and the trace-rendering portion of `tick.rs`, stating these
   surfaces read authoritative world truth and are for observer/debug/replay
   tooling only; normal player-facing UI must not depend on them. Pair the markers
   with an enforceable guard scoped to the **play surface** — defined as the
   action-menu path in `actions.rs` (`handle_actions`/`handle_do`) — asserting it
   does not call `entity_display_name`/`resolve_entity`/`format_location` for
   player-visible output (ticket-time choice: a module-boundary test or equivalent
   lint). The guard must fail if the play surface regains an omniscient display
   dependency. (Defining the play surface narrowly is required: the three helpers
   are called from `actions.rs`, `control.rs`, `inspect.rs`, `world_overview.rs`,
   `events.rs`, `tick.rs`, and `observer.rs`, so a guard over "all handlers" cannot
   be written — only the action-menu path is play-surface.)

4. **FND-19 player/AI symmetry test** — assert that, for the same controlled
   entity with the same belief state, the player action menu and the AI affordance
   set are identical, and that the menu's *labels* expose no fact the actor could
   not lawfully perceive (FND-14A same-tick local observation) or recall (belief).
   Combined with archived S162's belief-wall goldens, this proves **the action
   menu** adds no omniscient side channel. The claim is scoped to the play surface
   only — the debug/observer console commands (`world`/`inspect`/`events`/etc.)
   remain omniscient by design and are out of scope for POV-gating (see Non-Goals);
   `switch`/`observe` are likewise debug/meta and tested separately as such.

## Authoritative-to-AI Impact Analysis

This spec does not touch authoritative validation, affordance enumeration, or agent
decision logic — only CLI presentation and the cancel-scoping. The AGENTS.md
Authoritative-To-AI Impact Rule is therefore **not applicable** (no `validate_*`,
`can_exercise_control`, precondition, or planner-input change). There is no
behavioral change at all: Deliverable 1 alters only how the menu *labels* are
resolved, Deliverable 2 is a regression test over already-correct scoping, and
Deliverable 3 adds doc markers plus a boundary guard. No `InputKind` payload
semantics change.

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

Cross-surface feature (belief view → CLI play surface). Negative illegal-path case
the feature must **not** produce: a player reading, through the action menu or
cancel command, any entity name/location/in-flight-action that the controlled
agent could not lawfully perceive (FND-14A same-tick local observation) or recall
(belief). The debug/observer console commands are intentionally exempt — they are
omniscient by design and are only marked, not gated, here (see Non-Goals).
Feature-scoped checks: the player/AI symmetry test (Deliverable 4), the
play-surface boundary guard (Deliverable 3), and focused tests on `handle_actions`
labels and the `handle_cancel` scoping regression guard (Deliverable 2). Depends on
archived `archive/specs/S162-belief-view-source-gate-hardening.md` (the belief view
must be lawful for the symmetry test to be meaningful).
