# S147HTNMETDEC-007: MethodSelector with deterministic ranking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — adds the `MethodSelector` and `select_method()` function. Not yet wired into the planner (ticket 008).
**Deps**: 001 (MotiveSourceDiscriminant), 004 (MethodSchema + supporting types), 006 (MethodRegistry)

## Problem

S147 D3 defines the deterministic method-selection algorithm: filter methods by goal kind and per-agent denylist, filter by precondition satisfaction against the belief view, rank by motive-source bias score, tie-break by `MethodSchemaId`. Without this selector, the registry's content is unreachable from the planner. The selector must be pure (no side effects), deterministic (same inputs → same output), and integer-arithmetic-only (no floats per CLAUDE.md determinism invariant).

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `MethodRegistry` and `methods_for(goal_kind)` exist after ticket 006 lands at `crates/worldwake-ai/src/htn/registry.rs`. `MotiveSourceDiscriminant` and `From<&MotiveSource>` exist after ticket 001 lands at `crates/worldwake-core/src/motive_source.rs`. `MotiveSourceRef` at `crates/worldwake-core/src/motive_source.rs:25` carries `source: MotiveSource` (line 26) and `weight: Permille` (referenced from line 30) — both consumed by the ranking formula.
2. `RuntimeBeliefView` trait exists at `crates/worldwake-sim/src/belief_view.rs:1588` (verified during S147 reassessment). The selector reads via `&dyn RuntimeBeliefView` — no new trait accessor is required because `BeliefPredicate` variants (ticket 004) compose existing reads.
3. `AgentSchemaContextProfile.disabled_methods` exists after ticket 003 lands at `crates/worldwake-core/src/agent_schema_context_profile.rs:54`.
4. Shared boundary: `select_method()` is the only function the planner integration (ticket 008) calls into the htn module. The function signature is the contract; the planner does not reach into the registry or supporting types directly.
5. Ranking formula (per spec D3 step 3): integer-only, `Permille × Permille / 1000`, range `0..=1_000_000`, fits `u32`. No float arithmetic — satisfies CLAUDE.md determinism invariant. Tie-break by `MethodSchemaId` ordinal (deterministic via `BTreeMap` iteration order).

## Architecture Check

1. The selector is a pure function — no mutation of the registry, profile, belief view, or motives. This keeps method selection deterministic and side-effect-free, which is required for replay equivalence.
2. The function returns `Option<&'r MethodSchema>` rather than `Result` because "no method applicable" is a normal outcome (falls back to flat GOAP per spec D4). No-method is not an error.
3. Integer-only ranking (`Permille × Permille / 1000`) preserves the no-floats invariant. Each contribution fits `u32` (max 1_000_000); sum of N contributions fits `u64` for safety (N method-biases × motives is bounded by registry size × motive count, both small).
4. No backwards-compatibility shims. The selector is net-new.

## Verification Layers

1. Deterministic selection (same inputs → same output across runs) → focused unit test that calls `select_method` twice with identical inputs and asserts pointer equality of the returned `&MethodSchema`.
2. Denylist honored → focused unit test that calls `select_method` with `profile.disabled_methods` containing the otherwise-top-ranked method ID and asserts the next-ranked method is returned.
3. Precondition filtering → focused unit test that calls `select_method` with a belief view that satisfies preconditions for method A but not method B, and asserts A is returned (or `None` if no method qualifies).
4. Ranking formula correctness → focused unit test that constructs two methods with different `motive_bias` weights and asserts the higher-scoring method is returned given matching motives.
5. Tie-break by `MethodSchemaId` → focused unit test with two methods of equal score; lower ID wins.
6. Single-layer ticket — runtime planner integration verified by ticket 008.

## What to Change

### 1. Define `select_method()` in `htn/selector.rs`

New file `crates/worldwake-ai/src/htn/selector.rs`:

```rust
use worldwake_core::{AgentSchemaContextProfile, MotiveSourceRef, MotiveSourceDiscriminant};
use worldwake_sim::belief_view::RuntimeBeliefView;
use crate::goal_model::GoalOffer;
use crate::htn::{MethodRegistry, MethodSchema, MethodPrecondition, BeliefPredicate};

pub fn select_method<'r>(
    goal: &GoalOffer,
    registry: &'r MethodRegistry,
    profile: &AgentSchemaContextProfile,
    belief_view: &dyn RuntimeBeliefView,
    motives: &[MotiveSourceRef],
) -> Option<&'r MethodSchema> {
    let goal_kind = GoalKindDiscriminant::from(&goal.key.goal_kind);
    let candidates = registry.methods_for(goal_kind);

    candidates
        .iter()
        .filter_map(|id| registry.get(*id))
        .filter(|m| !profile.disabled_methods.contains(&m.id))
        .filter(|m| preconditions_satisfied(m, belief_view))
        .map(|m| (m, motive_score(m, motives)))
        .max_by(|(a, score_a), (b, score_b)| {
            score_a.cmp(score_b)
                .then_with(|| b.id.cmp(&a.id))   // higher score wins; tie-break: lower id wins
        })
        .map(|(m, _)| m)
}

fn preconditions_satisfied(method: &MethodSchema, belief_view: &dyn RuntimeBeliefView) -> bool {
    method.preconditions.iter().all(|p| evaluate_precondition(p, belief_view))
}

fn evaluate_precondition(p: &MethodPrecondition, belief_view: &dyn RuntimeBeliefView) -> bool {
    match p {
        MethodPrecondition::BeliefHolds(pred) => evaluate_belief_predicate(pred, belief_view),
        MethodPrecondition::MotiveSourcePresent(_)   => true,  // checked separately via motives
        MethodPrecondition::AgentRole(_)             => true,  // future: role lookup
        MethodPrecondition::LocationKnown(_)         => true,  // future: location lookup
    }
}

fn evaluate_belief_predicate(pred: &BeliefPredicate, belief_view: &dyn RuntimeBeliefView) -> bool {
    // Each BeliefPredicate variant routes to the appropriate RuntimeBeliefView accessor.
    // First-ship: only the variants used by ticket 006's methods need full implementations;
    // others may return false (conservative — method is filtered out).
    match pred {
        BeliefPredicate::BountyRecordExists { bounty }      => /* belief_view lookup */ todo!(),
        BeliefPredicate::TargetLastSeenKnown { target }     => /* belief_view lookup */ todo!(),
        // ... etc per spec D1
        _ => false,
    }
}

fn motive_score(method: &MethodSchema, motives: &[MotiveSourceRef]) -> u32 {
    let mut total: u64 = 0;
    for motive in motives {
        let motive_disc = MotiveSourceDiscriminant::from(&motive.source);
        for bias in &method.motive_bias {
            if bias.motive_variant == motive_disc {
                total += (bias.weight.value() as u64) * (motive.weight.value() as u64);
            }
        }
    }
    (total / 1000) as u32   // Permille × Permille / 1000 → u32-safe
}
```

### 2. Update `htn/mod.rs` to re-export

```rust
pub mod selector;
pub use selector::select_method;
```

### 3. Focused unit tests

Inline tests in `htn/selector.rs`:
- `select_method_returns_top_ranked_method_by_motive_score`
- `select_method_honors_disabled_methods_denylist`
- `select_method_skips_methods_with_failed_preconditions`
- `select_method_tie_breaks_by_lower_method_schema_id`
- `select_method_returns_none_when_no_method_matches_goal_kind`
- `select_method_is_deterministic_across_repeated_calls`

## Files to Touch

- `crates/worldwake-ai/src/htn/selector.rs` (new)
- `crates/worldwake-ai/src/htn/mod.rs` (modify — add `pub mod selector; pub use selector::select_method;`)

## Out of Scope

- Wiring the selector into `build_stages` (ticket 008).
- `template_to_stages` helper that expands `SubgoalTemplate` into `StrategicStage` values (ticket 008 — it's part of the planner integration).
- Full `BeliefPredicate` evaluation for variants beyond first-ship scope — those return `false` (conservative filter-out) and are filled in as future methods need them.

## Acceptance Criteria

### Tests That Must Pass

1. All 6 inline focused tests in `selector.rs` pass.
2. Existing suite: `cargo test -p worldwake-ai` passes.
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` clean.

### Invariants

1. `select_method()` is a pure function — no mutation of any input.
2. Same inputs → same output across repeated calls (deterministic).
3. Ranking arithmetic is integer-only — no floats, no wall-clock time.
4. Disabled methods are never returned, regardless of motive score or precondition satisfaction.
5. Tie-break by lower `MethodSchemaId` is stable and documented (matches `MethodRegistry::methods_for` insertion order from ticket 006).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/htn/selector.rs` inline tests — 6 cases covering filtering, ranking, determinism, denylist, and tie-break.

### Commands

1. `cargo test -p worldwake-ai --lib htn::selector`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`
