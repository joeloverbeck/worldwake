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
16. When adding a new shared enum variant ahead of integration tickets, sweep dependent exhaustive tables. Prefer bounded compile-safe inert branches over reusing older variant behavior. Also check bounded non-owner exhaustive consumers: ranking/policy code, failure handling, observation/runtime helpers, relay-selection or ordering helpers, renderers, detail-formatting surfaces, crate-root re-exports, inventory arrays, count assertions, and representative membership tests. For planner-op additions specifically, also sweep helpers keyed indirectly by shared transition semantics (for example synthetic candidate builders or search helpers that key off `PlannerTransitionKind` rather than the op enum directly). Absorb compile-safe inert handling rather than treating it as separate architecture change.
17. When widening a shared payload or belief/claim enum by value, check the enclosing enum for CI-shaped size lints such as `large_enum_variant` before broad verification. Decide whether the truthful contract should box the variant, restructure the carrier, or keep the persisted inline shape with a narrow documented lint allowance.

## Planner and goal family wiring

18. When making a new planner-visible operator lawful, sweep the full planner contract: goal dispatch, relevant-op declarations, progress barriers, goal-model expectations, heuristic/guidance surfaces (`goal_relevant_places`, evidence-place fallback, travel-pruning inputs when relevant), search tests. Verify the `may_appear_mid_plan` / `is_progress_barrier` combination: with `may_appear_mid_plan=false`, the operator can ONLY appear as a terminal step. With `may_appear_mid_plan=true`, it can appear anywhere.
19. When a planner goal must synthesize a runtime payload, verify the activation chain end to end: the goal carries enough identity, root/current-place guidance makes the operator reachable, and terminal-step semantics treat the action as goal-satisfying.
20. When the first planner fix only makes an operator partially live, immediately re-check the rest of the same operator chain: candidate shape, root synthesis, payload construction, terminal semantics, and the focused planner proof.
21. When one goal family spans multiple target subtypes, verify operator availability per subtype. Check for stale operators leaking across subtypes.
22. When a goal family ends in a place-sensitive terminal action, add focused coverage for both target satisfaction and return-to-terminal-place legality.
23. When a colocated leaf action becomes live, verify the colocated terminal case separately from travel-plus-leaf planning.
24. When adding a new candidate emitter for a domain that already has active goal families, verify the new goal does not cause goal-switching collisions with existing goals for the same target entity. Run existing golden suites for that domain first.
25. When a goal generates as a candidate with nonzero motive but is never selected, diagnose in order: (a) `compute_motive` returns > 0, (b) `synthesized_root_candidate_targets` provides a root candidate, (c) `is_progress_barrier` identifies the terminal op, (d) `build_payload_override` succeeds, (e) `estimate_duration` returns `Some`.
26. When adding a new `GoalKind` variant, use the compiler to surface exhaustive-match sites, but also sweep runtime-reachable surfaces: `GoalDispatchKey` enum + `ALL` array + `from_goal_kind`, `goal_kind_discriminant` in ranking.rs, `feasibility.rs` strategy-goal match, `format_goal_kind` in display.rs, and shared signal/motive helpers. Consider `cargo build --workspace` first for compiler errors, then grep for the closest existing sibling to find runtime-only sites.

## Helper extraction and shared sync helpers

27. When the clean fix requires extracting a helper out of an existing module into a neutral shared location, explicitly sweep sibling and transitive import sites for the old module path before relying on compile fallout alone. Shared-helper extraction often leaves behind stale `use crate::old_module::helper` assumptions even when the owned behavioral change is otherwise correct.
28. When a new component or metadata field is derivable from authoritative post-state plus the current tick, prefer a single sync helper over bespoke set/clear branches at each call site. Reuse that helper across every mutation path that can enter or leave the invariant so later lifecycle fallout changes the predicate in one place instead of many.
29. When a ticket needs extra per-call state but the existing seam is a stable public or test-facing helper that is still locally owned, prefer preserving that seam and introducing an internal wrapper/helper variant first. Only widen the established signature when the ticket truly owns that contract change or the extra state must become part of the caller-facing API.
30. Preserving an existing wrapper is not a backward-compatibility shim when it is a thin delegation to the new canonical implementation with a semantically neutral value, introduces no alternate behavior, and keeps the production caller on the canonical path. Record that wrapper/default shape in closeout when the ticket or spec says new parameters must be required or no compatibility path should remain.
31. When a staged migration introduces temporary dual-surface helpers or adapter returns (for example blocker+discrepancy carriers, traced/untraced variants, or compatibility wrappers), design the first landed helper shape with CI-matching clippy in mind. Prefer a small custom enum over `Option<Option<T>>`, and prefer explicit narrow `#[allow(clippy::too_many_arguments)]` only when the helper is truly an owned boundary rather than accidental argument sprawl.
32. For repo-wide struct-literal or constructor fallout caused by a shared field addition, target exact token-boundary constructor matches first (for example a parser-resolved `EventPayload` literal or a boundary-safe pattern that cannot also match `DecisionEventPayload`/`ContentionEventPayload`), then immediately re-scan touched files for accidental edits in same-shaped blocks before moving on to verification. Treat this cleanup pass as part of the implementation step, not optional polish after tests fail.
33. For function signature removals or parameter-list migrations, target the exact callee and call arity instead of broad trailing-argument patterns. After any mechanical callsite rewrite, inspect every touched hunk for same-shaped neighboring API calls before formatting or testing; do not rely on compile fallout to catch accidentally edited calls that still type-check.
34. When extending an existing helper with several tightly related new inputs, check the repo's CI-shaped clippy surface before letting the signature sprawl. Prefer a small context struct or similarly local bundling at the call boundary over growing the parameter list until `too_many_arguments` forces a late cleanup or lint allowance. If the same bounded payload context, such as `slice + cap`, `source + cap`, or `evidence set + cap`, is being threaded through two or more emitters, persistence helpers, or finalize paths, introduce the local context struct before broad verification.
35. When a ticket needs extra local runtime or call-site context inside one subsystem, do not widen an established public helper or crate-visible API by reflex. If the broader surface is not part of the owned contract, prefer a narrower internal helper or local wrapper that threads the added context only through the truthful live seam, and record that narrowed shape in ticket closeout when the draft implied a wider API edit.
36. After adding multiple focused tests with similar setup, inspect the source diff before broad verification for repeated fixture construction. If the setup repeats enough to obscure the asserted behavior, extract a local test helper or fixture struct first, then run the focused proof; avoid discovering obvious DRY cleanup only after broad gates have already passed.
37. Before reusing an existing helper across AI/runtime belief surfaces, verify that the helper already lives on the same trait/view boundary as the owned seam. If the live helper is bound to `RuntimeBeliefView`, another runtime-only trait, or a different trait-object family than the planner-facing code under audit, prefer a small local adapter or a new seam-local helper over widening the runtime helper just to make the types line up.
38. When a new path needs the same classification plus side-effectful aftermath as an existing handler (for example discrepancy recording, blocker/event writes, or memory updates), prefer extracting a shared helper from the live path over copying the write logic or only widening a classifier's visibility. Reuse the authoritative aftermath seam directly so same-domain paths stay behaviorally aligned.
39. When a maintenance pass transfers conserved quantity from one source to multiple possible consumers, audit batch staleness before choosing the implementation shape. If precomputed `(consumer, source, next_state)` updates could overdraw a shared source after an earlier consumer commits, prefer apply-time rereads, an explicit per-source accumulator, or another deterministic source-allocation structure; then record the landed source/sink contract in ticket closeout.

## Passive perception and per-tick occupancy

40. For passive perception/belief tickets where the owned invariant is "the agent is currently at place X this tick" or similar per-tick occupancy state, verify that the sync point is not accidentally gated on non-empty observation batches, co-located subjects, successful perception rolls, or event-derived updates. If the invariant should advance every lawful tick, keep its update path outside those optional batch/result guards.
