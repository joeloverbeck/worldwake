# E19GUAPAT-010: Retire the false single-substrate safety narrative and document the live contract

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ticket/spec scope correction, proof-surface strengthening, and tests guarding the existing architecture
**Deps**: [archive/specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/archive/specs/E19-guard-patrol.md), [archive/tickets/guard-patrol/E19GUAPAT-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-007.md), [archive/specs/E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/archive/specs/E17-crime-theft-justice.md), [specs/E22-integration-soak-tests.md](/home/joeloverbeck/projects/worldwake/specs/E22-integration-soak-tests.md)

## Problem

The ticket’s original framing assumes guards and thieves ought to consume one shared “settlement safety” substrate. Reassessment shows that assumption is wrong for the current architecture.

The live code intentionally has two different lawful decision surfaces for two different questions:

1. guard patrol urgency asks “how much reason do I have to intensify patrol duty in my jurisdiction?” and reads belief-carried violation/institutional evidence,
2. thief deterrence asks “is this immediate theft attempt risky right here, right now?” and reads direct co-located witness exposure.

The real defect is not “two systems exist.” The defect is that archived E19 language still overclaims a single public-order feedback loop and makes the code sound more unified than it is. That documentation drift is what now threatens explainability and future maintenance.

## Assumption Reassessment (2026-03-30)

1. The current guard patrol motive surface is in [`crates/worldwake-ai/src/ranking.rs:716`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs:716). `patrol_motive()` multiplies patrol profile weight by unresolved local theft memory plus believed vacancy and contested-control counts for offices relevant to the patrol route.
2. The current thief deterrence surface is in [`crates/worldwake-ai/src/theft.rs:31`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/theft.rs:31) and is consumed by both [`crates/worldwake-ai/src/candidate_generation.rs:2218`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs:2218) and [`crates/worldwake-ai/src/ranking.rs:667`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs:667). It is strictly based on directly observed co-located living agents, not on settlement-level derived state.
3. `public_order()` in [`crates/worldwake-systems/src/offices.rs:132`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs:132) is derived authoritative world-state math. Repository-wide usage is confined to that module’s own tests, so it is currently a designer/debug surface, not an AI substrate.
4. The exact shared abstraction boundary under audit is not “settlement safety” in the abstract. It is the contract between:
   - direct local theft-risk gating in `assess_theft_deterrence()`,
   - belief-driven patrol urgency in `patrol_motive()`,
   - and archived/spec language that currently misdescribes those distinct layers as one loop.
5. The live `GoalKind`s under test are `GoalKind::Patrol { place }` and `GoalKind::StealItem { target_item }`. The current operator surfaces are patrol ranking/candidate selection and theft candidate suppression, not a common settlement-security operator.
6. Existing proof already distinguishes the two layers:
   - [`crates/worldwake-ai/tests/golden_patrol.rs:476`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs:476) proves patrol urgency scales from local theft memory and vacancy belief,
   - [`crates/worldwake-ai/tests/golden_patrol.rs:682`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs:682) proves another agent’s remote report does not inflate this guard’s patrol motive,
   - [`crates/worldwake-ai/tests/golden_emergent.rs:6385`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs:6385) proves witness deterrence suppresses `StealItem`,
   - [`crates/worldwake-ai/src/candidate_generation.rs:6920`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs:6920) already has focused theft candidate gating coverage.
7. The archived E19 spec already records the key mismatch in its `Outcome` section at [`archive/specs/E19-guard-patrol.md:224`](/home/joeloverbeck/projects/worldwake/archive/specs/E19-guard-patrol.md:224), but the spec body and acceptance language still overclaim the feedback loop. The contradiction is therefore partly acknowledged but not fully retired.
8. Forcing one canonical substrate now would be architecturally worse under Principles 3, 7, 14, 15, 17, and 20 in [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): it would collapse immediate tactical witness risk and slower institutional/crime-memory patrol duty into one abstraction that answers two different questions.
9. The right clean end-state for this ticket is not aliasing both roles onto one score. It is explicit documentation that these are distinct lawful surfaces, plus test coverage that prevents future specs from silently reintroducing the false unified-loop claim.
10. If the project later wants a truly shared settlement-security substrate, it should be an explicit transmissible artifact or belief carrier that both roles can lawfully perceive. That is a separate architecture ticket, not an in-scope retrofit onto `public_order()`.
11. Mismatch + correction: the old ticket scope said “choose one canonical settlement-safety substrate.” The corrected scope is “remove the false canonical-substrate claim, document the live split contract, and harden tests around that contract.”

## Architecture Check

1. Keeping patrol urgency and immediate theft deterrence as separate first-class decision surfaces is cleaner than forcing them through one pseudo-shared substrate, because they operate on different time horizons, different lawful evidence, and different causal questions.
2. The robust fix is to make the contract explicit: guards reason from belief-carried crime/institutional instability; thieves reason from direct witness exposure; `public_order()` remains derived world-state telemetry until an explicit in-world security artifact exists.
3. This avoids two bad architectures:
   - piping `public_order()` into AI, which would violate locality and belief-only planning,
   - inventing a vague shared “safety score,” which would hide concrete provenance and reopen spec drift later.
4. No backwards-compatibility aliasing or shim path should be introduced. If a future shared substrate is added, it should replace or narrow one of these surfaces explicitly rather than sit beside them.

## Verification Layers

1. Guard patrol urgency remains belief-driven rather than world-state-driven -> decision-trace golden in [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs) plus focused ranking coverage
2. Theft deterrence remains direct local witness gating rather than settlement-score gating -> focused candidate-generation and ranking tests plus witness-deterrence golden in [`crates/worldwake-ai/tests/golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs)
3. `public_order()` stays derived/non-AI-facing -> focused unit tests in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs)
4. Spec/docs no longer claim a negative feedback loop the engine does not prove -> ticket/spec text review plus removal of stale acceptance language
5. Strongest proof surface for this ticket is focused AI/runtime coverage, not a new settlement-wide convergence golden. If future work adds a true shared security artifact, it will need its own mixed-layer proof.
6. Additional action-trace mapping is not applicable because this ticket is about candidate/ranking contracts and spec truthfulness, not action-lifecycle ordering.

## What to Change

### 1. Correct the written contract

Update the ticket/spec language so it describes the live architecture accurately:

- patrol urgency is belief-driven,
- theft deterrence is immediate witness-driven,
- `public_order()` is derived telemetry,
- and the archived E19 feedback-loop claim is removed or narrowed anywhere it still reads as live behavior.

### 2. Strengthen proof around the live split contract

Add or tighten focused tests where current proof is thin, especially around the boundary that patrolling-guard presence can deter theft only through lawful local witness exposure, not through `public_order()` or other settlement-level shortcuts.

### 3. Remove stale acceptance language from active planning material where needed

If [`specs/E22-integration-soak-tests.md`](/home/joeloverbeck/projects/worldwake/specs/E22-integration-soak-tests.md) or adjacent planning docs still imply a live patrol/public-order/crime convergence loop, narrow them to what the engine currently proves.

## Files to Touch

- `crates/worldwake-ai/tests/golden_patrol.rs` (modify, if proof strengthening is needed)
- `crates/worldwake-ai/tests/golden_emergent.rs` and/or focused AI tests (modify, if proof strengthening is needed)
- `archive/specs/E19-guard-patrol.md` (modify)
- `specs/E22-integration-soak-tests.md` (modify, if it still overclaims the loop)
- `tickets/E19GUAPAT-010.md` (modify)

## Out of Scope

- Re-architecting thieves to read `public_order()`
- Re-architecting guards to patrol from direct witness counts alone
- Inventing a hidden settlement “danger score”
- Captain-issued patrol assignment systems
- A brand-new shared security artifact or institutional notice system

## Acceptance Criteria

### Tests That Must Pass

1. Focused and/or golden tests explicitly prove the live split contract: belief-driven patrol urgency and direct-witness theft deterrence remain distinct lawful surfaces
2. No active ticket/spec text in scope still claims that `public_order()` currently drives thief avoidance or that a proven negative patrol/crime feedback loop is already live
3. Existing suite: `cargo test -p worldwake-ai golden_patrol -- --nocapture`
4. Existing suite: `cargo test -p worldwake-ai --test golden_emergent -- --nocapture`
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo test --workspace`
7. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Guard patrol urgency remains belief-local and explainable from local crime/institutional beliefs
2. Theft deterrence remains explainable from direct local witness exposure, not hidden settlement state
3. `public_order()` remains derived and non-authoritative for AI decisions unless a future explicit artifact-based design replaces that contract

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — `patrolling_guard_only_deterrs_theft_when_locally_observed`; proves guard presence only suppresses theft through same-place witness exposure, while remote patrol presence does not leak into theft candidate gating
2. `archive/specs/E19-guard-patrol.md` — retired the stale public-order feedback-loop claim from the archived spec body and acceptance language so the document now matches the shipped architecture
3. `specs/IMPLEMENTATION-ORDER.md` — corrected the E16→E19 dependency note so active planning material no longer implies guards depend on `public_order()` as an AI substrate

### Commands

1. `cargo test -p worldwake-ai patrolling_guard_only_deterrs_theft_when_locally_observed -- --nocapture`
2. `cargo test -p worldwake-ai theft_candidate_respects_preconditions_and_witness_gate -- --nocapture`
3. `cargo test -p worldwake-ai golden_patrol -- --nocapture`
4. `cargo test -p worldwake-ai --test golden_emergent -- --nocapture`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - Reassessed the ticket and narrowed scope away from forced guard/thief substrate unification, which would have been a worse architecture than the live split between belief-driven patrol urgency and direct-witness theft deterrence.
  - Added focused AI coverage in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) proving that a patrolling guard only deters theft when locally observed by the thief.
  - Updated [`archive/specs/E19-guard-patrol.md`](/home/joeloverbeck/projects/worldwake/archive/specs/E19-guard-patrol.md) and [`specs/IMPLEMENTATION-ORDER.md`](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) so planning/spec text no longer overclaims a live shared patrol/public-order/theft feedback loop.
- Deviations from original plan:
  - Did not unify guards and thieves behind one canonical settlement-safety substrate.
  - Did not route thieves through `public_order()` or introduce a new shared settlement-security score/artifact.
  - The ticket was completed as architecture correction plus proof strengthening, because that is the cleaner and more extensible outcome under the current foundations.
- Verification results:
  - `cargo test -p worldwake-ai patrolling_guard_only_deterrs_theft_when_locally_observed -- --nocapture`
  - `cargo test -p worldwake-ai theft_candidate_respects_preconditions_and_witness_gate -- --nocapture`
  - `cargo test -p worldwake-ai golden_patrol -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_emergent -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
