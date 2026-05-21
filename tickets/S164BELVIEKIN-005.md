# S164BELVIEKIN-005: Remote-kind-change adversarial belief-wall golden

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None (golden test + scenario only)
**Deps**: archive/tickets/S164BELVIEKIN-001.md, S164BELVIEKIN-002

## Problem

The kind source-gate fix (tickets 001/002) closes the same-tick omniscient-kind path
for remote entities. Per FND-31, a cross-system belief-boundary fix must be proven
with a negative illegal-path case, not merely a plausible run. This ticket extends the
S162 belief-wall golden family with a **remote kind change** scenario: an entity
changes kind (agent → corpse) at a remote place with no carrier reaching a distant
actor; the distant actor's `entity_kind` / candidate / affordance set must be
unchanged (keeps the stale kind) while authoritative truth diverged.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The S162 belief-wall golden family lives at
   `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (registered in
   `crates/worldwake-ai/tests/scenarios/mod.rs`). This ticket extends that family with
   a remote-kind-change case, mirroring its assertion discipline.
2. `EntityKind` (`crates/worldwake-core/src/entity.rs:7`) includes `Agent` and a kind
   the corpse transition targets; the golden seeds a remote agent observed via
   last-seen memory (with `observed_kind: Some(Agent)` from ticket 001), then transitions
   it authoritatively while the distant actor receives no carrier.
3. Intended invariant (restated before trusting the scenario narrative): removing the
   actor's belief/local observation of the remote entity's current kind must remove any
   planner candidate or affordance that depends on the new kind, even though
   authoritative truth changed (FND-14B / canonical scenario D — stale belief survives).
   This is the negative illegal-path case: the distant actor must NOT gain a candidate,
   affordance, ranking change, or HTN method selection from the remote kind change.
4. Verification layer: this is golden E2E coverage. The candidate/affordance absence is
   asserted at the decision-trace / candidate surface (not merely "the run looked
   plausible"), and authoritative divergence is asserted against world state — the two
   are distinct proof surfaces per FND-31.
5. Dependency surfaces: ticket 001 provides `observed_kind` (so the seeded last-seen
   record carries the stale kind), and ticket 002 provides the gated `entity_kind`
   accessor (so the distant actor reads the stale kind). Without both, the golden would
   either fail to compile (no `observed_kind`) or pass for the wrong reason (live read
   still returns the new kind).

## Architecture Check

1. Building on the existing S162 belief-wall family reuses the established
   negative-case harness rather than inventing a parallel one, keeping the systemic
   validation surface consistent (FND-31).
2. The golden asserts candidate/affordance *absence or invariance* against authoritative
   *divergence* — proving the right cause (belief boundary held), not just a plausible
   end state. No structural-activation-only assertion.

## Verification Layers

1. Distant actor keeps stale kind → decision-trace / belief-view assertion:
   `entity_kind` for the remote entity returns the stale `observed_kind`, not the new
   authoritative kind.
2. No candidate/affordance from the remote change → decision-trace / candidate surface:
   the distant actor's candidate and affordance set is unchanged across the
   authoritative kind transition.
3. Authoritative truth diverged → authoritative world-state assertion: the entity's
   authoritative kind is the new value while the actor's belief is the old value.
4. The three invariants map to three distinct surfaces (belief-view read, candidate/
   decision trace, authoritative world state) — not collapsed into one scenario-level
   assertion.

## What to Change

### 1. Add the remote-kind-change scenario

Extend `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (or add a sibling
scenario module registered in `tests/scenarios/mod.rs`) seeding a distant actor with a
last-seen record of a remote agent (`observed_kind: Some(Agent)`), then transitioning
that agent's authoritative kind (agent → corpse) at the remote place with no carrier.

### 2. Assert the negative illegal-path case

Assert: (a) the distant actor's `entity_kind` for the remote entity stays the stale
kind; (b) its candidate/affordance set is unchanged across the transition; (c)
authoritative world kind diverged. Use the S162 assertion helpers/discipline.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (modify — add scenario + assertions)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — register new scenario if added as a sibling)
- `Likely:` a scenario `.ron` fixture if the family uses external scenario files — confirm by inspecting how `belief_wall_trap.rs` constructs its world (inline vs. RON) during reassessment.

## Out of Scope

- The accessor and carrier changes (tickets 001/002) — this ticket only adds coverage.
- The bandit footgun and `facility_controller_at` cases (tickets 003/004).

## Acceptance Criteria

### Tests That Must Pass

1. The new golden: distant actor keeps the stale kind, gains no candidate/affordance
   from the remote kind change, while authoritative truth diverged.
2. All existing goldens: `cargo test -p worldwake-ai`.

### Invariants

1. A remote entity-kind change with no carrier produces no planner candidate,
   affordance, ranking change, or HTN method selection for a distant actor (FND-14B).
2. The golden proves the authored causal reason (belief boundary held under divergence),
   not a structurally-plausible end state (FND-31).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` — remote-kind-change
   negative-case golden, mirroring the S162 assertion discipline.

### Commands

1. `cargo test -p worldwake-ai -- --list` (confirm the new test name before finalizing)
2. `cargo test -p worldwake-ai belief_wall`
3. `./scripts/verify.sh`
