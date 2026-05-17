# S148PORMOTBAC-007: Resume/Abandon condition evaluator with Discrepancy::AbandonConditionFired variant

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new `evaluate_resume_abandon_conditions` evaluator in `crates/worldwake-ai/src/agent_tick/frame.rs` alongside existing `patience_limit` consumption at line 547+; new `Discrepancy::AbandonConditionFired(IntentionAbandonConditionDiscriminant)` variant in `crates/worldwake-core/src/discrepancy.rs` with payload-free discriminant mirror; FIFO cap enforcement at `causal_links` push sites
**Deps**: `S148PORMOTBAC-005`, `S148PORMOTBAC-006`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S148 PR-1's lifecycle extension makes intention resume and abandon decisions inspectable. Today's `frame.rs` consumes `patience_limit` at line 547+ and transitions `FrameState::Active ↔ Suspended ↔ Exhausted` (e.g., the existing Suspended→Active resume at line 519-523), but the transitions are decided implicitly by stall counters and have no typed cause record. After tickets 005-006 land the condition enum types and the `IntentionFrame.resume_conditions`/`abandon_conditions`/`causal_links` fields, ticket 007 wires the evaluator: each tick (alongside the existing `patience_limit` check), the evaluator walks the suspended frame's `resume_conditions` (resume if any holds) and the active/suspended frame's `abandon_conditions` (abandon if any holds), emits a typed `Discrepancy::AbandonConditionFired` variant when abandoning, and pushes the firing event into `causal_links` (bounded by `CognitiveProfile.causal_links_per_step_cap` per spec D9).

## Assumption Reassessment (2026-05-17)

1. Existing frame lifecycle in `crates/worldwake-ai/src/agent_tick/frame.rs`:
   - `FrameState::Suspended → Active` resume path at lines 519-523 (resumes when activity is detected, does not reset `stalled_ticks`).
   - `patience_limit` consumption at line 547+ (checks `stalled_ticks >= patience_limit`; transitions to `Exhausted`).
   - Frame-state transitions are decided inside frame.rs; the agenda manager at `crates/worldwake-ai/src/agenda_manager.rs::tick_agenda` handles separate candidate-revival concerns and is NOT changed by this ticket.
2. `Discrepancy` enum at `crates/worldwake-core/src/discrepancy.rs:8` derives `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. The new variant's payload type must satisfy these derives (esp. `Copy`). `IntentionAbandonCondition` (ticket 005) is `Clone + Eq + Ord` but NOT `Copy` (carries variant-bearing payload like `OpportunityAnchor`). Therefore the new variant carries a payload-free `IntentionAbandonConditionDiscriminant` (mirroring the `MotiveSourceDiscriminant` precedent at `crates/worldwake-core/src/motive_source.rs:25`) — the discriminant fits within `Discrepancy`'s `Copy` budget. Full condition recoverable from `frame.abandon_conditions` if downstream needs it.
3. Cross-crate `Discrepancy` exhaustive match audit: 23 genuinely-exhaustive `match.*Discrepancy` sites across 5 files at `crates/worldwake-core/src/event_record.rs` (9), `crates/worldwake-systems/src/perception.rs` (7), `crates/worldwake-core/src/belief.rs` (5), `crates/worldwake-ai/src/failure_handling.rs` (1), `crates/worldwake-ai/tests/golden_perception_omission.rs` (1). Total `Discrepancy::` references workspace-wide: ~178 (mostly construction); the 23 exhaustive sites are the migration target.
4. Shared abstraction under audit: the existing `FrameState` transition machinery in `frame.rs`. This ticket layers a new evaluator atop the existing transitions — the evaluator runs before the existing `patience_limit` check; if a resume_condition or abandon_condition fires, the evaluator's `FrameDecision` overrides the implicit stall-based transition. `PatienceExhausted` is itself an `IntentionAbandonCondition` variant (per ticket 005), so the existing `patience_limit` check still drives the abandon path — it now does so by constructing `IntentionAbandonCondition::PatienceExhausted` and routing through the new evaluator. This unifies the two paths under one typed surface.
5. `causal_links` cap from `CognitiveProfile.causal_links_per_step_cap` at `cognitive_profile.rs:125`: the evaluator is the primary push site for `IntentionFrame.causal_links` (push the event that caused the resume/abandon). FIFO eviction: `if vec.len() >= cap { vec.remove(0); } vec.push(event_id);`.

## Architecture Check

1. FND-21 alignment: every resume and abandon decision now carries an inspectable typed cause (a specific `IntentionResumeCondition` or `IntentionAbandonCondition` variant). Reviewers and observers can trace why a frame state changed without inspecting evaluator code or running a debugger.
2. FND-26 alignment: the evaluator emits a `Discrepancy::AbandonConditionFired` typed value into the existing discrepancy stream rather than directly calling other systems. Downstream systems (failure handling, observer) consume the typed value through the existing state-mediated discrepancy surface.
3. FND-28 alignment: `PatienceExhausted` is unified under the typed abandon-condition surface rather than living as a special-case parallel path. The existing `patience_limit` check at line 547+ continues to fire, but routes its decision through the new evaluator's typed cause record.
4. FND-29A alignment: `causal_links` cap is enforced FIFO at push sites; per-intention growth is bounded by the existing `CognitiveProfile.causal_links_per_step_cap` profile field (no new magic constant).

## Verification Layers

1. Evaluator decision correctness → focused unit tests in `crates/worldwake-ai/src/agent_tick/frame.rs::tests`: for each `IntentionResumeCondition` variant, construct a fixture frame and belief view that triggers the condition, assert `evaluate_resume_abandon_conditions` returns `FrameDecision::Resume`; for each `IntentionAbandonCondition` variant, assert `FrameDecision::Abandon(discriminant)`
2. `Discrepancy::AbandonConditionFired` emission → focused unit test asserting that when the evaluator decides Abandon, the existing discrepancy-emission path receives the new variant with the correct discriminant
3. `causal_links` cap enforcement → focused unit test pushing `causal_links_per_step_cap + 2` events and asserting `vec.len() == cap` with the oldest two evicted
4. Cross-crate exhaustive-match completeness → workspace compilation under `cargo clippy --workspace --all-targets -- -D warnings` — every match site listed in Assumption Reassessment item 3 either gains an arm for `AbandonConditionFired` or routes through an updated catch-all

## What to Change

### 1. Add `IntentionAbandonConditionDiscriminant` to `intention_condition.rs`

In `crates/worldwake-core/src/intention_condition.rs` (alongside the enums added in ticket 005):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IntentionAbandonConditionDiscriminant {
    MotiveSourceLost,
    AssumptionPermanentlyBroken,
    OpportunityForeverGone,
    PatienceExhausted,
    ArtifactDestroyed,
    ArtifactLegalEffectLost,
}

impl From<&IntentionAbandonCondition> for IntentionAbandonConditionDiscriminant {
    fn from(c: &IntentionAbandonCondition) -> Self {
        match c {
            IntentionAbandonCondition::MotiveSourceLost(_)            => Self::MotiveSourceLost,
            IntentionAbandonCondition::AssumptionPermanentlyBroken(_) => Self::AssumptionPermanentlyBroken,
            IntentionAbandonCondition::OpportunityForeverGone(_)      => Self::OpportunityForeverGone,
            IntentionAbandonCondition::PatienceExhausted              => Self::PatienceExhausted,
            IntentionAbandonCondition::ArtifactDestroyed(_)           => Self::ArtifactDestroyed,
            IntentionAbandonCondition::ArtifactLegalEffectLost(_)     => Self::ArtifactLegalEffectLost,
        }
    }
}
```

Re-export from `crates/worldwake-core/src/lib.rs`.

### 2. Add `Discrepancy::AbandonConditionFired` variant

In `crates/worldwake-core/src/discrepancy.rs:8`, add:

```rust
AbandonConditionFired(IntentionAbandonConditionDiscriminant),
```

Ord placement follows existing variant order; verify the `Ord` derive's outcome for the new variant is stable. Migrate the 23 exhaustive-match sites listed in Assumption Reassessment item 3 — each gets a new arm. For sites that route Discrepancies through a generic narrative-rendering helper (e.g., perception logging), the arm can format the discriminant via `Debug` or with explicit per-variant text; the format choice is a path-level detail decided at implementation time.

### 3. Add `evaluate_resume_abandon_conditions` evaluator

In `crates/worldwake-ai/src/agent_tick/frame.rs` (alongside the existing transition logic near line 506+):

```rust
pub(crate) enum FrameDecision {
    Resume,
    Abandon(IntentionAbandonConditionDiscriminant),
}

pub(crate) fn evaluate_resume_abandon_conditions<V: GoalBeliefView>(
    frame: &IntentionFrame,
    belief: &V,
    agent: EntityId,
    tick: Tick,
) -> Option<FrameDecision> {
    // First check abandon conditions — abandon takes precedence over resume
    for cond in &frame.abandon_conditions {
        if condition_holds(cond, frame, belief, agent, tick) {
            return Some(FrameDecision::Abandon(
                IntentionAbandonConditionDiscriminant::from(cond),
            ));
        }
    }
    // Then check resume (only meaningful if frame.state is Suspended)
    if matches!(frame.state, FrameState::Suspended { .. }) {
        for cond in &frame.resume_conditions {
            if resume_condition_holds(cond, frame, belief, agent, tick) {
                return Some(FrameDecision::Resume);
            }
        }
    }
    None
}
```

`condition_holds` and `resume_condition_holds` are file-private predicate helpers — one match arm per variant of `IntentionAbandonCondition` and `IntentionResumeCondition` respectively. Each arm reads the agent's belief view (FND-7 / FND-14) to decide whether the condition holds. Predicate semantics:

- `MotiveSourceLost(discriminant)`: holds when no `MotiveSourceRef` of that discriminant is present in the agent's motive ledger (read via belief view).
- `AssumptionPermanentlyBroken(assumption)`: holds when the assumption check returns a non-recoverable failure (the existing assumption-evaluation helper already classifies recoverable vs. permanent).
- `OpportunityForeverGone(anchor)`: holds when the anchor's entity is `is_dead()` or otherwise no longer satisfies the opportunity contract.
- `PatienceExhausted`: holds when `frame.stalled_ticks >= frame.patience_limit` (mirrors the existing line-547+ check; routing this through the evaluator unifies the two paths under one typed surface).
- `ArtifactDestroyed(entity_id)`: holds when `belief.artifact_header(entity_id).existence == ArtifactExistence::Destroyed`.
- `ArtifactLegalEffectLost(entity_id)`: holds when `ArtifactLegalEffectTag::from(&belief.artifact_header(entity_id).legal_effect) != ArtifactLegalEffectTag::Active`.

Resume predicates symmetric:

- `BeliefStatusChanged { subject, target_status }`: holds when the agent's belief about `subject` matches `target_status`.
- `OpportunityVisible(anchor)`: holds when the anchor's opportunity is currently visible to the agent.
- `LocationReached(entity_id)`: holds when `belief.effective_place(agent) == Some(entity_id)`.
- `TickElapsed(n)`: holds when `tick.0.saturating_sub(suspended_at_tick) >= n` (the suspended_at lives inside `FrameState::Suspended { suspended_at, … }`).
- `ArtifactLegalEffectActive(entity_id)`: holds when `ArtifactLegalEffectTag::from(&belief.artifact_header(entity_id).legal_effect) == ArtifactLegalEffectTag::Active`.

### 4. Wire the evaluator into the existing transition path

Integrate the call into `frame.rs` immediately before the existing `patience_limit` check at line 547+ and the Suspended→Active resume at line 519-523. When the evaluator returns:

- `Some(FrameDecision::Abandon(discriminant))`: emit `Discrepancy::AbandonConditionFired(discriminant)` through the existing discrepancy-emission path; transition `frame.state → FrameState::Exhausted`; push the firing tick as an `EventId` into `frame.causal_links` (bounded by cap; see change 5).
- `Some(FrameDecision::Resume)`: transition `frame.state → FrameState::Active` (preserve the existing "do NOT reset stalled_ticks" rule at line 520-522); push the firing tick into `causal_links` (bounded by cap).
- `None`: fall through to the existing implicit transitions.

The existing `patience_limit` check at line 547+ is updated to construct an `IntentionAbandonCondition::PatienceExhausted` and route through the same evaluator path rather than maintaining a parallel transition.

### 5. Enforce `causal_links` cap with FIFO eviction

Add a small helper at the push sites:

```rust
fn push_causal_link_bounded(frame: &mut IntentionFrame, cap: u32, event_id: EventId) {
    let cap = cap as usize;
    if cap == 0 {
        return;
    }
    while frame.causal_links.len() >= cap {
        frame.causal_links.remove(0);
    }
    frame.causal_links.push(event_id);
}
```

The cap value is read from `CognitiveProfile.causal_links_per_step_cap` at the agent's cognitive profile (available through the existing belief-view accessor or runtime read pattern at the call site).

## Files to Touch

- `crates/worldwake-core/src/intention_condition.rs` (modify — add `IntentionAbandonConditionDiscriminant` enum + `From<&IntentionAbandonCondition>` impl; tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export the new discriminant)
- `crates/worldwake-core/src/discrepancy.rs` (modify — add `AbandonConditionFired` variant; verify Ord derive stability)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — add `FrameDecision` enum + `evaluate_resume_abandon_conditions` + predicate helpers + cap helper; integrate into existing transition path at lines 506+, 519-523, 547+; rewrite existing `patience_limit` check to route through the evaluator)
- `crates/worldwake-core/src/event_record.rs` (modify — 9 exhaustive `match.*Discrepancy` sites need new arm)
- `crates/worldwake-systems/src/perception.rs` (modify — 7 exhaustive match sites)
- `crates/worldwake-core/src/belief.rs` (modify — 5 exhaustive match sites)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — 1 exhaustive match site)
- `crates/worldwake-ai/tests/golden_perception_omission.rs` (modify — 1 exhaustive match site)

## Out of Scope

- Populating `motive_refs`, `explicit_claims`, and `causal_links` on initial intention creation (the agenda manager and intention-establishment paths in other tickets — this ticket reads the populated fields and pushes additional `causal_links` events from the evaluator)
- Observer rendering of the new evaluator's decisions (ticket 009)
- Golden coverage exercising each `IntentionResumeCondition` and `IntentionAbandonCondition` branch (ticket 010)
- Cross-crate discrepancy consumers beyond the 23 exhaustive match sites (catch-all `_` arms in non-exhaustive sites continue to work without modification)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core intention_condition::tests::abandon_condition_discriminant_*` — new tests: `From<&IntentionAbandonCondition>` correctness for each variant
2. `cargo test -p worldwake-core discrepancy` — new variant's serde round-trip, ord stability
3. `cargo test -p worldwake-ai agent_tick::frame::tests::evaluate_*` — new focused tests covering each resume and abandon condition variant
4. `cargo test -p worldwake-ai agent_tick::frame::tests::causal_links_cap_*` — FIFO eviction at and above cap
5. Existing suite: `cargo test --workspace`
6. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `evaluate_resume_abandon_conditions` is the single source of truth for `FrameState` resume and abandon decisions; the legacy `patience_limit` check routes through it via `IntentionAbandonCondition::PatienceExhausted`.
2. Every `IntentionAbandonCondition` variant fires through `Discrepancy::AbandonConditionFired(IntentionAbandonConditionDiscriminant)` — the discriminant variant is exhaustively 1:1 with the source enum's variants.
3. `frame.causal_links.len() <= cognitive.causal_links_per_step_cap` at all times after push; FIFO eviction guarantees the bound.
4. No `Discrepancy` exhaustive match site is missing an `AbandonConditionFired` arm.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/intention_condition.rs::tests` — add: `abandon_condition_discriminant_mirrors_every_variant`
2. `crates/worldwake-core/src/discrepancy.rs::tests` — extend with `abandon_condition_fired_serde_round_trip`
3. `crates/worldwake-ai/src/agent_tick/frame.rs::tests` — add ~12 focused tests: one per resume condition variant + one per abandon condition variant; `causal_links_cap_evicts_oldest_in_fifo_order`; `patience_exhausted_routes_through_evaluator`; `abandon_takes_precedence_over_resume_when_both_fire`

### Commands

1. `cargo test -p worldwake-core intention_condition discrepancy`
2. `cargo test -p worldwake-ai agent_tick::frame`
3. `./scripts/verify.sh`
