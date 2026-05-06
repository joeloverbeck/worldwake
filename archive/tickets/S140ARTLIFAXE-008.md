# S140ARTLIFAXE-008: Source-event credibility refutation for artifact lifecycle

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — artifact-addressed institutional record source-carrier integration into `artifact_lifecycle_system` credibility transitions
**Deps**: archive/tickets/S140ARTLIFAXE-007.md, archive/specs/S140-artifact-lifecycle-axes.md, specs/S63-contested-evidence-warrants.md

## Problem

S140ARTLIFAXE-007 wires the live source-event subset for legal-effect suspension/restoration through `InstitutionalClaim::ForceControl { contested }` record events. Its reassessment found that the remaining S140 refutation branch still lacked a lawful source carrier on the live branch: `SceneEvidence` stores physical evidence on places, belief contradiction/status carriers can mark claims contradicted for readers, and justice records support accusations/verdicts, but no current case/evidence record addresses an `ArtifactHeader` or a posted artifact lifecycle axis.

Before this ticket, the `golden_artifact_lifecycle.rs` refutation scenario still injected an explicit `ArtifactTransitionPayload` for `Credibility::Refuted`. That proved the lower lifecycle/actionability cascade, but not the S140 source-event contract for false-rumor or evidence-against refutation.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-systems/src/artifact_lifecycle.rs` handles source-backed legal-effect suspension/restoration through `ForceControl` record events after S140ARTLIFAXE-007, but `credibility_stage()` was empty before this ticket.
2. `crates/worldwake-core/src/evidence.rs` defines `SceneEvidence` and `EvidenceEntry`, but those entries do not identify an artifact whose credibility they refute.
3. `crates/worldwake-core/src/institutional.rs` defines accusations and guilty verdicts, but the live branch does not yet include S63 `CaseRecord`, exoneration, contested evidence, alibi, or case-revision carriers. This ticket therefore owns only the minimal artifact-addressed record carrier needed for S140 lifecycle provenance: `InstitutionalClaim::ArtifactCredibilityRefutation { artifact, evidence, effective_tick }`.
4. `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` previously proved `Credibility::Refuted -> Actionability::Closed` through an explicit transition payload. This ticket moves that fixture to the source-backed path through the new artifact-addressed institutional record claim.
5. **Shared abstraction boundary**: artifact-addressed record claim -> `artifact_lifecycle_system` credibility transition -> append-only `ArtifactTransition` event -> actionability cascade.
6. **Ordering layer**: event-log ordering within `artifact_lifecycle_system`; source event, credibility transition, and actionability close remain distinct events for the source-backed branch.
7. **Scenario isolation**: refutation proof should isolate one artifact-addressed record/evidence branch. Full FOUNDATIONS Scenario G remains out of scope because the broader S63 case/alibi/exoneration substrate has not landed.

## Architecture Check

1. The clean implementation adds a source carrier that explicitly addresses the artifact being refuted. It must not infer refutation from unrelated scene evidence or generic contradicted beliefs.
2. No backwards-compatibility aliases or direct actionability writes are introduced. Actionability closes only by observing the emitted credibility transition event.

## Verification Layers

1. Artifact-addressed source carrier -> focused system test proving `ArtifactTransitionPayload { axis: Credibility, new: Refuted, cause_event: Some(source_event) }`.
2. Credibility transition -> event-log delta assertions proving `prior`, `new`, `axis`, `cause_event`, and tick/sequence ordering.
3. Actionability cascade -> focused lifecycle or golden assertion proving stage 5 observes the credibility transition and emits `ArtifactActionability::Closed { cause: Refuted }`.
4. Golden surface -> update `golden_artifact_lifecycle.rs` only through the honest source-backed refutation path.

## What to Change

### 1. Reassess live S63 substrate

Inspect current case/evidence/warrant implementation before coding. S63 carriers have not landed, so this ticket lands the bounded artifact-addressed institutional record claim rather than a placeholder event or full case workflow.

### 2. Wire source-backed refutation

Update `credibility_stage()` to emit `ArtifactCredibility::Refuted { refuted_at, evidence }` transitions with a concrete `cause_event` when it observes a same-tick `ArtifactCredibilityRefutation` record entry.

### 3. Update proof surfaces

Add focused source-event-to-transition tests, replace the explicit transition fixture with the source-backed path, and regenerate generated golden docs.

## Files to Touch

- `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify)
- `crates/worldwake-core/src/institutional.rs` and belief helpers (new persisted claim/key)
- `crates/worldwake-sim/src/*`, `crates/worldwake-systems/src/*`, and `crates/worldwake-ai/src/*` exhaustive belief/relay/trace/ranking fallout for the new institutional claim
- `crates/worldwake-sim/src/save_load.rs` (save-format bump for persisted institutional claim shape)
- `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` (replace explicit transition payload with source event)
- `docs/generated/golden-scenario-index.md` and `docs/generated/golden-scenario-details/artifact-lifecycle.md` (regenerated metadata)
- `archive/specs/S140-artifact-lifecycle-axes.md` (truth-sync source ownership and save-format version)

## Out of Scope

- Replacing the already-landed legal-effect suspension/restoration path from S140ARTLIFAXE-007.
- Full FOUNDATIONS Scenario G and the broader S63 case/alibi/exoneration workflow.
- Promoting accusations to `ArtifactHeader` without a spec-level decision.

## Acceptance Criteria

### Tests That Must Pass

1. Focused lifecycle/system tests prove source-event-backed credibility refutation emits the expected artifact-axis transition.
2. `cargo test -p worldwake-ai --test golden_artifact_lifecycle` passes and `python3 scripts/golden_inventory.py --write --check-docs` passes.
3. Existing suite: `./scripts/verify.sh`.

### Invariants

1. Refutation never mutates actionability directly; the close transition flows through append-only `ArtifactTransition` events.
2. Every source-backed refutation records a concrete `cause_event`.
3. S140 and generated golden docs name the bounded `ArtifactCredibilityRefutation` carrier honestly and do not imply full S63 case/evidence adjudication has landed.

## Test Plan

### New/Modified Tests

1. Focused lifecycle/system tests under the owning crate for source-event-to-refutation behavior.
2. `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` moves the refutation golden to a source-backed event.

### Commands

1. `cargo test -p worldwake-systems --lib artifact_lifecycle`
2. `cargo test -p worldwake-ai --test golden_artifact_lifecycle`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-06.

- Added `InstitutionalClaim::ArtifactCredibilityRefutation { artifact, evidence, effective_tick }` plus the matching institutional belief key so refutations can be stored in append-only `RecordData` and relayed through existing institutional knowledge paths.
- Wired `artifact_lifecycle_system` credibility handling to read same-tick record-entry deltas, emit `ArtifactTransitionPayload { axis: Credibility, new: Refuted, cause_event: Some(source_event) }`, and let the existing actionability stage close via the emitted transition.
- Replaced the Scenario 392 golden fixture’s explicit transition injection with the source-backed record event and regenerated the golden scenario docs.
- Bumped `SAVE_FORMAT_VERSION` to 74 because the new institutional claim variant is persisted.
- Truth-synced `archive/specs/S140-artifact-lifecycle-axes.md` to name the bounded carrier and leave full S63 case/evidence adjudication out of scope.

## Deviations

- The live branch still lacks S63 `CaseRecord`, exoneration, contested evidence, alibi, and case-revision carriers. This ticket therefore lands the minimal artifact-addressed institutional record carrier required for S140 lifecycle provenance instead of a full S63 case workflow.

## Verification Result

- Passed `cargo test -p worldwake-core --lib institutional`.
- Passed `cargo test -p worldwake-systems --lib artifact_lifecycle`.
- Passed `cargo test -p worldwake-ai --test golden_artifact_lifecycle`.
- Passed `cargo test -p worldwake-sim --lib save_load`.
- Passed `cargo test -p worldwake-sim --lib institutional_knowledge_trace`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `./scripts/verify.sh`.
