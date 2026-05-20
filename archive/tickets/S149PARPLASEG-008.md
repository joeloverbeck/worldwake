# S149PARPLASEG-008: Observer barrier rendering

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — observer agenda/diagnostics rendering (read-only tooling)
**Deps**: archive/tickets/S149PARPLASEG-001.md, archive/tickets/S149PARPLASEG-003.md, archive/tickets/S149PARPLASEG-005.md, archive/tickets/S149PARPLASEG-010.md

## Problem

D9 makes barriers debuggable: the observer renders, per suspended partial-plan segment, the typed terminal, the barrier fact, the resume condition, and the abandon status. This makes "what stopped this plan, and what would un-stop it?" answerable from the observer dump (FND-29).

## Assumption Reassessment (2026-05-20)

1. The spec's original "Observer Section 7 (planning)" is wrong — Section 7 is "End-State Inventory & Resources" (`crates/worldwake-cli/src/bin/observer.rs:4975`). Section 13 "Scenario Diagnostics" carries `terminal_kind_distribution`; the live per-segment runtime surface is Section 8's suspended agenda summary. Section header convention: `## Section <N> — <Title>\n`.
2. `terminal_kind_distribution` is keyed by `PlanTerminalKindDiscriminant` after ticket 001 (`scenario_diagnostics/mod.rs:43`); Section 13 already aggregates it, so the seven typed kinds appear there with no observer change beyond label text. The new work is per-attempt barrier rendering (terminal + barrier fact + resume/abandon), which reads `PartialPlanSegment` fields (tickets 002/003) populated by ticket 010 and lifecycle-updated by ticket 005.
3. Shared boundary under audit: the observer's agenda/diagnostics formatting and the read-only `PartialPlanSegment` surface. This is observer-only tooling — no engine/simulation-state mutation; items 4–15 of the template are inapplicable.
4. Output-format fidelity: new lines must follow the existing section's formatting (indented sub-lines under a `Plan terminal:` header, matching the example in the spec). Render the resume condition as the derived `IntentionResumeCondition` (e.g. `BeliefStatusChanged(...)`), not the spec's original `BeliefUpdated` text.

## Architecture Check

1. Rendering reads `PartialPlanSegment` and the discriminant-keyed distribution through existing public surfaces — a read-only consumer, no new accessor or authoritative state (FND-27/FND-29).
2. Placing the per-attempt barrier detail under the existing Section 8 suspended agenda summary avoids a new section and keeps the dump's structure stable. Section 13 remains the aggregate diagnostics surface for typed terminal distribution.

## Verified Layers

1. Barrier detail renders for a suspended attempt → `format_report_renders_agenda_state_summary` now asserts the `Plan terminal: <typed>` block with barrier fact + resume/abandon lines appears under the suspended agenda entry.
2. Distribution shows typed discriminants → `render_scenario_diagnostics_section_text_lists_typed_terminal_discriminants` asserts Section 13 lists all seven `PlanTerminalKindDiscriminant` labels. Tooling-only ticket: headless render tests are the proof surface; no action-trace/event-log mapping applies.

## Landed Changes

### 1. Per-attempt barrier rendering

The observer now renders per-suspended-entry partial-plan details in the per-agent agenda summary: `Plan terminal: <typed terminal>`, `Barrier fact: <BarrierFact>`, `Resume on: <IntentionResumeCondition>`, and `Abandon if: <abandon condition> (<attempt count> resume attempts used)`.

### 2. Distribution label text

The Section 13 `terminal_kind_distribution` sample coverage now asserts all seven discriminant labels render through the existing metric-map path.

## Landed Files

- `crates/worldwake-cli/src/bin/observer.rs` (modified) — partial-plan barrier rendering helpers, suspended-entry agenda rendering, and render tests.

## Out of Scope

- Any change to the typed-terminal taxonomy or discriminant (ticket 001).
- Producing/storing segments (tickets 002/003/010).
- Golden E2E coverage (ticket 009).

## Acceptance Result

### Tests Passed

1. Passed: a headless observer render of a suspended partial-plan segment shows the `Plan terminal:` block with barrier fact, resume condition, and abandon status.
2. Passed: Section 13 renders the typed discriminant kinds in `terminal_kind_distribution`.
3. Passed: `cargo test -p worldwake-cli`

### Invariants

1. Preserved: the observer reads `PartialPlanSegment`/distribution through existing public surfaces; it introduces no new authoritative state and mutates nothing.
2. Preserved: the observer keeps existing section headers intact and adds indented sub-lines under suspended agenda entries.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline render tests) — barrier-detail block + distribution labels.

### Commands Run

1. Passed `cargo test -p worldwake-cli`
2. Waived `scripts/verify.sh` for this ticket iteration because the owned diff is confined to observer rendering/tests and the harness reserves full pre-push verification for final spec-family closeout.

## Outcome

Completed on 2026-05-20.

- Added observer formatting for suspended partial-plan segments: typed terminal, barrier fact, resume condition, and abandon condition with the segment's used resume-attempt count.
- Extended the observer's Section 13 diagnostics test fixture to include every typed terminal discriminant and asserted all seven labels render.
- Kept the change read-only: no planner, runtime, authoritative state, or scenario semantics changed.

## Deviations

- The per-attempt barrier block landed under Section 8's per-agent suspended agenda summary, because the live `PartialPlanSegment` is stored on suspended `AgendaEntry` runtime state. Section 13 remains the aggregate diagnostics surface for `terminal_kind_distribution`.
- The observer cannot derive "remaining attempts left" from `PartialPlanSegment` alone because the segment stores `resume_attempt_count` but not the owning patience limit. The rendered abandon line therefore reports resume attempts used, which is the strongest truthful read-only observer surface for this ticket.

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-cli`
