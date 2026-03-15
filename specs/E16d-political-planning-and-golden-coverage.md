# E16d: Political Planning Fix & Golden E2E Coverage

## Epic Summary

Fix the architectural gap where `apply_planner_step` in `goal_model.rs` falls through to `_ => state` for `PlannerOpKind::Bribe` and `PlannerOpKind::Threaten`, meaning the GOAP planner never selects them despite their inclusion in `CLAIM_OFFICE_OPS`. Deliver outcome-based planning semantics for both operations and a comprehensive golden E2E test suite covering all political scenarios introduced by E16.

## Phase

Phase 3: Information & Politics (post-E16 fix)

## Crate

`worldwake-sim` (belief view trait extension)
`worldwake-ai` (planning snapshot, planning state, goal model, golden tests)

## Dependencies

- E16 (offices, succession laws, factions, support declarations, bribe/threaten actions)
- E13 (decision architecture, GOAP search, planning state)
- E12 (combat profiles — `attack_skill` used in threat pressure calculation)

## Problem

`CLAIM_OFFICE_OPS` includes `Bribe` and `Threaten` alongside `Travel` and `DeclareSupport`:

```rust
const CLAIM_OFFICE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Bribe,
    PlannerOpKind::Threaten,
    PlannerOpKind::DeclareSupport,
];
```

However, `apply_planner_step` (goal_model.rs ~line 488) has a catch-all `_ => state` that silently returns planning state unchanged for Bribe and Threaten. The planner therefore never observes any state improvement from these operations and never includes them in generated plans.

The fix models Bribe and Threaten as **outcome-based planning operations** that produce strategic outcomes (support declarations) directly in `PlanningState`, mirroring the authoritative handler effects at a planning level.

## Design Approach: Outcome-Based Planning for Social Actions

### Why outcome-based (not loyalty tracking)

1. The planner models the actor's own actions, not other agents' autonomous decisions.
2. Loyalty is an intermediate mechanism; the strategic outcome (support declaration) is what the planner needs to evaluate plan viability.
3. `PlanningState` already has `with_support_declaration()` and `with_commodity_quantity()` — no new abstractions required.
4. Follows existing pattern: `DeclareSupport` already writes support declarations directly in `apply_planner_step`.
5. Commodity cost for Bribe is naturally modeled via existing `with_commodity_quantity()`.

### Bribe planning semantics

Mirror `commit_bribe` in `office_actions.rs`:
- Actor loses `offered_quantity` of `offered_commodity` (via `with_commodity_quantity`)
- Target hypothetically declares support for actor at the goal's office (via `with_support_declaration`)
- The planner can then evaluate whether the commodity cost is worth the support gain

### Threaten planning semantics

Mirror `commit_threaten` in `office_actions.rs`:
- Read actor's `CombatProfile.attack_skill` as threat pressure (matches `threat_pressure()` in `office_actions.rs`)
- Read target's `UtilityProfile.courage` from snapshot
- If `attack_skill > courage` → target hypothetically declares support for actor (yield outcome)
- If `attack_skill <= courage` → state unchanged (resist outcome; planner sees no benefit, skips)

### Only new snapshot data needed

Add `courage: Option<Permille>` to `SnapshotEntity`. This is already stored in `UtilityProfile.courage` on agents; it just needs to be captured into the planning snapshot.

No new `PlanningState` fields — reuses existing `with_support_declaration()` and `with_commodity_quantity()`.

## Existing Infrastructure (Leveraged, Not Reimplemented)

| Infrastructure | Location | Usage in E16d |
|----------------|----------|---------------|
| `PlannerOpKind::Bribe` | `planner_ops.rs` | Already defined; needs planning semantics |
| `PlannerOpKind::Threaten` | `planner_ops.rs` | Already defined; needs planning semantics |
| `CLAIM_OFFICE_OPS` | `goal_model.rs:123-128` | Already includes Bribe/Threaten |
| `with_support_declaration()` | `planning_state.rs:104-113` | Reused for hypothetical support outcome |
| `with_commodity_quantity()` | `planning_state.rs:457-468` | Reused for bribe commodity cost |
| `commit_bribe()` | `office_actions.rs:399-422` | Authoritative handler to mirror |
| `commit_threaten()` | `office_actions.rs:456-478` | Authoritative handler to mirror |
| `threat_pressure()` | `office_actions.rs:620-622` | Returns `profile.attack_skill` |
| `UtilityProfile.courage` | `utility_profile.rs:18` | Already stored on agents |
| `CombatProfile.attack_skill` | `combat.rs` | Already in `SnapshotEntity.combat_profile` |
| `RuntimeBeliefView` trait | `belief_view.rs:109-229` | Trait to extend with `courage()` |
| `SnapshotEntity` | `planning_snapshot.rs:20-51` | Struct to extend with `courage` |
| `GoldenHarness` | `golden_harness/mod.rs` | Test harness to extend with office/faction helpers |

---

## Part A: Planner Gap Fix

### Deliverable A1 — Add `courage()` to `RuntimeBeliefView` trait + implementations

**Ticket**: E16d-001

#### `RuntimeBeliefView` trait (belief_view.rs)

Add a new default method:

```rust
fn courage(&self, agent: EntityId) -> Option<Permille> {
    let _ = agent;
    None
}
```

Place after `combat_profile()` (line 186) for logical grouping with agent profile queries.

#### `OmniscientBeliefView` (omniscient_belief_view.rs)

Implement by delegating to `UtilityProfile`:

```rust
fn courage(&self, agent: EntityId) -> Option<Permille> {
    self.world
        .get_component_utility_profile(agent)
        .map(|p| p.courage)
}
```

#### `PerAgentBeliefView` (per_agent_belief_view.rs)

Same delegation pattern:

```rust
fn courage(&self, agent: EntityId) -> Option<Permille> {
    self.world
        .get_component_utility_profile(agent)
        .map(|p| p.courage)
}
```

Note: Under post-E14 per-agent belief boundaries, this may need to be gated by observation. For now, both implementations use direct world access, matching the existing pattern for `combat_profile()`.

**Tests**: Verify `courage()` returns `Some(value)` for agents with `UtilityProfile`, `None` for entities without.

**Files modified**: `crates/worldwake-sim/src/belief_view.rs`, `crates/worldwake-sim/src/omniscient_belief_view.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`

---

### Deliverable A2 — Add `courage` to `SnapshotEntity` + expose through `PlanningState`

**Ticket**: E16d-002

#### `SnapshotEntity` (planning_snapshot.rs)

Add field:

```rust
pub(crate) courage: Option<Permille>,
```

Place after `combat_profile` for logical grouping.

In `Default` impl, add:

```rust
courage: None,
```

In `build_snapshot_entity()`, populate:

```rust
courage: view.courage(entity),
```

#### `PlanningState` RuntimeBeliefView impl (planning_state.rs)

Add implementation that reads from the snapshot:

```rust
fn courage(&self, agent: EntityId) -> Option<Permille> {
    self.snapshot.entity(agent).and_then(|e| e.courage)
}
```

This follows the existing pattern used by `combat_profile()` and other profile accessors on `PlanningState`.

**Tests**: Build a `SnapshotEntity` with `courage = Some(Permille::new(500))`, verify `PlanningState::courage()` returns matching value. Verify `None` for entities not in snapshot.

**Files modified**: `crates/worldwake-ai/src/planning_snapshot.rs`, `crates/worldwake-ai/src/planning_state.rs`

---

### Deliverable A3 — Implement `apply_planner_step` for `PlannerOpKind::Bribe`

**Ticket**: E16d-003

Replace the `_ => state` catch-all (goal_model.rs ~line 488) with an explicit `PlannerOpKind::Bribe` arm under `GoalKind::ClaimOffice`:

```rust
PlannerOpKind::Bribe => match self {
    GoalKind::ClaimOffice { office } => {
        // Extract bribe payload from the planned step
        let bribe = step
            .payload
            .as_ref()
            .and_then(ActionPayload::as_bribe);
        if let Some(bribe) = bribe {
            let current_qty = state.commodity_quantity(actor, bribe.offered_commodity);
            if current_qty >= bribe.offered_quantity {
                let remaining = Quantity(current_qty.0.saturating_sub(bribe.offered_quantity.0));
                state
                    .with_commodity_quantity(actor, bribe.offered_commodity, remaining)
                    .with_support_declaration(bribe.target, *office, actor)
            } else {
                state // insufficient goods, no planning benefit
            }
        } else {
            state
        }
    }
    _ => state,
},
```

**Semantics**: Actor pays commodity cost, target hypothetically supports actor for the goal's office. Mirrors `commit_bribe` which transfers commodity and increases loyalty.

**Tests**: See E16d-005.

**Files modified**: `crates/worldwake-ai/src/goal_model.rs`

---

### Deliverable A4 — Implement `apply_planner_step` for `PlannerOpKind::Threaten`

**Ticket**: E16d-004

Add explicit `PlannerOpKind::Threaten` arm:

```rust
PlannerOpKind::Threaten => match self {
    GoalKind::ClaimOffice { office } => {
        let threaten = step
            .payload
            .as_ref()
            .and_then(ActionPayload::as_threaten);
        if let Some(threaten) = threaten {
            let attack_skill = state
                .combat_profile(actor)
                .map(|p| p.attack_skill)
                .unwrap_or(Permille::ZERO);
            let target_courage = state
                .courage(threaten.target)
                .unwrap_or(Permille::MAX);
            if attack_skill > target_courage {
                // Target yields — hypothetical support declaration
                state.with_support_declaration(threaten.target, *office, actor)
            } else {
                // Target resists — no planning benefit
                state
            }
        } else {
            state
        }
    }
    _ => state,
},
```

**Semantics**: If actor's `attack_skill` (threat pressure) exceeds target's `courage`, target hypothetically supports actor. Otherwise state unchanged and planner will not select this step. Mirrors `commit_threaten` which compares `threat_pressure(profile)` against `courage` and either increases loyalty or adds hostility.

**Design note**: The planner conservatively defaults unknown courage to `Permille::MAX` (resist), matching GOAP's principle of only selecting steps that produce observable state improvement. The planner conservatively defaults missing attack_skill to `Permille::ZERO` (no threat).

After both A3 and A4 are added, the `_ => state` catch-all at line 488 should be removed entirely. All `PlannerOpKind` variants must have explicit handling or the code should fail to compile via a non-exhaustive match. If any future `PlannerOpKind` variants are added, this forces the developer to add planning semantics rather than silently falling through.

**Tests**: See E16d-005.

**Files modified**: `crates/worldwake-ai/src/goal_model.rs`

---

### Deliverable A5 — Unit tests for Bribe/Threaten planning state transitions

**Ticket**: E16d-005

Add unit tests in `goal_model.rs` (or a dedicated test module) covering:

1. **Bribe with sufficient goods**: Verify `apply_planner_step` for Bribe reduces actor's commodity quantity and adds support declaration for target.
2. **Bribe with insufficient goods**: Verify state returned unchanged when actor lacks offered quantity.
3. **Bribe with no payload**: Verify state returned unchanged when step has no bribe payload.
4. **Threaten with high attack vs low courage**: Verify support declaration added for target.
5. **Threaten with low attack vs high courage**: Verify state returned unchanged (resist).
6. **Threaten with missing combat profile**: Verify state returned unchanged (no threat possible).
7. **Threaten with missing target courage**: Verify state returned unchanged (defaults to resist).
8. **Bribe under non-ClaimOffice goal**: Verify state returned unchanged.
9. **Threaten under non-ClaimOffice goal**: Verify state returned unchanged.

**Files modified**: `crates/worldwake-ai/src/goal_model.rs` (test module)

---

### Deliverable A6 — Integration test: planner finds Bribe/Threaten plans

**Ticket**: E16d-006

Add integration tests verifying the planner actually selects Bribe and Threaten in realistic scenarios:

1. **Planner selects Bribe plan**: Agent at jurisdiction with goods, bribable target present, vacant office. Verify `search_plan()` returns a plan containing `PlannerOpKind::Bribe` followed by `PlannerOpKind::DeclareSupport`.
2. **Planner selects Threaten plan**: Agent at jurisdiction with high attack_skill, low-courage target present, vacant office. Verify plan contains `PlannerOpKind::Threaten`.
3. **Planner selects Travel + Bribe**: Agent NOT at jurisdiction but has goods. Verify plan starts with `Travel` then includes `Bribe` + `DeclareSupport`.
4. **Planner rejects Threaten against high-courage target**: Agent at jurisdiction, target courage exceeds attack_skill. Verify Threaten is NOT in the plan (planner finds DeclareSupport-only plan or no plan if DeclareSupport alone is insufficient).

**Files modified**: `crates/worldwake-ai/src/goal_model.rs` or `crates/worldwake-ai/tests/` (integration test)

---

## Part B: Golden E2E Test Suite

### New file: `crates/worldwake-ai/tests/golden_offices.rs`

### Harness extensions needed in `golden_harness/mod.rs`

Add the following helpers to `GoldenHarness`:

| Helper | Purpose |
|--------|---------|
| `seed_office(place, succession_law, succession_period, eligibility_rule) -> EntityId` | Create Office entity with `OfficeData` at a jurisdiction |
| `seed_faction(name) -> EntityId` | Create Faction entity with `FactionData` |
| `add_faction_membership(agent, faction)` | Add `member_of` relation between agent and faction |
| `set_loyalty(subject, target, value: Permille)` | Seed loyalty relation |
| `set_courage(agent, value: Permille)` | Update agent's `UtilityProfile.courage` |
| `enterprise_weighted_utility(enterprise: Permille) -> UtilityProfile` | Create utility profile with high enterprise weight |

### Scenario 11: Simple Office Claim via DeclareSupport

- **Setup**: Vacant office (Support law, period=5) at VillageSquare. Single sated agent with `enterprise_weight=pm(800)`, eligible (no faction rule). Agent has beliefs about the office.
- **Expected**: Agent generates `ClaimOffice` → plans `DeclareSupport(self)` → executes → after succession period, `succession_system` installs agent as holder.
- **Cross-system**: AI `candidate_generation` → `ranking` → `plan search` → `DeclareSupport` action handler → `succession_system` tick → installation.
- **Assertions**: Office holder == agent after N ticks. Event log contains `Political` + installation tags.
- **New coverage**: `GoalKind::ClaimOffice`, `PlannerOpKind::DeclareSupport`, succession_system (support law).

### Scenario 11b: Scenario 11 replays deterministically

Same seed, verify identical world + event log hashes.

### Scenario 12: Competing Claims with Loyal Supporter

- **Setup**: Vacant office. Agents A and B both eligible, both `enterprise_weight > 0`. Agent C has loyalty to A, `social_weight > 0`. All at jurisdiction, all sated.
- **Expected**: A generates `ClaimOffice` → declares for self. B generates `ClaimOffice` → declares for self. C generates `SupportCandidateForOffice(A)` → declares for A. A gets 2 declarations (self + C), B gets 1. Succession installs A.
- **Cross-system**: Multi-agent AI → loyalty-driven support candidate generation → concurrent `DeclareSupport` actions → support counting → succession resolution.
- **Assertions**: Office holder == A. C's support_declaration for office == A.
- **New coverage**: `GoalKind::SupportCandidateForOffice`, multi-agent support competition, loyalty-based ranking.

### Scenario 13: Bribe → Support Coalition

- **Setup**: Vacant office. Agent A eligible, `enterprise_weight=pm(900)`, holds 5 bread. Agent B at jurisdiction, no initial loyalty to A. Both sated.
- **Expected**: A generates `ClaimOffice`. Planner finds `Bribe(B, bread)` as intermediate step → `DeclareSupport(self)`. A bribes B (bread transfers), B's loyalty increases. B then generates `SupportCandidateForOffice(A)` and declares support. Succession installs A.
- **Cross-system**: `ClaimOffice` goal → planner selects Bribe op → commodity transfer (conservation) → loyalty change → B's AI generates support goal → `DeclareSupport` → succession.
- **Assertions**: A is office holder. B has less bread than initial. Conservation holds. A has support from both self and B.
- **New coverage**: Autonomous Bribe through planner, commodity cost in political planning, Bribe → downstream AI response.

### Scenario 14: Threaten with Courage Diversity

- **Setup**: Vacant office. Agent A (high `attack_skill=pm(800)`, `enterprise_weight=pm(900)`). Two targets: Agent B (`courage=pm(200)`, should yield) and Agent C (`courage=pm(900)`, should resist). All at jurisdiction, sated.
- **Expected**: A generates `ClaimOffice`. Planner finds `Threaten(B)` as viable (800 > 200) but not `Threaten(C)` (800 < 900). A threatens B → B yields → loyalty increase. A also declares for self. B may support A. C does not.
- **Cross-system**: `ClaimOffice` → planner courage threshold evaluation → Threaten action → courage-based yield/resist → divergent outcomes → Principle 20 (agent diversity).
- **Assertions**: B has increased loyalty to A. C has hostility toward A (if A threatens C) or C is unaffected. A becomes office holder if sufficient support.
- **New coverage**: Autonomous Threaten through planner, courage-based outcome diversity, Principle 20.

### Scenario 15: Travel to Distant Jurisdiction for Office Claim

- **Setup**: Vacant office at VillageSquare. Eligible agent starts at BanditCamp (4 hops away). Agent has beliefs about the vacant office. Sated, `enterprise_weight=pm(800)`.
- **Expected**: Agent generates `ClaimOffice` → plans `Travel(multi-hop)` + `DeclareSupport` → traverses route → arrives → declares support → installed after succession period.
- **Cross-system**: Political goal at remote location → multi-hop travel planning → sequential travel execution → `DeclareSupport` → succession.
- **Assertions**: Agent ends at VillageSquare. Office holder == agent.
- **New coverage**: Travel as political planning step, remote office awareness.

### Scenario 16: Survival Pressure Suppresses Political Goals

- **Setup**: Vacant office. Critically hungry agent with `enterprise_weight=pm(800)`, eligible. Food (bread) available locally. Both office and food at VillageSquare.
- **Expected**: Agent suppresses `ClaimOffice` (Medium priority) under survival pressure. Eats bread first. After hunger relief (below high threshold), generates `ClaimOffice` and declares.
- **Cross-system**: Needs suppression → survival priority → eat → hunger relief → political goal emergence → `DeclareSupport` → succession.
- **Assertions**: Bread consumed before `DeclareSupport` event in event log timeline. Agent eventually becomes office holder.
- **New coverage**: Political goal suppression under survival pressure, priority ordering, suppression lift.

### Scenario 17: Faction Eligibility Filters Office Claim

- **Setup**: Vacant office with `EligibilityRule::FactionMember(faction_x)`. Agent A is member of `faction_x` (eligible). Agent B is NOT a member (ineligible). Both at jurisdiction, sated, `enterprise_weight > 0`.
- **Expected**: A generates `ClaimOffice`, B does NOT (filtered by `candidate_is_eligible`). A declares and gets installed. B never generates a `ClaimOffice` goal.
- **Cross-system**: Faction membership → eligibility filtering → selective candidate generation → uncontested succession.
- **Assertions**: A is office holder. B never executed a `DeclareSupport` action. Event log shows no `ClaimOffice`-related events from B.
- **New coverage**: `EligibilityRule::FactionMember`, `FactionData`, faction_membership relation, eligibility-gated candidate generation.

### Scenario 18: Force Succession — Sole Eligible Agent Installed

- **Setup**: Office with `SuccessionLaw::Force` at VillageSquare. Agent A eligible, sated. Agent B has `dead_at` set. After succession period, A is sole living eligible agent → installed.
- **Expected**: Succession system detects vacancy → waits `succession_period` ticks → finds exactly 1 eligible agent → installs A.
- **Cross-system**: Death → vacancy detection → force law resolution → uncontested installation.
- **Assertions**: A is office holder. No `DeclareSupport` events (force law doesn't use support counting).
- **New coverage**: `SuccessionLaw::Force`, force succession resolution, vacancy from death.

### Scenario 18b: Scenario 18 replays deterministically

Same seed, verify identical world + event log hashes.

---

## Part C: Coverage Report Update

Update `reports/golden-e2e-coverage-analysis.md`:

1. **GoalKind table**: Add `ClaimOffice` (Yes, scenarios 11-17) and `SupportCandidateForOffice` (Yes, scenario 12). Coverage becomes 19/20 (`SellCommodity` still untested).
2. **ActionDomain table**: Note new Social sub-actions (`Bribe`, `Threaten`, `DeclareSupport`) exercised in scenarios 13, 14, 11-12.
3. **New cross-system chains**: Add ~8 new proven interactions:
   - Political goal generation (candidate_generation → ranking → plan search)
   - Succession resolution (support counting → installation)
   - Bribe commodity transfer (conservation-safe political resource expenditure)
   - Courage-based threaten (attack_skill vs courage → yield/resist)
   - Survival suppression of political goals (needs priority > enterprise priority)
   - Faction eligibility filtering (membership → candidate generation gate)
   - Travel-to-jurisdiction (multi-hop travel as political plan step)
   - Force succession (vacancy → sole eligible → installation)
4. **Places used**: VillageSquare and BanditCamp at minimum.
5. **Summary statistics**: Update proven test count, cross-system chain count.
6. **New scenario entries**: Add full documentation for scenarios 11-18b following existing format (file, test name, systems exercised, setup, emergent behavior proven, cross-system chain).

---

## Section H: Foundation Analysis (FND-01)

### H1: Information-Path Analysis

**Bribe planning**: Actor reads own `commodity_quantities` from `SnapshotEntity` (own state, captured at planning time from belief view). Target's identity and location read from snapshot. No locality violation — actor knows what they possess and who is nearby.

**Threaten planning**: Actor reads own `CombatProfile.attack_skill` from `SnapshotEntity.combat_profile` (own state). Target's `courage` read from `SnapshotEntity` via belief view — under `OmniscientBeliefView` this is direct world access; under `PerAgentBeliefView` (post-E14) it will be gated by observation. Current implementation uses omniscient view, consistent with all other agent profile reads at this development stage.

**Golden tests**: All scenarios use `OmniscientBeliefView` (pre-E14). Information paths are: world state → omniscient belief view → planning snapshot → planner decision. Post-E14, information will flow through perception events and belief updates, adding proper locality gating.

### H2: Positive-Feedback Analysis

**Bribe success loop**: Bribe success → more supporters → office claim → office holder resources → more bribes. This loop exists in the full simulation (not just the planner). The planner models the commodity COST of each bribe step, naturally limiting how many bribes a single plan can include.

**Threaten success loop**: Threaten success → more supporters → office claim → more power → more threatening. However, threaten has no commodity cost, so the loop is dampened only by the courage threshold (resistant agents) and planner budget limits.

### H3: Concrete Dampeners

1. **Bribe consumes real goods** — finite inventory limits bribe count. An agent cannot bribe infinitely because each bribe transfers physical commodities that must be acquired through harvesting, crafting, or trade (physical dampener, Principle 8).
2. **Threaten resistance** — agents with `courage > attack_skill` resist, producing hostility not support. High-courage agents are immune to threats, creating natural population-level diversity in threat effectiveness (Principle 20).
3. **Planning budget** — `PlanningBudget` limits plan depth and expansion count, preventing unbounded plan chains.
4. **Succession period** — office installation requires waiting `succession_period` ticks after vacancy, during which other agents can compete.
5. **Agent diversity** — `courage: Permille` varies per-agent via `UtilityProfile`, ensuring no uniform response to threats across the population (Principle 20).

Note: Numerical clamps are NOT used as dampeners. All dampeners are physical world processes or agent-diversity mechanisms.

### H4: Stored State vs. Derived Read-Model

**Authoritative stored state** (components/relations):
- `UtilityProfile.courage` — per-agent courage threshold (component on Agent entities)
- `CombatProfile.attack_skill` — per-agent attack skill (component on Agent entities)
- `loyalty_to` relation — subject→target loyalty `Permille`
- `support_declaration` relation — supporter→office→candidate
- `OfficeData` — office configuration (succession law, period, eligibility)
- `office_holder` relation — recognized office holder

**Transient derived computation** (discarded after use):
- `SnapshotEntity.courage` — snapshot of courage for planning (captured once, used during search, discarded)
- `PlanningState` support_declaration_overrides — hypothetical support declarations during plan search (discarded after search completes)
- `PlanningState` commodity_quantity_overrides — hypothetical commodity quantities during plan search (discarded after search completes)
- "Will bribe succeed?" — evaluated during `apply_planner_step` as `current_qty >= offered_qty` (not stored)
- "Will threaten succeed?" — evaluated during `apply_planner_step` as `attack_skill > courage` (not stored)

No derived value is stored as authoritative state. All planning state is transient.

---

## Tickets Summary

### Part A: Planner Gap Fix

| Ticket | Description | Dependencies | Files |
|--------|-------------|--------------|-------|
| E16d-001 | Add `courage()` to `RuntimeBeliefView` + implementations | None | belief_view.rs, omniscient_belief_view.rs, per_agent_belief_view.rs |
| E16d-002 | Add `courage` to `SnapshotEntity` + `PlanningState` | E16d-001 | planning_snapshot.rs, planning_state.rs |
| E16d-003 | `apply_planner_step` for `PlannerOpKind::Bribe` | E16d-002 | goal_model.rs |
| E16d-004 | `apply_planner_step` for `PlannerOpKind::Threaten` | E16d-002 | goal_model.rs |
| E16d-005 | Unit tests for Bribe/Threaten planning transitions | E16d-003, E16d-004 | goal_model.rs |
| E16d-006 | Integration test: planner finds Bribe/Threaten plans | E16d-005 | goal_model.rs or tests/ |

### Part B: Golden E2E Tests

| Ticket | Description | Dependencies | Files |
|--------|-------------|--------------|-------|
| E16d-007 | Golden harness extensions (office/faction helpers) | None | golden_harness/mod.rs |
| E16d-008 | Scenario 11 + 11b: Simple office claim + determinism | E16d-007 | golden_offices.rs |
| E16d-009 | Scenario 12: Competing claims with loyal supporter | E16d-007 | golden_offices.rs |
| E16d-010 | Scenario 13: Bribe → support coalition | E16d-003, E16d-007 | golden_offices.rs |
| E16d-011 | Scenario 14: Threaten with courage diversity | E16d-004, E16d-007 | golden_offices.rs |
| E16d-012 | Scenario 15: Travel to distant jurisdiction | E16d-007 | golden_offices.rs |
| E16d-013 | Scenario 16: Survival pressure suppresses political goals | E16d-007 | golden_offices.rs |
| E16d-014 | Scenario 17: Faction eligibility filters office claim | E16d-007 | golden_offices.rs |
| E16d-015 | Scenario 18 + 18b: Force succession + determinism | E16d-007 | golden_offices.rs |

### Part C: Coverage Report

| Ticket | Description | Dependencies | Files |
|--------|-------------|--------------|-------|
| E16d-016 | Update coverage report with E16 scenarios | E16d-008 through E16d-015 | reports/golden-e2e-coverage-analysis.md |

## Implementation Order

```
E16d-001 → E16d-002 → E16d-003, E16d-004 (parallel) → E16d-005 → E16d-006
                    ↘ E16d-007 → E16d-008 through E16d-015 (can overlap with A5-A6)
                                                           → E16d-016
```

## Verification

After full implementation:

1. `cargo test --workspace` — all existing + new tests pass
2. `cargo clippy --workspace` — no warnings
3. Verify planner selects Bribe in integration test (E16d-006 test 1)
4. Verify planner selects Threaten in integration test (E16d-006 test 2)
5. Verify planner rejects Threaten against high-courage target (E16d-006 test 4)
6. Verify all 10 golden scenarios pass (8 scenarios + 2 determinism replays)
7. Verify `_ => state` catch-all is eliminated from `apply_planner_step`
8. Review `reports/golden-e2e-coverage-analysis.md` for completeness
