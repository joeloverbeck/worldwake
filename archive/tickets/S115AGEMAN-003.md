# S115AGEMAN-003: agenda_manager module — tick_agenda core flow

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds `agenda_manager.rs` module with `tick_agenda`, revival-trigger evaluation, kill-condition enforcement, capacity eviction, cooldown enforcement, and S110 event emission.
**Deps**: [archive/tickets/S115AGEMAN-001](../archive/tickets/S115AGEMAN-001.md), [archive/tickets/S115AGEMAN-002](../archive/tickets/S115AGEMAN-002.md)

## Problem

With types defined (ticket 002) and the `AgendaProfile` component available (ticket 001), the agenda manager itself must implement the per-tick lifecycle flow: kill expired entries, fire revival triggers on pending entries (honoring `revive_cooldown_ticks`), merge fresh offers into pending without duplication, rank the candidate pool, commit-or-keep under margin-based commitment, and demote losers to pending/suspended. Every lifecycle transition must emit the corresponding S110 event (`GoalCommitted`, `GoalSuspended`, `GoalAbandoned`). This module is the single authority for agenda state mutation — downstream code reads `AgendaState` but does not write to it outside `tick_agenda` and the D4A classifier (ticket 004). Without this ticket, `AgendaState` is inert state and no committed-goal persistence actually happens.

## Assumption Reassessment (2026-04-22)

1. `DiscrepancyMemory` at `crates/worldwake-core/src/discrepancy.rs:53` provides `is_suppressed(key: &BlockerKey, current_tick: Tick) -> bool`, `record(entry: DiscrepancyEntry)`, and `clear_for(key: &BlockerKey)`. This is the only memory lookup `tick_agenda` needs per spec D3 — no new `AgendaMemory` trait is introduced.
2. `GoalBeliefView` at `crates/worldwake-sim/src/belief_view.rs:262` is the planner-facing read interface for agent beliefs. `evaluate_revival_trigger` reads through this view (belief-only per FND-14). No cross-agent reads.
3. The shared boundary under audit is `AgentDecisionRuntime.agenda_state` (from ticket 002) — the authoritative per-agent agenda state. `tick_agenda` takes `&mut AgendaState` and a `Tick`, and returns an `AgendaTransitions` value listing what was killed, revived, and committed this tick. Event emission is caller-driven (caller threads transitions into the event log), not manager-driven — this keeps the manager pure of I/O.
4. `OpportunityKey` (= `AgendaEntryKey` per ticket 002) is the natural key for `BlockerKey` synthesis when emitting `GoalCommitted`, `GoalSuspended`, `GoalAbandoned`. `BlockerKey { goal_key, place, target, action_def }` already lives in `worldwake-core` and is the key `DiscrepancyMemory::record` expects.
5. Positive-feedback loop dampening: the spec's Section H §2 names `AgendaProfile.revive_cooldown_ticks` (default 4) as the revival-oscillation dampener. Implementation: `promote_revived` checks `entry.last_reconsidered_tick + cooldown > tick` and skips the entry when true.
6. Capacity eviction: when `pending.len() >= profile.pending_capacity` after merge, evict entry with smallest `last_reconsidered_tick` (BTreeMap iteration order is key-ordered, not insertion-ordered; must scan for min-tick entry explicitly). Same pattern for `suspended`.
7. Intended invariant under audit: across ticks, a committed goal A that remains viable (revival trigger unchanged, kill condition not met, no higher-margin challenger) persists in `AgendaState.committed` — no re-commit churn. Margin-based-commit integration (ticket 005) owns the switch-margin check; this ticket owns the merge/demote mechanics.

## Architecture Check

1. `tick_agenda` is a pure transformation over `(AgendaState, fresh_offers, beliefs, memory, tick)` that returns `AgendaTransitions`. Event emission is caller-driven, keeping the module free of side effects and enabling unit-testability with an in-memory `DiscrepancyMemory` fixture. This aligns with FND-26 (systems interact through state) — the agent tick reads transitions and writes events; the manager never calls the event log directly.
2. No backwards-compatibility shims. Re-ranking candidates via `ranking::sort_in_place` (ticket 002 renamed-through) reuses the existing preference authority (S123) rather than introducing a parallel comparator — aligned with S115 Dependencies.

## Verification Layers

1. Merge-without-duplicate — focused unit test: seed pending entry with key K, feed fresh offer with same key; assert pending size unchanged, `last_reconsidered_tick` updated → unit test in `agenda_manager.rs::tests`.
2. Revival firing — focused unit test: create pending entry with `RevivalTrigger::CommodityAvailable`; configure `MockGoalBeliefView` to report quantity ≥ min; call `promote_revived`; assert entry removed from pending and returned in `revived` → unit test.
3. Kill condition — unit test: pending entry with `KillCondition::TickExpiry { at_tick: T }`; call `tick_agenda` at tick T+1; assert entry in `killed` transition.
4. Capacity eviction — unit test: populate pending to capacity+1; assert smallest-`last_reconsidered_tick` evicted.
5. Revival cooldown — unit test: pending entry revived at tick T; trigger fires again at tick T+1 (cooldown=4); assert not promoted until T+4.
6. Single-layer ticket — this is in-memory state mutation only; no action trace, no event-log delta surface from this module directly (caller emits events).

## What to Change

### 1. Create `crates/worldwake-ai/src/agenda_manager.rs`

Module skeleton:

```rust
use crate::{AgendaEntry, AgendaPhase, AgendaState, AgendaEntryKey, GoalOffer, RevivalTrigger, KillCondition};
use crate::ranking;
use worldwake_core::{AgendaProfile, DiscrepancyMemory, Tick};
use worldwake_sim::GoalBeliefView;

pub struct AgendaTransitions {
    pub killed: Vec<AgendaEntry>,
    pub revived: Vec<AgendaEntryKey>,
    pub commit_transition: CommitTransition,
    pub demoted_to_suspended: Vec<AgendaEntryKey>,
    pub demoted_to_pending: Vec<AgendaEntryKey>,
}

pub enum CommitTransition {
    Unchanged,
    Committed { new_key: AgendaEntryKey, previous_key: Option<AgendaEntryKey> },
    Cleared { previous_key: AgendaEntryKey },
}

pub fn tick_agenda(
    state: &mut AgendaState,
    fresh_offers: Vec<GoalOffer>,
    beliefs: &impl GoalBeliefView,
    discrepancy_memory: &DiscrepancyMemory,
    profile: &AgendaProfile,
    tick: Tick,
) -> AgendaTransitions { /* per spec D3 */ }
```

Implement helpers `drain_killed`, `promote_revived`, `merge_offers`, `rank_for_commit`, `commit_or_keep`, `demote_to_pending_or_suspended`. All helpers are `pub(crate)` or private; only `tick_agenda` and `AgendaTransitions` / `CommitTransition` are public.

### 2. Capacity + cooldown enforcement

- `merge_offers`: before inserting a fresh offer into pending, check `pending.len() >= profile.pending_capacity`; if so, remove the entry with smallest `last_reconsidered_tick`. Same pattern for suspension when entries are demoted.
- `promote_revived`: iterate pending in key order; for each entry, check `entry.last_reconsidered_tick + profile.revive_cooldown_ticks as u64 > tick.0` → skip. Also check `discrepancy_memory.is_suppressed(&blocker_key_from(entry), tick)` → skip (already suppressed).

### 3. Revival-trigger evaluation helper

```rust
fn evaluate_revival_trigger(
    trigger: &RevivalTrigger,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> bool { /* per trigger variant */ }
```

Each variant maps to a specific belief-view read (commodity availability, target presence, route knowledge, counterparty observation, or elapsed-tick check).

### 4. Kill-condition evaluation helper

```rust
fn should_kill(cond: &KillCondition, beliefs: &impl GoalBeliefView, tick: Tick) -> bool { .. }
```

Variants: `TickExpiry { at_tick }` → `tick >= at_tick`; `ObligationResolved { expectation }` → belief-view check for expectation fulfillment; `TargetDead { target }` → belief-view check; `External` → never kill.

### 5. Candidate pool ranking for commit decision

```rust
fn rank_for_commit(
    state: &mut AgendaState,
    fresh_offers: &[GoalOffer],
    revived: &[AgendaEntryKey],
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> CandidatePool { .. }
```

Builds a pool of `AgendaEntry` values (committed + revived + fresh offers converted to `AgendaEntry` with `phase: Pending`) and calls `ranking::sort_in_place` (ticket 002 renamed surface) to produce `OrderedRanked<'_>`. Motive scoring reuses existing ranking — this ticket does not change scoring algorithms.

### 6. Commit-or-keep stub

```rust
fn commit_or_keep(
    state: &mut AgendaState,
    ranked: OrderedRanked<'_>,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> CommitTransition { .. }
```

This ticket implements the mechanics (move top candidate into `committed` if no current commit; retain current commit otherwise). The switch-margin check itself lands in ticket 005 where S74's `cognitive.switch_margin` is read. For this ticket, `commit_or_keep` uses a placeholder: retain current unless current is empty. Ticket 005 replaces the placeholder with the margin-aware comparison.

### 7. Demote losers

```rust
fn demote_to_pending_or_suspended(
    state: &mut AgendaState,
    losers: Vec<AgendaEntry>,
    profile: &AgendaProfile,
    tick: Tick,
) { .. }
```

For each loser: if `revival_trigger.is_some()` → pending (with capacity eviction); else → suspended (with capacity eviction). Lifecycle classification from D4A (ticket 004) provides the trigger for losers that were probe-rejected; losers that are simply lower-motive get `revival_trigger = None` here and go to suspended.

Actually: lower-motive losers stay as normal ranking losers. They should NOT enter suspended — they re-compete next tick. Revision: demote only loser entries that originated as `Pending` or as `Committed` that lost the margin comparison. Ranking losers that were fresh offers this tick but didn't win go into `pending` with their existing revival trigger or `TickElapsed { at_tick: tick + 1 }` as a default keep-alive. Clarify in implementation per the spec's D3 step 6.

### 8. Helper: `blocker_key_from(entry: &AgendaEntry) -> BlockerKey`

Synthesizes `BlockerKey { goal_key: entry.offer.key, place: entry.offer.anchor.place(), target: entry.offer.anchor.entity(), action_def: None }` per spec D4A final paragraph.

## Files to Touch

- `crates/worldwake-ai/src/agenda_manager.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `tick_agenda`, `AgendaTransitions`, `CommitTransition`)

## Out of Scope

- D4A `classify_rejection` and its integration into demote-to-pending/suspended routing (ticket 004)
- S74 margin-based switch logic inside `commit_or_keep` (ticket 005 replaces the placeholder)
- `agenda_tick_system` SystemFn wiring into agent-tick phase (ticket 005)
- S110 event emission from the caller (ticket 005 threads `AgendaTransitions` into event-log writes)
- Golden scenarios (ticket 007) — module is unit-testable without golden fixtures

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- agenda_manager` — all new focused tests pass:
   - `merge_fresh_offer_with_same_key_refreshes_without_duplicate`
   - `revival_trigger_commodity_available_fires_when_belief_confirms_quantity`
   - `kill_condition_tick_expiry_drops_entry_on_or_after_expiry`
   - `capacity_overflow_evicts_smallest_last_reconsidered_tick`
   - `revival_cooldown_blocks_re_promotion_within_window`
   - `discrepancy_memory_suppression_blocks_revival_when_is_suppressed_true`
2. Existing suite: `cargo test --workspace` passes.

### Invariants

1. `tick_agenda` is deterministic: given the same `(AgendaState, fresh_offers, beliefs, memory, profile, tick)` inputs, output `AgendaTransitions` and the mutated `AgendaState` are byte-identical across runs (no HashMap / no wall-clock).
2. No entry appears in more than one of `committed` / `pending` / `suspended` simultaneously (single-slot invariant).
3. `pending.len() <= profile.pending_capacity` and `suspended.len() <= profile.suspended_capacity` at the end of every `tick_agenda` call.
4. Revival cooldown respected: no entry with key K is promoted twice within `revive_cooldown_ticks` ticks.
5. `AgendaTransitions` exhaustively accounts for every change: for every entry removed from any slot, it appears in `killed`, `revived`, `demoted_to_pending`, or `demoted_to_suspended` (or is the `previous_key` of a `CommitTransition::Committed` / `Cleared`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` (new inline `#[cfg(test)]`) — all 6 invariants from Acceptance Criteria §1 covered. Uses `MockGoalBeliefView` stub (belief-store mock) to drive revival-trigger and kill-condition paths.
2. No golden or integration tests in this ticket — module is unit-testable at the function level.

### Commands

1. `cargo test -p worldwake-ai -- agenda_manager`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`
