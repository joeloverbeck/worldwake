# S16S09GOLVAL-003: Golden — Multi-Agent Divergent Re-Evaluation (Principle 20)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S16S09GOLVAL-001 (shared helpers in harness)

## Problem

No existing golden test proves that two agents with different profile parameters produce **divergent** post-combat behavior from the same world state. Without this coverage, a regression could silently break Principle 20 (Agent Diversity) — the ranking pipeline could be collapsed to a single path without detection.

## Assumption Reassessment (2026-03-20)

1. No existing golden test covers multi-agent divergent re-evaluation after defend expiry. Checked via `grep -r "divergent\|defender_a\|defender_b\|multi_agent.*diverge" crates/worldwake-ai/tests/` — no matches in golden files.
2. The ranking pipeline that produces divergent behavior is `rank_candidates()` in `crates/worldwake-ai/src/ranking.rs`, which scores goals by `UtilityProfile` weights applied to concrete need/wound state. `pain_weight` and `hunger_weight` are the two relevant profile fields.
3. This is a golden E2E test. Verification layers: decision trace for goal selection divergence, action trace for lifecycle ordering, authoritative world state for durable outcomes (hunger decrease vs wound_load decrease).
4. Ordering contract: DefenderA's defend commits before DefenderB's (action lifecycle ordering, driven by `defend_stance_ticks: 3` vs `8`). The post-defend goal divergence is driven by `UtilityProfile` weight differences (pain_weight=800 vs hunger_weight=800), not by priority class or suppression. The compared branches are symmetric in substrate — both use the same `rank_candidates()` path with different weight vectors.
5. Not removing/weakening any heuristic.
6. Not a stale-request ticket.
7. Not a political ticket.
8. No ControlSource manipulation.
9. **Scenario isolation**: The Doomed Threat dies from deprivation, removing combat affordances entirely. Both defenders have identical wound severity (~200) and identical hunger (pm(500)), so the only divergence driver is `UtilityProfile` weights. Both have Bread x1 and Medicine x1, so neither branch is artificially constrained. `no_recovery_combat_profile()` prevents wound auto-recovery.
10. No mismatches found.

## Architecture Check

1. Testing profile-driven divergence at the golden level is the correct surface because the behavior emerges from the interaction of multiple subsystems (ranking, planning, execution). A focused unit test of `rank_candidates()` with different weights already exists conceptually but cannot prove the full execution chain.
2. No backwards-compatibility shims.

## Verification Layers

1. DefenderA defend commits before DefenderB defend -> action trace (compare `Committed` ticks for "defend" per agent)
2. Doomed Threat dies -> authoritative world state (`agent_is_dead`)
3. DefenderA prioritizes heal (pain_weight=800) -> decision trace (first post-defend `Planning` outcome shows `TreatWounds` or care-family goal) + authoritative world state (wound_load decreases before hunger decreases)
4. DefenderB prioritizes eat (hunger_weight=800) -> decision trace (first post-defend `Planning` outcome shows `ConsumeCommodity`) + authoritative world state (hunger decreases before wound_load decreases)
5. Both agents eventually address both needs -> authoritative world state (both hunger and wound_load decrease for each agent by end)
6. Deterministic replay -> world hash + event log hash

## What to Change

### 1. Add `golden_multi_agent_divergent_reevaluation` test to `golden_combat.rs`

Setup:
- `GoldenHarness::new(Seed([51; 32]))`
- Three agents at VillageSquare:
  - **DefenderA**: `no_recovery_combat_profile()` base with `defend_stance_ticks: nz(3)`. `UtilityProfile { pain_weight: pm(800), hunger_weight: pm(300), .. }`. Pre-seeded wound (severity ~200 via `stable_wound_list`). Hunger pm(500). Give Medicine x1, Bread x1.
  - **DefenderB**: `no_recovery_combat_profile()` base with `defend_stance_ticks: nz(8)`. `UtilityProfile { pain_weight: pm(300), hunger_weight: pm(800), .. }`. Pre-seeded wound (severity ~200). Hunger pm(500). Give Medicine x1, Bread x1.
  - **Doomed Threat**: near-lethal deprivation (same pattern as S16S09GOLVAL-002).
- Add hostility: Threat -> DefenderA, Threat -> DefenderB.
- Seed both defenders into active defend actions with respective durations.
- Seed local beliefs for all agents.
- Enable decision tracing and action tracing.

Observation loop (up to 80 ticks):
- Track per-agent defend commit ticks, post-defend goal kinds, state deltas (hunger and wound_load snapshots).

Assertions:
1. DefenderA's defend commits before DefenderB's.
2. Doomed Threat dies.
3. DefenderA (pain_weight=800): wound_load decreases before hunger decreases.
4. DefenderB (hunger_weight=800): hunger decreases before wound_load decreases.
5. Both defenders eventually address both needs.
6. Deterministic replay.

### 2. Add `golden_multi_agent_divergent_reevaluation_replays_deterministically` companion

Standard two-run hash comparison.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add two tests)

## Out of Scope

- Any engine/production code changes
- Modifying existing tests
- Changes to the golden harness module
- Testing more than two divergent agents
- Asserting specific tick numbers for defend commits (only relative ordering)
- Testing domain crossing (that's S16S09GOLVAL-004)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_multi_agent_divergent_reevaluation` — new test passes
2. `cargo test -p worldwake-ai golden_multi_agent_divergent_reevaluation_replays_deterministically` — replay passes
3. `cargo test -p worldwake-ai` — full suite, no regressions

### Invariants

1. Append-only event log is never mutated
2. Conservation invariants hold
3. Determinism: identical seed produces identical hashes
4. Both agents use the same `rank_candidates()` pipeline — divergence is profile-driven, not code-path-driven

## Test Plan

### New/Modified Tests

1. `golden_multi_agent_divergent_reevaluation` in `crates/worldwake-ai/tests/golden_combat.rs` — proves Principle 20: same world state + different profiles = divergent behavior
2. `golden_multi_agent_divergent_reevaluation_replays_deterministically` in `crates/worldwake-ai/tests/golden_combat.rs` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai golden_multi_agent_divergent_reevaluation`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
