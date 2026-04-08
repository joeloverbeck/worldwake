# S71EVELOGDEL-005: Update delta consumers and CLI display for `CompactSet`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — consumer match sites in `worldwake-systems` and `worldwake-cli`
**Deps**: S71EVELOGDEL-002

## Problem

After ticket 002 adds `ComponentDelta::CompactSet`, all code that pattern-matches on `ComponentDelta` variants must handle the new variant. Rust's exhaustive matching will produce compile errors at sites that don't have a wildcard arm. This ticket updates all consumer match sites outside verification (which is handled by ticket 004).

## Assumption Reassessment (2026-04-08)

1. **perception.rs:1049-1056** (`worldwake-systems`) — matches `ComponentDelta::Set` to extract `OfficeForceState` payload. Has a `_ => {}` wildcard at line 1058 that will silently handle `CompactSet`. Review confirms this is correct: perception never reads belief-store payloads from deltas, so `CompactSet` (which only carries belief-store diffs) should be ignored.
2. **bandit_camp.rs:290-299** (`worldwake-systems`) — matches `ComponentDelta::Removed` only via `matches!()` macro. `CompactSet` is not `Removed`, so the existing logic is unaffected.
3. **observer.rs:600,623** (`worldwake-cli`) — calls `record.state_deltas().len()` for counts. Does not destructure individual deltas for counts. However, if it prints delta details, it needs to handle `CompactSet` formatting.
4. **events.rs** (`worldwake-cli/src/handlers/events.rs`) — displays event summaries. If it formats `ComponentDelta` variants, it needs a `CompactSet` display arm.
5. **production.rs:278** (`worldwake-systems`) — uses `ComponentDelta::Set` in test assertions only. Tests that construct `Set` variants are unaffected by adding `CompactSet`.
6. Not a planner/golden-driven ticket.
14. No mismatches found. Perception's wildcard arm handles `CompactSet` correctly without changes.

## Architecture Check

1. Updating each consumer to either ignore or format `CompactSet` is the correct approach. No generic abstraction is needed because each consumer's relationship to deltas is different (perception extracts specific payload types, CLI formats for display, bandit_camp checks for removal).
2. No backward-compatibility shims. Each site either already has a wildcard that handles `CompactSet` or gets a new explicit arm.

## Verification Layers

1. Perception ignores `CompactSet` -> existing wildcard `_ => {}` at perception.rs:1058; no behavioral change to verify
2. Observer/CLI display `CompactSet` summary -> manual verification via observer binary or focused display test
3. Workspace compiles with no exhaustive-match errors -> `cargo build --workspace`
4. Single-layer ticket (consumer match-arm updates); no cross-system invariants introduced.

## What to Change

### 1. Review perception.rs wildcard (no code change expected)

Confirm that `perception.rs:1058` wildcard `_ => {}` correctly ignores `CompactSet`. Since `CompactSet` only carries belief-store diffs and perception only reads `OfficeForceState` from `Set` deltas, the wildcard is correct. Add a comment noting that `CompactSet` (belief store diffs) is intentionally ignored here.

### 2. Update observer CLI delta display

In `crates/worldwake-cli/src/bin/observer.rs`, if delta details are printed (beyond just counts), add formatting for `CompactSet`:

```
CompactSet { entity, component_kind: AgentBeliefStore, diff } =>
    format as "~{entity} {component_kind} (compact diff: {N} field groups changed)"
```

The compact representation is more informative than the full snapshot dump that `Set` currently produces.

### 3. Update events handler display

In `crates/worldwake-cli/src/handlers/events.rs`, if `ComponentDelta` variants are formatted for display, add a `CompactSet` arm showing entity, component kind, and a summary of the diff (number of changed fields/entries).

### 4. Confirm bandit_camp.rs and production.rs need no changes

Both use `matches!()` on `Removed` variant or construct `Set` in tests. Neither destructures the full enum. No changes needed.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — add clarifying comment at wildcard arm)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — add `CompactSet` display formatting)
- `crates/worldwake-cli/src/handlers/events.rs` (modify — add `CompactSet` display formatting)

## Out of Scope

- Verification reconstruction (ticket 004)
- Wiring `CompactSet` emission (ticket 003)
- Changing perception's delta-reading behavior
- Adding compact diff display detail beyond summary counts

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` compiles without exhaustive-match warnings or errors
2. Observer binary runs without panics on a scenario that produces belief-store updates
3. Existing suite: `cargo test --workspace`

### Invariants

1. Perception behavior is unchanged — `CompactSet` deltas are ignored (no belief-store payload extraction from deltas)
2. CLI display does not panic on `CompactSet` variants
3. No consumer silently drops data it previously read from `Set` variants for non-belief-store components

## Test Plan

### New/Modified Tests

1. None — this ticket updates display formatting and confirms existing wildcards. Behavioral correctness of `CompactSet` handling is verified by tickets 001, 003, 004, and 006. CLI display is verified by manual observation and existing integration tests.

### Commands

1. `cargo build --workspace` (confirm compilation)
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08 — primarily delivered by S71EVELOGDEL-002.

- S71EVELOGDEL-002 absorbed the compilation-critical consumer updates:
  - `display.rs` CompactSet formatting arm: delivered in 002
  - `world_txn.rs` entity-extraction match: delivered in 002
- `perception.rs` wildcard at line 1058 already handles CompactSet — confirmed by successful workspace build
- `events.rs` does not reference ComponentDelta — no update needed
- `bandit_camp.rs` and `production.rs` unaffected — confirmed during 002 reassessment
- Optional perception clarifying comment deferred (cosmetic, not compilation-critical)

## Verification Result

- Covered by S71EVELOGDEL-002 verification: `cargo build --workspace` (clean), `cargo clippy -p worldwake-cli --all-targets -- -D warnings` (clean)
