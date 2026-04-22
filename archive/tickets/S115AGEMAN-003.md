# S115AGEMAN-003: agenda_manager module — tick_agenda core flow

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds `agenda_manager.rs` module with `tick_agenda`, revival-trigger evaluation, kill-condition enforcement, capacity eviction, cooldown enforcement, and pure transition reporting for caller-driven event emission.
**Deps**: [archive/tickets/S115AGEMAN-001](../archive/tickets/S115AGEMAN-001.md), [archive/tickets/S115AGEMAN-002](../archive/tickets/S115AGEMAN-002.md)

## Problem

With types defined (ticket 002) and the `AgendaProfile` component available (ticket 001), the agenda manager itself must implement the per-tick lifecycle flow: kill expired entries, fire revival triggers on pending entries (honoring `revive_cooldown_ticks`), merge fresh ranked candidates without duplication, rank the candidate pool, commit-or-keep under placeholder commitment, and demote losers back to pending. Transition-to-event wiring stays caller-owned, but without this ticket `AgendaState` is otherwise inert state and no reusable lifecycle manager exists for later agent-tick integration.

## Assumption Reassessment (2026-04-22)

1. `DiscrepancyMemory` at `crates/worldwake-core/src/discrepancy.rs:53` provides `is_suppressed(key: &BlockerKey, current_tick: Tick) -> bool`, `record(entry: DiscrepancyEntry)`, and `clear_for(key: &BlockerKey)`. This is the only memory lookup `tick_agenda` needs per spec D3 — no new `AgendaMemory` trait is introduced.
2. `GoalBeliefView` at `crates/worldwake-sim/src/belief_view.rs:262` is the planner-facing read interface for agent beliefs. `evaluate_revival_trigger` reads through this view (belief-only per FND-14). No cross-agent reads.
3. The shared boundary under audit is `AgentDecisionRuntime.agenda_state` (from ticket 002) — the authoritative per-agent agenda state. That substrate is already landed on the live branch (`crates/worldwake-ai/src/agenda_types.rs`, `decision_runtime.rs:181`, `worldwake-core/src/agenda_profile.rs`). The remaining live delta for this ticket is the pure manager module plus its exports/tests.
4. `OpportunityKey` (= `AgendaEntryKey` per ticket 002) is the natural key for `BlockerKey` synthesis when emitting `GoalCommitted`, `GoalSuspended`, `GoalAbandoned`. `BlockerKey { goal_key, place, target, action_def }` already lives in `worldwake-core` and is the key `DiscrepancyMemory::record` expects.
5. Positive-feedback loop dampening: the spec's Section H §2 names `AgendaProfile.revive_cooldown_ticks` (default 4) as the revival-oscillation dampener. Implementation: `promote_revived` checks `entry.last_reconsidered_tick + cooldown > tick` and skips the entry when true.
6. Capacity eviction: when `pending.len() >= profile.pending_capacity` after merge, evict entry with smallest `last_reconsidered_tick` (BTreeMap iteration order is key-ordered, not insertion-ordered; must scan for min-tick entry explicitly). Same pattern for `suspended`.
7. The ticket's drafted `tick_agenda(state, fresh_offers: Vec<GoalOffer>, beliefs, ...)` sketch is stale on the live branch. Ranking already converts `GoalOffer -> AgendaEntry` upstream (`crates/worldwake-ai/src/ranking.rs:169-272`), and several belief reads the manager needs are agent-scoped (`GoalBeliefView::controlled_commodity_quantity_at_place`, `expectation_store`, `locally_observed_*`). The truthful manager signature therefore takes `actor: EntityId` and `fresh_candidates: Vec<AgendaEntry>`, not raw `GoalOffer`s.
8. Intended invariant under audit: across ticks, a committed goal A that remains viable (revival trigger unchanged, kill condition not met, no higher-margin challenger) persists in `AgendaState.committed` — no re-commit churn. Margin-based-commit integration (ticket 005) owns the switch-margin check and event-log wiring; this ticket owns the pure kill/revive/merge/capacity/commit-placeholder mechanics.

## Architecture Check

1. `tick_agenda` is a pure transformation over `(AgendaState, fresh_candidates, beliefs, memory, tick)` that returns `AgendaTransitions`. Event emission is caller-driven, keeping the module free of side effects and enabling unit-testability with an in-memory `DiscrepancyMemory` fixture. This aligns with FND-26 (systems interact through state) — the agent tick reads transitions and writes events; the manager never calls the event log directly.
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
use crate::{AgendaEntry, AgendaPhase, AgendaState, AgendaEntryKey, RevivalTrigger, KillCondition};
use crate::ranking;
use worldwake_core::{AgendaProfile, DiscrepancyMemory, EntityId, Tick};
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
    actor: EntityId,
    state: &mut AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    beliefs: &impl GoalBeliefView,
    discrepancy_memory: &DiscrepancyMemory,
    profile: &AgendaProfile,
    tick: Tick,
) -> AgendaTransitions { /* per spec D3 */ }
```

Implement helpers `drain_killed`, `promote_revived`, `merge_candidates`, `rank_for_commit`, `commit_or_keep`, `demote_to_pending_or_suspended`. All helpers are `pub(crate)` or private; only `tick_agenda` and `AgendaTransitions` / `CommitTransition` are public.

### 2. Capacity + cooldown enforcement

- `merge_candidates`: before inserting a fresh candidate into pending, check `pending.len() >= profile.pending_capacity`; if so, remove the entry with smallest `last_reconsidered_tick`. Same pattern for suspension when entries are demoted.
- `promote_revived`: iterate pending in key order; for each entry, check `entry.last_reconsidered_tick + profile.revive_cooldown_ticks as u64 > tick.0` → skip. Also check `discrepancy_memory.is_suppressed(&blocker_key_from(entry), tick)` → skip (already suppressed).

### 3. Revival-trigger evaluation helper

```rust
fn evaluate_revival_trigger(
    actor: EntityId,
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

Variants: `TickExpiry { at_tick }` → `tick >= at_tick`; `ObligationResolved { expectation }` → actor-scoped expectation-store check for resolved/expired-or-missing expectation; `TargetDead { target }` → belief-view check; `External` → never kill.

### 5. Candidate pool ranking for commit decision

```rust
fn rank_for_commit(
    state: &AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    revived: Vec<AgendaEntry>,
) -> CandidatePool { .. }
```

Builds a pool of `AgendaEntry` values (committed + revived + fresh candidates) and calls `ranking::sort_in_place` (ticket 002 renamed surface). Motive scoring remains upstream — this ticket does not rescore `GoalOffer`s.

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

For this ticket's placeholder commit policy, non-winning candidates are returned to `pending`, not `suspended`. Preserve an existing revival trigger when present; otherwise assign `TickElapsed { at_tick: tick + 1 }` as the keep-alive trigger so lower-motive losers can lawfully re-compete next tick. `suspended` routing remains sibling-ticket territory once D4A rejection classification lands.

### 8. Helper: `blocker_key_from(entry: &AgendaEntry) -> BlockerKey`

Synthesizes `BlockerKey` from the entry's anchor-first place/target identity, falling back to `GoalKey.place` / `GoalKey.entity` when the anchor is `None`.

## Files to Touch

- `crates/worldwake-ai/src/agenda_manager.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `tick_agenda`, `AgendaTransitions`, `CommitTransition`)
- `tickets/S115AGEMAN-003.md` (update — reassessment + truthful scope/proof surface)

## Out of Scope

- D4A `classify_rejection` and its integration into demote-to-pending/suspended routing (ticket 004)
- S74 margin-based switch logic inside `commit_or_keep` (ticket 005 replaces the placeholder)
- `agenda_tick_system` SystemFn wiring into agent-tick phase (ticket 005)
- S110 event emission from the caller (ticket 005 threads `AgendaTransitions` into event-log writes)
- Golden scenarios (ticket 007) — module is unit-testable without golden fixtures

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --lib agenda_manager::tests -- --exact` selectors resolve and all new focused tests pass:
   - `merge_fresh_offer_with_same_key_refreshes_without_duplicate`
   - `revival_trigger_commodity_available_fires_when_belief_confirms_quantity`
   - `kill_condition_tick_expiry_drops_entry_on_or_after_expiry`
   - `capacity_overflow_evicts_smallest_last_reconsidered_tick`
   - `revival_cooldown_blocks_re_promotion_within_window`
   - `discrepancy_memory_suppression_blocks_revival_when_is_suppressed_true`
2. Existing suite: `cargo test -p worldwake-ai` passes.
3. Existing suite: `cargo test --workspace` passes.

### Invariants

1. `tick_agenda` is deterministic: given the same `(AgendaState, fresh_candidates, beliefs, memory, profile, tick)` inputs, output `AgendaTransitions` and the mutated `AgendaState` are byte-identical across runs (no HashMap / no wall-clock).
2. No entry appears in more than one of `committed` / `pending` / `suspended` simultaneously (single-slot invariant).
3. `pending.len() <= profile.pending_capacity` and `suspended.len() <= profile.suspended_capacity` at the end of every `tick_agenda` call.
4. Revival cooldown respected: no entry with key K is promoted twice within `revive_cooldown_ticks` ticks.
5. `AgendaTransitions` accounts for lifecycle-visible slot changes (`killed`, `revived`, commit changes, demotions). Capacity eviction is internal state pruning and is not surfaced as a dedicated transition in this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` (new inline `#[cfg(test)]`) — all 6 invariants from Acceptance Criteria §1 covered. Uses `MockGoalBeliefView` stub (belief-store mock) to drive revival-trigger and kill-condition paths.
2. No golden or integration tests in this ticket — module is unit-testable at the function level.

### Commands

1. `cargo test -p worldwake-ai --lib agenda_manager::tests -- --list`
2. `cargo test -p worldwake-ai --lib agenda_manager::tests::merge_fresh_offer_with_same_key_refreshes_without_duplicate -- --exact`
3. `cargo test -p worldwake-ai --lib agenda_manager::tests::revival_trigger_commodity_available_fires_when_belief_confirms_quantity -- --exact`
4. `cargo test -p worldwake-ai --lib agenda_manager::tests::kill_condition_tick_expiry_drops_entry_on_or_after_expiry -- --exact`
5. `cargo test -p worldwake-ai --lib agenda_manager::tests::capacity_overflow_evicts_smallest_last_reconsidered_tick -- --exact`
6. `cargo test -p worldwake-ai --lib agenda_manager::tests::revival_cooldown_blocks_re_promotion_within_window -- --exact`
7. `cargo test -p worldwake-ai --lib agenda_manager::tests::discrepancy_memory_suppression_blocks_revival_when_is_suppressed_true -- --exact`
8. `cargo test -p worldwake-ai`
9. `cargo test --workspace`
10. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-22.

- Added [crates/worldwake-ai/src/agenda_manager.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agenda_manager.rs) with pure agenda lifecycle mechanics: kill-condition pruning, actor-scoped revival checks, discrepancy-memory suppression, candidate merge/dedup, deterministic ranking, placeholder commit-or-keep behavior, and pending-capacity eviction.
- Re-exported `tick_agenda`, `AgendaTransitions`, and `CommitTransition` from [crates/worldwake-ai/src/lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/lib.rs).
- Added focused inline unit coverage for duplicate merge, commodity-trigger revival, tick-expiry kills, capacity eviction, cooldown suppression, discrepancy-memory suppression, expectation-backed obligation kill behavior, and unchanged committed-goal retention.

## Deviations

- The live branch already landed the agenda substrate (`AgendaState`, `AgendaEntry`, `AgendaProfile`, runtime storage), so this ticket delivered the remaining manager module rather than re-landing those shared types.
- The truthful manager signature is `tick_agenda(actor, state, fresh_candidates: Vec<AgendaEntry>, ...)`, not the ticket's older raw-`GoalOffer` / actor-free sketch, because ranking now happens upstream and several belief reads are agent-scoped.
- Placeholder loser routing in this ticket returns non-winning candidates to `pending` with `TickElapsed { at_tick: tick + 1 }` when they have no revival trigger. `suspended` routing and event-log integration remain owned by sibling tickets.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests -- --list`
- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::merge_fresh_offer_with_same_key_refreshes_without_duplicate -- --exact`
- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::revival_trigger_commodity_available_fires_when_belief_confirms_quantity -- --exact`
- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::kill_condition_tick_expiry_drops_entry_on_or_after_expiry -- --exact`
- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::capacity_overflow_evicts_smallest_last_reconsidered_tick -- --exact`
- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::revival_cooldown_blocks_re_promotion_within_window -- --exact`
- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::discrepancy_memory_suppression_blocks_revival_when_is_suppressed_true -- --exact`
- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
