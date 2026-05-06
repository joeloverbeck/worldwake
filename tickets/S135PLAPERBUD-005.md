# S135PLAPERBUD-005: RootCandidateTrace omitted_anchor annotation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — decision-trace surface, planner candidate-discard wiring
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`, `archive/tickets/S135PLAPERBUD-002.md`

## Problem

Per S135 D4, when the planner discards a synthesized root candidate because its anchor entity is in the agent's `ObservationOmissionLog` rather than belief store, the decision trace must carry the typed reason so observer reports and goldens can answer "why did this agent ignore the dragon next to them?" This ticket adds the `omitted_anchor: Option<OmissionReason>` field to `RootCandidateTrace` and wires the planner's candidate-discard path to populate it.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RootCandidateTrace` lives at `crates/worldwake-ai/src/decision_trace.rs:786` with 7 existing fields (`def_id: ActionDefId`, `action_name: String`, `op_kind: Option<PlannerOpKind>`, `authoritative_targets: Vec<EntityId>`, `planner_only: bool`, `payload_status: RootCandidatePayloadStatus`, `outcome: RootCandidateOutcome`). Derives compatible with serde additive `#[serde(default)]` (validate against the actual derive set during implementation — likely `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`).
2. Synthesized root candidate emission lives in `crates/worldwake-ai/src/agent_tick/planning.rs` (the same area as ticket 003's snapshot construction at line 537). Candidate discard for a missing anchor is currently silent — the candidate is filtered out without trace annotation. The wiring point is the planner's candidate-validation loop where it confirms the anchor is in the agent's known beliefs.
3. Shared abstraction boundary under audit: `RootCandidateTrace` (the typed surface that observer Section 8 "Per-Agent Decision Summary" and the decision-trace inspection path consume). The new field must be additive (existing trace consumers don't break) and serde-`#[serde(default)]` so existing serialized traces deserialize unchanged.
4. Live `GoalKind` under test: this annotation surface is goal-family-agnostic — any candidate emitted by `agent_tick/planning.rs` whose anchor lives in `ObservationOmissionLog` is annotated. The mechanism does not depend on a specific `GoalKind`.

## Architecture Check

1. The annotation is per-candidate-level (not per-tick or per-agent) — placed on `RootCandidateTrace` rather than a separate sink. This matches the spec's framing: "discards a synthesized root candidate because its anchor entity is in the agent's `ObservationOmissionLog`."
2. `Option<OmissionReason>` (rather than a new variant on `RootCandidateOutcome`) keeps the trace's existing surfaces unchanged. `None` is the common case (anchor was found in belief store); `Some(reason)` is the omission attribution.
3. Reads `ObservationOmissionLog` through ticket 002's `GoalBeliefView::observation_omission_log` accessor — no direct world reads from the planner's trace path. FND-26 alignment preserved.
4. **Phase distinction (per `docs/precision-rules.md` Rule 1)**: this annotation surfaces the *candidate emission / discard* phase, not ranking, plan search, or authoritative outcome. Ticket 004 covers the *revalidation* phase (post-search, when execution attempts to act). Both phases need attribution but they are distinct surfaces — `RootCandidateTrace.omitted_anchor` does not subsume `Discrepancy::Omission`.

## Verification Layers

1. `RootCandidateTrace` retains its existing serde round-trip with the new field added → focused unit test in `decision_trace.rs` cfg-test block.
2. Planner discards a candidate whose anchor is in `ObservationOmissionLog` and populates `omitted_anchor` with the typed reason → focused unit test in `agent_tick/planning.rs` cfg-test block (decision-trace assertion on `RootCandidateTrace.omitted_anchor`, per `docs/precision-rules.md` Rule 6 — decision-trace preference for AI reasoning behavior).
3. Existing decision-trace consumers (observer Section 8 etc.) still work unchanged with the new field defaulted to `None` for traces that don't trigger the omission path.

## What to Change

### 1. Add field to `RootCandidateTrace`

In `crates/worldwake-ai/src/decision_trace.rs:786`, add:

```rust
pub struct RootCandidateTrace {
    // ... existing 7 fields ...
    #[serde(default)]
    pub omitted_anchor: Option<OmissionReason>,
}
```

Update the `OmissionReason` import line at the top of the file (`use worldwake_core::OmissionReason;` or similar).

### 2. Wire candidate-discard path

In `crates/worldwake-ai/src/agent_tick/planning.rs`, locate the synthesized-root-candidate validation loop (in the same area as the snapshot construction at lines 537-546 modified by ticket 003). Where the planner currently filters out a candidate because its anchor is not in the agent's belief store:

1. Consult `view.observation_omission_log(agent)` (ticket 002's accessor) for the candidate's anchor entity.
2. If the anchor is in the log, populate `omitted_anchor: Some(entry.reason)` on the corresponding `RootCandidateTrace` entry.
3. The `RootCandidateTrace.outcome` remains "discarded" (or whatever the existing variant is) — `omitted_anchor` is the *attribution*, not a new outcome.
4. If the anchor is genuinely unknown (not in log either), leave `omitted_anchor: None` and the discard reason is not the omission path.

### 3. Update consumer hot-paths if needed

If any existing consumer of `RootCandidateTrace` pattern-matches exhaustively on the struct (`RootCandidateTrace { def_id, action_name, .. }`), add the new field. Most consumers should access fields by name, so this should be limited to potentially observer.rs Section 8 rendering.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify) — field addition at line 786
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify) — wiring at the candidate-discard site

## Out of Scope

- Reading `omitted_anchor` for observer rendering → ticket 006.
- Discrepancy revalidation wiring → ticket 004 (different surface; revalidation is post-search, this is candidate emission).
- Goldens → ticket 007.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --lib decision_trace` passes (struct round-trip with new field).
2. `cargo test -p worldwake-ai --lib agent_tick` passes (candidate-discard path populates `omitted_anchor`).
3. `cargo build --workspace` succeeds.

### Invariants

1. `RootCandidateTrace` deserializes from older traces (without the field) via `#[serde(default)]`.
2. `omitted_anchor` is `None` for any candidate whose anchor was found in the belief store.
3. `omitted_anchor` is `Some(reason)` exactly when (a) the anchor is missing from the belief store AND (b) the anchor is present in `ObservationOmissionLog`.
4. The planner's existing `RootCandidateOutcome` variants are unchanged — `omitted_anchor` is attribution, not a new outcome variant.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` cfg-test block — new test: `RootCandidateTrace` with `omitted_anchor: Some(OmissionReason::OverBudget { budget: 24, candidates_seen: 60 })` round-trips serde; deserializing a JSON snippet missing `omitted_anchor` yields `None` (verifies `#[serde(default)]`).
2. `crates/worldwake-ai/src/agent_tick/planning.rs` cfg-test block — new test: synthesize a root candidate whose anchor is in the agent's `ObservationOmissionLog`, run the validation loop, assert the resulting `RootCandidateTrace.omitted_anchor == Some(reason)` matching the log entry. Use the decision-trace assertion surface, not weaker downstream evidence (per `docs/precision-rules.md` Rule 6).

### Commands

1. `cargo test -p worldwake-ai --lib decision_trace`
2. `cargo test -p worldwake-ai --lib agent_tick`
3. `cargo build --workspace`
4. `./scripts/verify.sh`
