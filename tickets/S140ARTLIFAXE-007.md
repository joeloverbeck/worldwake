# S140ARTLIFAXE-007: Source-event artifact lifecycle transitions for suspension and refutation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `artifact_lifecycle_system` source-event stages, event/source-carrier integration where live carriers exist, focused lifecycle/golden proof updates
**Deps**: archive/tickets/S140ARTLIFAXE-006.md, specs/S140-artifact-lifecycle-axes.md, specs/S63-contested-evidence-warrants.md

## Problem

S140ARTLIFAXE-006 proved the actionability cascade after explicit `ArtifactTransitionPayload` inputs, but its post-ticket review found that two drafted S140 validation branches are still not source-event-backed on the live branch. `crates/worldwake-systems/src/artifact_lifecycle.rs` keeps `credibility_stage()` empty and has no source-event path for `Suspended` / restored legal effect or `Credibility::Refuted`; the 006 golden fixture therefore injected transition payloads directly for those two branches.

That is an honest E2E lifecycle proof, but it is not the full S140 source-event contract now assigned to this follow-up in `specs/S140-artifact-lifecycle-axes.md` lines 187-188 and 304-305. This ticket owns reconciling that gap without inventing unsupported story triggers.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `archive/tickets/S140ARTLIFAXE-006.md` is archived and explicitly states that the suspended/refuted proofs use explicit lifecycle transition payloads, not domain-source event authoring.
2. `crates/worldwake-systems/src/artifact_lifecycle.rs` currently calls `credibility_stage()` and `visibility_stage()`, but those stages are empty. The legal-effect stage handles TTL expiry only; action commits already emit fulfillment/revocation transition payloads and actionability observes those payloads.
3. Post-ticket review truth-synced `specs/S140-artifact-lifecycle-axes.md` so the `Suspended ← suspension events` and `Refuted ← evidence-against events` branches now cite this ticket as the source-event wiring owner instead of implying that the empty lifecycle stages already handle them.
4. `specs/S63-contested-evidence-warrants.md` is the likely future domain owner for contested evidence, wrongful accusation, and institutional correction. This ticket must check that live substrate before deciding how much source-event wiring is lawful now.
5. **Shared abstraction boundary**: domain source event or authoritative case/evidence state -> `artifact_lifecycle_system` per-axis transition payload -> event-log append-only `ArtifactTransition` -> actionability cascade.
6. **Ordering layer**: event-log ordering within the artifact lifecycle system. If source events are same-tick inputs, assertions must distinguish source event, axis transition, and actionability cascade sequence.
7. **Scenario isolation**: source-event goldens must isolate one source branch at a time. Do not fold the full FOUNDATIONS Scenario G AskWitness / wrongful-accusation chain into this ticket unless the live S63 substrate is already capable of expressing it.
8. **Mismatch correction**: if live domain carriers for suspension or evidence refutation do not exist yet, update S140 validation prose to name the available source-event subset and create or cite the later S63-derived owner instead of emitting synthetic domain events.

## Architecture Check

1. The clean end state is a single lawful source-event path into artifact axis transitions. Tests may construct source events directly, but they must not bypass the source-to-transition boundary when the ticket claims source-event coverage.
2. No backwards-compatibility shims or aliases are allowed. Existing explicit transition-payload helpers remain valid as lifecycle-unit proof seams, but they are not a substitute for source-event integration.

## Verification Layers

1. Source event or authoritative case/evidence state -> focused system test proving the corresponding `ArtifactTransitionPayload` is emitted.
2. Axis transition -> event-log delta assertions proving `prior`, `new`, `axis`, `cause_event`, and tick/sequence ordering.
3. Actionability cascade -> lifecycle/golden assertion proving stage 5 observes the source-backed transition and emits `ArtifactActionability::Closed` when appropriate.
4. Planner/golden surface -> only required if the source-backed transition changes candidate availability or replaces one of the explicit-transition legs in `golden_artifact_lifecycle.rs`.

## What to Change

### 1. Reassess live source carriers

Inspect current evidence, justice, warrant, and artifact event/state surfaces to identify lawful source events for:

- legal-effect suspension and restoration
- credibility disputed/refuted transitions

Record which branches are implementable now and which belong to S63 or a later evidence-carrier spec.

### 2. Wire implementable source events into `artifact_lifecycle_system`

For each live source carrier that exists, update the appropriate lifecycle stage to emit a typed `ArtifactTransitionPayload` with a concrete `cause_event`.

Keep stage ordering deterministic. Do not add direct cross-axis writes; actionability must still close by observing the emitted transition event.

### 3. Update proof surfaces

Add or update focused tests for source-event-to-transition behavior. If an S140 golden can now use a source-backed transition honestly, update the corresponding `golden_artifact_lifecycle.rs` fixture and regenerate golden docs.

### 4. Truth-sync S140 when a branch remains future-owned

If one or more branches still lack lawful source carriers, update `specs/S140-artifact-lifecycle-axes.md` to say which branch remains future-owned and cite the owning spec or follow-up ticket. Do not leave S140 implying that empty lifecycle stages already handle those source events.

## Files to Touch

- `crates/worldwake-systems/src/artifact_lifecycle.rs` (modify — source-event stages)
- `crates/worldwake-systems/src/*` or `crates/worldwake-core/src/*` as needed for existing source-carrier integration (modify only if live reassessment proves ownership)
- `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` (modify only if a golden fixture moves from explicit transition payload to source-backed event)
- `docs/generated/golden-*` and `docs/generated/golden-scenario-details/*` (regenerate if golden metadata changes)
- `specs/S140-artifact-lifecycle-axes.md` (truth-sync source-event ownership)

## Out of Scope

- Full FOUNDATIONS Scenario G AskWitness / wrongful-accusation / exoneration chain unless the live S63 substrate already provides the necessary source carriers.
- Promoting accusations to `ArtifactHeader`; S140 explicitly left that to a future spec.
- Replacing the explicit-transition lifecycle unit tests that still prove the lower transition/cascade seam.

## Acceptance Criteria

### Tests That Must Pass

1. Focused lifecycle/system tests prove every newly wired source event emits the expected artifact-axis transition.
2. If `golden_artifact_lifecycle.rs` changes, `cargo test -p worldwake-ai --test golden_artifact_lifecycle` passes and `python3 scripts/golden_inventory.py --write --check-docs` passes.
3. Existing suite: `./scripts/verify.sh`.

### Invariants

1. No source-backed lifecycle branch mutates actionability directly; all cross-axis effects flow through append-only `ArtifactTransition` events.
2. Every source-backed transition records a concrete `cause_event` or records why no event id exists for the authoritative source carrier.
3. S140 spec prose does not claim source-event branches are live when the implementation still only supports explicit transition payloads.

## Test Plan

### New/Modified Tests

1. Focused system/lifecycle tests under the owning crate for source-event-to-transition behavior.
2. `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` only if source-backed proof replaces an explicit-transition golden leg.

### Commands

1. `cargo test -p worldwake-systems artifact_lifecycle`
2. `cargo test -p worldwake-ai --test golden_artifact_lifecycle` if the golden changes
3. `python3 scripts/golden_inventory.py --write --check-docs` if golden metadata changes
4. `./scripts/verify.sh`
