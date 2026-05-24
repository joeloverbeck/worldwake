# S165EPIVERREP-001: AskWitness single-step constructor for repair reuse

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` candidate generation (`candidate_generation.rs`)
**Deps**: specs/S165-epistemic-verification-repair.md (D2)

## Problem

The epistemic verification repair (S165) must splice a single `ask_witness`
`PlannedStep` toward a co-located witness. The existing
`extract_ask_witness_candidates` (`crates/worldwake-ai/src/candidate_generation.rs:3063`)
is a bulk per-agent emitter (`&GenerationContext` → pushes many `GoalOffer`s) and
cannot produce one targeted step for a given `(witness, subject)`. Without a reusable
constructor, the repair seam (ticket 003) would have to re-derive the S139
witness-anchoring rule and `AskWitnessPayload` synthesis, creating a second
construction path that can drift from the emitter (FND-28).

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `extract_ask_witness_candidates` exists at `crates/worldwake-ai/src/candidate_generation.rs:3063`
   as `fn extract_ask_witness_candidates(candidates: &mut Vec<GoalOffer>, diagnostics:
   &mut CandidateGenerationDiagnostics, ctx: &GenerationContext<'_>)`. It iterates local
   witnesses and known beliefs, gates each by confidence/cooldown/testimony reliability,
   and pushes one `GoalOffer` per `(witness, topic)` (cap
   `ASK_WITNESS_EMISSION_CAP_PER_TOPIC`). It emits **goals**, not `PlannedStep`s.
2. Spec deliverable D2 (`specs/S165-epistemic-verification-repair.md`). The repair-facing
   output is a `PlannedStep` (`RepairPlanCandidate.step: PlannedStep`,
   `crates/worldwake-ai/src/plan_repair.rs:24`), distinct from the emitter's `GoalOffer`
   output. The shareable surface is therefore the **lawfulness gate + payload synthesis**,
   not the full output construction. Live reassessment found one necessary signature
   correction: a `PlannedStep` also needs the concrete `ActionDefId`, so the
   repair-facing constructor takes the resolved `ask_witness` action id rather than
   deriving it from the belief view.
3. Shared boundary under audit: the `ask_witness` payload shape (`AskWitnessPayload`,
   `crates/worldwake-sim/src/action_payload.rs`) and the S139 anchoring rule (lawful
   co-located witness for a subject), surfaced via
   `entity_beliefs_sourced_from_witness` (`crates/worldwake-sim/src/belief_view.rs:339`)
   and `AskWitnessMemory` cooldown (`crates/worldwake-core/src/belief.rs:1845`).
4. Live `GoalKind` under test: `GoalKind::AskWitness { witness, topic }`
   (`crates/worldwake-core/src/goal.rs:145`) with `TellTopic::EntityBelief { subject }`
   (`crates/worldwake-core/src/belief.rs:1812`). The op is the existing `ask_witness`
   action (`crates/worldwake-systems/src/epistemic_actions.rs`); no new op.

## Architecture Check

1. Extracting the witness-anchoring + payload-synthesis logic into one helper, then
   having both the bulk emitter and a thin repair-facing `PlannedStep` constructor call
   it, keeps a single source of truth for "is this witness a lawful source for this
   subject, and what payload do we send" (FND-28). The two callers differ only in output
   type (`GoalOffer` vs `PlannedStep`), which is the legitimate impedance between
   candidate generation and a pre-built repair step.
2. No backwards-compatibility shim: `extract_ask_witness_candidates` is refactored to
   route through the shared helper, not aliased beside a duplicate.

## Verified Layers

1. Repair-facing constructor returns a correct `ask_witness` `PlannedStep` for a lawful
   co-located witness, and `None` when the anchoring rule or cooldown rejects → focused
   unit tests in `candidate_generation.rs`.
2. Bulk-emitter behavior unchanged after the refactor → existing `ask_witness_emitter_*`
   focused tests still pass through the shared helper.
3. Single-layer (candidate generation) ticket — no action-trace/event-log mapping
   applies; the constructor produces no authoritative mutation.

## Landed Changes

### 1. Extracted the witness-anchoring + payload helper

Factored the per-witness confidence threshold, `AskWitnessMemory` cooldown,
testimony-reliability suppression, salience calculation, and `AskWitnessPayload`
synthesis out of `extract_ask_witness_candidates` into shared helper functions.

### 2. Added the repair-facing step constructor

Added `ask_witness_verification_step(view, agent, witness, subject, ask_witness_def_id)`.
It uses the shared gate/payload helper and, on success, builds the `PlannedStep` for
the `ask_witness` action toward `witness` with `AskWitnessPayload {
topic_entity: Some(subject), topic_commodity: None, .. }` and
`TellTopic::EntityBelief`. It returns `None` when the witness is not a lawful report
source for the subject or the cooldown is active.

### 3. Routed the bulk emitter through the shared helper

Refactored `extract_ask_witness_candidates` to call the shared helper for its
gate/payload logic while preserving its `GoalOffer` output, cold-start branch,
testimony suppression diagnostics, and per-topic emission cap.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs` (modified)
- `specs/S165-epistemic-verification-repair.md` (modified — D2 signature truth-sync)

## Out of Scope

- The repair seam that *calls* the new constructor (ticket 003).
- Any change to `extract_ask_witness_candidates`'s emission cap or gating thresholds.
- Place-search / `ExploreLocation` verification (spec Non-Goal).

## Acceptance Criteria

### Test Result

1. Added: constructor returns a `PlannedStep` targeting the witness with an
   `AskWitnessPayload` whose `topic_entity == Some(subject)` for a lawful co-located
   witness.
2. Added: constructor returns `None` when the witness is not a lawful source for the
   subject, and when the `AskWitnessMemory` cooldown is active.
3. Existing emitter behavior unchanged: `cargo test -p worldwake-ai candidate_generation`
   passed.

### Invariants

1. Exactly one code path synthesizes the `ask_witness` anchoring decision and
   `AskWitnessPayload` (FND-28) — the bulk emitter and the repair constructor share it.
2. The constructor performs no authoritative world read for the subject; it reads only
   the lawful belief view (FND-14).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (inline `#[cfg(test)]`) — constructor
   success/`None` cases:
   `ask_witness_verification_step_builds_targeted_payload_for_lawful_witness` and
   `ask_witness_verification_step_rejects_non_source_witness_and_cooldown`.
2. Existing inline `ask_witness_emitter_*` tests continue to cover emitter parity
   through the shared helper.

### Commands Run

1. Passed `cargo test -p worldwake-ai candidate_generation`
2. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. Passed `./scripts/verify.sh`

## Outcome

Completed on 2026-05-24.

- Extracted the shared `AskWitness` gate/payload path from the bulk emitter.
- Added a repair-facing `ask_witness_verification_step` constructor that produces the
  single co-located `ask_witness` `PlannedStep` needed by later S165 repair-seam tickets.
- Added focused constructor tests for the lawful-source success case, non-source
  rejection, and cooldown rejection.
- Truth-synced S165 D2 to record the live `ActionDefId` requirement for constructing a
  `PlannedStep`.

## Deviations

- The draft described a constructor that could be called with only
  `(agent, witness, subject, view)`. The landed constructor also takes the resolved
  `ask_witness` `ActionDefId`, because `PlannedStep.def_id` is mandatory and the belief
  view does not own action-definition lookup.

## Verification Result

- Passed `cargo test -p worldwake-ai candidate_generation`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
