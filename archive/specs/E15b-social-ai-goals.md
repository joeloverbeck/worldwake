**Status**: COMPLETED

# E15b: Social AI Goals + Golden E2E Test Suites

## Context

Epic E15 (Social Information Transmission) delivered Tell as a mechanical action, belief degradation through rumor chains, discovery events, and bystander observation. However, the AI planner cannot autonomously initiate Tell — there is no `GoalKind`, `PlannerOpKind`, or candidate generation for social goals. The E15 spec explicitly deferred this: "Future work: Tell goal generation and mismatch-driven investigation goal generation." No subsequent spec (E16-E22) picks up this deferred work.

Additionally, the golden E2E test suite (48 tests across 6 files) has zero coverage of E15 features — no Tell-related scenarios, no discovery-driven replanning tests.

This plan addresses both gaps with two deliverables.

---

## Deliverable 1: Social AI Goal Integration

### 1.1 GoalKind Addition

Add to `crates/worldwake-core/src/goal.rs`:

```rust
GoalKind::ShareBelief {
    listener: EntityId,
    subject: EntityId,
}
```

GoalKey extraction follows the `BuryCorpse { corpse, burial_site }` precedent:
- `entity: Some(listener)`
- `place: Some(subject)` (reuses place slot as second discriminator)

Add `GoalKindTag::ShareBelief` to `crates/worldwake-ai/src/goal_model.rs`.

### 1.2 PlannerOpKind Addition

Add to `crates/worldwake-ai/src/planner_ops.rs`:

```rust
PlannerOpKind::Tell
```

With semantics:
- `may_appear_mid_plan: false` (standalone goal only)
- `is_materialization_barrier: false`
- `transition_kind: GoalModelFallback`
- `relevant_goal_kinds: &[GoalKindTag::ShareBelief]`

### 1.3 UtilityProfile Extension

Add `social_weight: Permille` to `crates/worldwake-core/src/utility_profile.rs`:
- Default: `Permille(200)` (low priority, below enterprise_weight's 500)
- Controls how strongly an agent values sharing information

### 1.4 Candidate Generation

Add `emit_social_candidates()` as 5th emitter in `crates/worldwake-ai/src/candidate_generation.rs`:

1. Check if agent has `TellProfile`. If not, skip.
2. Get agent's `AgentBeliefStore` beliefs.
3. Get co-located alive agents (from belief view).
4. For each co-located agent (up to `TellProfile.max_tell_candidates`):
   - For each belief passing `max_relay_chain_len` filter:
     - Emit `GoalKind::ShareBelief { listener, subject }`.
5. Filter by `BlockedIntentMemory`.

Candidate count bounded by `max_tell_candidates * memory_capacity` (defaults: 3 * 12 = 36 max).

### 1.5 Goal Ranking

In `crates/worldwake-ai/src/ranking.rs`:

**Suppression**: ShareBelief suppressed when `danger_high_or_above() || self_care_high_or_above()` (same as LootCorpse/BuryCorpse). Starving agents do not gossip.

**Priority class**: `GoalPriorityClass::Low` baseline, promotable to Medium via social_weight scaling. ShareBelief should never outrank survival (Critical/High).

**Motive score**: `social_weight * social_pressure()` where social_pressure derives from:
- Count of beliefs with fresh PerceptionSource (DirectObservation, Report with low chain_len)
- Recency factor from `observed_tick` vs `current_tick`
- This is a derived computation, never stored (Principle 3)

### 1.6 Tell Motivation Justification (FND-01 Section H)

**Why this is principle-compliant:**
- Principle 2: `social_weight` is a concrete agent property (explicitly allowed)
- Principle 3: Pressure derived from concrete belief store state (source type, observed_tick, count)
- Principle 7: Agent queries only own beliefs + co-located agents visible locally
- Principle 20: Different social_weight values produce agent diversity
- Principle 23: Rumors are explicitly first-class carriers of consequence

**Positive feedback loop**: A tells B → B tells C → C tells D (information amplification)

**Physical dampeners** (all world-grounded, no clamps):
1. Tell duration (2 ticks) — max 1 belief shared per 2 ticks
2. Action slot occupancy — telling prevents eating, trading, crafting, traveling
3. Memory capacity — bounded belief store
4. Memory retention — stale beliefs expire
5. Chain length filtering — max_relay_chain_len prevents deep relay
6. Acceptance fidelity — listener may reject
7. Priority suppression — ShareBelief capped at Low/Medium, suppressed by High needs
8. Candidate bound — max_tell_candidates limits affordances
9. Co-location requirement — must physically be near listener

**Stored vs derived:**
- Stored: `social_weight` (UtilityProfile field), `TellProfile` (existing), `AgentBeliefStore` (existing)
- Derived: social pressure, candidate enumeration, priority class, motive score

### 1.7 Deferred Work (Not In This Spec)

`GoalKind::InvestigateMismatch { subject, last_known_place }` — agent travels to verify a rumor. Substantial enough for a separate spec. Acknowledged as future work.

---

## Deliverable 2: Golden E2E Test Suites

### 2.1 File Organization

Create `crates/worldwake-ai/tests/golden_social.rs` — new domain file paralleling golden_combat.rs, golden_trade.rs, etc.

### 2.2 Harness Extensions

Add to `crates/worldwake-ai/tests/golden_harness/mod.rs`:
- `seed_agent_with_tell_profile(tell_profile, perception_profile)` — or extend existing `seed_agent()`
- `seed_belief(agent, subject, believed_state)` — inject a specific BelievedEntityState
- `agent_belief_about(agent, subject) -> Option<&BelievedEntityState>` — query accessor
- `agent_belief_count(agent) -> usize` — count accessor

### 2.3 Tier 1 Tests (Work With Current Code)

These use InputQueue to manually inject Tell actions, then verify AI reacts.

**T1: `golden_tell_transmits_belief_and_listener_replans`**
- Setup: Alice and Bob at Village Square. Bob hungry, no food, no knowledge of remote orchard. Orchard Farm has apples. Alice has DirectObservation belief about orchard.
- Inject: Tell(Alice → Bob) about orchard via InputQueue.
- Expected: Bob receives Report belief → generates AcquireCommodity goal → plans Travel to Orchard Farm → harvests apples.
- Systems: Tell handler, belief update, AI candidate generation, planner (Travel+Harvest), scheduler
- Checks: Conservation, determinism replay

**T2: `golden_rumor_chain_degrades_through_three_agents`**
- Setup: 3 agents (Alice, Bob, Carol) co-located. Alice has DirectObservation.
- Inject: Tell(Alice→Bob), then Tell(Bob→Carol).
- Expected: Alice's DirectObservation → Bob's Report{chain_len:1} → Carol's Rumor{chain_len:2}. Confidence ordering: Alice > Bob > Carol.
- Systems: Tell source degradation, confidence derivation
- Checks: Determinism

**T3: `golden_discovery_depleted_resource_triggers_replan`**
- Setup: Agent at Orchard Farm with stale belief that orchard has apples (Quantity(10)). Actually Quantity(0). Agent hungry, plans to harvest.
- Expected: Passive observation fires → InventoryDiscrepancy Discovery event → agent replans (seeks alternative food source or switches goal).
- Systems: Perception mismatch detection, Discovery emission, AI replan/interrupt
- Checks: Conservation, determinism

**T4: `golden_skeptical_listener_rejects_told_belief`**
- Setup: 2 agents co-located. Listener has `acceptance_fidelity: Permille(0)`. Speaker has fresh belief.
- Inject: Tell(Speaker→Listener).
- Expected: Tell completes but listener's belief store unchanged. No travel to rumored location.
- Systems: Tell acceptance check, AI (no new goals from rejected info)
- Checks: Determinism

**T5: `golden_bystander_sees_telling_but_gets_no_belief`**
- Setup: 3 agents co-located. A tells B about remote resource. C is bystander.
- Inject: Tell(A→B).
- Expected: C records WitnessedTelling observation. C has NO belief about resource. C does NOT travel there.
- Systems: Perception (bystander), belief isolation, information locality (Principle 7)
- Checks: Determinism

**T6: `golden_stale_belief_travel_reobserve_replan`** (existing backlog item)
- Setup: Agent at Village Square with stale belief (observed 20 ticks ago) about apples at Orchard Farm. Orchard depleted. Agent hungry.
- Expected: Agent plans Travel → arrives at Orchard Farm → passive observation → InventoryDiscrepancy Discovery → replans.
- Systems: Belief-only planning, travel, passive perception, discovery, replan
- Checks: Conservation, determinism

**T7: `golden_entity_missing_discovery_updates_belief`**
- Setup: Agent A believes Agent B at Village Square. B has since traveled to Orchard Farm. A travels to Village Square.
- Expected: A arrives → passive observation does NOT see B → EntityMissing Discovery fires → A's belief about B updated.
- Systems: Passive observation, EntityMissing detection, belief update
- Checks: Determinism

### 2.4 Tier 2 Tests (Require Deliverable 1)

**T8: `golden_agent_autonomously_tells_colocated_peer`**
- Setup: 2 agents, low needs (no survival pressure). Agent A has high social_weight (900) and fresh DirectObservation beliefs. Agent B has no beliefs. Both have TellProfile.
- Expected: A generates ShareBelief goal → plans Tell → executes → B receives Report belief.
- Systems: Social candidate generation, goal ranking, planner search, Tell execution, belief transfer
- Checks: Conservation, determinism

**T9: `golden_survival_needs_suppress_social_goals`**
- Setup: Agent with high social_weight and fresh beliefs, but critically hungry. Food available.
- Expected: Agent eats first (ConsumeOwnedCommodity outranks ShareBelief). After hunger satisfied, may then Tell.
- Systems: Goal ranking priority hierarchy, suppression logic
- Checks: Priority ordering (Critical > Low/Medium)

**T10: `golden_information_cascade_enables_trade`**
- Setup: 3-place topology. Merchant at Market with unmet apple demand. Farmer at Farm with apple production. Traveler with knowledge of both. Traveler visits Farm.
- Expected: Traveler tells Farmer about Market demand → Farmer generates Restock goal → produces apples → travels to Market → trade occurs. Cross-system chain that was impossible without information transmission.
- Systems: Tell → belief → enterprise candidate generation → production → travel → trade
- Checks: Conservation (commodities + coins across full chain), determinism

**T11: `golden_chain_length_filtering_stops_gossip`**
- Setup: 4 agents co-located. A has DirectObservation. B, C have max_relay_chain_len=3. D has max_relay_chain_len=1.
- Expected: A→B (Report, chain 1). B→C (Rumor, chain 2). C→D blocked (chain would be 3, D's max is 1). Information stops at C.
- Systems: TellProfile filtering, candidate generation bounds, chain depth
- Checks: Determinism, no infinite propagation

**T12: `golden_agent_diversity_in_social_behavior`**
- Setup: 3 agents with social_weight: Gossip (900), Normal (200), Loner (0). All have fresh beliefs, low needs.
- Expected: Gossip Tells quickly. Normal Tells eventually. Loner never generates ShareBelief.
- Systems: UtilityProfile-driven candidate generation, ranking variation (Principle 20)
- Checks: Determinism, diversity verification

**T13: `golden_rumor_leads_to_wasted_trip_then_discovery`**
- Setup: Agent receives Rumor via Tell about apples at Orchard Farm. Orchard actually depleted.
- Expected: Agent plans Travel → arrives → passive observation → InventoryDiscrepancy Discovery → replans. Rumor's low confidence replaced by DirectObservation of empty state.
- Systems: Tell → belief → travel plan → perception → discovery → replan. Full information lifecycle.
- Checks: Conservation, determinism, belief source upgrade (Rumor → DirectObservation)

### 2.5 Coverage Report Update

Update `reports/golden-e2e-coverage-analysis.md`:
- Add Part 1 section for social information scenarios (T1-T13)
- Update coverage matrix with Social domain
- Remove backlog item "stale belief → travel to depleted source → re-observation → replan" (covered by T6)
- Add new GoalKind coverage (ShareBelief)
- Add new ActionDomain coverage note (Social already covered mechanically)
- Note deferred: InvestigateMismatch goal as future backlog

---

## Implementation Sequence

**Phase A** — Tier 1 tests (no spec changes, current code):
1. Extend golden_harness with belief seeding + Tell helpers
2. Create golden_social.rs with T1-T7
3. Verify all pass: `cargo test -p worldwake-ai --test golden_social`

**Phase B** — Write spec:
1. Write `specs/E15b-social-ai-goals.md` with full FND-01 Section H analysis
2. Create tickets following project convention

**Phase C** — Implement spec:
1. Add `social_weight` to UtilityProfile (worldwake-core)
2. Add GoalKind::ShareBelief, GoalKindTag::ShareBelief (worldwake-core, worldwake-ai)
3. Add PlannerOpKind::Tell with PlannerOpSemantics (worldwake-ai)
4. Implement emit_social_candidates() (worldwake-ai)
5. Add ranking logic for ShareBelief (worldwake-ai)
6. Wire GoalKindPlannerExt for ShareBelief (worldwake-ai)

**Phase D** — Tier 2 tests:
1. Add T8-T13 to golden_social.rs
2. Update reports/golden-e2e-coverage-analysis.md

---

## Critical Files

| File | Change |
|------|--------|
| `crates/worldwake-core/src/goal.rs` | Add GoalKind::ShareBelief |
| `crates/worldwake-core/src/utility_profile.rs` | Add social_weight field |
| `crates/worldwake-ai/src/goal_model.rs` | Add GoalKindTag::ShareBelief |
| `crates/worldwake-ai/src/planner_ops.rs` | Add PlannerOpKind::Tell + semantics |
| `crates/worldwake-ai/src/candidate_generation.rs` | Add emit_social_candidates() |
| `crates/worldwake-ai/src/ranking.rs` | Add ShareBelief suppression, priority, motive |
| `crates/worldwake-ai/tests/golden_harness/mod.rs` | Add belief seeding + Tell helpers |
| `crates/worldwake-ai/tests/golden_social.rs` | New file: 13 test scenarios |
| `reports/golden-e2e-coverage-analysis.md` | Update with social coverage |

## Verification

1. `cargo test -p worldwake-ai --test golden_social` — all 13 tests pass
2. `cargo test --workspace` — no regressions
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean
4. Determinism: each scenario with replay produces identical hashes
5. Conservation: every test checks `verify_live_lot_conservation()` per tick

## Outcome

**Completion date**: 2026-03-16

**What was delivered**:
- **Deliverable 1 (Social AI Goal Integration)**: GoalKind::ShareBelief, GoalKindTag::ShareBelief, PlannerOpKind::Tell with GoalModelFallback semantics, social_weight in UtilityProfile, emit_social_candidates() with chain-length and co-location filtering, ShareBelief ranking with suppression under danger/self-care, social_pressure-based motive scoring. All implemented across tickets E15BSOCAIGOA-001 through E15BSOCAIGOA-006.
- **Deliverable 2 (Golden E2E Test Suite)**: 10 golden social tests in golden_social.rs covering autonomous Tell, rumor chain degradation, stale-belief correction, skeptical-listener rejection, bystander locality, entity-missing discovery, survival-needs suppression, chain-length filtering, agent diversity (Principle 20), and full rumor→wasted-trip→discovery lifecycle. Coverage report updated.
- **Architectural improvements discovered during T11-T13**: Zero-motive filter in rank_candidates() (system-wide invariant preventing execution of unmotivated goals) and treatment_pain() helper fixing healthy-healer treatment scoring.

**Deviations from spec**:
- Spec planned 13 tests; implemented 10 (T1-T7 from Phase A were replaced by T8-T10 which cover the same scenarios autonomously without manual InputQueue injection, plus T11-T13).
- InvestigateMismatch deferred as planned.
- Harness helpers (seed_belief, agent_belief_about, etc.) were added incrementally across tickets rather than in a single Phase A batch.
