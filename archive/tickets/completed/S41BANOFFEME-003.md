# S41BANOFFEME-003: Suite 2 — Raid-Belief Economic Cascade (Scenario 48)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None
**Deps**: S41BANOFFEME-001 (spec reassessment), S41BANOFFEME-002 (Suite 1 must pass first to confirm raid mechanics work)

## Problem

No golden test covers the downstream economic impact of bandit raids. The intended causal chain is raid -> witness perception -> belief propagation via the existing generic `ShareBelief`/`tell` path -> merchant route adaptation through planner-local perceived travel cost. That spans Combat, Perception, Beliefs, Social Tell, Enterprise, Travel, and AI while validating Principle 7 (locality), Principle 14 (world state is not belief state), and Principle 3 / Principle 12 (danger remains derived planner data, not authoritative edge state). Without this coverage, the belief-mediated economic impact of raids has zero golden validation.

## Assumption Reassessment (2026-03-30)

1. **Shared abstraction boundary under audit** — this ticket is not "combat causes merchant reroute" in the abstract. The live shared boundary is: `emit_social_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` produces a social tell goal from the witness belief store; `tell` in `crates/worldwake-systems/src/tell_actions.rs` transfers that knowledge; `PlanningSnapshot::perceived_travel_costs` plus `route_threat::perceived_direct_travel_cost_from_memory()` in `crates/worldwake-ai/src/planning_snapshot.rs` and `crates/worldwake-ai/src/route_threat.rs` convert the merchant's acquired danger memory into planner-local route preference.
2. **`GoalKind::ShareBelief { listener, .. }`** — confirmed live at `crates/worldwake-core/src/goal.rs`. The root synthesis surface is already documented in `docs/planner-contracts.md` and implemented through `GoalKindPlannerExt`.
3. **Tell action handler** — confirmed live at `crates/worldwake-systems/src/tell_actions.rs`; `TellTopic::EntityBelief` is a first-class path there. No combat-specific social alias is needed or desired.
4. **Danger-memory substrate** — the route-threat path can read either entity-level `BelievedActivity { action_domain: ActionDomain::Combat, .. }` or `SocialObservationKind::WitnessedConflict` when computing perceived travel cost. The live Scenario 48 golden uses the witnessed-conflict social-observation path because that is the artifact perception lawfully materializes for the witnessed raid setup.
5. **Planner danger-cost integration is already live** — verified by code, not inference. `PlanningSnapshot` builds `perceived_travel_costs` via `perceived_direct_travel_cost_from_memory()`, which uses `route_threat_estimate_from_memory()` over `BelievedActivity { action_domain: Combat }` and witnessed-conflict social observations. Existing coverage includes `crates/worldwake-ai/src/route_threat.rs` unit tests plus Scenario 22 in `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`, which already proves fresh danger beliefs can flip route choice.
6. **Social candidate generation is already generic** — verified by code and tests. `emit_social_candidates()` can relay arbitrary known entity beliefs and relayable social observations to co-located live listeners subject to tell-profile and observability filters; it is not restricted to special topic families. Existing focused coverage includes `social_candidates_emit_for_live_colocated_listeners_and_relayable_subjects` in `crates/worldwake-ai/src/candidate_generation.rs`.
7. **Critical missing setup in the original ticket** — `ShareBelief` does not plan travel-to-listener. The witness and merchant must become co-located through a lawful physical movement path first, and social goldens often also need explicit listener-belief seeding or local-belief refresh once co-located. `docs/golden-e2e-testing.md` calls this out explicitly, and `golden_tell_propagates_political_knowledge` in `crates/worldwake-ai/tests/golden_emergent.rs` uses the same pattern.
8. **Critical missing setup in the original ticket** — merchant restock does not arise from route knowledge alone. The merchant must have `MerchandiseProfile`, `DemandMemory`, and explicit belief/evidence for a lawful replenishment path. Existing coverage includes `restock_requires_profile_demand_gap_and_replenishment_path` in `crates/worldwake-ai/src/candidate_generation.rs` and `merchant_route_knowledge_alone_does_not_unlock_remote_restock` in `crates/worldwake-ai/tests/golden_trade.rs`.
9. **Topology assumption** — T22's existing topology is not suitable for Scenario 48. This suite still needs its own custom topology with Market, DangerousRoad, BanditCamp, SafeRoute, and RemoteFarm so the short dangerous route and longer safe route are both lawful and distinguishable.
10. **Scenario isolation** — the merchant's intended branch is `RestockCommodity { commodity: Apple }` over a known remote orchard source, not general hunger relief or local trade. The witness's intended branch is generic `ShareBelief`, not autonomous travel planning. Healing, multiple merchants, and unrelated production branches stay out of scope.
11. **Pre-belief route-proof surface correction** — the original ticket tried to prove "no omniscient reroute" only through the golden's runtime timeline. A cleaner proof is to keep the full golden for the cross-system information path, and add one focused planner/search test for `RestockCommodity` that compares route selection with and without the acquired danger memory. That avoids overfitting the golden to scheduler staging while still proving the architecture.
12. **Route-cost arithmetic correction** — the live perceived-travel penalty is proportional to edge base ticks (`penalty_ticks = ceil(base_ticks * threat / 1000)`). Under the original `1 + 1` dangerous path versus `2 + 2` safe path, a max-threat belief is not strong enough to make the safe route strictly better in all planner comparisons. The topology must therefore give the first dangerous leg a base cost of 2 ticks so the pre-belief route stays shorter (`2 + 1 = 3` vs `2 + 2 = 4`) while the post-belief perceived route becomes strictly worse (`4 + 2 = 6` vs `4`). This is a scenario-number correction, not an engine change.

### Adjacent Contradictions

- The original ticket and spec still cite stale principle numbers such as `FND-25`, `FND-26`, and `FND-27`. This ticket is corrected to the live `docs/FOUNDATIONS.md` numbering; the spec should be updated separately if the project owner wants docs-sync beyond this ticket.
- No engine gap was exposed during reassessment. The live architecture already has one canonical social transport path and one canonical belief-backed perceived-travel-cost path. Adding bandit-specific aliases here would be worse architecture, not better.

## Architecture Check

1. This should stay test-only. The current architecture already exposes the right generic paths: social propagation goes through `ShareBelief`/`tell`, and route adaptation goes through planner-local perceived travel cost. Extending those tests is cleaner than adding any bandit-specific code path.
2. The golden belongs in `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` alongside the other S41 bandit suites, but the pre-/post-belief route-choice arithmetic is cleaner as a focused planner/search test in AI unit coverage than as extra scheduler choreography inside the golden.
3. The scenario must use lawful physical movement for the witness and explicit merchant source knowledge. That preserves Principle 7 and avoids false negatives caused by missing setup rather than missing architecture.
4. No backwards-compatibility shims, alias paths, or combat-specific social helpers.

## Verification Layers

1. Raid occurs at DangerousRoad -> action trace: `ActionTraceKind::Committed` for `"attack"` by a bandit at DangerousRoad.
2. Witness forms a danger memory -> authoritative witness belief store inspection for `SocialObservationKind::WitnessedConflict` at DangerousRoad.
3. Witness physically reaches the merchant before sharing -> action trace: committed `"travel"` by the witness into Market.
4. Belief propagation uses the canonical social path -> action trace: committed `"tell"` with `listener = merchant`; merchant belief store then contains the transferred `WitnessedConflict` observation with report provenance from the witness.
5. Post-belief merchant adaptation -> decision trace in the golden proves the merchant's selected `RestockCommodity` plan begins with travel toward `SafeRoute`.
6. Pre-belief no-omniscience route preference -> focused planner/search test proves the same `RestockCommodity` setup selects `DangerousRoad` without the danger memory and `SafeRoute` with it.
7. Deterministic replay -> `hash_world()` plus `hash_event_log()` match across two Scenario 48 runs with the same seed.

## What to Change

### 1. Add Suite 2 topology builder

`build_s48_topology()` with 5 places:
- Market
- DangerousRoad
- BanditCamp
- SafeRoute
- RemoteFarm

Edges:
- Market -> DangerousRoad: 2 ticks
- DangerousRoad -> BanditCamp: 1 tick
- DangerousRoad -> RemoteFarm: 1 tick
- Market -> SafeRoute: 2 ticks
- SafeRoute -> RemoteFarm: 2 ticks

### 2. Add Suite 2 setup function

`seed_s48_scenario(h: &mut GoldenHarness) -> S48Ids`:
- 2 bandits at DangerousRoad with faction, camp at BanditCamp, `CombatProfile`, `PerceptionProfile`, and bandit utility
- 1 witness at DangerousRoad with `PerceptionProfile`, an accepting `TellProfile`, and no competing needs pressure
- 1 traveler-victim at DangerousRoad with weak `CombatProfile` and visible food
- 1 merchant at Market with `MerchandiseProfile` (Apple sales), `DemandMemory`, `TradeDispositionProfile`, `PerceptionProfile`, accepting `TellProfile`, and `UtilityProfile` with non-zero `danger_weight` / `enterprise_weight`
- 1 orchard workstation / `ResourceSource` at RemoteFarm producing Apple
- explicit merchant belief seeding for the orchard workstation or equivalent lawful replenishment evidence
- explicit local-belief refresh / listener-belief seeding once witness and merchant are co-located, if needed for `ShareBelief` materialization under the live social contract

### 3. Add `run_s48_scenario(seed: Seed)` function

Multi-phase tick loop:
1. Phase 1: raid phase — bandits attack traveler at DangerousRoad, witness observes, and witness belief store records a witnessed-conflict danger observation.
2. Phase 2: propagation phase — witness reaches Market through ordinary travel, refreshes lawful local beliefs as needed, and executes `tell` to the merchant.
3. Phase 3: adaptation phase — merchant's next `RestockCommodity` selection uses `SafeRoute`.
4. Accumulate assertion flags across all phases, assert at end, return state hashes.

### 4. Add Scenario 48 golden tests

- `golden_raid_belief_economic_cascade` — calls `run_s48_scenario(Seed([48; 32]))`
- `golden_raid_belief_economic_cascade_replays_deterministically` — calls twice, asserts hash equality

### 5. Add one focused planner/search regression test

- Compare the same `RestockCommodity { commodity: Apple }` setup with and without the merchant's acquired danger memory.
- Assert that the selected first travel destination is `DangerousRoad` before the belief and `SafeRoute` after the belief.
- Keep this proof at the planner boundary instead of staging a second fragile runtime branch in the golden.

## Files to Touch

- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify — add Suite 2 tests)
- `crates/worldwake-ai/src/search/tests.rs` (modify — add focused route-choice regression)

## Out of Scope

- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems` production code
- Suite 1 (S41BANOFFEME-002) and Suite 3 (S41BANOFFEME-004)
- Golden inventory updates (S41BANOFFEME-005)
- Modifying existing T22 tests

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_raid_belief_economic_cascade -- --exact` — main test passes
2. `cargo test -p worldwake-ai golden_raid_belief_economic_cascade_replays_deterministically -- --exact` — replay test passes
3. Focused planner regression for pre-/post-belief route choice passes
4. `cargo test -p worldwake-ai` — all existing AI tests still pass (no regressions)

### Invariants

1. Merchant route adaptation is driven by acquired belief state, not authoritative bandit placement the merchant never perceived (Principle 14).
2. Danger memory reaches the merchant only through a lawful information path: witness observation -> witness travel -> `tell` (Principle 7).
3. Route selection change is driven by planner-local perceived travel cost, not stored authoritative edge danger (Principle 3 / Principle 12).
4. Deterministic replay produces identical `StateHash` for world and event log.

## Test Plan

### New/Modified Tests

1. `golden_raid_belief_economic_cascade` — proves the cross-system chain: raid -> witness danger observation -> lawful witness travel -> `tell` -> merchant rerouting
2. `golden_raid_belief_economic_cascade_replays_deterministically` — proves deterministic replay invariant for Suite 2
3. Focused planner/search regression in `crates/worldwake-ai/src/search/tests.rs` — proves no-omniscience route choice at the strongest planner boundary without overfitting the golden to scheduler timing

### Commands

1. `cargo test -p worldwake-ai golden_raid_belief_economic_cascade -- --exact`
2. `cargo test -p worldwake-ai golden_raid_belief_economic_cascade_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai search::tests::search_restock_route_preference_follows_believed_combat_threat -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - Added `golden_raid_belief_economic_cascade` and its deterministic replay companion in `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`.
  - Added `search_restock_route_preference_follows_believed_combat_threat` in `crates/worldwake-ai/src/search/tests.rs` to prove the no-omniscience route-choice boundary directly at planner level.
  - Reassessed and corrected the ticket scope before implementation: no engine changes were required; the live scenario propagates danger through a relayed `WitnessedConflict` social observation; the route-topology arithmetic was adjusted so the safe path is strictly preferred under the current perceived-cost formula.
- Deviations from original plan:
  - The final golden uses the social-observation transport path rather than asserting an entity-level combat belief artifact, because that is the live perception surface for the witnessed raid.
  - The dangerous route's first leg is 2 ticks, not 1, because the original numbers did not create a strict post-belief route flip under the live planner arithmetic.
  - Added one focused planner regression beyond the original two golden tests to keep the no-omniscience proof at the strongest architectural boundary.
- Verification results:
  - `cargo test -p worldwake-ai golden_raid_belief_economic_cascade -- --exact`
  - `cargo test -p worldwake-ai golden_raid_belief_economic_cascade_replays_deterministically -- --exact`
  - `cargo test -p worldwake-ai search::tests::search_restock_route_preference_follows_believed_combat_threat -- --exact`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
