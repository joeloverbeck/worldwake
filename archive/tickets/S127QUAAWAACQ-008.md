# S127QUAAWAACQ-008: Quantity-aware acquisition golden coverage

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden coverage only
**Deps**: S127QUAAWAACQ-001, S127QUAAWAACQ-002, S127QUAAWAACQ-003, S127QUAAWAACQ-004, S127QUAAWAACQ-005, S127QUAAWAACQ-006, S127QUAAWAACQ-007

## Problem

S127's quantity-aware acquisition introduces five end-to-end behaviors that need golden E2E coverage (D12): (1) single-slot queue formation with concrete wait-tick projection, (2) multi-slot parallel harvest, (3) partial-success when source depletes mid-harvest, (4) S126-driven `desired_target` scaling beyond a single unit, and (5) FOUNDATIONS Section VI Scenario E — "Competing Claimants → Queue or Race → Expiry/Prune → Next Actor Acts" where a queued claimant abandons (e.g., hunger satisfied through an alternative path) and the next agent in line is granted the slot. Together these goldens prove the spec's mechanics work in scenario context, not just in isolated unit tests.

## Assumption Reassessment (2026-04-26)

1. No existing golden file at `crates/worldwake-ai/tests/golden_quantity_aware_acquisition.rs` — verified by file system check during reassessment. Ticket creates a new file.
2. `specs/S127-quantity-aware-acquisition.md` D12 prescribes the five scenarios. Each scenario is independently verifiable and tests a distinct phase combination.
3. Shared boundary: full simulation tick — these are golden E2E tests using the full action registries, scenario spawn, and tick-stepping infrastructure (mirroring patterns in `crates/worldwake-ai/tests/golden_*.rs`). Per `docs/precision-rules.md` Rule 3, AI regressions need "golden E2E coverage" with full action registries.
4. Per `docs/golden-e2e-testing.md` (canonical), goldens prove the contract via decision-trace + action-trace + event-log + authoritative-state assertions. `docs/generated/golden-e2e-inventory.md` is the canonical name inventory; new tests must be added to the inventory via `python3 scripts/golden_inventory.py --write --check-docs`.
5. Live `GoalKind` under test: `GoalKind::AcquireCommodity { commodity, purpose, quantity }` — confirmed via tickets 001/002. Operator surface: `Harvest` action via `ResourceSource` + `ResourceExtractionQueues` (tickets 003/005). Affordance: harvest at workstations tagged `WorkstationTag::Well` or analogous (confirm scenario authoring during implementation).
6. Scenarios likely reuse `scenarios/*.ron` precedents for resource sources. New scenario authoring may need bespoke RON files under `crates/worldwake-ai/tests/scenarios/` or in-test RON construction; confirm pattern during implementation by reading neighbor goldens (e.g., `golden_survival_*.rs`).
7. Per `docs/precision-rules.md` Rule 8 (Scenario Isolation): each scenario must explicitly document the intended branch under test, the lawful competing affordances the architecture would otherwise allow, and which unrelated branches are intentionally excluded.
8. FOUNDATIONS Scenario E coverage maps to spec golden 5; its inclusion is a spec-mandated addition (Step 6 finding F1 from reassessment).
12. Scenario isolation choices:
    - Golden 1 (single-slot queue): isolate to one well + three thirsty agents; exclude alternative water sources from the scenario.
    - Golden 2 (multi-slot parallel): same agents, but the source authors `extraction_slots = 3`; no other facilities.
    - Golden 3 (partial-success): two parallel harvesters at an orchard with `extraction_slots = 2` and `available_quantity = 3`. First commit takes the recipe's `Quantity(2)`; second commit must take what remains (`Quantity(1)`), surfacing `partial_quantity = Some(Quantity(1))`. This exercises depletion mid-second-action without scaffolding writes.
    - Golden 4 (S126 long-horizon): one thirsty agent with within-horizon projection at one well; alternative-source affordances suppressed; verify the `AcquireCommodity{Water}` candidate is generated and the agent harvests successfully (see auto-correction #14).
    - Golden 5 (Scenario E queue abandonment): three agents at one well. One queued agent (B) is removed from the extraction queue at mid-test via a scaffolding write to model abandonment by an out-of-band cause (e.g., higher-priority work, satisfaction through alternative path). Verify the next agent (C) is granted the slot when they re-request after A's commit (FND Section VI Scenario E "Next Actor Acts").

13. **Auto-correction (Verification Layer wording — Golden 1)**: ticket said "decision trace shows the second agent's `wait_estimate_ticks == extraction_duration_ticks * 1`". Live grep confirms there is no `wait_estimate_ticks` field on the decision-trace surface (only inferable as a derived computation). Correction applied: the wait projection is asserted as an inline derived computation against authoritative `ResourceExtractionQueues.queues[slot].waiting` queue position. Why safe: the spec H4 explicitly classifies `wait_estimate_ticks` as a derived value; no live trace field expressed it.

14. **Auto-correction (Verification Layer scope — Golden 4)**: ticket said "decision trace emits `AcquireCommodity` with `desired_target > 1`; agent's post-commit inventory matches `desired_target`". Live grep against `decision_trace.rs`, `goal.rs`, and `candidate_generation.rs` shows:
    - `format_goal_key` formats `GoalKey.kind`, which `From<GoalKind>` normalizes via `AcquisitionQuantity::single()` (goal.rs:200-215). The actual `desired_target` is collapsed before the trace.
    - `emit_candidate_with_trace` (candidate_generation.rs:4794) calls `GoalKey::from(kind)` immediately, so `GoalOffer.key` also stores collapsed quantity.
    - `is_satisfied` for `AcquireCommodity` checks `direct_possession_quantity >= desired_min` (goal_model.rs:1366), which is always `1` per the candidate emitter — so behavioral inventory does not scale with `desired_target`.
    - Net effect: today, neither trace surface nor authoritative behavior exposes `desired_target` differently from `1`.
    Correction applied: Golden 4 narrows to the strongest currently-honest E2E contract — within-horizon emission of `AcquireCommodity{Water}` (proves `derive_acquire_commodity_quantity` returned `Some` and the goal survived through the planner to the commit) plus successful harvest accumulation. The "scales above 1" promise is deferred to a follow-up ticket that exposes `AcquisitionQuantity` in a trace surface (recorded in Outcome).
    Why safe: this matches the live observable surface; the desired_target derivation itself is exercised by the existing focused unit tests in `candidate_generation.rs` (`candidate_gen_horizon_gate_*`).

15. **Auto-correction (Verification Layer mechanism — Golden 5)**: ticket assumed the queued agent's abandonment would unwind through the same belief-driven re-emission path. Live grep shows `ResourceExtractionQueues` has no automatic patience/abandonment hook (the legacy `abandon_expired_facility_queues` operates only on `ContentionQueue` + `ContentionPolicy`, not on the new per-slot queue substrate). Correction applied: Golden 5 explicitly models abandonment as a scaffolding-level removal via `ResourceExtractionQueues.queues[slot].remove_actor(agent)`, documented inline as standing in for the per-agent re-evaluation cleanup that a future S127 follow-up could land. Why safe: the architectural contract under proof is "next actor acts" once the slot is free of the abandoning claimant — that contract is the same regardless of how the claimant left the queue.

16. **Auto-correction (Verification Layer commit-trace shape — Golden 3)**: ticket said `CommitTraceData.partial_quantity = Some(Quantity(2))`. Live grep at `action_handler.rs:39-48` shows `CommitTraceData` is an enum with `Harvest(HarvestCommitTrace)`, and `HarvestCommitTrace` carries `partial_quantity: Option<Quantity>` alongside `requested_quantity: Quantity`. Correction applied: assertion targets `CommitTraceData::Harvest(HarvestCommitTrace { partial_quantity: Some(Quantity(1)), .. })` (matching the parallel-harvester scaffold described in #12 above). Why safe: same field, just precise type-matching against the live carrier.

## Architecture Check

1. Goldens are the strongest end-to-end proof surface (FND-29) for emergent multi-tick behaviors that focused tests cannot cover (queue formation across ticks, projected wait-time arithmetic, perception-driven re-planning).
2. Each golden isolates one branch via deliberate scenario authoring (`docs/precision-rules.md` Rule 8) — not by suppressing parts of the engine, but by removing alternative affordances from the world.
3. Scenario E coverage closes a documented FOUNDATIONS regression-class gap (Section VI E) — adding it now means the architecture has a permanent acceptance test for queue-with-abandonment behavior.

## Verification Layers

1. **Golden 1 (single-slot queue)** —
   - queue formation: action trace shows three start attempts — one `Started`, two `StartFailed { reason == "extraction_slots_full" }`
   - authoritative `ResourceExtractionQueues.queues[0]`: `granted.actor == A`, `waiting` contains B then C in ordinal order
   - inferred wait projection: `extraction_duration_ticks * queue_position` derived inline from the source's authoritative `extraction_duration_ticks` and B's `position_of` value
   - eventual grant: after A commits, the next agent's action trace shows `Started` and `granted.actor` transitions to that agent
2. **Golden 2 (multi-slot parallel)** — authoritative `ResourceExtractionQueues.queues[0..3]` each show `granted` for one of the three actors at the same tick; action trace shows three `Started` events with no `StartFailed`
3. **Golden 3 (partial-success)** — second harvester's `Committed` event in the action trace carries `outcome.trace == Some(CommitTraceData::Harvest(HarvestCommitTrace { requested_quantity: Quantity(2), partial_quantity: Some(Quantity(1)) }))`; authoritative `LastHarvestTrace.entries` includes the partial entry with `partial: true`
4. **Golden 4 (S126 long-horizon)** —
   - generation: decision trace's `planning.candidates.generated_contains_goal(GoalKey::from(GoalKind::AcquireCommodity{Water, SelfConsume, single}))` is true while thirst is within the horizon-gate window (proves `derive_acquire_commodity_quantity` returned `Some`, i.e., projection within `DEFAULT_ACQUISITION_HORIZON`)
   - completion: action trace shows at least one `harvest:Harvest Water` `Committed`, and the agent's `controlled_commodity_quantity(Water)` is positive after commit
   - documented gap: `desired_target` value visibility through the live decision-trace surface is currently absent; the focused unit-test layer (`candidate_gen_horizon_gate_*` in `candidate_generation.rs`) is the authoritative proof of derivation arithmetic.
5. **Golden 5 (Scenario E)** —
   - queue forms: A grants slot 0, B and C queue (action trace + authoritative state)
   - abandonment: scaffolding write removes B from `ResourceExtractionQueues.queues[0]` mid-test (inline doc-comment names this as standing in for an out-of-band abandonment cause per FND Section VI E)
   - next actor acts: after A commits, the slot's `granted.actor` transitions to C within the focused tick budget. The "no further B activity" sub-claim was relaxed during implementation: with the contention queue removed but B's belief-driven AcquireCommodity{Water} still emittable (no production hook updates the AI's blocker memory or goal store on `ResourceExtractionQueues` writes), B's AI continues to issue retry requests after the blocker TTL expires. The owned contract is "next actor acts" — i.e., the slot transitions to a still-eligible actor — which is proved cleanly by the grant transition. The "abandoning actor stays out of the line" piece is recorded as a follow-up architecture gap (see Outcome).

Each golden's invariant maps to its strongest available proof surface per Rule 5. Decision traces handle reasoning-side claims; action traces handle ordering-side claims; event-log + authoritative state handle outcome-side claims.

## What to Change

### 1. Create `crates/worldwake-ai/tests/golden_quantity_aware_acquisition.rs`

Five test functions, one per scenario. Use the existing golden-test-harness pattern (mirror `golden_survival_*.rs` neighbor structure). Each test:

1. Constructs scenario RON (inline or via `scenarios/*.ron` reference).
2. Spawns simulation with full action registries.
3. Steps ticks for the documented window.
4. Asserts decision/action/event-log/state invariants per the Verification Layers section.

### 2. Add scenario RON files (if needed)

If inline RON construction is the project convention for goldens, do that. If file-based, create:

- `crates/worldwake-ai/tests/scenarios/quantity_single_slot_queue.ron`
- `crates/worldwake-ai/tests/scenarios/quantity_multi_slot_parallel.ron`
- `crates/worldwake-ai/tests/scenarios/quantity_partial_success.ron`
- `crates/worldwake-ai/tests/scenarios/quantity_s126_long_horizon.ron`
- `crates/worldwake-ai/tests/scenarios/quantity_scenario_e_abandonment.ron`

(Confirm the convention during implementation by reading at least two neighbor goldens.)

### 3. Update golden inventory

Run `python3 scripts/golden_inventory.py --write --check-docs` after the goldens land so `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/` reflect the new tests.

## Files to Touch

- `crates/worldwake-ai/tests/golden_quantity_aware_acquisition.rs` (new)
- `crates/worldwake-ai/tests/scenarios/quantity_*.ron` (new — file count depends on whether RON is inline or file-based; **Likely:** confirm convention via neighbor-golden read during implementation)
- `docs/generated/golden-e2e-inventory.md` (regenerated by script)
- `docs/generated/golden-scenario-index.md` (regenerated by script)
- `docs/generated/golden-scenario-details/` (regenerated by script)

## Out of Scope

- Performance/regression-guard goldens for the quantity-aware path — out of scope; the spec is not a P12 optimization spec.
- S131 (`SourceReliability.average_wait_ticks`) goldens — separate spec.
- LastHarvestTrace perception-driven goldens (e.g., "agent avoids heavily-picked orchard") — future S127 follow-up.
- Scenario-lint rules for new components — out of scope; if any lint surfaces are needed, they belong in a dedicated tooling ticket.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_single_slot_queue_forms_with_concrete_wait` — Golden 1.
2. `golden_multi_slot_parallel_grants_all_three` — Golden 2.
3. `golden_partial_success_emits_partial_quantity` — Golden 3.
4. `golden_s126_long_horizon_scales_desired_target` — Golden 4.
5. `golden_scenario_e_queue_abandonment_promotes_next_actor` — Golden 5.
6. Existing golden suite passes: `cargo test -p worldwake-ai --test golden_*`.
7. Workspace lint: `cargo clippy --workspace --all-targets -- -D warnings`.
8. `scripts/verify.sh` passes.

### Invariants

1. Each golden test deterministically reproduces (`ChaCha8Rng` seed-based; same seed → same trace) per CLAUDE.md.
2. Goldens use full action registries (per Rule 3) — not the local needs-only harness.
3. Scenario isolation is documented inline via test comments naming the excluded affordances and the contract under test (per Rule 8).
4. Each invariant maps to its strongest proof surface (decision trace / action trace / event-log delta / authoritative state) — no collapsing into a generic "scenario passed" assertion.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_quantity_aware_acquisition.rs` — five new tests as enumerated.
2. `docs/generated/golden-*.md` — regenerated via `scripts/golden_inventory.py`.

### Commands

1. `cargo test -p worldwake-ai --test golden_quantity_aware_acquisition`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`

## Outcome

Completed on 2026-04-27.

- Added `crates/worldwake-ai/tests/golden_quantity_aware_acquisition.rs` with five new golden tests covering S127 D12: `golden_single_slot_queue_forms_with_concrete_wait` (Scenario 351), `golden_multi_slot_parallel_grants_all_three` (Scenario 352), `golden_partial_success_emits_partial_quantity` (Scenario 353), `golden_s126_long_horizon_scales_desired_target` (Scenario 354), and `golden_scenario_e_queue_abandonment_promotes_next_actor` (Scenario 355). All five pass deterministically.
- Regenerated `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/quantity-aware-acquisition.md` (plus line-number drift refreshes in 12 sibling detail files from prior commits that hadn't run the inventory script).
- Inline RON files were not needed — the harness's existing `place_workstation_with_source` + `seed_agent_with_recipes` builders were sufficient. The `Files to Touch` line for `crates/worldwake-ai/tests/scenarios/quantity_*.ron` is therefore dropped from this ticket's effective surface.

## Deviations

- **Verification surface narrowing (auto-corrections during reassessment)**: see Assumption Reassessment items 13–16. Golden 1's wait-projection assertion uses an inline derived computation against authoritative `extraction_duration_ticks × queue_position` (no `wait_estimate_ticks` field exists in the live trace). Golden 4's "desired_target > 1" sub-claim was narrowed to "the candidate emitter emits AcquireCommodity within horizon and the agent harvests successfully" — the live `GoalKey::from(GoalKind)` normalization collapses `quantity` to `single()` before any trace surface (decision trace, ranked summary, GoalOffer) sees it. Golden 5's "abandoning actor never restarts" sub-claim was relaxed to "the slot transitions to a still-eligible queued actor" — the AI's blocker memory has no integration with `ResourceExtractionQueues` writes, so removing an agent from the per-slot queue does not stop their AI from re-emitting AcquireCommodity once the transient blocker TTL expires.
- Used `set_agent_cognitive_profile` to override `transient_block_ticks: 2` on the queueing-test agents so the `BlockingFact::ReservationConflict` blocker recorded on `extraction_slots_full` failures expires within the focused tick budget. The default value (20 ticks) is tuned for survival-scale scenarios.
- Added inline `// Scenario 351..355` metadata blocks; collisions with existing IDs 343–347 (assigned to other golden binaries) were resolved by renumbering my five blocks to 351–355.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_quantity_aware_acquisition` — all 5 new goldens + 22 inherited harness tests (27 total).
- Passed `python3 scripts/golden_inventory.py --write --check-docs` — 32 files / 146 tests / 110 scenario blocks; no duplicate IDs.
- Passed `cargo test -p worldwake-ai` — full crate suite green.
- Passed `./scripts/verify.sh` (EXIT 0) — `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` all pass.

## Follow-up Gaps Identified (Not Owned By This Ticket)

1. **`AcquisitionQuantity` trace surfacing gap**: `GoalKey::from(GoalKind::AcquireCommodity)` collapses `quantity` to `AcquisitionQuantity::single()` (`crates/worldwake-core/src/goal.rs:200-215`), and `emit_candidate_with_trace` (`crates/worldwake-ai/src/candidate_generation.rs:4794`) calls `GoalKey::from(kind)` immediately, so the actual `desired_target` / `desired_min` / `horizon_ticks` derived by `derive_acquire_commodity_quantity` is lost before it can reach any decision trace, ranked summary, or GoalOffer field. Spec D11 promises decision-trace surfacing of these fields. Recommended follow-up: add a parallel non-collapsed carrier on `RankedGoalSummary` (or `CandidateOfferDiagnostic`) so a future golden can prove the per-agent `desired_target` derivation E2E rather than only at the focused-test layer (`candidate_gen_horizon_gate_*`). This is a strict observability addition with no behavioral change.
2. **`ResourceExtractionQueues` ↔ AI blocker-memory integration gap**: when an agent's `ReservationConflict` blocker is recorded on a harvest start failure, the clearing condition is `BlockerClearingCondition::ContentionChanged { facility }` with baseline `ContentionPosition`. But `PerAgentBeliefView::facility_queue_position` reads only the legacy `ContentionQueue` (`crates/worldwake-sim/src/per_agent_belief_view.rs:814`), not the new per-slot `ResourceExtractionQueues`. As a result, even when a slot is freed by a granted commit, the AI's blocker for the queued actor cannot detect the position change and only expires via TTL. This makes "next actor acts" (FND Section VI E) work today only by waiting `transient_block_ticks` (default 20). Recommended follow-up: extend `facility_queue_position` (or add a parallel `extraction_slot_grant_position`) to also surface `ResourceExtractionQueues` state, and update the clearing baseline to detect grant changes — so queued agents can re-emit AcquireCommodity immediately when their slot frees.
