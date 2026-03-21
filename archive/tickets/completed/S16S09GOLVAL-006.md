# S16S09GOLVAL-006: Decision Trace — Surface Planner Search and Travel-Pruning Provenance

**Status**: COMPLETED
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

1. Runtime decision tracing already exists and is the correct architectural layer. The relevant current symbols are `worldwake_ai::decision_trace::PlanSearchTrace`, `SearchExpansionSummary`, `SelectionTrace`, and `SelectedPlanTrace`, populated from `crates/worldwake-ai/src/agent_tick.rs::plan_and_validate_next_step_traced`.
2. The planner substrate already records more search detail than the original ticket implied. `crates/worldwake-ai/src/search.rs::search_plan` already accepts an optional `expansion_summaries` sink, and focused coverage already exists in `crates/worldwake-ai/src/search.rs::search_expansion_summaries_collected_when_tracing_enabled` and `search_expansion_summaries_empty_when_tracing_disabled`.
3. Golden coverage also already exists. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_spatial_multi_hop_plan` and `golden_spatial_multi_hop_plan_replays_deterministically` already prove the live runtime selects `Travel(SouthGate)` at tick 0 and later reaches `OrchardFarm`, harvests, and reduces hunger. The missing gap is not "add a spatial golden"; it is "make the existing golden able to assert compact winning-search provenance".
4. The current mismatch is therefore narrower than the original problem statement: this is not a missing planner trace system and not a missing route-selection golden. It is a canonical trace-surface ergonomics gap. Today, the raw `expansion_summaries` are available on `PlanAttemptTrace`, but the winning search's budget usage and root travel-pruning/heuristic facts are not surfaced in one compact selected-plan-oriented summary.
5. This remains an AI/runtime traceability ticket, not a candidate-generation or authoritative-validation ticket. The primary verification boundary is runtime `agent_tick` decision tracing, backed by focused unit/runtime coverage and a strengthened existing golden in `golden_ai_decisions.rs`. Full action registries remain required because the real scenario crosses planning, travel, harvest, and consumption.
6. Ordering is not the contract. The contract is explanatory provenance for the selected route: initial plan shape plus winning-search facts. Later arrival and hunger relief remain downstream authoritative confirmation only.
7. Not removing or weakening any heuristic. The current travel pruning in `crates/worldwake-ai/src/search.rs::prune_travel_away_from_goal` and the current heuristic computation in `crates/worldwake-ai/src/search.rs::compute_heuristic` are the intended architecture; this ticket should make those lawful planner decisions more legible, not alter them.
8. Not a stale-request, contested-affordance, political, or `ControlSource` ticket.
9. Existing docs already point at the right proof surface. `docs/golden-e2e-testing.md` says route-sensitive goldens should prove `selection.selected_plan.next_step` directly and cites `archive/tickets/completed/S16S09GOLVAL-004.md` as the current example. This ticket should extend that same canonical trace surface instead of adding ad-hoc logging or test-only hooks.
10. Scope correction: enrich the canonical decision trace with a compact winning-search provenance summary and strengthen the existing focused + golden assertions around it. Do not add a parallel debug trace system, and do not duplicate the already-delivered spatial-golden work.

## Architecture Check

1. The clean solution is to enrich the existing canonical decision trace schema rather than force tests to reverse-engineer the winning search from raw `PlanAttemptTrace.expansion_summaries`. Worldwake already treats traces as first-class architectural tooling; one authoritative surface is cleaner than layered debug helpers.
2. Search provenance should remain planner-owned but runtime-visible. The search layer should emit structured heuristic/pruning facts, and `agent_tick` should attach a compact winning-search summary to the selected plan rather than recomputing or guessing it later from plan steps.
3. The ideal shape is a compact selected-plan-oriented summary, not a large dump. Goldens should be able to ask "how many expansions did the winning search use?" and "which first-hop travel successors were retained vs pruned, and what distance signal favored the chosen hop?" without parsing every expansion record.
4. No backwards-compatibility shims. Update the trace structs and the tests that consume them directly.

## Verification Layers

1. Search layer emits structured heuristic/pruning facts for traced expansions -> focused unit tests in `crates/worldwake-ai/src/search.rs`
2. Runtime planning trace attaches compact winning-search provenance to the selected plan -> focused runtime tests in `crates/worldwake-ai/src/agent_tick.rs`
3. Human-readable decision summaries continue to format the richer selected-plan trace cleanly -> focused formatting tests in `crates/worldwake-ai/src/decision_trace.rs`
4. Existing VillageSquare spatial golden proves both selected route and compact planner provenance without inferring from arrival alone -> golden E2E assertions in `crates/worldwake-ai/tests/golden_ai_decisions.rs`
5. Later Orchard Farm arrival / harvest / hunger relief remain authoritative downstream confirmation, not proof of planner choice -> authoritative world state + action trace in the existing golden
6. Single-layer note: this ticket does not change authoritative world rules; no additional authoritative-layer mapping is required beyond the existing downstream confirmation in the strengthened golden

## What to Change

### 1. Enrich the canonical planning trace schema

Extend the decision-trace types in `crates/worldwake-ai/src/decision_trace.rs` to carry planner provenance that is currently difficult to access from runtime goldens, such as:

- total expansions used for the winning selected search
- root travel-pruning facts for the selected path when relevant
- root heuristic / remaining-distance facts or an equivalent route-guidance summary for the chosen first hop

The exact field names should follow the existing trace style and remain compact enough for golden assertions and debug dumps. Raw `PlanAttemptTrace.expansion_summaries` should remain available; the new selected-plan summary should be a compact projection of the winning attempt, not a replacement for the full trace.

### 2. Populate the richer provenance from the runtime path

Update `crates/worldwake-ai/src/agent_tick.rs` and any necessary `search.rs` plumbing so the richer planner provenance is carried into `DecisionOutcome::Planning` for real runtime ticks and attached to the selected plan that actually won selection.

### 3. Add focused coverage for the new trace fields

Add or adjust focused tests in the trace/search/runtime layers to prove:

- the new fields format cleanly in decision summaries
- the search layer records the needed pruning/heuristic facts
- the winning selected plan carries the compact summary for a real traced runtime selection
- the compact summary remains absent or neutral when not applicable

### 4. Strengthen the existing spatial golden to consume the richer trace

Update `golden_spatial_multi_hop_plan` to assert at least one of the new compact planner-provenance fields, so the trace enrichment is exercised through the real golden boundary that already proves the route and downstream outcome.

## Files to Touch

- `tickets/S16S09GOLVAL-006.md` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `docs/golden-e2e-testing.md` (modify only if the new compact summary warrants a stronger route-planning example)

## Out of Scope

- Changing planner correctness, heuristics, or default budget values
- Adding ad-hoc `eprintln!` instrumentation
- Adding a separate parallel debug-trace system outside the canonical decision trace
- Re-adding or duplicating `golden_spatial_multi_hop_plan`
- Refactoring unrelated golden tests

## Acceptance Criteria

### Tests That Must Pass

1. Focused search/runtime/trace tests covering the new planner-provenance fields
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`

### Invariants

1. Decision traces remain optional and zero-cost when disabled
2. Trace enrichment does not change planner decisions or authoritative world behavior
3. The selected-plan trace remains deterministic and serializable
4. Search/pruning provenance is surfaced through the canonical runtime trace, not through debug-only side channels

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — prove the richer selected-plan summary formats cleanly in the canonical decision summary output
2. `crates/worldwake-ai/src/search.rs` — prove the search layer emits the required structured travel-pruning / heuristic facts
3. `crates/worldwake-ai/src/agent_tick.rs` — prove a real traced runtime selection carries the compact winning-search provenance on the selected plan
4. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_spatial_multi_hop_plan` — prove the richer runtime trace is usable at the real E2E boundary that already exercises the VillageSquare route

### Commands

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - reassessed and corrected the ticket scope to match the current codebase: the raw planner trace and the VillageSquare spatial golden already existed, so the implementation focused on a compact winning-search provenance surface on `SelectedPlanTrace`
  - extended canonical decision-trace types with selected-plan search provenance plus structured travel-pruning summaries
  - populated that provenance from the existing search/runtime path without adding a parallel debug surface
  - strengthened focused search/runtime/trace tests and upgraded the existing `golden_spatial_multi_hop_plan` assertions to consume the richer trace
- Deviations from original plan:
  - `docs/golden-e2e-testing.md` did not need changes; the existing guidance was already aligned with the final architecture
  - the focused pruning assertion landed on the deterministic search fixture rather than the live VillageSquare golden because the real runtime scenario does not guarantee a specific pruned branch list
- Verification results:
  - `cargo test -p worldwake-ai prune_travel_keeps_only_toward_goal` passed
  - `cargo test -p worldwake-ai trace_snapshot_continuation_records_selected_plan_provenance` passed
  - `cargo test -p worldwake-ai summary_planning_includes_candidate_count` passed
  - `cargo test -p worldwake-ai golden_spatial_multi_hop_plan` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace` passed
