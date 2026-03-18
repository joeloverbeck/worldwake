# POLTRAC-001: Add Political System Traceability for Succession Decisions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` political tracing surface plus test/harness wiring
**Deps**: `crates/worldwake-systems/src/offices.rs`, `crates/worldwake-ai/tests/golden_harness/mod.rs`, `docs/FOUNDATIONS.md`, `specs/E16-offices-succession-factions.md`, `archive/specs/2026-03-17-action-execution-trace.md`

## Problem

Political succession is currently authoritative and inspectable only through low-level event-log deltas and final world state. That is enough for correctness, but not enough for fast debugging. There is no system-level trace explaining why succession did or did not fire on a given tick.

## Assumption Reassessment (2026-03-18)

1. `crates/worldwake-systems/src/offices.rs:13` implements `succession_system()` as authoritative system logic. It sets `vacancy_since`, vacates office holders, and installs replacements through system transactions.
2. There is no politics-specific trace sink analogous to decision traces (`worldwake-ai`) or action traces (`worldwake-sim`). Current debugging relies on event-log reconstruction after the fact.
3. The missing observability is especially visible for force-law succession, where no action lifecycle exists to inspect. The current event log shows what changed but not why the system decided "install", "reset timer", or "do nothing".
4. `docs/FOUNDATIONS.md` Principle 27 treats debuggability as a product feature, so this is aligned with repo-level architectural goals rather than test-only convenience.
5. The gap is not missing political correctness; it is missing explanatory traceability for authoritative political decisions.

## Architecture Check

1. The clean design is a parallel debug surface, not more event-log noise and not fake political actions. Succession remains a system mutation; tracing should explain it, not disguise it.
2. A political trace sink should record per-office per-tick evaluation facts such as:
   - living holder or none
   - vacancy clock before/after
   - succession law
   - contenders considered
   - eligibility filtering result
   - install/reset/no-op reason
3. This keeps politics symmetric with existing decision/action trace philosophy while preserving Principle 24 state-mediated system interaction.

## What to Change

### 1. Add political trace types and sink

Introduce trace structs and a sink for political system evaluation, scoped at least to office succession. The trace should capture enough data to answer:

- why an office was considered vacant or not
- why force/support succession did or did not install someone
- whether the timer blocked installation
- which contenders/votes were considered and filtered out

### 2. Thread trace sink through politics execution

Wire the sink into `succession_system()` and related helper paths in `crates/worldwake-systems/src/offices.rs` without changing authoritative behavior.

### 3. Expose test access

Extend the golden harness so tests can enable and inspect political tracing in the same way they currently can for action tracing.

### 4. Add focused and golden coverage

Add focused tests for the trace sink and at least one golden assertion showing that a succession scenario can explain its install/no-install result without manual event-log forensics.

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (modify)
- `crates/worldwake-systems/src/lib.rs` (modify if public exports are needed)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` or `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `docs/golden-e2e-testing.md` (modify if trace usage guidance needs extension)

## Out of Scope

- Reifying succession as an action
- Changing political behavior or succession semantics
- Cross-system unified timeline rendering

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`

### Invariants

1. Political tracing must be zero-cost when disabled and must not change authoritative outcomes.
2. Force-law succession remains a system mutation, not an AI action path or compatibility alias.
3. The trace must explain both positive and negative decisions, not only successful installations.

## Test Plan

### New/Modified Tests

1. Focused tests near `crates/worldwake-systems/src/offices.rs` — prove trace records vacancy activation, blocked timer, install, and no-op cases.
2. `crates/worldwake-ai/tests/golden_offices.rs` or `crates/worldwake-ai/tests/golden_emergent.rs` — assert that a real succession scenario exposes the expected political trace explanation.

### Commands

1. `cargo test -p worldwake-ai --test golden_offices`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`

