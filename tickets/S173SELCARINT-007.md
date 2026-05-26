# S173SELCARINT-007: Scenarios A + B + C — per-family abort traces, contested basin, interrupted release

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None (golden scenarios only)
**Deps**: `archive/tickets/S173SELCARINT-004.md` (wash/toilet contract), `archive/tickets/S173SELCARINT-005.md` (atomic-action abort traces), S173SELCARINT-006 (emitter filter), `specs/S173-self-care-interruption-occupancy.md` (Scenarios A, B, C)

## Problem

The behavioral contract from tickets 001-006 needs end-to-end verification through goldens. Three scenarios in the spec's Scenario Validation section cover the standard golden lane: A (per-family abort populates the typed trace detail), B (two co-located dirty agents contend for the same basin — one occupies, the other waits or replans), C (an interrupted wash releases occupancy and either Agent A's replan retries or queued Agent B receives a grant within the configured grant-expiry window). Without these goldens, the cross-system interactions between `SelfCareOccupancy` lifecycle, `PromotableContentionKind` queue classification, `ActionTraceDetail::SelfCareInterrupted` payloads, and the existing `EventTag::ActionAborted` / `ContentionResolved` / `QueueGrantPromoted` events are unproven in composition.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Live `GoalKind` surfaces under test:
   - Scenario A exercises five action families — the goal-family mapping is `eat` (commodity-consumption flow, no dedicated GoalKind for the abort path), `drink` (commodity-consumption), `sleep` (`GoalKind::Sleep`), `toilet` (`GoalKind::Relieve`), `wash` (`GoalKind::Wash`), `relieve_wilderness` (`GoalKind::Relieve`).
   - Scenario B exercises `GoalKind::Wash` with both agents.
   - Scenario C exercises `GoalKind::Wash` with one agent + interruption (predator presence or higher-priority self-care).
2. Existing golden infrastructure for self-care: `crates/worldwake-ai/tests/scenarios/survival_*.rs` is the survival-family golden lane. The new scenarios slot into this lane following the existing file structure (one `.rs` file per scenario, with a `.ron` companion if scenario data is externalized). Verify the canonical golden naming convention at implementation time via `docs/golden-e2e-testing.md` and `docs/generated/golden-scenario-index.md`.
3. Authoritative event-log assertions for Scenario A: each abort fires `EventTag::ActionAborted` exactly once per action; `ActionTraceDetail::SelfCareInterrupted` payload is captured in the action-trace sink with the correct `kind` discriminator. Sleep additionally fires `EventTag::SleepEpisodeEnded` (preserved by ticket 005). For the occupancy-bearing wash and toilet aborts, `SelfCareOccupancy` is removed by abort time.
4. Scenario B contention: two agents start at the same `WashBasin`-tagged `Facility` with one `clean_water_units` budget. Action-trace ordering key `(tick, sequence_in_tick)` per `docs/precision-rules.md` Rule 14 — only one agent's wash action commits in the same tick; the other either joins the contention queue (`S44` substrate; `EventTag::ContentionResolved` fires) or replans. Both branches are lawful per the spec's Scenario B assertions.
5. Scenario C interruption: Agent A starts wash; before commit, an interrupting event fires (hostile predator entering co-location, or a higher-priority self-care emerging). The abort handler from ticket 004 (`abort_release_self_care_occupancy`) removes `SelfCareOccupancy`; the action engine fires `EventTag::ActionAborted`; the `tick_step` trace mapper from ticket 005 populates trace detail. Then either Agent B (if queued) receives a grant via `EventTag::QueueGrantPromoted`, or Agent A re-emits a wash candidate next tick targeting the same (now free) basin.
6. Shared abstraction boundary across the three scenarios: the action-trace sink + the event log + authoritative world state (component presence/absence). Each scenario maps a distinct invariant to a distinct proof surface per `docs/precision-rules.md` Rule 5.
7. Scenario isolation per Rule 8: Scenario B must isolate the contention-resolution branch from a lawfully-competing branch where one of the agents simply has a lower-priority goal and never reaches wash. The setup forces both agents to dirty-need-priority on the same tick. Document the isolation choice in the scenario file.
8. Cumulative arithmetic per Rule 7: Scenario A's per-family abort fires once per action; no accumulation. Scenario B's contention resolution is a single-tick race; no accumulation. Scenario C's interruption fires once and the recovery path may or may not complete within the scenario's tick budget — verify the scenario's tick budget accommodates one full retry cycle.

## Architecture Check

1. Three scenarios in one ticket are cohesive because they exercise the same infrastructure (`SelfCareOccupancy`, `ActionTraceDetail::SelfCareInterrupted`, `PromotableContentionKind` queue classification) at distinct invariant boundaries. Splitting them across three tickets would add review overhead without test-independence benefit.
2. The scenarios use only existing world-modeling primitives (Facility + WashBasin tag, Place + Latrine tag, two-agent setup, hostile predator from existing combat infrastructure for the interruption source in Scenario C). No new scenario-authoring primitives are introduced.
3. Per FND-31, each scenario declares both a positive invariant (the expected behavior) and a negative case (the forbidden alternative — silent rescue, planner-intent lock, parallel use of the same basin). The negative cases are surfaced in scenario assertions.

## Verification Layers

1. Scenario A: per-action-family abort trace fires correctly.
   - Action-trace `detail` → typed `ActionTraceDetail::SelfCareInterrupted` payload with correct `kind` and `basin` per family.
   - Event log → `EventTag::ActionAborted` fires once per action; `EventTag::SleepEpisodeEnded` fires for the sleep abort.
   - Authoritative world state → no `SelfCareOccupancy` remains after abort (for wash, toilet).
2. Scenario B: contested basin resolves through one occupant + queue-or-replan for the other.
   - Action lifecycle ordering (`(tick, sequence_in_tick)`) → exactly one wash action commits on the basin in any single tick.
   - Authoritative world state → `SelfCareOccupancy` present on the basin during occupancy.
   - Event log → `EventTag::ContentionResolved` fires (S142 substrate).
   - Decision trace → Agent B's emitter filters or revalidation rejects (depending on which branch the scenario exercises).
3. Scenario C: interrupted wash releases basin; recovery happens through normal channels.
   - Action trace → Agent A's abort detail populated; abort tick equals interruption tick.
   - Authoritative world state → occupancy removed by abort.
   - Event log → `EventTag::QueueGrantPromoted` fires within the grant-expiry window (if Agent B is queued); OR Agent A's next-tick candidate emission produces a fresh wash candidate (`emit_wash_goal_produces_one_candidate_per_basin_at_place` semantics).
   - Decision trace → Agent A's replan exposes the abort cause + the new candidate.

## What to Change

### 1. Author Scenario A golden

Create `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption_traces.rs` (or per the canonical naming convention in `docs/generated/golden-scenario-index.md` — verify at implementation time):

- One agent set up with all six self-care interruption families exercised through the trace surface.
- Scenario controller forces the agent to start each action family in sequence, then interrupts each before commit.
- Assertions cover per-family `ActionTraceDetail::SelfCareInterrupted` payloads and per-family `EventTag::ActionAborted` event-log entries.

### 2. Author Scenario B golden

Create `crates/worldwake-ai/tests/scenarios/survival_self_care_contested_basin.rs`:

- Two dirty agents co-located at the same `WashBasin`-tagged `Facility`.
- Both attempt wash same tick.
- Assertions cover the action-lifecycle ordering (exactly one commits) and the contention-substrate event chain (`EventTag::ContentionResolved`).

### 3. Author Scenario C golden

Create `crates/worldwake-ai/tests/scenarios/survival_self_care_interrupted_release.rs`:

- Agent A wash setup; interruption source (hostile predator, or a higher-priority commitment emerging) fires before commit.
- Optional Agent B queued or co-present.
- Assertions cover the abort trace + occupancy-release + recovery path (queue grant or next-tick re-emission).

### 4. Update the golden inventory

Regenerate `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/` via `python3 scripts/golden_inventory.py --write --check-docs`.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption_traces.rs` (new — Scenario A)
- `crates/worldwake-ai/tests/scenarios/survival_self_care_contested_basin.rs` (new — Scenario B)
- `crates/worldwake-ai/tests/scenarios/survival_self_care_interrupted_release.rs` (new — Scenario C)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/` (regenerated subdirectory)
- Likely: a parent `tests/scenarios/mod.rs` or `tests/lib.rs` that registers new modules — verify at implementation time per the existing test-binary layout.

## Out of Scope

- Player POV symmetry (Scenario D) — owned by ticket 008.
- Repeated-interruption deprivation collapse (Scenario E) — owned by ticket 009.
- New abort handler or emitter logic — those land in tickets 004, 005, 006.

## Acceptance Criteria

### Tests That Must Pass

1. New golden: `survival_self_care_interruption_traces` — all six self-care action families abort with correct typed payload.
2. New golden: `survival_self_care_contested_basin` — exactly one wash commits per tick; the other agent's path is lawful (queued OR replanned).
3. New golden: `survival_self_care_interrupted_release` — abort releases occupancy; recovery happens through normal channels (grant promotion OR re-emission).
4. Existing survival-lane goldens pass: `cargo test -p worldwake-ai --test golden_ai survival`.
5. Golden inventory regenerates cleanly: `python3 scripts/golden_inventory.py --write --check-docs` exits 0.

### Invariants

1. The typed trace surface (`ActionTraceDetail::SelfCareInterrupted`) is populated for every self-care abort, distinguishing the six families by `kind`.
2. The authoritative event log (`EventTag::ActionAborted`) is the single causal surface for the abort fact; no parallel `EventTag::SelfCareInterrupted` variant exists in the workspace.
3. Contention resolution through `SelfCareOccupancy` + `PromotableContentionKind` substrate is observable through `EventTag::ContentionResolved` / `EventTag::QueueGrantPromoted`.
4. No silent rescue: an interrupted wash that replans goes through normal candidate emission (no "resume from saved state" or "secret reservation" path).

## Test Plan

### New/Modified Tests

1. Three new golden scenario files in `crates/worldwake-ai/tests/scenarios/` (see Files to Touch).
2. `docs/generated/*` regenerated.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai survival_self_care`
2. `cargo test -p worldwake-ai --test golden_ai survival` (full survival lane)
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `./scripts/verify.sh` before commit.
