**Status**: PENDING

# S19: Institutional Record Consultation Golden E2E Suites

## Summary

Add 3 golden E2E tests to `golden_offices.rs` that exercise the ConsultRecord action as a real-world knowledge acquisition mechanism. Currently all political golden tests seed institutional beliefs directly — no test exercises ConsultRecord end-to-end through the AI planner. These scenarios prove that records are real world artifacts with duration costs, that institutional knowledge requires physical consultation (Principle 7), and that knowledge asymmetry creates emergent competitive outcomes (Principle 20).

## Phase

Phase 3: Information & Politics (post-E16c)

## Crate

`worldwake-ai` (golden tests only — no new system code)

## Dependencies

- E16c (institutional beliefs, ConsultRecord action, Record entities) — COMPLETED
- E16d (political planning, golden harness office helpers) — COMPLETED
- S13 (political emergence goldens — establishes emergent golden patterns) — COMPLETED
- E14 (perception/belief system) — COMPLETED

## Gap Analysis

E16c delivered three novel mechanisms with zero golden E2E coverage:

| Mechanism | Unit Test Coverage | Golden E2E Coverage |
|-----------|-------------------|-------------------|
| ConsultRecord as mid-plan prerequisite | `search.rs:5330` (`search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown`) | **None** |
| ConsultRecord skipped when belief is Certain | `search.rs:5448` (`search_political_goal_skips_consult_record_when_vacancy_belief_is_already_certain`) | **None** |
| ConsultRecord step overrides Unknown belief | `goal_model.rs:2717` (`consult_record_step_overrides_unknown_vacancy_belief_and_unblocks_declare_support`) | **None** |
| Record entity creation and consultation handler | `world.rs:1158`, `consult_record_actions.rs` handler tests | **None** |

All existing political goldens (Scenarios 11-19, 22-25, 28) use `seed_office_holder_belief()` to provide `Certain` institutional beliefs, bypassing the ConsultRecord prerequisite path entirely. S13 scenarios (21-23) also seed beliefs directly.

## Scenarios

### Scenario 32: Local ConsultRecord Prerequisite → Political Action

**File**: `golden_offices.rs`
**Systems exercised**: AI planning (ConsultRecord as mid-plan prerequisite), ConsultRecord action handler (belief projection), Political actions (DeclareSupport), Succession system
**Principles proven**: P12 (world state ≠ belief — agent must consult to learn vacancy), P13 (knowledge acquired through explicit carrier — record entity), P16 (records are world state with durable entries), P21 (institutions exist as offices + records + agents)

**Setup**:
- Single sated agent at VillageSquare with high `enterprise_weight` (pm(800)). PerceptionProfile with `institutional_memory_capacity: 20`, `consultation_speed_factor: pm(500)`.
- Vacant office ("Village Elder") at VillageSquare with `SuccessionLaw::Support`, `succession_period_ticks: 5`, no eligibility rules.
- Office register (Record entity, `RecordKind::OfficeRegister`) at VillageSquare with `consultation_ticks: 3`, one entry recording vacancy (`InstitutionalClaim::OfficeHolder { office, holder: None, effective_tick: Tick(0) }`).
- Agent has entity beliefs about office and record (via `seed_actor_beliefs`), but **no institutional belief about office holder** — `InstitutionalBeliefRead::Unknown`.

**Emergent behavior proven**:
- Agent generates ClaimOffice candidate (the goal is generatable even with Unknown belief, per `goal_model.rs:893`).
- Planner inserts ConsultRecord as mid-plan prerequisite because belief is Unknown (per `search.rs:5330` unit test contract).
- Agent executes ConsultRecord (3 ticks scaled by `consultation_speed_factor`).
- ConsultRecord handler projects `Certain(None)` into agent's institutional belief store.
- Agent then executes DeclareSupport(self).
- After succession period, succession system installs agent as office holder.
- No belief was seeded for office holder — the agent discovered the vacancy through record consultation.

**Assertion surface**:
1. Action traces: `consult_record` committed before `declare_support`
2. Authoritative world state: agent becomes office holder after succession period
3. Decision trace (tick 0 or early): ClaimOffice candidate present, plan includes ConsultRecord step
4. Determinism: replay companion

**Why distinct from Scenario 11**: Scenario 11 seeds office-holder belief directly via `seed_office_holder_belief()`. Scenario 32 starts from `Unknown` and proves the full ConsultRecord → belief acquisition → political action chain end-to-end.

**Scenario isolation**: The agent has no competing needs (sated) and no competing affordances (single agent, single office). Unrelated lawful branches (ShareBelief, if any co-located entity) are removed by having only one agent. The contract under test is purely the ConsultRecord prerequisite path.

---

### Scenario 33: Remote Record → Travel + ConsultRecord + Political Action

**File**: `golden_offices.rs`
**Systems exercised**: Travel (multi-hop pathfinding), ConsultRecord action handler, Political actions (DeclareSupport), Succession system, AI planning (4-step plan: Travel→ConsultRecord→Travel→DeclareSupport)
**Principles proven**: P7 (locality — must physically travel to record location to gain institutional knowledge), P8 (travel + consultation have real duration and cost), P1 (maximal emergence — travel, records, and politics interact through state with no cross-system coupling)

**Setup**:
- Single sated agent at OrchardFarm with high `enterprise_weight` (pm(800)). PerceptionProfile with `institutional_memory_capacity: 20`, `consultation_speed_factor: pm(500)`.
- Vacant office ("Village Elder") at VillageSquare with `SuccessionLaw::Support`, `succession_period_ticks: 5`, no eligibility rules.
- Office register (Record entity, `RecordKind::OfficeRegister`) at RulersHall with `consultation_ticks: 3`, one entry recording vacancy.
- Agent has entity beliefs about office (at VillageSquare), record (at RulersHall), and relevant places. **No institutional belief about office holder** — `InstitutionalBeliefRead::Unknown`.

**Topology for this scenario**:
- OrchardFarm → EastFieldTrail (2 ticks) → SouthGate (1 tick) → VillageSquare (1 tick) → RulersHall (1 tick) = 5 ticks to record
- RulersHall → VillageSquare = 1 tick to office jurisdiction
- Total travel: 6 ticks + consultation (3 ticks scaled) + DeclareSupport + succession period

**Emergent behavior proven**:
- Agent generates ClaimOffice candidate despite being far from both record and office.
- Planner produces 4-step plan: Travel(OrchardFarm→...→RulersHall) → ConsultRecord(record) → Travel(RulersHall→VillageSquare) → DeclareSupport.
- Agent executes multi-hop travel to RulersHall, consults record, travels to VillageSquare, declares support.
- Succession installs agent after succession period.
- The plan routes through the record location (RulersHall) **before** the office jurisdiction (VillageSquare) — proving information locality shapes the agent's physical path through the world.

**Assertion surface**:
1. Action traces: travel commits (agent reaches RulersHall), `consult_record` commits, travel commits (agent reaches VillageSquare), `declare_support` commits — in this sequence
2. Authoritative world state: agent ends at VillageSquare and becomes office holder
3. Decision trace (early tick): plan includes Travel→ConsultRecord→Travel→DeclareSupport shape
4. Determinism: replay companion

**Why distinct from Scenario 15** (travel to distant jurisdiction): Scenario 15 tests travel to a remote office with **already-known** vacancy. Scenario 33 tests travel to a remote **record** to learn about vacancy, then travel again to the office to act. The information-gathering detour through the record location is the novel contract.

**Coverage bonus**: Uses RulersHall (currently unused in golden tests — brings place coverage from 9/12 to 10/12).

**Scenario isolation**: Single agent, no competing needs, no competing agents. The contract is the multi-hop information-gathering path shape. The only lawful competing branch would be if the planner found a shorter path that avoids record consultation, but the planner cannot skip ConsultRecord when belief is Unknown (proven by `search.rs:5330`).

---

### Scenario 34: Knowledge Asymmetry Race — Informed Agent Outpaces Consulting Agent

**File**: `golden_offices.rs`
**Systems exercised**: AI planning (ConsultRecord prerequisite vs direct action), ConsultRecord action handler (duration cost), Political actions (DeclareSupport), Succession system, multi-agent interaction
**Principles proven**: P14 (Unknown vs Certain creates real behavioral divergence), P20 (knowledge diversity → different competitive outcomes — same role, different knowledge, different results), P8 (consultation has real duration that costs competitive time), P1 (emergent outcome — who holds office — arises from knowledge state + action duration interaction, no system orchestrates this)

**Setup**:
- Two sated agents at VillageSquare, both with high `enterprise_weight` (pm(800)). Both have PerceptionProfiles.
- Vacant office ("Village Elder") at VillageSquare with `SuccessionLaw::Support`, `succession_period_ticks: 5`, no eligibility rules.
- Office register (Record entity, `RecordKind::OfficeRegister`) at VillageSquare with `consultation_ticks: 4` (long enough to create meaningful delay), one entry recording vacancy.
- Agent A ("Informed"): has entity beliefs about office AND `Certain(None)` institutional belief about office holder (seeded via `seed_office_holder_belief`). The planner will skip ConsultRecord for A.
- Agent B ("Uninformed"): has entity beliefs about office and record, but **no institutional belief about office holder** — `Unknown`. The planner will insert ConsultRecord for B.

**Emergent behavior proven**:
- Both agents generate ClaimOffice candidates on the same tick.
- Agent A's plan: DeclareSupport(self) — immediate, no prerequisites.
- Agent B's plan: ConsultRecord(record) → DeclareSupport(self) — consultation takes 4 ticks (scaled by `consultation_speed_factor`).
- Agent A declares support immediately. Succession period begins counting A's support.
- Agent B spends ticks consulting the record, then declares support — but A already has a head start in the succession window.
- Succession installs Agent A (more support ticks accumulated, or sole supporter at resolution time).
- Agent B either declares support after A is already installed (resulting in StartFailed or no-op), or B's support arrives too late to change the outcome.
- **The competitive outcome (who holds office) emerges from knowledge state + consultation duration, not from any explicit priority or ordering system.**

**Assertion surface**:
1. Action traces: A's `declare_support` commits before B's `consult_record` commits (A acts immediately while B is still consulting)
2. Authoritative world state: A is office holder (the informed agent wins)
3. Decision trace: A's plan has no ConsultRecord step; B's plan includes ConsultRecord step
4. Determinism: replay companion

**Why distinct from Scenario 28** (remote office claim race): Scenario 28 races two agents who both travel from different locations to the same office — the asymmetry is geographic distance. Scenario 34 races two co-located agents where the asymmetry is knowledge state — one must consult a record (costing time) while the other already knows. This proves that institutional knowledge is a real competitive resource, not just a planning convenience.

**Scenario isolation**: Both agents are sated with no competing needs. No travel needed (all co-located). The only variable between agents is institutional knowledge state. This isolates the consultation duration as the sole cause of competitive divergence.

**Setup math**: With `consultation_ticks: 4` and `consultation_speed_factor: pm(500)`, actual consultation duration = ceil(4 * 1000 / 500) = 8 ticks. DeclareSupport is a 1-tick action. So A declares support on tick ~1, while B finishes consultation around tick ~8 and declares support around tick ~9. With `succession_period_ticks: 5`, A has accumulated 5+ ticks of support before B even starts, ensuring A wins.

**Note**: The exact consultation duration scaling formula should be verified during ticket reassessment against the live `consult_record_actions.rs` handler. If `consultation_speed_factor: pm(500)` means "half speed" (takes longer), the 8-tick estimate holds. If it means "50% of normal duration" (takes shorter), adjust `consultation_ticks` accordingly.

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path Validated | Scenario |
|-------------|--------|----------------|----------|
| Office vacancy (via record consultation) | Record entity entries at record's `home_place` | Agent travels to record location → ConsultRecord action reads entries → handler projects `InstitutionalClaim::OfficeHolder { holder: None }` into agent's `AgentBeliefStore.institutional_beliefs` with `RecordConsultation` source | 32, 33 |
| Office vacancy (pre-seeded belief) | `seed_office_holder_belief()` test setup | Direct institutional belief injection with `WitnessedEvent` source — established pre-E16c golden pattern | 34 (Agent A only) |
| Record location | Entity belief about Record entity | Agent must have entity belief about the record to include it in planner evidence set | 32, 33, 34 |

**Key validation**: Scenarios 32 and 33 prove that institutional knowledge can be acquired through physical record consultation, not just through belief seeding or Tell. Scenario 34 proves that the presence vs absence of institutional knowledge creates real duration-based competitive consequences.

### H.2 Positive-Feedback Analysis

**Loop 1: Consultation → knowledge → political action → new record entries → more consultation**. An agent consults a record, gains knowledge, acts politically, and the outcome creates new record entries others can consult. Bounded by consultation duration, succession period, travel time to records, and the finite number of offices.

No new positive-feedback loops are introduced by these test scenarios — they exercise existing E16c mechanisms.

### H.3 Concrete Dampeners

- Consultation duration (physical time cost — `consultation_ticks` scaled by `consultation_speed_factor`)
- Travel time to record location (physical distance cost)
- Succession period delay (physical time gate on office installation)
- Enterprise weight threshold (agent-specific motivation gate)
- Record capacity (`max_entries_per_consult` limits per-consultation knowledge gain)

### H.4 Stored State vs Derived

**Stored (authoritative)**: `RecordData` component (entries, consultation_ticks, home_place), `AgentBeliefStore.institutional_beliefs` (post-consultation), office-holder relation, `OfficeData` component, `PerceptionProfile` (consultation_speed_factor)

**Derived (transient)**: `InstitutionalBeliefRead::Unknown` / `Certain` / `Conflicted` (derived from belief store queries), plan shape including/excluding ConsultRecord (derived by planner from belief state), consultation duration (derived from `consultation_ticks * 1000 / consultation_speed_factor`), office-holder outcome (consequence of succession resolution)

## Cross-System Interactions (Principle 24)

### Scenario 32 chain
1. AI planning reads `InstitutionalBeliefRead::Unknown` from belief store → inserts ConsultRecord into plan
2. ConsultRecord action handler reads `RecordData.entries` → projects `InstitutionalClaim` into `AgentBeliefStore`
3. AI planning reads updated `InstitutionalBeliefRead::Certain(None)` → proceeds to DeclareSupport
4. DeclareSupport action handler creates support relation
5. Succession system reads support relations + succession period → installs office holder

### Scenario 33 chain
1. AI planning reads `InstitutionalBeliefRead::Unknown` + record location at RulersHall → plans Travel→ConsultRecord→Travel→DeclareSupport
2. Travel action handler moves agent through topology: OrchardFarm→EastFieldTrail→SouthGate→VillageSquare→RulersHall
3. ConsultRecord handler at RulersHall reads record entries → projects belief
4. Travel action handler moves agent: RulersHall→VillageSquare
5. DeclareSupport at VillageSquare → succession installs

### Scenario 34 chain
1. Agent A: AI reads `Certain(None)` → plans DeclareSupport directly (no ConsultRecord)
2. Agent B: AI reads `Unknown` → plans ConsultRecord→DeclareSupport
3. A commits DeclareSupport while B is still consulting (action duration asymmetry)
4. Succession resolves in A's favor (A has more accumulated support ticks)
5. B's eventual DeclareSupport either arrives after A's installation or competes too late

## Tickets

### S19-001: Scenario 32 — Local ConsultRecord → Political Action

**Deliverable**: `golden_consult_record_prerequisite_political_action` + `golden_consult_record_prerequisite_political_action_replays_deterministically` in `golden_offices.rs`

**Golden harness additions**: `seed_office_register()` helper in `golden_harness/mod.rs` — creates a Record entity with `RecordKind::OfficeRegister`, places it at a specified location, and populates it with a vacancy entry. Also add `RULERS_HALL` constant: `pub const RULERS_HALL: EntityId = prototype_place_entity(PrototypePlace::RulersHall);`

**Assertion surface**:
- Action trace: `consult_record` committed before `declare_support`
- Authoritative state: agent is office holder
- Decision trace: plan shape includes ConsultRecord step
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai --test golden_offices golden_consult_record_prerequisite_political_action`, then `cargo test -p worldwake-ai`, then `cargo test --workspace`, then `cargo clippy --workspace --all-targets -- -D warnings`

---

### S19-002: Scenario 33 — Remote Record Travel + Consultation + Political Action

**Deliverable**: `golden_remote_record_consultation_political_action` + `golden_remote_record_consultation_political_action_replays_deterministically` in `golden_offices.rs`

**Assertion surface**:
- Action traces: travel commits (reaching RulersHall), `consult_record` commits, travel commits (reaching VillageSquare), `declare_support` commits — in sequence
- Authoritative state: agent ends at VillageSquare and is office holder
- Decision trace: initial plan shape is Travel→ConsultRecord→Travel→DeclareSupport
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`, then `cargo test -p worldwake-ai`, then `cargo test --workspace`, then `cargo clippy --workspace --all-targets -- -D warnings`

---

### S19-003: Scenario 34 — Knowledge Asymmetry Race

**Deliverable**: `golden_knowledge_asymmetry_race_informed_wins_office` + `golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically` in `golden_offices.rs`

**Assertion surface**:
- Action traces: A's `declare_support` commits before B finishes `consult_record`
- Authoritative state: A is office holder (not B)
- Decision trace: A's plan lacks ConsultRecord; B's plan includes ConsultRecord
- Determinism: replay companion

**Verification**: `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race`, then `cargo test -p worldwake-ai`, then `cargo test --workspace`, then `cargo clippy --workspace --all-targets -- -D warnings`

---

### S19-004: Update golden-e2e-coverage.md

**Deliverable**: Add S19 scenarios to coverage matrix and cross-system chains in `docs/golden-e2e-coverage.md`. New cross-system chains:
- ConsultRecord prerequisite → institutional belief acquisition → political action (32)
- Remote record → travel to record → consultation → travel to office → political action (33)
- Knowledge asymmetry → consultation duration cost → competitive political outcome (34)

---

### S19-005: Update golden-e2e-scenarios.md

**Deliverable**: Add detailed scenario descriptions for Scenarios 32-34 in `docs/golden-e2e-scenarios.md`.

## Critical Files

| File | Role |
|------|------|
| `specs/S19-institutional-record-consultation-golden-suites.md` | This spec |
| `crates/worldwake-ai/tests/golden_offices.rs` | Add 3 suites (~6 tests + replay companions) |
| `crates/worldwake-ai/tests/golden_harness/mod.rs` | Add `RULERS_HALL` constant, `seed_office_register()` helper |
| `docs/golden-e2e-coverage.md` | Update coverage matrix |
| `docs/golden-e2e-scenarios.md` | Add scenario descriptions |

## Existing Utilities to Reuse

| Utility | Location | Usage |
|---------|----------|-------|
| `seed_office()` | `golden_harness/mod.rs` | Creates office entity with SuccessionLaw and eligibility rules |
| `seed_agent()` | `golden_harness/mod.rs` | Creates agent with needs, metabolism, utility profile |
| `set_agent_perception_profile()` | `golden_harness/mod.rs` | Sets PerceptionProfile including institutional fields |
| `seed_actor_beliefs()` | `golden_harness/mod.rs` | Seeds entity beliefs for offices, records, places |
| `seed_office_holder_belief()` | `golden_harness/mod.rs` | Seeds Certain institutional belief (used for Agent A in Scenario 34; omitted for Unknown) |
| `enterprise_weighted_utility()` | `golden_harness/mod.rs` | Creates enterprise-heavy UtilityProfile |
| `World::create_record()` | `worldwake-core/src/world.rs:172` | Creates Record entity with RecordData |
| `RecordData` | `worldwake-core/src/institutional.rs:46` | Record struct with entries, consultation_ticks, home_place |
| `InstitutionalClaim::OfficeHolder` | `worldwake-core/src/institutional.rs` | Claim type for vacancy entries |
| `prototype_place_entity()` | `worldwake-core/src/topology.rs` | Converts PrototypePlace to EntityId |

## Verification

Per ticket:
1. `cargo test -p worldwake-ai --test golden_offices <test_name>` — targeted test
2. `cargo test -p worldwake-ai` — full AI crate suite
3. `cargo test --workspace` — workspace suite
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint
5. `python3 scripts/golden_inventory.py --write --check-docs` — inventory refresh

After all tickets:
6. Verify `docs/golden-e2e-coverage.md` reflects new scenarios in all matrices
7. Verify `docs/golden-e2e-scenarios.md` has detailed descriptions for Scenarios 32-34

## Implementation Order

S19-001 (Scenario 32, includes harness helpers) → S19-002 (Scenario 33) → S19-003 (Scenario 34) → S19-004 + S19-005 (doc updates, parallel)
