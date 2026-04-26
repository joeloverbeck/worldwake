# S127QUAAWAACQ-008: Quantity-aware acquisition golden coverage

**Status**: PENDING
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
    - Golden 3 (partial-success): one orchard with `available_quantity = 2`, agent requesting 3 apples; no replenishment in window.
    - Golden 4 (S126 long-horizon): one agent with high hunger and a large carry capacity; alternative-source affordances suppressed; verify `desired_target > 1` in the decision trace.
    - Golden 5 (Scenario E queue abandonment): three agents at one well; one agent has a satchel of water already (so once they perceive the queue and re-evaluate, hunger is satisfied via the satchel and they abandon the queue); next agent in line is granted.

## Architecture Check

1. Goldens are the strongest end-to-end proof surface (FND-29) for emergent multi-tick behaviors that focused tests cannot cover (queue formation across ticks, projected wait-time arithmetic, perception-driven re-planning).
2. Each golden isolates one branch via deliberate scenario authoring (`docs/precision-rules.md` Rule 8) — not by suppressing parts of the engine, but by removing alternative affordances from the world.
3. Scenario E coverage closes a documented FOUNDATIONS regression-class gap (Section VI E) — adding it now means the architecture has a permanent acceptance test for queue-with-abandonment behavior.

## Verification Layers

1. **Golden 1 (single-slot queue)** —
   - queue formation: action trace shows three start attempts, one granted, two enqueued
   - wait-tick projection: decision trace shows the second agent's `wait_estimate_ticks == extraction_duration_ticks * 1`
   - eventual grant: action trace shows the second agent granted after the first's commit
2. **Golden 2 (multi-slot parallel)** — action trace shows three concurrent grants at slot indices 0, 1, 2; no enqueueing
3. **Golden 3 (partial-success)** — `EventTag::Inventory` event-log delta shows item lot of 2 (not 3); action commit trace surfaces `partial_quantity == Some(2)`; `LastHarvestTrace` post-commit contains `partial: true`
4. **Golden 4 (S126 long-horizon)** — decision trace emits `AcquireCommodity` with `desired_target > 1`; agent's post-commit inventory matches `desired_target`
5. **Golden 5 (Scenario E)** — decision trace shows the abandoning agent re-plans away from `AcquireCommodity` (its `is_satisfied` returns true via inventory check); reservation state shows the abandoned slot's `granted` becoming `None`; next agent's start trace shows the slot grant transition

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
