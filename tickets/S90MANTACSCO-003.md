# S90MANTACSCO-003: Candidate count safety valve

**Status**: PENDING
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
5. Construction sites: 21 files match `CognitiveProfile {`. Non-archive production files use `Default` impl or `unwrap_or_default()`. The new field is covered by extending the `Default` impl — no construction sites need manual updates.

## Architecture Check

1. Per-agent `max_candidates_per_expansion` on `CognitiveProfile` follows the existing pattern of `max_node_expansions` — same struct, same access pattern, same profile-driven parameter philosophy (FND-22 agent diversity). Cleaner than a global constant.
2. No backwards-compatibility shims. The `Default` impl provides the default value (200) for all existing code paths.

## Verification Layers

1. Safety valve triggers at threshold → focused unit test (in 004)
2. Returns best barrier plan if available before aborting → code follows same pattern as existing budget-exhaustion check at lines 306-312
3. `CognitiveProfile::Default` covers all existing construction sites → `cargo build --workspace` succeeds
4. Single-layer ticket: safety valve is planner-internal, reads authoritative `CognitiveProfile` from ECS

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
- `crates/worldwake-ai/src/search/mod.rs` (modify)

## Out of Scope

- Scenario RON changes to set custom `max_candidates_per_expansion` per agent (can be done later)
- Adding a new `PlanSearchResult` variant for candidate explosion (uses existing `BudgetExhausted`)
- Changing `max_candidates_to_plan` semantics

## Acceptance Criteria

### Tests That Must Pass

1. `search_candidate_safety_valve_triggers_at_threshold` (new, in 004)
2. Existing suite: `cargo test -p worldwake-ai`
3. `cargo build --workspace` (confirms `Default` impl covers all construction sites)

### Invariants

1. Any expansion with `candidates.len() > cognitive.max_candidates_per_expansion` aborts immediately with `BudgetExhausted` (or returns best barrier plan if one exists)
2. Default value of 200 is well above normal tactical scoping (~20-50) and well below bypass explosion (2000+)
3. `CognitiveProfile` remains scenario-definable via `AgentDef` with `unwrap_or_default()`

## Test Plan

### New/Modified Tests

1. None in this ticket — tests are in S90MANTACSCO-004

### Commands

1. `cargo build --workspace`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
