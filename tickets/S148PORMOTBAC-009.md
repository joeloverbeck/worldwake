# S148PORMOTBAC-009: Observer Decision History rendering for slot, motives, claims, conditions

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No — observer-only changes to `crates/worldwake-cli/src/bin/observer.rs` Decision History section; renders the new five-slot winners and the enriched `IntentionFrame` field set
**Deps**: `S148PORMOTBAC-004`, `S148PORMOTBAC-006`, `S148PORMOTBAC-007`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

The observer Decision History section at `crates/worldwake-cli/src/bin/observer.rs:933+` currently renders `IntentionFrame` state minimally (committed goal + agent + payload summary in a markdown table). After tickets 004, 006, 007 land the five-slot portfolio, the enriched `IntentionFrame` (with `motive_refs`, `resume_conditions`, `abandon_conditions`, `explicit_claims`, `causal_links`), and the typed `Discrepancy::AbandonConditionFired` variant, the observer must surface this new state per spec D11 — otherwise operators inspecting decision traces cannot see *why* an intention was adopted, *what holds it together*, or *what conditions revise it*.

## Assumption Reassessment (2026-05-17)

1. Observer Decision History section starts at `crates/worldwake-cli/src/bin/observer.rs:933` with a markdown table header at line 940-941: `| Tick | Agent | Event | Payload Summary |`. Existing format conventions: motive sources rendered via `format_motive_source_ref` helper at line 1194 producing strings like `Variant(payload)`; "motive sources:" label at line 1161. Per-slot rendering and `IntentionFrame`-field surfacing do not yet exist.
2. Spec S148 D11 specifies the new rendering shape:
   ```
   Committed: BakeBread for Granger (Slot: EconomicOpportunity, weight 600)
     Motives:
       - Greed(SaleOpportunity:bread_lot_42) introduced t=412
       - NeedPressure(Hunger) introduced t=420
     Claims:
       - ContentionGrant#127 (oven queue)
       - SaleListing on bread_lot_42
     Resume on: OpportunityVisible(grain_supply_at_market)
     Abandon if:
       - MotiveSourceLost(NeedPressure)
       - ArtifactLegalEffectLost(bread_lot_42)
   ```
3. Shared abstraction under audit: the observer's per-tick decision-rendering surface. This ticket extends the existing markdown-table-plus-prose rendering — no new output channel, no schema change to `ScenarioDiagnosticsReport` (the existing `GoalPressureMetrics.candidates_emitted_by_slot: BTreeMap<SlotKind, u64>` at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs:27` already keys by `SlotKind` and automatically reflects the renamed five-variant set from ticket 001 — no D11-side modification needed).
4. Tooling-only ticket: no engine changes, no simulation state mutations. Per `tickets/_TEMPLATE.md` Verification Layers guidance for tooling-only specs, focused unit tests on the rendering helper functions are the primary proof surface.

## Architecture Check

1. The rendering extension is purely additive at the prose level: an existing rendering site gains new sub-bullets for slot, motives, claims, resume/abandon conditions. The observer's read-only consumer status is preserved per the Read-Only Tooling Consumer pattern in `references/worldwake-validation-patterns.md`.
2. Format conventions match existing observer style: indented sub-bullets follow the pattern already used in the perception/decision sections; the `format_motive_source_ref` helper at line 1194 is reused for motive rendering; the human-readable artifact-ID format (e.g., `ContentionGrant#127`) follows existing observer conventions for entity references.
3. No new lifetime-bound types stored as struct fields; no nonexistent shortcut accessors invented (`AgentDecisionRuntime.operating_mode` is read through the existing `AgentTickDriver::runtime(agent)` accessor at `crates/worldwake-ai/src/agent_tick/mod.rs`).

## Verification Layers

1. Rendering helper correctness → focused unit tests on the new rendering functions: each helper takes a fixture frame/portfolio/runtime and asserts the produced string matches the spec D11 format
2. Tooling smoke verification → headless observer invocation against a fixture scenario, assert the rendered output contains expected per-slot lines and per-intention sub-bullets (proof that the rendering integrates correctly with the live trace stream)

## What to Change

### 1. Extend `IntentionFrame` rendering

In `crates/worldwake-cli/src/bin/observer.rs` near the existing Decision History section (line 933+), add a rendering helper:

```rust
fn render_intention_frame_extended(
    frame: &IntentionFrame,
    slot: SlotKind,
    weight: Permille,
    out: &mut String,
) {
    let _ = writeln!(
        out,
        "Committed: {} for {} (Slot: {:?}, weight {})",
        format_goal_key(&frame.goal),
        format_agent(frame),
        slot,
        weight.value()
    );
    if !frame.motive_refs.is_empty() {
        writeln!(out, "  Motives:").unwrap();
        for motive_ref in &frame.motive_refs {
            let _ = writeln!(
                out,
                "    - {} introduced t={}",
                format_motive_source_ref(motive_ref),
                motive_ref.introduced_tick.0
            );
        }
    }
    if !frame.explicit_claims.is_empty() {
        writeln!(out, "  Claims:").unwrap();
        for claim_id in &frame.explicit_claims {
            let _ = writeln!(out, "    - {}", format_artifact_claim(*claim_id));
        }
    }
    for cond in &frame.resume_conditions {
        let _ = writeln!(out, "  Resume on: {}", format_resume_condition(cond));
    }
    if !frame.abandon_conditions.is_empty() {
        writeln!(out, "  Abandon if:").unwrap();
        for cond in &frame.abandon_conditions {
            let _ = writeln!(out, "    - {}", format_abandon_condition(cond));
        }
    }
}
```

(Method/function names like `format_goal_key`, `format_agent`, `format_artifact_claim` exist or are added inline at implementation time — verify against the actual observer module structure.)

### 2. Add format helpers for resume and abandon conditions

```rust
fn format_resume_condition(cond: &IntentionResumeCondition) -> String { /* per-variant render */ }
fn format_abandon_condition(cond: &IntentionAbandonCondition) -> String { /* per-variant render */ }
fn format_artifact_claim(entity_id: EntityId) -> String { /* peek at component types to label */ }
```

The artifact claim formatter examines which component the entity carries (`ContentionGrant`, `SaleListing`, `ArtifactHeader`) and labels accordingly — `ContentionGrant#<id>`, `SaleListing on <commodity>`, `Bounty notice <id>`. Use the read-only accessors named in the Read-Only Tooling Consumer pattern (`World::get_component_*`).

### 3. Wire the rendering into the existing Decision History loop

At the existing per-tick iteration that produces the markdown table at observer.rs:933+, add a follow-up block after each row that has a committed intention — invoke `render_intention_frame_extended` to produce the indented sub-bullets. Operating mode (read via `runtime.operating_mode`) can be surfaced once per-tick at the section header (e.g., `Tick 412 [Mode: Emergency]`) so the per-intention rendering doesn't need to repeat it.

### 4. Per-slot section header

The Decision History section already renders agents per-tick; extend the per-tick rendering to include a per-slot winner summary using the `Portfolio` produced by `assemble_portfolio` (read through the existing trace surface — `DecisionTraceSink` carries the portfolio per `crates/worldwake-ai/src/decision_trace.rs`).

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — add `render_intention_frame_extended`, `format_resume_condition`, `format_abandon_condition`, `format_artifact_claim` helpers; wire into existing Decision History section at line 933+; per-slot header at the per-tick boundary)

## Out of Scope

- Modifying `GoalPressureMetrics.candidates_emitted_by_slot` (no schema change — the existing `BTreeMap<SlotKind, u64>` field already reflects the renamed five-variant set after ticket 001 lands)
- Engine-side changes to the decision trace payload (`crates/worldwake-ai/src/decision_trace.rs` may need new struct fields if the existing trace surface doesn't already carry the enriched `IntentionFrame` fields — that addition belongs in ticket 007's scope; this ticket consumes whatever the trace surface exposes)
- Adding new operating-mode visualization beyond the per-tick header label
- Refactoring existing observer helpers to share logic with the new ones (defer to follow-up cleanup if warranted)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli observer::tests::render_intention_frame_*` — new focused tests on each helper assert the rendered string matches the spec D11 format
2. Tooling smoke test: build the observer binary, run against `scenarios/cli-evaluation.ron` (or a similar fixture exercising committed intentions), grep the output for expected per-slot lines (`Slot: NeedSurvival`, `Slot: ObligationDuty`, etc.)
3. Existing suite: `cargo test --workspace`
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The observer renders every `SlotKind` variant by its canonical name when that slot has a winner.
2. The observer renders every populated `IntentionFrame` field (`motive_refs`, `explicit_claims`, `resume_conditions`, `abandon_conditions`) without dropping any; empty vectors are skipped silently rather than producing blank sub-bullets.
3. The observer never reads non-co-located belief state on the agent's behalf — all reads use the read-only accessor surface named in the Read-Only Tooling Consumer pattern.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — extend the existing observer `#[cfg(test)]` block (if present) or add inline tests for the new helpers: `render_intention_frame_extended_full_population`, `render_intention_frame_extended_empty_vectors_skip_subsections`, `format_resume_condition_per_variant`, `format_abandon_condition_per_variant`, `format_artifact_claim_dispatches_by_component`

### Commands

1. `cargo test -p worldwake-cli observer`
2. `cargo run --bin observer -- --scenario scenarios/cli-evaluation.ron --ticks 50 | grep -E "Slot:|Motives:|Claims:|Resume on:|Abandon if:"` — tooling smoke check
3. `./scripts/verify.sh`
