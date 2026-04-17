# S116DRIESCSUS-009: Let Wash planning pursue lawful water acquisition before wash

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` candidate generation, goal dispatch/feasibility, focused planner tests
**Deps**: specs/S116-drive-escalation-sustained-critical.md, archive/tickets/S116DRIESCSUS-004.md, archive/tickets/S116DRIESCSUS-008.md

## Problem

The live `GoalKind::Wash` contract is split across two incompatible planner surfaces. The planner already knows how to route Wash toward believed `WorkstationTag::WashBasin` places, but `GoalKind::Wash` never materializes unless the agent already has locally controlled water. Under the current authoritative `wash` action contract, that means the AI cannot lawfully pursue the natural chain `travel to known basin / procure water / wash`; it can only wash when water is already in hand. This blocks the S116 wash-cycle golden from proving the intended escalation loop and leaves wash weaker than other self-care goals under the live planning architecture.

## Assumption Reassessment (2026-04-17)

1. The live `GoalKind` under audit is `GoalKind::Wash`, and the front-door candidate gate is `emit_wash_goal(...)` in [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). It currently emits Wash only when `local_controlled_commodity_evidence(..., CommodityKind::Water)` succeeds.
2. The live planner-feasibility surface for Wash is `GoalDispatchDeclaration::Wash` in [crates/worldwake-ai/src/goal_dispatch_decl.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs). It still uses `FeasibilityStrategy::CommodityPresenceCheck`, and [crates/worldwake-ai/src/feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs) treats Wash as likely only when the actor already has water quantity in hand.
3. The live operator family for Wash already includes `PlannerOpKind::Travel` and `PlannerOpKind::MoveCargo`, but not `PlannerOpKind::Harvest` or `PlannerOpKind::Trade`; see `WASH_OPS` in [crates/worldwake-ai/src/goal_dispatch_decl.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs). Under current goal dispatch, a known basin does not imply a lawful water-acquisition chain.
4. The live destination surface for Wash is already belief-correct after archived ticket `S116DRIESCSUS-008`: [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) maps `GoalKind::Wash` to believed `WorkstationTag::WashBasin` places, and focused tests `wash_ignores_unbelieved_remote_wash_basin` / `wash_returns_places_with_wash_basin_belief` already pin that boundary.
5. The authoritative action boundary is unchanged: `wash_preconditions()` in [crates/worldwake-systems/src/needs_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs_actions.rs) still requires the wash target to be a directly possessed `Water` item lot. This ticket therefore does not relax authoritative wash validation; it repairs AI planning so the planner can satisfy that existing precondition lawfully.
6. Existing focused proof already names the front-door mismatch. `wash_requires_dirtiness_and_local_water` in [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) proves Wash disappears entirely once held water is removed, even if dirtiness remains high.
7. Existing golden survival tickets explicitly exclude Wash from their budget-checked survival subset because the live planner cannot yet sustain a lawful wash path through basin discovery/procurement under pressure; see [crates/worldwake-ai/tests/golden_survival_scattered.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_scattered.rs) and [crates/worldwake-ai/tests/golden_survival_contested.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_contested.rs).
8. The exact shared boundary under audit is mixed-layer but still AI-owned: candidate generation (`emit_wash_goal`), goal dispatch feasibility (`GoalDispatchDeclaration::Wash` / `FeasibilityStrategy::CommodityPresenceCheck`), and the planner operator family for Wash (`WASH_OPS`). The authoritative precondition remains fixed input to that boundary.
9. Intended invariant before implementation: if an agent has dirtiness pressure, a believed wash-basin place, and a belief-visible lawful path to procure water under the current authoritative rules, `GoalKind::Wash` should materialize and search should be able to find a plan that acquires water and then commits `wash`. If basin belief is absent, Wash should still remain unavailable.
10. Adjacent contradiction classification: the direct-possession wash action contract is a separate residual bottleneck already named in the S116 spec. That authoritative contract remains separate from this immediate planner-gap fix and is tracked explicitly in follow-up ticket `S116DRIESCSUS-010`.
11. Mismatch + correction: the earlier S116 golden drafting treated “wash basin + co-located water source” as sufficient for a wash-cycle proof. Reassessment against live code showed that is false today because Wash is still gated on already-held water before planning even begins.

## Architecture Check

1. Repairing the single `GoalKind::Wash` planning contract is cleaner than seeding scenario-only held water, weakening the S116 goldens, or adding a second “wash-specific acquisition” story path. The goal should keep naming the world condition (get clean), while the planner composes the lawful prerequisite chain.
2. This ticket preserves one canonical authority path. The `wash` action still consumes directly possessed water; the planner is corrected to satisfy that same precondition through ordinary acquisition operators instead of inventing a parallel shortcut or backwards-compatibility shim.

## Verification Layers

1. Wash candidate admission under dirtiness + basin belief + water procurement path -> focused candidate-generation and decision-trace coverage in `worldwake-ai`
2. Found wash plan includes lawful procurement before terminal wash -> focused planner/search test proving `PlanSearchOutcome::Found` for `GoalKind::Wash` under the repaired operator family
3. No basin belief means no successful Wash plan -> focused planner/goal-model regression, not event-log or committed-action absence alone
4. Authoritative wash precondition remains unchanged -> existing focused runtime/conformance coverage on `wash_preconditions` / `conformance_wash`
5. This ticket does not need golden E2E proof itself; `S116DRIESCSUS-006` remains the golden owner once the planner gap is closed

## What to Change

### 1. Repair Wash candidate admission and feasibility

Rework `GoalKind::Wash` admission so it is no longer hard-gated on already-held water. The repaired contract should keep the fast path where local controlled water makes Wash immediately available, but also allow Wash to materialize when the agent believes in a wash basin and has a belief-visible lawful water-procurement path under current action rules.

Rework the Wash feasibility surface to match that contract. `CommodityPresenceCheck` is no longer truthful once the planner can legally acquire water as a prerequisite.

### 2. Expand Wash's operator family to cover water procurement

Update the Wash planner operator family and any supporting goal-model fallout so search can actually reach the possessed-water `wash` terminal under current authoritative rules. Under the live architecture, this means carrying whatever lawful procurement ops are needed for water acquisition before the terminal wash step rather than treating Wash as a “water already in hand” leaf.

### 3. Add focused planner-root coverage

Add focused AI tests that prove:

- Wash candidate generation still works with local held water
- Wash can now be found through a lawful procurement chain when a basin is believed but water is not yet held
- No believed basin still prevents a found Wash plan

Update active S116 ticket material only where dependency or scope alignment now requires it.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/feasibility.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify if the repaired Wash operator family needs goal-model/binding fallout)
- `crates/worldwake-ai/src/search/tests.rs` or `crates/worldwake-ai/tests/planner_conformance.rs` (modify for focused found-plan proof)
- `tickets/S116DRIESCSUS-006.md` (modify only for dependency alignment if needed)

## Out of Scope

- Relaxing or replacing the authoritative `wash_preconditions()` direct-possession rule
- New golden E2E coverage beyond unblocking ticket `S116DRIESCSUS-006`
- Ranking multiplier changes from the already-landed S116 substrate

## Acceptance Criteria

### Tests That Must Pass

1. Focused regression proves Wash candidate generation no longer requires already-held water when a lawful basin+procurement path is belief-visible.
2. Focused planner/search regression proves `GoalKind::Wash` can produce a found plan that acquires water before the terminal wash under the current authoritative action contract.
3. Focused negative regression proves no believed wash basin still means no found Wash plan.
4. Existing focused runtime parity still passes: `cargo test -p worldwake-ai conformance_wash -- --exact`
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Wash planning must remain belief-only; no authoritative remote basin or water-source reads may be introduced on behalf of the agent.
2. The authoritative `wash` action keeps one lawful water-consumption contract during this ticket; the planner is repaired to satisfy it rather than bypass it.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused regression for Wash admission without pre-held water when a lawful procurement path exists
2. `crates/worldwake-ai/src/search/tests.rs` or `crates/worldwake-ai/tests/planner_conformance.rs` — focused found-plan proof for acquire-water-then-wash under the repaired operator family
3. `crates/worldwake-ai/src/goal_model.rs` — focused negative/positive basin-belief regressions kept aligned with the repaired planner contract

### Commands

1. `cargo test -p worldwake-ai wash_requires_dirtiness_and_local_water -- --exact`
2. `cargo test -p worldwake-ai wash_returns_places_with_wash_basin_belief -- --exact`
3. `cargo test -p worldwake-ai conformance_wash -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Outcome amended: 2026-04-17

Completed on 2026-04-17.

- Reworked `GoalKind::Wash` admission in `crates/worldwake-ai/src/candidate_generation.rs` so Wash is no longer limited to already-held water. The fast path for local controlled water remains, and a second path now emits Wash when a believed wash-basin place also has a lawful belief-visible water procurement path.
- Updated `crates/worldwake-ai/src/goal_dispatch_decl.rs` so Wash uses `FeasibilityStrategy::EvidencePlaceLocal` and expanded `WASH_OPS` to include procurement operators needed for the current authoritative rule.
- Tightened `crates/worldwake-ai/src/goal_model.rs` so synthesized Harvest actions only become actionable when the workstation target is actually co-located with the actor, and modeled harvest output as a hypothetical ground lot at the workstation place rather than a planner-only in-hand shortcut.
- Extended `crates/worldwake-ai/src/planning_state.rs`, `crates/worldwake-ai/src/planner_ops.rs`, and `crates/worldwake-ai/src/search/candidates.rs` so the planner can lawfully continue from harvested hypothetical output through `pick_up` and then `wash`, producing the real search chain `travel -> harvest -> pick_up -> wash`.
- Added focused coverage in `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/feasibility.rs`, `crates/worldwake-ai/src/goal_model.rs`, and `crates/worldwake-ai/src/search/tests.rs` for basin-backed Wash admission, local-evidence feasibility, lawful harvest/pick-up/wash search progression, and the non-shortcut harvest substrate.

## Deviations

- Reassessment proved the lawful Wash chain under the current authoritative rules is not `travel -> harvest -> wash`. Harvest output is not in-hand at commit time, so the planner now models the honest chain `travel -> harvest -> pick_up -> wash`.
- The ticket’s acceptance command `cargo test -p worldwake-ai` is currently blocked by the pre-existing unfinished `S116DRIESCSUS-006` golden file `crates/worldwake-ai/tests/golden_drive_escalation.rs`, which still has failing wash-cycle assertions unrelated to the `009` planner-gap fix. The crate-wide `--lib` suite for `worldwake-ai` does pass after this ticket’s changes.
- To keep CI-matching clippy truthful, I made 2 non-semantic lint-only edits in the pre-existing `golden_drive_escalation.rs` WIP file. Those changes do not alter the still-failing `006` behavior.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::wash_emits_when_basin_and_water_procurement_are_believed -- --exact`
- Passed `cargo test -p worldwake-ai --lib feasibility::tests::test_wash_with_local_evidence_place_likely -- --exact`
- Passed `cargo test -p worldwake-ai --lib feasibility::tests::test_wash_with_water_without_local_evidence_is_uncertain -- --exact`
- Passed `cargo test -p worldwake-ai --lib goal_model::tests::wash_returns_places_with_wash_basin_belief -- --exact`
- Passed `cargo test -p worldwake-ai --lib goal_model::tests::harvest_step_creates_hypothetical_ground_output_without_crediting_actor_inventory -- --exact`
- Passed `cargo test -p worldwake-ai --lib goal_model::tests::acquire_self_consume_goal_is_not_satisfied_before_pickup_after_hypothetical_harvest -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::search_wash_candidates_include_hypothetical_local_water_after_pickup -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::search_wash_finds_harvest_then_wash_plan_at_believed_basin_place -- --exact`
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_wash -- --exact`
- Passed `cargo test -p worldwake-ai --lib`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Ran `cargo test -p worldwake-ai`; it failed only in the pre-existing unfinished `crates/worldwake-ai/tests/golden_drive_escalation.rs` assertions owned by `S116DRIESCSUS-006`
