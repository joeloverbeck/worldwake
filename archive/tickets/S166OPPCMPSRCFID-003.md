# S166OPPCMPSRCFID-003: Derive `source_belief.status` via shared helper

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — opportunity compiler reads belief envelope status instead of stamping `Probable`. Candidate-generation parity preserved (same opportunity keys emit); per-opportunity `source_belief.status` value changes to reflect actual belief state.
**Deps**: `archive/tickets/S166OPPCMPSRCFID-001.md` (shared `belief_status_tag_for_claim` helper); spec `archive/specs/S166-opportunity-compiler-source-fidelity.md` (D1)

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
2. The shared `belief_status_tag_for_claim` helper exists after ticket 001 lands at `crates/worldwake-ai/src/belief_status.rs`. Its signature: `pub(crate) fn belief_status_tag_for_claim(view: &dyn RuntimeBeliefView, agent: EntityId, claim: &EntityBeliefClaim, tick: Tick) -> BeliefStatusTag`. Ticket 001's archived closeout confirms the helper covers the four confidence/refutation branches (`Certain`, `Probable`, `Stale`, `Contradicted`) and intentionally does not derive `Disputed`.
3. Shared abstraction boundary under audit: the `BeliefClaimKey → EntityBeliefClaim` lookup surface on `RuntimeBeliefView`. The live view exposes the backing `AgentBeliefStore` through `SocialBeliefView::agent_belief_store(agent)`, and `AgentBeliefStore::get_entity_claims(&subject)` returns raw claims. There is no `claim_for_key` accessor, so this ticket uses the existing store surface and filters by `claim.aspect == claim_key.aspect`.
4. Disputed status correction: `BeliefStatusTag::Disputed` is not produced by `belief_status_tag_for_claim`. Existing AI call sites (`agent_tick/frame.rs::belief_status_matches`, `agenda_manager.rs::belief_status_matches`) detect disputed state separately when multiple non-refuted claims exist. For the opportunity compiler, `Disputed` is therefore derived from multiple active claims for the same inventory aspect; otherwise the single matching claim is passed to the shared helper. This keeps the confidence/refutation helper canonical without pretending it owns contradiction-between-active-claims semantics.
5. Existing inline tests in `crates/worldwake-ai/src/opportunity_compiler/compile.rs::tests` (named in ticket 002's Assumption 5: `compile_opportunities_emits_inventory_backed_opportunities`, `compile_opportunities_does_not_anchor_acquisition_on_self_inventory`, `compile_opportunities_applies_floor_damping_and_cap`, `compile_opportunities_skips_confirmed_empty_survey_places`, `compile_opportunities_damps_learned_memory_entries`) construct summary beliefs directly and do not currently record raw `EntityBeliefClaim`s. None assert `source_belief.status == BeliefStatusTag::Probable` directly, so they continue to pass through the defensive `Stale` fallback. New focused tests record raw claims explicitly to prove the status derivation.
6. Information-path classification: the same `(claim refutation + effective confidence + threshold → BeliefStatusTag)` derivation is consumed by `agent_tick/frame.rs` and `agenda_manager.rs` today (via the shared helper after ticket 001). After this ticket the opportunity compiler is the third lawful consumer for non-disputed claims. `Disputed` remains a separate multi-active-claim predicate, matching the live pattern at the other two call sites. No duplicate confidence/refutation derivation is created.
7. Intended verification layer: focused unit coverage on `source_belief()` across all 5 `BeliefStatusTag` cases. This is the right layer because the derivation is a pure read-model computation on the belief envelope; a decision-trace assertion would be a downstream proxy.
8. Heuristic removal discipline: this ticket *removes* the `BeliefStatusTag::Probable` literal stand-in. The "missing substrate" the literal stood in for was the belief envelope's actual status axis; that substrate is already wired through `RuntimeBeliefView` via `agent_belief_store(agent)` and the shared helper. The change does not reopen regressions in unrelated scenarios — only the opportunity record's `source_belief.status` value changes, and the field is read by zero runtime consumers today (it surfaces in the observer's display sort at `observer.rs:812` as a tie-breaker, which is cosmetic).
9. Cap-truncation tie-break consequence: `compile.rs:140-146` sorts opportunities by `(Reverse(salience), key, source_belief)` and truncates to `compile_opportunity_cap` at line 147-150. The `source_belief` tie-breaker includes `BeliefStatusTag` (via `BeliefRef`'s derived `Ord`). When status flips from always-`Probable` to mixed values, the cap-truncated subset can change only for otherwise-identical sort keys. The live proof locks deterministic truncation under mixed statuses rather than asserting byte-for-byte pre-change subset parity, which the new truthful tie-breaker intentionally does not guarantee for exact ties.

## Architecture Check

1. Reusing the shared helper from ticket 001 means S166's source-status fix does not create a third `belief_status_tag_for_claim` definition (FND-28). The same arithmetic that drives `any_claim_has_status` in `frame.rs` and `agenda_manager.rs` now drives the opportunity compiler's status field — one canonical derivation, three consumers.
2. The `source_belief()` rewrite preserves the existing `BeliefRef` return shape (no `Opportunity` field-shape change), keeping ticket 002's "no shape change, no new emitter" guarantee. Only the `status` field's *value* changes.
3. The claim-lookup mechanism (find the `EntityBeliefClaim` matching the constructed `BeliefClaimKey`) is the minimum new substrate needed to bridge the input shape difference: the existing helper takes `&EntityBeliefClaim`, but `source_belief()` already has a `BelievedEntityState` summary in hand. The live implementation uses `view.agent_belief_store(agent).and_then(|store| store.get_entity_claims(&entity))` and filters by aspect. That is O(claims-per-entity) and stays inside the existing belief-view/store boundary without widening `RuntimeBeliefView`.

## Verified Layers

1. Status derivation correctness across all 5 `BeliefStatusTag` cases (`Certain`, `Probable`, `Stale`, `Disputed`, `Contradicted`) → focused unit test in `opportunity_compiler/compile.rs::tests` constructing beliefs whose claim state triggers each branch and asserting the resulting `source_belief.status`.
2. Candidate-key parity → focused unit test asserting the set of `(OpportunityKey)` keys emitted by `compile_opportunities` is unchanged across mixed status variations for a representative acquisition fixture. (Only the `source_belief.status` field on each emitted opportunity differs; the opportunity set is identical.)
3. Cap-truncation determinism → focused unit test that constructs ≥`compile_opportunity_cap + 1` same-salience opportunities with mixed source-belief statuses, then asserts the truncated subset follows the deterministic `(Reverse(salience), key, source_belief)` ordering. This proves the live post-change contract without preserving the old all-`Probable` tie-breaker.
4. Acquisition golden no-regression → existing `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` goldens and any survival/acquisition E2E goldens pass unchanged.

## Landed Changes

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
    let status = source_belief_status(view, agent, &claim_key, tick);
    BeliefRef {
        claim_key,
        claim_held_at_tick: state.last_observed_tick().unwrap_or(Tick(0)),
        status,
    }
}
```

`source_belief_status()` uses the existing `agent_belief_store(agent)` + `get_entity_claims(&subject)` surface. If more than one non-refuted claim exists for the same inventory aspect, it returns `Disputed`. Otherwise it calls `belief_status_tag_for_claim()` for the matching claim and falls back to `Stale` for the defensive impossible-in-practice case where a `BelievedEntityState` summary exists but no raw matching claim exists.

Import `crate::belief_status::belief_status_tag_for_claim` at the file's import block.

### 2. Update the single call site at `crates/worldwake-ai/src/opportunity_compiler/compile.rs:121`

From `source_belief: source_belief(entity, commodity, &state)` to `source_belief: source_belief(belief_view, agent, entity, commodity, &state, current_tick)`. Both `belief_view` and `agent` are already in scope at the per-entity loop. `current_tick` is bound at `compile.rs:30` (`let current_tick = belief_view.current_tick();`) and is available.

### 3. Added focused tests for the status-derivation matrix

In `crates/worldwake-ai/src/opportunity_compiler/compile.rs::tests`, add 5 tests (or one parameterized test if the existing test style supports it) covering each `BeliefStatusTag` case:

- `compile_opportunities_emits_certain_status_for_high_confidence_observation`
- `compile_opportunities_emits_probable_status_for_threshold_observation`
- `compile_opportunities_emits_stale_status_for_decayed_observation`
- `compile_opportunities_emits_disputed_status_for_disputed_claim`
- `compile_opportunities_emits_contradicted_status_for_refuted_claim`

Each test constructs an `AgentBeliefStore` with a claim shaped to produce the target status (varying `acquired_tick`, `refuted_at_tick`, and source confidence), runs `compile_opportunities`, and asserts the emitted opportunity's `source_belief.status` matches the expected tag.

### 4. Added focused tests for parity and deterministic truncation

- `compile_opportunities_emits_same_keys_across_status_variations`: constructs an `AgentBeliefStore` with multiple known entities at mixed freshness, runs `compile_opportunities`, asserts the set of `(opportunity.key)` values matches a baseline (recorded once via a pre-change fixture or computed inline).
- `compile_opportunities_cap_truncation_is_deterministic_under_status_tie_break`: constructs enough opportunities to exceed `compile_opportunity_cap`, with equal salience and mixed source-belief status, asserts the cap-truncated subset matches the deterministic post-change sort baseline.

## Landed Files

- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify — rewrite `source_belief()`, update call site, add focused tests)

## Out of Scope

- Modifying any other `source_belief.status` reader. The only current production touch is the observer's display sort (`observer.rs:812`), which uses `BeliefRef` as a sort tie-breaker — cosmetic ordering change in the observer's display only, no logic change needed.
- Modifying `Opportunity`'s field shape (preserved by spec D1's "no shape change" rule).
- Adding the `compiled_by_status` diagnostics field — that belongs to ticket 004 (D3).
- Extending `RuntimeBeliefView` with new accessors. The landed implementation uses the existing `agent_belief_store(agent)` plus claim-list lookup surface instead.

## Acceptance Result

### Tests Passed

1. The 5 new status-derivation tests assert the correct `BeliefStatusTag` for each input case.
2. The 2 new parity tests assert opportunity-key set unchanged and cap-truncated subset deterministic under mixed status tie-breaks.
3. The 5 existing inline tests in `compile.rs::tests` (named in Assumption 4) pass unchanged.
4. `crates/worldwake-ai/tests/scenarios/opportunity_compiler.rs` golden tests pass unchanged.
5. `cargo test -p worldwake-ai` — full AI crate suite passes.

### Invariants

1. `source_belief.status` is never `BeliefStatusTag::Probable` *by construction* (the literal is removed). Any `Probable` status in an emitted opportunity now reflects a real per-agent confidence band derivation.
2. Opportunity-key set emitted by `compile_opportunities` is identical for the same `(belief_view, action_index)` inputs. Only the `source_belief.status` field's value can differ.
3. The cap-truncated opportunity subset is deterministic for fixed inputs — same agent, same belief store, same tick → same truncated set.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (inline `tests` module) — 5 status-derivation tests (per item 3 in Landed Changes).
2. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (inline `tests` module) — 2 parity/determinism tests (per item 4 in Landed Changes).
3. No modifications to existing inline tests; their fixtures fall into the `Certain`/`Probable` bands by construction (per Assumption 4).

### Commands Run

1. `cargo test -p worldwake-ai --lib opportunity_compiler::compile::tests` — targets the new status-derivation and parity tests.
2. `cargo test -p worldwake-ai --test golden_ai opportunity_compiler` — exercises the existing golden scenario.
3. `cargo test -p worldwake-ai` — full AI crate suite for no-regression.
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — clippy gate.
5. `./scripts/verify.sh` — waived for this per-ticket closeout because the `implement-spec-tickets` final branch phase owns the full pre-PR gate before push.

## Outcome

Completed on 2026-05-24.

- `compile_opportunities` now populates `Opportunity.source_belief.status` from the backing belief claims instead of stamping `BeliefStatusTag::Probable`.
- Non-disputed claims route through the shared `belief_status_tag_for_claim` helper from ticket 001. Competing active inventory claims for the same `BeliefClaimKey` produce `BeliefStatusTag::Disputed`, matching the separate disputed-claim predicate used by the existing AI call sites.
- Added focused compiler tests for `Certain`, `Probable`, `Stale`, `Disputed`, and `Contradicted`, plus opportunity-key parity and deterministic cap-truncation coverage under mixed source statuses.
- No opportunity shape change, no new emitter, no `RuntimeBeliefView` trait expansion, and no diagnostics-field work landed; ticket 004 still owns diagnostics aggregation.

## Deviations

- The drafted `claim_for_key` accessor does not exist on the live branch. The landed lookup uses the existing `agent_belief_store(agent)` plus `AgentBeliefStore::get_entity_claims(&subject)` surface and filters by aspect.
- The shared helper from ticket 001 does not derive `Disputed`; `Disputed` is derived by the compiler when multiple non-refuted claims exist for the same inventory aspect, then single-claim status falls through to the helper.
- The cap-truncation proof locks the deterministic post-change ordering for mixed-status opportunities. It does not claim byte-identical pre-change truncation for impossible duplicate-key ties under the old all-`Probable` literal.
- Existing compiler tests that directly insert `known_entities` without raw claims continue through the defensive `Stale` fallback; the new status tests record raw claims explicitly.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib opportunity_compiler::compile::tests`.
- Passed `cargo test -p worldwake-ai --test golden_ai opportunity_compiler`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Waived `./scripts/verify.sh` for this per-ticket closeout because the `implement-spec-tickets` final branch phase owns the full pre-PR gate before push.
