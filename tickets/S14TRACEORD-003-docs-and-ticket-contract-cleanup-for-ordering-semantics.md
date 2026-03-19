# S14TRACEORD-003: Docs And Ticket Contract Cleanup For Ordering Semantics

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation and ticketing guidance only
**Deps**: `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md`, `tickets/S14TRACEORD-001-explicit-intra-tick-ordering-for-action-traces.md`, `docs/FOUNDATIONS.md`, `AGENTS.md`, `tickets/README.md`, `docs/golden-e2e-testing.md`

## Problem

Current docs correctly say to use action traces for same-tick lifecycle questions, but they do not explicitly tell authors to distinguish tick separation from causal order. The result is ticket/spec drift toward over-strong timing assumptions such as “earlier tick” when the real engine contract is only action-trace order. That makes tickets less accurate and goldens more brittle.

## Assumption Reassessment (2026-03-19)

1. `tickets/README.md` already requires ordering-sensitive tickets to name the ordering layer, but it does not explicitly warn against assuming tick separation for same-tick cross-agent chains.
2. `docs/golden-e2e-testing.md` already warns against incidental tick-boundary assumptions and recommends action traces for same-tick actions, but it does not yet spell out the specific distinction between tick ordering and intra-tick causal ordering across actors.
3. `AGENTS.md` currently tells developers to inspect `events_for_at(agent, tick)` for same-tick action visibility, but it does not explain that relative order among multiple same-tick events may require explicit ordering metadata or append order depending on the trace substrate.
4. The completed ticket `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md` had to be corrected mid-implementation because its original same-place ordering assumption was too strong. That is exactly the kind of ticket-authoring drift this docs ticket should prevent.
5. This ticket is documentation-only. No engine or simulation semantics need to change here; the goal is to make future specs and tickets align with the real architecture and its trace surfaces.
6. The architectural risk is not missing behavior but missing precision. Imprecise docs push contributors toward brittle tests, scheduler-coupled assertions, and confusion about what the system actually guarantees.

## Architecture Check

1. Clarifying ordering contracts in docs is cleaner than repeatedly patching individual tickets after implementation begins.
2. This ticket should document the canonical rule: use the strongest semantic ordering surface available, and do not assume tick separation unless the engine explicitly specifies it.
3. No backward-compatibility layer is relevant. The docs should be updated in place so there is one canonical contract.

## Verification Layers

1. Ticket authoring guidance explicitly distinguishes tick separation, action-trace order, event-log order, and authoritative state order -> doc review in `tickets/README.md`
2. Golden testing guidance explicitly covers same-tick cross-agent ordering and warns against over-strong later-tick assumptions -> doc review in `docs/golden-e2e-testing.md`
3. Developer debugging guidance for action traces reflects the actual substrate and points readers to the correct query/ordering surfaces -> doc review in `AGENTS.md`
4. No additional runtime verification layer is applicable because this is a documentation-only ticket; existing runtime/golden coverage should be cited instead of duplicated

## What to Change

### 1. Tighten ticket-authoring guidance

Update `tickets/README.md` so ordering-sensitive tickets must explicitly state when the contract is:
- strict tick separation,
- same-tick action-trace order,
- event-log order,
- or authoritative world-state order.

Also add a warning that same-tick cross-agent chains should not be specified as “later tick” unless that is the actual engine contract.

### 2. Tighten golden testing guidance

Update `docs/golden-e2e-testing.md` so same-tick cross-agent chains are called out explicitly. The doc should say that when two actors can act in one tick, the correct contract may be action-trace order rather than tick order, and tests should assert on the explicit trace substrate rather than on incidental tick numbers.

### 3. Tighten agent workflow/debugging guidance

Update `AGENTS.md` so action-trace debugging guidance explains what `events_at()` / `events_for_at()` can and cannot prove, and how contributors should reason about same-tick cross-agent ordering.

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

1. Docs and ticket guidance distinguish same-tick action order from tick separation.
2. Future tickets are less likely to encode scheduler-coupled later-tick assumptions when the real contract is only causal order.
3. Documentation stays aligned with the foundational principles around local causality, debuggability, and clean system boundaries.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
