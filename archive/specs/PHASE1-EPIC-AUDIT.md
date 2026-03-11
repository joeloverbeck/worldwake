**Status**: COMPLETED

# Phase 1 Epic Audit: E01-E08

## What I changed
The revised epics tighten the Phase 1 stack around the actual gate:

1. **Determinism is now explicit policy, not a hope**
   - no unordered authoritative collections
   - no float fields in authoritative topology
   - canonical serialization and hashing are required

2. **The ECS direction was corrected**
   - removed the `TypeId` / `Box<dyn Any>` direction from E03
   - replaced it with explicit typed component tables

3. **Hybrid identity now matches the spec**
   - stackable bulk lots are distinct from unique items
   - weapons are unique items, not stackable lots
   - waste is conserved, not a silent sink

4. **Placement semantics were fixed**
   - `LocatedIn` is the effective place
   - `ContainedBy` is the immediate parent
   - descendants inherit place from their container chain

5. **Event provenance is now enforceable**
   - E06 adds a mutation journal / transaction layer
   - causal completeness is no longer hand-wavy

6. **Action state is save/load safe**
   - no borrowed `&ActionDef` in active action state
   - affordances are deterministic and view-based

7. **Replay got a stricter contract**
   - canonical state / event hashes
   - deterministic tick, input, and action ordering
   - RNG state is serialized explicitly

## Dependency changes
These revisions intentionally tighten a couple of dependencies:

- `E03` now depends on `E02` because the authoritative world model composes topology directly
- `E05` now depends on `E04` because legal placement / custody semantics need the item and container model first

That is a little less parallel on paper, but much more coherent for a greenfield Phase 1.

## Why the original set needed revision
The original docs had enough good intent to sketch the stack, but they still left several direct failure modes for the gate:
- unordered maps / sets in authoritative state
- float-valued topology metrics
- dynamic erased component storage
- stackable weapon lots
- ambiguous `LocatedIn` vs `ContainedBy` semantics
- event provenance without a mutation journal
- active action state that would be awkward to serialize

Those are not cosmetic issues. They would have made the gate fragile or outright misleading.

## Outcome
- Completion date: 2026-03-11
- What actually changed: finalized the Phase 1 epic audit documenting the corrected E01-E08 direction around determinism, typed ECS storage, item identity, placement semantics, event provenance, action-state serialization, and replay guarantees.
- Deviations from original plan: none recorded in this audit document; the work captured here is the revised Phase 1 epic baseline.
- Verification results: reviewed against the current Phase 1 spec set and archived under the repo archival workflow.
