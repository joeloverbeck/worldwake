# S141MOTSOULED-006: Observer Section 3b motive-source rendering

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No — observer-only (CLI binary)
**Deps**: `archive/tickets/S141MOTSOULED-003.md` (reads `RankedGoalSummary.motive_source_contributions`), `archive/tickets/S141MOTSOULED-005.md` (reads `GoalCommittedPayload.decisive_motive_sources`)

## Problem

S141's debuggability contract (FND-29) requires the per-commit causal answer to be inspectable from observer output alone — "Agent A committed `Eat` because of `NeedPressure(Hunger)` weighted 750 against pressure 950, plus `Greed(market_opportunity#42)` weighted 500 against opportunity_score 420". Today's observer Section 3b (Decision History, `crates/worldwake-cli/src/bin/observer.rs:833`) renders `GoalCommitted` events with the aggregate `motive_score` but not the per-source breakdown.

This ticket extends the existing Section 3b `GoalCommitted` rendering with the motive-source contributions read from the payload (per 005) and the per-source weighting metadata read from `RankedGoalSummary` (per 003).

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Section 3b — Decision History exists at `crates/worldwake-cli/src/bin/observer.rs:833` and already renders `GoalCommitted` events with the aggregate `motive_score`. The Section 3a (Opportunities) format at line 706 establishes the existing `→` and `(label=value, …)` formatting conventions this ticket extends. Existing focused/unit tests in `observer.rs#[cfg(test)]` at lines 6582, 6683–6686 already assert the presence of "## Section 3b — Decision History" in the output — those assertions are unchanged by this ticket.
2. Both source surfaces are wired by sibling tickets: `GoalCommittedPayload.decisive_motive_sources` (event payload, written at commit time by 005) and `RankedGoalSummary.motive_source_contributions` (decision trace, populated by `archive/tickets/S141MOTSOULED-004.md` via 003's field declaration). Observer reads decision events from `EventLog` and per-candidate decision-trace state from the installed `DecisionTraceSink` per the existing observer wiring; both are already plumbed.
3. Shared abstraction boundary: observer is a read-only tooling consumer per the worldwake-validation-patterns "Read-Only Tooling Consumer" pattern. It calls existing public APIs to read decision events and decision-trace state. No new accessor methods on world/sim are introduced.
4. Per `docs/precision-rules.md` Rule 5 (verification surface mapping): observer rendering is the downstream-most surface; the ticket's proof is the rendered text contents. The upstream contracts (payload contents per 005, trace contents per `archive/tickets/S141MOTSOULED-004.md`) are verified at their owning ticket boundaries.
5. Format-fidelity check (per `references/codebase-validation.md` 3.3A): the spec's example format uses `→` for contribution arrows and `(weight=NNN, pressure=MMM)` for metadata — these match Section 3a's existing convention. The spec's `(motive NNNNN)` aggregate-score parenthetical is already what the existing line renders.

## Architecture Check

1. Reading from existing surfaces (event payload + decision trace) without re-deriving the breakdown in the observer keeps the observer a thin read-only consumer (FND-26 + the Read-Only Tooling Consumer pattern). The alternative — recomputing per-source contributions in the observer from `offer.motive_sources` + `RankingContext` — would duplicate the scoring math and risk drift between observer and ranking arithmetic.
2. The added rendering is purely additive prose appended below the existing `GoalCommitted` line. Existing Section 3b assertions and downstream consumers of the observer dump are unaffected by the additional lines.
3. No backward-compat handling needed — observer rendering is not part of the authoritative simulation; format changes are free.

## Verification Layers

1. Rendered output contents → focused unit test in `crates/worldwake-cli/src/bin/observer.rs#[cfg(test)]` constructing a synthetic `GoalCommittedPayload` with `decisive_motive_sources` populated and a synthetic `RankedGoalSummary` with `motive_source_contributions` populated, then rendering Section 3b and asserting the expected lines appear.
2. End-to-end observer integration → existing Section 3b assertion (line 6582) still passes; new assertions added in the same test confirm motive-source lines appear when a committed goal carries non-empty motive sources.
3. Single-layer ticket — only the rendering layer is modified. Upstream contracts (payload, trace) are verified at 003/004/005 ticket boundaries.

## What to Change

### 1. Extend Section 3b `GoalCommitted` rendering at `observer.rs:833`+

Locate the existing per-event rendering block inside Section 3b (the code path that writes `"Tick NNN — Agent X — GoalCommitted: …"` to the output buffer). After the existing aggregate line, append:

```
  motive sources:
    <SourceVariant>(<Anchor>) → <contribution> (<weight_label>=<NNN>, <strength_label>=<MMM>)
    <SourceVariant>(<Anchor>) → <contribution> (<weight_label>=<NNN>, <strength_label>=<MMM>)
    ...
```

Where each line corresponds to one entry in the joined `(decisive_motive_sources × motive_source_contributions)` view. The contribution value is read from `RankedGoalSummary.motive_source_contributions`; the weight/strength labels are derived per-`MotiveSource`-variant for human readability (e.g., `NeedPressure` → `need_weight`, `pressure`; `Greed` → `greed_weight`, `opportunity_score`).

Format conventions:
- `→` (U+2192) between source and contribution — matches Section 3a's existing convention at observer.rs:706+.
- `(label=value, label=value)` parenthetical for per-source metadata — matches the existing aggregate `(motive NNNNN)` parenthetical convention used in the existing Section 3b line.
- Two-space leading indent on the `motive sources:` header; four-space leading indent on per-source lines.

### 2. Add a focused unit test

In `observer.rs#[cfg(test)]`, add a test (e.g., `section_3b_renders_motive_source_contributions`) that:
- Constructs a `GoalCommittedPayload` with `decisive_motive_sources: vec![MotiveSourceRef { source: MotiveSource::NeedPressure { need: HomeostaticNeedId::Hunger }, introduced_tick: Tick(412) }]`.
- Constructs a `RankedGoalSummary` with `motive_source_contributions: vec![(the same MotiveSourceRef, 14200)]`.
- Renders the relevant decision-history block.
- Asserts the rendered string contains both `"motive sources:"` and `"NeedPressure(Hunger) → 14200"`.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — Section 3b rendering + focused unit test)

## Out of Scope

- `RankedGoalSummary.motive_source_contributions` field declaration — owned by `archive/tickets/S141MOTSOULED-003.md`.
- Population of `motive_source_contributions` — owned by `archive/tickets/S141MOTSOULED-004.md`.
- `GoalCommittedPayload.decisive_motive_sources` field and commit-time emission — owned by 005.
- Any new observer section, header, or section numbering change — Section 3b is the existing home for `GoalCommitted` rendering per the S141 reassessment's I6 finding. Sections 4 (Anomaly Flags), 5+ remain unchanged.
- Golden scenarios that exercise the rendering end-to-end — owned by 007's `golden_motive_sources.rs` suite, which also includes "observer renders both" assertions per spec D8 scenario 2.

## Acceptance Criteria

### Tests That Must Pass

1. `section_3b_renders_motive_source_contributions` (new focused test) — synthetic payload + trace produces the expected rendered lines.
2. Existing Section 3b assertions (`observer.rs` lines 6582, 6683–6686) continue to pass — confirms the additive change doesn't break the existing header presence assertion.
3. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. Per-source rendering is purely additive prose below the existing `GoalCommitted` aggregate line — Section 3a, Section 4+, and the existing Section 3b header are untouched.
2. Format conventions match existing observer patterns (`→`, `(label=value, …)` parenthetical, indent levels) — verified by inspection against `observer.rs:706+` (Section 3a) during reassessment.
3. The observer does not re-derive per-source contributions from offer + ranking context — it consumes the values surfaced by 003 and 005.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs#[cfg(test)]` — new focused unit test for Section 3b motive-source rendering.

### Commands

1. `cargo test -p worldwake-cli observer`
2. `cargo test -p worldwake-cli`
3. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
