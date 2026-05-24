# S166OPPCMPSRCFID-003: Derive `source_belief.status` via shared helper

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — opportunity compiler reads belief envelope status instead of stamping `Probable`. Candidate-generation parity preserved (same opportunity keys emit); per-opportunity `source_belief.status` value changes to reflect actual belief state.
**Deps**: ticket 001 (shared `belief_status_tag_for_claim` helper); spec `specs/S166-opportunity-compiler-source-fidelity.md` (D1)

## Problem

`crates/worldwake-ai/src/opportunity_compiler/compile.rs:222` stamps
`status: BeliefStatusTag::Probable` on every compiled opportunity's
`source_belief` regardless of the underlying belief's actual freshness or
refutation state. A stale, disputed, or contradicted inventory belief produces
an opportunity that *claims to be probable* — violating FND-27 (summaries must
not over-assert provenance) and weakening FND-15 (knowledge carriers must carry
real freshness). This ticket rewrites `source_belief()` to derive the status by
calling the shared `belief_status_tag_for_claim` helper lifted by ticket 001,
looking up the corresponding `EntityBeliefClaim` from the `RuntimeBeliefView`
already in scope.

## Assumption Reassessment (2026-05-24)

1. `source_belief()` at `crates/worldwake-ai/src/opportunity_compiler/compile.rs:211-224` currently takes `(entity: EntityId, commodity: CommodityKind, state: &worldwake_core::BelievedEntityState)` and returns `BeliefRef`. It constructs the `BeliefClaimKey` inline at lines 217-220 (`subject: entity, aspect: EntityBeliefAspect::Inventory(commodity)`) and uses `state.last_observed_tick().unwrap_or(Tick(0))` for `claim_held_at_tick`. The single call site is `compile.rs:121`, inside the per-commodity emission loop. The function gains the belief-view reference and agent id from this call site — both already in scope (`belief_view: &impl RuntimeBeliefView` at `compile_opportunities` line 19 and the per-loop `agent: EntityId` parameter at line 18).
2. The shared `belief_status_tag_for_claim` helper exists after ticket 001 lands at `crates/worldwake-ai/src/belief_status.rs`. Its signature: `pub(crate) fn belief_status_tag_for_claim(view: &dyn RuntimeBeliefView, agent: EntityId, claim: &EntityBeliefClaim, tick: Tick) -> BeliefStatusTag`.
3. Shared abstraction boundary under audit: the `BeliefClaimKey → EntityBeliefClaim` lookup surface on `RuntimeBeliefView`. The view exposes per-claim access through its existing `known_entity_beliefs(agent)` iterator (already used at `compile.rs:45`) and the `AgentBeliefStore` it backs. Implementation must locate the specific accessor that returns `&EntityBeliefClaim` for a given `BeliefClaimKey` — likely a method on `RuntimeBeliefView` or `BeliefRead`-style access through `AgentBeliefStore`. If no such accessor exists, the compiler can search the per-entity `BelievedEntityState`'s claim list for the one matching the `(subject, aspect)` key (the same key already constructed at `compile.rs:217-220`).
4. Existing inline tests in `crates/worldwake-ai/src/opportunity_compiler/compile.rs::tests` (named in ticket 002's Assumption 5: `compile_opportunities_emits_inventory_backed_opportunities:327`, `compile_opportunities_does_not_anchor_acquisition_on_self_inventory:358`, `compile_opportunities_applies_floor_damping_and_cap:407`, `compile_opportunities_skips_confirmed_empty_survey_places:444`, `compile_opportunities_damps_learned_memory_entries:480`) construct beliefs via `BelievedEntityState::single_observation_defaults(Tick(10), PerceptionSource::DirectObservation)`. These fixtures produce high-confidence direct observations that derive to `Certain` or `Probable` (depending on the per-agent threshold). None of the existing tests assert `source_belief.status == BeliefStatusTag::Probable` directly (verified by grep), so they continue to pass without modification — the test fixtures happen to fall into the `Probable`/`Certain` bands.
5. Information-path classification: the same `(claim refutation + effective confidence + threshold → BeliefStatusTag)` derivation is consumed by `agent_tick/frame.rs` and `agenda_manager.rs` today (via the shared helper after ticket 001). After this ticket the opportunity compiler is the third lawful consumer; all three use the canonical helper. No duplicate transport path is created.
6. Intended verification layer: focused unit coverage on `source_belief()` across all `BeliefStatusTag` cases. This is the right layer because the derivation is a pure read-model computation on the belief envelope; a decision-trace assertion would be a downstream proxy.
7. Heuristic removal discipline: this ticket *removes* the `BeliefStatusTag::Probable` literal stand-in. The "missing substrate" the literal stood in for was the belief envelope's actual status axis; that substrate is already wired through `RuntimeBeliefView` (used today by `frame.rs` and `agenda_manager.rs`'s `any_claim_has_status` callers). The change does not reopen regressions in unrelated scenarios — only the opportunity record's `source_belief.status` value changes, and the field is read by zero runtime consumers today (it surfaces in the observer's display sort at `observer.rs:812` as a tie-breaker, which is cosmetic).
8. Cap-truncation tie-break consequence: `compile.rs:140-146` sorts opportunities by `(Reverse(salience), key, source_belief)` and truncates to `compile_opportunity_cap` at line 147-150. The `source_belief` tie-breaker includes `BeliefStatusTag` (via `BeliefRef`'s derived `Ord`). When status flips from always-`Probable` to mixed values, the cap-truncated subset can change for opportunities sharing `(salience, key)` but differing in status. A cap-stress parity scenario locks this — see Test Plan.

## Architecture Check

1. Reusing the shared helper from ticket 001 means S166's source-status fix does not create a third `belief_status_tag_for_claim` definition (FND-28). The same arithmetic that drives `any_claim_has_status` in `frame.rs` and `agenda_manager.rs` now drives the opportunity compiler's status field — one canonical derivation, three consumers.
2. The `source_belief()` rewrite preserves the existing `BeliefRef` return shape (no `Opportunity` field-shape change), keeping ticket 002's "no shape change, no new emitter" guarantee. Only the `status` field's *value* changes.
3. The claim-lookup mechanism (find the `EntityBeliefClaim` matching the constructed `BeliefClaimKey`) is the minimum new substrate needed to bridge the input shape difference: the existing helper takes `&EntityBeliefClaim`, but `source_belief()` already has `BelievedEntityState` in hand. Either the view exposes a direct `claim_for_key(agent, key)` accessor (preferred) or the compiler searches the per-entity claim list. The implementation picks the simpler route at write time; both are O(1) or O(claims-per-entity) and not in a hot enough loop to matter.

## Verification Layers

1. Status derivation correctness across all 5 `BeliefStatusTag` cases (`Certain`, `Probable`, `Stale`, `Disputed`, `Contradicted`) → focused unit test in `opportunity_compiler/compile.rs::tests` constructing beliefs whose claim state triggers each branch and asserting the resulting `source_belief.status`.
2. Candidate-key parity → focused unit test asserting the set of `(OpportunityKey)` keys emitted by `compile_opportunities` is unchanged across the before/after change for a representative acquisition fixture. (Only the `source_belief.status` field on each emitted opportunity differs; the opportunity set is identical.)
3. Cap-truncation parity → focused unit test that constructs ≥`compile_opportunity_cap + 1` opportunities all sharing identical `(salience, key)` but differing in source-belief status, then asserts the cap-truncated subset is the same as the pre-change subset (the tie-breaker ordering is locked by the derived status).
4. Acquisition golden no-regression → existing `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` goldens and any survival/acquisition E2E goldens pass unchanged.

## What to Change

### 1. Rewrite `source_belief()` in `crates/worldwake-ai/src/opportunity_compiler/compile.rs`

Replace the existing function (lines 211-224) with one that takes the belief view, agent id, and tick:

```rust
fn source_belief(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    entity: EntityId,
    commodity: CommodityKind,
    state: &worldwake_core::BelievedEntityState,
    tick: Tick,
) -> BeliefRef {
    let claim_key = BeliefClaimKey {
        subject: entity,
        aspect: EntityBeliefAspect::Inventory(commodity),
    };
    let status = view
        .claim_for_key(agent, &claim_key)
        .map(|claim| belief_status_tag_for_claim(view, agent, claim, tick))
        .unwrap_or(BeliefStatusTag::Stale);
    BeliefRef {
        claim_key,
        claim_held_at_tick: state.last_observed_tick().unwrap_or(Tick(0)),
        status,
    }
}
```

The `view.claim_for_key(agent, &claim_key)` is illustrative — the implementer must use whatever accessor `RuntimeBeliefView` actually exposes (or extend the view minimally if no direct accessor exists; see Assumption 3 for the search fallback). The `.unwrap_or(BeliefStatusTag::Stale)` defensive fallback handles the impossible-in-practice case where a `BelievedEntityState` exists but no matching claim is found in the store — falling through to `Stale` rather than panicking preserves emission.

Import `crate::belief_status::belief_status_tag_for_claim` at the file's import block.

### 2. Update the single call site at `crates/worldwake-ai/src/opportunity_compiler/compile.rs:121`

From `source_belief: source_belief(entity, commodity, &state)` to `source_belief: source_belief(belief_view, agent, entity, commodity, &state, current_tick)`. Both `belief_view` and `agent` are already in scope at the per-entity loop. `current_tick` is bound at `compile.rs:30` (`let current_tick = belief_view.current_tick();`) and is available.

### 3. Add focused tests for the status-derivation matrix

In `crates/worldwake-ai/src/opportunity_compiler/compile.rs::tests`, add 5 tests (or one parameterized test if the existing test style supports it) covering each `BeliefStatusTag` case:

- `compile_opportunities_emits_certain_status_for_high_confidence_observation`
- `compile_opportunities_emits_probable_status_for_threshold_observation`
- `compile_opportunities_emits_stale_status_for_decayed_observation`
- `compile_opportunities_emits_disputed_status_for_disputed_claim`
- `compile_opportunities_emits_contradicted_status_for_refuted_claim`

Each test constructs an `AgentBeliefStore` with a claim shaped to produce the target status (varying `acquired_tick`, `refuted_at_tick`, and source confidence), runs `compile_opportunities`, and asserts the emitted opportunity's `source_belief.status` matches the expected tag.

### 4. Add focused tests for parity

- `compile_opportunities_emits_same_keys_across_status_variations`: constructs an `AgentBeliefStore` with multiple known entities at mixed freshness, runs `compile_opportunities`, asserts the set of `(opportunity.key)` values matches a baseline (recorded once via a pre-change fixture or computed inline).
- `compile_opportunities_cap_truncation_is_stable_under_status_tie_break`: constructs enough opportunities to exceed `compile_opportunity_cap`, all sharing identical `(salience, key)` but differing in source-belief status, asserts the cap-truncated subset matches a deterministic baseline.

## Files to Touch

- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify — rewrite `source_belief()`, update call site, add focused tests)

## Out of Scope

- Modifying any other `source_belief.status` reader. The only current production touch is the observer's display sort (`observer.rs:812`), which uses `BeliefRef` as a sort tie-breaker — cosmetic ordering change in the observer's display only, no logic change needed.
- Modifying `Opportunity`'s field shape (preserved by spec D1's "no shape change" rule).
- Adding the `compiled_by_status` diagnostics field — that belongs to ticket 004 (D3).
- Extending `RuntimeBeliefView` with new accessors beyond the minimum needed for the claim-key lookup. If a direct `claim_for_key` accessor doesn't exist, prefer the search-the-claim-list fallback over expanding the trait surface.

## Acceptance Criteria

### Tests That Must Pass

1. The 5 new status-derivation tests assert the correct `BeliefStatusTag` for each input case.
2. The 2 new parity tests assert opportunity-key set unchanged and cap-truncated subset stable.
3. The 5 existing inline tests in `compile.rs::tests` (named in Assumption 4) pass unchanged.
4. `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` golden tests pass unchanged.
5. `cargo test -p worldwake-ai` — full AI crate suite passes.

### Invariants

1. `source_belief.status` is never `BeliefStatusTag::Probable` *by construction* (the literal is removed). Any `Probable` status in an emitted opportunity now reflects a real per-agent confidence band derivation.
2. Opportunity-key set emitted by `compile_opportunities` is identical before/after the change for the same `(belief_view, action_index)` inputs. Only the `source_belief.status` field's value can differ.
3. The cap-truncated opportunity subset is deterministic for fixed inputs — same agent, same belief store, same tick → same truncated set.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (inline `tests` module) — 5 new status-derivation tests (per item 3 in What to Change).
2. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (inline `tests` module) — 2 new parity tests (per item 4 in What to Change).
3. No modifications to existing inline tests; their fixtures fall into the `Certain`/`Probable` bands by construction (per Assumption 4).

### Commands

1. `cargo test -p worldwake-ai opportunity_compiler::compile::tests` — targets the new status-derivation and parity tests.
2. `cargo test -p worldwake-ai --test golden_ai opportunity_compiler` — exercises the existing golden scenario.
3. `cargo test -p worldwake-ai` — full AI crate suite for no-regression.
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — clippy gate.
5. `./scripts/verify.sh` — full pre-PR gate before pushing.
