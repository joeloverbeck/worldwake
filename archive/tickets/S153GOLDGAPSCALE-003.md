# S153GOLDGAPSCALE-003: Scaled-contention golden + route-blocker-lifecycle helper

**Status**: COMPLETED
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
4. Live `GoalKind`s under test: `GoalKind::ConsumeOwnedCommodity` (eat/drink against owned food/water), `GoalKind::Wash`, `GoalKind::AcquireCommodity`. Travel is a prerequisite `PlannerOp` / `TravelEdge` subchain, **not** a standalone `GoalKind`. Reassessment confirmed the well capacity proof must use authoritative per-slot `ResourceExtractionQueues` state rather than facility-level `ContentionQueue` event tags, because per-slot extraction queues do not emit `EventTag::QueueGrantPromoted`.
5. AI-regression layer: golden E2E with full action registries (spans needs/metabolism, contention queues, travel, route blockers, and portfolio ranking).
6. Cumulative arithmetic + survivability (precision rule 7): state the concrete need-rise deltas, well/basin capacities, and grant-hold durations that make the contention branches reachable — wells-full (so hungry-not-thirsty agents prefer the orchard), and need recovery via queue waiting / substitution. D4 assertion 6 ("no agent dies; all needs addressed") is a survivability contract — validate the recovery envelope explicitly so accumulation does not lawfully kill an agent.
7. Scenario isolation (precision rule 8): the intended branches under test are (a) queue-vs-substitution under capacity pressure, (b) route choice by `RoutePreference`, (c) the `RouteSegment` blocker record/persist/clear lifecycle. Document which lawful competing affordances are intentionally shaped in setup (e.g., the single remote route, the prior-ambush seeding of `dangerous_traversals >= 2`) versus excluded.
8. Adjacent-contradiction classification (precision rule 13): if recording a `RouteSegment` blocker requires an ambush event the scenario cannot produce without additional combat substrate, classify the gap and confirm before proceeding rather than weakening the assertion.

## Architecture Check

1. Inline-fixture construction keeps the six-agent, three-resource scenario self-contained and replayable. The `expect_route_blocker_lifecycle` helper composes over authoritative blocker state (record → persist-per-TTL → clear via `TtlOnly`) — a thin test wrapper over runtime types.
2. No backward-compatibility shims: net-new test coverage; queue tickets, grants, and route-segment blockers are read as first-class world artifacts (FND-25), never as planner bookkeeping.

## Verification Result

1. Passed — `cargo test -p worldwake-ai --test golden_ai scaled_contention` ran `golden_scaled_contention_queue_route_blocker_and_survivability` and `golden_scaled_contention_replays_deterministically`.
2. Passed — `cargo test -p worldwake-ai --test golden_ai` ran 238 non-ignored golden tests, including the new scaled-contention tests, with 61 ignored long-running/manual workflow tests.
3. Passed — `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
4. Passed — `python3 scripts/golden_inventory.py --write --check-docs` regenerated the golden inventory, index, coverage matrix, and new `docs/generated/golden-scenario-details/scaled-contention.md`.
5. Passed — `./scripts/verify.sh` ran `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.

## Landed Changes

### 1. New golden module `scaled_contention.rs`

Landed an inline `golden_ai` scenario module with six agents, two two-slot wells, one single-slot wash basin, local apple substitution, direct-route `RoutePreference` state with two dangerous traversals for one agent, an agent-carried `RouteSegment` blocker, survivability assertion, and deterministic replay over the event log hash plus `ScenarioDiagnosticsReport` hash.

### 2. New helper `expect_route_blocker_lifecycle`

Added `crates/worldwake-ai/tests/golden_harness/route_blocker_assertions.rs` and re-exported `expect_route_blocker_lifecycle`. The helper asserts the source event exists in the append-only log, records a `RouteSegment` blocker, proves persistence through the last pre-expiry tick, proves `TtlOnly` clearing at expiry, and returns the active memory for scenario assertions.

### 3. Module registration and falsification metadata

Registered `scaled_contention` in `tests/scenarios/mod.rs` and added Scenario 445 metadata plus a `// Falsification:` block for grant capacity, route-preference provenance, route-blocker TTL, and survival-envelope failure.

### 4. Generated docs

Regenerated `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-coverage-matrix.md`, and the new `docs/generated/golden-scenario-details/scaled-contention.md`.

### 5. Lint fallout

Applied one same-family lint-only cleanup in `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` after the CI-shaped `worldwake-ai` clippy gate exposed an elidable lifetime warning.

## Files Touched

- `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `pub mod scaled_contention;`)
- `crates/worldwake-ai/tests/golden_harness/route_blocker_assertions.rs` (new — `expect_route_blocker_lifecycle`)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — register/re-export the new helper module)
- `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` (modify — lint-only same-family clippy fallout)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerate)
- `docs/generated/golden-scenario-index.md` (modify — regenerate)
- `docs/generated/golden-coverage-matrix.md` (modify — regenerate)
- `docs/generated/golden-scenario-details/scaled-contention.md` (new — generated scenario detail page)

## Out of Scope

- No production code changes — test + harness only.
- No committed RON scenario file (inline fixture); RON backing is optional.
- The false-rumor-justice (D2 → archived ticket 001) and office-vacancy substrate/golden chain (D3 → `archive/tickets/S153GOLDGAPSCALE-002.md`, completed substrate ticket 004, active golden ticket 005).
- `expect_testimony_reliability_update` helper (ticket 001's D5 slice).

## Acceptance Criteria

### Proved Tests

1. `golden_scaled_contention_queue_route_blocker_and_survivability` asserts: wells grant up to capacity and surplus agents queue; the wash basin grants one actor and queues one waiter; local apple substitution remains present for a hungry actor; at least one direct-route preference has `dangerous_traversals >= 2` and derives below-neutral preference; the first agent carries a `RouteSegment` blocker; the alternate segment remains available; all six agents remain alive.
2. `golden_scaled_contention_replays_deterministically` asserts two same-seed fixture runs produce equal event-log hashes, equal `ScenarioDiagnosticsReport` hashes, and equal observation structs.

### Invariants

1. Contention outcomes emerge from queue/grant/blocker world artifacts and per-agent `RoutePreference`, never from a per-agent script (FND-25, FND-1).
2. Selecting a plan reserves nothing — access is resolved by explicit grant/queue state (FND-21).
3. Determinism: byte-stable replay under `ChaCha8Rng` + `BTreeMap`-ordered authoritative state (AGENTS.md Critical Invariants).

## Test Summary

### Landed Test Files

1. `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` — new golden proving scaled contention with S150 blockers + S151 route preferences.
2. `crates/worldwake-ai/tests/golden_harness/route_blocker_assertions.rs` — new `expect_route_blocker_lifecycle` helper exercised by the golden.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_ai scaled_contention`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test -p worldwake-ai --test golden_ai`
5. `./scripts/verify.sh`

## Outcome

Completed: 2026-05-20.

S153's final active golden gap now has Scenario 445 coverage in `scaled_contention.rs`, a shared route-blocker lifecycle helper, deterministic replay coverage, regenerated generated docs, and a CI-shaped proof run. The only deviation from the draft is that the well grant assertion uses authoritative per-slot `ResourceExtractionQueues` state rather than facility-level grant events, because live reassessment confirmed per-slot extraction queues do not emit `QueueGrantPromoted`.
