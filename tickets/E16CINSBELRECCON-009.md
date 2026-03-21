# E16CINSBELRECCON-009: InstitutionalBeliefRead Derivation Helpers on AgentBeliefStore

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new methods on AgentBeliefStore in worldwake-core
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-003

## Problem

The AI and belief view layers need to derive `InstitutionalBeliefRead<T>` (Unknown / Certain / Conflicted) from the raw `Vec<BelievedInstitutionalClaim>` stored per key. Without derivation helpers, every consumer would need to duplicate the conflict resolution logic. This ticket adds the query surface described in spec §9.

## Assumption Reassessment (2026-03-21)

1. `AgentBeliefStore` in `belief.rs` will have `institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>` after ticket -003. No derivation methods exist yet.
2. Derivation logic: if key absent → Unknown; if all claims agree → Certain(value); if claims disagree → Conflicted(values). "Agree" means same effective holder/membership/candidate.
3. Spec §5 explicitly says: do not collapse multiple claims into one silent winner at storage time. Conflict detection happens at read time.
4. N/A — not a planner ticket yet.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Derivation on `AgentBeliefStore` keeps the logic near the data. Consumers (BeliefView, PlanningSnapshot) call these helpers instead of reimplementing conflict detection.
2. No backward-compatibility shims.

## Verification Layers

1. No beliefs → Unknown → unit test
2. Single claim → Certain → unit test
3. Two agreeing claims (different sources) → Certain → unit test
4. Two conflicting claims → Conflicted → unit test
5. Single-layer ticket — pure derivation methods.

## What to Change

### 1. Add derivation methods to `AgentBeliefStore` in `belief.rs`

```rust
pub fn believed_office_holder(&self, office: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
pub fn believed_membership(&self, faction: EntityId, member: EntityId) -> InstitutionalBeliefRead<bool>;
pub fn believed_support_declaration(&self, office: EntityId, supporter: EntityId) -> InstitutionalBeliefRead<Option<EntityId>>;
pub fn believed_support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)>;
pub fn consultable_records_at(&self, place: EntityId) -> Vec<EntityId>;
```

### 2. Conflict detection logic

For each key:
- Look up `institutional_beliefs[key]`
- If absent or empty → `InstitutionalBeliefRead::Unknown`
- Extract the "effective value" from each claim (e.g., holder EntityId for OfficeHolder)
- If all effective values agree → `Certain(value)`
- If effective values disagree → `Conflicted(values)`

For `consultable_records_at`: scan `known_entities` for entities at the given place that the agent believes are records (from entity belief state).

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add derivation methods to `AgentBeliefStore`)

## Out of Scope

- `PerAgentBeliefView` backing (ticket -010 area)
- `PlanningSnapshot`/`PlanningState` integration (ticket -010)
- AI candidate generation (ticket -012)
- Contradiction tolerance thresholds (AI layer uses `PerceptionProfile.contradiction_tolerance` — not this ticket)

## Acceptance Criteria

### Tests That Must Pass

1. `believed_office_holder` returns `Unknown` when no beliefs exist for the office
2. `believed_office_holder` returns `Certain(Some(holder))` when one claim exists
3. `believed_office_holder` returns `Certain(None)` when claim says office is vacant
4. `believed_office_holder` returns `Conflicted` when two claims name different holders
5. Two claims with same holder but different sources → `Certain` (not Conflicted)
6. `believed_membership` returns `Unknown`, `Certain(true)`, `Certain(false)`, or `Conflicted` correctly
7. `believed_support_declaration` handles unknown/certain/conflicted cases
8. `believed_support_declarations_for_office` aggregates all supporters for an office
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Derivation is pure — no mutation of belief store, no side effects
2. Conflict detection compares effective values, not sources or ticks
3. `Unknown` is only returned when no claims exist (not when claims are old)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` (extend test module) — all derivation methods with Unknown/Certain/Conflicted cases

### Commands

1. `cargo test -p worldwake-core belief`
2. `cargo clippy --workspace && cargo test --workspace`
