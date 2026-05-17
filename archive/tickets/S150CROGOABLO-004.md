# S150CROGOABLO-004: Observer Section 3b typed-scope rendering

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — tooling-only (observer binary)
**Deps**: archive/tickets/S150CROGOABLO-002.md

## Problem

Ticket 002 added the `scope: BlockerScope` field to `BlockerRecordedPayload` and preserved the existing observer Section 3b rendering against the new payload shape. The current rendering uses generic debug formatting that prints `BlockerScope::Exact(BlockerKey { goal_key: ..., place: Some(...), ... })` for every variant — readable for `Exact` but verbose and structurally indistinct between the three scope variants. S150 D7 specifies a distinct per-variant rendering format that surfaces the cross-goal scope information at a glance (the FND-29 debuggability motivation: "why is this blocker here?" is answerable visually from Section 3b without parsing nested debug output).

## Assumption Reassessment (2026-05-17)

1. Observer Section 3b "Decision History" lives at `crates/worldwake-cli/src/bin/observer.rs:872`. The `BlockerRecordedPayload` rendering is consumed there through the existing event-log walking code (after ticket 002, the payload carries `scope: BlockerScope`). The target rendering is a one-line-per-event format consistent with the surrounding decision-history table.
2. Spec source: `archive/specs/S150-cross-goal-blocker-scoping.md` D7's rendering example:
   ```
   Blocker: RouteSegment(Thornwall ↔ Ashford) — DangerTooHigh — observed tick 1247, expires 1487
   Blocker: Counterparty(Merchant#42) — NoWillingCounterparty — observed tick 1310, expires 1670
   Blocker: Exact(Sleep at Inn) — WorkstationBusy — observed tick 1422, expires 1442
   ```
3. Shared abstraction boundary: the rendering reads `BlockerScope` (from `worldwake-core`) and produces a `String`/`writeln!` output that is consumed by the observer's text-format rendering. The boundary is read-only — observer is a downstream consumer per the "Read-Only Tooling Consumer" pattern; no engine state is mutated.
4. Place- and entity-name resolution: `RouteSegment(from, to)` and `Counterparty(other)` render with human-readable names via the existing `worldwake_cli::display::entity_display_name(world, id) -> String` helper (per the Read-Only Tooling Consumer pattern's name-accessor convention). `Exact(BlockerKey)` renders the `goal_key`'s `GoalKind` debug summary plus the optional place/target/action context.

## Architecture Check

1. **Tooling-only**: No engine state mutation, no new types in `worldwake-core` / `worldwake-sim` / `worldwake-ai`. The rendering is a presentation refinement of an existing observer section.
2. **Per-variant clarity**: Distinct format per variant makes cross-goal scope visible at a glance — a reviewer skimming a long decision-history table can spot the RouteSegment and Counterparty entries without parsing nested debug output. Satisfies FND-29's debuggability requirement at the observer-output level.
3. **Reuses existing name accessors**: `entity_display_name` is the canonical helper per the Read-Only Tooling Consumer pattern; no parallel name-resolution path is introduced.

## Verification Layers

1. Rendering format correctness — focused observer test (in `crates/worldwake-cli/src/bin/observer.rs` `#[cfg(test)]` if such block exists, otherwise a new test module) that constructs a `BlockerRecordedPayload` for each scope variant, runs the rendering, and asserts the formatted output matches the per-variant template.
2. Single-layer ticket — additional layer mapping (decision trace, action trace, event-log delta, authoritative world state) is not applicable: this ticket is presentation-only over already-recorded event-log content.

## What to Change

### 1. Add per-variant rendering for `BlockerScope` in observer Section 3b

In `crates/worldwake-cli/src/bin/observer.rs:872` Section 3b rendering, replace the existing generic debug formatting of `BlockerRecordedPayload.scope` with a match:

```rust
let scope_str = match payload.scope {
    BlockerScope::Exact(bk) => format!(
        "Exact({} at {})",
        format_goal_kind(bk.goal_key.kind()),
        bk.place.map(|p| entity_display_name(world, p))
            .unwrap_or_else(|| "—".to_string()),
    ),
    BlockerScope::RouteSegment(seg) => format!(
        "RouteSegment({} ↔ {})",
        entity_display_name(world, seg.from),
        entity_display_name(world, seg.to),
    ),
    BlockerScope::Counterparty(other) => format!(
        "Counterparty({})",
        entity_display_name(world, other),
    ),
};
writeln!(out, "Blocker: {scope_str} — {fact:?} — observed tick {observed}, expires {expires}", ...).unwrap();
```

(Adapt to the exact surrounding API and existing column conventions — the structure above is illustrative; the implementer matches the existing format for the same-row decision-history line.)

### 2. Add a focused rendering test

If `observer.rs` already carries a `#[cfg(test)]` block for rendering helpers, extend it with a per-variant assertion. Otherwise add a small test module that constructs three `BlockerRecordedPayload` instances (one per variant) and snapshot-tests their rendered output against the per-variant template.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- **`scope: BlockerScope` field on `BlockerRecordedPayload`** — landed in ticket 002.
- **Engine-side rendering changes** — none; this is observer-only.
- **Section 13 (S144 diagnostics) per-scope blocker histogram rendering** — that's ticket 005's `BlockerScopeVariantId` histogram in `BeliefMetrics`, rendered through the existing Section 13 framework.
- **Other observer sections** — Section 7 (End-State Inventory & Resources) and other sections are unchanged; blocker rendering only lives in Section 3b.

## Acceptance Criteria

### Tests That Must Pass

1. Per-variant rendering test (new): each of `Exact`, `RouteSegment`, `Counterparty` renders to the expected per-variant format string.
2. Existing observer regression tests pass unchanged.
3. Workspace: `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. Every `BlockerRecorded` event in Section 3b renders to exactly one line with the per-variant scope prefix.
2. `RouteSegment(A ↔ B)` and `RouteSegment(B ↔ A)` render identically (canonical-ordering preservation from `RouteSegment::new`).
3. Entity-name resolution uses `entity_display_name` (no parallel name-resolution path introduced).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (new focused test or extension of existing rendering test) — per-variant assertion.

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh` for the full pre-PR gate.

## Outcome

Completed: 2026-05-17

What changed:

- Added observer-side `BlockerScope` summary rendering for `Exact`, `RouteSegment`, and `Counterparty` blocker scopes.
- Threaded the live `World` into Section 3b decision-history summary rendering so blocker scope labels use the existing `entity_display_name` and `format_goal_kind` helpers.
- Preserved single-line decision-history table rows while replacing generic `Debug` scope output for `BlockerRecorded` events.
- Added a focused observer test covering all three scope variants and canonical route ordering.

Deviation from original plan:

- The route separator is rendered as ASCII `<->` instead of the illustrative Unicode arrow in the ticket text, matching the repository's default ASCII editing convention.
- Exact scope rendering includes optional target and action context when present, e.g. `Exact(Sleep at Ashford target=Merchant Vara action=adef6)`.

Verification:

- `cargo fmt --all`
- `cargo test -p worldwake-cli --bin observer`
- `cargo clippy --workspace --all-targets -- -D warnings`
