# S27INVPROF-001: Require `ViolationDispositionProfile` for investigate behavior

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — shared investigate runtime contract in `worldwake-sim` and `worldwake-systems`
**Deps**: `archive/specs/S27-expectation-violation-goals.md`, `archive/tickets/completed/E17CRITHEJUS-007.md`

## Problem

The live investigate pipeline has a cross-layer contract split:

- AI candidate generation already treats `ViolationDispositionProfile` as required and emits no violation candidates when the actor lacks the profile.
- The authoritative investigate action still allows profile-less execution by falling back to a hardcoded 3-tick duration.
- Investigate commit only resolves `ViolationMemory` when the profile exists, so a profile-less runtime path can execute the action yet skip the intended incident-lifecycle mutation.

That is not a clean architecture. It lets the same action family be lawfully available in one runtime path but not in the actual AI behavior contract, and it leaves action duration/retention partly sourced from a hidden fallback rather than concrete per-agent state.

## Assumption Reassessment (2026-03-26)

1. Exact shared abstraction boundary under audit: the `ViolationDispositionProfile` contract for `GoalKind::InvestigateViolation` currently spans `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-sim/src/action_semantics.rs`, and `crates/worldwake-systems/src/investigate_actions.rs`.
2. In `crates/worldwake-ai/src/candidate_generation.rs`, `emit_expectation_violation_candidates()` returns early when `ctx.view.violation_disposition_profile(ctx.agent)` is absent. The live focused test `violation_no_profile_emits_nothing` already proves that no profile means no investigate behavior on the AI side.
3. In `crates/worldwake-sim/src/action_semantics.rs`, `DurationExpr::ActorInvestigationDisposition` currently falls back to `ActionDuration::new(3)` when the actor lacks `ViolationDispositionProfile`. This is the authoritative shared runtime source of the profile-less investigate path.
4. In `crates/worldwake-systems/src/investigate_actions.rs`, `commit_investigate()` only resolves the selected `ViolationMemory` record inside `if let Some(profile) = txn.get_component_violation_disposition_profile(instance.actor)`. So the same missing profile that still allows the action to start also suppresses the intended resolution/retention mutation at commit time.
5. Existing focused runtime coverage includes `investigate_action_falls_back_to_three_ticks_without_profile` in `crates/worldwake-systems/src/investigate_actions.rs`. That test currently locks in the inconsistent behavior and should be replaced, not preserved.
6. Existing golden coverage for investigation in `crates/worldwake-ai/tests/golden_emergent.rs` seeds `ViolationDispositionProfile` explicitly. Current goldens therefore do not depend on the fallback path.
7. Mismatch with archived S27 details: `archive/specs/S27-expectation-violation-goals.md` describes per-agent `ViolationDispositionProfile` as the source of investigation parameters, but also documents a profile-less duration fallback. The live AI layer already chose the cleaner contract: no profile means no investigation behavior. This ticket should align the runtime/action layers to that contract instead of extending the fallback.
8. FOUNDATIONS alignment:
   - P2: hardcoded fallback duration is an ungrounded parameter path.
   - P8: action preconditions and duration should be explicit and concrete.
   - P17: humans and AI should not have different lawful access to investigate because one path bypasses the profile requirement.
   - P20: investigation variation should come from concrete per-agent profiles, not an invisible default.
9. This is a shared runtime/action ticket, not a planner-ranking ticket. The live `GoalKind` under audit remains `InvestigateViolation`, and the exact operator surface is the `investigate` action with `DurationExpr::ActorInvestigationDisposition`.
10. Adjacent contradiction classification: the issue surfaced during E17 work, but it is a separate shared investigate/runtime bug, not a required consequence of theft-specific logic. It should be tracked independently rather than folded into later accusation or workspace-verification tickets.
11. Information-path note: this ticket does not add or move any social-evidence path. It only closes a runtime contract gap around who can lawfully investigate and how that action resolves.
12. Recommended correction: make `ViolationDispositionProfile` a hard requirement for investigate affordance generation, authoritative start, and duration resolution. Do not add a default retention fallback and do not keep the 3-tick duration shim.

## Architecture Check

1. Requiring `ViolationDispositionProfile` end-to-end is cleaner than adding more fallback logic. The AI layer already treats the profile as the capability boundary, and the foundations prefer concrete per-agent parameters over hidden defaults.
2. This removes a backwards-compatibility-style alias path where profile-less investigate remains manually lawful even though the simulation’s planning and retention semantics assume the opposite.
3. Rejecting profile-less investigate is better than trying to preserve it with a default retention duration. A default would keep the same hidden parameter problem alive, reopen P2/P20 drift, and continue to make the action partially lawless for agents who lack the component that defines investigation behavior.

## Verification Layers

1. No `ViolationDispositionProfile` means no investigate affordance/runtime start -> focused runtime test in `crates/worldwake-systems/src/investigate_actions.rs`
2. `DurationExpr::ActorInvestigationDisposition` rejects missing profile instead of silently using `3` -> focused unit test in `crates/worldwake-sim/src/action_semantics.rs`
3. AI and authoritative layers agree on the capability boundary -> existing `crates/worldwake-ai/src/candidate_generation.rs` focused test `violation_no_profile_emits_nothing` plus new runtime rejection coverage
4. Profile-backed investigate still starts, commits, and resolves `ViolationMemory` -> existing focused investigate tests in `crates/worldwake-systems/src/investigate_actions.rs`
5. Single shared-contract ticket; additional golden mapping is not the primary proof surface because the contradiction is already provable earlier in focused runtime/unit layers.

## What to Change

### 1. Remove the profile-less investigate duration fallback

- Update `DurationExpr::ActorInvestigationDisposition` in `crates/worldwake-sim/src/action_semantics.rs` to error when `ViolationDispositionProfile` is absent, matching the other profile-driven duration expressions.
- Remove the hardcoded `3`-tick fallback path for investigate.

### 2. Make investigate require the profile authoritatively

- In `crates/worldwake-systems/src/investigate_actions.rs`, reject profile-less investigate at the authoritative boundary.
- The cleanest implementation is to require the profile during action start so the action cannot enter the active-action set without the component that defines its duration and retention behavior.
- Keep commit logic profile-driven, but after this change the missing-profile branch should become unreachable for lawful action execution rather than silently tolerated.

### 3. Replace fallback tests with contract tests

- Remove or rewrite the current fallback test that asserts 3 ticks without a profile.
- Add focused tests proving:
  - no profile -> no lawful investigate start
  - with profile -> existing investigate lifecycle still works unchanged
  - AI candidate-generation no-profile contract remains aligned with the authoritative action gate

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (tests only if needed; existing proof may be sufficient)

## Out of Scope

- Theft-specific `SuspectedTheft` logic
- New AI candidate heuristics or ranking changes
- Broad `ViolationDispositionProfile` rollout to agents that currently do not need investigation behavior
- Golden scenario expansion unless focused proof surfaces prove insufficient

## Acceptance Criteria

### Tests That Must Pass

1. Actor without `ViolationDispositionProfile` cannot lawfully start `investigate`
2. `DurationExpr::ActorInvestigationDisposition` no longer falls back to a hardcoded duration when the profile is absent
3. Existing investigate lifecycle tests with a profile still pass
4. Existing AI focused test `violation_no_profile_emits_nothing` still passes
5. Existing suite: `cargo test -p worldwake-systems`
6. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `ViolationDispositionProfile` is the single concrete authority for investigate duration and retention behavior
2. AI and authoritative runtime layers agree on whether an agent can investigate at all
3. No hidden default duration or retention shim remains for profile-less investigate

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/investigate_actions.rs` — replace the profile-less fallback test with authoritative rejection coverage.
2. `crates/worldwake-sim/src/action_semantics.rs` — add focused duration-resolution coverage proving `ActorInvestigationDisposition` errors without the profile.
3. `crates/worldwake-ai/src/candidate_generation.rs` — keep `violation_no_profile_emits_nothing` as the AI-side alignment proof; add nothing unless reassessment shows the current test is too weak.

### Commands

1. `cargo test -p worldwake-systems investigate_action`
2. `cargo test -p worldwake-sim ActorInvestigationDisposition`
3. `cargo test -p worldwake-ai violation_no_profile_emits_nothing`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-sim`
6. `cargo clippy -p worldwake-systems -- -D warnings`
7. `cargo clippy -p worldwake-sim -- -D warnings`
