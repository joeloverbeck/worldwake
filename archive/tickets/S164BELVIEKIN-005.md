# S164BELVIEKIN-005: Remote-kind-change adversarial belief-wall golden

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None (golden test + scenario only)
**Deps**: archive/tickets/S164BELVIEKIN-001.md, archive/tickets/S164BELVIEKIN-002.md

## Problem

Before this ticket, the kind source-gate fix (tickets 001/002) closed the same-tick omniscient-kind path
for remote entities. Per FND-31, a cross-system belief-boundary fix must be proven
with a negative illegal-path case, not merely a plausible run. This ticket extends the
S162 belief-wall golden family with a **remote kind divergence** scenario: the actor's
last-seen memory says a remote entity was an `Agent`, while live authoritative truth
says the same remote entity is a `Facility` and no carrier reaches the actor.
The distant actor's `entity_kind` / candidate / affordance set must be unchanged
(keeps the stale kind) while authoritative truth diverges.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The S162 belief-wall golden family lives at
   `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (registered in
   `crates/worldwake-ai/tests/scenarios/mod.rs`). This ticket extends that family with
   a remote-kind-change case, mirroring its assertion discipline.
2. `EntityKind` (`crates/worldwake-core/src/entity.rs`) is immutable entity metadata in
   the live model; there is no lawful `agent -> corpse` kind-mutation API to exercise.
   The golden therefore seeds a last-seen memory with `observed_kind: Some(Agent)`
   from ticket 001 and compares otherwise equivalent fixtures where current
   authoritative truth is `Agent` vs. `Facility`. That proves the same FND-14B source
   gate without inventing a nonexistent transition seam.
3. Intended invariant (restated before trusting the scenario narrative): removing the
   actor's belief/local observation of the remote entity's current kind must remove any
   planner candidate or affordance that depends on the current authoritative kind,
   even though authoritative truth diverges (FND-14B / canonical scenario D — stale
   belief survives). This is the negative illegal-path case: the distant actor must
   NOT gain or lose a candidate, affordance, ranking change, or HTN method selection
   solely from the remote authoritative kind divergence.
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

## Verified Layers

1. Distant actor keeps stale kind → decision-trace / belief-view assertion:
   `entity_kind` for the remote entity returns the stale `observed_kind`, not the current
   authoritative kind.
2. No candidate/affordance from the remote divergence → decision-trace / candidate
   surface: the distant actor's candidate and affordance set is unchanged across the
   authoritative `Agent` vs. `Facility` comparison.
3. Authoritative truth diverged → authoritative world-state assertion: the entity's
   authoritative kind is the live value while the actor's belief is the stale value.
4. The three invariants map to three distinct surfaces (belief-view read, candidate/
   decision trace, authoritative world state) — not collapsed into one scenario-level
   assertion.

## Landed Changes

### 1. Added the remote-kind-divergence scenario

Extended `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` in place, because
the belief-wall family is already an inline programmatic golden suite. The added
fixture seeds a distant actor with a last-seen record whose `observed_kind` is
`Some(Agent)`, then compares `Agent` and `Facility` authoritative truth surfaces with
no carrier reaching the actor.

### 2. Asserted the negative illegal-path case

The golden asserts: (a) the distant actor's `entity_kind` for the remote entity stays
the stale kind; (b) candidate and affordance fingerprints are unchanged across the
authoritative divergence; (c) authoritative world kind diverges; and (d) the decision
trace retains the stale-kind hostile candidate.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (modified — added inline scenario, replay, and assertions)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-details/belief-wall-trap.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `specs/S164-belief-view-kind-source-gate.md` (modified — truth-synced deliverable 5 to the immutable-kind proof seam)
- `crates/worldwake-ai/tests/scenarios/mod.rs` unchanged — no sibling scenario module was added.
- No scenario `.ron` fixture was added; the existing belief-wall family uses inline programmatic fixtures.

## Out of Scope

- The accessor and carrier changes (tickets 001/002) — this ticket only adds coverage.
- The bandit footgun and `facility_controller_at` cases (tickets 003/004).

## Acceptance Criteria

### Test Result

1. The added golden proves the distant actor keeps the stale kind, has unchanged
   candidate/affordance fingerprints across authoritative divergence, and records a
   stale-kind decision trace while authoritative truth diverges.
2. Existing non-ignored `worldwake-ai` tests pass with `cargo test -p worldwake-ai`.

### Invariants

1. Remote authoritative kind divergence with no carrier produces no planner candidate,
   affordance, ranking change, or HTN method selection change for a distant actor
   (FND-14B).
2. The golden proves the authored causal reason (belief boundary held under divergence),
   not a structurally-plausible end state (FND-31).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` —
   `golden_belief_wall_trap_remote_kind_change_uses_stale_kind_not_live_truth`.
2. `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` —
   `golden_belief_wall_trap_remote_kind_change_replays_deterministically`.

## Outcome

Completed on 2026-05-22.

- Added Scenario 460 to the existing belief-wall golden family.
- Added an inline fixture that compares stale last-seen `Agent` belief against
  divergent authoritative `Facility` truth without any carrier reaching the actor.
- Regenerated the golden inventory, scenario index, scenario detail page, and coverage
  matrix.
- Corrected the ticket/spec seam from a nonexistent mutable kind transition to the live
  immutable-kind divergence proof.

## Deviations

- The original draft described an `agent -> corpse` kind transition. Live
  `EntityKind` is immutable metadata, so the landed golden uses two equivalent
  fixtures with different current authoritative kinds to prove the same source-gate
  invariant.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai remote_kind_change -- --list`
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::belief_wall_trap::golden_belief_wall_trap_remote_kind_change_uses_stale_kind_not_live_truth -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::belief_wall_trap::golden_belief_wall_trap_remote_kind_change_replays_deterministically -- --exact`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai --test golden_ai belief_wall`
- Passed `cargo test -p worldwake-ai`
- Passed `./scripts/verify.sh`
