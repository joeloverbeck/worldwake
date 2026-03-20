# S09INDACTREEVA-006: Remove brittle positional `CombatProfile::new()` construction

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — shared `CombatProfile` construction API changes; positional constructor removed after migration
**Deps**: archive/tickets/completed/S09INDACTREEVA-001.md

## Problem

`CombatProfile::new()` is now an 11-argument positional constructor. The S09 field addition made the weakness explicit: adding one profile field forced mechanical edits across core, sim, systems, AI, golden harnesses, and CLI test fixtures. This is brittle, hard to review, and scales poorly as the authoritative combat model evolves.

`CombatProfile` is plain authoritative data with fully public fields. The current positional constructor hides field names at the call site, makes accidental argument swaps plausible, and turns every schema evolution into low-signal churn. That is the opposite of a clean, robust, extensible authority model.

## Assumption Reassessment (2026-03-20)

1. `CombatProfile` currently lives in `crates/worldwake-core/src/combat.rs` with all fields `pub`. The type is plain data, derives serde traits, and does not appear to require invariant enforcement beyond field types such as `Permille` and `NonZeroU32`.
2. `CombatProfile::new()` is currently a `const fn` positional constructor with 11 parameters in `crates/worldwake-core/src/combat.rs`. The most recent S09 field addition required updating 37 live call sites across 19 files.
3. Current `CombatProfile::new()` call sites are fixture-heavy and test-heavy. They appear in focused unit tests, integration tests, golden harness defaults, and CLI scenario tests. No current production runtime behavior depends on the constructor shape itself; the constructor is primarily a convenience API for local fixture creation.
4. The remaining active S09 tickets (`S09INDACTREEVA-002` through `S09INDACTREEVA-005`) do not remove this weakness. Ticket 002 adds a new duration variant, 003 switches defend to that variant, 004 removes `Indefinite`, and 005 adds golden coverage. None of them replace the positional construction pattern or reduce future profile-schema churn.
5. This is not an AI-regression ticket. It affects AI tests and harnesses only because they construct `CombatProfile` fixtures; no candidate-generation, ranking, plan-search, or action-lifecycle invariant is being changed.
6. No ordering contract is involved. The intended change is construction clarity only, not action lifecycle ordering, event ordering, or world-state ordering.
7. No heuristic, stale-request, political, or `ControlSource` path is involved.
8. Foundation alignment check:
   - Principle 3: concrete authoritative state should be explicit and inspectable. Named field construction is more legible than positional tuples of combat numbers.
   - Principle 20: agent diversity is encoded through concrete per-agent parameters. A construction API that obscures which parameter is which works against safe diversity expansion.
   - Principle 26: no backward-compatibility layers in live authority paths. If the positional constructor is architecturally wrong, the clean fix is to remove or replace it directly rather than keep a deprecated alias indefinitely.
9. Mismatch corrected: there is currently no ticket that owns this constructor/API cleanup. This ticket is needed if the repo wants to address the weakness instead of carrying it forward.

## Architecture Check

1. The cleanest direction is to stop expressing `CombatProfile` creation as a long positional argument list. Use named struct literals at call sites that specify explicit values, and use narrow local helper fixtures only where repeated defaults materially reduce noise.
2. If a convenience API is still warranted after migration, it should be named-field oriented rather than positional, for example a small builder or explicit preset helpers. The goal is not more abstraction; the goal is preserving field names at the call site.
3. Removing the positional constructor after migration is cleaner than keeping it around as a compatibility shim. The repo explicitly prefers fixing breakage over layering aliases.
4. This cleanup should stay narrowly focused on `CombatProfile` construction ergonomics. Do not opportunistically refactor unrelated profile types unless the same ticket is explicitly expanded.

## Verification Layers

1. `CombatProfile` construction remains explicit and correct after API cleanup -> focused core tests in `crates/worldwake-core/src/combat.rs`
2. Cross-crate fixtures still compile and preserve current behavior after constructor migration -> targeted package tests for `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, and `worldwake-cli`
3. Serialized scenario/test fixtures remain aligned with the authoritative schema -> focused CLI scenario tests plus full-workspace regression
4. No action/planner/runtime behavior changed -> existing unit, integration, and golden suites should pass without behavioral assertion rewrites
5. This is a construction-surface ticket, so decision trace and action trace assertions are not the primary proof surface

## What to Change

### 1. Replace positional construction with named construction

In `crates/worldwake-core/src/combat.rs` and all current call sites:
- Migrate `CombatProfile::new(...)` usage to explicit named-field construction, or to a named-field builder/preset API if one is introduced by this ticket
- Preserve field names at the construction site so future schema additions are mechanically obvious and reviewable

### 2. Remove `CombatProfile::new()` if it is no longer justified

If all call sites can migrate cleanly to struct literals or a clearer API:
- Remove `CombatProfile::new()` entirely
- Update focused core tests accordingly

If a helper remains necessary:
- It must not be another long positional constructor
- It must preserve field identity explicitly at the call site

### 3. Normalize repeated fixture creation only where it materially helps

In files with repeated near-identical test profiles:
- Prefer small local helpers with named overrides, or one explicit base struct plus per-test modifications
- Do not create a global abstraction just to deduplicate a handful of lines

### 4. Reassess S09-related ticket text if constructor references become stale

If any active S09 tickets still describe `CombatProfile::new()` as part of future work:
- Update those tickets so their assumptions and file-touch descriptions match the new construction architecture
- Do not change their behavior scope; only correct the stale construction assumptions

## Files to Touch

- `crates/worldwake-core/src/combat.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/wounds.rs` (modify)
- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/action_validation.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-systems/src/combat.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `crates/worldwake-systems/tests/e12_combat_integration.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify, only if fixture/schema assumptions need adjustment)
- `tickets/S09INDACTREEVA-002.md` (modify if constructor assumptions become stale)
- `tickets/S09INDACTREEVA-003.md` (modify if constructor assumptions become stale)
- `tickets/S09INDACTREEVA-004.md` (modify if constructor assumptions become stale)
- `tickets/S09INDACTREEVA-005.md` (modify if constructor assumptions become stale)

## Out of Scope

- Changing combat balance values or defend behavior
- Changing `DurationExpr`, `ActionDuration`, planner behavior, or scheduler behavior
- Generalizing the cleanup to every profile type in the codebase
- Introducing compatibility aliases or deprecated wrappers for the old positional constructor

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo test -p worldwake-ai`
5. `cargo test -p worldwake-cli`
6. `cargo test --workspace`
7. `cargo clippy --workspace`

### Invariants

1. `CombatProfile` construction no longer relies on an opaque long positional argument list
2. Field identity is visible at construction sites, or visible through an explicit named builder API
3. No backward-compatibility shim remains for the old positional constructor if it is architecturally replaced
4. Existing runtime behavior and test semantics remain unchanged; this is a construction/API cleanup only

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/combat.rs` — adjust focused construction/roundtrip tests to match the new named construction surface; rationale: core proves the authoritative type is still explicit and serializable.
2. Existing fixture-backed tests across core/sim/systems/ai/cli — migrate construction syntax only; rationale: compile-time coverage should catch stale construction assumptions without changing behavior assertions.
3. Active S09 tickets in `tickets/` — update only if their assumptions mention the removed constructor; rationale: ticket fidelity must match the actual architecture.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo test -p worldwake-ai`
5. `cargo test -p worldwake-cli`
6. `cargo test --workspace`
7. `cargo clippy --workspace`
