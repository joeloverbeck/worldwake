# FND02-003: Wire AcquireCommodity(Treatment) Goal Emission in Candidate Generation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate_generation.rs
**Deps**: Phase 2 complete

## Problem

`GoalKind::AcquireCommodity { commodity, purpose: CommodityPurpose::Treatment }` is defined in `crates/worldwake-core/src/goal.rs` and already ranked in `crates/worldwake-ai/src/ranking.rs` (priority derives from pain pressure), but no emission path exists in `crates/worldwake-ai/src/candidate_generation.rs`. As a result, treatment acquisition is reachable in planning/search but never proposed during candidate generation.

## Assumption Reassessment (2026-03-13)

1. `CommodityPurpose::Treatment` exists in `goal.rs` — confirmed (lines ~7-12).
2. `ranking.rs` handles `AcquireCommodity { purpose: Treatment }` with pain-pressure-derived priority — confirmed.
3. `candidate_generation.rs` has `emit_combat_candidates()` (line ~134) but no treatment/medicine-seeking emission — confirmed.
4. `WoundList` component exists on agents with `wounds: Vec<Wound>` — confirmed via `wounds.rs`.
5. Wound severity uses `Permille` — confirmed, no floats.
6. Treatment commodity mapping already exists in the item catalog: `CommodityKindSpec::treatment_profile` marks treatment-capable commodities, and the current catalog exposes that only for `CommodityKind::Medicine` — confirmed in `items.rs`.
7. `candidate_generation.rs` already has `local_wounded_targets()` and already uses treatment capability in `emit_produce_goals()` to emit `ProduceCommodity` for medicine recipes when a local wounded target exists — confirmed.
8. Existing regression coverage already proves `SellCommodity` remains deferred before S04, including the merchant-with-stock-and-demand case — confirmed; no additional ticket work is needed here.
9. Important architecture limitation: `AcquireCommodity { purpose: Treatment }` has no patient identity, and ranking currently derives urgency from the acting agent's pain pressure only.
10. Important code discrepancy discovered during implementation: the combat `Heal` action currently forbids self-targeting. Candidate generation therefore must not emit self-heal follow-up work or claim that this ticket delivers full self-care behavior.

## Architecture Check

1. The clean extension point is candidate generation, not search or ranking: search already supports `AcquireCommodity(Treatment)`, and production/heal candidate logic already treats treatment as a concrete commodity capability rather than a special-case subsystem.
2. The robust mapping is to derive treatment commodities from `CommodityKind::spec().treatment_profile`, not from recipe scanning and not from a new aliasing layer. That keeps treatment tied to concrete item definitions and scales cleanly if more treatment commodities are added later.
3. No backwards-compatibility shims — add the missing emission path and keep all existing goal kinds and planning semantics intact.
4. This ticket should improve local, satisfiable treatment acquisition for actionable care under current rules. It should not expand into a redesign of care architecture. If stronger third-party care behavior or self-care behavior is desired, that should be a separate follow-up that gives treatment acquisition a patient-aware goal and aligns `Heal` semantics accordingly.

## What to Change

### 1. Add wound-aware treatment candidate emission

Add `emit_treatment_candidates()` in `candidate_generation.rs`:

**Emission conditions** (all must be true):
- At least one local wounded other-agent target exists that can actually be healed under current action semantics.
- The agent does not already control a local treatment commodity lot and does not already hold that commodity quantity directly.
- The commodity is treatment-capable according to `CommodityKind::spec().treatment_profile`.
- There is a satisfiable acquisition path via the existing `acquisition_path_evidence()` helper.

**Priority note**: For non-self patients, the current architecture still ranks through the acting agent's own pain pressure, so this ticket only wires the missing candidate emission and removes unsatisfiable self-heal candidate generation. It does not redesign care prioritization.

**Evidence**: Include the wounded entities and the acquisition-path evidence.

### 2. Wire into candidate generation pipeline

Call `emit_treatment_candidates()` from `generate_candidates_with_travel_horizon()` in the combat/self-care portion of the pipeline, alongside the existing wound-aware emitters.

### 3. Reuse the existing treatment commodity model

Do not invent a new mapping mechanism. Reuse `CommodityKind::spec().treatment_profile` as the authoritative treatment capability signal.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `emit_treatment_candidates()` and call site)

## Out of Scope

- Do NOT implement healing action handlers — those exist in combat system.
- Do NOT modify `GoalKind` enum, `CommodityPurpose`, or `ranking.rs`.
- Do NOT modify wound tracking, needs system, or combat system.
- Do NOT add new `CommodityKind` variants — use existing ones.
- Do NOT touch `worldwake-core`, `worldwake-sim`, or `worldwake-systems` crates.
- Do NOT redesign treatment goals to carry patient identity in this ticket.
- Do NOT claim self-care is completed while `Heal` remains self-target forbidden.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: Agent with a local wounded other and no treatment commodity emits `AcquireCommodity { commodity, purpose: Treatment }` candidate.
2. Unit test: Unwounded agent with no wounded co-located entities does not emit treatment candidates.
3. Unit test: Agent already holding medicine does not emit treatment acquisition candidates.
4. Unit test: Self-wounds alone do not emit treatment acquisition or self-heal candidates under current semantics.
5. Existing suite: `cargo test -p worldwake-ai`
6. Full suite: `cargo test --workspace`

### Invariants

1. All evidence uses `BTreeSet<EntityId>` — no `HashSet`.
2. No `f32`/`f64` in any new code — use `Permille` and integer types.
3. Existing combat and needs candidate emission behavior unchanged.
4. Deterministic output — same inputs produce same candidates in same order.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — unit tests for treatment emission with local wounded others, suppression when medicine is already available, and suppression of self-only unsatisfiable care candidates.
2. `crates/worldwake-ai/tests/golden_care.rs` — healer acquires accessible ground medicine and successfully treats a wounded patient.

### Commands

1. `cargo test -p worldwake-ai -- treatment` — targeted treatment-related tests
2. `cargo test -p worldwake-ai` — full AI crate suite
3. `cargo clippy --workspace` — lint check
4. `cargo test --workspace` — full workspace suite

## Outcome

- Completed: 2026-03-13
- What actually changed:
  - Added `AcquireCommodity { commodity: Medicine, purpose: Treatment }` candidate emission for actionable local care cases where a wounded other-agent is present and medicine has a satisfiable acquisition path.
  - Reused `CommodityKind::spec().treatment_profile` as the treatment capability source instead of inventing a new mapping layer.
  - Tightened candidate generation so unsatisfiable self-heal follow-up work is no longer emitted under the current `Heal` action semantics.
  - Added targeted unit coverage and a golden care scenario where a healer acquires accessible ground medicine and treats a wounded patient.
- Deviations from original plan:
  - The original ticket assumed self-treatment was part of the current architecture. Implementation reassessment found that `Heal` is self-target forbidden, so the shipped scope was corrected to actionable third-party care support under current semantics.
  - No recipe-registry scan or new treatment-mapping abstraction was added because the item catalog already models treatment capability directly.
- Verification:
  - `cargo test -p worldwake-ai candidate_generation -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_care -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
