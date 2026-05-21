# S162BELVIESOU-004: Snapshot-through-view invariant test

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (test/guard only, plus same-family lint cleanup) —
`worldwake-ai`, `worldwake-sim`
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
5. Live planner contract checked: `docs/planner-contracts.md` says planner-visible
   fields are source-scoped and must preserve the belief-view source classification.
   The landed guard locks that construction boundary rather than adding per-field
   source types.
6. Verification fallout: the first full `cargo test -p worldwake-ai` run exposed two
   pre-existing positive political unit fixtures that still expected office metadata
   from direct entity observation after S162BELVIESOU-006. This ticket absorbed the
   narrow test-fixture repair by seeding `BelievedOfficeDataSnapshot` through
   `WorldTxn::project_believed_office_data`; no production fallback was reopened.
7. Broader golden fallout: the package-level command still fails in pending golden/
   scenario surfaces, mostly `scenarios::offices::*`. That is not part of the
   snapshot guard seam and remains owned by `tickets/S162BELVIESOU-005.md`.
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

## Verified Layers

1. No direct authoritative read in snapshot construction -> guard test (source-scan
   assertion over `planning_snapshot.rs` for a `world.`-read pattern, scoped to
   non-test code, OR an equivalent module-boundary mechanism). This is a single-layer
   ticket: the invariant is structural, so the proof surface is the guard itself; no
   runtime trace/event-log layer applies because no runtime behavior changes.

## Landed Changes

### 1. Add the snapshot-through-view guard

Added `planning_snapshot_construction_has_no_direct_world_reads`, a source-scan
test over the production portion of `planning_snapshot.rs`, and added the
construction-boundary comment tying snapshot construction to `RuntimeBeliefView`.

### 2. Repair same-family proof fallout

Repaired two stale positive political unit fixtures so they seed the lawful
believed-office snapshot carrier landed by S162BELVIESOU-006. This preserves the
post-S162 contract that whole `OfficeData` is planner-visible only through a
belief-backed snapshot.

### 3. Apply lint-only same-family cleanup

Changed `PerAgentBeliefView::loyalty_to` from an explicit `is_none` early return to
the equivalent `?` form required by the CI clippy gate.

## Landed Files

- `crates/worldwake-ai/src/planning_snapshot.rs` — invariant comment and source-scan
  guard test.
- `crates/worldwake-ai/src/agent_tick/tests.rs` — positive political fixtures now
  seed `BelievedOfficeDataSnapshot`.
- `crates/worldwake-sim/src/per_agent_belief_view.rs` — lint-only equivalent `?`
  rewrite in `loyalty_to`.

## Out of Scope

- Any change to snapshot field population or the belief view (covered by 001/002/003).
- Per-field `SnapshotFieldSource` provenance typing — explicitly rejected by the spec.

## Acceptance Result

### Tests

1. Passed: the guard asserts zero direct `world.` reads in production
   `planning_snapshot.rs`.
2. Passed: `cargo test -p worldwake-ai --lib` after repairing the same-family
   positive political unit fixtures.
3. Deferred to `tickets/S162BELVIESOU-005.md`: package-level `cargo test -p
   worldwake-ai` still fails in pending golden/scenario surfaces, primarily
   `scenarios::offices::*`, while the library owner seam is green.

### Invariants

1. `planning_snapshot.rs` construction reads only through the belief view; no direct
   authoritative `world.*` read in non-test code.

## Outcome

Completed on 2026-05-21.

- Added the snapshot-through-view guard test and construction-boundary comment.
- Repaired two same-family positive political unit fixtures exposed by the broad
  library run so they use the lawful believed-office snapshot carrier from
  S162BELVIESOU-006.
- Applied a lint-only same-family `loyalty_to` cleanup required by the all-target
  clippy gate.
- Left package-level golden/scenario fallout to `tickets/S162BELVIESOU-005.md`.

## Verification Result

- Passed `cargo test -p worldwake-ai planning_snapshot`
- Passed `cargo test -p worldwake-ai --lib`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Waived `cargo test -p worldwake-ai` as 004 completion proof because the current
  package-level failures are golden/scenario surfaces owned by
  `tickets/S162BELVIESOU-005.md`; the 004 library and guard surfaces pass.
