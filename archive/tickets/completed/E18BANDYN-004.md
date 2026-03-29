# E18BANDYN-004: Implement EstablishCamp action with faction-scoped camp policy

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-core (bandit camp data contract), worldwake-sim (action payload/duration semantics), worldwake-systems (action def/handler/registry)
**Deps**: E18BANDYN-001 (bandit camp components landed, but their live shape must be corrected here to support the regroup architecture described by E18)

## Problem

Survivors regrouping at a rally point need an `EstablishCamp` action that lawfully creates a new bandit camp through the normal action framework. The live code can already store `BanditCamp` and `BanditCampProfile`, but it cannot yet answer the faction-scoped questions this action depends on:

- which faction an existing camp belongs to,
- which establishment policy applies after survivors regroup away from the old camp,
- whether a faction already has an active camp elsewhere,
- how action duration should resolve from camp policy instead of a hardcoded constant.

Without fixing that shared contract first, an `EstablishCamp` handler would either guess, duplicate state, or introduce a hidden alias path. That would age badly and would also weaken the follow-on abandonment and regroup tickets.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit: faction-scoped camp identity and policy transport across `worldwake-core::BanditCamp`, `worldwake-core::BanditCampProfile`, `worldwake_sim::ActionPayload` / `DurationExpr`, and the worldwake-systems action registry.

1. `BanditCamp` and `BanditCampProfile` already exist in [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs), but their live shape is not sufficient for the ticket narrative:
   - `BanditCamp` currently stores only `supplies: EntityId`.
   - `BanditCampProfile` currently stores `min_regroup_count`, `establishment_duration_ticks`, `flee_wound_threshold`, and `rally_place`, but no faction link.
   - Correction: this ticket must extend the shared contract so faction-scoped camp identity is explicit instead of inferred.
2. The current ticket text is stale when it says commit should attach `BanditCamp { faction, supplies }`; that struct does not exist yet. This is not a reason to keep the old design. It is the exact architectural correction required here.
3. The current action registry is centralized in [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs). There is no separate sim-side def registry wiring file to touch. The live ticket scope must update to that layout.
4. `DurationExpr` has no variant that can resolve establishment duration from bandit camp policy. The live enum in [`crates/worldwake-sim/src/action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) supports travel, combat, metabolism, consult-record, and other existing sources only. Correction: this ticket must add a duration expression for establishment policy rather than smuggling in a fixed magic number.
5. `ActionPayload` has no `EstablishCamp` variant today. That part of the original ticket remains valid.
6. `EntityKind::Container`, `WorldTxn::create_container`, and placement helpers already exist in [`crates/worldwake-core/src/world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs). Camp supply container creation should reuse those primitives rather than inventing a parallel path.
7. `ActionDomain::Generic`, `Interruptibility::InterruptibleWithPenalty`, `PlaceTag::Camp`, `PlaceTag::Forest`, and `EventTag::WorldMutation` all exist as assumed.
8. `members_of(faction)` exists on `World` / `WorldTxn`, but the original ticket omitted that the bandit camp architecture currently lacks a lawful way to discover "the camp for faction X." Follow-on tickets E18BANDYN-005 and E18BANDYN-006 already rely on that answer. Correction: faction identity must become explicit on active camp state in this ticket.
9. The original ticket claimed "moderate body cost" without a live profile-backed source for that number. The current `BanditCampProfile` has no establishment body-cost field, and introducing hardcoded metabolic numbers would violate the repo’s no-magic-number standard. Correction: this ticket should model establishment cost through duration plus explicit supply transfer only; adding profile-driven metabolic cost is a separate follow-up if still wanted.
10. The original "reuse existing camp rather than creating a duplicate" rule is underspecified. Reusing a same-place camp for the same faction is lawful; silently reusing a camp owned by a different faction is not. Correction: same-faction same-place reuse is allowed, cross-faction reuse must fail authoritatively.

## Architecture Check

1. Implementing `EstablishCamp` as a normal action remains the right architecture. It preserves duration, interruption, explicit material transfer, and authoritative commit-time validation instead of hiding camp formation inside a passive system.
2. The current live camp contract is not robust enough for the E18 architecture. The clean fix is to make faction-scoped camp identity explicit in authoritative state and to resolve establishment duration from faction-scoped camp policy through the action semantics layer. That is more beneficial than the current architecture because it removes hidden inference and gives E18BANDYN-004, E18BANDYN-005, and E18BANDYN-006 a single lawful source of truth.
3. The cleaner long-term model would likely move purely faction-scoped policy off the place entity entirely, or split "active camp state" from "faction regroup policy" into separate concepts. That is broader than this ticket. In-scope here: make the current place-backed model coherent without adding shims or duplicate lookup paths.
4. No backwards-compatibility aliases. Update the data contract directly and fix call sites/tests that break.

## Verification Layers

1. faction-scoped camp identity is explicit and roundtrips through world/component plumbing -> focused core unit coverage on `BanditCamp` / `BanditCampProfile`
2. establish duration resolves from bandit camp policy rather than a hardcoded constant -> sim duration semantics/unit coverage
3. affordance/start-gate rejects unlawful requests (wrong place tag, not a faction member, insufficient colocated living members, no carried edible supplies, cross-faction reuse, duplicate active camp elsewhere) -> focused systems action tests plus authoritative start-gate validation
4. commit creates or reuses the correct same-faction camp, creates/moves supply container, and transfers carried edible supplies without duplication -> authoritative world-state assertions and conservation checks
5. interruption loses progress and creates no camp state -> action trace plus authoritative world-state assertions
6. same-place same-faction reoccupation is lawful, but cross-faction reoccupation is not -> focused systems action tests

## What to Change

### 1. Correct the bandit camp shared contract

In [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs):

- extend `BanditCamp` to store the owning `faction: EntityId` alongside `supplies: EntityId`
- extend `BanditCampProfile` to store the owning `faction: EntityId`

Rationale:

- `BanditCamp` needs explicit faction identity so authoritative code can answer "does faction X already have an active camp?" without guessing from incidental ownership.
- `BanditCampProfile` needs explicit faction identity so establishment policy can be recovered and rehomed when a new camp is formed.

### 2. Add EstablishCamp payload and duration support

In [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs):

```rust
pub struct EstablishCampActionPayload {
    pub faction: EntityId,
}
```

- add `ActionPayload::EstablishCamp(EstablishCampActionPayload)`
- add `as_establish_camp()`

In [`crates/worldwake-sim/src/action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs):

- add a `DurationExpr` variant for bandit-camp establishment policy
- resolve it from the payload faction by looking up the canonical `BanditCampProfile` for that faction
- update duration-estimation / serialization / contract tests accordingly

### 3. Register the EstablishCamp action

Add a new module [`crates/worldwake-systems/src/bandit_camp_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs) that:

- registers the action definition and handler
- exports `register_establish_camp_action`

Definition shape:

- `ActionDomain::Generic`
- actor constraints:
  - `ActorAlive`
  - `ActorNotIncapacitated`
  - `ActorHasControl`
  - `ActorNotInTransit`
- target spec:
  - `ActorPlace`
- declarative preconditions:
  - target exists
  - target kind is `Place`
- payload:
  - required `EstablishCampActionPayload { faction }`
- duration:
  - profile-driven duration expression from step 2
- body cost:
  - `BodyCostPerTick::zero()` in this ticket; the explicit material cost is supply transfer, and no profile-backed metabolic cost exists yet
- interruptibility:
  - `InterruptibleWithPenalty`
- visibility:
  - `SamePlace`
- causal tags:
  - `WorldMutation`

Authoritative payload/start/commit validation must enforce:

- actor is a live member of the payload faction
- actor’s current place has `PlaceTag::Camp` or `PlaceTag::Forest`
- at least `BanditCampProfile.min_regroup_count` living members of the payload faction are colocated there
- actor carries at least one edible commodity lot
- no other active `BanditCamp` already exists for the payload faction at a different place
- if the current place already has a `BanditCamp`, it must be for the same faction to be reusable

### 4. Implement commit behavior

`on_commit` must:

1. resolve the actor’s current place
2. resolve or create the same-faction camp supply container at that place
3. create `BanditCamp { faction, supplies }` if none exists there
4. move or rehome the matching `BanditCampProfile` onto the newly active camp place if it currently lives elsewhere
5. transfer the actor’s directly carried edible lots into the camp supply container
6. emit the normal action-commit world mutation event through the existing transaction/event path

Rules:

- same-place same-faction camp: reuse, do not create duplicates
- same-place different-faction camp: reject
- different-place active camp for same faction: reject
- interrupted / aborted action: no camp created, no supply transfer

### 5. Wire the action into the live registry layout

Update:

- [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)
- [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs)

## Files to Touch

- [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs)
- [`crates/worldwake-core/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/lib.rs) if re-export wiring changes are needed
- [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs)
- [`crates/worldwake-sim/src/action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs)
- [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) if duration estimation needs the new variant
- [`crates/worldwake-sim/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/lib.rs)
- [`crates/worldwake-systems/src/bandit_camp_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs)
- [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)
- [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs)

## Out of Scope

- Raid action semantics and handler work (E18BANDYN-003)
- `bandit_camp_system()` abandonment detection (E18BANDYN-005)
- AI candidate generation for `RegroupWithFaction` (E18BANDYN-006)
- planner search integration for regroup goals (E18BANDYN-007)
- route threat estimation (E18BANDYN-008)
- golden T22 integration scenario (E18BANDYN-009)
- broader spec cleanup for whether camp policy should ultimately live on faction entities instead of place entities

## Acceptance Criteria

1. `EstablishCamp` affordance/start succeeds when the actor is a live member of the payload faction, is at a `Camp`/`Forest` place, has enough colocated living faction members, and carries edible supplies
2. `EstablishCamp` rejects non-membership, wrong place tags, insufficient member count, or missing carried edible supplies
3. Establishment duration resolves from the faction-matched `BanditCampProfile`, not from a hardcoded constant
4. Commit creates `BanditCamp { faction, supplies }` on the place when no same-faction camp exists there
5. Commit reuses an existing same-place same-faction camp rather than duplicating it
6. Commit rejects reuse of a camp belonging to a different faction
7. Commit rejects establishing a second active camp for the same faction at another place
8. Commit creates or reuses the camp supply container and transfers the actor’s carried edible lots into it
9. Conservation holds for the transferred commodity lots
10. Abort / interruption leaves no new camp state and preserves the actor’s carried supplies
11. Existing targeted suites and repo quality gates pass

## Tests

### New / Modified Tests

1. `crates/worldwake-core/src/bandit_camp.rs`
Rationale: proves the corrected faction-scoped bandit camp components serialize, compare, and expose the canonical shared contract.

2. `crates/worldwake-sim/src/action_payload.rs`
Rationale: proves the new payload variant and typed accessor are stable and roundtrip safely.

3. `crates/worldwake-sim/src/action_semantics.rs`
Rationale: proves establishment duration resolves from `BanditCampProfile` instead of a magic number and fails clearly when no canonical policy exists.

4. `crates/worldwake-systems/src/bandit_camp_actions.rs`
Rationale: covers lawful start, same-faction reuse, cross-faction rejection, duplicate active-camp rejection, commit-time supply transfer, conservation, and abort behavior.

5. `crates/worldwake-systems/src/action_registry.rs`
Rationale: proves the full live action catalog includes `establish_camp`.

### Commands

1. `cargo test -p worldwake-core bandit_camp`
2. `cargo test -p worldwake-sim action_payload`
3. `cargo test -p worldwake-sim action_semantics`
4. `cargo test -p worldwake-systems bandit_camp_actions`
5. `cargo test -p worldwake-systems action_registry::tests::build_full_action_registries_returns_complete_action_catalog`
6. `cargo test -p worldwake-systems`
7. `cargo test --workspace`
8. `cargo clippy --workspace`
9. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-29
- What changed:
  - corrected the shared bandit-camp contract so both `BanditCamp` and `BanditCampProfile` carry explicit `faction` identity
  - added `ActionPayload::EstablishCamp` plus a profile-driven `DurationExpr::BanditCampEstablishmentProfile`
  - implemented and registered `establish_camp` with authoritative validation for faction membership, regroup count, edible carried supplies, same-faction reuse, and duplicate active-camp rejection
  - commit now creates or reuses the lawful camp state, rehomes the canonical profile, and transfers carried edible lots into the camp supply container with provenance preserved
  - updated planner-facing action classification and duration-contract coverage so the AI/action-registry invariants remain truthful even though regroup-goal planning is still out of scope
- Deviations from original plan:
  - establishment body cost was intentionally left at `BodyCostPerTick::zero()` because the live profile contract still has no lawful source for a separate camp-establishment metabolic cost
  - the final verification exposed a pre-existing planner/frame invariant gap (`GENERIC_PROGRESS_OPS` was missing `AskWitness` despite claiming to cover all planner op kinds); that was corrected while integrating the new action
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo test -p worldwake-ai planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs` passed
  - `cargo test -p worldwake-ai planner_ops::tests::classify_action_def_fixed_name_families_ignore_placeholder_payload_shape` passed
  - `cargo test -p worldwake-ai planner_duration_contract` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
  - `cargo build --workspace` passed
