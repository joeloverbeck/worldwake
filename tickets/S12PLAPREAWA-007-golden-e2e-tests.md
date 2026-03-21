# S12PLAPREAWA-007: Golden E2E tests for prerequisite-aware multi-hop planning

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: S12PLAPREAWA-001 through S12PLAPREAWA-006 (all implementation and unit tests complete)

## Problem

The prerequisite-aware search enhancement must be validated end-to-end with golden tests that prove agents can autonomously form multi-step plans requiring intermediate resource procurement. The spec identifies two scenarios: (1) a healer procuring medicine before treating a patient, and (2) an agent gathering recipe inputs before crafting. Both scenarios previously failed and were worked around in earlier golden test design.

## Assumption Reassessment (2026-03-21)

1. Golden test infrastructure exists in `crates/worldwake-ai/tests/` with `GoldenHarness` providing `step_once()`, `enable_tracing()`, `enable_action_tracing()`, `agent_active_action_name()`, and assertion helpers — confirmed via existing golden tests.
2. `PerceptionProfile` must be set on agents that need to observe post-production output or newly created entities — confirmed per CLAUDE.md golden test guidance.
3. Deterministic replay companions are required for golden tests — confirmed per existing pattern (every golden test has a `_replay` companion).
4. The healer/medicine/patient scenario was originally attempted in S07 golden coverage but redesigned to give medicine upfront — confirmed per spec "Discovered Via" section.
5. `DecisionTraceSink` can be queried for per-agent per-tick traces including `PlanSearchOutcome` and `SearchExpansionSummary` — confirmed.
6. `ActionTraceSink` can verify action lifecycle (started, committed) — confirmed.
7. Resource sources, merchants, recipes, travel edges, wound application, and Medicine `CommodityKind` are all available in the golden test setup infrastructure — confirmed via existing golden tests covering these domains.
8. Golden test count is currently 133 — these additions will bring it to 135+ (must update golden inventory docs).
9. Test-only ticket — no production code changes.
10. Golden scenarios isolate the intended multi-hop planning branch by ensuring the agent has no alternative source of the needed commodity at their current location, forcing cross-location procurement.

## Architecture Check

1. Golden tests follow the established pattern: setup world → run N ticks → assert behavioral outcomes + deterministic replay. No new test infrastructure needed.
2. No backwards-compatibility concerns — pure test additions.

## Verification Layers

1. Multi-hop medicine procurement plan emerges → action trace showing Travel→PickUp→Travel→Heal sequence
2. Multi-hop craft input procurement plan emerges → action trace showing Travel→PickUp→Travel→Craft sequence
3. Deterministic replay produces identical event logs → replay companion tests
4. Decision trace shows prerequisite places guiding search → `SearchExpansionSummary.prerequisite_places_count > 0` in early expansions, `== 0` in later expansions (after hypothetical pickup)
5. All 133+ existing golden tests remain passing → regression gate

## What to Change

### 1. `golden_multi_hop_medicine_procurement` (golden_emergent.rs or golden_care.rs)

**Scenario**: Healer at Village Square. Patient at Village Square (wounded — has at least one active wound). Medicine lot on ground at Orchard Farm. No medicine at Village Square. Travel edge VS↔OF exists.

**Expected emergent behavior**: Healer autonomously plans and executes:
1. Travel(Village Square → Orchard Farm)
2. PickUp(medicine lot at Orchard Farm)
3. Travel(Orchard Farm → Village Square)
4. Heal(patient)

**Assertions**:
- After sufficient ticks, patient's wound count has decreased (heal action committed)
- Medicine lot ownership transferred from ground to healer (then consumed by heal)
- Action trace shows the 4-step sequence in order
- Decision trace shows `prerequisite_places_count > 0` when planning started (medicine location was guiding search)
- Deterministic replay companion produces identical event log hash

**Setup details**:
- Healer agent: `ControlSource::Ai`, `CombatProfile` with healing capability, `PerceptionProfile`, `UtilityProfile` with `care_weight > 0`
- Patient agent: at Village Square, with at least one wound (e.g., bleeding wound from combat or deprivation)
- Medicine lot: `CommodityKind::Medicine`, `Quantity(1)`, on ground at Orchard Farm
- No medicine anywhere else accessible to the healer
- Travel edge between Village Square and Orchard Farm

### 2. `golden_prerequisite_aware_craft` (golden_emergent.rs or golden_production.rs)

**Scenario**: Agent at Workshop (where workstation exists). Recipe requires Wheat as input. Wheat lot on ground at Farm. No Wheat at Workshop. Travel edge Workshop↔Farm exists.

**Expected emergent behavior**: Agent autonomously plans and executes:
1. Travel(Workshop → Farm)
2. PickUp(wheat lot at Farm)
3. Travel(Farm → Workshop)
4. Craft(recipe at workstation)

**Assertions**:
- After sufficient ticks, crafted output exists (craft action committed)
- Wheat lot was consumed by crafting
- Action trace shows the 4-step sequence in order
- Deterministic replay companion produces identical event log hash

**Setup details**:
- Agent: `ControlSource::Ai`, `PerceptionProfile`, `UtilityProfile` with production weight
- Workstation at Workshop location (matching recipe's `WorkstationTag`)
- Recipe in `RecipeRegistry` requiring Wheat input
- Wheat lot: appropriate commodity kind, on ground at Farm
- No wheat at Workshop
- Travel edge between Workshop and Farm

### 3. Update golden test inventory documentation

Update `docs/golden-test-inventory.md` (or equivalent) to reflect the new test count and scenario descriptions.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` or `golden_care.rs` (modify — add medicine procurement golden)
- `crates/worldwake-ai/tests/golden_emergent.rs` or `golden_production.rs` (modify — add craft prerequisite golden)
- Golden test inventory docs (modify — update count and scenario list)

## Out of Scope

- Production code changes — this ticket is test-only
- Unit tests (S12PLAPREAWA-006)
- Changes to `GoldenHarness` or test infrastructure
- Changes to action handlers, planner internals, or goal model
- Testing materialization barrier scenarios (Trade→Craft chains) — that requires hypothetical materialization modeling which is explicitly out of S12 scope

## Acceptance Criteria

### Tests That Must Pass

1. `golden_multi_hop_medicine_procurement` — healer procures medicine from remote location and heals patient
2. `golden_multi_hop_medicine_procurement_replay` — deterministic replay companion
3. `golden_prerequisite_aware_craft` — agent procures recipe input from remote location and crafts
4. `golden_prerequisite_aware_craft_replay` — deterministic replay companion
5. All 133+ existing golden tests: `cargo test -p worldwake-ai golden`
6. Full workspace: `cargo test --workspace`

### Invariants

1. All existing golden tests pass unchanged — the enhancement is strictly additive
2. New golden tests include deterministic replay companions
3. Agents plan from beliefs only (Principle 7) — no omniscient shortcuts in test setup
4. Conservation invariants hold — Medicine consumed by heal, Wheat consumed by craft
5. `PerceptionProfile` is set on all agents that need to observe entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_*.rs` — 2 new golden scenarios + 2 replay companions
2. Golden inventory docs — updated scenario count

### Commands

1. `cargo test -p worldwake-ai golden_multi_hop`
2. `cargo test -p worldwake-ai golden_prerequisite`
3. `cargo test -p worldwake-ai && cargo clippy --workspace`
4. `cargo test --workspace`
