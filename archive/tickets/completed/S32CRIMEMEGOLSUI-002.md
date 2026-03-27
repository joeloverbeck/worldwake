# S32CRIMEMEGOLSUI-002: Scenario 42 — Witness Deterrence Suppresses Theft Candidate

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: E17 (crime/theft/justice), E14 (perception), S32CRIMEMEGOLSUI-001 (implementation order)

## Problem

The witness deterrence path in `emit_theft_candidates()` is missing golden E2E coverage. When `witness_risk_penalty * witness_count >= theft_motive_weight`, the function early-returns with zero `StealItem` candidates. This is the P10 physical dampener that prevents crime-escalation loops: the deterrent is the concrete presence of other agents, not a numeric cap. Focused candidate-generation coverage already proves the arithmetic in isolation; the missing proof is the cross-system golden that shows the thief observes witnesses, declines theft, and follows a lawful self-care branch instead of idling.

## Assumption Reassessment (2026-03-27)

1. **Shared abstraction boundary under audit**: this ticket is about the information path from authoritative co-location + perception into AI theft candidate generation and then into needs-driven selection. The relevant live symbols are `emit_theft_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs:1923` and `theft_motive()` in `crates/worldwake-ai/src/ranking.rs:582`.
2. **`emit_theft_candidates()`** still uses the witness gate described by the spec: it counts co-located living agents in `locally_observed_entities_at()` and returns early when `theft_motive_weight <= witness_risk_penalty * witness_count`.
3. **The arithmetic is not completely untested today**: `crates/worldwake-ai/src/candidate_generation.rs::theft_candidate_respects_preconditions_and_witness_gate` already proves the witness gate at focused unit-test level. The real gap is missing golden coverage, not missing test coverage in general. The ticket scope must say that explicitly.
4. **`TheftDispositionProfile`** still has the expected fields `theft_motive_weight: Permille`, `witness_risk_penalty: Permille`, and `steal_duration_ticks: NonZeroU32` in `crates/worldwake-core/src/crime.rs`.
5. **The live self-care goal surface differs from the ticket wording**: `ConsumeCommodity` does not exist. The correct live goals are `ConsumeOwnedCommodity { commodity }` and `AcquireCommodity { commodity, purpose: CommodityPurpose::SelfConsume }` in `crates/worldwake-core/src/goal.rs`.
6. **Golden inventory check**: `cargo test -p worldwake-ai --test golden_emergent -- --list` confirms that Scenarios 37, 38, 40, and 41 exist, and there is no Scenario 42 test yet. The gap is real.
7. **Current crime goldens do not cover deterrence**: Scenarios 37, 38, and 41 all configure theft to succeed (`witness_risk_penalty=pm(0)` or effectively non-blocking). No golden currently proves candidate suppression from witness presence.
8. **Scenario-isolation correction**: the original ticket’s “reachable food source” setup is broader than needed and introduces extra lawful branches. The cleaner golden is to give the thief a locally controlled food lot so the post-deterrence branch is the narrow, deterministic `ConsumeOwnedCommodity` self-care path. That still proves “not idle” without introducing avoidable acquisition/travel competition.
9. **Witness stability remains necessary**: the witness agents should stay `ControlSource::Human` so they do not move away or create unrelated plan noise that changes the observed witness count mid-scenario.
10. **Perception requirement remains real**: the thief and witness agents need `PerceptionProfile` so the thief can lawfully observe the co-located witness set through the live perception/belief pipeline before the decision-trace assertions.
11. **Architecture note outside this ticket’s scope**: the same witness-penalty arithmetic currently exists in both `emit_theft_candidates()` and `theft_motive()`. That duplication is worth a follow-up cleanup ticket if it starts drifting, but this golden-only ticket should not refactor production code.

## Architecture Check

1. The beneficial change here is the golden itself, not an engine rewrite. The current architecture already models deterrence cleanly as local observation feeding candidate generation. What is missing is a robust cross-system proof that this physical dampener survives full-tick execution.
2. The cleanest scenario is narrower than the original ticket proposed: use an immediately consumable owned-food fallback instead of a broader acquire/travel branch. That keeps the golden focused on the witness-deterrence contract instead of entangling it with trade, production, or pathing behavior that this ticket does not need to prove.
3. This test proves a negative through positive evidence: decision trace proves no `StealItem` candidate exists while witnesses are present, and authoritative/action consequences prove the thief followed a lawful alternative branch. Relying on unchanged world state alone would be too weak because it cannot distinguish “candidate suppressed” from “candidate existed but later failed.”
4. No backwards-compatibility aliasing or shims are introduced.

## Verification Layers

1. Witness deterrence suppresses theft candidate generation while co-location holds -> decision trace (`DecisionOutcome::Planning`, inspect `candidates.generated` for absence of `GoalKind::StealItem`)
2. The thief does not idle after suppression -> decision trace (`selection.selected`) proves a self-care goal is selected
3. The alternative self-care branch actually executes -> authoritative `HomeostaticNeeds` read, optionally corroborated by action trace `eat` commit if needed
4. The theft target remains untouched -> authoritative world state (`World::effective_place`, `World::owner_of`, commodity quantity)
5. Conservation still holds -> `verify_live_lot_conservation`
6. Determinism -> replay companion `(StateHash, StateHash)` equality

## What to Change

### 1. Add `run_witness_deterrence_suppresses_theft_candidate` function to `golden_emergent.rs`

Setup:
- Thief at `VillageSquare`: `TheftDispositionProfile { theft_motive_weight: pm(400), witness_risk_penalty: pm(150), steal_duration_ticks: nz(2) }`, `HomeostaticNeeds` with hunger above the low threshold, `PerceptionProfile`, and a locally controlled bread or apple lot so the lawful fallback is `ConsumeOwnedCommodity`
- 3 witness agents at `VillageSquare`: all with `PerceptionProfile`, all switched to `ControlSource::Human`, all otherwise inert
- Theft target: a loose ground item lot at `VillageSquare`, owned by a separate human-controlled owner, not possessed, not containerized, and within thief carry capacity so it would be theft-eligible without witnesses
- Standard prototype topology only; no harness changes

Tick loop:
- Step until the thief has had at least one full planning tick with witnesses stably co-located
- On every traced planning tick in the scenario, assert that `GoalKind::StealItem` never appears in `candidates.generated`
- Assert that at least one selected goal is `ConsumeOwnedCommodity { commodity: Bread | Apple }`
- Assert hunger decreases before scenario end
- Assert the theft target stays at the original place with the original owner and quantity

### 2. Add test functions

- `golden_witness_deterrence_suppresses_theft_candidate` — runs the scenario once
- `golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically` — runs twice with same seed, asserts hash equality

### 3. Add scenario metadata comment block

```
// Scenario 42: Witness Deterrence Suppresses Theft Candidate
// Systems: AI, Perception, Needs
// GoalKinds: ConsumeOwnedCommodity (NOT StealItem)
// ActionDomains: Needs
// Places: VillageSquare
// Principles: 1, 10, 24
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)

## Out of Scope

- **No engine changes** — no modifications to `candidate_generation.rs` or any source crate
- **No harness changes** — no new helpers in `golden_harness/mod.rs`
- **No golden docs update** — that is S32CRIMEMEGOLSUI-004
- **Scenario 41 and 43** — separate tickets
- **Threshold-boundary arithmetic coverage** (e.g. the 2-witness flip case) — already belongs at focused-test level, not this golden
- **Witness agent AI behavior** — witnesses are human-controlled; their decision pipeline is not under test
- **Refactoring duplicated theft-motive arithmetic** between candidate generation and ranking — note for follow-up, not this ticket

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate` — targeted main test
2. `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically` — targeted replay test
2. `cargo test -p worldwake-ai` — full AI crate suite (all existing goldens unchanged)
3. `cargo test --workspace` — no regressions
4. `cargo clippy --workspace --all-targets -- -D warnings` — clean

### Invariants

1. **No theft candidates**: at every tick where Thief is co-located with 3+ witnesses, `candidates.generated` contains zero `GoalKind::StealItem` entries
2. **Alternative goal executes**: Thief selects and follows a self-care goal from the live surface (`ConsumeOwnedCommodity`, not an obsolete `ConsumeCommodity` variant)
3. **Item untouched**: stealable item lot remains at original place with original owner, quantity unchanged
4. **Hunger satisfied**: Thief's `HomeostaticNeeds.hunger` value is lower at end than at start
5. **Determinism**: replay companion produces identical `(world_hash, event_log_hash)` for same seed
6. **Conservation**: `verify_live_lot_conservation` passes
7. **All existing golden tests remain unchanged**

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_witness_deterrence_suppresses_theft_candidate` — proves witness presence suppresses theft candidates
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate`
2. `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - Updated the ticket assumptions to match the live architecture and test surface before implementation.
  - Added `golden_witness_deterrence_suppresses_theft_candidate` and its deterministic replay companion to `crates/worldwake-ai/tests/golden_emergent.rs`.
  - Implemented the golden with a narrower, more robust isolation strategy than originally drafted: the thief now falls back to a locally controlled `ConsumeOwnedCommodity` branch instead of a broader acquire/travel branch.
- Deviations from original plan:
  - Corrected the ticket from “zero coverage” to “missing golden coverage”; focused coverage already existed in `candidate_generation.rs`.
  - Corrected the live goal naming from obsolete `ConsumeCommodity` wording to `ConsumeOwnedCommodity` / `AcquireCommodity(SelfConsume)`.
  - Kept the implementation golden-only and did not refactor the duplicated theft-motive arithmetic in production code.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate` passed.
  - `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
