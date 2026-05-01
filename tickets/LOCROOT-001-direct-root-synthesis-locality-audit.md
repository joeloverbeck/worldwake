# LOCROOT-001: Audit direct-root synthesis for `EntityAtActorPlace` / `ActorPlace`-precondition actions

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Possibly — depending on audit findings, may add locality gates to additional `synthesized_root_candidate_targets` arms
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md, archive/tickets/S129CIREM-003-tell-session-vs-self-care.md

## Problem

S129CIREM-003 fixed a planner-correctness bug in
`GoalOffer::synthesized_root_candidate_targets`
(`crates/worldwake-ai/src/goal_model.rs:2373+`): the `PlannerOpKind::Wash`
arm could synthesize a direct root for a remote basin, even though
the `Wash` action's target spec is `EntityAtActorPlace`. The fix
made the arm require `actor_place` to be in `self.evidence_places`
before synthesizing, forcing remote wash plans to compose through
`travel + wash` instead of selecting an invalid direct root.

The architectural concern: the same gap may exist on other arms.
Inspecting `synthesized_root_candidate_targets` shows two distinct
gating patterns:

- **Wash** (`PlannerOpKind::Wash`): explicit locality check after
  CIREM-003.
- **Investigate** (`PlannerOpKind::Investigate`): explicit locality
  check via `actor_place == Some(*place)` (predates this audit).
- **Trade**, **Harvest**: target spec is `EntityAtActorPlace`, but
  the synthesis arms emit `self.evidence_entities` directly without
  verifying the entity's last-known place equals `actor_place`. The
  affordance query upstream may already enforce co-location, but
  CIREM-003's bug shows that "the precondition exists" does not
  imply "the synthesizer enforces it" — Wash had the same target
  spec and was emitting non-local roots.

The audit answers: for every `synthesized_root_candidate_targets`
arm whose action has an `EntityAtActorPlace` or `ActorPlace`
precondition, does the synthesizer require local evidence at the
actor's place, or is correctness presumed from upstream filters?

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Live `synthesized_root_candidate_targets`** at
   `crates/worldwake-ai/src/goal_model.rs:2373+`. Arms verified to
   exist: Trade, Harvest, Wash, PressForceClaim, Investigate, plus
   the post-2459 arms not read in detail (Tell, Accuse, ClaimBounty,
   PostNotice, PostBounty, EstablishCamp, etc., per the test names
   visible in the file).
2. **Live target-spec referent**: `worldwake_sim::TargetSpec::EntityAtActorPlace`
   and `worldwake_sim::TargetSpec::ActorPlace` are the locality-bearing
   specs. Verify by `grep -rn 'TargetSpec::EntityAtActorPlace\|TargetSpec::ActorPlace' crates/worldwake-systems/`
   to find the action registrations using each.
3. **Live Wash gate (post-CIREM-003)**:
   `if actor_place.is_none_or(|place| !self.evidence_places.contains(&place)) { return NoSynthesisPath; }`
   plus the basin-anchor / single-evidence-entity branches. This is
   the canonical gate pattern.
4. **Live Investigate gate**: `actor_place == Some(*place)` with
   `*place` derived from `GoalKind::InvestigateViolation { place, .. }`.
   Equivalent to Wash's gate, expressed via goal payload.
5. **Live Trade gate**: only checks `def.targets` is
   `EntityAtActorPlace` and `evidence_entities.len() == 1`. Does
   not verify `actor_place` is in `evidence_places` or that the
   evidence entity's last-known place equals `actor_place`. The
   audit must determine whether upstream affordance filtering
   already enforces co-location, or whether this is the same gap
   Wash had.
6. **Live Harvest gate**: same shape as Trade. Same audit question.
7. **Mismatch + correction (likely)**: if upstream affordance
   filtering (`get_affordances`) already drops non-local
   `EntityAtActorPlace` candidates, the Trade/Harvest synthesizers
   are correct by construction and the audit produces a no-op +
   documentation. If upstream filtering does not, the synthesizers
   are silently emitting non-local roots and the bug is hidden by
   downstream revalidation rejection. In either case, the audit's
   deliverable is the explicit answer + per-arm locality gate (or
   per-arm comment naming the upstream guarantee).
8. **Coverage gap (precision-rules §3)**: existing focused tests at
   `goal_model.rs::tests` (e.g.
   `grounded_goal_synthesizes_trade_root_targets_from_single_evidence_entity`)
   exercise the Trade arm. Add a test asserting the Trade arm
   *refuses* to synthesize when the evidence entity is not at the
   actor's place, mirroring the Wash arm's contract. Same for
   Harvest. If the upstream filter is the canonical guard, the
   refusal test still passes — the synthesizer would have been
   called with an evidence_entities filtered upstream.
9. **First failure boundary (precision-rules §9)**: this ticket
   isolates the synthesis layer specifically. If the bug surfaces
   only under stale-belief conditions where an entity's last-known
   place is wrong, the upstream affordance query may not catch it
   and the synthesizer's locality check is the correct defense.
10. **Information-path refactor discipline**: this ticket does not
    add new transport paths. It either adds a defensive check that
    matches an existing upstream guarantee (preserving canonical
    path) or surfaces a redundancy that should be removed instead
    of adding a parallel guard. The audit decides per-arm.

## Architecture Check

1. **No backwards-compatibility shim**: any locality gate added is
   the live authoritative path, with no preserved alternate path.
2. **Defense in depth vs. duplication**: adding a synthesizer-side
   locality check when the upstream affordance query already
   guarantees co-location is duplication, not defense in depth, in
   this codebase's discipline (FND-26). The audit must distinguish.
3. **Locality of motion (FND-7)**: the principle is that physical
   interaction requires co-location or explicit range. Direct-root
   synthesis emitting a remote target violates locality silently;
   downstream revalidation may catch the violation, but the
   planner's intermediate state is invalid. The audit's purpose is
   to detect this class of violation across all arms.

## Verification Layers

1. **Per-arm audit table** -> a matrix listing every
   `synthesized_root_candidate_targets` arm, its action's
   `TargetSpec`, whether the arm has an explicit locality gate,
   and whether the upstream affordance query enforces co-location.
2. **Trade arm coverage** -> new focused test asserting the Trade
   arm refuses to synthesize when evidence entity is not at the
   actor's place (or asserts upstream filtering prevents the call
   altogether — pick the verification surface that matches the
   actual canonical guarantee).
3. **Harvest arm coverage** -> companion focused test on Harvest.
4. **Existing Wash and Investigate tests** -> continue to pass.

## What to Change

### 1. Read every arm of `synthesized_root_candidate_targets`

Walk `crates/worldwake-ai/src/goal_model.rs:2373+`. List each arm,
its `PlannerOpKind`, the goal kinds it synthesizes for, and the
`TargetSpec` it requires.

### 2. Cross-reference with action registrations

For each arm, find the action registration in
`crates/worldwake-systems/` that declares the precondition:
`grep -rn 'TargetSpec::EntityAtActorPlace\|TargetSpec::ActorPlace' crates/worldwake-systems/`.

### 3. Determine upstream guarantee for each arm

For each arm whose action has an `EntityAtActorPlace` or `ActorPlace`
precondition, walk upstream from the synthesizer call site to
`get_affordances`
(`crates/worldwake-ai/src/affordance_query.rs`) and determine
whether the affordance query filters on co-location. Document the
finding per arm.

### 4. Build the audit table

Write findings to `docs/audits/2026-05-01-direct-root-synthesis-locality.md`:

| Arm | Goal kinds | TargetSpec | Synthesizer locality gate | Upstream filter | Verdict |
|---|---|---|---|---|---|
| Wash | Wash | `EntityAtActorPlace` | yes (post-CIREM-003) | (verify) | covered |
| Investigate | InvestigateViolation | `ActorPlace` | yes | (verify) | covered |
| Trade | AcquireCommodity etc. | `EntityAtActorPlace` | no | (verify) | (decide) |
| Harvest | AcquireCommodity etc. | `EntityAtActorPlace` | no | (verify) | (decide) |
| ... | ... | ... | ... | ... | ... |

Verdict per row: `covered` (synthesizer or upstream enforces
locality), `gap` (neither enforces; add synthesizer gate), or
`redundant` (both enforce; pick canonical and note the other in
comment).

### 5. Land per-arm changes

For each `gap` row: add a synthesizer-side locality gate matching
the Wash arm's pattern, plus a focused test asserting refusal on
non-local evidence.

For each `redundant` row: pick one canonical site and add a
comment at the other naming the canonical guarantee.

For `covered` rows: no code change.

## Files to Touch

- `docs/audits/2026-05-01-direct-root-synthesis-locality.md` (new)
- `crates/worldwake-ai/src/goal_model.rs` (modify — only for `gap`
  rows; possibly no change if all rows are `covered`)

## Out of Scope

- Refactoring `synthesized_root_candidate_targets` into a generic
  pattern. The arms are diverse and per-arm gates are the right
  granularity.
- Auditing actions whose `TargetSpec` does not include co-location
  (e.g. `ActorOnly`, free-form actions).
- Changing the affordance query's filter semantics.

## Acceptance Criteria

### Tests That Must Pass

1. Existing `grounded_goal_synthesizes_*` tests (Trade, Harvest,
   Wash, etc.) continue to pass.
2. (For each `gap` row) New `synthesized_root_*_refuses_non_local_evidence`
   focused test passes.
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh` only required if engine changes land.

### Invariants

1. **Direct-root synthesis respects locality**: every arm whose
   action requires `EntityAtActorPlace` or `ActorPlace` either
   gates synthesis on `actor_place ∈ evidence_places` (or
   equivalent) or has a documented upstream guarantee at the
   affordance-query layer.
2. **No silent non-local direct roots**: there is no arm where
   stale-belief evidence can drive synthesis of a remote target
   for a co-location-required action.

## Test Plan

### New/Modified Tests

1. (Per `gap` row) `crates/worldwake-ai/src/goal_model.rs::tests::synthesized_root_<arm>_refuses_non_local_evidence`
   — new
2. None — documentation-only ticket if all rows audit as `covered`.

### Commands

1. `grep -rn 'TargetSpec::EntityAtActorPlace\|TargetSpec::ActorPlace' crates/worldwake-systems/`
   — sanity check the audit walked every action.
2. `cargo test -p worldwake-ai goal_model::tests::synthesized`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh` (only if engine changes land)
