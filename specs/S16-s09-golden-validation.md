**Status**: READY

# S16: S09 Golden Validation Suite — Behavioral Promises

## Summary

Add four golden E2E tests proving the emergent behavioral promises of S09 (Indefinite Action Re-Evaluation + Travel-Aware Plan Search). The existing `golden_defend_replans_after_finite_stance_expires` test only proves the mechanical defend lifecycle. It does NOT prove:

1. Re-evaluation leads to **different goals** when the world changed during the stance.
2. Multiple agents **diverge** in post-defend behavior due to profile diversity (Principle 20).
3. Agents fluidly **cross domain boundaries** (combat → needs/care) after stance expiry.
4. Spatial awareness **enables plans** that would otherwise fail at hub nodes.

These are the core behavioral promises of S09 — without golden coverage, regressions could silently break emergent behavior.

## Discovered Via

Review of S09 deliverables against golden test coverage. The only existing defend golden (`golden_defend_replans_after_finite_stance_expires` in `golden_combat.rs`) asserts that defend commits, the agent replans, and the agent does *something* after. It does not assert *what* the agent does or *why*, which is where the emergent value lies.

## Foundation Alignment

- **Principle 1** (Maximal Emergence): All four scenarios prove emergent chains where outcomes are not hardcoded — they arise from concrete state + utility-driven AI ranking.
- **Principle 19** (Intentions Are Revisable Commitments): Tickets 1–3 prove that agents revise their intentions after defend expiry based on current local evidence.
- **Principle 20** (Agent Diversity): Ticket 2 proves that per-agent profile parameters (defend_stance_ticks, utility weights) produce divergent autonomous behavior.
- **Principle 18** (Resource-Bounded Practical Reasoning): Ticket 4 proves that A* heuristics + travel pruning make multi-hop plans reachable within the default planning budget.

## Phase

Post-Phase-2 hardening. No engine changes required.

## Crates

- `worldwake-ai` (test file only — `golden_combat.rs`)

## Dependencies

- S09 (Indefinite Action Re-Evaluation) — COMPLETED, archived
- S09 (Travel-Aware Plan Search) — COMPLETED, archived
- Existing `golden_harness` infrastructure — no new utilities needed

## Engine Changes

None. Tests only.

---

## Tickets

### S16GOLDVAL-001: Defend Re-Evaluation Under Changed Conditions

**Goal**: Prove that after a finite defend stance expires, an agent re-evaluates and selects a *different* goal when the world changed during the stance — specifically, switching from combat to self-care when the threat dies.

**Test file**: `crates/worldwake-ai/tests/golden_combat.rs`

**Setup**:
- `GoldenHarness::new(Seed([50; 32]))`
- Two agents at VillageSquare:
  - **Defender**: AI-controlled, `CombatProfile` with `defend_stance_ticks: nz(3)`, pre-seeded with a clotted wound (severity ~120, bleed_rate 0), hunger at ~300 (moderate). Give Bread × 1 and Medicine × 1. Set `no_recovery_combat_profile()` pattern (natural_recovery_rate: 0) so wounds only decrease through medicine.
  - **Doomed Attacker**: AI-controlled, near-lethal deprivation state: hunger at pm(950), `DeprivationExposure { hunger_critical_ticks: 2, .. }`, `MetabolismProfile` with high hunger tick rate (pm(50)). Give `CombatProfile` with low wound_capacity (pm(200)). Seed existing starvation wound (severity ~150, bleed_rate 0).
- Add hostility: Attacker → Defender.
- Seed the Defender into an active defend action (duration 3 ticks) with `CombatStance::Defending`, following the pattern in `golden_defend_replans_after_finite_stance_expires`.
- Seed local beliefs for both agents via `seed_actor_local_beliefs`.
- Enable decision tracing and action tracing.

**Observation loop** (up to 60 ticks):
1. Track whether the Doomed Attacker dies (`h.agent_is_dead(attacker)`).
2. Track the Defender's defend commit tick via action trace.
3. After defend commits AND attacker is dead, track the Defender's next selected goal via decision trace — it should NOT be `ReduceDanger` (threat is dead). Check for `ConsumeCommodity` (eat) or `TreatWounds` (self-heal).
4. Track whether the Defender's wound load decreases or hunger decreases.

**Assertions**:
1. The seeded defend action commits within the first ~5 ticks.
2. The Doomed Attacker dies from deprivation during or shortly after the stance.
3. After defend commit + attacker death, the Defender's next selected goal is NOT `ReduceDanger` — it is a self-care or needs goal (e.g., `TreatWounds { patient: defender }` or `ConsumeCommodity`).
4. The Defender eventually takes a non-combat action (eat or heal observable via state delta).
5. Deterministic replay: run twice with same seed, assert `(hash_world, hash_event_log)` match.

**Emergence proof**: The Defender does not blindly re-enter combat after the stance. The world changed (threat died), so re-evaluation produces a different goal. This is Principle 19 in action — the agent revises its intention based on current evidence.

---

### S16GOLDVAL-002: Multi-Agent Divergent Re-Evaluation

**Goal**: Prove that two defenders with different `defend_stance_ticks` and different `UtilityProfile` weights independently choose *different* post-combat goals after their stances expire at staggered times. This is Principle 20 (Agent Diversity) through profile-driven timing + utility divergence.

**Test file**: `crates/worldwake-ai/tests/golden_combat.rs`

**Setup**:
- `GoldenHarness::new(Seed([51; 32]))`
- Three agents at VillageSquare:
  - **DefenderA**: `defend_stance_ticks: nz(3)`, `UtilityProfile { pain_weight: pm(800), hunger_weight: pm(300), .. }`. Pre-seeded wound (severity ~200, clotted). Moderate hunger pm(500). Give Medicine × 1 and Bread × 1. Use `no_recovery_combat_profile()` base with overridden defend_stance_ticks.
  - **DefenderB**: `defend_stance_ticks: nz(8)`, `UtilityProfile { pain_weight: pm(300), hunger_weight: pm(800), .. }`. Pre-seeded wound (severity ~200, clotted). Moderate hunger pm(500). Give Medicine × 1 and Bread × 1. Use `no_recovery_combat_profile()` base with overridden defend_stance_ticks.
  - **Doomed Threat**: Same pattern as Ticket 1 — near-lethal deprivation, will die within ~5 ticks.
- Add hostility: Threat → DefenderA, Threat → DefenderB.
- Seed both defenders into active defend actions with their respective durations and `CombatStance::Defending`.
- Seed local beliefs for all agents.
- Enable decision tracing and action tracing.

**Observation loop** (up to 80 ticks):
1. Track DefenderA's defend commit tick (should be ~tick 3).
2. Track DefenderB's defend commit tick (should be ~tick 8).
3. After each defender's defend commits AND threat is dead, record their first selected non-combat goal kind via decision trace.
4. Track state deltas: which defender heals first vs eats first.

**Assertions**:
1. DefenderA's defend commits before DefenderB's (staggered timing from different `defend_stance_ticks`).
2. The Doomed Threat dies from deprivation.
3. DefenderA (pain_weight=800) first takes a heal/care action (wound load decreases before hunger decreases).
4. DefenderB (hunger_weight=800) first takes an eat action (hunger decreases before wound load decreases).
5. Both defenders eventually address both needs (heal and eat) — the difference is *ordering*.
6. Deterministic replay.

**Emergence proof**: Same world state, same threat death, but two agents diverge in post-combat behavior purely due to profile parameters. No special-case code distinguishes them — it emerges from the shared ranking pipeline applied to different utility weights.

---

### S16GOLDVAL-003: Combat→Non-Combat Domain Crossing

**Goal**: Prove that an agent in a defend stance with no living threat fluidly transitions through multiple non-combat domains (needs → care) after stance expiry. This tests the absence of "combat lock-in" — the defend→commit→replan cycle must cleanly hand off to non-combat goal families.

**Test file**: `crates/worldwake-ai/tests/golden_combat.rs`

**Setup**:
- `GoldenHarness::new(Seed([52; 32]))`
- Single agent at VillageSquare:
  - **Fighter**: `defend_stance_ticks: nz(5)`, `UtilityProfile { hunger_weight: pm(700), pain_weight: pm(500), .. }`. High hunger pm(700). Pre-seeded wound (severity ~300, clotted). Give Bread × 2 and Medicine × 1. Use `no_recovery_combat_profile()` base.
- NO hostile agents present — the Fighter was seeded into defend preemptively (or the threat was already dead).
- Seed Fighter into an active defend action (duration 5 ticks) with `CombatStance::Defending`.
- Seed local beliefs.
- Enable decision tracing and action tracing.

**Observation loop** (up to 60 ticks):
1. Track defend commit tick via action trace.
2. After defend commits, track the sequence of goal kinds selected via decision trace.
3. Track state deltas: hunger decrease (eat), wound load decrease (heal).

**Assertions**:
1. The defend action commits after ~5 ticks.
2. After defend commit, the Fighter does NOT select `ReduceDanger` (no threat exists).
3. The Fighter's first post-defend action addresses the highest-pressure need. With hunger_weight=700 at hunger=700 vs pain_weight=500 at wound=300, hunger should dominate — first action should be eat (hunger decreases).
4. After eating, the Fighter addresses wound care (wound load decreases via medicine).
5. The Fighter transitions through at least two domain families: needs (eat) → care (heal).
6. Deterministic replay.

**Emergence proof**: The defend→needs→care chain crosses three goal-family domains without any orchestrator. The combat system's defend commit simply releases the agent back to the decision cycle; the ranking pipeline selects the highest-pressure non-combat goal. This is Principle 1 (maximal emergence) and Principle 24 (systems interact only through state).

---

### S16GOLDVAL-004: Spatial Awareness Enables Multi-Hop Plan

**Goal**: Prove that the A* heuristic and travel pruning from S09 enable an agent at VillageSquare (7 outgoing edges) to find a multi-hop plan to a remote resource within the default planning budget. Without spatial awareness, the search would exhaust its budget exploring all 7 directions equally.

**Test file**: `crates/worldwake-ai/tests/golden_combat.rs` (or a new `golden_spatial.rs` if preferred — but combat file already has the travel pattern from `golden_death_while_traveling`)

**Setup**:
- `GoldenHarness::new(Seed([53; 32]))`
- Single agent at VillageSquare:
  - **HungryTraveler**: Critical hunger pm(850), sated on all other needs. Default `MetabolismProfile`. Default `UtilityProfile` (hunger_weight dominates at critical level). No known recipes except `RecipeId(0)` (harvest apples). Give the agent a `PerceptionProfile` (to observe entities at destination).
- NO food at VillageSquare or any adjacent 1-hop place.
- Place an `OrchardRow` workstation with `ResourceSource { commodity: Apple, available_quantity: 10, .. }` at OrchardFarm (3 hops from VillageSquare: VS→SouthGate→EastFieldTrail→OrchardFarm, 7 travel ticks).
- Seed world beliefs for the agent via `seed_actor_world_beliefs` so the agent knows about the remote resource.
- Enable decision tracing.

**Observation loop** (up to 100 ticks):
1. Track whether the agent leaves VillageSquare (`h.world.effective_place(agent) != Some(VILLAGE_SQUARE)`).
2. Track whether the agent reaches OrchardFarm.
3. Track whether the agent starts a harvest action at OrchardFarm.
4. Track whether the agent's hunger eventually decreases (eats harvested apple).

**Assertions**:
1. The agent leaves VillageSquare within the first ~10 ticks (plan found, travel begins).
2. The agent reaches OrchardFarm (not stuck at any intermediate hub).
3. At OrchardFarm, the agent performs a harvest action (active action name == "harvest").
4. The agent's hunger eventually decreases (eats the harvested apple).
5. Decision trace at tick 0 shows a `Planning` outcome with a selected plan that includes travel steps toward OrchardFarm (not random directions).
6. Deterministic replay.

**Emergence proof**: This test would fail without S09's A* heuristic. At VillageSquare with 7 outgoing edges, a blind uniform-cost search exhausts the default 512-node budget exploring all directions equally. The A* heuristic guides expansion toward SouthGate (the correct direction) and travel pruning eliminates the other 6 directions, making the 3-hop plan reachable. This is not a performance test — it tests *reachability*: the plan literally cannot be found without spatial awareness.

**Note on existing coverage**: The `golden_death_while_traveling` test already exercises travel from BanditCamp to OrchardFarm, but that agent starts 1 hop away from the food source, bypassing the hub branching problem. This test specifically starts at the 7-edge hub to exercise the worst-case search scenario.

---

## Verification

After implementing the four tickets:

```bash
cargo test -p worldwake-ai golden_defend_changed_conditions
cargo test -p worldwake-ai golden_multi_agent_divergent
cargo test -p worldwake-ai golden_combat_to_noncombat_domain
cargo test -p worldwake-ai golden_spatial_multi_hop_plan
cargo test --workspace  # full regression check
```

Each test should also have a `_replays_deterministically` variant using the standard two-run hash comparison pattern.

## Cross-References

- **S09 (Indefinite Action Re-Evaluation)**: `archive/specs/S09-indefinite-action-re-evaluation.md` — tickets 1–3 exercise the finite defend→replan cycle.
- **S09 (Travel-Aware Plan Search)**: `archive/specs/S09-travel-aware-plan-search.md` — ticket 4 exercises A* heuristic plan reachability.
- **S07 (Care Intent & Treatment Targeting)**: Tickets 1–3 use the `TreatWounds` goal and `no_recovery_combat_profile()` pattern established in S07 golden tests.
- **S02 (Goal Decision Policy Unification)**: The ranking pipeline that produces divergent behavior in ticket 2 is the shared `evaluate_suppression()` + `rank_candidates()` path from S02.
- **Existing defend golden**: `golden_defend_replans_after_finite_stance_expires` in `golden_combat.rs` — the mechanical lifecycle test that S16 extends with behavioral assertions.
- **Existing travel golden**: `golden_death_while_traveling` in `golden_combat.rs` — exercises BanditCamp→OrchardFarm travel but does not stress the VillageSquare hub branching.
- **Prototype world topology**: `crates/worldwake-core/src/topology.rs` — VillageSquare has 7 outgoing edges (GeneralStore, CommonHouse, RulersHall, GuardPost, PublicLatrine, SouthGate, NorthCrossroads). VillageSquare→OrchardFarm is 3 hops / 7 travel ticks via SouthGate→EastFieldTrail→OrchardFarm.
