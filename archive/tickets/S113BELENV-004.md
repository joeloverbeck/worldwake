# S113BELENV-004: Remote-target emitters gain envelope reads

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate emission for remote-target goals plus pursuit omission-trace honesty for contradicted target beliefs (`worldwake-ai/src/candidate_generation.rs`, `worldwake-ai/src/decision_trace.rs`)
**Deps**: archive/tickets/S113BELENV-001.md, archive/tickets/S113BELENV-006.md

## Problem

Candidate emitters that fire for **remote** targets — targets the agent is not currently co-located with and must rely on belief for — are the most direct beneficiaries of the envelope. Two such emitters exist today:

- `emit_remote_engage_hostile_targets` (`crates/worldwake-ai/src/candidate_generation.rs:2095`) — emits engage-hostile goals against targets believed to be at other places.
- `emit_remote_raid_targets` (`crates/worldwake-ai/src/candidate_generation.rs:2308`) — emits raid goals against remote target entities.

Today these emitters gate on whether the agent *has* a belief about the target at all, not on whether that belief is fresh, decayed, or refuted. The planner can emit an engage-hostile goal against a target whose last known location is two hundred ticks stale, alongside a fresh one. `S113BELENV-003` now discounts the `RaidTarget` ranking path via target-location confidence, but that downstream scaling is not an emission gate and does not fix contradicted candidate creation. Worse, a contradicted belief (a later observation refuted the target's presence at the remembered place) still gates the emitter open.

This ticket instruments both emitters with envelope-aware gating. After `S113BELENV-006` lands explicit contradiction carriage, the contract is: skip on `Contradicted`, emit on `Stale` (the agent may still want to plan a verification step), and proceed normally on `Certain`/`Probable`. It establishes the pattern for other emitters to adopt in follow-up tickets.

## Assumption Reassessment (2026-04-21)

1. `emit_remote_engage_hostile_targets` at `candidate_generation.rs:2095` and `emit_remote_raid_targets` at `candidate_generation.rs:2308` are the two present-tense `emit_remote_*` emitters in the file. Full list of `emit_*` functions (30 total) includes `emit_need_candidates` (517), `emit_combat_candidates` (957), `emit_political_candidates` (1591), etc., but only the two `emit_remote_*` variants are target-belief-anchored on remote targets by construction — the others either act on co-located data or are not belief-anchored on a single identity target.
2. `GoalBeliefView` exposes (after T001) `believed_target_location(agent, target)`. The emitters call the belief view via whatever handle the candidate-generation surface uses today (`state.last_known_place` shortcuts via `pursuit_belief.rs` do exist but are per-agent, not target-keyed — the envelope accessor generalizes this). Shared abstraction boundary under audit: the emitter → belief-view contract and the envelope `status` semantics.
3. Live `GoalKind` surfaces exercised: `EngageHostile` (emit_remote_engage_hostile_targets) and `Raid` (emit_remote_raid_targets) — confirmed by the emitter names. No goal-model changes (P30's "relevant op kinds" are already correct; this ticket does not add a GoalKind).
5. This is a planner-candidate-generation layer ticket. The exact current operator/affordance surface the scenario depends on is belief-view reads on target location. Skipping on `Contradicted` and continuing on `Stale` affect *whether a candidate is emitted*, not *how* the goal is later planned — so the emitter change is narrow and cannot cascade into search-plan assumptions.
6. AI regression — intended verification layer is candidate-generation focused/unit coverage. `cargo test -p worldwake-ai candidate_generation` is the narrowest real test binary. Runtime `agent_tick` / golden coverage is out of scope (golden is T005, runtime coverage via full AI suite).
8. No heuristic is being removed. The envelope gating is additive — non-envelope-aware branches remain for goals without belief-based target anchors. The substrate the envelope introduces (confidence, status) is new; this emitter instrumentation is the first consumer.
13. Adjacent contradiction: no existing emitter currently uses `believed_target_location` (grep confirmed zero matches workspace-wide). The "migrate `.is_some()` checks" language from the pre-reassessment spec draft was fabricated. This ticket is therefore a first-consumer ticket, not a migration. Also, `S113BELENV-001` does not emit `Contradicted` yet because the live claim store has no refutation carrier; that branch depends on `S113BELENV-006`.

## Architecture Check

1. Both emitters already read belief-view state to make emission decisions; the envelope read is a refinement of that existing read, not a new cross-system authority path (P26). No direct belief-store access, no global queries.
2. The skip-on-`Contradicted` rule operationalizes FND-16 (contradictions first-class): agents should not plan against refuted beliefs, and the envelope makes the refutation visible. Emit-on-`Stale` preserves FND-20 (resource-bounded reasoning): the agent can still consider a stale target and optionally plan verification first.
3. No backward-compatibility shim. The emitters did not read `believed_target_location` before (it didn't exist), so there is no prior path to preserve (P28).
4. Live information-path split under audit: target-location provenance still has two lawful planner-adjacent carriers in this file family. After this ticket, `believed_target_location(...)` is the canonical **emission gate** for contradicted remote targets; the older `pursuit_target_belief(...)` helper remains in-scope for route, confidence, and pursuit-diagnostic provenance on non-contradicted paths. Removing that older provenance helper entirely is deferred.

## Verification Layers

1. Emitter skips emission when target envelope has `status == Contradicted` → focused unit test in `candidate_generation.rs` `#[cfg(test)]` for each of the two emitters.
2. Existing non-contradicted remote-pursuit tests still prove the preserved emission path for ordinary remote targets (`emitted_when_pursuit_conditions_met`, `omitted_when_confidence_too_low`, route/block checks).
3. Pursuit omission diagnostics remain honest when the new contradicted gate fires → focused trace assertion on at least one remote emitter path.
4. No changes to `GroundedGoal` shape, `GoalKey`, or `OpportunityAnchor` — these remain the emitter's output contract (P27, emitter-vs-ranking architecture per validation patterns).

## What to Change

### 1. `emit_remote_engage_hostile_targets` — envelope gating

In `crates/worldwake-ai/src/candidate_generation.rs` at line 2095, where the emitter iterates candidate hostile targets, before committing to emit the `EngageHostile` candidate:

- Read `view.believed_target_location(agent, target)` (or the analog surface; implementer verifies whether the emitter's signature exposes `&dyn GoalBeliefView` or needs routing through its context).
- If `envelope.status == BeliefStatus::Contradicted`, skip emission for this target (continue to the next candidate).
- Otherwise continue through the existing pursuit helper path for confidence/routing/provenance. `Stale` does not block emission — the landed `S113BELENV-003` ranking consumer already discounts the `RaidTarget` motive score, and the agent can still plan verification.

### 2. `emit_remote_raid_targets` — envelope gating

Same pattern applied at `candidate_generation.rs:2308`.

### 3. Unit tests

Add to `candidate_generation.rs` `#[cfg(test)]` (or the nearest existing emitter-test module):

1. `emit_remote_engage_hostile_targets` with a target whose envelope is `Contradicted` → no candidate emitted for that target.
2. `emit_remote_raid_targets` with a target whose envelope is `Contradicted` → no candidate emitted for that target.
3. At least one contradicted-path test should also assert the pursuit omission trace stays honest (new contradicted omission reason instead of pretending the target place was merely unknown).
4. Existing emitted/low-confidence/route/block tests remain the proof surface for non-contradicted remote pursuit paths; no duplicate re-tests needed if those cases still cover the preserved behavior after reassessment.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — two emitter functions + emitter-test additions)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — pursuit omission reason taxonomy gains a contradicted-belief branch so candidate traces stay truthful)

## Out of Scope

- Instrumenting other emitters. The 28 other `emit_*` functions may benefit from envelope reads, but each requires its own design decision about what signal (target presence, remote stock, entity-at-place) applies. Follow-up tickets can address them once this pattern is in place and the landed ranking/revalidation consumers from `S113BELENV-003` prove the end-to-end story.
- Additional ranking scaling for emitted candidates outside the already-landed `S113BELENV-003` `RaidTarget` seam.
- Modifying `GoalKind::EngageHostile` or `GoalKind::Raid` — no goal-model surface changes.
- Changing the `emit_candidate_with_trace` surface or introducing a drive-score field on `GroundedGoal` (architectural mismatch per the Candidate Scoring Architecture pattern — would double-classify as emitter-vs-ranking concern).

## Acceptance Criteria

### Tests That Must Pass

1. Focused contradicted-envelope unit tests in §3 pass.
2. `cargo test -p worldwake-ai candidate_generation` passes (existing emitter tests do not regress — the added gate only affects remote targets whose envelope is `Contradicted`).
3. Full AI suite: `cargo test -p worldwake-ai` passes.

### Invariants

1. Emitters are gates (emit-or-not decisions), not scorers — no motive/drive score attached to the emitted `GroundedGoal` (Candidate Scoring Architecture).
2. `GoalKind::EngageHostile` and `GoalKind::Raid` variant shapes are unchanged; their `payload` surface (action anchors, target binding) is unchanged.
3. No new `Discrepancy` variants emitted by this ticket — a contradicted-target skip simply does not emit; the `Contradicted` discrepancy (if propagated elsewhere) comes from the already-landed revalidation/probe consumers in `S113BELENV-003`, not from candidate generation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — 2 new contradicted-envelope omission tests, one per remote emitter.
2. Existing remote-pursuit tests in the same module remain the proof surface for non-contradicted emission behavior.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::remote_raid_target_omitted_when_target_location_belief_is_contradicted -- --exact`
2. `cargo test -p worldwake-ai candidate_generation::tests::remote_engage_hostile_omitted_when_target_location_belief_is_contradicted -- --exact`
3. `cargo test -p worldwake-ai candidate_generation`
4. `cargo fmt --all`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `./scripts/verify.sh` before PR

## Outcome

Implemented on 2026-04-21.

- `emit_remote_engage_hostile_targets` and `emit_remote_raid_targets` now read `believed_target_location(...)` as the canonical emission gate and skip contradicted remote targets instead of treating refuted target-location beliefs like ordinary pursuit inputs.
- The existing `pursuit_target_belief(...)` helper remains in use for route, confidence, and provenance on non-contradicted paths; this ticket does not remove that older helper.
- `PursuitOmissionReason` now includes `ContradictedBelief`, and the raid-path focused trace proof confirms contradicted omission is recorded honestly instead of masquerading as `UnknownPlace`.
- Test-harness fallout was absorbed in-scope: the `candidate_generation` test double now implements `believed_target_location(...)` against its seeded belief-store claims so envelope-aware emitter tests exercise the real trait contract.
- Deviation from the earlier draft: the honest proof surface is two new contradicted-envelope regressions plus existing remote-pursuit emission tests for preserved non-contradicted behavior, not six new band-by-band tests. The live emitter change only adds a contradicted short-circuit; it does not add a new stale-only emission branch.

## Verification Result

Passed on 2026-04-21:

1. `cargo test -p worldwake-ai candidate_generation::tests::remote_raid_target_omitted_when_target_location_belief_is_contradicted -- --exact`
2. `cargo test -p worldwake-ai candidate_generation::tests::remote_engage_hostile_omitted_when_target_location_belief_is_contradicted -- --exact`
3. `cargo test -p worldwake-ai candidate_generation`
4. `cargo fmt --all`
5. `cargo test -p worldwake-ai`

Not run:

1. `cargo clippy --workspace --all-targets -- -D warnings`
2. `./scripts/verify.sh`
