# S41BANOFFEME-003: Suite 2 — Raid-Belief Economic Cascade (Scenario 48)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Potentially — see reassessment item 4 and 5
**Deps**: S41BANOFFEME-001 (spec reassessment), S41BANOFFEME-002 (Suite 1 must pass first to confirm raid mechanics work)

## Problem

No golden test covers the downstream economic impact of bandit raids. The causal chain — raid → witness perception → belief propagation via Tell → merchant route adaptation — spans 5 systems (Combat, Perception, Beliefs, Social Tell, Enterprise/Travel AI) and validates FND-7 (Locality), FND-14 (World State != Belief State), and FND-27 (Derived Summaries Are Caches). Without this coverage, the belief-mediated economic impact of raids has zero E2E validation.

## Assumption Reassessment (2026-03-30)

1. **`GoalKind::ShareBelief { listener, .. }`** — confirmed live at `crates/worldwake-core/src/goal.rs:63`. Candidate generation must emit this for witnesses of combat events.
2. **Tell action handler** — confirmed at `crates/worldwake-systems/src/tell_actions.rs`. Registered in `crates/worldwake-systems/src/action_registry.rs`.
3. **`BelievedActivity { action_domain: ActionDomain::Combat, .. }`** — confirmed as a belief-state field at `crates/worldwake-core/src/belief.rs`. The T22 test uses `seed_danger_belief()` to inject these manually.
4. **Planner danger-cost integration** — **NEEDS VERIFICATION**: The spec assumes the merchant's planner weights travel routes using danger beliefs (BelievedActivity with Combat domain) to prefer safer routes. This requires the planner's travel-cost heuristic to read the agent's belief store for danger observations along candidate routes. **If the planner does not currently incorporate danger beliefs into route costing, this suite requires an engine change to the planner's travel-cost computation**. The T22 test already demonstrates that danger beliefs affect route selection (fresh travelers avoid BanditRoad), which suggests this mechanism exists. Cite: `latest_traveler_selected_travel_destination()` in `golden_t22_bandit_camp_destruction.rs:414–441` shows travelers choosing SafeFarm over BanditRoad based on danger beliefs.
5. **ShareBelief candidate generation for combat observations** — **NEEDS VERIFICATION**: Does `emit_share_belief_candidates()` in `candidate_generation.rs` generate ShareBelief candidates when an agent holds a `BelievedActivity { action_domain: Combat }` belief about a co-located or recently-observed entity? If not, the witness won't spontaneously share what they saw. Check whether the candidate generation emits ShareBelief for danger-relevant beliefs or only for specific trigger types.
6. **`MerchandiseProfile`** — confirmed at `crates/worldwake-core/src/trade.rs`. Required for merchant enterprise goal generation.
7. **`DemandMemory`** — confirmed at `crates/worldwake-core/src/trade.rs`. Required for merchant restock motivation.
8. **Enterprise restock goal generation** — The merchant needs to generate `RestockCommodity` goals based on `DemandMemory` and `MerchandiseProfile`. This is existing functionality from E11/E13.
9. **4-place + RemoteFarm topology** — Not covered by T22's existing topology. This suite needs its own `build_s48_topology()` function with Market, DangerousRoad, BanditCamp, SafeRoute, RemoteFarm and appropriate travel edges (1-tick short routes through danger, 4-tick long safe route).
10. **Scenario isolation**: The test intentionally excludes healing, multiple merchants, and production concerns. The merchant's only goal pressure is restock. Witness is a non-merchant, non-faction agent whose only relevant behavior is ShareBelief after observing combat.

### Adjacent Contradictions

- If reassessment item 4 reveals the planner does NOT use danger beliefs for route costing, that's a **separate engine gap** that should become its own ticket (not part of S41). Suite 2 would then be blocked on that prerequisite.
- If reassessment item 5 reveals ShareBelief is not generated for combat observations, that's similarly a separate candidate-generation gap ticket.

## Architecture Check

1. Extends `golden_t22_bandit_camp_destruction.rs` — co-locates with other bandit golden tests.
2. Suite 2's topology is distinct from T22 and Suite 1. Uses its own builder function and entity ID range.
3. The test proves a 5-hop causal chain through 5 separate systems via state-mediated interaction (FND-26). No cross-system imperative calls needed.
4. No backwards-compatibility shims.

## Verification Layers

1. Raid occurs at DangerousRoad → action trace: `ActionTraceKind::Committed` for `"attack"` by bandit at DangerousRoad
2. Witness forms danger belief → belief store inspection: witness has `BelievedActivity { action_domain: Combat }` for bandit after raid tick
3. Belief propagation to merchant → action trace: `"tell"` committed with listener=merchant, OR merchant belief store contains danger belief after traversal/Tell
4. Merchant pre-belief route selection → decision trace: before danger belief, merchant's restock plan routes through DangerousRoad (shorter path)
5. Merchant post-belief route adaptation → decision trace: after danger belief, merchant's restock plan routes through SafeRoute (longer but safe path)
6. No omniscient rerouting → decision traces in pre-belief ticks show DangerousRoad route preference
7. Deterministic replay → `hash_world()` + `hash_event_log()` match across two runs with same seed

## What to Change

### 1. Add Suite 2 topology builder

`build_s48_topology()` with 5 places:
- Market, DangerousRoad, BanditCamp, SafeRoute, RemoteFarm
- Market → DangerousRoad: 1 tick
- DangerousRoad → BanditCamp: 1 tick
- DangerousRoad → RemoteFarm: 1 tick
- Market → SafeRoute: 2 ticks; SafeRoute → RemoteFarm: 2 ticks (total 4 ticks, safe path)

### 2. Add Suite 2 setup function

`seed_s48_scenario(h: &mut GoldenHarness) -> S48Ids`:
- 2 bandits at DangerousRoad with faction, camp at BanditCamp, elevated hunger, `CombatProfile`, `PerceptionProfile`
- 1 witness (non-faction, non-merchant) at DangerousRoad with `PerceptionProfile`
- 1 traveler-victim at DangerousRoad with weak `CombatProfile` and food items
- 1 merchant at Market with `MerchandiseProfile` (Apple sales), `DemandMemory`, restock beliefs pointing to RemoteFarm, `PerceptionProfile`, `UtilityProfile` (non-zero `danger_weight`, `enterprise_weight`)
- `ResourceSource` at RemoteFarm producing Apple

### 3. Add `run_s48_scenario(seed: Seed)` function

Multi-phase tick loop:
1. Phase 1: Raid phase — bandits attack traveler at DangerousRoad, witness observes
2. Phase 2: Propagation phase — witness travels to Market, executes Tell to merchant (or merchant traverses DangerousRoad)
3. Phase 3: Adaptation phase — merchant's next restock plan avoids DangerousRoad
4. Accumulate assertion flags across all phases, assert at end, return state hashes

### 4. Add two test functions

- `golden_raid_belief_economic_cascade` — calls `run_s48_scenario(Seed([48; 32]))`
- `golden_raid_belief_economic_cascade_replays_deterministically` — calls twice, asserts hash equality

## Files to Touch

- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify — add Suite 2 tests)

## Out of Scope

- Engine changes to planner travel-cost heuristic (if needed, becomes its own prerequisite ticket)
- Engine changes to ShareBelief candidate generation (if needed, becomes its own prerequisite ticket)
- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems` production code
- Suite 1 (S41BANOFFEME-002) and Suite 3 (S41BANOFFEME-004)
- Golden inventory updates (S41BANOFFEME-005)
- Modifying existing T22 tests

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_raid_belief_economic_cascade -- --exact` — main test passes
2. `cargo test -p worldwake-ai golden_raid_belief_economic_cascade_replays_deterministically -- --exact` — replay test passes
3. `cargo test -p worldwake-ai` — all existing golden tests still pass (no regressions)

### Invariants

1. Merchant does NOT reroute before acquiring danger belief (no omniscience — FND-14).
2. Danger belief reaches merchant only through a lawful information path (direct observation or Tell action — FND-7).
3. Route selection change is driven by belief state, not authoritative world state (FND-27 — danger estimate is a planning heuristic, not stored edge weight).
4. Deterministic replay produces identical `StateHash` for world and event log.

## Test Plan

### New/Modified Tests

1. `golden_raid_belief_economic_cascade` — proves the 5-hop causal chain: raid → witness perception → belief propagation → merchant rerouting
2. `golden_raid_belief_economic_cascade_replays_deterministically` — proves deterministic replay invariant for Suite 2

### Commands

1. `cargo test -p worldwake-ai golden_raid_belief_economic_cascade` — targeted suite
2. `cargo test -p worldwake-ai` — full AI crate regression
3. `cargo clippy --workspace` — no warnings
