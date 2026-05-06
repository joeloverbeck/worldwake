# S135PLAPERBUD-005: RootCandidateTrace omitted_anchor annotation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — decision-trace surface, planner candidate-discard wiring
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`, `archive/tickets/S135PLAPERBUD-002.md`

## Problem

Per S135 D4, when the planner traces a synthesized root candidate whose anchor entity is absent from the planning snapshot and present in the agent's `ObservationOmissionLog`, the decision trace must carry the typed reason so observer reports and goldens can answer "why did this agent ignore the dragon next to them?" This ticket adds the `omitted_anchor: Option<OmissionReason>` field to `RootCandidateTrace` and wires the root candidate trace path to populate it.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RootCandidateTrace` lives at `crates/worldwake-ai/src/decision_trace.rs:786` with 7 existing fields (`def_id: ActionDefId`, `action_name: String`, `op_kind: Option<PlannerOpKind>`, `authoritative_targets: Vec<EntityId>`, `planner_only: bool`, `payload_status: RootCandidatePayloadStatus`, `outcome: RootCandidateOutcome`). Live reassessment found it is an in-memory decision-trace carrier (`Clone, Debug, Eq, PartialEq`), not a serde-persisted carrier, so this ticket does not add `#[serde(default)]` or serde round-trip proof.
2. Synthesized root candidate emission and root trace construction live in `crates/worldwake-ai/src/search/candidates.rs` (`goal_synthesized_candidates`, `root_candidate_trace_from_candidate`, and `search_candidates_with_expansion_trace`). `agent_tick/planning.rs` only builds the snapshot and invokes search. Candidate discard for a missing anchor is currently silent — the root candidate trace records the filtered/skipped outcome without typed omission attribution.
3. Shared abstraction boundary under audit: `RootCandidateTrace` (the typed in-memory surface that planner diagnostics and downstream observer/report rendering consume). The new field is additive for Rust constructors and trace consumers; older serialized trace compatibility is not applicable because this carrier is not serialized on the live branch.
4. Live `GoalKind` under test: this annotation surface is goal-family-agnostic — any root candidate traced through `search/candidates.rs` whose anchor is absent from the planning snapshot and present in `ObservationOmissionLog` is annotated. The focused proof uses `ShareBelief` only as a representative synthesized root candidate.

## Architecture Check

1. The annotation is per-candidate-level (not per-tick or per-agent) — placed on `RootCandidateTrace` rather than a separate sink. This matches the spec's D4 root-boundary framing while binding the live check to planning-snapshot absence plus `ObservationOmissionLog` attribution.
2. `Option<OmissionReason>` (rather than a new variant on `RootCandidateOutcome`) keeps the trace's existing surfaces unchanged. `None` is the common case (anchor was found in belief store); `Some(reason)` is the omission attribution.
3. Reads `ObservationOmissionLog` through ticket 002's `GoalBeliefView::observation_omission_log` accessor — no direct world reads from the planner's trace path. FND-26 alignment preserved.
4. **Phase distinction (per `docs/precision-rules.md` Rule 1)**: this annotation surfaces the *candidate emission / discard* phase, not ranking, plan search, or authoritative outcome. Ticket 004 covers the *revalidation* phase (post-search, when execution attempts to act). Both phases need attribution but they are distinct surfaces — `RootCandidateTrace.omitted_anchor` does not subsume `Discrepancy::Omission`.

## Verification Layers

1. `RootCandidateTrace` constructors and sample trace fixtures include the new field with `None` for ordinary candidates.
2. Planner search traces a candidate whose anchor is absent from the planning snapshot and present in `ObservationOmissionLog`, populating `omitted_anchor` with the typed reason → focused unit test in `search/tests.rs` using the root decision-trace assertion surface, per `docs/precision-rules.md` Rule 6.
3. Existing decision-trace consumers (observer Section 8 etc.) still work unchanged with the new field defaulted to `None` for traces that don't trigger the omission path.

## What to Change

### 1. Add field to `RootCandidateTrace`

In `crates/worldwake-ai/src/decision_trace.rs:786`, add:

```rust
pub struct RootCandidateTrace {
    // ... existing 7 fields ...
    pub omitted_anchor: Option<OmissionReason>,
}
```

Update the `OmissionReason` import line at the top of the file (`use worldwake_core::OmissionReason;` or similar).

### 2. Wire root candidate trace path

In `crates/worldwake-ai/src/search/candidates.rs`, locate the root trace construction path (`search_candidates_with_expansion_trace` and `root_candidate_trace_from_candidate`). When a root candidate's anchor entity is absent from the planning snapshot and present in the actor's `ObservationOmissionLog`:

1. Consult `GoalBeliefView::observation_omission_log(&state, agent)` (ticket 002's accessor) for the candidate's anchor entity.
2. If the anchor is in the log, populate `omitted_anchor: Some(entry.reason)` on the corresponding `RootCandidateTrace` entry.
3. The `RootCandidateTrace.outcome` remains "discarded" (or whatever the existing variant is) — `omitted_anchor` is the *attribution*, not a new outcome.
4. If the anchor is genuinely unknown (not in log either), leave `omitted_anchor: None` and the discard reason is not the omission path.

### 3. Update consumer hot-paths if needed

If any existing consumer of `RootCandidateTrace` pattern-matches exhaustively on the struct (`RootCandidateTrace { def_id, action_name, .. }`), add the new field. Most consumers should access fields by name, so this should be limited to potentially observer.rs Section 8 rendering.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify) — field addition at line 786
- `crates/worldwake-ai/src/search/candidates.rs` (modify) — wiring at the root trace construction/filter path
- `crates/worldwake-ai/src/search/tests.rs` (modify) — focused root decision-trace coverage

## Out of Scope

- Reading `omitted_anchor` for observer rendering → ticket 006.
- Discrepancy revalidation wiring → ticket 004 (different surface; revalidation is post-search, this is candidate emission).
- Goldens → ticket 007.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --lib decision_trace` passes (sample fixtures and constructors compile with the new field).
2. `cargo test -p worldwake-ai --lib search` passes (root decision-trace path populates `omitted_anchor`).
3. `cargo build --workspace` succeeds.

### Invariants

1. `omitted_anchor` is `None` for any candidate whose anchor was found in the planning snapshot.
2. `omitted_anchor` is `Some(reason)` exactly when (a) the anchor is missing from the planning snapshot AND (b) the anchor is present in `ObservationOmissionLog`.
3. The planner's existing `RootCandidateOutcome` variants are unchanged — `omitted_anchor` is attribution, not a new outcome variant.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — new test: synthesize a root candidate whose anchor is absent from the planning snapshot but present in the actor's `ObservationOmissionLog`, run root search with expansion summaries, and assert the resulting `RootCandidateTrace.omitted_anchor == Some(reason)` matching the log entry. Use the decision-trace assertion surface, not weaker downstream evidence (per `docs/precision-rules.md` Rule 6).

### Commands

1. `cargo test -p worldwake-ai --lib decision_trace`
2. `cargo test -p worldwake-ai --lib search`
3. `cargo build --workspace`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-06.

- Added `RootCandidateTrace.omitted_anchor: Option<OmissionReason>` to the in-memory root decision-trace carrier.
- Wired root candidate trace construction in `search/candidates.rs` to read the actor's `ObservationOmissionLog` through `GoalBeliefView` and annotate a candidate when its anchor is absent from the planning snapshot but present in the omission log.
- Extended the `worldwake-ai` search test harness with an agent belief-store fixture and a focused `ShareBelief` synthesized-root test proving `RootCandidateTrace.omitted_anchor == Some(reason)` at the root expansion summary.
- Truth-synced `archive/specs/S135-planner-perception-budget.md` D4 to the live in-memory trace boundary.

## Deviations

- The drafted serde/default proof was removed during reassessment. Live `RootCandidateTrace` is not serde-persisted, so no `#[serde(default)]`, save-format bump, or serde round-trip test applies.
- The drafted wiring location was corrected from `agent_tick/planning.rs` to `search/candidates.rs`; `agent_tick/planning.rs` builds the snapshot and invokes search, while root candidate trace construction happens in the search module.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib search_trace_annotates_root_candidate_with_omitted_anchor_reason -- --list` (confirmed one focused test id).
- Passed `cargo test -p worldwake-ai --lib search::tests::search_trace_annotates_root_candidate_with_omitted_anchor_reason -- --exact`.
- Passed `cargo test -p worldwake-ai --lib decision_trace`.
- Passed `cargo test -p worldwake-ai --lib search`.
- Passed `cargo build --workspace`.
- Passed `./scripts/verify.sh`, which ran `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
