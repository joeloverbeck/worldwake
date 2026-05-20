# S155: Belief-View Boundary Correctness

## Summary

Close two confirmed FND-14/FND-14A violations on the per-agent belief surface that previously
let planning and affordance code read **current authoritative world state** for entities the
agent had not co-located with this tick:

1. `PerAgentBeliefView::effective_place(entity)` for non-self entities falls back to
   authoritative `world.effective_place(entity)` whenever `knows_entity(entity)` is true —
   and `knows_entity()` returns true for entities known **only** through institutional
   beliefs or last-seen memory (both non-co-located). An agent therefore obtains the
   *current* location of a target it merely remembers or was told about.
2. Before D2, `ControlBeliefView::can_control(actor, entity)` had **no belief gate**: after a
   local unowned-item co-location shortcut it fell straight through to authoritative
   `world.can_exercise_control(actor, entity)`. Its sibling `believed_rights()` already had
   an explicit FND-14/FND-15 accessibility gate; `can_control()` did not, yet it was called
   from belief-facing planning/affordance paths.

This ticket family corrects both so that non-co-located reads return belief/last-seen state
(or nothing), never live remote truth. The authoritative path is preserved for dispatch/commit
only.

## Phase

AI Architecture Consolidation (Adjunct Wave — derived from `reports/ai-architecture-consolidation-first-iteration.md`)

## Status

DRAFT

Implementation status: D1 landed in `archive/tickets/S155BELVIEBOU-001.md` on 2026-05-20; D2
landed in `archive/tickets/S155BELVIEBOU-002.md` on 2026-05-20; D3-D4 remain active in
`tickets/S155BELVIEBOU-003.md`.

## Crates

- `worldwake-sim` — `per_agent_belief_view.rs` (the `SpatialBeliefView::effective_place` and
  `ControlBeliefView::can_control` impls) plus its unit tests. `belief_view.rs` and
  `affordance_query.rs` are **not** modified: fixing `can_control` in place leaves the trait
  declaration and the ~18 belief-facing callers untouched.
- `worldwake-ai` — belief-boundary golden tests only (no caller edits under the in-place fix).

## Dependencies

- E06 (GOAP planner) — completed
- FND-14A canonical split (`per_agent_belief_view.rs`) — present; this spec tightens it

## Problem Statement

### Motivation and evidence

`reports/ai-architecture-consolidation-first-iteration.md` (Finding #1, rated *Critical*;
Finding #7 and #12, rated *Medium/High*) flagged a possible remote-truth leak through the
belief view. Direct code verification confirmed both:

**`effective_place()` (`per_agent_belief_view.rs`):**

```rust
fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
    if entity == self.agent {
        return self.world.effective_place(entity);
    }
    self.believed_entity(entity)
        .and_then(|state| state.last_known_place)
        .or_else(|| {
            self.knows_entity(entity)
                .then(|| self.world.effective_place(entity)) // <-- authoritative leak
                .flatten()
        })
}
```

`knows_entity()` returns true when the entity appears in `belief_store.institutional_beliefs`
(a social claim, not an observation) **or** in the agent's `LastSeenMemory` records (stale,
prior-tick). When either is the *only* source of knowledge and there is no `believed_entity`
record carrying `last_known_place`, the `or_else` branch returns the entity's **current**
authoritative location. This is the exact omniscience FND-14A forbids: "Off-place or delayed
knowledge is always belief-backed; authoritative reads for non-co-located entities are an
FND-14 violation."

`has_authoritative_local_visibility(entity)` (same-tick co-location) is the **only**
FND-14A-legal authoritative read for a non-self entity, plus direct possession by the actor.

**`can_control()` vs `believed_rights()` (`per_agent_belief_view.rs`):**

```rust
fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
    // Effective rights are a social/jurisdictional fact (FND-14/FND-15).
    let accessible = entity == self.agent
        || self.believed_entity(entity).is_some()
        || self.world.possessor_of(entity) == Some(self.agent)
        || self.world.owner_of(entity) == Some(self.agent);
    if !accessible { return Vec::new(); }     // <-- explicit belief gate, early return
    self.world.effective_rights(actor, entity)
}

fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
    if self.world.owner_of(entity).is_none()
        && self.world.effective_place(actor) == self.world.effective_place(entity)
        && matches!(self.world.entity_kind(entity),
            Some(EntityKind::ItemLot | EntityKind::UniqueItem | EntityKind::Container))
    { return true; }                          // FND-14A co-location shortcut (legal)
    self.world.can_exercise_control(actor, entity).is_ok() // <-- no belief gate
}
```

`can_control()` is called from belief-facing affordance/planning paths:
`affordance_query.rs:286`, `affordance_query.rs:378` (`TargetUnownedOrActorControls`
precondition), `worldwake-ai/src/exhaustion.rs:505` (goal feasibility),
`worldwake-ai/src/enterprise.rs:164,170` (planning context). Control rights are a
social/jurisdictional fact (FND-24), so a planning/UI answer about them must be belief-gated
exactly as `believed_rights()` is.

### Why this matters

The belief view is the planner's and (future) player UI's window onto the world. A single
leak here propagates into snapshot admission, strategic place selection, candidate emission,
and visible affordances — producing behavior that looks intelligent but is omniscient,
defeating the core emergence/legibility goal (FND-14, FND-15, FND-29) and breaking agent
symmetry (FND-19) once a human controls the same body.

### Key interview decisions

- The fix is **minimal and at the source**: correct the two accessors so the belief surface
  is belief-correct. Heavier defense-in-depth (snapshot admission-source tagging) is deferred
  to S157, out of the active order, because fixing the source removes the leak the snapshot
  would otherwise amplify.
- Doc-contract updates are folded into this spec, not split into a docs-only spec.

## Design Goals

- `effective_place()` for a non-self entity returns authoritative location **only** for
  same-tick co-located entities (FND-14A) or entities the actor directly possesses; otherwise
  it returns belief-store `last_known_place`, then last-seen memory place, then `None`.
- Planning/affordance code asks "do I *believe* I can control this?" through a belief-gated
  accessor; dispatch/commit continues to ask the authoritative `can_exercise_control()`.
- The two surfaces are already named distinctly — `ControlBeliefView::can_control` (belief view,
  now belief-gated) for planning/affordance, and `World::can_exercise_control` for dispatch — so
  no new accessor is introduced and the leaking belief-facing answer is fixed at its single source.
- Belief-boundary golden tests prove the canonical stale-location, unknown-ownership, and
  control-source-swap scenarios behave correctly and would fail against the pre-fix code.

## Non-Goals

- Snapshot admission-source provenance tagging — deferred to **S157** (out of active order).
- Goal-semantics consolidation, candidate-generation restructuring, ranking changes —
  dismissed or deferred by the triage.
- Any change to authoritative dispatch validation (`can_exercise_control`,
  `validate_*`) — dispatch legality is unchanged; only the belief-facing read is corrected.
- A full CLI/player-POV affordance audit — the control-source-swap golden in this spec is the
  bounded symmetry check; a broader CLI audit is left as follow-up.

## FOUNDATIONS Alignment

| Principle | How this spec satisfies it |
|-----------|----------------------------|
| FND-14 (World ≠ Belief) | Removes the authoritative `effective_place` fallback for non-co-located entities; planning reads belief/memory only. |
| FND-14A (Same-tick co-location is belief-equivalent; social facts are not) | Authoritative `effective_place` read retained exactly for co-located/possessed entities; control rights (a social fact) gated behind explicit belief access like `believed_rights()`. |
| FND-15 (Knowledge is local, carries provenance) | Non-local location comes from `last_known_place`/last-seen records that already carry acquisition metadata, not from live truth. |
| FND-19 (Agent symmetry) | Control-source-swap golden asserts an AI-controlled and human-controlled instance of the same body see the identical lawful affordance set. |
| FND-24 (Ownership/rights distinct) | Belief-gated control answer separates "I believe I may control X" from authoritative "the world will allow it at commit." |
| FND-28 (No backward compat) | The leaking `or_else` branch and the un-gated `can_control` fallthrough are fixed in place, not wrapped; `can_control` stays the single belief-facing control answer (no parallel `believed_can_control` method, so no fossil and no caller migration). |
| FND-29 (Debuggability) | Belief-boundary goldens make the absence of leaks falsifiable; decision traces continue to show belief-sourced places. |

## Section H — Causal Hooks Declaration

### H.1 Information-path analysis
This spec **removes** an illegal zero-hop information path (live remote location/control truth
reaching the planner with no perception, testimony, record, or memory hop). Post-fix, a
non-co-located entity's place reaches the agent only via (a) a `believed_entity` record
written by perception/testimony, or (b) a `LastSeenMemory` record written by prior
observation — both lawful multi-hop carriers. No new information path is introduced.

### H.2 Positive-feedback analysis
None. No amplifying loop is created or modified; this is a read-boundary correctness fix.

### H.3 Concrete dampeners
N/A — no feedback loop introduced (per H.2).

### H.4 Stored state vs. derived read-model
No new stored state. `effective_place`/`can_control` are **derived reads** over existing
authoritative state and belief stores; this spec narrows which source they read. Authoritative
location/control state and the belief store remain the only sources of truth.

### H.5 Planner-formalism analysis
No planner formalism change (no GOAP/HTN behavior change). The fix operates entirely on the
`RuntimeBeliefView`/`PerAgentBeliefView` accessor surface the planner consumes. Plans differ
only insofar as they can no longer be built on leaked truth.

### Agent Profile Scenario Contract
N/A — no new ECS component registered on `EntityKind::Agent`. No `AgentDef`/`spawn_agent`
change. No `Permille` or profile-driven numeric parameter is introduced (pure boundary fix).

## Deliverables

### D1 — Belief-correct `SpatialBeliefView::effective_place()`

Status: landed in `archive/tickets/S155BELVIEBOU-001.md` on 2026-05-20.

Rewrite the non-self path of the `SpatialBeliefView::effective_place` impl
(`per_agent_belief_view.rs:951`) so authoritative `world.effective_place(entity)` is reached **only**
when `has_authoritative_local_visibility(entity)` (same-tick co-location) or
`world.possessor_of(entity) == Some(self.agent)` (direct possession). Otherwise return, in
order: `believed_entity(entity).last_known_place`, then the actor's `LastSeenMemory` record
place for that entity, then `None`. The broad `knows_entity()`-gated `or_else` fallback to
live truth is deleted.

Acceptance: with agent A having last seen target T at P1, T moving to P2, and A receiving no
new observation/testimony/record, `PerAgentBeliefView(A).effective_place(T)` returns P1 (or
None if no belief/memory record), never P2.

### D2 — Belief-gate `ControlBeliefView::can_control` in place
Status: landed in `archive/tickets/S155BELVIEBOU-002.md` on 2026-05-20.

Add the belief-accessibility gate to the existing `ControlBeliefView::can_control` impl
(`per_agent_belief_view.rs:433`), mirroring `believed_rights()`'s gate: keep the existing
FND-14A co-location unowned-item shortcut, then require the entity be belief-accessible
(`entity == self.agent` || `believed_entity(entity).is_some()` || possessed by the view agent ||
owned by the view agent) before consulting `world.can_exercise_control()`; return `false` when
the entity is not belief-accessible. Do **not** introduce a parallel `believed_can_control`
method: `can_control` is the belief-facing control answer (consumed only from belief/planning
paths), so fixing it in place corrects every consumer at once and avoids a fossil second method
(FND-28).

`can_control` has **~18 belief-facing callers** across `worldwake-sim`
(`affordance_query.rs:286,378,933`; `per_agent_belief_view.rs:336`; the `belief_view.rs` blanket
`GoalControlBeliefView` forward) and `worldwake-ai` (`enterprise.rs:164,170`; `exhaustion.rs:505`;
`plan_revalidation.rs:198`; `goal_explanation.rs:293`; `planning_snapshot.rs:1103`;
`planning_state.rs:2956`; `effect_sink_hypothetical.rs:607`; `goal_model.rs:4028`;
`candidate_generation.rs:1641,5352,7157,7177,8312`). **None is a dispatch caller** — authoritative
dispatch uses `World::can_exercise_control` directly, which is **unchanged**. All belief-facing
callers inherit the gate automatically; no caller file is edited. Per the Authoritative-to-AI
Impact Rule, trace the full decision cycle (see the Authoritative-to-AI Impact Analysis section
below) — the candidate-generation and replan ripples are the load-bearing ones.

### D3 — Belief-boundary golden/focused tests
Add tests under the post-S154 golden form (and focused unit tests on `PerAgentBeliefView`):
- **Stale target location** (Regression Scenario D): no remote moved target is revealed; A
  plans from P1 or seeks information, never targets P2.
- **Unknown ownership beside chest** (FND-14A corollary): co-located physical chest actions
  visible; owner/effective-right/control facts unavailable through the belief view unless an
  explicit belief entry exists.
- **Control-source swap symmetry** (FND-19): swapping `ControlSource::Ai`↔`Human` on the same
  body mid-decision yields the identical lawful affordance set.
Each test must fail against the pre-fix accessors and pass after D1/D2.

### D4 — Doc contract update (folded in)
Update `docs/planner-contracts.md` "Entity admission and the belief barrier" section to state:
non-self `effective_place` is belief/last-seen only (authoritative read permitted solely for
co-located or directly-possessed entities); planning/UI control visibility uses belief-gated
`ControlBeliefView::can_control` while dispatch uses authoritative `World::can_exercise_control`.

## Authoritative-to-AI Impact Analysis

D2 modifies `can_control`, which feeds affordance generation and candidate emission, so the
AGENTS.md Authoritative-to-AI Impact Rule applies. (D1 narrows belief-visible places but is a
belief-read change, not authoritative validation; its ripple is covered by the same goldens and
the full AI suite.)

1. `get_affordances` — **trace**: `affordance_query.rs:286,378,933` gate on `can_control`; the
   gate narrows which targets are affordable. Verified by D3 goldens + full AI suite.
2. `generate_candidates` — **trace**: `candidate_generation.rs:1641,5352,7157,7177,8312` gate
   emission on `can_control`; the gate changes which candidates emit. The deepest ripple — must
   be traced explicitly.
3. `search_plan` — covered: no direct `can_control` use; `planning_state.rs:2956` /
   `planning_snapshot.rs:1103` entity filtering inherits the same gate.
4. `BestEffort` action start — N/A: dispatch uses `World::can_exercise_control` (unchanged).
5. `handle_plan_failure` — **trace**: `plan_revalidation.rs:198` uses `can_control` for replan
   legality; the gate affects replan decisions.
6. Payload revalidation — N/A: no synthesized-payload validator changes.
7. Golden tests — D3 belief-boundary goldens + full AI suite (`cargo test -p worldwake-ai`);
   existing trace shifts are expected, world-outcome regressions are not.

## Test Plan

1. Focused: `cargo test -p worldwake-sim per_agent_belief_view` (new accessor unit tests).
2. Golden: `cargo test -p worldwake-ai --test golden_ai <belief_boundary scenario filter>`.
3. Full AI suite: `cargo test -p worldwake-ai` (Authoritative-to-AI Impact Rule step 7).
4. `./scripts/verify.sh` before PR.
