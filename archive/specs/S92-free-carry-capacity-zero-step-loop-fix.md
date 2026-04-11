# S92: FreeCarryCapacity Zero-Step Loop Fix

## Summary

Fix the `FreeCarryCapacity` planner pathology reproduced by Scenario 143 in `crates/worldwake-ai/tests/golden_planner_pathology.rs`: Forager Lina repeatedly selects `FreeCarryCapacity`, the planner returns `GoalSatisfied[steps=0]`, no executable disposal action commits, and urgent self-care goals are starved until hunger rises. This is not a generic "planner should dislike zero-step plans" issue. The live bug is a contract mismatch between `emit_disposal_candidates()` in `candidate_generation.rs`, `motive_score()` in `ranking.rs`, `GoalKind::FreeCarryCapacity.is_satisfied()` in `goal_model.rs`, and the `GoalSatisfied` fast path in `search::transition::terminal_kind()`.

This spec unifies `FreeCarryCapacity` around one lawful disposal contract: if the candidate is emitted because the actor is materially strained by directly possessed Waste, the root planning state must not already count as success. Success must require disposal progress through the existing `drop_item` action path from S82, not a planner-local no-op. The existing golden regression then flips from failure proof to fix proof.

**Evidence**: Scenario 143 (`degenerate_zero_step_loop_blocks_actionable_goals`) uses the exact `scenarios/cli-evaluation.ron` Eldergrove/Forager Lina substrate, seed `7777`, and a late-run observation window after real waste accumulation. The current proof shows repeated `PlanSearchOutcome::Found { steps: [] }` for `FreeCarryCapacity`, no late `eat` commit, and rising hunger during the loop window.

**Phase**: 7 (Adjunct — Planner Pathology Remediation)
**Status**: COMPLETED
**Crates**: `worldwake-ai`
**Dependencies**:
- `archive/specs/S82-waste-disposal-inventory-management.md`
- `archive/specs/S91-planner-pathology-golden-tests.md`
- `archive/tickets/S91PLAPATGOL-002.md`

## Design Goals

- Eliminate the root `FreeCarryCapacity -> GoalSatisfied[steps=0]` loop reproduced by Scenario 143.
- Define one shared `FreeCarryCapacity` actionability/satisfaction contract reused by candidate emission, ranking, and goal satisfaction.
- Preserve S82's physical disposal model: freeing capacity still happens only by lawful world actions such as `drop_item`.
- Convert the Scenario 143 golden from a bug reproduction into a durable proof that Lina breaks the loop and returns to actionable self-care behavior.

## Non-Goals

- Adding a planner-wide suppression rule for all zero-step terminal plans.
- Changing unrelated S91 pathologies (`budget_exhaustion_blocks_cross_location_water_acquisition` or `role_agent_generates_survival_goals_under_critical_needs`).
- Introducing a new waste-destruction, trash-can, or cleanup system beyond the existing S82 disposal mechanics.
- Rebalancing hunger, metabolism, or utility weights as a substitute for fixing the `FreeCarryCapacity` contract.

## FOUNDATIONS Alignment

| Principle | How This Spec Satisfies It |
|-----------|----------------------------|
| FND-3 (Concrete State Over Abstract Scores) | The fix keys off real carried load, carry capacity, directly possessed Waste lots, and disposal progress through `drop_item`, not a detached planner score or special-case suppression bit. |
| FND-5 (Simulate Carriers of Consequence) | Waste remains a real consequence carrier that occupies inventory until physically dropped. The spec fixes the planner contract around that carrier instead of bypassing it. |
| FND-11 (Physical Dampeners for Positive Feedback) | The runaway loop is damped by an existing physical process: dropping Waste reduces carried load and unblocks other goals. No invisible cap or "don't pick this again" cooldown is introduced. |
| FND-14 (World State Is Not Belief State) | All planning-side checks remain on the planning snapshot / belief-backed surfaces already used by `worldwake-ai`; no omniscient authoritative query is introduced. |
| FND-20 (Resource-Bounded Practical Reasoning) | `FreeCarryCapacity` must represent a real actionable world condition. The fix prevents bounded reasoning from wasting every tick on a self-satisfied no-op. |
| FND-21 (Intentions Are Revisable Commitments) | Once disposal lowers strain below the actionable threshold, the goal stops demanding attention and the agent can lawfully return to eating, drinking, or other urgent work. |
| FND-28 (No Backward Compatibility Layers) | The old mismatched satisfaction path is replaced, not wrapped by a compatibility flag or special-case post-filter. |

## Evidence

The current live code exposes four separate `FreeCarryCapacity` contracts:

1. `candidate_generation::emit_disposal_candidates()` decides when disposal candidates exist and what item kinds qualify.
2. `ranking::motive_score()` computes `FreeCarryCapacity` pressure from carried load.
3. `goal_model::GoalKind::FreeCarryCapacity.is_satisfied()` declares whether the goal is already complete in the planning state.
4. `search::transition::terminal_kind()` immediately returns `PlanTerminalKind::GoalSatisfied` when `is_satisfied()` is true at the current node.

The divergence is compounded by two distinct load-computation methods: the shared `GoalBeliefView`-side helper now reconstructs carried load from concrete direct possessions, while `carried_load_of_actor()` in `goal_model.rs` computes `capacity - remaining_carry_capacity` via recursive BFS over directly possessed items in `PlanningState`. Before the final S92 fix, planner snapshot rebuilds could still undercount directly possessed lot quantities by falling back to stale `last_known_inventory` inside `PerAgentBeliefView::commodity_quantity()`, which let the same carried substrate appear materially different across the reasoning pipeline. Lina's scenario-defined `disposal_profile` has `capacity_strain_threshold: 700` (not the default 800), per `scenarios/cli-evaluation.ron:281`.

Scenario 143 proves those contracts diverge badly enough that Lina can be "strained enough to emit and rank disposal" while also being "already satisfied" at the planning root, which yields a 0-step `Found` result and blocks more urgent self-care goals.

The existing `golden_waste_disposal_cycle` in `crates/worldwake-ai/tests/golden_production.rs` already demonstrates the desired lower-layer contract: when `FreeCarryCapacity` is actually actionable, the selected plan exposes `PlannerOpKind::DropItem`, a `drop_item` action commits, and the goal stops recurring after load is reduced.

## Deliverables

### D1: Introduce a shared FreeCarryCapacity contract helper

**Files**: `crates/worldwake-ai/src/goal_model.rs`, `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`, or a new shared planner-local helper module under `crates/worldwake-ai/src/`

Create a single helper that encodes the canonical actionable/satisfaction contract for `FreeCarryCapacity`. The helper must accept pre-computed contract values rather than a specific data surface, because the three call sites operate on different trait surfaces: candidate emission and ranking use `GoalBeliefView` (via `GenerationContext.view` / `RankingContext.view`), while goal satisfaction uses `PlanningState`. The helper's inputs are at minimum:

- current carried load (`LoadUnits`)
- carry capacity (`LoadUnits`)
- active disposal threshold (`Permille`), including the S82 default (800) when `DisposalProfile` is absent
- whether directly possessed, non-empty Waste lots exist (lawful disposal targets)
- for the satisfaction path only: the root-baseline carried-load value from the planning snapshot, used to judge disposal progress (see D2)

Each call site is responsible for extracting these values from its available surface. The helper then computes:

- whether the actor is currently strained enough that disposal is actionable
- whether the goal is satisfied (for the satisfaction path, requiring progress relative to root baseline)

This helper becomes the canonical contract for candidate emission, ranking, and satisfaction. `FreeCarryCapacity` must no longer have three slightly different interpretations of "needs disposal" or "already solved."

### D2: Redefine FreeCarryCapacity satisfaction around disposal progress

**File**: `crates/worldwake-ai/src/goal_model.rs`

Redefine `GoalKind::FreeCarryCapacity.is_satisfied()` so the planning root is not satisfied for an emitted disposal candidate. The success rule must be based on disposal progress relative to the planning snapshot baseline, not merely on the current node having some disposal-compatible state.

The intended success contract is:

- if the unified helper says `FreeCarryCapacity` is not actionable at all, the goal is satisfied and should not compete
- if the goal is actionable at the planning root, root satisfaction must be `false`
- once planning simulates disposal steps, the goal becomes satisfied only when:
  - carried load has decreased relative to the root baseline, and
  - carried load is now below the active disposal threshold

This is the architectural fix. Do not solve the pathology by teaching the planner to distrust or discard empty-step `GoalSatisfied` plans generically. That would mask the contract bug instead of fixing the world-condition semantics of this specific goal.

### D3: Make candidate emission and ranking use the same contract

**Files**: `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`

Update `emit_disposal_candidates()` and the `FreeCarryCapacity` branch in `ranking::motive_score()` to consume the shared helper from D1.

Required behavior:

- candidate emission only occurs when the unified contract says disposal is actionable
- only directly possessed, non-empty Waste lots qualify as disposal targets
- motive score uses the same load and threshold semantics as emission
- motive score returns `0` when disposal is not actionable

After this change, emission, priority, and satisfaction all agree on the same underlying disposal state.

### D4: Preserve S82's physical operator path

**Files**: existing `worldwake-ai` planning/operator wiring as needed

The fix must preserve the live S82 operator contract:

- `FreeCarryCapacity` still resolves through executable planner steps
- the relevant operator remains `PlannerOpKind::DropItem`
- authoritative progress still comes from the existing `drop_item` action path
- if multiple waste lots or repeated drops are required, the planner may still produce repeated disposal steps until the threshold is cleared

No synthetic "free capacity" effect, inventory rewrite, or instant cleanup shortcut is allowed.

### D5: Add focused parity tests for the unified contract

**Files**: focused tests near the touched modules, likely under:
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`

Add focused tests proving the new contract is consistent across layers. At minimum:

1. A strained root state with actionable Waste disposal is not satisfied.
2. A single simulated drop that lowers carried load below threshold makes the goal satisfied.
3. A simulated drop that reduces load but still leaves the actor above threshold keeps the goal unsatisfied.
4. Candidate emission and motive score both go inactive/zero when the actor is below threshold or has no lawful Waste target.
5. Only directly possessed, non-empty Waste lots are considered disposal targets.

### D6: Flip Scenario 143 from failure proof to fix proof

**File**: `crates/worldwake-ai/tests/golden_planner_pathology.rs`

Keep the exact live substrate for `degenerate_zero_step_loop_blocks_actionable_goals`:

- `scenarios/cli-evaluation.ron`
- seed `7777`
- the real Eldergrove / Forager Lina setup
- a late-run observation window after real waste accumulation

Replace the current failure assertions with fix assertions that prove the pathology is gone. The golden should show, during the late-run observation window:

1. No repeated `FreeCarryCapacity` plan attempt returns `PlanSearchOutcome::Found { steps: [] }`.
2. When `FreeCarryCapacity` is selected, it produces an executable next step whose operator surface is disposal (`PlannerOpKind::DropItem`), or the planner switches to another actionable self-care goal instead of the old zero-step no-op loop.
3. The pathological idle run is broken, evidenced by bounded inactivity and a downstream self-care recovery signal such as a later `eat` commit and/or hunger decreasing over the window.

The goal of the golden after this flip is not merely "Lina stopped picking FreeCarryCapacity." It is to prove that the exact observer-derived failure is resolved through lawful planner behavior on the real scenario substrate.

### D7: Refresh generated golden docs

**Command**:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

Keep expected generated fallout under `docs/generated/golden-*` when the updated Scenario 143 assertions change the golden metadata or narrative surfaces.

## Section H — Causal Hooks Declaration

### H.1 — Motivating consequence gap

Waste is a lawful consequence carrier from S82, but the planner currently treats `FreeCarryCapacity` as both actionable and already complete at the same time. That contradiction breaks self-care behavior on the real observer scenario. The consequence gap is therefore not "need more disposal content"; it is "the planner misclassifies a concrete disposal condition and starves other lawful actions."

### H.2 — Entities, relations, records introduced

None. This spec should not add new entities, relations, or record types.

### H.3 — Actions or world processes that mutate them

No new action families. The existing `drop_item` action from S82 remains the authoritative disposal process.

### H.4 — Information produced, travel, observability

No new information path. All logic operates on the existing planning snapshot / belief-backed surfaces already visible to the actor's reasoning pipeline.

### H.5 — Conserved quantities

No quantity changes outside existing lawful action semantics. Waste remains an item with stable identity and explicit location transfer when dropped.

### H.6 — Scarce capacities, contention

Carry capacity remains the scarce capacity under management. The spec only fixes how planner logic interprets that scarcity.

### H.7 — Partial failures, aftermath

If disposal cannot currently be planned, the goal may still fail through the existing planner failure paths. The fix does not guarantee success; it guarantees that success is not falsely declared at the root with zero steps.

### H.8 — Positive feedback loops amplified

None introduced. The current loop is removed, not replaced by a new amplification path.

### H.9 — Physical dampeners

The dampener is the existing physical disposal step: dropping Waste lowers carried load and eventually removes the strain that made the goal actionable.

### H.10 — Cross-system interaction

This is planner-local contract repair inside `worldwake-ai`. It preserves the existing S82 authoritative action path without adding new cross-system calls.

## Information-path analysis

No new information transport path is introduced. The fix must stay on the same actor-local planning surfaces already used by `worldwake-ai`: planning snapshot inventory, carry-capacity state, and belief-backed accessible items. The agent is not granted omniscient knowledge about hidden inventory, future drops, or world-state shortcuts.

## Positive-feedback analysis

The current pathology is itself a positive planner loop: selection of `FreeCarryCapacity` yields zero-step success, which yields no world change, which preserves the same selection conditions next tick. This spec breaks that loop by requiring physical disposal progress before satisfaction can be declared. No new amplification loop is added.

## Concrete dampeners

The concrete dampener is already present in the world:

- Waste occupies carried capacity.
- `drop_item` transfers Waste out of the actor's carried inventory.
- reduced carried load drops the actor below the disposal threshold.
- once below threshold, `FreeCarryCapacity` no longer outranks eating or other urgent self-care work.

No invisible clamp, cooldown, or score suppression is part of the design.

## Stored state vs. derived read-model list

| Item | Classification | Justification |
|------|---------------|---------------|
| `DisposalProfile` | Authoritative stored state | Existing per-agent profile controlling disposal threshold behavior. |
| Carry capacity | Authoritative stored state | Existing agent capability / inventory constraint. |
| Waste item identity, possession, quantity | Authoritative stored state | Existing item state and location/possession relations. |
| Current carried load | Derived | Computed from planning-state inventory/load surfaces. |
| Active disposal threshold | Derived | Computed from `DisposalProfile` or the existing default rule. |
| Actionable disposal targets | Derived | Computed from directly possessed, non-empty Waste lots in the planning state. |
| Root disposal baseline load | Derived per planning call | Captured from the planning snapshot to evaluate progress. |
| Goal satisfaction for `FreeCarryCapacity` | Derived | Computed from the unified contract, not stored. |

## SystemFn Integration

No new SystemFn is expected. This is a `worldwake-ai` planning-contract change that preserves the existing S82 authoritative action execution path.

## Component Registration

No new components. No registration changes required.

## Authoritative-to-AI Impact Rule

This spec modifies planner-internal contract logic for an existing goal kind. Checklist:

1. `get_affordances` — N/A (affordance generation not modified)
2. `generate_candidates` — **affected** (D3 changes `emit_disposal_candidates` emission condition)
3. `search_plan` — **affected** (D2 changes `is_satisfied` which feeds `terminal_kind`)
4. `BestEffort` action start — N/A (action start paths unchanged)
5. `handle_plan_failure` — N/A (failure paths unchanged)
6. Payload revalidation — N/A (`drop_item` uses affordance-derived payloads, not planner-synthesized)
7. Golden tests — **affected** (D6 flips Scenario 143; D5 adds focused parity tests; verification runs all `worldwake-ai` tests)

## Verification

```bash
cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals -- --nocapture
cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture
cargo test -p worldwake-ai
python3 scripts/golden_inventory.py --write --check-docs
cargo clippy --workspace --all-targets -- -D warnings
```

## Expected Outcome

After implementation, Scenario 143 no longer proves a planner no-op loop. Instead, it proves that on the exact `cli-evaluation.ron` observer substrate, `FreeCarryCapacity` either plans real disposal work through `drop_item` or yields to another actionable self-care goal, allowing Lina to break the late-run stall and recover from the starvation trajectory.

## Outcome

- Completion date: 2026-04-11
- What changed: unified `FreeCarryCapacity` around a shared planner-local contract across satisfaction, candidate emission, and ranking; fixed directly possessed lot quantity reconstruction in `PerAgentBeliefView`; flipped Scenario 143 into a fix-proof golden; refreshed generated golden docs.
- Deviations from original plan: the final root cause included belief-view quantity undercounting for directly possessed lots, so the completed scope included `worldwake-sim/src/per_agent_belief_view.rs` in addition to the originally named `worldwake-ai` planner surfaces.
- Verification results:
  - `cargo test -p worldwake-sim directly_possessed_item_lot_quantity_uses_authoritative_quantity_over_stale_belief -- --nocapture`
  - `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
  - `cargo test -p worldwake-ai degenerate_zero_step_loop_blocks_actionable_goals -- --nocapture`
  - `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy --workspace --all-targets -- -D warnings`
