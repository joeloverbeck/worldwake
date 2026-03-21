# S16S09GOLVAL-006: Decision Trace — Surface Planner Search and Travel-Pruning Provenance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision/planning trace surface
**Deps**: `archive/tickets/completed/S16S09GOLVAL-004.md`, `docs/golden-e2e-testing.md`

## Problem

Current golden coverage can prove that the runtime selected a plan and later reached the durable outcome, but it cannot cleanly explain why one travel route won over other lawful branches in a high-branching topology. This is a traceability gap, not a behavior gap.

For route-sensitive and budget-sensitive goldens, the current decision trace exposes:

- selected goal
- selected plan summary
- selected plan provenance
- search outcome variants
- per-expansion summaries in `PlanSearchTrace`

But the runtime surface does not currently provide a compact, test-friendly summary of:

- total node expansions used by the winning search
- which travel successors were pruned vs retained by spatial pruning
- the heuristic state that made the selected first hop win from a branchy hub

That makes spatial goldens harder to explain and debug than they should be, which is misaligned with the project's explainable-emergence standard.

## Assumption Reassessment (2026-03-21)

1. Runtime decision tracing already exists and is the correct architectural layer. The relevant current symbols are `worldwake_ai::decision_trace::PlanSearchTrace`, `SearchExpansionSummary`, `SelectionTrace`, and `SelectedPlanTrace`, plus the runtime population path in `crates/worldwake-ai/src/agent_tick.rs`.
2. The planner substrate already records some lower-level search detail. `crates/worldwake-ai/src/search.rs` accepts an optional `expansion_summaries` sink and already records `SearchExpansionSummary` entries plus final `BudgetExhausted` / `FrontierExhausted` counts. This ticket is therefore about surfacing missing runtime-planning provenance, not inventing a parallel trace system.
3. The exact missing golden/debugging gap showed up in `archive/tickets/completed/S16S09GOLVAL-004.md`: the new VillageSquare spatial golden could prove `selection.selected_plan.next_step == Travel(SouthGate)` and later Orchard Farm arrival, but not a compact explanation of why that route won or how much default budget was consumed in the live runtime path.
4. This is an AI/runtime traceability ticket, not a candidate-generation or authoritative-validation ticket. The primary verification boundary is runtime `agent_tick` decision tracing, backed by focused unit coverage and one or more goldens that consume the richer trace.
5. Ordering is not the main issue. The contract is explanatory provenance: initial plan shape and search/projection facts, not strict tick ordering. Any later world-state assertion remains downstream confirmation only.
6. Not removing or weakening any heuristic. The current travel pruning in `crates/worldwake-ai/src/search.rs::prune_travel_away_from_goal` and the current heuristic computation are the intended architecture; this ticket should make those lawful planner decisions more legible, not alter them.
7. Not a stale-request, contested-affordance, political, or `ControlSource` ticket.
8. No mismatches in production behavior were found during reassessment. The gap is trace surface completeness and test ergonomics.
9. Existing docs already lean in this direction. `docs/golden-e2e-testing.md` now says route-sensitive goldens should prove `selection.selected_plan.next_step` directly; this ticket should provide the next level of planner provenance so those assertions do not stop at the selected first hop.
10. Scope correction: do not add ad-hoc logging or test-only debug hooks. The right solution is to extend the canonical decision trace types so the same data is available to goldens, focused tests, and human debugging.

## Architecture Check

1. The clean solution is to enrich the existing decision trace schema rather than add separate debug-only output. Worldwake already treats traces as first-class architectural tooling; extending the canonical trace types preserves one authoritative surface for AI reasoning.
2. Search provenance should remain planner-owned but runtime-visible. The search layer should emit structured facts, and `agent_tick` should forward them into the existing `DecisionOutcome::Planning` trace rather than recomputing or guessing them later.
3. No backwards-compatibility shims. Update the trace structs and the tests that consume them directly.

## Verification Layers

1. Search/pruning provenance is captured in the canonical planning trace -> focused unit/runtime tests on `decision_trace.rs`, `search.rs`, and `agent_tick.rs`
2. Selected-plan runtime traces expose budget/pruning summaries for a real AI tick -> runtime `agent_tick` tracing tests
3. Spatial golden can prove both selected route and planner provenance without inferring from arrival alone -> golden E2E assertion in `golden_ai_decisions.rs`
4. Later Orchard Farm arrival remains authoritative downstream confirmation, not proof of planner choice -> authoritative world state in the golden
5. Single-layer note: this ticket does not change authoritative world rules; additional authoritative-layer mapping is not applicable beyond downstream confirmation in existing goldens

## What to Change

### 1. Enrich the canonical planning trace schema

Extend the decision-trace types in `crates/worldwake-ai/src/decision_trace.rs` to carry planner provenance that is currently difficult to access from runtime goldens, such as:

- total expansions used for the selected search
- retained/pruned successor counts for travel-pruning decisions when relevant
- selected-node heuristic or equivalent route-guidance summary

The exact field names should follow the existing trace style and remain compact enough for golden assertions and debug dumps.

### 2. Populate the richer provenance from the runtime path

Update `crates/worldwake-ai/src/agent_tick.rs` and any necessary `search.rs` plumbing so the richer planner provenance is carried into `DecisionOutcome::Planning` for real runtime ticks.

### 3. Add focused coverage for the new trace fields

Add or adjust focused tests in the trace/search/runtime layers to prove:

- the new fields serialize/format cleanly
- they are present for a search-selected plan
- they remain absent or neutral when not applicable

### 4. Upgrade one spatial golden to consume the richer trace

Update the VillageSquare spatial golden to assert at least one of the new planner-provenance fields, so the trace enrichment is exercised through the real golden boundary.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `docs/golden-e2e-testing.md` (modify if the new trace fields warrant a stronger rule/example)

## Out of Scope

- Changing planner correctness, heuristics, or default budget values
- Adding ad-hoc `eprintln!` instrumentation
- Adding a separate parallel debug-trace system outside the canonical decision trace
- Refactoring unrelated golden tests

## Acceptance Criteria

### Tests That Must Pass

1. Focused trace/runtime tests covering the new planner-provenance fields
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
3. `cargo test -p worldwake-ai`

### Invariants

1. Decision traces remain optional and zero-cost when disabled
2. Trace enrichment does not change planner decisions or authoritative world behavior
3. The selected-plan trace remains deterministic and serializable
4. Search/pruning provenance is surfaced through the canonical runtime trace, not through debug-only side channels

## Test Plan

### New/Modified Tests

1. Focused tests in `crates/worldwake-ai/src/decision_trace.rs` and/or `crates/worldwake-ai/src/agent_tick.rs` — prove the new provenance fields are populated and formatted correctly
2. Focused tests in `crates/worldwake-ai/src/search.rs` — prove the search layer emits the required structured pruning/budget facts
3. `golden_spatial_multi_hop_plan` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` — proves the richer runtime trace is usable at the real E2E boundary

### Commands

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
