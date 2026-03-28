# S34GENEPIACT-001: Core types for proactive epistemic goals

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core`: new epistemic-domain types, new goal variant, new agent component registration
**Deps**: S27 (completed), `specs/S34-general-epistemic-actions.md`

## Problem

S34 depends on a proactive epistemic goal family, but `worldwake-core` does not yet expose the shared value types needed to describe that goal in a stable, serializable, schema-registered way. Until those types exist, downstream S34 tickets in `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` cannot add lawful action payloads, planner operators, or candidate generation for deliberate belief verification.

## Assumption Reassessment (2026-03-28)

1. The live shared abstraction boundary under audit is `worldwake-core` goal identity plus agent component registration: `GoalKind` / `GoalKey` in `crates/worldwake-core/src/goal.rs` and the authoritative component manifest rooted in `crates/worldwake-core/src/component_schema.rs`. This is a single-layer core contract ticket, not an AI/planner ticket yet.
2. `GoalKind` currently lives in `crates/worldwake-core/src/goal.rs` and already contains `InvestigateViolation { violation_id, place }` as the closest epistemic precedent. No `VerifyBelief` variant exists.
3. `GoalKey` canonical-field derivation is implemented only through `impl From<GoalKind> for GoalKey` in `crates/worldwake-core/src/goal.rs`; there is no separate `goal_key()` helper. Ticket scope must target that conversion and its tests directly.
4. `ViolationKind` / `ViolationDispositionProfile` in `crates/worldwake-core/src/violation.rs` remain the right pattern reference for value semantics and per-agent disposition storage, but they model reactive mismatch handling after observation. S34 needs proactive verification subjects and verification preferences, so reusing `violation.rs` directly would collapse two different epistemic stages into one module.
5. Mismatch: `crates/worldwake-core/src/verification.rs` already exists and owns event-log/world-state verification (`verify_completeness`, `VerificationError`). The original ticket’s proposed new `verification.rs` module path would collide with live architecture. Correction: add a new epistemic-domain module with a non-conflicting name instead of overloading the existing verification namespace.
6. The component manifest is macro-driven. Adding a new schema entry in `component_schema.rs` automatically expands into `ComponentKind` / `ComponentValue` (`delta.rs`), typed tables (`component_tables.rs`), world component APIs (`world.rs`), and transaction setters (`world_txn.rs`) via `with_component_schema_entries!`. The original ticket understated these compile surfaces and file touches.
7. The live crate already follows the pattern "agent gets only universally required default epistemic/perception components on creation" in `World::create_agent()` (`AgentBeliefStore`, `PerceptionProfile`, `TellProfile`). `VerificationDispositionProfile` should remain optional and agent-scoped, matching the S34 spec’s diversity rule rather than being auto-installed on every agent.
8. `cargo test -p worldwake-core -- --list` succeeds and confirms the current focused harness already contains in-module round-trip tests in `goal.rs`, `violation.rs`, and component/world transaction tests. The ticket should target those existing test surfaces instead of inventing non-existent test modules.
9. The current macro-generated `WorldTxn` manifest test (`txn_simple_set_components_match_manifest_projection`) means adding the new component is not "out of scope" for deltas/txn wiring; it is an expected compile and manifest consequence of the schema entry. Ticket scope must explicitly account for that.
10. Spec alignment check: `specs/S34-general-epistemic-actions.md` still calls for `VerificationSubject`, `VerificationDispositionProfile`, and a proactive `VerifyBelief` goal kind. That design remains architecturally sound because it keeps knowledge-seeking explicit, local, and cost-bearing without adding global lookups or shortcuts.
11. Mismatch + correction: the original file list was incomplete. Besides the new module, `goal.rs`, `component_schema.rs`, and `lib.rs`, the live manifest/import layout will also require the standard schema-projection files that mention component types by name (`component_tables.rs`, `delta.rs`, `world.rs`, and likely `world_txn.rs`).

## Architecture Check

1. A dedicated epistemic-domain module is cleaner than either alternative:
   - better than `violation.rs`, because proactive verification targets are not themselves violations;
   - better than reusing `verification.rs`, because that module already means structural event-log/world verification in this crate.
2. The robust end-state is a narrow new core namespace for epistemic-action shared types, with `GoalKind::VerifyBelief` carrying the full verification subject directly. That keeps downstream planner/action work explicit and serializable without aliasing or back-compat wrappers.
3. `VerificationDispositionProfile` should stay optional on agents. That preserves P20 diversity and avoids smuggling a global "all agents verify beliefs the same way" policy into authoritative defaults.
4. No backwards-compatibility shims or alias paths are acceptable here. If downstream matches break because `GoalKind` gained a new variant, those sites should be updated directly.

## Verification Layers

1. `VerificationSubject` and `VerificationDispositionProfile` remain deterministic, serializable core values -> focused unit round-trip tests in the defining module.
2. `GoalKind::VerifyBelief` carries the correct canonical identity fields into `GoalKey` -> focused unit tests in `goal.rs`.
3. `VerificationDispositionProfile` is agent-only in the authoritative schema -> focused authoritative world/world-txn test that agent insertion succeeds and non-agent insertion fails with schema validation.
4. Macro manifest wiring remains complete after the new component entry -> existing `world_txn` manifest tests plus crate compilation via `cargo test -p worldwake-core`.
5. Additional decision-trace, action-trace, or golden proof surfaces are not applicable yet because this ticket does not change candidate generation, planner search, or action execution.

## What to Change

### 1. Add a dedicated epistemic core module

Create a new `worldwake-core` module for S34 epistemic shared types using a non-conflicting name such as `epistemic.rs`.

Define:

- `VerificationSubject` with:
  - `EntityLocation { entity: EntityId, place: EntityId }`
  - `SupplyAvailability { commodity: CommodityKind, source: EntityId, place: EntityId }`
- `VerificationDispositionProfile` with:
  - `belief_verification_threshold: Permille`
  - `verify_belief_duration_ticks: NonZeroU32`
  - `witness_query_duration_ticks: NonZeroU32`
  - `verification_motive_weight: Permille`
  - `ask_memory_retention_ticks: u32`

Derive the same value-semantic traits used by adjacent core types, and implement `Component` for `VerificationDispositionProfile`.

### 2. Extend `GoalKind` with proactive verification identity

Add:

```rust
VerifyBelief {
    subject: VerificationSubject,
    generation_tick: Tick,
}
```

to `crates/worldwake-core/src/goal.rs`, and update `impl From<GoalKind> for GoalKey` so canonical fields reflect the verification subject:

- `EntityLocation` -> `entity = Some(entity)`, `place = Some(place)`, `commodity = None`
- `SupplyAvailability` -> `entity = Some(source)`, `place = Some(place)`, `commodity = Some(commodity)`

`generation_tick` must remain part of `GoalKind` identity rather than being split into a side channel.

### 3. Register the new profile in the authoritative component manifest

Add `VerificationDispositionProfile` to `crates/worldwake-core/src/component_schema.rs` as an agent-only component, following the existing `ViolationDispositionProfile` pattern.

Because the component manifest is macro-projected, update any required imports/usages in:

- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/world_txn.rs`

Only make the minimal edits needed to keep the generated schema surfaces compiling cleanly.

### 4. Wire the module through crate exports

Expose the new module from `crates/worldwake-core/src/lib.rs` and re-export:

- `VerificationDispositionProfile`
- `VerificationSubject`

Do not disturb the existing `verification` re-export for event-log/world verification.

## Files to Touch

- `crates/worldwake-core/src/epistemic.rs` (new; exact filename may vary if a cleaner non-conflicting name is chosen)
- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/world_txn.rs` (modify, if needed for schema-projected setters/imports)
- `crates/worldwake-core/src/lib.rs` (modify)

## Out of Scope

- `verify_belief` / `ask_witness` action payloads or handlers in `worldwake-sim` / `worldwake-systems`
- planner operators, candidate generation, ranking, or affordance enumeration in `worldwake-ai`
- automatic default installation of `VerificationDispositionProfile` on every created agent
- any change to `ViolationKind`, `ViolationMemory`, or `ViolationDispositionProfile`
- any backwards-compatibility alias such as a second `verification` namespace for the same concept

## Acceptance Criteria

### Tests That Must Pass

1. `VerificationSubject` round-trips through bincode for both variants.
2. `VerificationDispositionProfile` round-trips through bincode and satisfies component/value trait bounds.
3. `GoalKind::VerifyBelief` round-trips through bincode.
4. `GoalKey::from(GoalKind::VerifyBelief { ... })` extracts canonical fields correctly for both verification subject variants.
5. Distinct verification subjects at the same place produce distinct `GoalKey`s.
6. Inserting `VerificationDispositionProfile` on an agent succeeds through the authoritative schema surface.
7. Inserting `VerificationDispositionProfile` on a non-agent is rejected by the authoritative schema surface.
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Proactive verification identity is explicit in `GoalKind`; there is no hidden side-channel or alias type carrying the same fact.
2. `VerificationDispositionProfile` is registrable only on `EntityKind::Agent`.
3. New core types remain deterministic and serializable; no `HashMap`, `HashSet`, floats, or wall-clock state are introduced.
4. Existing structural `verification.rs` semantics remain untouched; epistemic verification does not hijack that namespace.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/epistemic.rs` (or chosen module file) — add round-trip/value-bound tests for `VerificationSubject` and `VerificationDispositionProfile` so the new core types are proven deterministic and serializable at their definition site.
2. `crates/worldwake-core/src/goal.rs` — add `VerifyBelief` round-trip and canonical `GoalKey` derivation tests so downstream AI/runtime code can rely on stable goal identity.
3. `crates/worldwake-core/src/world.rs` or `crates/worldwake-core/src/world_txn.rs` — add an agent-only schema enforcement test because that is the authoritative contract the new profile relies on.
4. Existing `world_txn` manifest projection tests remain relevant and should continue passing without bespoke changes unless the live harness requires a minimal update.

### Commands

1. `cargo test -p worldwake-core goal::tests::`
2. `cargo test -p worldwake-core world::tests::`
3. `cargo test -p worldwake-core`
4. `cargo clippy -p worldwake-core`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - Added `crates/worldwake-core/src/epistemic.rs` with `VerificationSubject` and `VerificationDispositionProfile`.
  - Added `GoalKind::VerifyBelief { subject, generation_tick }` plus canonical `GoalKey` derivation in `crates/worldwake-core/src/goal.rs`.
  - Registered `VerificationDispositionProfile` as an agent-only authoritative component and wired the manifest projections needed for `ComponentKind`, `ComponentValue`, typed tables, world APIs, and transaction setters.
  - Re-exported the new epistemic types from `worldwake-core`.
  - Added focused `worldwake-core` tests for serde/value bounds, `GoalKey` derivation, and agent-only schema enforcement.
  - Updated `worldwake-ai` exhaustive `GoalKind` matches so the new core variant compiles cleanly across the workspace without introducing compatibility shims or pretending the full S34 planner/operator implementation already exists.
- Deviations from original plan:
  - The ticket originally proposed `crates/worldwake-core/src/verification.rs`; that was corrected to a dedicated `epistemic.rs` module because `verification.rs` already serves event-log/world verification.
  - Workspace compilation required a small explicit `worldwake-ai` exhaustiveness follow-through beyond the original core-only scope.
  - Workspace lint surfaced a few unrelated existing `worldwake-ai` pedantic-clippy failures; those were fixed so final verification could pass.
- Verification results:
  - `cargo test -p worldwake-core` ✅
  - `cargo build --workspace` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
