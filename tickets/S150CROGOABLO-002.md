# S150CROGOABLO-002: BlockerScope substrate + cross-store key migration + recording-site source_event

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core` substrate (`BlockerScope`, `RouteSegment`, `BlockerMemory`, `DiscrepancyMemory`, `BlockerRecordedPayload`); `worldwake-ai` consumer migrations (3 read sites + 3 record sites + 17 dependent modules); `worldwake-sim` save-format bump; `worldwake-systems` trade-action recording; `worldwake-cli` observer rendering preservation
**Deps**: archive/tickets/S150CROGOABLO-001.md

## Problem

The current blocker substrate keys `BlockerMemory` and `DiscrepancyMemory` on `BlockerKey { goal_key, place, target, action_def }` — a flat 4-tuple where `goal_key` is always set. Cross-goal facts ("route X is dangerous regardless of which goal needs it", "counterparty Y refuses everyone") cannot be represented without inserting one entry per goal-kind that might encounter the fact. The downstream consequence is that blocker-recent learning fails to suppress goals it should suppress, multiplying redundant planning attempts and violating FND-22A's "accountable origin per agent" by fragmenting the same fact into many goal-keyed entries.

S150 introduces `BlockerScope` — a typed enum `Exact(BlockerKey) | RouteSegment(RouteSegment) | Counterparty(EntityId)` — that supersedes the flat `BlockerKey` as the map key for both stores. Per FND-28's single-truth requirement, `BlockerKey` survives only as the payload of `BlockerScope::Exact(_)` (contained, not coexisting). All 181 `BlockerKey` reference sites across 5 crates migrate to `BlockerScope` in one pass — splitting the migration breaks compile because intermediate states have one store keyed on the old type while sibling code expects the new type. The migration also captures `source_event: EventId` on every recorded blocker / discrepancy per FND-22A's "accountable origin" and FND-29A's append-only causal-history linkage.

## Assumption Reassessment (2026-05-17)

1. The substrate types live in `crates/worldwake-core/src/blocker_memory.rs` (`BlockerKey` at line 10, `BlockerMemory` at line 24, `Blocker` at line 168, `BlockerClearingCondition` at line 131, `BlockingFact` at line 190) and `crates/worldwake-core/src/discrepancy.rs` (`DiscrepancyMemory` at line 73, `DiscrepancyEntry` at line 47, `Discrepancy` at line 9). The spec previously claimed these lived in `worldwake-ai`; reassessment confirmed core ownership. Both stores derive `Component`; both are registered on `EntityKind::Agent` through `component_schema.rs`.
2. Spec source: `specs/S150-cross-goal-blocker-scoping.md` D1 (BlockerScope/RouteSegment), D2 (substrate migration), D3 (read sites + helpers), D4 (recording paths with source_event), and the FND-28 single-truth wrapper rationale in Summary + FOUNDATIONS Alignment row.
3. Shared abstraction boundary: the BTreeMap key type that simultaneously controls (a) lookup matching at three AI-layer read sites, (b) serialization round-trip via `worldwake-sim/src/save_load.rs`, (c) decision-event payload shape at `decision_event_payload.rs:477-493`. All three consume the same key shape; the migration must update all three in lockstep.
4. The same fact (a blocker for a specific (goal, place, target, action) tuple) currently has exactly one transport path — `BlockerMemory.intents: BTreeMap<BlockerKey, Blocker>` with the per-struct `blocker_key` field duplicating the map key. After the migration, the canonical path is `BlockerMemory.intents: BTreeMap<BlockerScope, Blocker>` with the per-struct `scope` field replacing `blocker_key`. The duplication between map key and struct field collapses to one source of truth (the struct field). No mixed-state coexistence is deferred; ticket 002 lands the entire migration. Information-path refactor rule 16 satisfied.
5. AI regression layer: this ticket touches AI candidate generation (`candidate_generation.rs:759`), feasibility probe (`feasibility_probe.rs:42`), and search candidate filtering (`search/candidates.rs:1336`). The first two emit `is_blocked` queries that gate candidate emission and feasibility verdict; the third uses `find_blocked_for_search` for search-time suppression. All three live in the AI crate, all three are exercised by both inline `#[cfg(test)]` tests and golden E2E goldens. The harness boundary is full action registries (golden E2E coverage) because the migration touches multi-goal candidate suppression across travel + trade + Tell flows.
6. Ordering layer: blocker suppression ordering relative to candidate generation is preserved unchanged — the migration changes the key type and adds new lookup paths, not the ordering of lookup. The blocker check still happens *during* candidate generation before the candidate is added to the offering set; failure path still records the blocker through the same recording sites. No ordering layer changes.
7. Existing tests in target modules (named per `docs/precision-rules.md` Rule 3):
   - `crates/worldwake-core/src/blocker_memory.rs` `#[cfg(test)]`: `blocker_types_satisfy_required_bounds`, `is_blocked_matches_only_live_entries_for_goal_key`, `record_replaces_existing_entry_for_same_compound_key`, `record_preserves_different_place_for_same_goal`, `expire_removes_entries_at_or_before_current_tick`, `clear_for_removes_matching_blocker_key`, `clear_all_for_goal_removes_all_entries_for_goal`, `blocker_memory_roundtrips_through_bincode`, `exclusive_facility_blockers_do_not_block_goal_generation`, `is_blocked_for_search_ignores_blocks_goal_generation_gate`, `global_blocker_matches_any_place_query`, `place_scoped_blocker_does_not_match_different_place`, `place_scoped_goal_blocking_fact_matches_global_query`, `place_scoped_non_goal_blocking_fact_does_not_match_global_query`, `sweep_cleared_removes_matching_entries`, `pursuit_target_gone_blocker_scoped_to_target_and_place` — every test that constructs `Blocker { blocker_key, ... }` or queries via `is_blocked(&goal_key, ...)` needs migration to the scope-keyed shape. The semantic intent (Exact-scope behavior) is preserved by wrapping legacy `BlockerKey` constructions in `BlockerScope::Exact(...)`.
   - `crates/worldwake-core/src/discrepancy.rs` `#[cfg(test)]`: `discrepancy_memory_record_and_expire_prunes_stale_entries`, `discrepancy_memory_clear_for_removes_matching_entry`, `discrepancy_memory_clear_by_condition_matches_reobservation_target`, `discrepancy_memory_clear_by_condition_matches_belief_update_key`, `discrepancy_memory_clear_by_condition_matches_commodity_availability_change`, `discrepancy_entry_roundtrips_through_bincode` — same migration pattern.
   - `crates/worldwake-core/src/decision_event_payload.rs` `#[cfg(test)]`: `blocker_recorded_payload_roundtrips_with_belief_snapshot_some`, `blocker_recorded_payload_roundtrips_with_belief_snapshot_none`, `decision_event_payload_variants_roundtrip_through_bincode` (sample_decision_payloads fixture) — add `scope` field to constructions.
   - `crates/worldwake-ai/src/feasibility_probe.rs` `#[cfg(test)]`: `probe_rejects_on_blocker_memory_hit`, `probe_rejects_on_discrepancy_memory_hit` — migrate to scope-keyed lookups.
8. Adjacent contradictions classified: (a) `Blocker.diagnostic_context: Option<BlockerDiagnostic>` is preserved unchanged (not part of S150 scope; predates the spec); (b) `is_blocked`/`find_blocked_for_search`/etc. method signatures change from accepting `&GoalKey, Option<EntityId>, Option<EntityId>, Option<ActionDefId>` (current 4-arg call) to a unified scope-resolution path — the new helpers (`route_segment_blocked`, `counterparty_blocked`, `any_blocker_on_path`) compose the matching. The existing 4-arg signature is preserved for `BlockerScope::Exact` queries (call sites currently passing the 4-arg form become `is_blocked(&BlockerScope::Exact(BlockerKey {...}), current_tick)` or remain on a backwards-named convenience wrapper); the spec leaves this implementation detail open. Required consequence of the migration, not a separate bug.

## Architecture Check

1. **FND-28 single-truth, not back-compat**: `BlockerScope::Exact(BlockerKey)` is containment, not aliasing. Post-migration, no live code reads `BlockerKey` outside of `BlockerScope::Exact(_)` — the struct field `Blocker.scope: BlockerScope` is the one canonical key for every recorded blocker. The 181 reference migration is the price of single-truth; splitting the migration would force shims that violate FND-28.
2. **Cross-store symmetry**: `BlockerMemory` and `DiscrepancyMemory` migrating in lockstep preserves the parallel-substrate symmetry. Without the parallel migration, `Discrepancy::RouteUnknown` keyed by goal-A's RouteSegment would not suppress goal-B's retry of the same route, fragmenting cross-goal failure attribution between the two stores.
3. **FND-22A accountable origin**: `source_event: EventId` on `Blocker` and `DiscrepancyEntry` makes "why is this blocker here?" answerable by direct event-log lookup rather than temporal correlation. The append-only event log already carries the originating event; this ticket links the live learned state back to it.
4. **No back-compat shim for `blocker_key` field**: the per-struct `blocker_key: BlockerKey` field is replaced by `scope: BlockerScope` (renamed and retyped in one pass). Some current code paths that reach into `.blocker_key` would have continued working under a coexistence pattern; eliminating the field forces all consumers to use `scope` and prevents fossilized read paths.

## Verification Layers

1. Scope-keyed lookup behavior — focused unit tests on `BlockerMemory::route_segment_blocked`, `counterparty_blocked`, `any_blocker_on_path` (in `blocker_memory.rs` `#[cfg(test)]`).
2. Migrated existing-test behavior — extension of the 22 existing `blocker_memory.rs` tests + 13 `discrepancy.rs` tests, proving that scope-keyed semantics preserve the prior `BlockerKey`-keyed behavior for `BlockerScope::Exact(...)` queries.
3. Recording-path `source_event` capture — focused unit test asserting that each of the three recording sites (agent_tick/execution.rs:1341, agent_tick/observation.rs:626, failure_handling.rs:224) produces a `Blocker` whose `source_event` points to a real event in the log.
4. Feasibility probe rejection — extension of `probe_rejects_on_blocker_memory_hit` / `probe_rejects_on_discrepancy_memory_hit` to use scope-keyed lookups.
5. Decision-event payload serialization — extension of `blocker_recorded_payload_roundtrips_with_belief_snapshot_some/none` to assert the new `scope` field round-trips through bincode.
6. Save-format compatibility — focused test in `worldwake-sim/src/save_load.rs` proving that a world snapshot with scope-keyed `BlockerMemory` round-trips through the new `SAVE_FORMAT_VERSION = 86` format.
7. Workspace build coherence — `cargo test --workspace` proves every consumer of `BlockerKey` (181 sites) was migrated coherently. Splitting the migration would surface as compile errors at any intermediate state.

## What to Change

### 1. Define `BlockerScope` enum and `RouteSegment` newtype in core

New file `crates/worldwake-core/src/blocker_scope.rs` containing:

```rust
use crate::{BlockerKey, EntityId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BlockerScope {
    Exact(BlockerKey),
    RouteSegment(RouteSegment),
    Counterparty(EntityId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RouteSegment {
    pub from: EntityId,
    pub to: EntityId,
}

impl RouteSegment {
    pub fn new(from: EntityId, to: EntityId) -> Self {
        // Canonical ordering for undirected segments
        if from <= to { Self { from, to } } else { Self { from: to, to: from } }
    }
}
```

Add `pub mod blocker_scope;` to `crates/worldwake-core/src/lib.rs` and re-export `BlockerScope`, `RouteSegment`.

### 2. Migrate `BlockerMemory` and `Blocker` to scope-keyed shape

In `crates/worldwake-core/src/blocker_memory.rs`:

- Change `BlockerMemory.intents` from `BTreeMap<BlockerKey, Blocker>` to `BTreeMap<BlockerScope, Blocker>`.
- Rewrite `Blocker` struct: replace `blocker_key: BlockerKey` with `scope: BlockerScope`; add `source_event: EventId` as a new field. Preserve `blocking_fact`, `diagnostic_context`, `observed_tick`, `expires_tick`, `clearing_condition`, `baseline_snapshot` unchanged.
- Rewrite method signatures to take `&BlockerScope`:
  - `is_blocked(&self, scope: &BlockerScope, current_tick: Tick) -> bool` (with internal Exact-vs-typed dispatch)
  - `is_blocked_for_search(&self, scope: &BlockerScope, current_tick: Tick) -> bool`
  - `find_blocked_for_search(&self, scope: &BlockerScope, current_tick: Tick) -> Option<&Blocker>`
  - `record(&mut self, intent: Blocker)` — unchanged signature; internally inserts at `intent.scope`
  - `clear_for(&mut self, scope: &BlockerScope)`
  - `clear_all_for_goal(&mut self, goal_key: &GoalKey)` — preserved; iterates the map and removes entries whose `scope` is `BlockerScope::Exact(bk)` with `bk.goal_key == *goal_key` (other scope variants are not goal-keyed and stay)
  - `sweep_cleared(&mut self, is_cleared: impl FnMut(&Blocker) -> bool)` — unchanged
- Add new helper methods:
  - `route_segment_blocked(&self, from: EntityId, to: EntityId, tick: Tick) -> Option<&Blocker>` — canonicalizes the segment and looks up `BlockerScope::RouteSegment(RouteSegment::new(from, to))`
  - `counterparty_blocked(&self, other: EntityId, tick: Tick) -> Option<&Blocker>` — looks up `BlockerScope::Counterparty(other)`
  - `any_blocker_on_path(&self, path: &[EntityId], tick: Tick) -> Option<&Blocker>` — walks adjacent pairs in path, returns first matching RouteSegment blocker
- The internal `matches_scope` helper at lines 97-126 becomes scope-variant-aware; the legacy 4-arg matching applies only inside `BlockerScope::Exact(...)` dispatch.

### 3. Migrate `DiscrepancyMemory` and `DiscrepancyEntry`

In `crates/worldwake-core/src/discrepancy.rs`:

- Change `DiscrepancyMemory.entries` from `BTreeMap<BlockerKey, DiscrepancyEntry>` to `BTreeMap<BlockerScope, DiscrepancyEntry>`.
- Rewrite `DiscrepancyEntry`: replace `blocker_key: BlockerKey` with `scope: BlockerScope`; add `source_event: EventId`. Preserve `discrepancy`, `observed_tick`, `expires_tick`, `clearing_condition`.
- Method signatures: `record(&mut self, entry: DiscrepancyEntry)` unchanged shape, internally inserts at `entry.scope`; `is_suppressed(&self, scope: &BlockerScope, current_tick: Tick) -> bool`; `clear_for(&mut self, scope: &BlockerScope)`; `clear_by_condition` unchanged signature; `expire` unchanged.
- `DiscrepancyClearing` variants stay unchanged — the new scopes still use the existing clearing predicates (ReobservationOf, BeliefUpdate, CommodityAvailabilityChanged, etc.).

### 4. Extend `BlockerRecordedPayload` with `scope` field

In `crates/worldwake-core/src/decision_event_payload.rs:476-493`:

- Add `pub scope: BlockerScope,` to `BlockerRecordedPayload`. Position it next to or replace `pub blocker_key: BlockerKey,` (line 479) — per the FND-28 single-truth principle, `blocker_key` is fully eliminated and `scope` is the canonical identifier. The 4 construction sites in the module's tests and the 1 site in `sample_decision_payloads` are updated.

### 5. Update the three AI-layer read sites

- `crates/worldwake-ai/src/candidate_generation.rs:759` `is_blocked(goal_key, place, target, action_def, ctx.current_tick)` — rewrite to call `is_blocked(&BlockerScope::Exact(BlockerKey { goal_key, place, target, action_def }), ctx.current_tick)` AND, for travel-bearing emitters (`AcquireCommodity`, `EscortToSafety`, `PatrolRoute`, `BountyHunt`), also probe `route_segment_blocked(segment_from, segment_to, tick)` along the candidate's intended path; for trade/Tell emitters (`BuyCommodity`, `AskWitness`, `ContractNegotiate`), also probe `counterparty_blocked(target_entity, tick)`.
- `crates/worldwake-ai/src/feasibility_probe.rs:42` — same migration: `BlockerKey` construction wraps in `BlockerScope::Exact(...)`, plus scope-aware checks for path traversal and counterparty.
- `crates/worldwake-ai/src/search/candidates.rs:1336` `find_blocked_for_search` — wrap legacy 4-arg call in `BlockerScope::Exact(...)`; add path-traversal check for search-time successors that traverse a route segment.

### 6. Update the three recording sites with `source_event` capture

- `crates/worldwake-ai/src/agent_tick/execution.rs:1341` — capture the originating event ID (the `ContentionEvent` or `ActionAbortedEvent` whose payload triggered the recording) from the surrounding tick context.
- `crates/worldwake-ai/src/agent_tick/observation.rs:626` — capture the originating `PerceptionEvent` ID. When the perception is a witnessed dangerous traversal, additionally record a `BlockerScope::RouteSegment(...)` entry alongside the existing Exact-scope one.
- `crates/worldwake-ai/src/failure_handling.rs:224` — capture the `ExpectationMismatchPayload` or `SourceExpectationFailurePayload` event ID. When the expectation mismatch is `PartyDeclined`, additionally record a `BlockerScope::Counterparty(...)` entry.

### 7. Update systems-layer counterparty recording

In `crates/worldwake-systems/src/trade_actions.rs:1920-1930`, the `BlockingFact::NoBuyer` recording path rewrites to use `BlockerScope::Counterparty(counterparty_id)` (rather than `BlockerScope::Exact(BlockerKey { goal_key, place: Some(seller_place), target: Some(counterparty_id), action_def: ... })`) when the failure is "this specific buyer refused"; preserves `BlockerScope::Exact(...)` for the goal-keyed case. Captures `source_event` from the failed-trade commit event.

### 8. Bump save format

`crates/worldwake-sim/src/save_load.rs`: bump `SAVE_FORMAT_VERSION: u32 = 85` to `86`. Update the serialization round-trip path for `BlockerMemory` and `DiscrepancyMemory` to use the new scope-keyed shape. No load-from-old-version path is added per FND-28 (no back-compat shim).

### 9. Update all remaining `BlockerKey` consumer sites

The full `BlockerKey` reference list (181 sites, 27 files) was enumerated in spec D3's migration blast-radius section. Each consumer that constructs `BlockerKey { ... }` and passes it as a map key wraps it in `BlockerScope::Exact(...)`. Consumers that destructure `blocker.blocker_key` switch to `blocker.scope` (handling all three variants). Recording-side helpers (`crates/worldwake-ai/src/feasibility.rs:579-589`, `crates/worldwake-ai/src/agenda_manager.rs:569-570`, etc.) update accordingly.

### 10. Trait-bound regression coverage

In `blocker_memory.rs` `#[cfg(test)]`, extend `blocker_types_satisfy_required_bounds` to include `assert_copy_value_bounds::<BlockerScope>()` and `assert_copy_value_bounds::<RouteSegment>()`. Add roundtrip tests for `BlockerScope::Exact(...)`, `BlockerScope::RouteSegment(...)`, `BlockerScope::Counterparty(...)` and for `Blocker`/`DiscrepancyEntry` with non-zero `source_event` values.

## Files to Touch

- `crates/worldwake-core/src/blocker_scope.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify) — module declaration + re-export
- `crates/worldwake-core/src/blocker_memory.rs` (modify)
- `crates/worldwake-core/src/discrepancy.rs` (modify)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify)
- `crates/worldwake-core/src/test_utils.rs` (modify) — `sample_blocker`, `sample_blocker_key`, plus a new `sample_blocker_scope` helper
- `crates/worldwake-sim/src/save_load.rs` (modify) — SAVE_FORMAT_VERSION bump + serialization
- `crates/worldwake-systems/src/trade_actions.rs` (modify) — NoBuyer recording with `BlockerScope::Counterparty(...)`
- `crates/worldwake-ai/src/agenda_manager.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify) — recording site + source_event capture
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify) — recording site + source_event capture
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — scope-aware read site
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify) — recording site + source_event capture
- `crates/worldwake-ai/src/feasibility.rs` (modify)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify) — scope-aware read site
- `crates/worldwake-ai/src/search/candidates.rs` (modify) — scope-aware read site
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify)
- `crates/worldwake-ai/tests/golden_contention_inspectability.rs` (modify)
- `crates/worldwake-ai/tests/golden_need_projection.rs` (modify)
- `crates/worldwake-ai/tests/golden_plan_repair.rs` (modify)
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/need_projection_assertions.rs` (modify)
- `crates/worldwake-cli/src/bin/observer.rs` (modify) — preserve existing decision-history rendering against the new payload shape (D7's distinct-per-variant rendering refinement lives in ticket 004)

## Out of Scope

- **`BlockerClearingCondition` new variants** (`RouteRetraversedSafely(RouteSegment)`, `CounterpartyAccepted(EntityId)`) — ticket 003. The recording paths in this ticket use `BlockerClearingCondition::TtlOnly` as the default clearing condition for new `RouteSegment` and `Counterparty` blockers.
- **Observer Section 3b typed-scope rendering** — ticket 004. This ticket preserves the existing debug formatting against the new payload shape; ticket 004 refines the rendering to display each scope variant distinctly.
- **S144 per-scope blocker diagnostics** — ticket 005.
- **`golden_cross_goal_blocker_scoping.rs` E2E coverage** — ticket 006.
- **Per-scope blocker TTL fields on `CognitiveProfile`** — already landed in ticket 001 (foundation dep).
- **Eliminating `BlockerKey` itself**: the type continues to exist as the payload of `BlockerScope::Exact(_)`. The migration eliminates `BlockerKey` as a *map key* and as a *Blocker struct field*, not as a type.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core --lib blocker_memory` — all 22 existing tests pass with scope-keyed migration; new trait-bound tests for `BlockerScope`/`RouteSegment` pass; new helper tests for `route_segment_blocked`/`counterparty_blocked`/`any_blocker_on_path` pass.
2. `cargo test -p worldwake-core --lib discrepancy` — all 13 existing tests pass with scope-keyed migration; new roundtrip test with `BlockerScope::RouteSegment` / `Counterparty` keys passes.
3. `cargo test -p worldwake-core --lib decision_event_payload` — `blocker_recorded_payload_roundtrips_with_belief_snapshot_some/none` pass with the new `scope` field.
4. `cargo test -p worldwake-ai --lib feasibility_probe` — `probe_rejects_on_blocker_memory_hit`, `probe_rejects_on_discrepancy_memory_hit` pass with scope-keyed lookups.
5. `cargo test -p worldwake-ai --test golden_portfolio_planning`, `--test golden_plan_repair`, `--test golden_contention_inspectability`, `--test golden_need_projection` — all existing blocker-exercising goldens pass unchanged (`BlockerScope::Exact(...)` semantics preserve the prior behavior).
6. `cargo test -p worldwake-sim --lib save_load` — `SAVE_FORMAT_VERSION = 86` round-trip test passes.
7. Workspace: `./scripts/verify.sh` clean.

### Invariants

1. **FND-28 single-truth**: No live code reads `BlockerKey` as a map key after this ticket. The type exists only as the payload of `BlockerScope::Exact(_)`. (Verified by `grep -rn "BTreeMap<BlockerKey" crates/` returning zero matches.)
2. **Cross-store key symmetry**: `BlockerMemory` and `DiscrepancyMemory` are both keyed on `BlockerScope`. No drift between the two stores' key types.
3. **`source_event` provenance**: Every `Blocker` and `DiscrepancyEntry` constructed by the recording paths carries a non-default `EventId` that points to an event present in the agent's append-only log at the time of recording.
4. **`Blocker.scope` and map key coherence**: For every `(scope, blocker)` entry in `BlockerMemory.intents`, `blocker.scope == scope` (the map key and the struct field are the same value; insertion logic in `record()` enforces this).
5. **`RouteSegment` canonical ordering**: `RouteSegment::new(A, B) == RouteSegment::new(B, A)` for all `A, B: EntityId`.
6. **Save format compatibility**: Saving and loading a world with scope-keyed `BlockerMemory` and `DiscrepancyMemory` round-trips byte-identical through `SAVE_FORMAT_VERSION = 86`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocker_memory.rs` — new unit tests for `route_segment_blocked`, `counterparty_blocked`, `any_blocker_on_path`, scope-canonical ordering, trait-bound regression for `BlockerScope`/`RouteSegment`; migrate all 22 existing tests to scope-keyed.
2. `crates/worldwake-core/src/discrepancy.rs` — new unit tests for `DiscrepancyMemory` with non-Exact scopes; migrate all 13 existing tests.
3. `crates/worldwake-core/src/decision_event_payload.rs` — migrate `blocker_recorded_payload_roundtrips_with_belief_snapshot_some/none`; update `sample_decision_payloads` BlockerRecordedPayload fixture.
4. `crates/worldwake-core/src/blocker_scope.rs` (new) — focused unit tests for `BlockerScope` variant ordering, `RouteSegment::new` canonical ordering, serialization roundtrip for each variant.
5. `crates/worldwake-sim/src/save_load.rs` — extend save-format roundtrip tests to include scope-keyed `BlockerMemory` and `DiscrepancyMemory`.
6. `crates/worldwake-ai/src/feasibility_probe.rs` — migrate `probe_rejects_on_blocker_memory_hit`, `probe_rejects_on_discrepancy_memory_hit`.

### Commands

1. `cargo test -p worldwake-core` — substrate-level tests.
2. `cargo test -p worldwake-ai --test golden_portfolio_planning --test golden_plan_repair --test golden_contention_inspectability --test golden_need_projection` — existing blocker goldens unchanged.
3. `cargo test -p worldwake-sim` — save-format roundtrip.
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh` for the full pre-PR gate.

Merge note: Ticket 002 bumps `SAVE_FORMAT_VERSION` 85→86; no sibling ticket in this spec bumps the value. Tickets 003-006 are additive against the new payload shape and do not re-bump.
