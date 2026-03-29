# E18BANDYN-006: AI candidate generation for `RaidTarget`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` candidate generation
**Deps**: [archive/tickets/E18BANDYN-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E18BANDYN-002.md), [archive/tickets/E18BANDYN-003.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E18BANDYN-003.md), [archive/tickets/completed/E18BANDYN-004.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E18BANDYN-004.md), [archive/tickets/completed/E18BANDYN-010.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E18BANDYN-010.md)

## Problem

Bandit agents still generate generic `EngageHostile` combat candidates, even though the live architecture already distinguishes `GoalKind::RaidTarget { target }` from ordinary hostility. This means the AI layer is leaving a real E18 semantic distinction unused.

The clean fix is to emit `RaidTarget` for the lawful bandit case only: a living faction member, currently co-located with living non-faction agents, under the same combat-pressure gates and blocked-intent filtering already used by combat candidates.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit: `worldwake_ai::candidate_generation` consuming `worldwake_sim::GoalBeliefView` reads for local co-presence and faction membership, then emitting `worldwake_core::GoalKind::RaidTarget`.

1. `GoalKind::RaidTarget { target }` and `GoalKind::RegroupWithFaction { faction }` already exist in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). The old ticket text was stale in treating them as not-yet-landed candidate targets.
2. The live candidate generator in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) has no raid-specific emission path today. Combat emission is still centralized in `emit_combat_candidates() -> emit_engage_hostile_goals()`, which emits only `GoalKind::EngageHostile { target }`.
3. Reassessment initially suggested the live `GoalBeliefView` already exposed enough lawful surface for the raid half of the work. Final implementation proved that was only partially true:
   - the existing runtime traits already had `factions_of(entity)` and locality reads
   - but the `impl_goal_belief_view!` forwarding macro did not actually forward `factions_of`
   - and candidate generation still needed one precise self-knowledge query to distinguish "member of a bandit faction" from generic faction membership
   Correction: the clean implementation adds a minimal `bandit_factions_of(entity)` read on the belief-view boundary and forwards both faction queries through the live macro/runtime surfaces.
4. The live authoritative camp state already stores faction identity on [`BanditCamp`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs), and faction policy already lives on [`BanditFactionPolicy`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs). The old ticket text was stale in referring to missing camp/faction links.
5. The live combat architecture intentionally keeps one authoritative combat action. [`archive/tickets/E18BANDYN-003.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/E18BANDYN-003.md) already corrected the architecture: `RaidTarget` is an AI-level motive that still plans through the canonical `attack` action path. This ticket must not introduce a parallel authoritative raid action.
6. `RegroupWithFaction` is architecturally blocked by missing belief substrate, not by missing candidate plumbing. The spec requires rally-point knowledge to arrive through lawful belief acquisition, but the live code still has no explicit rally-point belief carrier or query surface. Implementing regroup from direct authoritative reads here would violate `docs/FOUNDATIONS.md` principles on locality and belief-vs-world separation. Correction: regroup is removed from this ticket and deferred to `E18BANDYN-011`.
7. The live suppression policy for `RegroupWithFaction` is also not what the old ticket claimed. In [`crates/worldwake-ai/src/goal_policy.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs), it is suppressed at `GoalPriorityClass::High`, not `Critical`. That stale claim disappears with the regroup scope removal.
8. The live ranking arithmetic already treats `RaidTarget` as enterprise-weighted in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). This ticket does not need a new motive formula; it only needs the right emitted goal kind.
9. `BlockedIntentMemory` already filters generated candidates by exact `GoalKey` plus scoped opportunity in [`crates/worldwake-core/src/blocked_intent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs) and [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). Once raid candidates emit as `RaidTarget`, target-scoped `CombatTooRisky` blockers for that goal key will suppress retries without special-case code.
10. Current focused coverage for candidate generation already lives inside [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), while the live belief-view boundary lives in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) and [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs). The narrowest real verification boundary is to extend those focused unit tests rather than start with goldens.
11. Adjacent contradiction exposed during reassessment: the active spec still describes regrouping as if rally-point belief acquisition already exists. That contradiction is real but not solvable inside a raid-only candidate ticket. It must be handled by `E18BANDYN-011`, not by adding an omniscient shortcut here.

## Architecture Check

1. Emitting `RaidTarget` from the existing candidate-generation pipeline is cleaner than inventing a parallel bandit-candidate subsystem. It keeps one ranking/suppression/planning entry point while preserving the semantic distinction E18 already introduced at the goal layer.
2. The right split is:
   - `RaidTarget` now, using existing local co-presence + faction reads.
   - `RegroupWithFaction` later, only after a lawful rally-point belief path exists.
   That is more beneficial than the current architecture because it fixes the real shipped gap without contaminating the belief model.
3. The minimal shared-boundary addition needed for correctness is a self-knowledge query for bandit faction membership on the belief-view layer. That is cleaner than inferring "bandit" from generic faction membership or from unrelated heuristics.
4. No backwards-compatibility aliasing. For the lawful bandit case, emit `RaidTarget` instead of `EngageHostile`; do not emit both for the same target.

## Verification Layers

1. bandit local co-presence with non-faction agent emits `RaidTarget` instead of generic hostile combat -> focused candidate-generation unit test
2. same-faction colocated agents do not become raid targets -> focused candidate-generation unit test
3. target-scoped blocked intent suppresses generated raid opportunity -> focused candidate-generation unit test proving `BlockedIntentMemory` still filters the emitted raid goal key
4. generic hostile combat for non-bandit actors still emits `EngageHostile` -> focused candidate-generation unit test
5. broader AI ranking/planning surfaces remain coherent because only the emitted goal kind changes -> `cargo test -p worldwake-ai`
6. this ticket does not prove regrouping; the missing lawful belief path is deferred to `E18BANDYN-011`

## What to Change

### 1. Add the minimal belief-view support needed to identify bandit faction membership

In [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) and [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs):

- forward the already-existing `factions_of(entity)` runtime surface through `GoalBeliefView`
- add `bandit_factions_of(entity)` as a minimal self-knowledge query backed by faction membership filtered through `BanditFactionPolicy`

### 2. Add a raid-specific local target query inside candidate generation

In [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs):

- identify whether the acting agent is currently a faction member
- inspect the agent's locally observed co-located agents
- filter to living non-self, non-faction agents
- treat those as raid candidates

This should use existing `GoalBeliefView` reads only. Do not extend the belief-view trait for the raid half of the work.

### 3. Emit `RaidTarget` for the lawful bandit case

- add a dedicated helper such as `emit_raid_target_goals(...)`
- keep the same danger-pressure gate and current-attacker exclusion used by local combat candidate generation
- emit `GoalKind::RaidTarget { target }`
- ensure the ordinary `emit_engage_hostile_goals(...)` path does not also emit `EngageHostile` for the same bandit-vs-nonfaction local target

### 4. Leave regroup out of scope and point to the missing substrate ticket

- do not add `RegroupWithFaction` candidate generation here
- do not add direct `BanditFactionPolicy.rally_place` reads to candidate generation
- do not add a fake rally-point query that bypasses lawful belief transport

## Files to Touch

- [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) (modify)
- [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) (modify)
- [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) (modify)

## Out of Scope

- `RegroupWithFaction` candidate generation
- rally-point belief acquisition or storage
- planner search integration for regroup or raid (`tickets/E18BANDYN-007.md`)
- route threat estimation (`tickets/E18BANDYN-008.md`)
- golden integration scenario T22 (`tickets/E18BANDYN-009.md`)
- any new authoritative combat action surface

## Acceptance Criteria

### Tests That Must Pass

1. A faction-member bandit co-located with a living non-faction agent generates `RaidTarget { target }`
2. The same bandit does not generate `EngageHostile { target }` for that same lawful raid opportunity
3. A bandit co-located only with same-faction members generates no `RaidTarget`
4. A non-bandit actor with a hostile target still generates `EngageHostile`
5. A target-scoped `CombatTooRisky` blocker for `RaidTarget` suppresses that raid candidate
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace`

### Invariants

1. Bandit predation remains an AI-goal distinction over the existing canonical combat action path
2. Candidate generation uses only lawful local/faction reads already exposed by `GoalBeliefView`
3. No omniscient regroup shortcut is introduced
4. No duplicate `RaidTarget` + `EngageHostile` emission for the same bandit local target

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) — bandit local non-faction co-presence emits `RaidTarget`
2. [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) — same-faction co-presence does not emit `RaidTarget`
3. [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) — generic hostile combat still emits `EngageHostile` for the non-bandit case
4. [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) — blocked raid opportunity is filtered by `BlockedIntentMemory`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - corrected the ticket scope from raid+regroup to raid-only and created follow-up [`tickets/E18BANDYN-011.md`](/home/joeloverbeck/projects/worldwake/tickets/E18BANDYN-011.md) for the missing rally-point belief substrate
  - added `emit_raid_target_goals()` and `local_raid_targets()` so bandit actors emit `RaidTarget` for lawful local non-faction targets
  - prevented duplicate `EngageHostile` emission for the same bandit raid opportunity
  - added the minimal belief-view boundary needed for correctness by forwarding `factions_of()` through `GoalBeliefView` and adding `bandit_factions_of()` for precise self bandit-faction identification
  - strengthened focused candidate-generation tests for raid emission, same-faction exclusion, and blocked-memory suppression
- Deviations from original plan:
  - regroup candidate generation was intentionally not implemented because the lawful rally-point belief path does not exist yet
  - implementation touched `worldwake-sim` belief-view surfaces in addition to `worldwake-ai` because the live macro/runtime boundary was missing the required faction forwarding and bandit self-knowledge read
- Verification results:
  - `cargo test -p worldwake-ai candidate_generation`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo build --workspace`

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo build --workspace`
