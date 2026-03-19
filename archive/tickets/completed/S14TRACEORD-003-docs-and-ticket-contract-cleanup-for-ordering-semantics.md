# S14TRACEORD-003: Docs And Ticket Contract Cleanup For Ordering Semantics

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation and ticketing guidance only
**Deps**: `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md`, `archive/tickets/completed/S14TRACEORD-001-explicit-intra-tick-ordering-for-action-traces.md`, `archive/tickets/completed/S14TRACEORD-002-focused-same-tick-cross-agent-ordering-coverage.md`, `docs/FOUNDATIONS.md`, `AGENTS.md`, `tickets/README.md`, `docs/golden-e2e-testing.md`

## Problem

The repo now has the right runtime substrate for same-tick cross-agent ordering, but contributor-facing docs still leave a wording gap. They recommend action traces and warn against incidental tick-boundary assumptions, yet they do not consistently name the explicit ordering key that same-tick cross-agent tests should use. That leaves room for ticket/spec drift toward over-strong phrasing such as “earlier tick” when the real contract is the ordered action trace via `(tick, sequence_in_tick)`.

## Assumption Reassessment (2026-03-19)

1. `tickets/README.md` already requires ordering-sensitive tickets to name the ordering layer and warns against incidental tick-boundary assumptions. The remaining gap is narrower: it does not explicitly call out same-tick cross-agent chains as a case where the contract may be action-trace order via `(tick, sequence_in_tick)` rather than strict tick separation.
2. `docs/golden-e2e-testing.md` already recommends action traces for same-tick actions and explicitly warns against overfitting to tick numbers. What it still lacks is one direct rule for cross-agent same-tick chains: assert on the explicit trace ordering key, not on “later tick” phrasing.
3. `AGENTS.md` already points contributors to `events_for_at(agent, tick)` for same-tick visibility, but that guidance is incomplete for multi-actor ordering. The current trace substrate is not vague append order: `ActionTraceEvent` already carries `sequence_in_tick`, and `archive/tickets/completed/S14TRACEORD-002-focused-same-tick-cross-agent-ordering-coverage.md` added focused runtime coverage proving that `(tick, sequence_in_tick)` is the inspectable ordering contract.
4. `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md` and the live test `crates/worldwake-ai/tests/golden_emergent.rs::golden_same_place_office_fact_still_requires_tell` already demonstrated the concrete failure mode this docs ticket is meant to prevent: same-place cross-agent causality can remain within one tick, so “later tick” was too strong while action-trace order was correct.
5. `archive/tickets/completed/S14TRACEORD-001-explicit-intra-tick-ordering-for-action-traces.md` and `archive/tickets/completed/S14TRACEORD-002-focused-same-tick-cross-agent-ordering-coverage.md` are already complete. This ticket should build on that architecture instead of restating an older pre-ordering-key gap.
6. This remains a documentation-only ticket. No engine or simulation semantics need to change; the goal is to align ticketing and golden guidance with the current trace architecture.
7. The architectural risk is missing precision, not missing substrate. If the docs stay vague here, contributors will continue writing scheduler-coupled assertions against tick numbers even though the engine now exposes a cleaner, explicit ordering surface.

## Architecture Check

1. Clarifying the already-existing runtime contract in docs is cleaner than repeatedly correcting individual tickets after implementation starts.
2. The clean architecture is to have one canonical rule across ticketing, golden guidance, and agent workflow: use the strongest semantic ordering surface available, and for same-tick cross-agent action ordering that surface is the explicit action-trace key `(tick, sequence_in_tick)`, not incidental tick separation.
3. This is more robust than introducing new aliases or compatibility wording such as “usually later tick.” The docs should describe the actual substrate directly.
4. No backward-compatibility layer is relevant. The docs should be updated in place so there is one canonical contract.

## Verification Layers

1. Ticket authoring guidance explicitly distinguishes strict tick separation from same-tick action-trace ordering and points authors at the explicit `(tick, sequence_in_tick)` contract when relevant -> doc review in `tickets/README.md`
2. Golden testing guidance explicitly covers same-tick cross-agent ordering and warns against over-strong later-tick assumptions -> doc review in `docs/golden-e2e-testing.md`
3. Developer debugging guidance for action traces reflects the actual substrate and points readers to `sequence_in_tick` when `events_at()` / `events_for_at()` alone are insufficient -> doc review in `AGENTS.md`
4. The cited runtime and golden contracts still exist and pass at the code layer -> `cargo test -p worldwake-sim`, `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`

## What to Change

### 1. Tighten ticket-authoring guidance

Update `tickets/README.md` so ordering-sensitive tickets must explicitly state when the contract is:
- strict tick separation,
- same-tick action-trace order,
- event-log order,
- or authoritative world-state order.

Also add a warning that same-tick cross-agent chains should not be specified as “later tick” unless that is the actual engine contract, and that the current action-trace ordering surface is the explicit `(tick, sequence_in_tick)` key.

### 2. Tighten golden testing guidance

Update `docs/golden-e2e-testing.md` so same-tick cross-agent chains are called out explicitly. The doc should say that when two actors can act in one tick, the correct contract may be action-trace order rather than tick order, and tests should assert on the explicit `(tick, sequence_in_tick)` trace substrate rather than on incidental tick numbers.

### 3. Tighten agent workflow/debugging guidance

Update `AGENTS.md` so action-trace debugging guidance explains what `events_at()` / `events_for_at()` can and cannot prove, and that same-tick cross-agent ordering should be reasoned about with `ActionTraceEvent.sequence_in_tick`.

## Files to Touch

- `tickets/README.md` (modify)
- `docs/golden-e2e-testing.md` (modify)
- `AGENTS.md` (modify)

## Out of Scope

- Adding engine instrumentation itself
- Adding new focused runtime tests beyond citing the relevant existing and planned coverage
- Changing simulation semantics around same-tick propagation

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
2. Existing suite: `cargo test -p worldwake-sim`
3. Existing suite: `cargo clippy --workspace`

### Invariants

1. Docs and ticket guidance distinguish strict tick separation from same-tick action order.
2. Same-tick cross-agent ordering guidance points contributors to the explicit action-trace key `(tick, sequence_in_tick)` instead of incidental tick numbers or raw vector position.
3. Future tickets are less likely to encode scheduler-coupled later-tick assumptions when the real contract is only causal order.
4. Documentation stays aligned with the foundational principles around local causality, debuggability, and clean system boundaries.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-19
- What actually changed: corrected the ticket assumptions to reflect the already-landed `S14TRACEORD-001` / `S14TRACEORD-002` trace substrate and the existing same-place motivating golden, then updated `tickets/README.md`, `docs/golden-e2e-testing.md`, and `AGENTS.md` so they explicitly point same-tick cross-agent ordering work at the action-trace key `(tick, sequence_in_tick)`.
- Deviations from original plan: no code or test changes were needed because the engine and runtime coverage were already in place; the real gap was narrower than originally stated and limited to contributor-facing wording.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell` ✅
  - `cargo test -p worldwake-sim` ✅
  - `cargo clippy --workspace` ✅
