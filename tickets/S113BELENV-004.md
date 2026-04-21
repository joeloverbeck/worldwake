# S113BELENV-004: Remote-target emitters gain envelope reads

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate emission for remote-target goals (`worldwake-ai/src/candidate_generation.rs`)
**Deps**: archive/tickets/S113BELENV-001.md, archive/tickets/S113BELENV-006.md

## Problem

Candidate emitters that fire for **remote** targets — targets the agent is not currently co-located with and must rely on belief for — are the most direct beneficiaries of the envelope. Two such emitters exist today:

- `emit_remote_engage_hostile_targets` (`crates/worldwake-ai/src/candidate_generation.rs:2095`) — emits engage-hostile goals against targets believed to be at other places.
- `emit_remote_raid_targets` (`crates/worldwake-ai/src/candidate_generation.rs:2308`) — emits raid goals against remote target entities.

Today these emitters gate on whether the agent *has* a belief about the target at all, not on whether that belief is fresh, decayed, or refuted. The planner can emit an engage-hostile goal against a target whose last known location is two hundred ticks stale, alongside a fresh one — and ranking has no confidence signal to separate them until T003 lands. Worse, a contradicted belief (a later observation refuted the target's presence at the remembered place) still gates the emitter open.

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

## Verification Layers

1. Emitter skips emission when target envelope has `status == Contradicted` → focused unit test in `candidate_generation.rs` `#[cfg(test)]` for each of the two emitters.
2. Emitter emits normally when target envelope has `status == Certain` or `Probable` → focused unit test covering at least one band.
3. Emitter emits when `status == Stale` (the "plan verification first" semantics) → focused unit test.
4. No changes to `GroundedGoal` shape, `GoalKey`, or `OpportunityAnchor` — these remain the emitter's output contract (P27, emitter-vs-ranking architecture per validation patterns).

## What to Change

### 1. `emit_remote_engage_hostile_targets` — envelope gating

In `crates/worldwake-ai/src/candidate_generation.rs` at line 2095, where the emitter iterates candidate hostile targets, before committing to emit the `EngageHostile` candidate:

- Read `view.believed_target_location(agent, target)` (or the analog surface; implementer verifies whether the emitter's signature exposes `&dyn GoalBeliefView` or needs routing through its context).
- If `envelope.status == BeliefStatus::Contradicted`, skip emission for this target (continue to the next candidate).
- Otherwise emit as today. `Stale` does not block emission — the ranking (T003) will discount its motive score, and the agent can still plan verification.

### 2. `emit_remote_raid_targets` — envelope gating

Same pattern applied at `candidate_generation.rs:2308`.

### 3. Unit tests

Add to `candidate_generation.rs` `#[cfg(test)]` (or the nearest existing emitter-test module):

1. `emit_remote_engage_hostile_targets` with a target whose envelope is `Contradicted` → no candidate emitted for that target.
2. Same emitter with a target whose envelope is `Certain` → candidate emitted.
3. Same emitter with a target whose envelope is `Stale` → candidate emitted.
4. Repeat 1–3 for `emit_remote_raid_targets`.

Six unit tests total. Use fixture belief views that return controlled `BeliefValue<Option<EntityId>>` for the three bands.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — two emitter functions + emitter-test additions)

## Out of Scope

- Instrumenting other emitters. The 28 other `emit_*` functions may benefit from envelope reads, but each requires its own design decision about what signal (target presence, remote stock, entity-at-place) applies. Follow-up tickets can address them once this pattern is in place and the ranking/revalidation consumers (T003) prove the end-to-end story.
- Ranking scaling for emitted candidates (T003 owns ranking-side envelope use).
- Modifying `GoalKind::EngageHostile` or `GoalKind::Raid` — no goal-model surface changes.
- Changing the `emit_candidate_with_trace` surface or introducing a drive-score field on `GroundedGoal` (architectural mismatch per the Candidate Scoring Architecture pattern — would double-classify as emitter-vs-ranking concern).

## Acceptance Criteria

### Tests That Must Pass

1. Six new unit tests in §3 pass.
2. `cargo test -p worldwake-ai candidate_generation` passes (existing emitter tests do not regress — the added gate only affects remote targets whose envelope is `Contradicted`).
3. Full AI suite: `cargo test -p worldwake-ai` passes.

### Invariants

1. Emitters are gates (emit-or-not decisions), not scorers — no motive/drive score attached to the emitted `GroundedGoal` (Candidate Scoring Architecture).
2. `GoalKind::EngageHostile` and `GoalKind::Raid` variant shapes are unchanged; their `payload` surface (action anchors, target binding) is unchanged.
3. No new `Discrepancy` variants emitted by this ticket — a contradicted-target skip simply does not emit; the `Contradicted` discrepancy (if propagated elsewhere) comes from revalidation/probe in T003, not from candidate generation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — 6 new unit tests per §3.
2. If emitter testing lives in a dedicated `tests/` module (not inline), mirror the structure of the nearest existing emitter test.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests` (targeted, narrowed to the file).
2. `cargo test -p worldwake-ai emit_remote` (keyword-narrow to both new-tests).
3. `cargo test -p worldwake-ai` (full AI suite).
4. `cargo clippy --workspace --all-targets -- -D warnings`.
5. `./scripts/verify.sh` before PR.
