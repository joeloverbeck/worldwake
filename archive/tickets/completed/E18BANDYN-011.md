# E18BANDYN-011: Add lawful rally-point institutional belief path for `RegroupWithFaction`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — institutional belief substrate, passive perception projection, and AI regroup consumer
**Deps**: [archive/tickets/completed/E18BANDYN-006.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E18BANDYN-006.md), `E18BANDYN-005`, `E18BANDYN-007`, `specs/E18-bandit-dynamics.md`

## Problem

`GoalKind::RegroupWithFaction { faction }` exists and the planner already treats it as a travel-only goal, but the live code still has no lawful way for a survivor to hold the specific belief "my faction regroups at place X" and no AI consumer that can resolve that belief into a destination without directly reading authoritative faction policy.

Today the only authoritative rally-point source is `BanditFactionPolicy.rally_place` in [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs). If regroup were implemented by reading that component from candidate generation or planner state, it would bypass the belief architecture and violate locality and belief-vs-world separation.

## Assumption Reassessment (2026-03-29)

1. The exact shared abstraction boundary under audit is rally doctrine transport: authoritative `BanditFactionPolicy.rally_place` on a faction entity -> lawful local camp observation/perception -> agent-held institutional belief -> `GoalBeliefView`/planning reads consumed by `RegroupWithFaction`.
2. The current ticket text was stale in pointing at `BelievedEntityState` / `AgentBeliefStore.known_entities` as the likely carrier. The live architecture already has a general institutional belief substrate in [`crates/worldwake-core/src/institutional.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs), [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs), and [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs). Correction: rally-point knowledge should extend that substrate, not add a one-off entity-belief field.
3. `BanditCamp` already stores its owning faction directly in [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs). The old ticket text understated this by describing the boundary only in terms of faction policy. An active camp already names the faction-local doctrine source; what is missing is lawful projection of that doctrine into beliefs.
4. `GoalBeliefView` and `RuntimeBeliefView` already expose institutional reads such as `believed_membership(...)` and raw `institutional_belief_claims(...)` in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs). Correction: the missing AI read is not "any institutional query surface," but specifically a rally-point belief query or equivalent canonical institutional-claim access pattern that avoids direct policy reads.
5. The live perception system in [`crates/worldwake-systems/src/perception.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs) currently projects institutional claims only from witnessed events. It does not passively project doctrine from a colocated active camp. Correction: this ticket must extend perception/information transport, not just storage/query helpers.
6. The live `GoalKind::RegroupWithFaction` pipeline is only partially wired. `goal_policy`, `goal_dispatch_decl`, `ranking`, and `goal_model` already know the goal exists, but candidate generation intentionally still defers it; see `still_deferred_goal_kinds_are_not_emitted` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). Correction: this ticket must include candidate emission and destination resolution now that the belief substrate is landing.
7. The planner/operator surface under test is still the live `GoalKind::RegroupWithFaction { faction }` goal family from `E18BANDYN-007`, and the intended operator remains `PlannerOpKind::Travel`. What is missing is canonical destination discovery from belief, not a new operator family.
8. Current focused coverage already exists for adjacent pieces:
   - institutional belief storage/reads in `worldwake-core` belief tests
   - `PerAgentBeliefView` institutional belief access in `worldwake-sim`
   - bandit camp lifecycle in `worldwake-systems`
   - raid-only bandit candidate generation in `worldwake-ai`
   There is no current focused test proving passive rally-point acquisition, rally-point query, regroup candidate emission, or regroup travel target resolution.
9. There is currently zero lawful transport path for the rally fact. The same fact does not yet have duplicate lawful transports in live code. Canonical end state after this ticket: authoritative faction policy -> passive local observation at an active camp -> institutional belief claim in the agent store -> AI regroup reads from that belief only. No direct-policy fallback is allowed to remain in-scope.
10. Adjacent contradiction exposed during reassessment: the active spec text in [`specs/E18-bandit-dynamics.md`](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md) describes rally-point belief acquisition as if the substrate already existed. That contradiction is a required consequence of this ticket and belongs here.
11. `docs/FOUNDATIONS.md` requires one canonical information path, concrete state, and no omniscient AI reads. That rules out both a direct `BanditFactionPolicy` read from candidate generation and a parallel shadow cache on `BelievedEntityState`.
12. Mismatch + correction: the old ticket scoped this as "belief/perception substrate plus AI candidate consumer." The live code shows that regroup also needs planning-time destination resolution from belief. Scope is corrected accordingly.

## Architecture Check

1. The clean architecture is to model rally doctrine as institutional knowledge, because it is faction-scoped doctrine parallel to office-holder, force-control, and faction-membership claims. This reuses the project’s existing provenance, retention, relay, and read semantics instead of inventing a bandit-only belief side channel.
2. Passive observation should project exactly one canonical institutional claim when a faction member is lawfully colocated with an active camp for that faction. That keeps locality intact while avoiding a camp-specific helper in AI code.
3. `RegroupWithFaction` should consume only the belief-borne claim when deciding whether regroup is available and where travel should go. No backwards-compatibility aliasing or temporary direct-policy fallback is acceptable once the belief path exists.

## Verification Layers

1. passive camp observation lawfully creates a rally-point belief only for eligible colocated members -> focused `worldwake-systems` perception test
2. institutional belief store derives the rally-point read deterministically from stored claims -> focused `worldwake-core` belief test
3. `PerAgentBeliefView` exposes the rally-point belief without authoritative faction-policy reads -> focused `worldwake-sim` belief-view test
4. regroup candidate absence/presence depends on rally-point belief rather than direct policy access -> focused `worldwake-ai` candidate-generation test
5. `RegroupWithFaction` resolves travel destination from belief during planning/search rather than direct policy access -> focused `worldwake-ai` goal-model or search test
6. the strongest proof surface for "no omniscient shortcut remains" is the AI-focused tests using a belief view that lacks any direct faction-policy helper. No golden trace is required yet because the invariant is local to candidate/planner substrate, not full emergent scenario behavior.

## What to Change

### 1. Extend the canonical institutional belief substrate for faction rally doctrine

Add a faction-scoped rally-point institutional claim/key and belief-store read that fit the existing institutional-knowledge model.

### 2. Project rally doctrine through lawful local perception at active camps

When a living faction member is colocated with an active camp for their faction, passive perception should project the faction’s current rally doctrine into that member’s institutional belief store with proper provenance and retention. Agents outside that lawful local context must not acquire the claim.

### 3. Expose regroup consumers on the AI belief surfaces

Add the minimal `GoalBeliefView` / `RuntimeBeliefView` read needed for regroup code to query the believed rally point canonically.

### 4. Finish the `RegroupWithFaction` consumer path

Enable regroup candidate generation and planning-time destination resolution from the belief-borne rally-point claim only.

## Files to Touch

- `crates/worldwake-core/src/institutional.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)

## Out of Scope

- `RaidTarget` changes unrelated to rally doctrine
- route threat estimation
- golden T22 regroup scenario coverage
- any direct authoritative `BanditFactionPolicy` read from AI as a shortcut

## Acceptance Criteria

### Tests That Must Pass

1. A colocated eligible bandit member lawfully acquires a rally-point institutional belief from an active camp.
2. A non-member or remote agent does not acquire that rally-point belief.
3. The live belief store and `PerAgentBeliefView` can query the believed rally point for a faction.
4. `generate_candidates(...)` emits `RegroupWithFaction` only when the rally-point belief exists and conditions for regroup otherwise hold.
5. `RegroupWithFaction` resolves travel destination from the believed rally point during planning/search.

### Invariants

1. Rally-point knowledge is institutional belief, not omniscient world access.
2. One fact has one canonical transport path from faction doctrine to agent reasoning.
3. No direct-policy fallback or alias path remains once the institutional belief path exists.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — add focused tests for rally-point institutional reads and conflict handling.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — add focused test proving the view exposes the actor’s rally-point belief and hides other agents’ reads.
3. `crates/worldwake-systems/src/perception.rs` — add focused tests for passive rally-point acquisition by eligible colocated members and rejection for remote/non-member observers.
4. `crates/worldwake-ai/src/candidate_generation.rs` — add focused tests proving regroup remains absent without rally belief and appears when the belief is present.
5. `crates/worldwake-ai/src/goal_model.rs` — add focused test proving regroup travel destination resolves from belief rather than direct policy access.

### Commands

1. `cargo test -p worldwake-core belief::`
2. `cargo test -p worldwake-sim per_agent_belief_view::`
3. `cargo test -p worldwake-systems perception::`
4. `cargo test -p worldwake-ai candidate_generation:: goal_model::`
5. `cargo test -p worldwake-core`
6. `cargo test -p worldwake-sim`
7. `cargo test -p worldwake-systems`
8. `cargo test -p worldwake-ai`
9. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-29
- What actually changed:
  - Added canonical institutional rally doctrine (`InstitutionalClaim::FactionRallyPoint`) plus belief-store/read helpers.
  - Extended passive local perception so colocated faction members at an active bandit camp acquire the rally-point claim through lawful observation rather than AI policy reads.
  - Added AI-side regroup candidate emission and planning-time destination resolution from the believed rally point only.
  - Updated institutional tracing/relay/topic plumbing required for the new claim kind to behave like the rest of the institutional knowledge system.
- Deviations from original plan:
  - The original ticket assumptions about `BelievedEntityState` were stale. The implemented path intentionally did not add a bandit-specific entity-belief field and instead extended the existing institutional belief substrate.
  - Completing the clean architecture required a few additional institutional plumbing updates outside the initially listed files so the new claim kind participates consistently in tell/consult/trace/ranking flows.
- Verification results:
  - Focused tests passed in `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai`.
  - Broader affected crate suites passed: `cargo test -p worldwake-core`, `cargo test -p worldwake-sim`, `cargo test -p worldwake-systems`, `cargo test -p worldwake-ai`.
  - Workspace lint passed with `cargo clippy --workspace --all-targets -- -D warnings`.
