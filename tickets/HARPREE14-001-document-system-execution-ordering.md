# HARPREE14-001: Document system execution ordering contract

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None (Wave 1, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A04

## Problem

`SystemManifest::canonical()` and `SystemId::ALL` define the tick execution order (Needs -> Production -> Trade -> Combat -> Perception -> Politics), but the rationale for this specific ordering is undocumented. The ordering is load-bearing -- changing it could produce different emergent behavior. Additionally, if a new `SystemId` variant is added without updating `ALL`, the omission is silent.

## Assumption Reassessment (2026-03-11)

1. `SystemId::ALL` exists at line 16 of `system_manifest.rs` -- confirmed, array of 6 variants
2. `SystemManifest::canonical()` exists at line 75 -- confirmed
3. No doc comments currently explain the ordering rationale -- confirmed

## Architecture Check

1. Pure documentation + compile-time safety. No behavioral change, no API change.
2. No backwards-compatibility shims. A `const` assertion prevents future silent omission.

## What to Change

### 1. Add doc comments to `SystemId::ALL`

Explain the ordering rationale:
- Needs first: deprivation wounds must be assessed before production/trade decisions
- Production before Trade: new goods must exist before they can be traded
- Trade before Combat: economic actions resolve before violence
- Combat before Perception: combat outcomes are visible in the same tick
- Perception before Politics: agents perceive before social systems run

### 2. Add compile-time assertion

Add a `const` assertion (or `static_assert!` equivalent) that `SystemId::ALL.len()` matches the number of enum variants, preventing silent omission when new systems are added. In Rust, this can be done with a const fn that panics at compile time if the count mismatches.

### 3. Add doc comment to `SystemManifest::canonical()`

State that it is the authoritative tick order and must not be reordered without understanding the rationale documented on `ALL`.

## Files to Touch

- `crates/worldwake-sim/src/system_manifest.rs` (modify)

## Out of Scope

- Changing the actual execution order
- Adding new `SystemId` variants
- Modifying `SystemManifest` logic or dispatch
- Any behavioral changes

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim` -- all existing tests pass unchanged
2. `cargo clippy --workspace` -- no new warnings
3. Golden e2e hashes identical (no behavioral change)

### Invariants

1. Tick execution order is unchanged
2. `SystemId::ALL` count matches enum variant count at compile time
3. No public API changes

## Test Plan

### New/Modified Tests

1. No new runtime tests needed -- the compile-time assertion IS the test

### Commands

1. `cargo build --workspace` (validates compile-time assertion)
2. `cargo test --workspace`
3. `cargo clippy --workspace`
