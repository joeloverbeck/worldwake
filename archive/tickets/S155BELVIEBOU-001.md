# S155BELVIEBOU-001: Belief-correct `effective_place` for non-self entities

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` belief view (`SpatialBeliefView::effective_place` impl)
**Deps**: None (parallel with the now-archived `archive/tickets/S155BELVIEBOU-002.md`; shares `per_agent_belief_view.rs`, different method)

## Problem

Before this ticket, `PerAgentBeliefView`'s `SpatialBeliefView::effective_place` (`crates/worldwake-sim/src/per_agent_belief_view.rs`) leaked **live authoritative location** for non-self entities the agent had not co-located with this tick. The non-self path returned `believed_entity(entity).last_known_place`, then fell back via `knows_entity(entity).then(|| self.world.effective_place(entity))`. Because `knows_entity()` returns true for entities known **only** through `institutional_beliefs` (a social claim) or `LastSeenMemory` (stale, prior-tick), an agent that merely remembered or was told about a target obtained its *live* position — the omniscience FND-14/FND-14A forbid. This propagated into snapshot admission, strategic place selection, and candidate emission, producing intelligent-looking but omniscient pursuit.

## Assumption Reassessment (2026-05-20)

<!-- Spec S155 reassessed this session (/reassess-spec); abbreviated spot-check confirmed targets. -->

1. **Current code**: `effective_place` is the `SpatialBeliefView` impl at `per_agent_belief_view.rs:951`. Its non-self path uses the `knows_entity()`-gated `or_else` fallback to `self.world.effective_place(entity)`. `knows_entity()` (`:292`) returns true for `institutional_beliefs` subjects and `LastSeenMemory` records (both non-co-located). `has_authoritative_local_visibility()` (`:285`) is same-tick co-location only. `LastSeenRecord { place: EntityId, .. }` and `LastSeenMemory { records: BTreeMap<EntityId, LastSeenRecord>, .. }` are at `crates/worldwake-core/src/expectation.rs:126,136`; `get_component_last_seen_memory` exists (used by `knows_entity`). All confirmed.
2. **Current specs/docs**: `specs/S155-belief-view-boundary-correctness.md` D1 (post-reassessment, in-place). `docs/FOUNDATIONS.md` FND-14, FND-14A (same-tick co-location is the only legal non-self authoritative read; off-place/delayed knowledge must be belief-backed).
3. **Shared boundary under audit**: the `SpatialBeliefView::effective_place` accessor — the belief view's location answer consumed by planning/snapshot. The contract: non-self location is belief/last-seen only unless same-tick co-located or directly possessed.
4. **Intended invariant (restated before trusting any scenario)**: an agent that last saw entity T at P1 and received no new observation/testimony/record must read T's place as P1 (or `None`), never T's current P2.
5. **Existing focused coverage** (`per_agent_belief_view.rs` `#[cfg(test)]`): `self_expectation_and_last_seen_queries_are_authoritative_only_for_self:2457`, `directly_possessed_item_lot_quantity_uses_authoritative_quantity_over_stale_belief:2587`, `current_place_entities_use_authoritative_local_set_over_stale_beliefs:2621`, `stale_beliefs_do_not_auto_refresh_from_world:2846`. These cover adjacent accessors (expectation/last-seen self-queries, possessed-quantity, co-located set) but **not** `effective_place`'s non-self fallback specifically — confirm during implementation that none asserts the leaking behavior; if one does, it encodes a bug and must be corrected, not preserved (never adapt tests to bugs).
6. **AI regression layer**: this is a belief-view accessor fix; intended verification is a focused unit test on `PerAgentBeliefView` plus the full AI golden suite (place narrowing may shift existing golden traces — re-baseline expected trace shifts, not world-outcome regressions).
13. **Adjacent contradictions**: the `can_control` gate gap is a *separate* root cause handled by the now-archived `archive/tickets/S155BELVIEBOU-002.md` — out of scope here.

## Architecture Check

1. Narrowing `effective_place` at its single source is cleaner than tagging snapshot admission downstream (the S157 deferred approach): the leak is closed where it originates, so every consumer reads belief-correct locations with no per-consumer guard. Mirrors the existing FND-14A discipline already present for self/possession/co-location reads in this same impl block.
2. No backwards-compatibility shim: the `knows_entity()`-gated `or_else` branch is deleted, not wrapped. No alias accessor is introduced.

## Verified Layers

1. Non-self stale-location returns belief/last-seen place, never live truth → focused unit test on `PerAgentBeliefView` (decision-trace not needed at this layer; the accessor return value is the contract).
2. Same-tick co-located / directly-possessed reads still return authoritative location (FND-14A preserved) → focused unit test (positive cases).
3. No omniscient pursuit emerges downstream → full AI golden suite (`cargo test -p worldwake-ai`); covered end-to-end by S155BELVIEBOU-003's stale-location golden, not asserted here.

## Landed Changes

### 1. Rewrote the non-self path of `SpatialBeliefView::effective_place`

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, the non-self path now reaches authoritative `self.world.effective_place(entity)` **only** when `has_authoritative_local_visibility(entity)` (same-tick co-location) **or** `self.world.possessor_of(entity) == Some(self.agent)` (direct possession). Otherwise it returns, in order: `believed_entity(entity).and_then(|s| s.last_known_place)`, then the actor's last-seen record place (`get_component_last_seen_memory(self.agent).and_then(|m| m.records.get(&entity).map(|r| r.place))`), then `None`. The `knows_entity()`-gated `or_else` fallback to live truth was deleted.

### 2. Added focused unit tests

Added a last-seen-only stale-location regression test that failed against the pre-fix accessor, then passed after the fix. Added positive controls for co-located and directly-possessed non-self entities retaining authoritative reads.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs` — `effective_place` implementation plus focused unit coverage.
- `crates/worldwake-ai/src/agent_tick/tests.rs` — same-domain AI fallout update: a carried cargo lot now resolves to its authoritative destination place, so the post-arrival cargo test expects the immediate stocking action and the delivered container state.
- `specs/S155-belief-view-boundary-correctness.md` — active spec truth-sync marking D1 landed and correcting the active Authoritative-to-AI rule reference to `AGENTS.md`.

## Out of Scope

- `can_control` belief gate — now archived at `archive/tickets/S155BELVIEBOU-002.md`.
- Golden E2E stale-location pursuit + control-source-swap symmetry, and the `planner-contracts.md` doc line — S155BELVIEBOU-003.
- Snapshot admission-source provenance tagging — deferred to S157.
- Any change to authoritative `World::effective_place` itself.

## Acceptance Result

### Tests Passed

1. Non-self last-seen-only stale location returns the remembered place, never the moved authoritative place.
2. Co-located and directly-possessed non-self reads still return authoritative location (FND-14A preserved).
3. `cargo test -p worldwake-sim per_agent_belief_view` and `cargo test -p worldwake-ai` passed with one expected same-domain AI test update for the possessed-cargo place boundary.

### Invariants

1. For a non-self entity that is neither same-tick co-located nor directly possessed by the actor, `effective_place` never consults authoritative `World::effective_place` (FND-14/FND-14A).
2. The FND-14A same-tick co-location read and the direct-possession read remain authoritative.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (`#[cfg(test)]`) — stale-location negative case + co-location/possession positive controls for `effective_place`.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — updated `cargo_satisfaction_at_destination_while_carrying` to assert the post-S155 delivered-container state after the now-visible possessed-cargo place is used.

### Commands Run

1. `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::effective_place_uses_last_seen_without_refreshing_remote_truth -- --exact`
2. `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::effective_place_keeps_authoritative_reads_for_local_or_possessed_entities -- --exact`
3. `cargo test -p worldwake-sim per_agent_belief_view`
4. `cargo test -p worldwake-ai --lib agent_tick::tests::cargo_satisfaction_at_destination_while_carrying -- --exact`
5. `cargo test -p worldwake-ai`
6. `cargo fmt --all`

## Outcome

Completed on 2026-05-20.

- Removed the `knows_entity()`-gated authoritative `World::effective_place` fallback from the non-self `PerAgentBeliefView` path.
- Preserved authoritative location reads for self, same-tick co-located entities, and directly possessed entities.
- Added focused coverage for last-seen-only stale location and for the preserved local/possessed exceptions.
- Updated one AI cargo test whose old expectation depended on the possessed lot retaining stale believed placement after arrival; with the corrected boundary, the cargo action stocks the lot at the destination and the goal still parks suspended rather than being abandoned.
- Updated the active S155 spec so it no longer reads as though D2-D4 have already landed.

## Deviations

- `./scripts/verify.sh` was not run for this first ticket iteration; the harness reserves that pre-PR wrapper for final branch verification. This ticket's executable proof used the focused `worldwake-sim` accessor selector plus the full `worldwake-ai` suite required by the Authoritative-to-AI impact check.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::effective_place_uses_last_seen_without_refreshing_remote_truth -- --exact` after first observing the same command fail against the pre-fix accessor.
- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::effective_place_keeps_authoritative_reads_for_local_or_possessed_entities -- --exact`.
- Passed `cargo test -p worldwake-sim per_agent_belief_view`.
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::cargo_satisfaction_at_destination_while_carrying -- --exact`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo fmt --all`.
- Waived `./scripts/verify.sh` until final harness pre-push verification because this iteration's owned proof is covered by the focused accessor tests and full AI suite.
