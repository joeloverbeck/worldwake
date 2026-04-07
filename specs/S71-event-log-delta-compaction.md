# S71: Event Log Delta Compaction

## Summary

The append-only event log stores full before+after `ComponentValue` snapshots in every `ComponentDelta::Set` record. For high-churn components like `AgentBeliefStore` (updated every tick per agent via perception), this stores two complete copies of the belief store (~150 KB each at steady state) in every perception event. With 20 agents, the event log grows by ~6 MB/tick from belief-store deltas alone, reaching multi-gigabyte RSS within a few hundred ticks.

This spec addresses the unbounded memory growth by replacing full component snapshots in `ComponentDelta` with compact structural diffs, while preserving the causal history, debuggability, and replay fidelity that the event log provides.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Motivation

Profiling data (2880-tick T30 soak, seed 0):

| Tick | Events | RSS (MB) | Avg KB/event |
|------|--------|----------|-------------|
| 60 | 2,353 | 253 | — |
| 120 | 5,440 | 686 | 140 |
| 300 | 19,477 | 2,914 | 153 |
| 600 | 51,548 | 6,266 | — |
| 1080 | 78,357 | 8,733 | — |
| ~1100 | — | OOM killed | — |

Comparison with S52 baseline (before belief stores grew):

| Metric | S52 | Current | Ratio |
|--------|-----|---------|-------|
| Avg KB/event | 26 | 153 | 5.8× |
| RSS at 300 ticks | 503 MB | 2,914 MB | 5.8× |

The 5.8× growth in per-event size directly corresponds to belief stores gaining more social observations (S56), entity claims (S54), and institutional beliefs — all of which are stored in full in every `ComponentDelta::Set`.

CI soak workflows (10,080 ticks) went from ~6 minutes to 19+ minutes. The soak binary is OOM-killed on WSL2 within ~1,100 ticks.

## Crates

- `worldwake-core` (delta representation, event record, world transaction)
- `worldwake-sim` (save/load if it reads deltas)

## Dependencies

None. Spec S70 (belief store query encapsulation) is independent and complementary.

## Design Goals

- **Bounded event log memory growth** — event log RSS should be O(events × avg_delta_size), not O(events × full_component_size)
- **Preserve causal history** — every component change is still recorded and queryable (FND-29A)
- **Preserve debuggability** — the "what changed and why" question remains answerable (FND-29)
- **Preserve replay fidelity** — save/load roundtrip and replay determinism are unchanged (FND-12)
- **Preserve verification** — `verify_causal_link_integrity` and event-log-based assertions continue to work

## Non-Goals

- Changing the append-only invariant of the event log (FND-29A is non-negotiable)
- Event log compaction by removing old events (that changes history)
- Reducing the number of events emitted per tick (that's a separate design concern)
- Changing belief store semantics or capacity

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-12 (Performance Compresses Computation, Never Causality) | **Core constraint.** The delta must preserve what changed. The new representation records the same causal fact (component X on entity Y changed from state A to state B) with less memory. It compresses the encoding, not the causality. |
| FND-29 (Debuggability) | Preserved. Structural diffs answer "what fields changed?" directly. Full reconstruction remains possible by replaying diffs from initial state if needed. |
| FND-29A (Causal History Is Append-Only) | Unchanged. Events are still appended, never mutated or deleted. Only the delta representation within each event changes. |
| FND-27 (Derived Summaries Are Caches) | Aligned. If any downstream consumer needs the full before/after snapshot, it can derive it from authoritative world state at the relevant tick — the full snapshot in the delta was always a convenience, not the source of truth. |
| FND-28 (No Backward Compatibility) | Aligned. The old `ComponentDelta::Set { before, after }` format is replaced, not wrapped. Save migration normalizes old format if encountered. |

## Information-Path Analysis

Component deltas flow through one path: `WorldTxn::commit` → `EventLog::emit` → stored in `events: Vec<EventRecord>`. Consumers read them via `record.state_deltas()`. The change is to the representation stored in that path, not to the path itself.

Consumers of `state_deltas()`:
1. **Perception** (`perception.rs:1004, 1039`) — reads `RelationDelta` and `ComponentDelta` to extract institutional claims. Does NOT read `ComponentValue` payloads.
2. **Verification** (`verification.rs:187`) — reconstructs world state from deltas. Reads `ComponentDelta::Set { after }` to build the final component state. This is the primary consumer of full snapshots.
3. **Production** (`production.rs:278`) — checks for specific component kinds in deltas.
4. **Combat/BanditCamp** — check for specific delta patterns (death, wounds).
5. **Observer CLI** — prints delta counts and details.
6. **Save/Load** — hashes the event log for determinism checks.

Of these, only **verification** and **observer CLI** actually inspect `ComponentValue` payloads. All others match on delta structure (entity, component_kind) without examining the full before/after values.

## Positive-Feedback Analysis

No amplifying loops. This spec reduces memory per event; it does not change event emission rates or belief update frequency.

## Concrete Dampeners

N/A — no positive feedback loops introduced.

## Stored State vs. Derived Read-Model

| Item | Classification |
|------|---------------|
| `ComponentDelta` in event log | Authoritative stored state (part of causal history) |
| Full component before/after snapshots | **Currently stored, proposed to become derivable.** The authoritative component state lives in `World`'s component tables. The delta in the event log needs only enough information to describe what changed — the full state can be reconstructed by applying deltas to the initial state. |

## Design Options

### Option A: Compact Structural Diffs for Large Components

Replace `ComponentDelta::Set { before: Option<ComponentValue>, after: ComponentValue }` with a representation that stores only the changed fields for components above a size threshold.

For `AgentBeliefStore`, instead of storing the full store twice (~300 KB), store:
- Which `known_entities` entries were added/removed/changed (typically 0-3 per tick)
- Which `social_observations` were added/removed (typically 0-5 per tick)
- Which `told_beliefs`/`heard_beliefs` entries changed
- Which `entity_claims` changed

Expected delta size: ~1-5 KB instead of ~300 KB.

**Tradeoff**: Requires a diff/apply implementation per component type. Adds complexity to the delta model. Verification would need to apply diffs sequentially instead of reading snapshots directly.

### Option B: Store Only the `after` Value, Drop `before`

Remove the `before: Option<ComponentValue>` field from `ComponentDelta::Set`. The "before" state is derivable from the previous event's "after" or from the initial world state.

Expected savings: ~50% of current delta cost (one snapshot instead of two).

**Tradeoff**: Simpler than Option A but only halves the problem. Belief stores are still ~150 KB each. With 20 agents, that's still ~3 MB/tick.

### Option C: Component Delta Summarization

Replace full `ComponentValue` with a `ComponentDeltaSummary` that records:
- `entity: EntityId`
- `component_kind: ComponentKind`
- `change_description: ComponentChangeSummary` — an enum with per-component-kind variants that describe what changed without storing the full value

For belief stores: `BeliefStoreChanged { entities_updated: Vec<EntityId>, social_obs_added: u16, social_obs_removed: u16, ... }`.

Verification would reconstruct full state from authoritative world snapshots at tick boundaries rather than from event deltas.

**Tradeoff**: Loses the ability to reconstruct exact component state purely from the event log. Verification needs periodic world snapshots. But the causal fact ("agent X's beliefs changed because of perception at tick Y") is preserved.

### Option D: Tiered Storage — Compact In-Memory, Full On-Disk

Keep the current full-snapshot format for serialization (save files, replay), but store only compact summaries in the live in-memory event log. When the event log is serialized for save/load, expand summaries back to full snapshots.

**Tradeoff**: Two representations to maintain. But it separates the memory pressure (runtime) from the archival fidelity (save files).

## Recommendation

**Option A (compact structural diffs)** for `AgentBeliefStore` and other large components, with Option B as a fallback for components where writing a custom diff is not worthwhile.

Rationale:
- Option A gives 30-100× memory reduction for belief store deltas (1-5 KB vs 150 KB)
- It preserves full reconstructability — applying diffs to initial state yields the exact same result as storing full snapshots
- It aligns with FND-12: same causal information, less memory
- The diff/apply logic can be implemented incrementally: start with `AgentBeliefStore` (the worst offender), measure, then decide if other components need it

Option C loses information that verification currently uses. Option D adds maintenance burden. Option B is a partial fix that may need to be revisited.

## Deliverables

### 1. Compact Delta Type for Belief Store Changes

Define a `BeliefStoreDiff` struct that captures only the mutations:
```
struct BeliefStoreDiff {
    known_entities_set: Vec<(EntityId, BelievedEntityState)>,
    known_entities_removed: Vec<EntityId>,
    social_observations_added: Vec<SocialObservation>,
    social_observations_removed_count: u16,
    told_beliefs_set: Vec<(TellMemoryKey, ToldBeliefMemory)>,
    told_beliefs_removed: Vec<TellMemoryKey>,
    heard_beliefs_set: Vec<(TellMemoryKey, HeardBeliefMemory)>,
    heard_beliefs_removed: Vec<TellMemoryKey>,
    entity_claims_set: Vec<(EntityId, Vec<EntityBeliefClaim>)>,
    entity_claims_removed: Vec<EntityId>,
    institutional_beliefs_set: Vec<(InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>)>,
    institutional_beliefs_removed: Vec<InstitutionalBeliefKey>,
}
```

### 2. Diff Computation in `replace_simple_component`

When `component_kind == ComponentKind::AgentBeliefStore`, compute a structural diff between before and after instead of storing both full values. The delta enum gets a new variant:
```
enum ComponentDelta {
    Set { entity, component_kind, before: Option<ComponentValue>, after: ComponentValue },
    CompactSet { entity, component_kind, diff: ComponentDiff },
    Removed { entity, component_kind, before: ComponentValue },
}
```

### 3. Apply Logic for Verification

`verification.rs` reconstruction logic learns to apply `CompactSet` diffs to its running component state instead of replacing with full snapshots.

### 4. Serialization Compatibility

`bincode` serialization handles the new variant. Save files written with the new format are not loadable by old code (FND-28: no backward compatibility in live authority paths).

## Files to Touch

- `crates/worldwake-core/src/delta.rs` (modify — new delta variant, diff types)
- `crates/worldwake-core/src/belief.rs` (modify — diff computation between two `AgentBeliefStore` instances)
- `crates/worldwake-core/src/world_txn.rs` (modify — use compact diff for belief store sets)
- `crates/worldwake-core/src/verification.rs` (modify — apply compact diffs)
- `crates/worldwake-core/src/event_record.rs` (modify if delta representation changes EventView)
- `crates/worldwake-cli/src/handlers/events.rs` (modify — display compact diffs)

## Out of Scope

- Compacting non-belief-store components (measure first, then decide)
- Event log pruning or garbage collection (violates FND-29A)
- Reducing event emission frequency
- Changing perception update granularity (e.g., batching multiple agents into one event)

## Acceptance Criteria

### Memory

- RSS at 300 ticks (seed 0, T30 world) drops from ~2,914 MB to under 600 MB
- RSS at 2,880 ticks stays under 2 GB
- Soak binary (10,080 ticks) completes without OOM on a machine with 8 GB RAM

### Correctness

- `verify_causal_link_integrity` passes on soak runs
- Save/load roundtrip produces identical world hash
- All golden tests pass with identical hashes (deterministic replay preserved)
- `cargo test --workspace` passes

### Performance

- CI soak workflow completes within 12 minutes (down from 19+)
- Per-tick timing at tick 300 drops from ~85ms to under 30ms (matching S52 baseline proportionally)

## Test Plan

### New Tests

1. `crates/worldwake-core/src/belief.rs` — unit tests for `BeliefStoreDiff::compute` and `BeliefStoreDiff::apply` (roundtrip: apply(compute(before, after), before) == after)
2. `crates/worldwake-core/src/delta.rs` — serialization roundtrip for `ComponentDelta::CompactSet`
3. `crates/worldwake-core/src/verification.rs` — verification with compact diffs produces same final state as full snapshots

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai` (golden tests for determinism)
3. `cargo test -p worldwake-ai --features soak --test golden_soak` (soak for memory)
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`
