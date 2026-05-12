# S141MOTSOULED-001: `MotiveSource` enum and `MotiveSourceRef` carrier in core

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new types in `worldwake-core::motive_source` module
**Deps**: spec S141 deliverable D1 (foundation; blocks 003, 004, 005)

## Problem

S141 (Motive Source Ledger) requires a typed core-resident carrier for motive provenance so `motive_score` can derive from concrete per-agent state references rather than dispatching on `GoalKind` (FND-3: concrete state over abstract scores). Without the carrier, no downstream ticket (003 `RankedGoalSummary` field, 004 `GoalOffer` field, 005 `GoalCommittedPayload` field) can wire up.

This ticket lands the foundation only: type definitions and module registration. No consumer code is touched.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The 7 kept variants' referent types all exist in `worldwake-core` per the S141 reassessment (this session): `HomeostaticNeedId` (`crates/worldwake-core/src/needs.rs:19`), `WoundId` (`crates/worldwake-core/src/wounds.rs:9`), `OpportunityKey` (`crates/worldwake-core/src/goal.rs:201`), `ViolationId` (`crates/worldwake-core/src/violation.rs:20`), `EntityId` (`crates/worldwake-core/src/ids.rs:44`), `Tick` (`crates/worldwake-core/src/ids.rs:57`). No focused/unit, runtime trace, or golden test exercises `MotiveSource` yet — it's a net-new type.
2. The 5 deferred variants (`Fear`, `Obligation`, `Debt`, `Habit`, `Curiosity`) are documented as Phase 12 follow-ups in `specs/S141-motive-source-ledger.md` Deferred Variants table. This ticket does NOT introduce stubs or placeholders for those variants — adding zero-contribution stubs would violate FND-28 (no dead paths in live authoritative paths).
3. Shared abstraction boundary: `MotiveSource` is a new core-resident enum embedded in higher-crate types. Its variant payloads reference only core-resident types so `worldwake-core` retains no upward dependency on `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, or `worldwake-cli`. Derive set (`Clone, Debug, Eq, PartialEq, Serialize, Deserialize`) matches `GoalOffer`'s existing derives at `crates/worldwake-ai/src/goal_model.rs:2037` and the always-on decision-payload convention at `crates/worldwake-core/src/decision_event_payload.rs:11`.

## Architecture Check

1. Restricting the initial variant set to referents that already exist as core types respects S141 Design Goal 5 ("No new authoritative state") and FND-28. The alternative — introducing opaque `pub u64` stubs for `ThreatBeliefId`, `ContractId`, `DebtId`, `HypothesisId`, `HabitId` — would carry no-op variants in a live authoritative type whose `score_motive_source` arms would always return zero. That is the dead-path pattern FND-28 forbids.
2. No backward-compatibility shims: the type is net-new; there are no prior `MotiveSource` definitions to deprecate.
3. Module placement in core (not in ai) lets S136's always-on decision payload (`GoalCommittedPayload`, owned by core) reference `Vec<MotiveSourceRef>` directly without a core-side Tag mirror.

## Verification Layers

1. Type-definition correctness → focused unit test in `crates/worldwake-core/src/motive_source.rs#[cfg(test)]` (round-trip `bincode` serialize/deserialize per variant; exhaustive `match` over the 7 variants compiles without `_ =>` fallback).
2. Crate-boundary preservation → `cargo build -p worldwake-core` alone succeeds with no new upward dependency.
3. Single-layer ticket — additional layer mapping is not applicable until consumers (003/004/005) wire up their fields. The downstream wiring is owned by those tickets, not this one.

## What to Change

### 1. New module `crates/worldwake-core/src/motive_source.rs`

Define:

```rust
use serde::{Deserialize, Serialize};

use crate::{
    goal::OpportunityKey,
    ids::{EntityId, Tick},
    needs::HomeostaticNeedId,
    violation::ViolationId,
    wounds::WoundId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MotiveSource {
    NeedPressure { need: HomeostaticNeedId },
    Pain { wound: WoundId },
    OfficeDuty { office: EntityId },
    Loyalty { other: EntityId },
    Greed { opportunity: OpportunityKey },
    Shame { reputation_record: EntityId },
    Revenge { violation: ViolationId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MotiveSourceRef {
    pub source: MotiveSource,
    pub introduced_tick: Tick,
}
```

Include `#[cfg(test)]` block with two tests: `motive_source_roundtrips_through_bincode` (encode/decode every variant) and `motive_source_ref_roundtrips_through_bincode` (encode/decode the carrier struct).

### 2. Module registration in `crates/worldwake-core/src/lib.rs`

Add `pub mod motive_source;` in the existing module list (alphabetically near `needs` / `obligation`). Re-export `MotiveSource` and `MotiveSourceRef` at crate root following the existing re-export convention used for sibling types (`HomeostaticNeedId`, `WoundId`, `OpportunityKey`, etc.).

## Files to Touch

- `crates/worldwake-core/src/motive_source.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — `pub mod motive_source;` + crate-root re-exports)

## Out of Scope

- Any consumer of the new types. `GoalOffer.motive_sources` is owned by 004; `RankedGoalSummary.motive_source_contributions` is owned by `archive/tickets/S141MOTSOULED-003.md`; `GoalCommittedPayload.decisive_motive_sources` is owned by 005.
- `derive_default_motive_sources` mapping helper — owned by 004 (lives in `worldwake-ai`, not core).
- The 5 deferred variants and their referent substrate (Phase 12 follow-ups per spec's Deferred Variants table).
- `SAVE_FORMAT_VERSION` bump — owned by `archive/tickets/S141MOTSOULED-002.md` (single-shot bump shared with 003/005 via `#[serde(default)]` on their respective new fields).

## Acceptance Criteria

### Tests That Must Pass

1. `motive_source_roundtrips_through_bincode` — every `MotiveSource` variant survives bincode round-trip with byte-identical re-encoding.
2. `motive_source_ref_roundtrips_through_bincode` — `MotiveSourceRef { source, introduced_tick }` survives the same round-trip.
3. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `worldwake-core` has no new upward dependency. Specifically, the new module's `use` list contains only `crate::*` paths and `serde`.
2. The 7 kept variants and their payload shapes match `specs/S141-motive-source-ledger.md` Deliverable D1 exactly — variant names, payload field names, and field types are not allowed to drift between spec and code.
3. Derive set on both types is exactly `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (no `Copy`, no `Hash`, no `Ord`) — matches `GoalOffer`'s derive set at `crates/worldwake-ai/src/goal_model.rs:2037`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/motive_source.rs#[cfg(test)]` — two round-trip tests as named above. Pure focused unit tests; no runtime/integration harness.

### Commands

1. `cargo test -p worldwake-core motive_source`
2. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
3. `cargo build --workspace` — confirms downstream crates still compile after the new module is exported (no consumer changes expected at this point).

## Outcome

Completed on 2026-05-12.

- Added `crates/worldwake-core/src/motive_source.rs` with `MotiveSource` and `MotiveSourceRef` exactly on the S141 D1 variant/payload boundary.
- Registered `worldwake_core::motive_source` and re-exported `MotiveSource` / `MotiveSourceRef` at the crate root.
- Added focused bincode round-trip tests for every initial `MotiveSource` variant and for `MotiveSourceRef`, including byte-identical re-encoding checks.
- No downstream consumers were touched; `GoalOffer`, `RankedGoalSummary`, and `GoalCommittedPayload` wiring remains owned by S141MOTSOULED-003/004/005.

## Verification Result

- Passed `cargo test -p worldwake-core motive_source`
- Passed `cargo test -p worldwake-core`
- Passed `cargo build -p worldwake-core`
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
- Passed `cargo build --workspace`
