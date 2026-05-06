# S140ARTLIFAXE-008: Source-event credibility refutation for artifact lifecycle

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — S63-style case/evidence source-carrier integration into `artifact_lifecycle_system` credibility transitions
**Deps**: archive/tickets/S140ARTLIFAXE-007.md, specs/S140-artifact-lifecycle-axes.md, specs/S63-contested-evidence-warrants.md

## Problem

S140ARTLIFAXE-007 wires the live source-event subset for legal-effect suspension/restoration through `InstitutionalClaim::ForceControl { contested }` record events. Its reassessment found that the remaining S140 refutation branch still lacks a lawful source carrier on the live branch: `SceneEvidence` stores physical evidence on places, belief contradiction/status carriers can mark claims contradicted for readers, and justice records support accusations/verdicts, but no current case/evidence record addresses an `ArtifactHeader` or a posted artifact lifecycle axis.

The existing `golden_artifact_lifecycle.rs` refutation scenario therefore still injects an explicit `ArtifactTransitionPayload` for `Credibility::Refuted`. That proves the lower lifecycle/actionability cascade, but not the S140 source-event contract for false-rumor or evidence-against refutation.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-systems/src/artifact_lifecycle.rs` handles source-backed legal-effect suspension/restoration through `ForceControl` record events after S140ARTLIFAXE-007, but `credibility_stage()` remains empty.
2. `crates/worldwake-core/src/evidence.rs` defines `SceneEvidence` and `EvidenceEntry`, but those entries do not identify an artifact whose credibility they refute.
3. `crates/worldwake-core/src/institutional.rs` defines accusations and guilty verdicts, but the live branch does not yet include S63 `CaseRecord`, exoneration, contested evidence, alibi, or case-revision carriers.
4. `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` still proves `Credibility::Refuted -> Actionability::Closed` through an explicit transition payload. That fixture must not be described as source-event-backed until a real source carrier exists.
5. **Shared abstraction boundary**: artifact-addressed evidence/case source carrier -> `artifact_lifecycle_system` credibility transition -> append-only `ArtifactTransition` event -> actionability cascade.
6. **Ordering layer**: event-log ordering within `artifact_lifecycle_system`; source event, credibility transition, and actionability close must remain distinct events when the source-backed branch lands.
7. **Scenario isolation**: refutation proof should isolate one artifact-addressed evidence/case branch. Full FOUNDATIONS Scenario G remains out of scope unless the necessary S63 substrate has already landed.

## Architecture Check

1. The clean implementation must add or reuse a source carrier that explicitly addresses the artifact or artifact-backed case being refuted. It must not infer refutation from unrelated scene evidence or generic contradicted beliefs.
2. No backwards-compatibility aliases or direct actionability writes are introduced. Actionability closes only by observing the emitted credibility transition event.

## Verification Layers

1. Artifact-addressed source carrier -> focused system test proving `ArtifactTransitionPayload { axis: Credibility, new: Refuted, cause_event: Some(source_event) }`.
2. Credibility transition -> event-log delta assertions proving `prior`, `new`, `axis`, `cause_event`, and tick/sequence ordering.
3. Actionability cascade -> focused lifecycle or golden assertion proving stage 5 observes the credibility transition and emits `ArtifactActionability::Closed { cause: Refuted }`.
4. Golden surface -> update `golden_artifact_lifecycle.rs` only after the fixture can use the source-backed refutation path honestly.

## What to Change

### 1. Reassess live S63 substrate

Inspect current case/evidence/warrant implementation before coding. If S63 carriers have not landed yet, update this ticket rather than inventing a placeholder refutation event.

### 2. Wire source-backed refutation

When a lawful source carrier exists, update `credibility_stage()` to emit `ArtifactCredibility::Refuted { refuted_at, evidence }` transitions with a concrete `cause_event`.

### 3. Update proof surfaces

Add focused source-event-to-transition tests. If the golden can now use the source path, replace the explicit transition fixture and regenerate generated golden docs.

## Files to Touch

- `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify)
- `crates/worldwake-core/src/*` or `crates/worldwake-systems/src/*` as needed for the landed S63 source carrier (modify only if live reassessment proves ownership)
- `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` (modify if the refutation golden moves from explicit transition payload to source event)
- `docs/generated/golden-*` and `docs/generated/golden-scenario-details/*` (regenerate if golden metadata changes)
- `specs/S140-artifact-lifecycle-axes.md` (truth-sync if source ownership changes)

## Out of Scope

- Replacing the already-landed legal-effect suspension/restoration path from S140ARTLIFAXE-007.
- Full FOUNDATIONS Scenario G unless the live S63 substrate already supports the required source carriers.
- Promoting accusations to `ArtifactHeader` without a spec-level decision.

## Acceptance Criteria

### Tests That Must Pass

1. Focused lifecycle/system tests prove source-event-backed credibility refutation emits the expected artifact-axis transition.
2. If `golden_artifact_lifecycle.rs` changes, `cargo test -p worldwake-ai --test golden_artifact_lifecycle` passes and `python3 scripts/golden_inventory.py --write --check-docs` passes.
3. Existing suite: `./scripts/verify.sh`.

### Invariants

1. Refutation never mutates actionability directly; the close transition flows through append-only `ArtifactTransition` events.
2. Every source-backed refutation records a concrete `cause_event`.
3. S140 and generated golden docs do not call the explicit refutation fixture source-event-backed while the source carrier is still absent.

## Test Plan

### New/Modified Tests

1. Focused lifecycle/system tests under the owning crate for source-event-to-refutation behavior.
2. `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` only if the refutation golden moves to a source-backed event.

### Commands

1. `cargo test -p worldwake-systems --lib artifact_lifecycle`
2. `cargo test -p worldwake-ai --test golden_artifact_lifecycle` if the golden changes
3. `python3 scripts/golden_inventory.py --write --check-docs` if golden metadata changes
4. `./scripts/verify.sh`
