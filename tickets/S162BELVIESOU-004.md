# S162BELVIESOU-004: Snapshot-through-view invariant test

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (test/guard only) — `worldwake-ai`
**Deps**: Spec `specs/S162-belief-view-source-gate-hardening.md` (D6)

## Problem

The planning snapshot is lawful **by construction** only because
`planning_snapshot.rs` reads every entity/field through the belief view and never
reads `world.*` directly (verified 2026-05-21: `grep -c "world\." planning_snapshot.rs`
→ `0`). This is the architectural guarantee that makes per-field source typing
unnecessary (FND-14B requires the snapshot to preserve source classification, which
routing-through-view satisfies). Nothing currently guards against a future field
regressing to a direct world read. This ticket locks the invariant.

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Verified (2026-05-21): `crates/worldwake-ai/src/planning_snapshot.rs` has zero
   direct `world.` reads; all snapshot fields are sourced via `view.*` accessors
   (e.g., `view.entity_kind`, `view.direct_container`, `view.has_control`,
   `view.loyalty_to`, `view.effective_place`). `SnapshotControl.has_control` at
   `:211` is populated by `view.has_control(entity)` at `:1118`.
2. Spec D6 and the FND-14B/FND-27 alignment in
   `specs/S162-belief-view-source-gate-hardening.md` define the invariant: the
   snapshot may read only through the belief view, so lawfulness flows from the view
   (hardened by S162BELVIESOU-001/002/003), not from per-field source tags.
3. Shared boundary under audit: the construction surface of `planning_snapshot.rs`
   (the `build_snapshot_entity`/`build_*` functions and any helper they call). The
   guard asserts no authoritative `world.` read is reintroduced there.
4. Intended invariant: snapshot construction reads exclusively through the
   `RuntimeBeliefView`/`PerAgentBeliefView` surface; a direct `world.` read in
   `planning_snapshot.rs` is a defect.
13. Adjacent contradiction: this ticket does not depend on the gate tickets
    (001/002/003) — the invariant holds today regardless of whether the view is fully
    lawful. It is independent and may land in any order. (Its *value* compounds once
    the gates land, but the guard itself is orthogonal.)

## Architecture Check

1. A guard test (or equivalent compile-time/source-scan check) is the minimal,
   robust mechanism to lock the invariant: it fails loudly if a future field reads
   `world.*` directly, which is exactly the regression that would silently reintroduce
   a leak the view cannot mediate. This is cheaper and more honest than per-field
   `SnapshotFieldSource` typing (rejected in the spec) — it enforces the same FND-14B
   guarantee at the construction boundary.
2. No backwards-compatibility concern; this is a net-new guard with no production
   behavior change.

## Verification Layers

1. No direct authoritative read in snapshot construction -> guard test (source-scan
   assertion over `planning_snapshot.rs` for a `world.`-read pattern, scoped to
   non-test code, OR an equivalent module-boundary mechanism). This is a single-layer
   ticket: the invariant is structural, so the proof surface is the guard itself; no
   runtime trace/event-log layer applies because no runtime behavior changes.

## What to Change

### 1. Add the snapshot-through-view guard

Add a test in `worldwake-ai` that fails if `planning_snapshot.rs` (excluding its
`#[cfg(test)]` blocks) contains a direct authoritative `world.` read. The exact
mechanism is an implementation detail — a source-scan test that reads the file and
asserts zero matches of the `world.`-read pattern outside test code is the simplest;
if a more structural guard (e.g., constraining construction to a `&dyn RuntimeBeliefView`
parameter with no `&World` in scope) is cleaner, prefer it. Document the invariant in
a comment at the construction entry point so the intent is discoverable.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — invariant doc comment + `#[cfg(test)]` guard test) or a sibling test module under `crates/worldwake-ai/` if a source-scan test fits better there.

## Out of Scope

- Any change to snapshot field population or the belief view (covered by 001/002/003).
- Per-field `SnapshotFieldSource` provenance typing — explicitly rejected by the spec.

## Acceptance Criteria

### Tests That Must Pass

1. New: the guard passes on the current `planning_snapshot.rs` (0 direct `world.` reads).
2. New (negative confirmation, optional): the guard would fail if a direct `world.` read were present (demonstrated via a scoped fixture or a documented manual check).
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `planning_snapshot.rs` construction reads only through the belief view; no direct authoritative `world.*` read in non-test code.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` (`#[cfg(test)]`) — snapshot-through-view guard; rationale: locks the lawful-by-construction property that replaces per-field source typing.

### Commands

1. `cargo test -p worldwake-ai planning_snapshot`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `./scripts/verify.sh` (before PR)
