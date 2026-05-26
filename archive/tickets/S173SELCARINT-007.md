# S173SELCARINT-007: Scenarios A + B + C — per-family abort traces, contested basin, interrupted release

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None (golden scenarios only)
**Deps**: `archive/tickets/S173SELCARINT-004.md` (wash/toilet contract), `archive/tickets/S173SELCARINT-005.md` (atomic-action abort traces), `archive/tickets/S173SELCARINT-006.md` (emitter filter), `specs/S173-self-care-interruption-occupancy.md` (Scenarios A, B, C)

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

## Verified Layers

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
   - Event log → `EventTag::QueueGrantPromoted` fires within the grant-expiry window (if Agent B is queued); OR Agent A's later candidate emission produces a fresh wash candidate (`emit_wash_goal_produces_one_candidate_per_basin_at_place` semantics).
   - Decision trace → Agent A's replan exposes the abort cause plus the refreshed candidate.

## Landed Changes

### 1. Authored Scenario A golden

Added `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs::golden_self_care_abort_traces_cover_every_family`:

- Human-controlled agents start each of the six self-care action families and cancel before commit.
- Assertions cover per-family `ActionTraceDetail::SelfCareInterrupted` payloads and per-family `EventTag::ActionAborted` event-log entries.
- Eat and Drink retain their item quantities after abort, Sleep emits `SleepEpisodeEnded`, and Wash/Toilet remove `SelfCareOccupancy`.

### 2. Authored Scenario B golden

Added `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs::golden_self_care_contested_basin_promotes_one_occupant`:

- Two dirty agents wait in the same `WashBasin`-tagged facility queue.
- The contention system grants the head claimant through `EventTag::ContentionResolved` and `EventTag::QueueGrantPromoted`.
- The granted claimant starts a real `wash` action and becomes the sole `SelfCareOccupancy` occupant; the queued sibling does not commit wash while the basin is occupied.

### 3. Authored Scenario C golden

Added `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs::golden_interrupted_wash_releases_basin_and_promotes_waiter`:

- Agent A starts wash and writes `SelfCareOccupancy`.
- Agent B waits in the same basin queue.
- Cancelling Agent A's wash emits the typed abort detail, removes `SelfCareOccupancy`, and lets the ordinary post-action contention-system pass promote Agent B in the same tick.

### 4. Updated the golden inventory

Regenerated `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-coverage-matrix.md`, and `docs/generated/golden-scenario-details/survival-self-care-interruption.md` via `python3 scripts/golden_inventory.py --write --check-docs`.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (registered the new module)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `docs/generated/golden-scenario-details/survival-self-care-interruption.md` (new generated detail page)

## Out of Scope

- Player POV symmetry (Scenario D) — landed by `archive/tickets/S173SELCARINT-008.md`.
- Repeated-interruption deprivation collapse (Scenario E) — owned by ticket 009.
- New abort handler or emitter logic — those land in tickets 004, 005, 006.

## Acceptance Result

### Tests That Passed

1. Passed `golden_self_care_abort_traces_cover_every_family` — all six self-care action families abort with correct typed payload.
2. Passed `golden_self_care_contested_basin_promotes_one_occupant` — self-care facility contention grants one claimant and only the granted claimant becomes the wash occupant.
3. Passed `golden_interrupted_wash_releases_basin_and_promotes_waiter` — abort releases occupancy and recovery happens through ordinary grant promotion.
4. Passed existing non-ignored survival-lane selector: `cargo test -p worldwake-ai --test golden_ai survival`.
5. Passed golden inventory regeneration: `python3 scripts/golden_inventory.py --write --check-docs`.

### Verified Invariants

1. The typed trace surface (`ActionTraceDetail::SelfCareInterrupted`) is populated for every self-care abort, distinguishing the six families by `kind`.
2. The authoritative event log (`EventTag::ActionAborted`) is the single causal surface for the abort fact; no parallel `EventTag::SelfCareInterrupted` variant exists in the workspace.
3. Contention resolution through `SelfCareOccupancy` + `PromotableContentionKind` substrate is observable through `EventTag::ContentionResolved` / `EventTag::QueueGrantPromoted`.
4. No silent rescue: an interrupted wash that replans goes through normal candidate emission (no "resume from saved state" or "secret reservation" path).

## Test Plan Result

### Added/Modified Tests

1. One new golden scenario file in `crates/worldwake-ai/tests/scenarios/` with three S173 scenario blocks and tests.
2. `docs/generated/*` regenerated.

### Commands Run

1. Passed `cargo test -p worldwake-ai --test golden_ai self_care_interruption -- --list`.
2. Passed `cargo test -p worldwake-ai --test golden_ai survival_self_care`.
3. Passed `cargo test -p worldwake-ai --test golden_ai survival`.
4. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
5. Passed `python3 scripts/golden_inventory.py --write --check-docs`.
6. Waived per-ticket `./scripts/verify.sh` because this run is inside `$implement-spec-tickets`; the harness final branch phase still owns the full pre-PR gate before push.

## Outcome

Completed on 2026-05-26.

- Added three S173 golden scenario blocks in `survival_self_care_interruption.rs`, registered under `golden_ai`.
- Proved Scenario A through all six typed self-care abort trace details plus `ActionAborted` event emission and family-specific aftermath.
- Proved Scenario B through a self-care wash-basin facility queue promotion, a `ContentionResolved` / `QueueGrantPromoted` payload, and one granted `SelfCareOccupancy` occupant.
- Proved Scenario C through wash abort cleanup, `SelfCareOccupancy` removal in the abort event delta, and ordinary queue promotion for the waiting actor.
- Regenerated golden inventory/docs. The generator also refreshed an already-present `survival_drive_escalation.rs` test count in `golden-e2e-inventory.md`; this was generated-doc freshness fallout, not new S173 behavior.

## Deviations

- The three drafted scenario files landed as one cohesive `survival_self_care_interruption.rs` module because the live golden harness already groups scenario families by module under the single `golden_ai` integration binary.
- Scenario B uses a pre-seeded self-care facility queue to isolate contention promotion and occupancy, rather than relying on autonomous same-tick race timing.
- Scenario C observes same-tick post-abort queue promotion. The live `step_tick` order runs systems after input cancellation, so the released basin can promote the waiting actor in the same tick instead of waiting for the next tick.
- No replay companion tests were added because these are deterministic direct scheduler/action fixtures, not authored long-run autonomous scenarios. The tests assert the action trace, event log, component state, and queue grant state at the causal boundary they create.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai self_care_interruption -- --list`.
- Passed `cargo test -p worldwake-ai --test golden_ai survival_self_care`.
- Passed `cargo test -p worldwake-ai --test golden_ai survival`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Waived `./scripts/verify.sh` for this ticket because `$implement-spec-tickets` owns the final pre-push verification gate for the whole S173 family.
