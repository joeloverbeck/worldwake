# BELASPCOV-001: BelievedEntityState ↔ EntityBeliefAspect claim-aspect coverage audit

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — audit-only; no claim-aspect gaps found
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md, archive/tickets/S129CIREM-003-tell-session-vs-self-care.md

## Problem

S129CIREM-003 surfaced a gap in the belief layer: `WashBasinState`
existed as a field on `BelievedEntityState`
(`crates/worldwake-core/src/belief.rs:1585`) and was populated by
direct observation, but had no backing `EntityBeliefAspect` claim. The
direct observation carried a stale claim through other aspects
(`Location`, `WorkstationPresent`); when that claim's confidence
decayed below `claim_confidence_threshold`, the summary lost the
field, even though the entity itself was still retained. Listener
Bea's chronic 1197-tick critical-dirtiness equilibrium was a direct
consequence — wash-basin state vanished from her belief summary
without the entity vanishing. CIREM-003 fixed `WashBasinState`
specifically by adding `EntityBeliefAspect::WashBasinState`,
`ClaimValue::WashBasinState`, projecting the claim back into
`derive_entity_summary`, and bumping `SAVE_FORMAT_VERSION` to 58.

The architectural concern is whether other `BelievedEntityState`
fields have the same gap. Without an audit, the next state-rich field
added to the summary will silently inherit the same chronic-stall
failure mode. This ticket performs the audit and files secondary
tickets per discovered gap.

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Live `BelievedEntityState` fields** (verified at
   `crates/worldwake-core/src/belief.rs:1585`):
   `believed_kind`, `last_known_place`, `last_known_inventory`,
   `workstation_tag`, `resource_source`, `wash_basin_state`, `alive`,
   `wounds`, `last_known_courage`, `believed_activity`,
   `believed_artifact`, `believed_contention`, `believed_evidence`,
   plus the `presentation_ticks` / `presentation_tick_count` /
   `source` triple (the last is observation metadata, not belief
   content).
2. **Live `EntityBeliefAspect` variants** (verified at
   `crates/worldwake-core/src/entity_belief_claim.rs:17`):
   `Location`, `Inventory(CommodityKind)`, `Alive`, `Wounded`,
   `Activity`, `WorkstationPresent`,
   `ResourceAvailable(CommodityKind)`, `ContentionState`,
   `WashBasinState`, `ArtifactState`, `Courage`, `Evidence`.
3. **Cross-reference**: `derive_entity_summary`
   (`crates/worldwake-core/src/belief.rs:2241+`) projects each aspect
   back into the summary. The audit must verify the projection is
   complete (every field is hydrated from at least one aspect) and
   the projection is sound (every aspect maps unambiguously to a
   summary slot).
4. **Initial scan candidates** (likely-but-unverified gaps to confirm):
   - `believed_kind: Option<EntityKind>` — does any
     `EntityBeliefAspect` variant carry the kind? `WorkstationPresent`
     carries `WorkstationTag`, not `EntityKind`. The claim-projection
     code may set `believed_kind` only via the snapshot pathway, not
     via claim hydration. Confirm whether this is a gap or whether
     `believed_kind` is intentionally derived from another claim.
   - `resource_source: Option<ResourceSource>` — `ResourceAvailable(CommodityKind)`
     looks like a per-commodity boolean rather than a full
     `ResourceSource`. Confirm whether the summary-side `resource_source`
     is fully recoverable from claim state.
5. **Mismatch + correction**: This ticket's deliverable is the audit
   report and any secondary tickets, not engine changes. If the audit
   finds gaps, each gap becomes its own implementation ticket
   (filed as `BELASPCOV-002`, `BELASPCOV-003`, ...). Adopting "fix
   everything found" as scope here would balloon and risk save-format
   changes without per-gap evidence.
6. **Coverage gap (precision-rules §3)**: This ticket sits inside
   the existing belief layer; the verification is a focused unit test
   per identified gap, plus any save-format/serde tests already living
   in `entity_belief_claim_roundtrips_through_bincode`-style suites.
   No new golden coverage is required for the audit itself.
7. **Information-path refactor discipline**: The audit is **not** an
   information-path refactor. It does not introduce new transport
   paths; it verifies existing ones are complete. Any per-gap
   secondary ticket that *does* add a claim aspect must declare that
   claim hydration is the canonical path and that the snapshot
   pathway is preserved as the only direct mutator (matching CIREM-003's
   pattern).
8. **Audit result (2026-05-01)**: The live audit found no mutable
   `BelievedEntityState` field that follows the pre-CIREM-003
   wash-basin failure mode. `resource_source` is fully claim-backed by
   `EntityBeliefAspect::ResourceAvailable(CommodityKind)` plus
   `ClaimValue::ResourceSource`. `believed_kind` is not claim-backed,
   but is intentionally preserved as stable entity identity /
   presentation metadata by `record_entity_snapshot_claims` and
   `preserve_believed_kind`; it is not a mutable claim-aspect gap.
   Therefore no `BELASPCOV-002` follow-up ticket was filed.

## Architecture Check

1. **No backwards-compatibility shims**: the audit is a read-only
   pass. Per-gap secondary tickets that add claim aspects must
   eliminate any direct field-set path that bypasses claim recording,
   not preserve it as a fallback.
2. **Save-format discipline**: every secondary ticket that adds an
   `EntityBeliefAspect` variant must bump `SAVE_FORMAT_VERSION` and
   add a roundtrip test, matching CIREM-003's pattern at
   `crates/worldwake-sim/src/save_load.rs`.
3. **Audit-only scope**: this ticket does not modify engine code.
   Findings drive secondary tickets only when a `gap` row exists; the
   completed audit found none.

## Verification Layers

1. **Field-by-field coverage** -> a written audit table mapping each
   `BelievedEntityState` field to the set of `EntityBeliefAspect`
   variants that hydrate it (via `derive_entity_summary`), plus the
   verdict per field: `covered` / `gap` / `intentionally-derived`.
2. **Per-gap implementation contract** -> each `gap` row produces a
   secondary ticket naming the missing aspect, the claim semantics,
   the projection in `derive_entity_summary`, and the save-format
   bump.
3. **Single-layer ticket**: this is an audit, so verification is the
   audit document plus any focused unit tests created under the
   secondary tickets.

## What to Change

### 1. Read every field of `BelievedEntityState`

Walk `crates/worldwake-core/src/belief.rs:1585` and list each
non-metadata field.

### 2. Read every variant of `EntityBeliefAspect` and `ClaimValue`

Walk `crates/worldwake-core/src/entity_belief_claim.rs:17,33`. For
each, identify which `BelievedEntityState` field it projects into
via `derive_entity_summary`
(`crates/worldwake-core/src/belief.rs:2241+`).

### 3. Build the coverage table

For each `BelievedEntityState` field, name:
- The `EntityBeliefAspect` variant(s) that hydrate it.
- Whether direct observation populates the field via the claim path
  (`record_entity_snapshot_claims`) or only via direct field-set in
  the snapshot.
- Whether the field survives stale-claim decay
  (`prune_decayed_beliefs`) once direct observation stops, or whether
  it follows the pre-CIREM-003 wash-basin pattern (vanishes on
  decay).

### 4. Write findings

Write the audit table to a new doc
`docs/audits/2026-05-01-believed-entity-state-claim-coverage.md`.
Findings are: `covered` (no action), `intentionally-derived` (note in
audit), or `gap` (file secondary ticket).

### 5. File secondary tickets per gap

For each `gap` row, file `BELASPCOV-002`, `BELASPCOV-003`, etc.
under `tickets/`, each scoped to one missing aspect. The audit doc
links to each secondary ticket.

## Files to Touch

- `docs/audits/2026-05-01-believed-entity-state-claim-coverage.md` (new)
- No `tickets/BELASPCOV-NNN-*.md` follow-up was created because the
  audit found no `gap` verdicts.

No engine files are touched in this ticket.

## Out of Scope

- Implementing the per-gap fixes. Each is its own ticket.
- Auditing `AgentBeliefStore` accessors more broadly (`believed_kind`
  population paths, perception throttling) — those are S77 / S101 /
  S105 territory.
- Auditing `BelievedActivity`, `BelievedArtifactState`,
  `BelievedContentionState`, `BelievedEvidenceState` internal fields
  — they are nested under their respective claim aspects already.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — unchanged, since this ticket is audit-only.
2. The audit doc exists at the path above and lists every
   `BelievedEntityState` field with a verdict.

### Invariants

1. **Claim coverage is the canonical path**: the audit enumerates
   which `BelievedEntityState` fields persist *only* via claim
   hydration (covered), which persist via both claim hydration and
   direct snapshot (covered), and which persist only via direct
   snapshot (gap). The latter set is the secondary-ticket queue.
2. **No silent field decay**: any field that the summary exposes for
   planner reads must survive stale-claim decay or be flagged as
   gap.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is the audit doc
   and the existing claim-carrier roundtrip test. No per-gap
   secondary tickets were created.

### Commands

1. `grep -n 'EntityBeliefAspect::' crates/worldwake-core/src/belief.rs | sort -u`
   — sanity check that the audit walked every projection.
2. `cargo test -p worldwake-core entity_belief_claim_roundtrips_through_bincode -- --list`
   — confirm the live bincode roundtrip selector exists. The drafted
   `--test entity_belief_claim_roundtrips_through_bincode` form is
   stale because the proof is a `worldwake-core` library unit test, not
   an integration-test binary.
3. `./scripts/verify.sh` — only required if any per-gap secondary
   ticket lands in the same PR; not required for the audit doc alone.

## Closeout Evidence (2026-05-01)

Passed:

1. `grep -n 'EntityBeliefAspect::' crates/worldwake-core/src/belief.rs | sort -u`
2. `cargo test -p worldwake-core entity_belief_claim_roundtrips_through_bincode -- --list`
3. `cargo test -p worldwake-core entity_belief_claim_roundtrips_through_bincode`
4. `cargo test --workspace`

Not run:

1. `./scripts/verify.sh` — not required by the ticket unless a per-gap
   secondary implementation ticket lands in the same PR; no per-gap
   ticket was created.

## Outcome

Completed on 2026-05-01.

- Added `docs/audits/2026-05-01-believed-entity-state-claim-coverage.md`.
- Audited every non-metadata `BelievedEntityState` field against
  `EntityBeliefAspect`, `ClaimValue`, `entity_claims_for_snapshot`,
  `derive_entity_summary`, and stale-claim pruning behavior.
- Found no missing claim-aspect coverage gaps for mutable belief
  content.
- Classified `believed_kind` as intentionally preserved identity /
  presentation metadata, not a claim aspect.
- Created no secondary `BELASPCOV-002` ticket because there were no
  `gap` rows.
