# S90MANTACSCO-003: Candidate count safety valve

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `CognitiveProfile` field addition (`worldwake-core`), planner search internals (`worldwake-ai`)
**Deps**: S90 spec (completed reassessment)

## Problem

Even with tactical scoping working correctly, a structural safety valve is needed to prevent any search from running with an explosive candidate set. If a future code path bypasses tactical scoping, the safety valve catches it. This is defense-in-depth.

## Assumption Reassessment (2026-04-11)

1. `CognitiveProfile` confirmed at `crates/worldwake-core/src/cognitive_profile.rs:5-26`. Has `Default` impl at lines 28-47. No `max_candidates_per_expansion` field exists yet. Existing similar field: `max_candidates_to_plan` (different purpose — total candidates across entire plan search, not per expansion).
2. `CognitiveProfile` in `AgentDef` confirmed at `crates/worldwake-cli/src/scenario/types.rs:86` as `pub cognitive_profile: Option<CognitiveProfile>`. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:368` uses `unwrap_or_default()`.
3. Shared boundary: `CognitiveProfile` struct in `worldwake-core`, read by `search_plan` in `worldwake-ai`. The new field follows the same pattern as `max_node_expansions`.
4. `candidates_generated` computed at `mod.rs:386` as `candidates.len() as u16`, right after `apply_tactical_candidate_filter`. The existing budget-exhaustion check with `best_barrier` pattern is at lines 306-312.
5. Reassessment correction: many live Rust call sites still use explicit `CognitiveProfile { .. }` literals in tests and support code (`crates/worldwake-ai/src/search/tests.rs`, `crates/worldwake-ai/tests/conformance_execution_budget.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, others). The new field will create compile fallout even though some runtime paths still use `Default` or `unwrap_or_default()`.
6. Scenario boundary correction: `AgentDef` deserializes `Option<CognitiveProfile>` directly at `crates/worldwake-cli/src/scenario/types.rs:86`, and `scenarios/cli-evaluation.ron` already contains an explicit `cognitive_profile` block. Extending `Default` alone is not enough; the new field must also preserve explicit scenario deserialization, either by defaulting the new field at serde time or by updating every active scenario profile block.

## Architecture Check

1. Per-agent `max_candidates_per_expansion` on `CognitiveProfile` follows the existing pattern of `max_node_expansions` — same struct, same access pattern, same profile-driven parameter philosophy (FND-22 agent diversity). Cleaner than a global constant.
2. No backwards-compatibility shims. The `Default` impl provides the default value (200) for all existing code paths.

## Verification Layers

1. Safety valve triggers at threshold → focused unit test (in 004)
2. Returns best barrier plan if available before aborting → code follows same pattern as existing budget-exhaustion check at lines 306-312
3. Existing explicit scenario `cognitive_profile` blocks still deserialize after the field addition → focused `worldwake-cli` scenario/types coverage
4. Manual `CognitiveProfile` literals across crates compile cleanly after the field addition → `cargo build --workspace` succeeds
5. Single-layer production behavior: safety valve is planner-internal, reads authoritative `CognitiveProfile` from ECS

## What to Change

### 1. Add `max_candidates_per_expansion` field to `CognitiveProfile`

**File**: `crates/worldwake-core/src/cognitive_profile.rs`

Add field to struct:

```rust
/// Maximum candidates per expansion before the search aborts.
/// Prevents degenerate unscoped searches from burning expansion budget
/// on explosive candidate sets that will never produce a viable plan.
/// Note: `max_candidates_to_plan` limits total candidates across an entire
/// plan search; `max_candidates_per_expansion` limits candidates at a single
/// expansion step.
pub max_candidates_per_expansion: u16,
```

Add to `Default` impl with value `200`.

Also default the new field at serde time so existing explicit scenario `cognitive_profile` blocks remain valid without forcing same-ticket scenario RON churn.

### 2. Add safety valve check in search loop

**File**: `crates/worldwake-ai/src/search/mod.rs`

After `apply_tactical_candidate_filter()` and `candidates_generated` computation (line 386), add:

```rust
if candidates_generated > cognitive.max_candidates_per_expansion {
    if let Some(barrier_plan) = best_barrier {
        return PlanSearchResult::Found(Box::new(barrier_plan));
    }
    return PlanSearchResult::BudgetExhausted {
        expansions_used: expansions,
    };
}
```

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Scenario RON changes to set custom `max_candidates_per_expansion` per agent (can be done later)
- Adding a new `PlanSearchResult` variant for candidate explosion (uses existing `BudgetExhausted`)
- Changing `max_candidates_to_plan` semantics

## Acceptance Criteria

### Tests That Must Pass

1. `test_scenario_def_cognitive_profile_missing_new_field_uses_default`
2. Existing suite: `cargo test -p worldwake-ai`
3. `cargo build --workspace` (confirms manual literals and shared callers compile after the field addition)
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Any expansion with `candidates.len() > cognitive.max_candidates_per_expansion` aborts immediately with `BudgetExhausted` (or returns best barrier plan if one exists)
2. Default value of 200 is well above normal tactical scoping (~20-50) and well below bypass explosion (2000+)
3. `CognitiveProfile` remains scenario-definable via `AgentDef` with `unwrap_or_default()`
4. Existing explicit scenario `cognitive_profile` blocks remain deserializable when they omit the new field

## Outcome

Completion date: 2026-04-11

Implemented the D3 production substrate by adding `CognitiveProfile::max_candidates_per_expansion` with a default value of `200`, wiring the candidate-count safety valve into `search_plan`, and defaulting the new field at serde time so existing explicit scenario `cognitive_profile` blocks continue to deserialize when they omit it.

What actually changed:
1. `crates/worldwake-core/src/cognitive_profile.rs` now carries `max_candidates_per_expansion`, defaulted both in `Default` and for omitted-field serde inputs
2. `crates/worldwake-ai/src/search/mod.rs` now aborts an expansion with `BudgetExhausted` when post-filter candidate count exceeds `cognitive.max_candidates_per_expansion`, still preferring an already-found barrier plan when present
3. `crates/worldwake-cli/src/scenario/types.rs` now has focused proof that explicit scenario `cognitive_profile` blocks remain valid when the new field is omitted
4. Compile-fallout constructors and fixtures in `worldwake-ai` support modules/tests plus `crates/worldwake-core/src/delta.rs` were updated so the shared type addition compiles cleanly across all targets
5. `specs/S90-mandatory-tactical-scoping.md` was corrected so the active D3 spec matches the real scenario-deserialization contract for the new field

Deviation from the original ticket draft:
1. Reassessment showed that extending `Default` alone was insufficient because explicit scenario RON already deserializes full `CognitiveProfile` values; the ticket therefore also owned the narrow serde-default fix for the new field
2. The focused threshold assertion remains intentionally owned by `S90MANTACSCO-004`; this ticket landed the substrate and scenario/shared-type compatibility work

Verification completed:
1. `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_cognitive_profile_missing_new_field_uses_default`
2. `cargo build --workspace`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs::test_scenario_def_cognitive_profile_missing_new_field_uses_default` — proves explicit scenario profiles still deserialize with the new field omitted
2. Existing `crates/worldwake-core/src/cognitive_profile.rs` default/roundtrip tests updated for the new field value

### Commands

1. `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_cognitive_profile_missing_new_field_uses_default`
2. `cargo build --workspace`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
