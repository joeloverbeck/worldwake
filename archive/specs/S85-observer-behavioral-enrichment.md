# S85: Observer Behavioral Enrichment

**Status**: COMPLETED

## Summary

Enrich the observer binary (`crates/worldwake-cli/src/bin/observer.rs`) with five diagnostic enhancements that improve the ability to diagnose agent behavioral pathologies in simulation runs. These address specific diagnostic gaps identified in the simulation observer report: missing death tick/cause display, missing frontier-exhaustion rejection reasons, missing need snapshots at behavioral transitions, missing post-travel affordance snapshots, and confusing "Unknown location" display for place entities.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Crates

- `worldwake-cli` (observer binary)

## Dependencies

- S78 (Observer Failed-Plan Diagnostics) — completed (provides the base diagnostic infrastructure this spec extends)
- S81 (Golden Gaps — Simulation Remediation) — completed (provides `DeadAt { tick, cause: DeathCause }` component this spec surfaces)
- S77 (Belief Capacity Prioritization) — completed (provides `believed_kind` field on `BelievedEntityState` used in Deliverable 5)

## Design Goals

- Every enhancement is observer-only — no changes to simulation runtime, AI, or core systems
- Each enhancement addresses a specific diagnostic question that cannot currently be answered from observer output
- Output additions are concise and actionable — they aid diagnosis, not clutter the report
- All five enhancements are independent and can be implemented in any order

## Non-Goals

- Modifying simulation behavior or AI decision-making
- Adding new components or systems to the engine
- Interactive observer features or live dashboards
- Observer performance optimization (these are small additions to an already-bounded output)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-29 (Debuggability) | All five enhancements directly serve debuggability: surfacing death causes, explaining plan failures, correlating needs with behavior changes, showing affordance evolution, and clarifying location semantics |
| FND-10 (Aftermath) | Death tick and cause display makes mortality aftermath visible in observer output |

## Deliverables

### 1. Death Tick and Cause Display (from TK-7)

**Diagnostic question**: "When exactly did this agent die and why?"

**Current state**: S81 implemented `DeadAt { tick: Tick, cause: DeathCause }` component and `EventTag::Death` events. The observer counts `dead_ticks` in the Decision Trace Summary tick breakdown but does not query the `DeadAt` component for the exact tick and cause.

**Change**: In the per-agent summary section ("Section 2 — Per-Agent Summary" in observer.rs), query `world.get_component_dead_at(agent_id)`. If present, emit at the top of the agent's section:

```
**Death**: Tick 1438 (cause: NeedDeprivation { Hunger })
```

Format `DeathCause` variants as:
- `DeathCause::NeedDeprivation { need }` → `"NeedDeprivation { {need:?} }"`
- `DeathCause::CombatWounds` → `"CombatWounds"`

### 2. Frontier-Exhaustion Rejection Reasons (from TK-4)

**Diagnostic question**: "Why did no operators match for this frontier-exhausted goal?"

**Current state**: S78 added frontier-exhausted counting and failed-plan tables with `Max Depth`, `Candidates`, `Location`, `Had Target Beliefs`. But when a plan frontier-exhausts at depth 0, the output shows only `"frontier-exhausted (1 expansion, 0 depth)"` with no explanation of why operators didn't match.

**Change**: When a `PlanSearchOutcome::FrontierExhausted` has `expansions_used <= 1` (depth-0 exhaustion), extract from the `expansion_summaries` in `PlanAttemptTrace`:

- `candidates_generated` at depth 0
- `candidates_skipped` at depth 0

Emit a human-readable reason:

```
frontier-exhausted at depth 0: 0 candidates generated (0 target entities matched TargetSpec)
```

or:

```
frontier-exhausted at depth 0: 3 candidates generated, all pruned by beam
```

The `expansion_summaries` field of `PlanAttemptTrace` already contains `SearchExpansionSummary` with `candidates_generated`, `candidates_skipped`, `terminal_successors`, and `non_terminal_after_beam` per depth (among other fields — the struct has 14 fields total; only these 4 are relevant here). The observer needs to read and format these for depth-0 frontier-exhaustion cases.

### 3. Need Snapshots at Behavioral Transitions (from TK-5)

**Diagnostic question**: "What were the agent's needs when their behavior narrowed?"

**Current state**: The observer samples needs every tick (the `NeedsSample` struct defined near the top of observer.rs) and reports min/max/average in the per-agent summary. It also computes per-agent action type counts per 100-tick time bin in the "Per-Agent Action Timeline" section. But it does not correlate the two.

**Change**: After computing per-agent action timelines (action type counts per 100-tick bin), detect behavioral transitions: consecutive bins where the action type count drops by 50% or more. At each detected transition, emit a snapshot:

```
**Behavioral transition** at tick 500: action repertoire narrowed (5 types → 2 types)
  Needs: hunger=750, thirst=800, fatigue=200, bladder=100, dirtiness=500
```

Place these in the per-agent summary, after the needs trajectory and before the anomaly flags.

### 4. Affordance Snapshots After Travel (from TK-6)

**Diagnostic question**: "What affordances did the agent have after arriving at a new location?"

**Current state**: Affordances are captured from the first planning decision that has them (effectively tick 0 or near-0, in the "Affordances available" rendering section of the per-agent decision trace). Agents that travel have different affordances at their new location, but these are never shown.

**Change**: In addition to the initial affordance snapshot, capture affordances from the first planning decision after each `travel` action commits. Store as `Vec<(Tick, AffordanceTrace)>` in `AgentStats`, where `AffordanceTrace` (from `decision_trace.rs`) bundles `available: Vec<AffordanceSummary>` with `place: Option<EntityId>`. In the per-agent summary, emit:

```
**Affordances after travel** (tick 340, arrived at Thornwall Village):
  harvest(Water, Well), pick_up(Bread), sell(Ore), ...
```

Also emit end-of-simulation affordances (from the last planning decision with affordances) in the per-agent summary:

```
**Final affordances** (tick 1400):
  sleep, relieve, ...
```

### 5. "Unknown Location" Clarity (from TK-2)

**Diagnostic question**: "Why does this agent believe a place's location is 'Unknown location'?"

**Current state**: The "Believed entity locations" rendering section of observer.rs renders `last_known_place: None` as `"Unknown location"`. For place entities this is expected behavior — places don't have a "location of a location." But the display is confusing.

**Change**: When rendering entity beliefs, distinguish between entity kinds:

- For `EntityKind::Place`: Render as `"(place entity — no parent location)"` instead of `"Unknown location"`
- For other entity kinds: Keep `"Unknown location"` as-is (this is a genuine belief gap)

This requires checking the believed entity kind (available from `believed_kind` field added by S77) when formatting the location field.

## Section H: Causal Hooks (FND-01)

N/A — observer-only changes. No simulation state is created, modified, or consumed. The observer reads existing state and trace data in a read-only post-simulation pass.

## SystemFn Integration

No new SystemFn. All changes are in the observer binary's report formatting.

## Component Registration

No new components. The observer reads existing components (`DeadAt`, `HomeostaticNeeds`, affordance data from `PlanAttemptTrace`).

## Cross-System Interactions

None — the observer is a read-only analysis tool that runs after simulation completes. It has no runtime interactions with simulation systems.

## Outcome

Completed on 2026-04-10.

- Landed all five observer-only enrichments in `crates/worldwake-cli/src/bin/observer.rs` across tickets `S85OBSBEHENR-001` through `S85OBSBEHENR-005`: death tick/cause display, depth-0 frontier-exhaustion reason formatting, behavioral-transition need snapshots, post-travel/final affordance snapshots, and clearer unknown-location rendering for believed place entities.
- Verification passed during the ticket sequence via repeated `cargo test -p worldwake-cli` and `cargo clippy --workspace --all-targets -- -D warnings` runs after each bounded observer change.
- The affordance-follow-up slice landed as observer report-time derivation from existing decision and action traces rather than persistent `AgentStats` storage, preserving the same delivered diagnostic contract without adding duplicate trace-derived state.
