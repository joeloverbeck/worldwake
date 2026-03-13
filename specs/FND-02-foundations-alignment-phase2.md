# FND-02: Phase 2 Foundations Alignment

**Status**: ACTIVE
**Priority**: BLOCKER — must complete before Phase 3 implementation begins
**Scope**: Strengthen Phase 2 code and specs to align with `docs/FOUNDATIONS.md`
**Source**: Golden e2e test reports, Phase 2 alignment analysis

## Goal

Eliminate remaining architectural gaps where Phase 2 code and specs violate foundational principles — specifically: abstract float confidence scores in E14 spec, non-deterministic HashMap in E14 spec, missing GoalKind candidate emissions (SellCommodity, AcquireCommodity for Treatment), undocumented feedback dampeners across Phase 2 systems, and missing debuggability APIs.

## Non-Goals

- Do not implement E14 (Perception & Beliefs) — this spec only fixes the E14 *spec document* to be determinism-safe.
- Do not add backward-compatibility shims (Principle 26).
- Do not restructure Phase 2 systems — only wire missing goal emissions and add inspection APIs.
- Do not implement full causal tracing UI — only the foundational query APIs.

---

## Section A — Strengthen E14 Spec (FND02-001)

**Ticket**: FND02-001
**Principles violated**: 3 (Concrete State Over Abstract Scores), determinism invariants (no floats, no HashMap)

### Problem

`specs/E14-perception-beliefs.md` line 35 specifies `known_facts: HashMap<FactId, PerceivedFact>` and line 41 specifies `confidence: f32`. Both violate project-wide determinism invariants:

- `HashMap` has non-deterministic iteration order — all authoritative state must use `BTreeMap`/`BTreeSet` (see CLAUDE.md Critical Invariants).
- `f32` is forbidden in authoritative state — all [0,1] or [0,1000] range values must use `Permille` (see CLAUDE.md Spec Drafting Rules, Principle 3).

Additionally, the E14 spec lacks the FND-01 Section H analysis (information-path, positive-feedback, concrete dampeners, stored vs derived) now required for all specs.

Finally, `LoyalTo.strength: Permille` must be resolved through E14 by replacing the abstract scalar with concrete relational state derived from shared experiences, obligations, and betrayals.

### Required Changes

**File**: `specs/E14-perception-beliefs.md`

1. **Replace `HashMap<FactId, PerceivedFact>` → `BTreeMap<FactId, PerceivedFact>`** in the Memory component specification.

2. **Replace `confidence: f32` → `confidence: Permille`** in the `PerceivedFact` struct specification. Document the scale: `Permille(1000)` = direct observation, degrading for indirect sources.

3. **Add FND-01 Section H analysis** to the spec:
   - Information-path analysis: How does each piece of information reach agents? (Trace: event → witness → belief store, or event → rumor → belief store with time-per-hop delay.)
   - Positive-feedback analysis: Identify amplifying loops in the perception/belief system.
   - Concrete dampeners: For each loop, specify the physical world mechanism that limits it.
   - Stored state vs. derived read-model list: Enumerate what is authoritative (belief stores, witness records) vs. derived (belief queries, staleness checks).

4. **Add LoyalTo resolution requirement**: The E14 implementation must replace `LoyalTo.strength: Permille` with concrete relational state. Loyalty emerges from: shared experiences (witnessed cooperation, saved life), fulfilled obligations, betrayals, and time spent together. The abstract `strength` field becomes a derived read-model computed from these concrete records.

5. **Require full OmniscientBeliefView replacement**: E14 must fully replace `OmniscientBeliefView` (not wrap it). After E14, no code path may use `OmniscientBeliefView`.

### Acceptance Criteria

- [ ] E14 spec contains no `HashMap` in authoritative state definitions.
- [ ] E14 spec contains no `f32` in authoritative state definitions.
- [ ] E14 spec includes complete FND-01 Section H analysis.
- [ ] E14 spec documents LoyalTo resolution through concrete relational state.
- [ ] E14 spec requires full OmniscientBeliefView removal (not wrapping).

---

## Section B — Wire SellCommodity Goal Emission (FND02-002)

**Ticket**: FND02-002
**Principles violated**: 28 (Every New System Spec Must Declare Its Causal Hooks — completeness), 18 (Resource-Bounded Practical Reasoning Over Scripts)

### Problem

`GoalKind::SellCommodity` exists in `crates/worldwake-core/src/goal.rs` but `crates/worldwake-ai/src/candidate_generation.rs` has no emission logic for it. The goal kind is defined but unreachable through the candidate generation pipeline — agents with merchandise can never autonomously decide to sell.

### Required Changes

**File**: `crates/worldwake-ai/src/candidate_generation.rs`

1. **Add sell-candidate emission function**: When an agent has a `MerchandiseProfile` and holds commodity quantity above their restock threshold at a place with other agents (potential buyers), emit `GoalKind::SellCommodity { commodity }` candidates.

2. **Emission conditions**:
   - Agent has `MerchandiseProfile` component.
   - Agent holds at least one commodity where current stock exceeds restock threshold (surplus to sell).
   - Agent is at a place (not in transit).
   - Evidence: the commodity entity and the place.

3. **Priority/motive**: Sell goals should have enterprise-class priority, below survival needs but competitive with restock goals.

### Test

- Golden e2e test proving emergent sell behavior: merchant with surplus commodity at market emits SellCommodity candidate.
- Unit test: agent without MerchandiseProfile does not emit sell candidates.
- Unit test: agent with MerchandiseProfile but no surplus does not emit sell candidates.

### Acceptance Criteria

- [ ] `SellCommodity` candidates emitted when conditions met.
- [ ] No sell candidates when agent lacks MerchandiseProfile.
- [ ] No sell candidates when no surplus above restock threshold.
- [ ] Golden e2e test passes.

---

## Section C — Wire AcquireCommodity(Treatment) Goal Emission (FND02-003)

**Ticket**: FND02-003
**Principles violated**: 28 (Completeness), 18 (Resource-Bounded Practical Reasoning Over Scripts)

### Problem

Treatment acquisition is not wired in candidate generation. When an agent with healing capability (or any agent aware of wounded agents) lacks medicine/treatment commodities, they cannot autonomously decide to acquire them.

### Required Changes

**File**: `crates/worldwake-ai/src/candidate_generation.rs`

1. **Add treatment-acquisition emission**: When an agent knows of a wounded entity (self or other) and lacks the commodity needed for treatment, emit `GoalKind::AcquireCommodity { commodity, purpose: CommodityPurpose::Treatment }`.

2. **Emission conditions**:
   - Agent or a co-located entity has active wounds (from `WoundTracker` or belief view).
   - Agent does not currently hold sufficient treatment commodity.
   - A treatment commodity exists in the recipe/item system.
   - Evidence: the wounded entity.

3. **Self-treatment priority**: When the wounded entity is the agent itself, priority should be elevated (pain/danger pressure from wounds feeds into this naturally through existing pressure derivation).

### Test

- Golden e2e test proving medicine-seeking behavior: wounded agent or healer emits AcquireCommodity(Treatment) candidate.
- Unit test: unwounded agent with no wounded co-located entities does not emit treatment candidates.

### Acceptance Criteria

- [ ] `AcquireCommodity` with `CommodityPurpose::Treatment` emitted when conditions met.
- [ ] No treatment candidates when no wounds present.
- [ ] Golden e2e test passes.

---

## Section D — Feedback Dampening Audit (FND02-004)

**Ticket**: FND02-004
**Principles violated**: 10 (Every Positive Feedback Loop Needs a Physical Dampener)

### Problem

No systematic audit has been performed on Phase 2 systems for amplifying feedback loops. Each system was implemented independently; cross-system amplification patterns may exist undocumented and undamped.

### Required Changes

**Scope**: Analysis and documentation, plus code fixes for any undamped loops found.

1. **Audit each Phase 2 system for amplifying loops**:

   **Needs/Metabolism** (`crates/worldwake-systems/src/needs.rs`, `needs_actions.rs`):
   - Identify: Does need satisfaction create conditions that accelerate need growth? (e.g., eating → energy → more activity → faster hunger)
   - Document dampeners: resource depletion (food consumed is gone), action duration (eating takes time), capacity limits.

   **Production** (`crates/worldwake-systems/src/production.rs`, `production_actions.rs`):
   - Identify: Does production create conditions that accelerate further production? (e.g., crafting tools → faster crafting → more tools)
   - Document dampeners: raw material depletion, workstation occupancy, action duration, storage/load limits.

   **Trade** (`crates/worldwake-systems/src/trade.rs`, `trade_actions.rs`):
   - Identify: Does successful trade create conditions for more trade? (e.g., profit → buy more → sell more → more profit)
   - Document dampeners: inventory limits, travel time between markets, demand saturation (DemandMemory aging), coin/commodity conservation.

   **Combat** (`crates/worldwake-systems/src/combat.rs`):
   - Identify: Does combat create conditions for more combat? (e.g., wounds → vulnerability → more attacks → more wounds)
   - Document dampeners: wound incapacitation, death, flight responses, weapon/energy depletion.

   **AI Enterprise** (`crates/worldwake-ai/src/enterprise.rs`):
   - Identify: Does enterprise goal generation create runaway goal spirals?
   - Document dampeners: planning budget limits, blocked intent memory, goal switching margins.

2. **Document findings** in a new file: `docs/dampening-audit-phase2.md`

3. **Fix any undamped loops**: If the audit reveals loops with no physical dampener (only numerical clamps), add concrete dampening mechanisms.

4. **Add gate note**: All Phase 3+ specs must include their own Section H dampening analysis before implementation.

### Acceptance Criteria

- [ ] All five Phase 2 system domains audited for amplifying loops.
- [ ] Each identified loop has a documented physical dampener.
- [ ] No loops rely solely on numerical clamps (`min`, `max`, `clamp`) as dampeners.
- [ ] `docs/dampening-audit-phase2.md` created with audit results.
- [ ] Any undamped loops fixed with concrete world mechanisms.

---

## Section E — Debuggability Foundation (FND02-005)

**Ticket**: FND02-005
**Principles violated**: 27 (Debuggability Is a Product Feature)

### Problem

No structured causal inspection APIs exist. The simulation produces emergent behavior but provides no programmatic way to answer "why did this agent do that?" or "what caused this event?" These are core debuggability requirements per Principle 27.

### Required Changes

1. **Add `explain_goal()` to AI crate**:

   **File**: New file `crates/worldwake-ai/src/goal_explanation.rs`

   ```rust
   pub struct GoalExplanation {
       pub goal: GoalKind,
       pub priority_class: PriorityClass,
       pub motive_value: Permille,
       pub evidence_entities: Vec<EntityId>,
       pub evidence_places: Vec<EntityId>,
       pub competing_goals: Vec<(GoalKind, PriorityClass, Permille)>,
   }

   pub fn explain_goal(
       view: &dyn BeliefView,
       agent: EntityId,
       goal: &GoalKind,
       blocked: &BlockedIntentMemory,
       recipes: &RecipeRegistry,
       current_tick: Tick,
   ) -> Option<GoalExplanation>;
   ```

   This is a **derived read-model** (Principle 25) — it recomputes the ranking for a specific goal and returns the explanation. It does not store any state.

2. **Add `trace_event_cause()` to sim crate**:

   **File**: New file `crates/worldwake-sim/src/event_trace.rs`

   ```rust
   pub fn trace_event_cause(
       event_log: &EventLog,
       event_id: EventId,
   ) -> Vec<EventId>;
   ```

   Walks the `CauseRef` chain from the given event backwards through the event log, returning the ordered causal ancestry. This is a **derived read-model** — it traverses existing event log data without storing anything new.

3. **Wire into `lib.rs`** for both crates.

### Test

- Unit test for `explain_goal()`: Create an agent with hunger pressure, verify explanation includes the correct goal kind, priority class, and evidence.
- Unit test for `trace_event_cause()`: Create a chain of 3+ causally linked events, verify the trace returns the correct ancestry in order.
- Edge case: `trace_event_cause()` on an event with no cause returns empty vec.

### Acceptance Criteria

- [ ] `explain_goal()` function exists in `worldwake-ai` and returns structured explanation.
- [ ] `trace_event_cause()` function exists in `worldwake-sim` and returns causal ancestry.
- [ ] Both are derived read-models (no stored state).
- [ ] Unit tests for both APIs pass.
- [ ] `cargo test --workspace` passes.

---

## Section F — DRAFT Promotion to Formal Specs

**Ticket**: FND02-006

### Problem

Six DRAFT specs in `specs/` represent valuable architectural work from Phase 2 analysis that has not been formally integrated into the implementation order. They need formal spec numbers, dependency declarations, and phase placement.

### Required Changes

1. **Rename files** (preserving git history via `git mv`):

   | Current | New |
   |---------|-----|
   | `specs/DRAFT-production-output-ownership-claims.md` | `specs/S01-production-output-ownership-claims.md` |
   | `specs/DRAFT-goal-decision-policy-unification.md` | `specs/S02-goal-decision-policy-unification.md` |
   | `specs/DRAFT-planner-target-identity-and-affordance-binding.md` | `specs/S03-planner-target-identity-and-affordance-binding.md` |
   | `specs/DRAFT-merchant-selling-market-presence.md` | `specs/S04-merchant-selling-market-presence.md` |
   | `specs/DRAFT-merchant-stock-storage-and-stalls.md` | `specs/S05-merchant-stock-storage-and-stalls.md` |
   | `specs/DRAFT-commodity-opportunity-valuation.md` | `specs/S06-commodity-opportunity-valuation.md` |

2. **Each spec must include** (add if missing):
   - FND-01 Section H analysis (information-path, feedback loops, dampeners, stored vs derived).
   - Explicit dependency declarations.
   - Phase placement in the implementation order.

3. **Integration into `specs/IMPLEMENTATION-ORDER.md`**:
   - S01, S02, S03 → Phase 3, Step 10 (parallel after E14)
   - S04, S05, S06 → Phase 4+, Step 14 (economy deepening after E22)

### Acceptance Criteria

- [ ] No files matching `specs/DRAFT-*.md` exist.
- [ ] Six files `specs/S01-*.md` through `specs/S06-*.md` exist.
- [ ] Each S-spec has dependency and phase placement declarations.
- [ ] All S-specs appear in `specs/IMPLEMENTATION-ORDER.md`.

---

## Implementation Order

1. **FND02-001** (Section A): Fix E14 spec — spec-only change, no code
2. **FND02-004** (Section D): Dampening audit — analysis + documentation + possible code fixes
3. **FND02-002** (Section B): Wire SellCommodity — code change in candidate_generation.rs
4. **FND02-003** (Section C): Wire AcquireCommodity(Treatment) — code change in candidate_generation.rs
5. **FND02-005** (Section E): Debuggability APIs — new files in ai and sim crates
6. **FND02-006** (Section F): DRAFT promotion — file renames + spec amendments

Tickets 1 and 4 are spec/analysis work and can be parallelized.
Tickets 2, 3, and 5 are code work and can be parallelized.
Ticket 6 (DRAFT promotion file renames) is done as part of this FND-02 creation.

## FND-02 Gate Criteria

Before proceeding to Phase 3 (E14 implementation):

- [ ] E14 spec is determinism-safe (no `f32`, no `HashMap` in authoritative state)
- [ ] GoalKind candidate coverage: all defined GoalKind variants have emission paths
- [ ] Dampening audit documented in `docs/dampening-audit-phase2.md`
- [ ] `explain_goal()` and `trace_event_cause()` APIs exist and tested
- [ ] 6 DRAFTs promoted to S01-S06 and integrated into implementation order
- [ ] `cargo test --workspace` passes

## Cross-References

- `docs/FOUNDATIONS.md` — Principles 3, 10, 18, 25, 26, 27, 28
- `archive/specs/FND-01-phase1-foundations-alignment.md` — predecessor spec, pattern reference
- `specs/E14-perception-beliefs.md` — Section A target
- `crates/worldwake-ai/src/candidate_generation.rs` — Sections B, C target
- `crates/worldwake-systems/src/` — Section D audit targets
- `crates/worldwake-ai/src/enterprise.rs` — Section D audit target

## Tests Summary

| Ticket | Test | Description |
|--------|------|-------------|
| FND02-001 | spec review | E14 spec contains no f32 or HashMap in authoritative state |
| FND02-002 | golden e2e + unit | Merchant with surplus emits SellCommodity; no emission without MerchandiseProfile or surplus |
| FND02-003 | golden e2e + unit | Wounded agent/healer emits AcquireCommodity(Treatment); no emission without wounds |
| FND02-004 | audit document | All Phase 2 loops documented with physical dampeners |
| FND02-005 | unit tests | explain_goal returns structured explanation; trace_event_cause returns causal ancestry |
| FND02-006 | file listing | No DRAFT-*.md files; S01-S06 exist and appear in IMPLEMENTATION-ORDER.md |
