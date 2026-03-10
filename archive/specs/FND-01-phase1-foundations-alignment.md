# FND-01: Phase 1 Foundations Alignment Corrections

**Status**: ARCHIVED
**Priority**: BLOCKER — must complete before Phase 2 implementation begins
**Scope**: Amend Phase 1 code (E02, E04, E05, E07) to align with `docs/FOUNDATIONS.md`
**Source**: `brainstorming/phase-1-foundations-alignment-correction.md`

## Goal

Eliminate remaining architectural loopholes where Phase 1 code violates foundational principles — specifically: abstract route scores masquerading as world state, omniscient knowledge views used on behalf of agents, zero-cost actions, hidden magic-number load tables, and unconstrained loyalty mutations.

## Non-Goals

- Do not rework deterministic IDs, world transaction journaling, event log structure, replay/save-load, or agent symmetry — these are already aligned.
- Do not add backward-compatibility shims or alias paths (Principle 13).
- Do not implement the full perception/belief pipeline — that belongs to E14. This spec only renames interfaces and enforces the separation boundary.
- Do not implement concrete route presence — that is deferred to E10/E11/E12 with a gate note (Section D).

---

## Section A — Split KnowledgeView into BeliefView

**Ticket**: FND01-004
**Principles violated**: 7 (Locality of Interaction and Information), 10 (Intelligent Agency Over Behavioral Scripts)

### Problem

`KnowledgeView` is used both for agent-facing affordance queries and for authoritative legality checks. The name does not signal that agent-facing code must operate on beliefs, not world truth. `WorldKnowledgeView` wraps `&World` directly, making it trivially easy for future agent-facing code to read authoritative state.

### Required Changes

**Files**: `crates/worldwake-sim/src/knowledge_view.rs`, `world_knowledge_view.rs`, `affordance_query.rs`, `tick_step.rs`, `start_gate.rs`, `tick_action.rs`, `interrupt_abort.rs`, `lib.rs`

1. **Rename `KnowledgeView` trait → `BeliefView`**
   - All methods remain identical for now (E14 will add real belief filtering).
   - The rename signals intent: anything consuming `&dyn BeliefView` must be safe to feed stale/partial data.

2. **Rename `WorldKnowledgeView` → `OmniscientBeliefView`**
   - The "Omniscient" prefix makes the temporary shortcut explicit and grep-able.
   - Doc-comment must state: "Temporary stand-in until E14 provides per-agent belief stores. MUST NOT be used in agent-facing code after E14 lands."

3. **Enforce affordance pipeline separation**:
   - `get_affordances()` signature takes `&dyn BeliefView` (already does via trait, just rename).
   - `start_action()` and legality checks in `start_gate.rs` continue to use `&World` directly for authoritative precondition validation.
   - `tick_action.rs` internal legality (co-location checks) uses `&World` directly — this is correct and stays.

4. **Add divergent-belief test**:
   - Create two `StubBeliefView` instances with different `effective_place` returns for the same actor.
   - Call `get_affordances()` with each — assert they produce different affordance sets.
   - This test proves the pipeline respects the belief boundary.

### Acceptance Criteria

- [ ] No symbol named `KnowledgeView` or `WorldKnowledgeView` exists in the codebase.
- [ ] `get_affordances()` accepts `&dyn BeliefView`.
- [ ] `start_gate` and `tick_action` use `&World` for authoritative checks.
- [ ] Divergent-belief test passes.
- [ ] `cargo test --workspace` passes.

---

## Section B — Information Pipeline Requirements (DEFERRED → E14)

**Principles violated**: 7 (Locality of Interaction and Information)

### Problem

Phase 1 has no perception system. Events do not generate witnesses, and agents have no belief stores. This is expected — E14 is the perception epic. However, E14's current spec does not explicitly require several locality constraints identified in the correction analysis.

### Action

**No code changes in this spec.** Instead, deferral notes are added to `specs/E14-perception-beliefs.md` (see below) requiring:

1. **Time-per-hop propagation**: Any information propagation beyond direct perception MUST consume time per hop through the place graph.
2. **PublicRecord = consultable-at-location**: A public record means "a record exists at a place/entity and can be consulted," not "this becomes globally known."
3. **AdjacentPlaces = immediate spillover only**: Adjacent-place perception MUST be limited to immediate physical spillover and MUST NOT serve as free multi-hop information spread.
4. **Belief traceability**: Agent beliefs MUST be traceable to witness, report, record consultation, or prior belief state.

---

## Section C — Remove Route Scores from TravelEdge

**Ticket**: FND01-001
**Principles violated**: 3 (Concrete State Over Abstract Scores)

### Problem

`TravelEdge` stores `danger: Permille` and `visibility: Permille` — abstract scores with no causal grounding. These violate Principle 3: "Instead of assigning `danger_score: 0.7` to a road, model the actual bandits present on that route."

### Required Changes

**Files**: `crates/worldwake-core/src/topology.rs`

1. **Remove fields from `TravelEdge` struct**:
   - Remove `danger: Permille`
   - Remove `visibility: Permille`

2. **Remove from constructor** `TravelEdge::new()`:
   - Remove `danger` and `visibility` parameters.

3. **Remove accessor methods**:
   - Remove `pub fn danger(&self) -> Permille`
   - Remove `pub fn visibility(&self) -> Permille`

4. **Update `PrototypeEdgeSpec`**:
   - Remove `danger: u16` and `visibility: u16` fields.
   - Update all entries in the prototype edge table.

5. **Update `build_prototype_world()`**:
   - Remove `prototype_permille(spec.danger)` and `prototype_permille(spec.visibility)` from edge construction.

6. **Update all call sites**:
   - Remove `danger` and `visibility` arguments from every `TravelEdge::new()` call (including tests).

7. **Remove or update tests**:
   - Remove assertions on `edge.danger()` and `edge.visibility()`.
   - Remove `forest_dangers` / `village_dangers` comparison test (this test validated abstract scores — exactly what we're removing).
   - Keep all structural/connectivity/pathfinding tests.

8. **Update serde/bincode round-trip tests**: Remove danger/visibility from expected serialized forms.

### Acceptance Criteria

- [ ] `TravelEdge` has no `danger` or `visibility` fields.
- [ ] No accessor methods for danger/visibility exist.
- [ ] `PrototypeEdgeSpec` has no danger/visibility fields.
- [ ] All tests compile and pass.
- [ ] `cargo clippy --workspace` clean.

---

## Section D — Concrete Route Presence Gate (DEFERRED → E10/E11/E12)

**Principles violated**: 3 (Concrete State Over Abstract Scores)

### Problem

Before any system can implement route-based encounters, ambushes, patrols, or interceptions, the world must support concrete presence on routes — knowing which entities are physically on a route segment and can encounter each other. Without this, implementers will be tempted to reintroduce stored route scores.

### Action

**No code changes in this spec.** Gate notes are added to `specs/E10-production-transport.md`, `specs/E11-trade-economy.md`, and `specs/E12-combat-health.md` (see below) requiring:

1. A concrete route presence model MUST exist before any route-based encounter/risk/interception logic is implemented.
2. It is **forbidden** to reintroduce stored route danger or visibility scores to compensate for missing route presence.
3. The presence model must support: determining which entities are on a route, which travelers can encounter each other, and which agents can witness route events locally.

---

## Section E — Ban Zero-Tick Actions

**Ticket**: FND01-002
**Principles violated**: 6 (Every Action Has Physical Cost)

### Problem

`DurationExpr::Fixed(u32)` allows `Fixed(0)`, meaning a world action can complete instantly with no time cost. This violates Principle 6: "Actions consume time, materials, energy, or attention. Nothing is free."

### Required Changes

**Files**: `crates/worldwake-sim/src/action_semantics.rs`

1. **Change `DurationExpr::Fixed(u32)` to `DurationExpr::Fixed(NonZeroU32)`**:
   - Add `use std::num::NonZeroU32;`
   - Change variant: `Fixed(NonZeroU32)`

2. **Update `resolve()` method**:
   - `DurationExpr::Fixed(n) => n.get()`

3. **Update all call sites** that construct `DurationExpr::Fixed(...)`:
   - Replace `DurationExpr::Fixed(n)` with `DurationExpr::Fixed(NonZeroU32::new(n).unwrap())` where `n > 0`.
   - Files affected: `action_def.rs`, `action_def_registry.rs`, `affordance_query.rs`, `tick_action.rs`, `tick_step.rs`, `start_gate.rs`, `interrupt_abort.rs`.

4. **Remove `Fixed(0)` test case**:
   - In `action_semantics.rs` tests, remove `DurationExpr::Fixed(0)` from `ALL_DURATION_EXPRS` and remove `assert_eq!(DurationExpr::Fixed(0).resolve(), 0)`.

5. **Add enforcement test**:
   - `NonZeroU32::new(0)` returns `None` — add a test asserting this property to document the invariant.

### Acceptance Criteria

- [ ] `DurationExpr::Fixed` wraps `NonZeroU32`.
- [ ] No `Fixed(0)` exists anywhere in the codebase.
- [ ] Enforcement test documents the zero-duration ban.
- [ ] All tests compile and pass.

---

## Section F — Replace Load Match-Arms with Physical Profiles

**Ticket**: FND01-003
**Principles violated**: 2 (No Magic Numbers)

### Problem

`load_per_unit()` and `load_of_unique_item_kind()` in `crates/worldwake-core/src/load.rs` use match-arm tables that assign load values via magic numbers. These values have no named data structure — they're embedded in function bodies, making them invisible to introspection and untraceable.

### Required Changes

**Files**: `crates/worldwake-core/src/load.rs`, `crates/worldwake-core/src/items.rs`

1. **Create `CommodityPhysicalProfile` struct** (in `items.rs` or a new `physical_profile.rs`):
   ```rust
   pub struct CommodityPhysicalProfile {
       pub load_per_unit: LoadUnits,
   }
   ```

2. **Create `UniqueItemPhysicalProfile` struct**:
   ```rust
   pub struct UniqueItemPhysicalProfile {
       pub load: LoadUnits,
   }
   ```

3. **Add profile accessor methods** on `CommodityKind` and `UniqueItemKind`:
   ```rust
   impl CommodityKind {
       pub fn physical_profile(&self) -> CommodityPhysicalProfile { ... }
   }
   impl UniqueItemKind {
       pub fn physical_profile(&self) -> UniqueItemPhysicalProfile { ... }
   }
   ```
   The match arms move inside these methods — same values, but now returned as named struct fields.

4. **Refactor `load_per_unit()` and `load_of_unique_item_kind()`** to delegate:
   ```rust
   pub fn load_per_unit(commodity: CommodityKind) -> LoadUnits {
       commodity.physical_profile().load_per_unit
   }
   ```

5. **Keep existing public API**: `load_per_unit()` and `load_of_unique_item_kind()` remain as convenience functions. No callers need to change.

6. **Add test**: Assert that every `CommodityKind` variant returns a non-zero profile. Assert that every `UniqueItemKind` variant returns a non-zero profile.

### Acceptance Criteria

- [ ] `CommodityPhysicalProfile` and `UniqueItemPhysicalProfile` structs exist.
- [ ] `CommodityKind::physical_profile()` and `UniqueItemKind::physical_profile()` methods exist.
- [ ] `load_per_unit()` delegates to profile.
- [ ] `load_of_unique_item_kind()` delegates to profile.
- [ ] Exhaustive coverage test for all variants.
- [ ] All existing tests pass unchanged.

---

## Section G — Constrain Loyalty Mutations

**Ticket**: FND01-005
**Principles violated**: 1 (Maximal Emergence), 3 (Concrete State Over Abstract Scores)

### Problem

`LoyalTo.strength: Permille` is a scalar disposition. While scalars are not banned outright, they require constraints to prevent script-like threshold logic ("if loyalty < 400 then betray") that bypasses emergent decision-making.

### Required Changes

**Files**: `crates/worldwake-core/src/world/social.rs`, `crates/worldwake-core/src/world_txn.rs`

1. **Add doc-comments** to `set_loyalty()` and `clear_loyalty()` on both `World` and `WorldTxn`:
   - Document the constraint: initial values MUST come from seeded traits, background, or bootstrap events.
   - Document: all changes MUST be event-sourced (already enforced by `WorldTxn` delta recording — verify this).
   - Document: no direct script threshold of the form "if loyalty < X then betray."
   - Document: decisions involving loyalty MUST flow through beliefs, goals, and utility.

2. **Verify event-sourcing**: Confirm that `WorldTxn::set_loyalty()` and `WorldTxn::clear_loyalty()` both record `RelationDelta` entries. (From grep results: they do — `push_relation_delta` is called.)

3. **Add regression test**:
   - Create a test that sets loyalty via `WorldTxn`, commits, and verifies the event log contains a `RelationDelta::Added` with `RelationKind::LoyalTo`.
   - Create a test that clears loyalty via `WorldTxn`, commits, and verifies the event log contains a `RelationDelta::Removed` with `RelationKind::LoyalTo`.

### Acceptance Criteria

- [ ] `set_loyalty()` and `clear_loyalty()` have doc-comments stating the constraints.
- [ ] Event-sourcing verified: loyalty mutations through `WorldTxn` produce `RelationDelta` entries.
- [ ] Regression tests confirm delta recording.
- [ ] No behavioral code changes (this is documentation + verification).

---

## Section H — Future Spec Template Rule

**Action**: Add to `CLAUDE.md` (see modification below).

Every future system spec (E09+) MUST include:

1. **Information-path analysis**: How does each piece of information reach the agents who act on it? Trace the path from source event through perception, witnesses, reports, and belief updates.
2. **Positive-feedback analysis**: Identify every amplifying loop (A increases B, B increases A) in the system.
3. **Concrete dampeners**: For each positive-feedback loop, specify the physical world mechanism that limits amplification. Numerical clamps are not acceptable.
4. **Stored state vs. derived read-model list**: Explicitly enumerate what is authoritative stored state and what is a transient derived computation. No derived value may be stored as authoritative state.

---

## Implementation Order

1. **FND01-001** (Section C): Remove route scores — isolated change in topology.rs
2. **FND01-002** (Section E): Ban zero-tick actions — isolated change in action_semantics.rs + call sites
3. **FND01-003** (Section F): Physical profiles — isolated change in load.rs + items.rs
4. **FND01-004** (Section A): BeliefView rename — touches multiple sim files but is a rename refactor
5. **FND01-005** (Section G): Loyalty constraints — doc-comments + tests, no behavioral changes

Tickets 1–3 are independent and can be parallelized. Ticket 4 is a larger rename. Ticket 5 is documentation + verification.

## Phase Gates

- **E10/E11/E12** (Phase 2: Survival & Logistics): MUST NOT implement route-based encounter/risk/interception logic until a concrete route presence model exists. Forbidden to reintroduce stored route danger/visibility scores.
- **E14** (Phase 3: Perception & Beliefs): MUST implement time-per-hop propagation, consultable-at-location public records, adjacent-place spillover limits, and belief traceability.

## Cross-References

- `docs/FOUNDATIONS.md` — Principles 1, 2, 3, 6, 7, 10
- `brainstorming/phase-1-foundations-alignment-correction.md` — original analysis
- `specs/E14-perception-beliefs.md` — Section B deferral target
- `specs/E10-production-transport.md` — Section D gate target
- `specs/E11-trade-economy.md` — Section D gate target
- `specs/E12-combat-health.md` — Section D gate target

## Tests Summary

| Ticket | Test | Description |
|--------|------|-------------|
| FND01-001 | compile + existing | `TravelEdge` has no danger/visibility fields |
| FND01-002 | enforcement | `NonZeroU32::new(0)` returns `None` |
| FND01-003 | exhaustive coverage | Every commodity/unique-item kind has a non-zero physical profile |
| FND01-004 | divergent beliefs | Two stub belief views produce different affordances |
| FND01-005 | delta recording | Loyalty set/clear via `WorldTxn` produces `RelationDelta` entries |

## Archive Note (2026-03-10)

Archived after completion of the five FND01 Phase 1 alignment tickets:

- `FND01PHA1FOUALI-001`
- `FND01PHA1FOUALI-002`
- `FND01PHA1FOUALI-003`
- `FND01PHA1FOUALI-004`
- `FND01PHA1FOUALI-005`
