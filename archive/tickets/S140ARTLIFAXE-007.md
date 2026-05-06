# S140ARTLIFAXE-007: Source-event artifact lifecycle suspension transitions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `artifact_lifecycle_system` source-event stages, event/source-carrier integration where live carriers exist, focused lifecycle/golden proof updates
**Deps**: archive/tickets/S140ARTLIFAXE-006.md, specs/S140-artifact-lifecycle-axes.md, specs/S63-contested-evidence-warrants.md

## Problem

S140ARTLIFAXE-006 proved the actionability cascade after explicit `ArtifactTransitionPayload` inputs, but its post-ticket review found that two drafted S140 validation branches were still not source-event-backed on the live branch. `crates/worldwake-systems/src/artifact_lifecycle.rs` kept `credibility_stage()` empty and had no source-event path for `Suspended` / restored legal effect or `Credibility::Refuted`; the 006 golden fixture therefore injected transition payloads directly for those two branches.

That is an honest E2E lifecycle proof, but it is not the full S140 source-event contract. This ticket owns the implementable source-event subset for legal-effect suspension/restoration and the truthful handoff for source-backed refutation without inventing unsupported story triggers.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `archive/tickets/S140ARTLIFAXE-006.md` is archived and explicitly states that the suspended/refuted proofs use explicit lifecycle transition payloads, not domain-source event authoring.
2. `crates/worldwake-systems/src/artifact_lifecycle.rs` called `credibility_stage()` and `visibility_stage()`, but those stages were empty. The legal-effect stage handled TTL expiry only; action commits already emitted fulfillment/revocation transition payloads and actionability observed those payloads.
3. Live source-carrier reassessment found one lawful source-event subset: `InstitutionalClaim::ForceControl { contested }` entries appended through `RecordData` component deltas can suspend and restore artifacts issued by or under that office because they identify the concrete office whose legal authority is contested/resolved.
4. Live refutation reassessment found no lawful source carrier yet. `SceneEvidence` entries do not address an artifact; belief contradiction/status carriers are reader projections, not authoritative artifact lifecycle sources; live justice records have accusations and guilty verdicts but no S63 `CaseRecord`, exoneration, contested evidence, alibi, or case revision carrier.
5. `tickets/S140ARTLIFAXE-008.md` now owns source-event-backed credibility refutation after the S63-style case/evidence carrier can lawfully address an artifact lifecycle axis.
6. **Shared abstraction boundary**: domain source event or authoritative case/evidence state -> `artifact_lifecycle_system` per-axis transition payload -> event-log append-only `ArtifactTransition` -> actionability cascade.
7. **Ordering layer**: event-log ordering within the artifact lifecycle system. Source events, axis transitions, and actionability cascades remain distinct events.
8. **Scenario isolation**: the updated suspended/restored golden isolates one `ForceControl` contested/resolved source branch. The full FOUNDATIONS Scenario G AskWitness / wrongful-accusation chain remains out of scope until S63 substrate exists.
9. **Mismatch correction**: S140 now names `ForceControl` as the live suspension/restoration source and hands off refutation to S140ARTLIFAXE-008 instead of implying the empty credibility stage handles source-backed refutation.

## Architecture Check

1. The clean landed path is a single lawful source-event path from office force-control records into legal-effect axis transitions. Tests construct source events directly by appending record entries, but they do not bypass the source-to-transition boundary for the suspension/restoration branch.
2. No backwards-compatibility shims or aliases are added. Existing explicit transition-payload helpers remain valid lifecycle-unit proof seams, but the refutation fixture is explicitly future-owned until an artifact-addressed evidence/case source exists.

## Verification Layers

1. `InstitutionalClaim::ForceControl { contested: true }` source event -> focused system test proving `ArtifactLegalEffect::Suspended { reason: JurisdictionDispute }` transition is emitted with `cause_event`.
2. `InstitutionalClaim::ForceControl { contested: false }` source event -> focused system test proving a jurisdiction suspension restores to `ArtifactLegalEffect::Active { expires_at }` with `cause_event`.
3. Axis transition -> event-log delta assertions proving `prior`, `new`, `axis`, `cause_event`, and tick ordering.
4. Golden surface -> `golden_artifact_lifecycle.rs` suspended/restored scenario proves the source-backed branch and generated docs mirror the source chain.
5. Refutation source-event coverage -> deferred to `tickets/S140ARTLIFAXE-008.md`; current explicit transition fixture remains lower-lifecycle proof only.

## What to Change

### 1. Reassess live source carriers

Inspect current evidence, justice, warrant, and artifact event/state surfaces to identify lawful source events for:

- legal-effect suspension and restoration
- credibility disputed/refuted transitions

Record which branches are implementable now and which belong to S63 or a later evidence-carrier spec.

### 2. Wire implementable source events into `artifact_lifecycle_system`

For each live source carrier that exists, update the appropriate lifecycle stage to emit a typed `ArtifactTransitionPayload` with a concrete `cause_event`. S140ARTLIFAXE-007 wires only the `ForceControl` legal-effect source subset.

Keep stage ordering deterministic. Do not add direct cross-axis writes; actionability must still close by observing the emitted transition event.

### 3. Update proof surfaces

Add or update focused tests for source-event-to-transition behavior. If an S140 golden can now use a source-backed transition honestly, update the corresponding `golden_artifact_lifecycle.rs` fixture and regenerate golden docs.

### 4. Truth-sync S140 when a branch remains future-owned

If one or more branches still lack lawful source carriers, update `specs/S140-artifact-lifecycle-axes.md` to say which branch remains future-owned and cite the owning spec or follow-up ticket. Do not leave S140 implying that empty lifecycle stages already handle those source events.

## Files to Touch

- `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify — source-event stages)
- `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` (modify — suspended/restored fixture moves from explicit transition payload to source-backed `ForceControl` events)
- `docs/generated/golden-scenario-index.md` and `docs/generated/golden-scenario-details/artifact-lifecycle.md` (regenerate)
- `specs/S140-artifact-lifecycle-axes.md` (truth-sync source-event ownership)
- `tickets/S140ARTLIFAXE-008.md` (new — refutation source-event follow-up)

## Out of Scope

- Full FOUNDATIONS Scenario G AskWitness / wrongful-accusation / exoneration chain unless the live S63 substrate already provides the necessary source carriers.
- Promoting accusations to `ArtifactHeader`; S140 explicitly left that to a future spec.
- Replacing the explicit-transition lifecycle unit tests that still prove the lower transition/cascade seam.
- Source-event-backed credibility refutation; this is now owned by `tickets/S140ARTLIFAXE-008.md`.

## Acceptance Criteria

### Tests That Must Pass

1. Focused lifecycle/system tests prove `ForceControl` contested/resolved source events emit the expected legal-effect transitions.
2. `cargo test -p worldwake-ai --test golden_artifact_lifecycle` passes and `python3 scripts/golden_inventory.py --write --check-docs` passes.
3. Existing suite: `./scripts/verify.sh`.

### Invariants

1. No source-backed lifecycle branch mutates actionability directly; all cross-axis effects flow through append-only `ArtifactTransition` events.
2. Every source-backed transition records a concrete `cause_event`.
3. S140 spec prose does not claim credibility refutation is source-event-backed while the implementation still only supports explicit transition payloads for that branch.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_lifecycle.rs` — focused source-event-to-transition tests for `ForceControl` contest and resolution.
2. `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` — suspended/restored scenario now uses source-backed `ForceControl` events.

### Commands

1. `cargo test -p worldwake-systems --lib artifact_lifecycle`
2. `cargo test -p worldwake-ai --test golden_artifact_lifecycle`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-06.

- Wired `artifact_lifecycle_system` legal-effect handling to same-tick `InstitutionalClaim::ForceControl { contested }` source events from `RecordData` deltas. Contested office control now emits `ArtifactLegalEffect::Suspended { reason: JurisdictionDispute }`; a resolved force-control claim restores jurisdiction suspensions to `Active { expires_at }`.
- Added focused lifecycle tests for source-backed suspension and restoration, including `cause_event` assertions against the originating record event.
- Updated the S140 suspended/restored golden fixture from explicit legal-effect transition injection to source-backed `ForceControl` record events, then regenerated the generated golden scenario detail and index pages.
- Truth-synced `specs/S140-artifact-lifecycle-axes.md` to name the live source-backed suspension path and created `tickets/S140ARTLIFAXE-008.md` for future source-backed credibility refutation.

## Deviations

- The refuted-rumor golden remains an explicit `ArtifactTransitionPayload` fixture. Live `SceneEvidence`, belief contradiction/status projection, and current justice accusation/verdict records do not yet provide an artifact-addressed case/evidence source carrier, so source-backed credibility refutation is deferred to `tickets/S140ARTLIFAXE-008.md`.
- No core event payload or save-format shape changed; the implementation derives source events from existing persisted `EventRecord`/`RecordData` component deltas.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib artifact_lifecycle -- --list`.
- Passed `cargo test -p worldwake-systems --lib artifact_lifecycle`.
- Passed `cargo test -p worldwake-ai --test golden_artifact_lifecycle -- --list`.
- Passed `cargo test -p worldwake-ai --test golden_artifact_lifecycle`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `./scripts/verify.sh`.
