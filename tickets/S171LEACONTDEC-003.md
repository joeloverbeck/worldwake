# S171LEACONTDEC-003: Render learned-context attribution in decision-trace text

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — formatter-only changes in `crates/worldwake-ai/src/decision_trace.rs`. No ranking, planning, or world-state effect.
**Deps**: `archive/tickets/S171LEACONTDEC-001.md` (provides `LearnedOpportunityBonusAttribution`, `RepairMemoryBonusAttribution`, and the extended `SourceReliabilityDiscount` field surface that the new formatters render); `archive/tickets/S171LEACONTDEC-002.md` (populates the bonus attribution fields); `archive/tickets/S171LEACONTDEC-004.md` (populates source-reliability provenance from the matched `ReliabilityRecord` ring)

## Problem

After archived `archive/tickets/S171LEACONTDEC-001.md`, the attribution carriers exist on `RankedGoalSummary`; after S171LEACONTDEC-002 the two bonus carriers are populated when bonuses apply; after S171LEACONTDEC-004 source-reliability discount provenance is populated from the matched `ReliabilityRecord` ring when lawful event provenance exists — but the decision-trace text rendered for human inspection still doesn't show those attribution details. The existing `format_competition_discount_summary` (`decision_trace.rs:2429`) and `format_source_reliability_discount_summary` (`decision_trace.rs:2440`) suffix the trace with discount details; the two bonus axes have no equivalent formatter. The discount-formatter call sites at `decision_trace.rs:317-325`, `1794-1802`, and `2110` concatenate `_suffix` strings into the candidate's rendered summary. This ticket adds two new formatter functions in the same style, extends the existing `format_source_reliability_discount_summary` to render the already-defined provenance fields when non-zero, and wires both new formatters into the three suffix-concat sites alongside the existing discount renderers.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The existing formatters `format_competition_discount_summary` at `decision_trace.rs:2429` and `format_source_reliability_discount_summary` at `decision_trace.rs:2440` are module-private `fn` (not `pub`); their three call sites at `decision_trace.rs:319, 325, 1794, 1802, 2110` use the `.map_or_else(String::new, format_*_summary)` pattern on `selected_summary.<field>.as_ref()`. The new bonus-attribution formatters mirror this shape exactly. Observer (`crates/worldwake-cli/src/bin/observer.rs`) does not call these formatters directly — per the just-completed reassessment, observer reads `motive_source_contributions` only and consumes the rendered trace text through the existing dump pipeline; no observer code change is needed.
2. The three suffix-concat sites at `decision_trace.rs:317-325`, `1794-1802`, `2110` each build a `selected_summary.<field>.as_ref().map_or_else(String::new, format_<field>_summary)` binding and concatenate the resulting suffix into the trace text. Adding two new bindings per site is mechanical and follows the existing pattern.
3. Shared abstraction boundary under audit: the trace-text concatenation contract within `decision_trace.rs` — formatters return `String` (typically prefixed with `, learned_opp_bonus=…` or similar separator), the suffix is concatenated into the per-candidate summary, and the result is consumed by observer's dump pipeline unchanged in shape.

## Architecture Check

1. The two new formatters mirror the existing two-discount formatter style exactly (private `fn`, return `String`, called via `.map_or_else(String::new, ...)`); no new abstraction, no shared formatter trait. Per FND-3: concrete state over abstract scores — each attribution type gets its own concrete formatter rather than a generic Display-trait dispatch.
2. No backwards-compatibility shim: the new formatters are new symbols; the extended `format_source_reliability_discount_summary` adds new fields to its rendered output unconditionally when `provenance_event_count > 0`, with no parallel old-format path retained.

## Verification Layers

1. New formatters produce expected strings for representative input -> focused unit tests in `decision_trace.rs::tests` constructing `sample_learned_opportunity_bonus_attribution()` / `sample_repair_memory_bonus_attribution()` (from `archive/tickets/S171LEACONTDEC-001.md`'s D8 fixture additions) and asserting the formatter output contains the expected fields.
2. Extended `format_source_reliability_discount_summary` includes provenance fields when populated -> focused unit test asserts the output contains `provenance_event_count` and `most_recent_provenance_event` when they're non-zero / Some; omits them or renders as `0`/`None` when they're empty.
3. Suffix-concat sites at lines 317-325, 1794-1802, 2110 thread the new bindings -> existing decision-trace tests at `decision_trace.rs::tests` continue to pass; trace text now contains the new suffixes when attribution is `Some(_)`.
4. Single-layer ticket — formatter-only; no ranking, planning, or world-state effect. No mixed-layer mapping applies.

## What to Change

### 1. Add two new formatter functions

In `crates/worldwake-ai/src/decision_trace.rs`, alongside `format_competition_discount_summary` (line 2429) and `format_source_reliability_discount_summary` (line 2440):

```rust
fn format_learned_opportunity_bonus_summary(attribution: &LearnedOpportunityBonusAttribution) -> String {
    let source = match attribution.entry_source {
        LearnedOpportunitySource::Event(id) => format!("event={}", id.0),
        LearnedOpportunitySource::ReadPhaseInference => String::from("read-phase"),
    };
    format!(
        ", learned_opp(src={}, obs_t={}, exp_t={}, motive {}→{})",
        source,
        attribution.entry_observed_tick.0,
        attribution.entry_expires_tick.0,
        attribution.pre_bonus_motive,
        attribution.post_bonus_motive,
    )
}

fn format_repair_memory_bonus_summary(attribution: &RepairMemoryBonusAttribution) -> String {
    format!(
        ", repair_mem(sig={:?}, succ_count={}, exp_t={}, motive {}→{})",
        attribution.signature,
        attribution.entry_success_count,
        attribution.entry_expires_tick.0,
        attribution.pre_bonus_motive,
        attribution.post_bonus_motive,
    )
}
```

Exact field formatting may vary; the contract is that source-kind, observed/expires ticks, signature/success-count where applicable, and pre/post motive are all named.

### 2. Extend `format_source_reliability_discount_summary` to render provenance fields

In `crates/worldwake-ai/src/decision_trace.rs:2440-2449`, extend the existing formatter so that when `discount.provenance_event_count > 0`, the rendered string includes `prov_count={count}, most_recent_event={id}` (or `most_recent_event=None` when `most_recent_provenance_event` is None despite a non-zero count — defensive against ring corruption). When `provenance_event_count == 0`, omit the provenance subfields entirely to keep the existing rendered shape unchanged for pre-S171 fixture data.

### 3. Wire new bindings into three suffix-concat sites

At `crates/worldwake-ai/src/decision_trace.rs:317-325`, alongside the existing `source_reliability_suffix` and `competition_suffix` bindings, add:

```rust
let learned_opp_bonus_suffix = selected_summary
    .and_then(|summary| summary.learned_opportunity_bonus.as_ref())
    .map_or_else(String::new, format_learned_opportunity_bonus_summary);
let repair_memory_bonus_suffix = selected_summary
    .and_then(|summary| summary.repair_memory_bonus.as_ref())
    .map_or_else(String::new, format_repair_memory_bonus_summary);
```

Append both suffixes to the concatenated trace text at the same point that the existing `source_reliability_suffix` and `competition_suffix` are appended. Repeat the same binding-and-append pattern at the two other suffix-concat sites: `decision_trace.rs:1794-1802` and `decision_trace.rs:2110`.

### 4. Focused tests for the new formatters

Add tests in `decision_trace.rs::tests` (alongside the existing formatter tests):

- `format_learned_opportunity_bonus_summary_renders_event_source_form` — calls the formatter with a sample carrying `Event(EventId(N))`; asserts the output contains `event=N`.
- `format_learned_opportunity_bonus_summary_renders_read_phase_form` — calls the formatter with `ReadPhaseInference`; asserts `read-phase`.
- `format_repair_memory_bonus_summary_renders_signature_and_succ_count` — calls the formatter with a sample; asserts both fields appear.
- `format_source_reliability_discount_summary_includes_provenance_when_non_zero` — calls the extended formatter with `provenance_event_count > 0` and `most_recent_provenance_event = Some(EventId(N))`; asserts both subfields appear in the output. Negative case: `provenance_event_count == 0` produces output identical to the pre-S171 rendered shape.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify) — two new formatters, one extended formatter, three suffix-concat-site extensions, four focused tests

## Out of Scope

- Any ranking-layer change (D5+D6 — landed by S171LEACONTDEC-002).
- Type or struct shape changes (D1-D4 — landed by `archive/tickets/S171LEACONTDEC-001.md`).
- Observer/CLI code changes — observer renders the trace text unchanged through the existing dump pipeline.
- Trace-format documentation or external trace-consumer migration — the rendered shape gains suffixes but the overall trace contract is forward-compatible.

## Acceptance Criteria

### Tests That Must Pass

1. New focused tests for the two new formatters and the extended `format_source_reliability_discount_summary` pass.
2. Existing tests in `crates/worldwake-ai/src/decision_trace.rs::tests` that assert rendered trace text continue to pass (the new suffixes only appear when the corresponding attribution is `Some(_)`; pre-S171 fixture data renders unchanged).
3. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. New formatters render their attribution fields with the contract: source-kind (for learned-opp), signature + success-count (for repair-memory), pre/post motive (both), observed/expires ticks (both).
2. Extended `format_source_reliability_discount_summary` renders provenance subfields only when `provenance_event_count > 0`; the zero-count form produces output byte-identical to the pre-S171 shape.
3. The three suffix-concat sites at `decision_trace.rs:317-325, 1794-1802, 2110` consistently thread both new bindings; no site is silently skipped.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs::tests::format_learned_opportunity_bonus_summary_renders_event_source_form` (new)
2. `crates/worldwake-ai/src/decision_trace.rs::tests::format_learned_opportunity_bonus_summary_renders_read_phase_form` (new)
3. `crates/worldwake-ai/src/decision_trace.rs::tests::format_repair_memory_bonus_summary_renders_signature_and_succ_count` (new)
4. `crates/worldwake-ai/src/decision_trace.rs::tests::format_source_reliability_discount_summary_includes_provenance_when_non_zero` (new)

### Commands

1. `cargo test -p worldwake-ai -- decision_trace::tests::format_`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh` — confirms fmt/clippy/full-workspace gates before opening the PR.
