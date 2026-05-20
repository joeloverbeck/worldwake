# S153GOLDGAPSCALE-003: Scaled-contention golden + route-blocker-lifecycle helper

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: None (substrate is archived: S140 contention queues/grants, S150 route-segment blockers, S151 route preferences, S148 portfolio slots)

## Problem

S153 D4 calls for a golden proving cross-goal blocker scoping (S150) and route preferences (S151) under realistic resource pressure: six agents share three capacity-bounded resources, outcomes emerge from queue contention + route preferences + route-segment blockers with no per-agent script. `survival-contested.ron` exists but does not exercise S150 `RouteSegment` blockers or S151 `RoutePreference` state. This ticket adds that regression plus the shared `expect_route_blocker_lifecycle` harness helper (D5 slice), the determinism rerun (D6 slice), and the falsification comment (D7 slice).

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Substrate confirmed against current code: `ContentionGrant` and `ContentionQueue` (`crates/worldwake-core/src/contention.rs:43` / `:10`). `BlockerScope::RouteSegment` (`crates/worldwake-core/src/blocker_scope.rs`). `BlockerClearingCondition::TtlOnly` (`crates/worldwake-core/src/blocker_memory.rs:176`). `RoutePreferenceEntry.dangerous_traversals` (`crates/worldwake-core/src/route_preference.rs`). `GoalKind::ConsumeOwnedCommodity` (`crates/worldwake-core/src/goal.rs:63`), `GoalKind::AcquireCommodity` (`:66`), `GoalKind::Wash` (`:73`). `scenarios/survival-contested.ron` exists (4 agents; water modeled as a `ResourceSource` of commodity `Water` at a `Well` workstation, capacity 4). No existing `golden_scaled_contention*` test.
2. Spec reference: `specs/S153-golden-gaps-ai-architecture-scaling.md` D4 (post-reassessment — target module `crates/worldwake-ai/tests/scenarios/scaled_contention.rs`, run via `golden_ai`).
3. Shared boundary under audit: authoritative contention queue/grant state (core/systems) and route-segment blocker state (S150) are read by the AI portfolio ranking + route-choice layer. The golden audits AI slot/route-choice reacting to authoritative contention + blocker state — it modifies neither layer.
4. Live `GoalKind`s under test: `GoalKind::ConsumeOwnedCommodity` (eat/drink against owned food/water), `GoalKind::Wash`, `GoalKind::AcquireCommodity`. Travel is a prerequisite `PlannerOp` / `TravelEdge` subchain, **not** a standalone `GoalKind` — do not assert a `TravelTo` goal. **Confirm the well contention substrate before asserting grant events:** facility-level `ContentionQueue` emits `EventTag::QueueGrantPromoted`, whereas per-slot `ResourceExtractionQueues` (`crates/worldwake-systems/src/production_actions.rs`) does **not**. Key any grant-event assertion to whichever substrate the well actually uses.
5. AI-regression layer: golden E2E with full action registries (spans needs/metabolism, contention queues, travel, route blockers, and portfolio ranking).
6. Cumulative arithmetic + survivability (precision rule 7): state the concrete need-rise deltas, well/basin capacities, and grant-hold durations that make the contention branches reachable — wells-full (so hungry-not-thirsty agents prefer the orchard), and need recovery via queue waiting / substitution. D4 assertion 6 ("no agent dies; all needs addressed") is a survivability contract — validate the recovery envelope explicitly so accumulation does not lawfully kill an agent.
7. Scenario isolation (precision rule 8): the intended branches under test are (a) queue-vs-substitution under capacity pressure, (b) route choice by `RoutePreference`, (c) the `RouteSegment` blocker record/persist/clear lifecycle. Document which lawful competing affordances are intentionally shaped in setup (e.g., the single remote route, the prior-ambush seeding of `dangerous_traversals >= 2`) versus excluded.
8. Adjacent-contradiction classification (precision rule 13): if recording a `RouteSegment` blocker requires an ambush event the scenario cannot produce without additional combat substrate, classify the gap and confirm before proceeding rather than weakening the assertion.

## Architecture Check

1. Inline-fixture construction keeps the six-agent, three-resource scenario self-contained and replayable. The `expect_route_blocker_lifecycle` helper composes over authoritative blocker state (record → persist-per-TTL → clear via `TtlOnly`) — a thin test wrapper over runtime types.
2. No backward-compatibility shims: net-new test coverage; queue tickets, grants, and route-segment blockers are read as first-class world artifacts (FND-25), never as planner bookkeeping.

## Verification Layers

1. Wells grant up to capacity; surplus agents queue -> authoritative world state (`ContentionQueue`/`ContentionGrant`) + event-log delta.
2. Wells-full → hungry-not-thirsty agents substitute the orchard over waiting -> decision trace (slot ranking `EconomicOpportunity` vs `NeedSurvival`).
3. Negative-`RoutePreference` agents detour/wait while neutral/positive agents use the remote route -> decision trace (route choice) + authoritative `RoutePreferenceEntry` state.
4. The `RouteSegment` blocker is recorded, persists per TTL, and clears via `BlockerClearingCondition::TtlOnly` -> event-log delta + authoritative blocker state, asserted via the new `expect_route_blocker_lifecycle` helper.
5. No agent dies; hunger/thirst/dirtiness needs are addressed -> authoritative world state (`HomeostaticNeeds`; absence of `DeadAt`).
6. Determinism (D6): two same-seed runs produce a byte-identical event log AND an equal `ScenarioDiagnosticsReport`.

## What to Change

### 1. New golden module `scaled_contention.rs`

Inline fixture: six agents, two wells (capacity 2 each), one wash basin (capacity 1) at a central hub, all with hunger/dirtiness/thirst rising; a single remote route to a source, pre-seeded as previously ambushed (`RoutePreferenceEntry.dangerous_traversals >= 2` for ≥1 agent). Run ticks and assert D4 assertions 1–6 per the Verification Layers above. Pin need/capacity/grant arithmetic per Assumption Reassessment item 6 and the well substrate per item 4 before writing assertions.

### 2. New helper `expect_route_blocker_lifecycle`

Add `expect_route_blocker_lifecycle(segment, observation_event, ttl)` to the golden harness — asserts blocker recording (the observation event present), persistence across the TTL window, and clearing via `BlockerClearingCondition::TtlOnly`.

### 3. Register the module and add the falsification comment

Add `pub mod scaled_contention;` to `tests/scenarios/mod.rs`. Add a `// Falsification:` comment block (D7): e.g., "If an agent re-uses the blocked remote route before its `RouteSegment` blocker TTL expires, S150 blocker scoping failed; if an agent dies under the contention load, the survivability envelope is wrong."

### 4. Determinism rerun (D6)

Run twice at the same seed; assert byte-identical event log and equal `ScenarioDiagnosticsReport`.

### 5. Regenerate golden-inventory docs

Run `python3 scripts/golden_inventory.py --write --check-docs` and commit the regenerated inventory.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `pub mod scaled_contention;`)
- `crates/worldwake-ai/tests/golden_harness/route_blocker_assertions.rs` (new — `expect_route_blocker_lifecycle`)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — register/re-export the new helper module)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerate)
- `docs/generated/golden-scenario-index.md` (modify — regenerate)
- `To be confirmed:` `docs/generated/golden-scenario-details/<scaled_contention>.md` (regenerate output path created by `scripts/golden_inventory.py`; confirm exact filename after running the generator)

## Out of Scope

- No production code changes — test + harness only.
- No committed RON scenario file (inline fixture); RON backing is optional.
- The false-rumor-justice (D2 → archived ticket 001) and office-vacancy substrate/golden chain (D3 → `archive/tickets/S153GOLDGAPSCALE-002.md`, completed substrate ticket 004, active golden ticket 005).
- `expect_testimony_reliability_update` helper (ticket 001's D5 slice).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_scaled_contention_*` passes, asserting: wells grant up to capacity and surplus agents queue; wells-full → orchard substitution by hungry-not-thirsty agents; route choice by `RoutePreference` (negative-preference agents detour/wait); `RouteSegment` blocker recorded, persists, and clears via `TtlOnly` (via `expect_route_blocker_lifecycle`); no agent dies and all needs are addressed.
2. Determinism: two same-seed runs produce a byte-identical event log and an equal `ScenarioDiagnosticsReport`.
3. Existing suite: `cargo test -p worldwake-ai --test golden_ai`
4. Golden-inventory consistency: `python3 scripts/golden_inventory.py --check-docs`

### Invariants

1. Contention outcomes emerge from queue/grant/blocker world artifacts and per-agent `RoutePreference`, never from a per-agent script (FND-25, FND-1).
2. Selecting a plan reserves nothing — access is resolved by explicit grant/queue state (FND-21).
3. Determinism: byte-stable replay under `ChaCha8Rng` + `BTreeMap`-ordered authoritative state (AGENTS.md Critical Invariants).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` — new golden proving scaled contention with S150 blockers + S151 route preferences.
2. `crates/worldwake-ai/tests/golden_harness/route_blocker_assertions.rs` — new `expect_route_blocker_lifecycle` helper exercised by the golden.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai scaled_contention`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `scripts/verify.sh`
