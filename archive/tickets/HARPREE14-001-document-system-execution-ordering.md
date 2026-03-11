# HARPREE14-001: Document system execution ordering contract

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None (Wave 1, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A04

## Problem

`SystemManifest::canonical()` and `SystemId::ALL` define the tick execution order (Needs -> Production -> Trade -> Combat -> Perception -> Politics), but the rationale for this specific ordering is undocumented. The ordering is load-bearing -- changing it could produce different emergent behavior.

The deeper maintenance risk is not just missing docs. `SystemId` metadata is currently duplicated across four places in `system_manifest.rs`: the enum declaration, `SystemId::ALL`, `SystemId::as_str()`, and `SystemId::ordinal()`. The current tests confirm today's ordering, but future enum growth still depends on multiple manual edits staying in sync.

## Assumption Reassessment (2026-03-11)

1. `SystemId::ALL` exists in `crates/worldwake-sim/src/system_manifest.rs` and is the canonical scheduler-order list -- confirmed.
2. `SystemManifest::canonical()` exists and delegates directly to `SystemId::ALL` -- confirmed.
3. No doc comments currently explain the ordering rationale on either `SystemId::ALL` or `SystemManifest::canonical()` -- confirmed.
4. `system_manifest.rs` already has invariant-oriented unit tests for canonical order, insertion-order preservation, display stability, bincode round-trips, and ordinal density -- confirmed.
5. The omission risk is narrower than the original ticket stated. Adding a new enum variant already forces updates to exhaustive `match` expressions in `as_str()` and `ordinal()`, so failure is not fully silent. The remaining problem is architectural: the same system list is still maintained in too many places.

## Architecture Check

1. This should remain a no-behavior-change hardening pass. The canonical tick order itself stays unchanged.
2. A separate count assertion is weaker than removing the duplication. The cleaner design is one authoritative definition that generates `SystemId`, `ALL`, and the stable string/ordinal metadata together.
3. No backwards-compatibility shims or alias paths. If this cleanup reveals any mismatched assumptions in tests, fix the tests to match the true invariant rather than preserving parallel sources of truth.

## What to Change

### 1. Add doc comments to `SystemId::ALL`

Explain the ordering rationale:
- Needs first: deprivation wounds must be assessed before production/trade decisions
- Production before Trade: new goods must exist before they can be traded
- Trade before Combat: economic actions resolve before violence
- Combat before Perception: combat outcomes are visible in the same tick
- Perception before Politics: agents perceive before social systems run

### 2. Collapse duplicated `SystemId` metadata into one authoritative definition

Replace the current parallel maintenance of:
- the enum variant list
- `SystemId::ALL`
- `SystemId::as_str()`
- `SystemId::ordinal()`

with a single definition in `system_manifest.rs` that emits those views together.

Preferred shape:
- keep the public API the same (`SystemId`, `ALL`, `as_str()`, `ordinal()`)
- use declaration order as the ordinal source of truth rather than a second hand-maintained ordinal match
- make omission from `ALL` impossible by construction, instead of trying to catch it later with another maintained assertion

### 3. Add doc comment to `SystemManifest::canonical()`

State that it is the authoritative tick order and must not be reordered without understanding the rationale documented on `ALL`.

## Files to Touch

- `crates/worldwake-sim/src/system_manifest.rs` (modify)

## Out of Scope

- Changing the actual execution order
- Adding new `SystemId` variants
- Modifying `SystemManifest` logic or dispatch
- Any behavioral changes
- Refactoring scheduler, RNG, or dispatch call sites outside the invariants this ticket directly touches

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim system_manifest` -- focused invariant coverage passes
2. `cargo test -p worldwake-sim` -- all existing tests pass
2. `cargo clippy --workspace` -- no new warnings
3. Golden e2e hashes identical (no behavioral change)

### Invariants

1. Tick execution order is unchanged
2. `SystemId::ALL`, `as_str()`, and `ordinal()` all derive from one authoritative system list
3. No public API changes
4. The canonical order remains explicit and reviewable in source

## Test Plan

### New/Modified Tests

1. Strengthen `crates/worldwake-sim/src/system_manifest.rs` inline tests if needed so the generated `SystemId` metadata remains dense, ordered, and stable for dispatch/scheduler consumers

### Commands

1. `cargo test -p worldwake-sim system_manifest`
2. `cargo build --workspace`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-11
- What actually changed:
  - Documented the canonical tick-order rationale directly on `SystemId::ALL`.
  - Documented `SystemManifest::canonical()` as the authoritative scheduler order.
  - Replaced the hand-maintained duplication between the enum declaration, `ALL`, `as_str()`, and `ordinal()` with one authoritative `SystemId` definition in `system_manifest.rs`.
  - Added a dense-ordinal invariant test so dispatch/scheduler consumers stay covered by focused unit tests.
- Deviations from original plan:
  - Did not add a standalone count assertion. A single-source-of-truth `SystemId` definition is stronger and cleaner than adding another maintained invariant beside the duplicated metadata.
  - Made two additional no-behavior-change lint cleanups in `crates/worldwake-sim/src/action_semantics.rs` and `crates/worldwake-ai/src/search.rs` because `cargo clippy --workspace` exposed existing pedantic failures unrelated to `system_manifest.rs`, and the ticket's finalization criteria required the workspace lint gate to pass.
- Verification results:
  - `cargo test -p worldwake-sim system_manifest`
  - `cargo test -p worldwake-sim action_semantics`
  - `cargo test -p worldwake-ai search`
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
