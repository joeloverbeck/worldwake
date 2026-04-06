# S54ENTBEL-001: Claim types and AgentBeliefStore extension

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new types in worldwake-core, AgentBeliefStore field additions, derivation function, standalone claim-capacity enforcement, save format bump
**Deps**: None

## Problem

`BelievedEntityState` is both the source of truth and the working-memory summary for entity beliefs. This conflation prevents contradictory reports from coexisting, provenance queries, and uneven correction. This ticket adds the claim-based substrate types and persisted storage alongside the existing direct-write path, enabling incremental migration in ticket 002.

## Assumption Reassessment (2026-04-05)

1. `AgentBeliefStore` at `crates/worldwake-core/src/belief.rs:40-47` has 6 fields. `known_entities: BTreeMap<EntityId, BelievedEntityState>` is the current direct-write target. Confirmed.
2. `BelievedEntityState` at `belief.rs:738-755` has 13 fields. The spec's `EntityBeliefAspect` enum (11 variants) maps to all independent fields — `observed_tick` and `source` are per-summary metadata, not aspects. Confirmed.
3. `BeliefConfidencePolicy` at `belief.rs:1274-1282` has 7 fields including `staleness_penalty_per_tick`. Used by `derive_entity_summary`. Confirmed.
4. `PerceptionSource` at `belief.rs:1265-1270` has 4 variants. Used on claims for provenance. Confirmed.
5. `enforce_capacity` at `belief.rs:108` operates on `known_entities`. New `enforce_entity_claim_capacity` will operate on `entity_claims`. Modeled after `enforce_institutional_capacity` at `belief.rs:489`. Confirmed.
6. `BelievedEvidenceState` exists at `belief.rs:752` — the `Evidence` aspect and `EvidenceState` ClaimValue variant cover this. Confirmed.
7. All ClaimValue types exist: `BelievedActivity` (693), `BelievedContentionState` (700), `BelievedArtifactState` (715), `BelievedEvidenceState` (752), `Wound`, `WorkstationTag`, `ResourceSource`, `Permille`, `Quantity`. Confirmed.
8. `AgentBeliefStore` is serialized as part of authoritative world state. Adding persisted fields changes the on-disk shape immediately, so the save-version change belongs in this ticket, not deferred to 002. Confirmed from `crates/worldwake-sim/src/save_load.rs`.
9. The original invariant "`known_entities` is always derivable from `entity_claims`" contradicts the coexistence plan where perception still writes `known_entities` directly in 001. Full source-of-truth migration must remain deferred to 002. Confirmed.
10. `BelievedEntityState` has only one `source` and `observed_tick`, while different aspects can be won by different claims. Summary metadata therefore needs an explicit deterministic reduction rule in this ticket. Confirmed.

## Architecture Check

1. Claim types are added alongside the existing direct-write path — no perception or planner behavior changes in this ticket. `entity_claims` starts empty and `known_entities` continues to be written directly by perception. Ticket 002 migrates perception to use claims as the source of truth.
2. Because `AgentBeliefStore` is persisted directly, this ticket includes the save-version bump now. Older save versions are rejected rather than migrated. Ticket 002 owns later behavioral migration, not this shape change.
3. `derive_entity_summary` is a pure function over claims. It returns `Option<BelievedEntityState>` so entities with no surviving claims can be removed cleanly instead of forcing a fake summary.
4. Summary-level `source` / `observed_tick` remain a lossy cache artifact here. They are derived deterministically from the highest-ranked winning claim across all aspect winners.
5. `ClaimId(pub u64)` stays consistent with `ViolationId` and `EvidenceEntryId`.

## Verification Layers

1. All new types compile with required derives → `cargo build -p worldwake-core`
2. `derive_entity_summary` produces correct `BelievedEntityState` from claims when claims exist → focused unit test
3. Confidence-based resolution: highest-confidence claim wins → focused unit test
4. Staleness penalty applied during resolution → focused unit test
5. `enforce_entity_claim_capacity` evicts old/low-confidence claims → focused unit test
6. Re-derivation after eviction updates only affected claim-backed `known_entities` entries consistently → focused unit test
7. Current-format save/load roundtrip preserves the new `AgentBeliefStore` shape → focused save test
8. Single-layer ticket plus save boundary only — no cross-system verification needed.

## What to Change

### 1. Add claim types

Create `crates/worldwake-core/src/entity_belief_claim.rs` (or add to `belief.rs`):

```rust
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ClaimId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EntityBeliefAspect {
    Location,
    Inventory(CommodityKind),
    Alive,
    Wounded,
    Activity,
    WorkstationPresent,
    ResourceAvailable(CommodityKind),
    ContentionState,
    ArtifactState,
    Courage,
    Evidence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimValue {
    Place(Option<EntityId>),
    Quantity(Quantity),
    Bool(bool),
    Activity(Option<BelievedActivity>),
    WorkstationTag(Option<WorkstationTag>),
    ResourceSource(Option<ResourceSource>),
    ContentionState(Option<BelievedContentionState>),
    ArtifactState(Option<BelievedArtifactState>),
    Courage(Option<Permille>),
    WoundSnapshot(Vec<Wound>),
    EvidenceState(Option<BelievedEvidenceState>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntityBeliefClaim {
    pub claim_id: ClaimId,
    pub subject: EntityId,
    pub aspect: EntityBeliefAspect,
    pub value: ClaimValue,
    pub source: PerceptionSource,
    pub acquired_tick: Tick,
    pub claimed_event_tick: Option<Tick>,
    pub confidence: Permille,
}
```

### 2. Extend AgentBeliefStore

In `crates/worldwake-core/src/belief.rs`, add fields to `AgentBeliefStore`:

```rust
pub entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>,
pub next_claim_id: ClaimId,
```

Update `Default` impl, `new()`, and any constructor to initialize both fields (empty map, `ClaimId(0)`).

### 3. Implement derive_entity_summary

```rust
pub fn derive_entity_summary(
    claims: &[EntityBeliefClaim],
    current_tick: Tick,
    policy: &BeliefConfidencePolicy,
) -> Option<BelievedEntityState>
```

For each `EntityBeliefAspect`, find the claim with highest effective confidence (recomputed with staleness: `confidence - staleness_penalty_per_tick * (current_tick - acquired_tick)`). The winning claim's value populates the corresponding `BelievedEntityState` field. Tie-break: newer `acquired_tick` wins, then higher `claim_id`. If no claims survive, return `None`. Summary-level `source` and `observed_tick` come from the highest-ranked winning claim across all aspects.

### 4. Implement enforce_entity_claim_capacity

```rust
pub fn enforce_entity_claim_capacity(
    &mut self,
    profile: &PerceptionProfile,
    current_tick: Tick,
)
```

- Evict claims where `current_tick - acquired_tick > memory_retention_ticks`
- If total claims per entity exceed `memory_capacity`, evict lowest effective-confidence claims
- After eviction, re-derive or clear `known_entities` only for affected entities that already have claim-backed state
- `enforce_capacity` itself stays on the old `known_entities` path in this ticket; ticket 002 switches entity-belief retention over to claims

### 5. Save format boundary

In `crates/worldwake-sim/src/save_load.rs`:

- Bump `SAVE_FORMAT_VERSION` from 26 to 27
- Persist `entity_claims` and `next_claim_id` in the current format
- Reject pre-27 saves as unsupported rather than migrating them
- Do not synthesize claims from `known_entities`; that behavioral migration remains owned by ticket 002

### 6. Re-export types

Add module declaration and re-exports in `crates/worldwake-core/src/lib.rs`.

## Files to Touch

- `crates/worldwake-core/src/entity_belief_claim.rs` (new — or extend belief.rs)
- `crates/worldwake-core/src/belief.rs` (modify — AgentBeliefStore fields, derive function, capacity enforcement)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump + current-format persistence only)

## Out of Scope

- Perception migration to emit claims — ticket 002
- Any hypothetical claim synthesis from older saves — out of scope and not supported
- Golden tests — ticket 003
- Modifying the planner (reads known_entities unchanged)
- Institutional beliefs (already claim-based)

## Acceptance Criteria

### Tests That Must Pass

1. `derive_entity_summary` with single claim per aspect produces correct `BelievedEntityState`
2. `derive_entity_summary` with multiple claims per aspect: highest confidence wins
3. `derive_entity_summary` with staleness: fresh claim beats stale high-confidence claim
4. `derive_entity_summary` tie-break: newer acquired_tick wins, then higher claim_id
5. `enforce_entity_claim_capacity` evicts claims beyond retention ticks
6. `enforce_entity_claim_capacity` evicts lowest-confidence when over capacity
7. Re-derivation after eviction updates or clears only affected claim-backed `known_entities`
8. Current-format save/load roundtrip preserves `known_entities`, `entity_claims`, and `next_claim_id`
9. Existing suite: `cargo test --workspace`

### Invariants

1. For entities already using the new claim lane, `known_entities` is derivable from `entity_claims`; full global derivability is deferred to ticket 002
2. `next_claim_id` is monotonically increasing — never reused
3. Claim eviction never leaves a stale derived `known_entities` entry for an entity whose summary came from claims
4. Existing perception and planner behavior unchanged (claims start empty, `known_entities` still written directly)
5. `SAVE_FORMAT_VERSION == 27`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` (or `entity_belief_claim.rs`) — `derive_entity_summary`: single-claim, multi-claim, staleness, tie-break, empty-input handling
2. `crates/worldwake-core/src/belief.rs` — `enforce_entity_claim_capacity`: retention eviction, capacity eviction, re-derivation consistency
3. `crates/worldwake-sim/src/save_load.rs` — version bump + current-format persistence only

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim -- save`
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-05
- Added the claim substrate in `worldwake-core`: `ClaimId`, `EntityBeliefAspect`, `ClaimValue`, and `EntityBeliefClaim`, plus `AgentBeliefStore.entity_claims` / `next_claim_id`, `record_entity_claim`, `derive_entity_summary`, and `enforce_entity_claim_capacity`.
- Registered the expected current-shape fallout across shared struct literals and re-exports in `worldwake-core` and AI fixtures so the new persisted fields are present everywhere the component is constructed.
- Kept the save boundary honest at `SAVE_FORMAT_VERSION = 27` with current-format-only support. Older saves are rejected rather than migrated.
- Deviation from the original ticket chain: the earlier migration plan was corrected away. Focused save coverage now proves roundtrip of populated `entity_claims` and `next_claim_id` instead of any older-version load path.
- Verification:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-sim -- save --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
