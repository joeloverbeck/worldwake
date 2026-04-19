# S108PERACTBIN-003: Strictness gate on plan-revalidation best-effort fallbacks

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — reassessment showed `revalidate_best_effort_payload_override_step` and `revalidate_exact_target_step` preserve the originally planned concrete targets rather than substituting different ones, so no production gate change was required.
**Deps**: archive/tickets/S108PERACTBIN-001.md (needs `BindingStrictness`, `check_binding_strictness`).

## Problem

`crates/worldwake-ai/src/plan_revalidation.rs::revalidate_next_step` (lines 14–49) has three revalidation paths:

1. Primary: enumerate affordances via `get_affordances_for_defs`, check each with `requested_affordance_matches`.
2. Fallback 1 (`revalidate_best_effort_payload_override_step`, lines 51–82): accept a planner-synthesized payload if `actor_constraints`, `preconditions`, and `payload_override_is_valid` all pass for the raw requested targets.
3. Fallback 2 (`revalidate_exact_target_step`, lines 84–118): if the `ActionDef.targets` are ALL `TargetSpec::SpecificEntity(_)`, synthesize an `Affordance` from the step's targets and rerun `requested_affordance_matches`.

The drafted concern was that both fallbacks looked like substitution paths because they bypass primary affordance enumeration. Live code reassessment showed the opposite: both fallbacks preserve the originally planned concrete targets. `revalidate_best_effort_payload_override_step` only succeeds when the requested `targets` still satisfy the action's preconditions and the payload override is valid for those same targets. `revalidate_exact_target_step` synthesizes an affordance from the step's own `targets` and re-runs `requested_affordance_matches` against those same bound targets.

That means the blanket gate drafted here would have violated the live binding contract for lawful same-target exact-identity steps such as `post_notice`, whose payload-only revalidation path is still tied to the same posting place. Under FND-4, FND-14, and FND-21, the real architectural invariant is "no silent retargeting," not "no fallback revalidation when the same exact target is still the target."

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `revalidate_next_step` is at `crates/worldwake-ai/src/plan_revalidation.rs:14`; `revalidate_best_effort_payload_override_step` at line 51; `revalidate_exact_target_step` at line 84. All three return `bool`. Existing focused tests in the same file's `#[cfg(test)]` block include `build_transport_registry` (line 707), `build_specific_entity_payload_registry` (line 758), `build_explicit_payload_registry` (line 813), and their exercising tests — these will be re-verified to confirm the strictness gate preserves the intended existing paths (FungibleEquivalentCommodity, non-ExactIdentity flows).
2. The active spec originally described D5 as a blanket gate on both fallbacks, but live branch verification shows that wording is too broad. The spec is corrected in-scope by this ticket so D5 now states that the fallback paths are same-target revalidation surfaces rather than lawful retargeting surfaces.
3. Shared abstraction boundary: `plan_revalidation.rs`'s three-path revalidation surface. This ticket reassesses whether Fallbacks 1 and 2 are true substitution paths under the authoritative `BindingStrictness` classifier from T-001.
4. Not applicable — no failing golden motivates this ticket.
5. Not applicable — not a planner- or golden-driven ticket. Revalidation is belief-first by construction (FND-14); it reads the agent's belief view.
6. AI regression intended layer: runtime `agent_tick` revalidation boundary; local unit tests against `revalidate_next_step` are sufficient here. Goldens live in T-005.
7. Not applicable — no ordering claim.
8. Not applicable — no heuristic removal.
9. First failure boundary for stale fully bound exact-identity steps remains plan revalidation when primary affordance enumeration no longer finds a match. The two fallback helpers do not introduce alternate-target substitution because they operate on the already planned `targets`.
10. Not applicable.
11. Not applicable.
12. Not applicable.
13. Adjacent observation: `revalidate_exact_target_step` is not itself a retargeting path; it replays `requested_affordance_matches` against the step's own planned targets. Likewise, `revalidate_best_effort_payload_override_step` validates the payload against the same planned targets.
14. Mismatch discovered: the drafted ticket/spec premise that both fallbacks are substitution-style and must be gated by `check_binding_strictness` was false on the live branch. The ticket is narrowed to factual reassessment plus spec/ticket alignment; no production change is required.
15. Not applicable.

## Architecture Check

1. Cleaner architecture here is not to force the shared classifier onto a surface it does not actually govern. `binding_strictness` answers whether the system may substitute a different target after binding; these fallback helpers do not substitute a different target at all. Preserving that distinction keeps FND-4/FND-21 honest and avoids turning one metadata field into an over-broad policy switch.
2. No backward-compatibility shim (FND-28). The live helpers remain unchanged; the correction is in the ticket/spec contract, not in a compatibility alias.

## Verification Layers

1. Exact-identity same-target payload revalidation remains lawful -> focused unit test in `plan_revalidation.rs`.
2. Exact-identity same-target specific-entity revalidation remains lawful -> focused unit test in `plan_revalidation.rs`.
3. The social-obligation loop depending on repeated `post_notice` commits remains lawful -> exact golden in `golden_planner_pathology.rs`.
4. Single-layer ticket after narrowing: no production mutation, action-trace, or event-log contract change.

## What to Change

### 1. Correct the ticket/spec contract

Update this ticket and the active S108 spec to state the live narrower contract:

- `revalidate_best_effort_payload_override_step` is a same-target payload validation path, not a substitution path.
- `revalidate_exact_target_step` is a same-target identity replay path, not a substitution path.
- `ExactIdentity` still forbids silent retargeting, but these helpers are not the place where alternate-target substitution occurs.

### 2. Keep `plan_revalidation.rs` unchanged

No production change is required in `crates/worldwake-ai/src/plan_revalidation.rs`. The live behavior is already the FOUNDATIONS-aligned one for this surface.

### 3. Re-verify the existing proof surfaces

Use the existing focused tests that already prove same-target exact-identity revalidation remains lawful:
- `specific_entity_payload_override_revalidates_with_concrete_step_target`
- `explicit_payload_variant_steps_revalidate_via_best_effort_fallback`
- `obligation_satiation_allows_survival_needs_to_override_posting`

## Files to Touch

- `tickets/S108PERACTBIN-003.md` (modify — reassessment correction, closeout)
- `specs/S108-per-action-binding-strictness.md` (modify — factual D5 correction)

## Out of Scope

- Removing `revalidate_exact_target_step`.
- Sim-side dispatch gate — T-002.
- Decision trace field — T-004.
- New planner production behavior; this ticket is now a reassessment/alignment closeout.

## Acceptance Criteria

### Tests That Must Pass

1. Existing `plan_revalidation` focused tests proving same-target exact-identity fallback legality continue to pass.
2. Exact golden `obligation_satiation_allows_survival_needs_to_override_posting` continues to pass, proving the reassessment did not break the lawful social loop.
3. Ticket and active spec text no longer claim that these helpers are substitution gates.

### Invariants

1. `revalidate_best_effort_payload_override_step` and `revalidate_exact_target_step` do not substitute different concrete targets; they validate the step's already planned targets.
2. `ExactIdentity` continues to forbid silent retargeting, but this ticket does not introduce any new planner-side gate because these helpers are not retargeting surfaces.
3. T-002 remains the live request-resolution/start-time validation correction for malformed or stale dispatch shapes; this ticket only corrects the planner-side narrative around same-target revalidation.

## Test Plan

### New/Modified Tests

1. No new tests required. Existing focused and golden proofs are the honest verification surface after reassessment narrowing.

### Commands

1. `cargo test -p worldwake-ai plan_revalidation::tests::specific_entity_payload_override_revalidates_with_concrete_step_target -- --exact`
2. `cargo test -p worldwake-ai plan_revalidation::tests::explicit_payload_variant_steps_revalidate_via_best_effort_fallback -- --exact`
3. `cargo test -p worldwake-ai --test golden_planner_pathology obligation_satiation_allows_survival_needs_to_override_posting -- --exact`

## Outcome

Completed as a reassessment/alignment ticket rather than a production-code change. Live branch verification showed that `revalidate_best_effort_payload_override_step` and `revalidate_exact_target_step` preserve the step's already planned concrete targets and therefore do not create the silent retargeting risk the draft described. No `plan_revalidation.rs` code change was required.

The active S108 spec was updated factually so D5 no longer describes these helpers as blanket `ExactIdentity` substitution gates.

## Deviations

1. The drafted ticket and spec treated both revalidation fallbacks as substitution-style paths and required a blanket `check_binding_strictness` gate.
2. Focused and golden verification proved that premise false on the live branch: exact-identity `post_notice` depends on lawful same-target payload revalidation, and adding the drafted gate removed repeated `post_notice` commits entirely.
3. The landed result is therefore narrower: no production patch, only ticket/spec correction and factual closeout.

## Verification Result

1. `cargo test -p worldwake-ai plan_revalidation::tests::specific_entity_payload_override_revalidates_with_concrete_step_target -- --exact`
2. `cargo test -p worldwake-ai plan_revalidation::tests::explicit_payload_variant_steps_revalidate_via_best_effort_fallback -- --exact`
3. `cargo test -p worldwake-ai --test golden_planner_pathology obligation_satiation_allows_survival_needs_to_override_posting -- --exact`
