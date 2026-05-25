# S171LEACONTDEC-002: Thread learned-context attribution through ranking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/ranking.rs` (return-shape changes on `learned_opportunity_bonus`, `repair_memory_bonus`, and `memory_motive_bonus`; new attribution threading in the `AgendaEntry::pending` construction path; extended `apply_source_reliability_discount` body)
**Deps**: S171LEACONTDEC-001 (provides `LearnedOpportunityBonusAttribution`, `RepairMemoryBonusAttribution`, and the extended `SourceReliabilityDiscount` field surface)

## Problem

After S171LEACONTDEC-001 the attribution types and field surfaces exist on `RankedGoalSummary` and `SourceReliabilityDiscount`, but they are populated as `None` / `0` everywhere — the attribution chain from a learned-store mutation (provenance recorded by S170) to a ranking-time decision is still broken. This ticket wires the two bonus functions in `ranking.rs` to return both the bonus value AND a populated attribution carrier, threads those carriers into the `AgendaEntry::pending` construction at `ranking.rs:290-301`, and extends `apply_source_reliability_discount` (`ranking.rs:492-510`) to populate the new `SourceReliabilityDiscount` provenance fields from the matched `TestimonyReliabilityEntry::provenance_events` ring buffer. After this ticket lands, every ranking decision that consults a learned-state entry records *which* entry it consulted — closing the FND-22A "experience path" gap that the spec's central problem statement names.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `learned_opportunity_bonus` at `ranking.rs:439-460` and `repair_memory_bonus` at `ranking.rs:413-437` both currently return `u32` and are called only from `memory_motive_bonus` at `ranking.rs:397-411` via `.saturating_add()` chain — the integer return is consumed immediately at one site, so changing the shape to `(u32, Option<…>)` is a local refactor with one caller to update. Existing tests `repair_memory_boosts_matching_alternative_only_while_live` (line 5826) and `learned_opportunity_memory_boosts_matching_opportunity_only_while_live` (line 5883) assert bonus behavior today; they will be extended in this ticket to assert the new attribution structure per V1.
2. `apply_source_reliability_discount` at `ranking.rs:492-510` already binds the matched `TestimonyReliabilityEntry` as `record` at line 504 via `source_reliability.sources.get(&SourceKey {…})?`; `record.provenance_events` (per `TestimonyReliabilityEntry` definition at `crates/worldwake-core/src/testimony_reliability.rs:20-62`) is in scope at the `SourceReliabilityDiscount` construction site and can be read directly with no new lookup required.
3. The `AgendaEntry::pending` construction at `ranking.rs:290-301` already has local bindings `source_reliability_discount` (line 277), `competition_discount` (line 282), and `provenance` (line 273) in scope — the new bonus-attribution fields populate from this same site by extending the `memory_motive_bonus` call to return its tupled attributions alongside the integer.
4. Shared abstraction boundary under audit: the `RankedGoalSummary` per-decision trace record produced by `ranking.rs:290-301` and consumed by `decision_trace.rs` formatter sites (rendered separately in S171LEACONTDEC-003) and by `observer.rs:1207-1213` (reads `motive_source_contributions` only; unaffected). The contract is that score arithmetic is byte-identical pre/post; only the trace surface gains data.
5. Per FND-22A: this ticket closes the spec's named gap — the experience path from learned-store mutation event to ranking-time consumption. The store-side provenance was added by S170; this ticket adds the consumption-side trace. No new learned state is introduced.
6. AI regression layer: candidate-generation/ranking focused/unit coverage. `agent_tick` and golden E2E remain unaffected (V3 — no new goldens needed; score arithmetic unchanged).
13. No adjacent contradictions exposed by reassessment — the change is bounded to ranking.rs internals and the function-signature impact is local to one caller (`memory_motive_bonus`).

## Architecture Check

1. The return-shape change `u32` → `(u32, Option<LearnedOpportunityBonusAttribution>)` keeps the bonus integer at tuple-position-0 so `memory_motive_bonus`'s `.saturating_add()` chain still composes cleanly — the attribution carriers flow alongside the integers rather than wrapping them. Score arithmetic is mechanically unchanged: `memory_motive_bonus` continues to sum the two integers; the two attribution carriers are forwarded as new return values to the `AgendaEntry::pending` site.
2. No backwards-compatibility shim or dual-API split — `learned_opportunity_bonus` and `repair_memory_bonus` are private (`fn` not `pub fn`) per `ranking.rs:439, 413`, so the signature change is fully contained within the crate. No external callers exist to migrate.

## Verification Layers

1. Score arithmetic is byte-identical pre/post -> existing focused tests at `ranking.rs:6418-6805` (the 7 `source_reliability_discount_*` tests), `ranking.rs:5826`, `ranking.rs:5883` continue to pass without modification. This is the spec's V2 contract realized.
2. Attribution carriers populate when bonuses apply -> extended focused tests in `ranking.rs` assert `Some(_)` with `pre_bonus_motive + bonus == post_bonus_motive` for both learned-opportunity and repair-memory paths (V1.1, V1.2).
3. `SourceReliabilityDiscount.provenance_event_count` and `.most_recent_provenance_event` populate from the matched `TestimonyReliabilityEntry::provenance_events` ring -> extended focused tests in `ranking.rs` assert the count matches `record.provenance_events.len()` and the most-recent event id matches the ring's last entry (V1.3).
4. Single-layer ticket at the ranking-substrate boundary — decision-trace rendering of the new attribution text is deferred to S171LEACONTDEC-003 and not asserted here.

## What to Change

### 1. Return-shape change for `learned_opportunity_bonus` and `repair_memory_bonus`

In `crates/worldwake-ai/src/ranking.rs`:

```rust
// Was: fn learned_opportunity_bonus(...) -> u32
fn learned_opportunity_bonus(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    base_motive: u32,
) -> (u32, Option<LearnedOpportunityBonusAttribution>) {
    let opportunity = OpportunityKey {
        goal_key: candidate.key,
        anchor: candidate.anchor,
    };
    let Some(entry) = context.learned_opportunity_memory.opportunities.get(&opportunity) else {
        return (0, None);
    };
    if entry.expires_tick <= context.current_tick {
        return (0, None);
    }
    let bonus = (base_motive / 20).max(1);
    let attribution = LearnedOpportunityBonusAttribution {
        opportunity,
        entry_source: entry.source,
        entry_observed_tick: entry.observed_tick,
        entry_expires_tick: entry.expires_tick,
        pre_bonus_motive: base_motive,
        post_bonus_motive: base_motive.saturating_add(bonus),
    };
    (bonus, Some(attribution))
}
```

Symmetrically for `repair_memory_bonus` (lines 413-437): return `(u32, Option<RepairMemoryBonusAttribution>)`; populate `signature`, `entry_success_count`, `entry_expires_tick`, `pre_bonus_motive`, `post_bonus_motive`.

### 2. Update `memory_motive_bonus` to thread attributions

In `crates/worldwake-ai/src/ranking.rs:397-411`, change the return type to `(u32, Option<LearnedOpportunityBonusAttribution>, Option<RepairMemoryBonusAttribution>)`. Compute the integer as the existing `saturating_add` chain; forward both attribution carriers to the caller. The bonus integer remains the sum of the two bonuses, exactly as today.

### 3. Thread attributions into `RankedGoalSummary` at the `AgendaEntry::pending` site

At `crates/worldwake-ai/src/ranking.rs:290-301`, `memory_motive_bonus` is called as part of building each candidate's ranked summary. Extend the call to destructure the new tuple; pass the two attribution carriers into `AgendaEntry::pending` (or directly into the `RankedGoalSummary` populated by it) as `learned_opportunity_bonus: <attribution>` and `repair_memory_bonus: <attribution>`. The local bindings `source_reliability_discount`, `competition_discount`, and `provenance` are already in scope at this site.

### 4. Extend `apply_source_reliability_discount` to populate provenance fields

In `crates/worldwake-ai/src/ranking.rs:492-510`, the `SourceReliabilityDiscount` construction at the function's return path (the foundation ticket S171LEACONTDEC-001 populates these with `0` / `None` as a placeholder). Replace the placeholders with reads from the matched `record: &TestimonyReliabilityEntry`:

```rust
SourceReliabilityDiscount {
    source_entity,
    commodity,
    failure_ratio_permille,
    pre_discount_motive,
    post_discount_motive,
    provenance_event_count: u32::try_from(record.provenance_events.len()).unwrap_or(u32::MAX),
    most_recent_provenance_event: record.provenance_events.last().copied(),
}
```

The placeholder construction sites in `ranking.rs` (lines 657, 6577, 6871, 6993) at production paths inherit this populated form via the same code path; test-fixture sites in `decision_runtime.rs:639`, `agent_tick/planning.rs:5548`, `goal_model.rs:2787` retain explicit values appropriate to their fixture intent.

### 5. Extend existing focused tests to assert attribution structure

- `ranking.rs:5826 repair_memory_boosts_matching_alternative_only_while_live` — after the existing motive_score assertion, assert `ranked.repair_memory_bonus` is `Some(_)` with `post_bonus_motive == pre_bonus_motive + bonus_integer`.
- `ranking.rs:5883 learned_opportunity_memory_boosts_matching_opportunity_only_while_live` — symmetric assertion for `ranked.learned_opportunity_bonus`.

### 6. New focused test for SourceReliabilityDiscount provenance population

Add a test in `ranking.rs` (alongside the existing `source_reliability_discount_*` group at lines 6418-6805) that constructs a ranking context with a `TestimonyReliabilityEntry` containing a non-empty `provenance_events` ring; assert the emitted `SourceReliabilityDiscount` has `provenance_event_count == ring.len()` and `most_recent_provenance_event == ring.last().copied()`. Negative case: empty ring → `provenance_event_count == 0` and `most_recent_provenance_event == None`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify) — signature changes on three functions, threading at `AgendaEntry::pending` site, populated `SourceReliabilityDiscount` construction, two extended tests, one new focused test

## Out of Scope

- Decision-trace formatter additions and trace-text rendering (D7 — landed by S171LEACONTDEC-003).
- Any change to motive_score arithmetic, priority class assignment, or candidate ordering (Design Goal 3: no behavior change).
- Bonus formula tuning (Non-Goal 3: bonus integers returned are identical to pre-S171).
- Any change to learned-store mutation paths (S170 provenance fields are read here, never written).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai ranking::tests::source_reliability_discount` — all 7 existing `source_reliability_discount_*` tests pass without modification (V2 contract).
2. `cargo test -p worldwake-ai ranking::tests::repair_memory_boosts_matching_alternative_only_while_live` — passes with extended attribution assertion.
3. `cargo test -p worldwake-ai ranking::tests::learned_opportunity_memory_boosts_matching_opportunity_only_while_live` — passes with extended attribution assertion.
4. New focused test asserting `SourceReliabilityDiscount` provenance population from `TestimonyReliabilityEntry::provenance_events` passes (V1.3).
5. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. For every ranking decision where `learned_opportunity_bonus` returned a non-zero integer, the corresponding `RankedGoalSummary.learned_opportunity_bonus` is `Some(_)` with `post_bonus_motive == pre_bonus_motive + bonus`. Symmetric for `repair_memory_bonus`. (V1.1, V1.2)
2. Attribution carrier `Some(_)` only when the underlying bonus was non-zero; attribution carrier `None` when the bonus was zero (spec Negative Cases).
3. `most_recent_provenance_event` references an `EventId` actually present in the consulted `TestimonyReliabilityEntry::provenance_events` ring — never synthesized.
4. Score arithmetic, priority class, candidate ordering, and `motive_source_contributions` are byte-identical pre/post (V2).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs::tests::repair_memory_boosts_matching_alternative_only_while_live` (modify) — extend with attribution assertion.
2. `crates/worldwake-ai/src/ranking.rs::tests::learned_opportunity_memory_boosts_matching_opportunity_only_while_live` (modify) — extend with attribution assertion.
3. `crates/worldwake-ai/src/ranking.rs::tests::source_reliability_discount_populates_provenance_from_testimony_ring` (new) — V1.3 contract.

### Commands

1. `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount`
2. `cargo test -p worldwake-ai -- ranking::tests::learned_opportunity_memory_boosts ranking::tests::repair_memory_boosts`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh` — confirms fmt/clippy/full-workspace gates before opening the PR.
