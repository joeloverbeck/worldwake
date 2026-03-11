# E13DECARC-001: Fix worldwake-ai Cargo.toml dependency wiring

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E09, E10, E11, E12 (all completed)

## Problem

`worldwake-ai/Cargo.toml` is missing the `worldwake-sim` dependency that the E13 spec and crate dependency graph require. Without that dependency, the crate cannot lawfully import the decision-facing simulation types E13 is built around.

The earlier version of this ticket also proposed pre-creating a full `worldwake-ai` module tree. That assumption does not hold up against the rest of the E13 ticket stack and would push the architecture in the wrong direction before the later tickets define concrete APIs.

## Assumption Reassessment (2026-03-11)

1. `worldwake-ai/Cargo.toml` currently depends on `worldwake-core` and `worldwake-systems` only — confirmed.
2. `worldwake-sim` exports `BeliefView`, `Affordance`, `ActionDefRegistry`, `ActionDefId`, `ActionPayload`, `InputEvent`, `InputKind`, `ReplanNeeded`, `OmniscientBeliefView` — confirmed.
3. `worldwake-ai/src/lib.rs` contains only a doc comment, no module declarations — confirmed.
4. Later E13 tickets do **not** consistently place their work in `worldwake-ai` modules matching the old scaffold list:
   - `E13DECARC-002` defines `UtilityProfile` in `worldwake-core`
   - `E13DECARC-003` defines `BlockedIntentMemory` in `worldwake-core`
5. `cargo test -p worldwake-ai` already passes with an effectively empty crate, so stub files are not needed to regain a green baseline.

## Architecture Check

1. Adding `worldwake-sim` follows the documented crate dependency graph: `worldwake-ai` depends on `worldwake-core`, `worldwake-sim`, `worldwake-systems`.
2. Pre-scaffolding empty modules is not beneficial here:
   - it would create wrong ownership for at least `utility_profile` and `blocked_intent`
   - it would lock in file layout before the concrete types and boundaries are implemented
   - it adds dead files with no behavior, tests, or stable API surface
3. The clean, extensible move is to add the missing dependency now and let later tickets create modules when their real contents are introduced.

## What to Change

### 1. Add `worldwake-sim` dependency to `worldwake-ai/Cargo.toml`

Add `worldwake-sim = { path = "../worldwake-sim" }` to `[dependencies]`.

### 2. Add a crate-level smoke test instead of speculative scaffolding

Keep `lib.rs` minimal. Add a small test that proves `worldwake-ai` can compile against the `worldwake-sim` types E13 depends on. This gives the ticket a concrete verification target without freezing an inaccurate module structure.

## Files to Touch

- `crates/worldwake-ai/Cargo.toml` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify — add smoke test only)

## Out of Scope

- Any actual type definitions or implementations — those belong in subsequent tickets
- Pre-creating empty AI modules or files
- Changes to any other crate's Cargo.toml
- Changes to `worldwake-core` or `worldwake-sim`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` compiles without errors
2. `cargo test -p worldwake-ai` passes, including the new smoke test
3. `cargo clippy --workspace` passes
4. Existing suite: `cargo test --workspace`

### Invariants

1. `worldwake-ai` depends on exactly `worldwake-core`, `worldwake-sim`, `worldwake-systems`
2. No circular dependencies introduced
3. E13 does not predeclare module ownership that contradicts later tickets
4. All existing tests in other crates remain green

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/lib.rs` — add a smoke test proving the crate can reference the `worldwake-sim` types called out by the E13 spec.

### Commands

1. `cargo build -p worldwake-ai`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - added `worldwake-sim` to `crates/worldwake-ai/Cargo.toml`
  - added a `worldwake-ai` smoke test that proves the crate can reference the E13-facing `worldwake-sim` types
  - corrected the ticket scope to remove speculative module scaffolding
- Deviations from original plan:
  - did **not** create empty `worldwake-ai` module files
  - removed the scaffold requirement because it contradicted later ticket ownership (`UtilityProfile` and `BlockedIntentMemory` belong in `worldwake-core`) and would have added dead files with unstable boundaries
- Verification results:
  - `cargo build -p worldwake-ai` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo build --workspace` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
