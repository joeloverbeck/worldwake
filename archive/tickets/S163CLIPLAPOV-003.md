# S163CLIPLAPOV-003: handle_cancel regression guard + FND-19 player/AI symmetry test

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — `worldwake-cli` tests only
**Deps**: `archive/tickets/S163CLIPLAPOV-001.md` (the symmetry test asserts label lawfulness, which holds only after D1's POV resolver lands)

## Problem

Before this ticket, two FND-19 contracts on the play surface lacked locking tests:

- **D2** — `handle_cancel` already scopes to the controlled entity
  (`.find(|(_, instance)| instance.actor == entity)`,
  `crates/worldwake-cli/src/handlers/actions.rs:160-165`) and has done so since the
  function was created (commit `f3697cc9`, E21CLIHUMCON-008). No behavioral change
  is needed, but nothing locks the scoping: a future refactor could revert to
  global enumeration without a test failing.
- **D4** — there is no test proving the player action menu equals the AI affordance
  set for the same controlled entity and belief state, nor that the menu's labels
  expose no fact the actor could not lawfully perceive or recall.

This ticket added both as focused tests. This is S163 Deliverables 2 and 4.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `handle_cancel` is at `crates/worldwake-cli/src/handlers/actions.rs:153`; it
   `.find`s only `instance.actor == entity` (`:160-165`) and enqueues
   `CancelAction { actor: entity, action_instance_id }` (`:174-178`). Existing inline
   tests: `test_cancel_enqueues_input:502` (cancels the controlled agent's own
   started action), `test_cancel_no_controlled_agent:576` (no controlled agent →
   error). **Neither** asserts the negative case — that with *another* agent's
   action active and the controlled agent idle, `handle_cancel` enqueues nothing.
   That is the D2 gap this ticket fills.
2. The menu is built via `PerAgentBeliefView::with_runtime_from_world` →
   `get_affordances` at `actions.rs:43-46`, then filtered (self-target removal `:49`,
   `HIDDEN_ACTIONS` `:52-57`, dedup `:60`). `control.rs:451 test_switch_new_agent_affordances`
   loosely confirms affordances come from agent context but does **not** assert
   menu==AI symmetry or label lawfulness. The D4 symmetry test must reproduce the
   same view + `get_affordances` call the AI runtime uses and compare the resulting
   affordance set (post the same filters) to the menu's `last_affordances`.
3. Shared boundary under audit: the affordance surface
   `worldwake_sim::get_affordances` over `PerAgentBeliefView` — the single lawful
   source both the player menu and the AI consume. D4 asserts the play surface adds
   nothing beyond it.
4. Layer precision (precision rule 2): D2 is a focused runtime test over the CLI
   input-queue layer (`InputKind::CancelAction` enqueue), not a planner or
   authoritative-action assertion — no action trace needed because no action
   lifecycle transition is under test, only which input the human may enqueue. D4 is
   a focused belief-view/affordance-layer equality assertion, not a golden E2E.
5. Coverage-gap classification (precision rule 3): both gaps are **missing
   focused/unit coverage** in `worldwake-cli`; no runtime-trace or golden E2E gap is
   claimed. Archived S162's belief-wall goldens already cover the belief-view
   lawfulness; D4 composes over that rather than re-proving it.
6. D4 label-lawfulness assertion must align with FND-14A: a co-located physical
   label (item-lot/workstation) is lawful even without a stored belief entry, so the
   assertion is "no label exposes a fact the actor could not lawfully perceive
   (FND-14A) or recall (belief)", not "every label is in the belief store". This
   matches the resolver delivered by archived `archive/tickets/S163CLIPLAPOV-001.md`.

## Architecture Check

1. Both tests prove existing/established contracts at the strongest available
   layer: D2 at the CLI input-enqueue layer (the exact surface the contract lives
   on), D4 at the affordance/belief-view layer (the single lawful source). Neither
   reaches for a downstream golden as a proxy.
2. No production change and no shim: D2 locks already-correct scoping; D4 composes
   over the lawful affordance path and archived `archive/tickets/S163CLIPLAPOV-001.md`'s resolver.

## Verified Layers

1. `handle_cancel` scoping (player may not cancel another agent's action) → focused
   CLI test on the input queue: with another agent's action active and the
   controlled agent idle, `handle_cancel` enqueues no `CancelAction` and references
   no other agent's `action_instance_id`.
2. Player/AI affordance symmetry → focused CLI test asserting the filtered menu
   affordance set equals the `get_affordances` set over the same
   `PerAgentBeliefView` for the same controlled entity and belief state.
3. Label lawfulness → focused CLI test asserting the menu labels (via the
   archived `archive/tickets/S163CLIPLAPOV-001.md` resolver) expose no fact the actor could not lawfully perceive
   (FND-14A) or recall (belief).
4. Single-layer (tests-only) ticket: no decision trace / action trace / event-log
   delta applies because no production code or authoritative state changes; the
   proof surfaces are focused CLI unit tests.

## Landed Changes

### 1. D2 — `handle_cancel` scoping regression guard

Added `test_cancel_ignores_other_agents_active_action`: it builds a two-agent
scenario, inserts an active action for the non-controlled agent, calls
`handle_cancel`, and asserts no `InputKind::CancelAction` is enqueued while the
other agent's active action remains untouched. The test fails if
`handle_cancel` reverts to global enumeration.

### 2. D4 — FND-19 player/AI symmetry test

Added `action_menu_matches_ai_affordances_and_pov_labels`: it builds the same
`PerAgentBeliefView` used by `handle_actions`, calls `get_affordances`, applies the
same self-target, `HIDDEN_ACTIONS`, and dedup filters, and asserts the filtered set
equals the menu's stored `last_affordances`. It also checks the POV label path,
including the remote/unknown case, so the player menu does not expose Bram's
authoritative name through the label resolver.

## Landed Files

- `crates/worldwake-cli/src/handlers/actions.rs` (modified — added both inline
  `#[cfg(test)]` tests and a small test-only helper mirroring the action-menu
  affordance filters)

## Out of Scope

- The POV resolver implementation (D1) — archived `archive/tickets/S163CLIPLAPOV-001.md`.
- The debug-only markers and play-surface boundary guard (D3) —
  `archive/tickets/S163CLIPLAPOV-002.md`.
- Any production behavior change to `handle_cancel` (it is already correctly
  scoped) or `handle_actions`.
- POV-gating or testing the console commands (`world`/`inspect`/`events`/`switch`/
  `observe`) — S163 Non-Goals.

## Acceptance Result

### Passed Criteria

1. D2 guard: with another agent's action active and the controlled agent idle,
   `handle_cancel` enqueues no `CancelAction` and references no other agent's action.
2. D4 symmetry: the filtered player menu affordance set equals the AI
   `get_affordances` set for the same controlled entity and belief state.
3. D4 label lawfulness: a remote/unknown bound-target label resolves to a believed
   label or the "unknown" token, never the authoritative `World` name.
4. Existing suite passed: `cargo test -p worldwake-cli` (including
   `test_cancel_enqueues_input`, `test_cancel_no_controlled_agent`).

### Preserved Invariants

1. A human at the play surface can neither enumerate nor cancel another agent's
   in-flight action. (FND-19.)
2. The player action menu surfaces exactly the lawful affordance set and labels the
   AI would see for the same belief state — no omniscient side channel beyond it.
   (FND-14, FND-14A, FND-19.)

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/handlers/actions.rs` (inline `#[cfg(test)]`) — D2
   negative-scoping regression guard using the existing two-place scenario helper
   plus an inserted active action for the non-controlled agent.
2. `crates/worldwake-cli/src/handlers/actions.rs` (inline `#[cfg(test)]`) — D4
   menu==affordances symmetry test plus the label-lawfulness assertion, reusing the
   view-construction path from `handle_actions`.

### Commands Run

1. `cargo test -p worldwake-cli handlers::actions`
2. `cargo test -p worldwake-cli`
3. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-22.

- Added the D2 negative regression guard proving `handle_cancel` ignores active
  actions owned by other agents and enqueues no cross-agent cancel request.
- Added the D4 player/AI symmetry test proving the action menu stores exactly the
  filtered affordance set returned by the same `PerAgentBeliefView` + `get_affordances`
  path.
- Reused the S163 D1 POV resolver in the D4 label-lawfulness assertion, including
  the unknown remote target case, without changing production behavior.

## Deviations

- No standalone `crates/worldwake-cli/tests/` file was added; the landed proof is
  inline in `actions.rs`, matching the existing handler-test style.
- `scripts/verify.sh` was not run for this ticket iteration; it remains the
  required harness pre-push gate after the S163 family is archived.

## Verification Result

- Passed `cargo test -p worldwake-cli handlers::actions`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
