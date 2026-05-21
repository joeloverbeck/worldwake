# S160HTNAUTHHON-003: Rename fake group-hunt method

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — HTN method registry (`worldwake-ai`)
**Deps**: None

## Problem

Before this ticket, `htn/methods.rs::fulfill_bounty_group_hunt` declared subgoals
`[DeclareSupport, TravelTo(staging), Attack(target)]` and a code comment admitted
*"Existing planner ops have no RecruitAlly leaf. DeclareSupport is the first-ship
social signal for assembling a lawful group hunt."* There was no
recruit/coordination action leaf; the actual confrontation was a solo `Attack`.
The method *name* promised group coordination the world could not enforce
(FND-20: a method must express how this kind of agent pursues this kind of
condition, not a desired story beat). This ticket renamed the method to its honest
behavior — declare support, then pursue directly — preserving the real
`DeclareSupport` social signal while removing the misleading "group hunt" promise.

## Assumption Reassessment (2026-05-21)

1. Before this ticket, `htn/methods.rs::fulfill_bounty_group_hunt` built method id
   3 for `GoalKindDiscriminant::FulfillBounty`, with the misleading comment at
   lines 149–150. Its subgoals were `DeclareSupport` (a real op —
   `planner_ops.rs:44`, `declare_support` action at `planner_ops.rs:131`),
   `TravelTo(staging)`, and a solo `Attack`.
2. Rename blast radius (grepped workspace-wide, all in `worldwake-ai`): the fn name
   (`methods.rs:138`), the registry insert (`registry.rs:56`), and the then-live
   selector test `canonical_group_hunt_selects_from_real_belief_preconditions`
   plus its `.expect(...)` message (`selector.rs:1222–1243`). No other crate
   referenced the old name.
3. Shared boundary under audit: the method registry assembled by
   `build_method_registry()` (`registry.rs:52`). The count test
   `registry_builds_with_11_methods_without_dead_method_ids` (`registry.rs:73`) is
   **unaffected** — a rename keeps 11 methods; only names change. Removal (spec
   option a) is rejected: the `DeclareSupport` stage is a lawful belief-backed step
   worth keeping.
4. Live `GoalKind` under test: `FulfillBounty`. The renamed method's preconditions
   (`TargetBelievedDangerous`, `AllyOrBountyOfficeAvailable`) and behavior are
   unchanged — selection still reads the same belief-backed preconditions; only the
   name and the now-honest comment change.

## Architecture Check

1. Renaming to e.g. `fulfill_bounty_support_declared_direct` makes the method name
   match what the code does (declare support, then pursue directly), removing the
   FND-20 honesty gap without inventing a coordination system the world cannot
   enforce.
2. No backward-compatibility alias: the old name is removed outright and all three
   references updated in this ticket. The misleading comment is rewritten to state
   the honest behavior.

## Verified Layers

1. Registry no longer exposes a method *named* group hunt -> focused unit assertion
   (renamed selector test) that the method selected for the dangerous-target + ally
   beliefs is the support-declared-direct method.
2. Behavior is identical to prior (solo pursuit after support declaration) ->
   existing selector test (renamed) continues to pass with the same belief
   preconditions; existing HTN goldens unchanged.
3. Single-layer ticket (schema/registry rename): no action-trace / event-log mapping
   applies — no runtime behavior changes.

## Landed Changes

### 1. Renamed the method fn and rewrote its comment

Renamed `fulfill_bounty_group_hunt` to
`fulfill_bounty_support_declared_direct`. Rewrote the local comment to state the
honest behavior: `DeclareSupport` is a real social signal, after which the agent
pursues the target directly without enforced group coordination.

### 2. Updated the registry insert

`registry.rs` now inserts
`methods::fulfill_bounty_support_declared_direct()`. Method id stayed 3; count
stayed 11.

### 3. Updated the selector test name and message

`selector.rs` now uses
`support_declared_direct_selects_from_real_belief_preconditions`, with an updated
`.expect(...)` message.

## Landed Files

- `crates/worldwake-ai/src/htn/methods.rs` (modify — fn name + comment)
- `crates/worldwake-ai/src/htn/registry.rs` (modify — insert call)
- `crates/worldwake-ai/src/htn/selector.rs` (modify — test name + message)

## Out of Scope

- Adding any real coordination/recruit artifacts — explicitly a future gameplay
  spec (the spec's stated non-goal).
- The `MethodSubgoalAuthority` labeling —
  `archive/tickets/S160HTNAUTHHON-001.md`.
- Changing the method's preconditions, subgoals, or runtime behavior — rename only.

## Acceptance Result

### Tests That Passed

1. Renamed selector test selected the support-declared-direct method from the
   dangerous-target + ally beliefs.
2. `registry_builds_with_11_methods_without_dead_method_ids` still passed
   unchanged (11 methods).
3. Existing `worldwake-ai` suite passed.

### Invariants

1. No registered method is *named* "group hunt" without a real coordination leaf.
2. The method behavior stayed identical to its prior behavior (solo pursuit after
   support declaration); existing AI/golden coverage passed.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/src/htn/selector.rs` — renamed
   `canonical_group_hunt_selects_from_real_belief_preconditions` into the
   support-declared-direct name; assertion and message updated.

## Outcome

Completed on 2026-05-21.

- Renamed the misleading HTN method symbol to
  `fulfill_bounty_support_declared_direct`.
- Preserved the method id, preconditions, subgoals, motive bias, and planning
  budget so behavior remains a rename-only honesty fix.
- Updated the registry call and selector unit test name/message to the landed
  terminology.
- Truth-synced the active S160 spec to describe this method as renamed while the
  remaining escort sentinel deliverable stays active.

## Deviations

- None. The landed shape matches spec option b: keep `DeclareSupport`, remove the
  misleading group-hunt naming, and do not add coordination/recruit artifacts.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib htn::selector::tests::support_declared_direct_selects_from_real_belief_preconditions -- --exact`
- Passed `cargo test -p worldwake-ai --lib htn::registry::tests::registry_builds_with_11_methods_without_dead_method_ids -- --exact`
- Passed `cargo test -p worldwake-ai htn::`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai --quiet`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
