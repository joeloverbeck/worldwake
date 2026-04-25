# S125INSTREBOU-004: GoalBeliefView accessor for institutional reward source

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new trait method on `GoalBeliefView` + `RuntimeBeliefView` impl + macro forwarding
**Deps**: S125INSTREBOU-001

## Problem

S125 Deliverable D6 mandates a `GoalBeliefView` accessor that returns whether the actor has a lawful funded reward source for an accusation case. `emit_bounty_posting_candidates` (`crates/worldwake-ai/src/candidate_generation.rs:765-878`) today hard-codes `RewardSource::InstitutionalTreasury { treasury_entity: office }` at lines 867-868 without consulting fund availability. Per FND-7 / FND-14 / FND-14A, the AI crate must read this through the belief view, not directly from world state. This ticket adds the accessor primitive that ticket 006 will consume.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:263`. `RuntimeBeliefView` impl at `crates/worldwake-sim/src/belief_view.rs:1199`. `impl_goal_belief_view!` macro provides blanket forwarding (per the worldwake-validation-patterns "New Component Read by AI Crate" recipe). No existing accessor for treasury/funds. The pattern is exemplified by existing accessors such as `known_institutional_beliefs`.
2. S125 §5 (AI Candidate Generation) and Section H "Information-path analysis > AI crate read path" specify the accessor returns `Option<RewardSource>` — `Some` when a lawful funded source exists, `None` otherwise. The accessor consults the actor's office-holder belief, co-located observation of treasury container contents (FND-14A allows reading lot commodity/quantity at co-location), and any active `RewardEncumbrance` records visible to the actor through their role.
3. Shared abstraction boundary: the `GoalBeliefView` trait and its forwarding macro. This ticket adds one method at three sites (trait declaration, `RuntimeBeliefView` impl, blanket-forwarding macro).
4. Adjacent contradictions: none. Accessor is additive; existing belief-view methods remain unchanged.
5. Live `GoalKind` under test: `PostBounty` (already exists; no GoalKind variant addition). This ticket lands the accessor only — ticket 006 consumes it from `emit_bounty_posting_candidates`.
6. Implementation note on contract shape: for v1, the accessor returns `Some(RewardSource::InstitutionalTreasury { treasury_entity: office })` based on a positive unencumbered office balance, leaving reward sizing to the candidate emitter and authoritative validation. If implementation discovers the accessor must also pre-compute reward quantity (so the emitter can pick a feasible amount), surface as a finding before completing — ticket 006 would then need to adopt the richer return shape.

## Architecture Check

1. The accessor is the AI-crate's read path; authoritative validation in ticket 005 re-checks at start/commit. The accessor does not call into `worldwake-systems` validators, preserving system decoupling (FND-26).
2. The accessor reads only belief-view-accessible state and same-tick co-located observations per FND-14A. Social/relational facts (the actor's right to spend office funds) come from explicit belief entries (the office-holder belief), never from co-location alone.
3. No backward compat: net-new accessor.

## Verification Layers

1. Accessor returns `Some(InstitutionalTreasury)` when the office holder has visible unencumbered funds → focused unit test on `RuntimeBeliefView`.
2. Accessor returns `None` when no funds, when funds are fully encumbered, or when the actor is not the holder → focused unit tests.
3. Single-layer ticket — AI-crate consumption (ticket 006) and authoritative validation (ticket 005) are separate concerns proven by their own tickets.

## What to Change

### 1. New trait method on `GoalBeliefView`

Add to `crates/worldwake-sim/src/belief_view.rs:263`:

```rust
fn actor_lawful_reward_source_for_case(
    &self,
    actor: EntityId,
    accusation: &AccusationCase,
) -> Option<RewardSource>;
```

(Verify the exact `AccusationCase` parameter type during implementation by reading how `known_institutional_beliefs` and `emit_bounty_posting_candidates` currently express the case.)

### 2. `RuntimeBeliefView` implementation

In `crates/worldwake-sim/src/belief_view.rs:1199`'s impl block, implement the accessor by:
- Reading the actor's office-holder belief to identify whether they hold an office relevant to the accusation case (right-to-spend is a belief, not co-location-derived per FND-14A).
- For the held office, observing the treasury container's lots at co-location (the office holder is co-located with the seat during posting; lots inside the container are co-located with the container per the convention documented in ticket 001).
- Subtracting any active `RewardEncumbrance` records on the office.
- Returning `Some(RewardSource::InstitutionalTreasury { treasury_entity: office })` if the unencumbered balance is positive; otherwise `None`.

### 3. Macro forwarding

Forward through `impl_goal_belief_view!` blanket impl so any composing wrapper inherits the new method.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — new trait method + `RuntimeBeliefView` impl + macro forwarding)

## Out of Scope

- Consumer in `crates/worldwake-ai/src/candidate_generation.rs` — ticket 006.
- Authoritative validation re-check at start/commit — ticket 005.
- Stale-balance memory for non-co-located holders — S125 OQ3, deferred.
- Pre-computed reward quantity in the accessor return — only adopt if implementation discovers ticket 006 needs it; otherwise leave reward sizing to the consumer.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `accessor_returns_institutional_source_for_holder_with_funded_unencumbered_office`.
2. New focused test: `accessor_returns_none_for_non_holder`.
3. New focused test: `accessor_returns_none_when_office_has_no_treasury`.
4. New focused test: `accessor_returns_none_when_office_funds_are_fully_encumbered`.
5. Existing suite: `cargo test -p worldwake-sim`.

### Invariants

1. Accessor reads only from belief-view-accessible state and same-tick co-located observation; no world-state read for non-co-located entities or social/relational facts (FND-14, FND-14A).
2. Accessor is read-only.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` (existing `#[cfg(test)]` block, or sibling `tests/` module — match the existing convention) — four `accessor_*` tests.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
3. `scripts/verify.sh`
