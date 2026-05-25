# S171LEACONTDEC-002: Thread learned-context attribution through ranking

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/ranking.rs` (return-shape changes on `learned_opportunity_bonus`, `repair_memory_bonus`, and `memory_motive_bonus`; new attribution threading in the `AgendaEntry::pending` construction path), `crates/worldwake-ai/src/agenda_types.rs` (AgendaEntry carrier fields), `crates/worldwake-ai/src/agent_tick/planning.rs` (RankedGoalSummary projection), and constructor fallout across AI tests/helpers
**Deps**: `archive/tickets/S171LEACONTDEC-001.md` (provides `LearnedOpportunityBonusAttribution`, `RepairMemoryBonusAttribution`, and the extended `SourceReliabilityDiscount` field surface)

## Problem

After archived `archive/tickets/S171LEACONTDEC-001.md`, the attribution types and field surfaces exist on `RankedGoalSummary` and `SourceReliabilityDiscount`, but the two bonus fields are populated as `None` everywhere — the attribution chain from a learned-store mutation (provenance recorded by S170) to a ranking-time decision is still broken for learned-opportunity and repair-memory bonuses. Live S171LEACONTDEC-001 reassessment also clarified the projection seam: `ranking.rs` produces `AgendaEntry`, then `agent_tick/planning.rs::summarize_ranked_goal` projects that entry into `RankedGoalSummary`. This ticket wires the two bonus functions in `ranking.rs` to return both the bonus value AND a populated attribution carrier, extends the `AgendaEntry` carrier and `AgendaEntry::pending` construction at `ranking.rs:290-301`, and copies those fields in `summarize_ranked_goal`. After this ticket lands, every ranking decision that consults a learned-opportunity or repair-memory entry records *which* entry it consulted — closing the FND-22A "experience path" gap for the two bonus axes.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `learned_opportunity_bonus` at `ranking.rs:439-460` and `repair_memory_bonus` at `ranking.rs:413-437` both currently return `u32` and are called only from `memory_motive_bonus` at `ranking.rs:397-411` via `.saturating_add()` chain — the integer return is consumed immediately at one site, so changing the shape to `(u32, Option<…>)` is a local refactor with one caller to update. Existing tests `repair_memory_boosts_matching_alternative_only_while_live` (line 5826) and `learned_opportunity_memory_boosts_matching_opportunity_only_while_live` (line 5883) assert bonus behavior today; they will be extended in this ticket to assert the new attribution structure per V1.
2. Superseded premise: `apply_source_reliability_discount` at `ranking.rs:492-510` does not bind a `TestimonyReliabilityEntry`; it reads `SourceReliability.sources.get(&SourceKey {…})?`, whose value is `ReliabilityRecord` from `crates/worldwake-core/src/experience.rs:77-95`. `ReliabilityRecord` has successful/failed attempt counts, wait/capacity observations, and `last_attempt_tick`, but no event provenance ring. Therefore this ticket cannot truthfully populate `SourceReliabilityDiscount.provenance_event_count` from `TestimonyReliabilityEntry::provenance_events` without expanding the core source-reliability carrier and save shape. That broader source-reliability provenance path is out of this ticket's narrowed implementation scope and must be owned by a follow-up if S171 still wants real non-zero discount provenance.
3. The `AgendaEntry::pending` construction at `ranking.rs:290-301` already has local bindings `source_reliability_discount` (line 277), `competition_discount` (line 282), and `provenance` (line 273) in scope — the new bonus-attribution fields populate from this same site by extending the `memory_motive_bonus` call to return its tupled attributions alongside the integer.
4. Shared abstraction boundary under audit: the `AgendaEntry` ranking carrier produced by `ranking.rs:290-301` and the `RankedGoalSummary` per-decision trace record projected by `agent_tick/planning.rs::summarize_ranked_goal`, then consumed by `decision_trace.rs` formatter sites (rendered separately in S171LEACONTDEC-003) and by `observer.rs:1207-1213` (reads `motive_source_contributions` only; unaffected). The contract is that score arithmetic is byte-identical pre/post; only the trace surface gains data.
5. Per FND-22A: this ticket closes the spec's named gap — the experience path from learned-store mutation event to ranking-time consumption. The store-side provenance was added by S170; this ticket adds the consumption-side trace. No new learned state is introduced.
6. AI regression layer: candidate-generation/ranking focused/unit coverage. `agent_tick` and golden E2E remain unaffected (V3 — no new goldens needed; score arithmetic unchanged).
13. No adjacent contradictions exposed by reassessment — the change is bounded to ranking.rs internals and the function-signature impact is local to one caller (`memory_motive_bonus`).

## Architecture Check

1. The return-shape change `u32` → `(u32, Option<LearnedOpportunityBonusAttribution>)` keeps the bonus integer at tuple-position-0 so `memory_motive_bonus`'s `.saturating_add()` chain still composes cleanly — the attribution carriers flow alongside the integers rather than wrapping them. Score arithmetic is mechanically unchanged: `memory_motive_bonus` continues to sum the two integers; the two attribution carriers are forwarded as new return values to the `AgendaEntry::pending` site.
2. No backwards-compatibility shim or dual-API split — `learned_opportunity_bonus` and `repair_memory_bonus` are private (`fn` not `pub fn`) per `ranking.rs:439, 413`, so the signature change is fully contained within the crate. No external callers exist to migrate.

## Verified Layers

1. Score arithmetic is byte-identical pre/post -> existing focused tests at `ranking.rs:6418-6805` (the 7 `source_reliability_discount_*` tests), `ranking.rs:5826`, `ranking.rs:5883` continue to pass without modification. This is the spec's V2 contract realized.
2. Attribution carriers populate when bonuses apply -> extended focused tests in `ranking.rs` assert `Some(_)` with `pre_bonus_motive + bonus == post_bonus_motive` for both learned-opportunity and repair-memory paths (V1.1, V1.2).
3. `SourceReliabilityDiscount.provenance_event_count` remains `0` and `.most_recent_provenance_event` remains `None` on the live `ReliabilityRecord` path in this ticket; the placeholder fields were added by `archive/tickets/S171LEACONTDEC-001.md` but do not yet have a lawful source-reliability provenance producer.
4. Single-layer ticket at the ranking-substrate boundary — decision-trace rendering of the landed attribution fields remains deferred to S171LEACONTDEC-003 and was not asserted here.

## Landed Changes

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

In `crates/worldwake-ai/src/ranking.rs:397-411`, the return type changed to `(u32, Option<LearnedOpportunityBonusAttribution>, Option<RepairMemoryBonusAttribution>)`. The implementation computes the integer as the existing `saturating_add` chain and forwards both attribution carriers to the caller. The bonus integer remains the sum of the two bonuses.

### 3. Thread attributions into `RankedGoalSummary` at the `AgendaEntry::pending` site

At `crates/worldwake-ai/src/ranking.rs:290-301`, `memory_motive_bonus` is called as part of building each candidate's `AgendaEntry`. Extend the call to destructure the new tuple; add matching optional fields to `AgendaEntry` / `AgendaEntry::pending`, pass the two attribution carriers there, and copy them into `RankedGoalSummary` in `agent_tick/planning.rs::summarize_ranked_goal` as `learned_opportunity_bonus: <attribution>` and `repair_memory_bonus: <attribution>`. The local bindings `source_reliability_discount`, `competition_discount`, and `provenance` are already in scope at this site.

### 4. Extend existing focused tests to assert attribution structure

- `ranking.rs:5826 repair_memory_boosts_matching_alternative_only_while_live` — after the existing motive_score assertion, assert `ranked.repair_memory_bonus` is `Some(_)` with `post_bonus_motive == pre_bonus_motive + bonus_integer`.
- `ranking.rs:5883 learned_opportunity_memory_boosts_matching_opportunity_only_while_live` — symmetric assertion for `ranked.learned_opportunity_bonus`.

## Landed Files

- `crates/worldwake-ai/src/agenda_types.rs` (modify) — add optional attribution fields to `AgendaEntry` and `AgendaEntry::pending`
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify) — copy attribution fields from `AgendaEntry` into `RankedGoalSummary`
- `crates/worldwake-ai/src/ranking.rs` (modify) — signature changes on three functions, threading at `AgendaEntry::pending` site, two extended tests

## Out of Scope

- Decision-trace formatter additions and trace-text rendering (D7 — landed by S171LEACONTDEC-003).
- Any change to motive_score arithmetic, priority class assignment, or candidate ordering (Design Goal 3: no behavior change).
- Bonus formula tuning (Non-Goal 3: bonus integers returned are identical to pre-S171).
- Any change to learned-store mutation paths (S170 provenance fields are read here, never written).

## Acceptance Result

### Tests Passed

1. Passed `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount` — all 7 existing `source_reliability_discount_*` tests passed without source-reliability arithmetic changes (V2 contract).
2. Passed `cargo test -p worldwake-ai -- ranking::tests::repair_memory_boosts_matching_alternative_only_while_live` with extended attribution assertion.
3. Passed `cargo test -p worldwake-ai -- ranking::tests::learned_opportunity_memory_boosts_matching_opportunity_only_while_live` with extended attribution assertion.
4. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. For every ranking decision where `learned_opportunity_bonus` returned a non-zero integer, the corresponding `RankedGoalSummary.learned_opportunity_bonus` is `Some(_)` with `post_bonus_motive == pre_bonus_motive + bonus`. Symmetric for `repair_memory_bonus`. (V1.1, V1.2)
2. Attribution carrier `Some(_)` only when the underlying bonus was non-zero; attribution carrier `None` when the bonus was zero (spec Negative Cases).
3. `SourceReliabilityDiscount` provenance fields are not synthesized from unrelated testimony reliability data; they remain `0` / `None` until a follow-up adds a lawful source-reliability provenance carrier.
4. Score arithmetic, priority class, candidate ordering, and `motive_source_contributions` are byte-identical pre/post (V2).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs::tests::repair_memory_boosts_matching_alternative_only_while_live` (modify) — extend with attribution assertion.
2. `crates/worldwake-ai/src/ranking.rs::tests::learned_opportunity_memory_boosts_matching_opportunity_only_while_live` (modify) — extend with attribution assertion.
### Commands Run

1. `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount`
2. `cargo test -p worldwake-ai -- ranking::tests::repair_memory_boosts_matching_alternative_only_while_live`
3. `cargo test -p worldwake-ai -- ranking::tests::learned_opportunity_memory_boosts_matching_opportunity_only_while_live`
4. `cargo test -p worldwake-ai -- agent_tick::planning::tests::summarize_ranked_goal_preserves_learned_context_bonus_attributions`
5. `cargo test -p worldwake-ai`
6. Waived `./scripts/verify.sh` for this ticket iteration; the full gate is reserved for final branch push after the S171 family lands.

## Verification Result

1. Passed `cargo test -p worldwake-ai -- ranking::tests::repair_memory_boosts_matching_alternative_only_while_live` (2026-05-25).
2. Passed `cargo test -p worldwake-ai -- ranking::tests::learned_opportunity_memory_boosts_matching_opportunity_only_while_live` (2026-05-25).
3. Passed `cargo test -p worldwake-ai -- agent_tick::planning::tests::summarize_ranked_goal_preserves_learned_context_bonus_attributions` (2026-05-25).
4. Passed `cargo test -p worldwake-ai -- ranking::tests::source_reliability_discount` (2026-05-25).
5. Passed `cargo test -p worldwake-ai` (2026-05-25).
6. Waived `./scripts/verify.sh` for this ticket iteration; the full gate is reserved for final branch push after the S171 family lands.

## Outcome

Completed 2026-05-25.

Changed:
- Added learned-opportunity and repair-memory attribution fields to `AgendaEntry` and `AgendaEntry::pending`, with serialized defaults for existing runtime state.
- Changed the ranking memory-bonus helpers to return the existing bonus integer plus a concrete attribution carrier when a live learned-opportunity or repair-memory entry is consulted.
- Threaded those carriers through ranking into `AgendaEntry`, then through `agent_tick/planning.rs::summarize_ranked_goal` into `RankedGoalSummary`.
- Extended focused ranking tests to assert the attributed entry identity, expiry/source fields, and pre/post motive arithmetic for both bonus axes.
- Added a focused projection test proving `summarize_ranked_goal` preserves both bonus-attribution fields.
- Created `tickets/S171LEACONTDEC-004.md` for the live source-reliability provenance gap and truth-synced the active S171 spec plus S171LEACONTDEC-003 dependency wording.

Deviations:
- Live reassessment disproved the drafted `TestimonyReliabilityEntry::provenance_events` source-reliability plan. The actual discount path reads `ReliabilityRecord`, which has no event provenance carrier. This ticket did not synthesize provenance from the wrong store; `tickets/S171LEACONTDEC-004.md` now owns the lawful source-reliability provenance producer.
- Constructor fallout was broader than the drafted three-file list because adding serialized fields to `AgendaEntry` required explicit `None` values in existing test/helper literals across `worldwake-ai`.
