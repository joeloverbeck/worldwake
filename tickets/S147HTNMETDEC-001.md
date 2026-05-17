# S147HTNMETDEC-001: Core discriminant mirrors and MethodSchemaId newtype

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — adds `MethodSchemaId` newtype, `MotiveSourceDiscriminant`, and `GoalKindDiscriminant` mirror enums in `worldwake-core`. No runtime behavior change.
**Deps**: spec `specs/S147-htn-method-decomposition.md` (D12 + MethodSchemaId sub-deliverable)

## Problem

S147 introduces HTN method decomposition. Method definitions need to key biases and aggregations by the *kind* of motive source (Loyalty, Revenge, …) without committing to a specific `WoundId`/`EntityId` payload, and to identify the *kind* of goal a method decomposes without carrying the goal's runtime payload. The existing `MotiveSource` and `GoalKind` enums in `worldwake-core` are payload-bearing, so they cannot directly serve as map keys, bias keys, or method dispatch keys. Methods themselves also need a stable identifier that save/replay payloads can name without referring back to the ai crate. Without these three core-side types, every subsequent S147 ticket either fabricates types or violates the workspace crate layering (`core → sim → systems → ai → cli`).

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `MotiveSource` exists as a payload-bearing enum at `crates/worldwake-core/src/motive_source.rs:14` with 7 variants (`NeedPressure { need }`, `Pain { wound }`, `OfficeDuty { office }`, `Loyalty { other }`, `Greed { opportunity }`, `Shame { reputation_record }`, `Revenge { violation }`). `MotiveSourceRef` lives at line 25. No `MotiveSourceDiscriminant` or `MotiveSourceVariantId` exists in the workspace today.
2. `GoalKind` exists at `crates/worldwake-core/src/goal.rs:62` with ~30 payload-bearing variants. No `GoalKindDiscriminant` exists in the workspace today. The closest existing analog is `GoalDispatchKey` (`crates/worldwake-ai/src/goal_dispatch_key.rs`) but it lives in ai (cannot serve as a core-side key) and carries dispatch-routing semantics rather than being a clean discriminant mirror.
3. No `MethodSchemaId` exists in the workspace today. The newtype is the first introduction of a method identifier surface.
4. Spec `specs/S147-htn-method-decomposition.md` D12 and the spec's Crates section call out `worldwake-core` as the residence for all three types and specifies derives (`Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`) following the `BeliefStatusTag` precedent at `crates/worldwake-core/src/decision_event_payload.rs:231`.
5. Sibling specs forward-reference these types: `specs/S148-portfolio-and-motive-backed-intentions.md:44` references `GoalKindDiscriminant`, `specs/S152-cognitive-archetypes-seeded-diversity.md:116-117` references `MethodSchemaId`. Both are downstream of this ticket and depend on its delivery; no shape negotiation with them is needed in this ticket.

## Architecture Check

1. Adding payload-free discriminant mirrors alongside the source enums (rather than relocating the source enums or introducing dispatch-routing types) follows the `BeliefStatusTag` precedent at `decision_event_payload.rs:231` and the Core-Side Mirror Enum pattern in `.claude/skills/reassess-spec/references/worldwake-validation-patterns.md`. The mirror is mechanically 1:1 with the source enum, derives the full bound set required for `BTreeMap`/`BTreeSet` keying and serialization, and has a single conversion site (`From<&Source>` impl in the same file as the source).
2. No backwards-compatibility aliasing or shims are introduced. The source enums (`MotiveSource`, `GoalKind`) are unchanged; the discriminant mirrors live alongside them.

## Verification Layers

1. Mirror completeness (every source variant maps to exactly one discriminant) → focused unit test (`crates/worldwake-core/tests/discriminant_mirrors.rs`) iterating fixture-constructed source variants and asserting `.discriminant()` round-trips.
2. `GoalKindDiscriminant::ALL` is exhaustive and stable → focused unit test asserting `ALL.len() == <source variant count>` and that every variant appears exactly once.
3. Single-layer ticket — the new types do not participate in any cross-system runtime flow yet; downstream tickets verify the integration layers.

## What to Change

### 1. Define `MethodSchemaId` newtype in core

New file `crates/worldwake-core/src/method_schema_id.rs`:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MethodSchemaId(pub u32);
```

Re-export from `crates/worldwake-core/src/lib.rs` so ai-crate consumers can `use worldwake_core::MethodSchemaId;`.

### 2. Add `MotiveSourceDiscriminant` to `motive_source.rs`

Extend `crates/worldwake-core/src/motive_source.rs` to add the payload-free mirror enum (variants 1:1 with `MotiveSource`), the `From<&MotiveSource>` impl, and the `MotiveSource::discriminant() -> MotiveSourceDiscriminant` accessor method. Derives match the `BeliefStatusTag` precedent: `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.

### 3. Add `GoalKindDiscriminant` to `goal.rs`

Extend `crates/worldwake-core/src/goal.rs` to add the payload-free mirror enum (variants 1:1 with `GoalKind`), the `From<&GoalKind>` impl, the `GoalKind::discriminant() -> GoalKindDiscriminant` accessor method, and a `pub const ALL: &[GoalKindDiscriminant]` constant enumerating every variant. Derives match `MotiveSourceDiscriminant`.

### 4. Round-trip completeness test

New file `crates/worldwake-core/tests/discriminant_mirrors.rs`:
- A fixture function constructs one instance of each `MotiveSource` variant (using sentinel `EntityId`/`WoundId`/etc. payloads) and asserts `source.discriminant()` matches the expected `MotiveSourceDiscriminant` variant for each.
- A fixture function does the same for `GoalKind`/`GoalKindDiscriminant`.
- Asserts `GoalKindDiscriminant::ALL.len() == <source variant count>` and that every variant appears exactly once via `BTreeSet` collection.

## Files to Touch

- `crates/worldwake-core/src/method_schema_id.rs` (new)
- `crates/worldwake-core/src/motive_source.rs` (modify — append `MotiveSourceDiscriminant` + `From` impl + `discriminant()` accessor)
- `crates/worldwake-core/src/goal.rs` (modify — append `GoalKindDiscriminant` + `From` impl + `discriminant()` accessor + `ALL` constant)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `MethodSchemaId`, `MotiveSourceDiscriminant`, `GoalKindDiscriminant`)
- `crates/worldwake-core/tests/discriminant_mirrors.rs` (new)

## Out of Scope

- HTN module in worldwake-ai (separate tickets 004, 006, 007, 008).
- `MethodSchema` and supporting types (separate ticket 004).
- `Discrepancy::MethodFailure` variant (separate ticket 002, depends on `MethodSchemaId` from this ticket).
- `AgentSchemaContextProfile.disabled_methods` (separate ticket 003, depends on `MethodSchemaId` from this ticket).
- Reverse `Discriminant → Source` conversion — no current consumer needs to lift a discriminant back to a source enum; do not add speculatively.

## Acceptance Criteria

### Tests That Must Pass

1. `discriminant_mirrors::motive_source_round_trip_covers_all_variants` — every `MotiveSource` variant projects to the matching `MotiveSourceDiscriminant`.
2. `discriminant_mirrors::goal_kind_round_trip_covers_all_variants` — every `GoalKind` variant projects to the matching `GoalKindDiscriminant`.
3. `discriminant_mirrors::goal_kind_all_constant_is_exhaustive_and_unique` — `GoalKindDiscriminant::ALL` length equals source-enum variant count and contains each variant exactly once.
4. Existing suite: `cargo test -p worldwake-core` passes (no regressions).
5. `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. `MotiveSourceDiscriminant` variants are mechanically 1:1 with `MotiveSource` variants — same names, same arity (unit). No semantic narrowing, merging, or renaming.
2. `GoalKindDiscriminant` variants are mechanically 1:1 with `GoalKind` variants — same names, same arity (unit).
3. Both discriminant enums and `MethodSchemaId` derive `Copy, Hash, Ord, Serialize, Deserialize` — sufficient to serve as `BTreeMap`/`BTreeSet` keys and to round-trip through save/load.
4. No source enum (`MotiveSource`, `GoalKind`) gains semantic changes — the only edits are appending the mirror enum, the `From` impl, the accessor, and (for `GoalKind`) the `ALL` constant.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/tests/discriminant_mirrors.rs` — new — verifies round-trip mapping for both discriminant enums and `ALL`-constant completeness for `GoalKindDiscriminant`.

### Commands

1. `cargo test -p worldwake-core --test discriminant_mirrors`
2. `cargo test -p worldwake-core`
3. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
