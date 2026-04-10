# Implementation Discipline

Worldwake-specific coding rules for Step 5.

## General discipline

1. Keep edits minimal and targeted.
2. Prefer existing abstraction boundaries over duplicating logic.
3. TDD for bug fixes: add test capturing the bug, confirm it fails, fix behavior.
4. Never adapt tests to preserve a bug.
5. No backward-compatibility shims, aliases, or dual paths.
6. Preserve critical invariants from [AGENTS.md](../../../../AGENTS.md): belief-only planning, information locality, append-only event log, determinism, conservation, unique location.
7. When authoritative validation or affordance-surface behavior changes, verify the full AI pipeline per `Authoritative-To-AI Impact Rule` in [AGENTS.md](../../../../AGENTS.md).

## Action lifecycle

8. When widening an action into a new custody or state regime, audit related stored state carriers for stale markers.
9. When turning a single-shot action into a staged lifecycle, prove each phase separately: start admission, intermediate evolution, commit conditions, abort aftermath.
10. When an action uses a profile-driven or expression-driven duration, make test helpers derive or tolerate the real completion window. Do not copy a nearby fixed-duration helper.
11. When splitting uniform behavior into variant-specific rules, rewrite existing compressed tests into per-case proofs.

## Enum variant handling

12. When adding a new enum variant, search for exhaustive matches and state validators in dependent crates. Also search for:
    - Hardcoded array/vec inventories (`const ALL`, test-only `ALL_KEYS`) and count assertions (`assert_eq!(keys.len(), N)`)
    - Runtime-reachable wildcard catch-all matches that panic (e.g., `(strategy, goal_kind) => unreachable!(...)` in `feasibility.rs`)
    - `Display` impl, cross-crate error-mapping functions (e.g., `map_reservation_error` in `start_gate.rs`), variant-inventory tests, and crate-root re-exports
13. When a new variant is not supposed to be live yet, land explicit inert dispatch/policy/ranking branches.
14. When adding or replacing an `EntityKind`, include kind-classification and lifecycle-routing helpers in the sweep.
15. When adding a field to a shared model, search for hand-written constructors and test literals across sibling modules. When a field's value differs per dispatch variant but current code shares a single constant across multiple variants, split into per-variant constants.
16. When adding a new shared enum variant ahead of integration tickets, sweep dependent exhaustive tables. Prefer bounded compile-safe inert branches over reusing older variant behavior. Also check bounded non-owner exhaustive consumers: ranking/policy code, failure handling, observation/runtime helpers, relay-selection or ordering helpers, renderers, and detail-formatting surfaces. For planner-op additions specifically, also sweep helpers keyed indirectly by shared transition semantics (for example synthetic candidate builders or search helpers that key off `PlannerTransitionKind` rather than the op enum directly). Absorb compile-safe inert handling rather than treating it as separate architecture change.

## Planner and goal family wiring

17. When making a new planner-visible operator lawful, sweep the full planner contract: goal dispatch, relevant-op declarations, progress barriers, goal-model expectations, heuristic/guidance surfaces (`goal_relevant_places`, evidence-place fallback, travel-pruning inputs when relevant), search tests. Verify the `may_appear_mid_plan` / `is_progress_barrier` combination: with `may_appear_mid_plan=false`, the operator can ONLY appear as a terminal step. With `may_appear_mid_plan=true`, it can appear anywhere.
18. When a planner goal must synthesize a runtime payload, verify the activation chain end to end: the goal carries enough identity, root/current-place guidance makes the operator reachable, and terminal-step semantics treat the action as goal-satisfying.
19. When the first planner fix only makes an operator partially live, immediately re-check the rest of the same operator chain: candidate shape, root synthesis, payload construction, terminal semantics, and the focused planner proof.
20. When one goal family spans multiple target subtypes, verify operator availability per subtype. Check for stale operators leaking across subtypes.
21. When a goal family ends in a place-sensitive terminal action, add focused coverage for both target satisfaction and return-to-terminal-place legality.
22. When a colocated leaf action becomes live, verify the colocated terminal case separately from travel-plus-leaf planning.
23. When adding a new candidate emitter for a domain that already has active goal families, verify the new goal does not cause goal-switching collisions with existing goals for the same target entity. Run existing golden suites for that domain first.
24. When a goal generates as a candidate with nonzero motive but is never selected, diagnose in order: (a) `compute_motive` returns > 0, (b) `synthesized_root_candidate_targets` provides a root candidate, (c) `is_progress_barrier` identifies the terminal op, (d) `build_payload_override` succeeds, (e) `estimate_duration` returns `Some`.
25. When adding a new `GoalKind` variant, use the compiler to surface exhaustive-match sites, but also sweep runtime-reachable surfaces: `GoalDispatchKey` enum + `ALL` array + `from_goal_kind`, `goal_kind_discriminant` in ranking.rs, `feasibility.rs` strategy-goal match, `format_goal_kind` in display.rs, and shared signal/motive helpers. Consider `cargo build --workspace` first for compiler errors, then grep for the closest existing sibling to find runtime-only sites.
