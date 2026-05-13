# S137PLACAULIN-009: Observer Section 3b — render RepairApplied with rejected alternatives

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (observer-only — read-only consumer of event log + decision trace)
**Deps**: archive/tickets/S137PLACAULIN-003.md (RepairAppliedPayload.substitute_recipe), archive/tickets/S137PLACAULIN-008.md (completed RepairAttemptTrace)

## Problem

S137 D12 extends Observer Section 3b (`render_decision_history_section` at `crates/worldwake-cli/src/bin/observer.rs:828`) to render `EventTag::RepairApplied` events with the new `substitute_recipe` field and the rejected `RepairKind` alternatives surfaced by `RepairAttemptTrace` (ticket 008). Without this, the chosen repair is logged but the "why not the others?" inspection question is unanswerable from observer output alone, regressing FND-29 debuggability.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `render_decision_history_section` lives at `crates/worldwake-cli/src/bin/observer.rs`. Existing format is a table with columns `| Tick | Agent | Event | Payload Summary |`, rendering all decision events via `decision_event_name()` and `decision_payload_summary()`. Before this ticket, `RepairApplied` used only the generic payload summary. `archive/tickets/S137PLACAULIN-003.md` adds `substitute_recipe: Option<RecipeId>` to `RepairAppliedPayload`; ticket 008 adds `RepairAttemptTrace` to `AgentDecisionTrace`.
2. Spec `specs/S137-plan-causal-links-and-repair.md` D12 specifies the new output format (example at the bottom of the spec): `Tick 412 — Agent A — RepairApplied: ReplaceProvider`, then indented lines for breach, substitute_target, substitute_recipe, and rejected alternatives.
3. Shared boundary: the observer's read-only consumer relationship with the event log and decision trace per `references/worldwake-validation-patterns.md` Read-Only Tooling Consumer. Observer reads via public APIs (`scheduler.active_actions()`, decision-trace sink iteration); does not mutate simulation state.
4. **Tooling-only ticket**: this ticket has no engine changes. Per the spec-to-tickets observer-only guidance, Assumption Reassessment items 1-3 are sufficient; items 4-15 do not apply.

## Architecture Check

1. **Read-only consumer**: observer reads `RepairAppliedPayload` (event log) and `RepairAttemptTrace` (decision-trace sink) — both are existing surfaces (extended by tickets 003 and 008). No simulation state mutated.
2. **Format consistency**: the indented multi-line detail format follows the existing decision-history-table convention for events that carry richer context. No format breaking change for events outside `RepairApplied`.

## Verification Layers

1. Format-fidelity → focused unit test in observer (or a sibling test file) constructing a synthetic `RepairAppliedPayload` + `RepairAttemptTrace` and asserting the rendered string matches the expected multi-line format.
2. Single-layer ticket (rendering only); no runtime mutation surface.

## Implemented Change

### 1. Added `RepairApplied`-specific rendering in `render_decision_history_section`

In `crates/worldwake-cli/src/bin/observer.rs`, `EventTag::RepairApplied` now renders table-continuation detail rows in the Section 3b payload column:

```
| <T> | <Agent> | RepairApplied | ... kind=<repair_kind> ... |
  breach: Invalidator::<tag>(target=<entity>) at step <step_index>
  substitute_target: <Option-rendered>
  substitute_recipe: <Option-rendered>
  rejected: <Kind1> (<failure1>), <Kind2> (<failure2>), ...
```

The renderer reads rejected alternatives from the agent's `RepairAttemptTrace` for the event tick and agent, preferring the attempt whose `chosen_kind` and `goal_key` match the `RepairAppliedPayload`.

### 2. Added local rendering helpers

Local observer helpers render optional entity/recipe fields and sort rejected `(RepairKind, RepairFailure)` pairs by `RepairKind` before rendering. No new core `Display` impls were needed.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — `render_decision_history_section` at 828; also covers the existing site at 5477 that was migrated in `archive/tickets/S137PLACAULIN-003.md`)

## Out of Scope

- Engine logic changes — purely a rendering update.
- Other observer sections (1, 2, 4, ...) — unchanged.
- Decision-trace shape — ticket 008 owns `RepairAttemptTrace` shape.
- `EventTag::RepairApplied` payload shape — `archive/tickets/S137PLACAULIN-003.md` owns the `substitute_recipe` field.

## Acceptance Result

### Tests That Passed

1. `cargo test -p worldwake-cli --bin observer tests::render_repair_applied_with_rejected_alternatives -- --exact` asserts the rendered output for a synthetic `RepairApplied` event with a matching `RepairAttemptTrace`.
2. `cargo test -p worldwake-cli --bin observer`
3. `cargo test -p worldwake-cli`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `RepairApplied` events render with the indented multi-line format documented in the spec.
2. Rejected-alternatives line lists `(RepairKind, RepairFailure)` pairs in `RepairKind` Ord order (matches `RepairAttemptTrace` ordering invariant from ticket 008).
3. Observer rendering does not mutate event-log or simulation state — verified by inspection (no `&mut` borrows of authoritative state in the rendering path).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` `#[cfg(test)]` — new test `render_repair_applied_with_rejected_alternatives` constructing a synthetic event + trace and asserting the multi-line output.

### Commands Run

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-13.

- Added Section 3b continuation rows for `RepairApplied` events: breach, substitute target, substitute recipe, and rejected repair alternatives.
- Matched `RepairAppliedPayload` rows to `RepairAttemptTrace` by tick, agent, goal key, and chosen repair kind when traces are available.
- Kept the change observer-only; no engine, event payload, or decision-trace schema changes landed in this ticket.

## Deviations

- The spec example uses a free-form `Tick 412 — Agent A — RepairApplied` block, but the live Section 3b surface is a Markdown table. The implementation uses the existing table-continuation row convention already used for motive-source detail lines.
- `RecipeId` is rendered with its existing debug representation (`RecipeId(7)`) because `RecipeId` does not implement `Display`; adding a core display surface was unnecessary for this observer-only ticket.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer tests::render_repair_applied_with_rejected_alternatives -- --exact`
- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
