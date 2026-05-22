# S165EPIVERREP-001: AskWitness single-step constructor for repair reuse

**Status**: PENDING
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
   not the full output construction.
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

## Verification Layers

1. Repair-facing constructor returns a correct `ask_witness` `PlannedStep` for a lawful
   co-located witness, and `None` when the anchoring rule or cooldown rejects → focused
   unit test in `candidate_generation.rs`.
2. Bulk-emitter parity: `extract_ask_witness_candidates` emits the same `GoalOffer` set
   before/after the refactor → focused unit test (existing emitter test extended).
3. Single-layer (candidate generation) ticket — no action-trace/event-log mapping
   applies; the constructor produces no authoritative mutation.

## What to Change

### 1. Extract the witness-anchoring + payload helper

Factor the per-witness gate (lawful-source check via
`entity_beliefs_sourced_from_witness` / confidence threshold + `AskWitnessMemory`
cooldown) and the `AskWitnessPayload` synthesis out of
`extract_ask_witness_candidates` into a private helper that both callers share.

### 2. Add the repair-facing step constructor

Add `fn ask_witness_verification_step(agent, witness, subject, view) -> Option<PlannedStep>`
(name at implementer discretion) that uses the shared helper and, on success, builds the
`PlannedStep` for the `ask_witness` action toward `witness` with the synthesized
`AskWitnessPayload { topic_entity: Some(subject), .. }` and `TellTopic::EntityBelief`.
Returns `None` when the witness is not a lawful source for the subject or the cooldown is
active.

### 3. Route the bulk emitter through the shared helper

Refactor `extract_ask_witness_candidates` to call the shared helper for its gate/payload
logic, preserving its `GoalOffer` output and emission cap.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- The repair seam that *calls* the new constructor (ticket 003).
- Any change to `extract_ask_witness_candidates`'s emission cap or gating thresholds.
- Place-search / `ExploreLocation` verification (spec Non-Goal).

## Acceptance Criteria

### Tests That Must Pass

1. New: constructor returns a `PlannedStep` targeting the witness with an
   `AskWitnessPayload` whose `topic_entity == Some(subject)` for a lawful co-located
   witness.
2. New: constructor returns `None` when the witness is not a lawful source for the
   subject, and when the `AskWitnessMemory` cooldown is active.
3. Existing emitter behavior unchanged: `cargo test -p worldwake-ai candidate_generation`.

### Invariants

1. Exactly one code path synthesizes the `ask_witness` anchoring decision and
   `AskWitnessPayload` (FND-28) — the bulk emitter and the repair constructor share it.
2. The constructor performs no authoritative world read for the subject; it reads only
   the lawful belief view (FND-14).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (inline `#[cfg(test)]`) — constructor
   success/`None` cases; emitter-parity assertion.

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `./scripts/verify.sh`
