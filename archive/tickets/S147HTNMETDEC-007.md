# S147HTNMETDEC-007: MethodSelector with deterministic ranking

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — adds the `MethodSelector` and `select_method()` function. Not yet wired into the planner (ticket 008).
**Deps**: `archive/tickets/S147HTNMETDEC-001.md` (MotiveSourceDiscriminant), `archive/tickets/S147HTNMETDEC-003.md` (`AgentSchemaContextProfile.disabled_methods`), `archive/tickets/S147HTNMETDEC-004.md` (MethodSchema + supporting types), `archive/tickets/S147HTNMETDEC-006.md` (MethodRegistry + explicit method binding templates)

## Problem

S147 D3 defines the deterministic method-selection algorithm: filter methods by goal kind and per-agent denylist, filter by precondition satisfaction against the actor's belief view, rank by motive-source bias score, tie-break by `MethodSchemaId`. Without this selector, the registry's content is unreachable from the planner. The selector must be pure (no side effects), deterministic (same inputs → same output), and integer-arithmetic-only (no floats per AGENTS.md determinism invariant).

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `MethodRegistry` and `methods_for(goal_kind)` exist after `archive/tickets/S147HTNMETDEC-006.md` landed at `crates/worldwake-ai/src/htn/registry.rs`. `MotiveSourceDiscriminant` and `From<&MotiveSource>` exist after `archive/tickets/S147HTNMETDEC-001.md` landed at `crates/worldwake-core/src/motive_source.rs`. Live reassessment corrected the drafted formula: `MotiveSourceRef` carries `source: MotiveSource` and `introduced_tick`, not a `weight`; method ranking therefore sums matching `MethodSchema.motive_bias[].weight` values for present motive-source kinds. Upstream goal-ranking already owns source magnitude through `RankedGoal.motive_source_contributions`.
2. `RuntimeBeliefView` trait exists at `crates/worldwake-sim/src/belief_view.rs:1588` (verified during S147 reassessment). The selector reads via `&dyn RuntimeBeliefView` plus an explicit `actor: EntityId` parameter — no new trait accessor is required because `BeliefPredicate` variants (from `archive/tickets/S147HTNMETDEC-004.md`) compose existing agent-relative reads.
3. `AgentSchemaContextProfile.disabled_methods` exists after `archive/tickets/S147HTNMETDEC-003.md` landed at `crates/worldwake-core/src/agent_schema_context_profile.rs`.
4. Shared boundary: `select_method()` is the only function the planner integration (ticket 008) calls into the htn module. The function signature is the contract; the planner does not reach into the registry or supporting types directly.
5. Ranking formula (corrected against the live S141 motive-source contract): integer-only sum of matching `Permille::value()` method biases for present motive-source discriminants. No float arithmetic — satisfies AGENTS.md determinism invariant. Tie-break by `MethodSchemaId` ordinal (deterministic via `BTreeMap` iteration order).
6. Ticket 006 corrected the method-schema contract from concrete runtime IDs to explicit `EntityTemplate`, `CommodityTemplate`, and `RecipeTemplate` bindings. Selector precondition evaluation must resolve those templates against the live `GoalOffer`, actor, and belief view before asking whether a predicate holds; it must not compare hidden sentinel IDs.

## Architecture Check

1. The selector is a pure function — no mutation of the registry, profile, belief view, or motives. This keeps method selection deterministic and side-effect-free, which is required for replay equivalence.
2. The function returns `Option<&'r MethodSchema>` rather than `Result` because "no method applicable" is a normal outcome (falls back to flat GOAP per spec D4). No-method is not an error.
3. Integer-only ranking by matching method-bias `Permille::value()` preserves the no-floats invariant. The score is a `u32` sum over the method's bounded bias list and present motive-source discriminants.
4. No backwards-compatibility shims. The selector is net-new.

## Verified Layers

1. Deterministic selection (same inputs → same output across runs) → focused unit test that calls `select_method` twice with identical inputs and asserts pointer equality of the returned `&MethodSchema`.
2. Denylist honored → focused unit test that calls `select_method` with `profile.disabled_methods` containing the otherwise-top-ranked method ID and asserts the next-ranked method is returned.
3. Precondition filtering → focused unit test that calls `select_method` with a belief view that satisfies preconditions for method A but not method B, and asserts A is returned (or `None` if no method qualifies).
4. Ranking formula correctness → focused unit test that constructs two methods with different `motive_bias` weights and asserts the higher-scoring method is returned given matching motives.
5. Tie-break by `MethodSchemaId` → focused unit test with two methods of equal score; lower ID wins.
6. Single-layer ticket — runtime planner integration verified by ticket 008.

## Landed Changes

### 1. Defined `select_method()` in `htn/selector.rs`

`crates/worldwake-ai/src/htn/selector.rs` now exports a pure selector with the live signature `select_method(actor, goal, registry, profile, belief_view, motives)`. It filters by goal kind, `AgentSchemaContextProfile.disabled_methods`, actor-relative belief preconditions, and motive-source kind bias, then tie-breaks equal scores by lower `MethodSchemaId`.

The landed selector resolves the already-authored `EntityTemplate`, `CommodityTemplate`, and `RecipeTemplate` values against `GoalOffer`, the actor, and `RuntimeBeliefView`. Unresolved templates conservatively filter the method out.

### 2. Updated `htn/mod.rs` to re-export

`htn/mod.rs` now declares `pub mod selector;` and re-exports `select_method`.

### 3. Added focused unit tests

Inline tests in `htn/selector.rs`:
- `select_method_returns_top_ranked_method_by_motive_score`
- `select_method_honors_disabled_methods_denylist`
- `select_method_skips_methods_with_failed_preconditions`
- `select_method_tie_breaks_by_lower_method_schema_id`
- `select_method_returns_none_when_no_method_matches_goal_kind`
- `select_method_is_deterministic_across_repeated_calls`

## Landed Files

- `crates/worldwake-ai/src/htn/selector.rs` (new)
- `crates/worldwake-ai/src/htn/mod.rs` (modified)
- `specs/S147-htn-method-decomposition.md` (truth-synced selector signature and ranking formula)
- `archive/tickets/S147HTNMETDEC-007.md` (truth-synced, closed out, and archived)

## Out of Scope

- Wiring the selector into `build_stages` (ticket 008).
- `template_to_stages` helper that expands `SubgoalTemplate` into `StrategicStage` values (ticket 008 — it's part of the planner integration).
- Full `BeliefPredicate` evaluation for variants beyond first-ship scope — those return `false` (conservative filter-out) and are filled in as future methods need them.

## Acceptance Result

### Tests Passed

1. All 6 inline focused tests in `selector.rs` passed.
2. Existing suite `cargo test -p worldwake-ai` passed.
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passed after lint-only selector cleanup.

### Invariants

1. `select_method()` is a pure function — no mutation of any input.
2. Same inputs → same output across repeated calls (deterministic).
3. Ranking arithmetic is integer-only — no floats, no wall-clock time.
4. Disabled methods are never returned, regardless of motive score or precondition satisfaction.
5. Tie-break by lower `MethodSchemaId` is stable and documented (matches `MethodRegistry::methods_for` insertion order from ticket 006).

## Test Plan Result

### Focused Tests

1. `crates/worldwake-ai/src/htn/selector.rs` inline tests — 6 cases covering filtering, ranking, determinism, denylist, and tie-break.

### Commands Run

1. Passed `cargo test -p worldwake-ai --lib htn::selector`
2. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. Passed `cargo test -p worldwake-ai`
4. Waived `./scripts/verify.sh` for this ticket iteration because the harness owns the final pre-push workspace wrapper; this ticket's owned source surface was covered by the focused selector test, full `worldwake-ai` package test, and package all-target clippy.

## Outcome

Completed on 2026-05-17.

- Added `crates/worldwake-ai/src/htn/selector.rs` with deterministic method selection.
- Re-exported `select_method` from `crates/worldwake-ai/src/htn/mod.rs`.
- Corrected S147 and this ticket to the live actor-relative selector boundary and live `MotiveSourceRef` shape.

## Deviations

- The drafted `MotiveSourceRef.weight` formula was not live. The selector uses present motive-source kind plus per-method `MotiveBias.weight`; upstream goal ranking remains the owner of source magnitude.
- The selector signature includes `actor: EntityId` because `RuntimeBeliefView` reads are agent-relative under FND-14.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib htn::selector`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `cargo test -p worldwake-ai`
