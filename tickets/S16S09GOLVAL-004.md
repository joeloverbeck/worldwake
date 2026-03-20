# S16S09GOLVAL-004: Golden — Combat-to-Non-Combat Domain Crossing

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S16S09GOLVAL-001 (shared helpers in harness)

## Problem

No existing golden test proves that an agent in a defend stance with **no living threat** fluidly transitions through multiple non-combat domains (needs -> care) after stance expiry. Without this coverage, a "combat lock-in" regression could go undetected — the defend->commit->replan cycle might fail to cleanly hand off to non-combat goal families, violating Principle 1 (maximal emergence) and Principle 24 (systems interact only through state).

## Assumption Reassessment (2026-03-20)

1. No existing golden test covers defend expiry with zero threats + multi-domain transition. Checked via `grep -r "domain_cross\|no.*threat\|combat.*noncombat\|combat_to_" crates/worldwake-ai/tests/` — no matches.
2. The defend->commit->replan cycle releases the agent back to `AgentTickDriver` which calls `rank_candidates()`. With no threat, `ReduceDanger` candidates have zero pressure, so needs/care candidates dominate. The specific ordering (needs first vs care first) depends on `UtilityProfile` weights applied to concrete hunger vs wound severity.
3. This is a golden E2E test. Verification layers: decision trace for goal selection sequence, action trace for lifecycle, authoritative world state for multi-domain state deltas.
4. Ordering contract: The first post-defend action should address the highest-pressure need. With `hunger_weight: pm(700)` at `hunger: pm(700)` vs `pain_weight: pm(500)` at `wound_severity: ~300`, hunger pressure (700*700/1000 = 490) should exceed pain pressure (500*300/1000 = 150). The ordering is driven by motive score within the same priority class (physiological needs). After eating, the next highest pressure becomes wound care.
5. Not removing/weakening any heuristic.
6. Not a stale-request ticket.
7. Not a political ticket.
8. No ControlSource manipulation.
9. **Scenario isolation**: NO hostile agents present — the Fighter was seeded into defend preemptively. This intentionally eliminates all combat affordances from the start, isolating the test to the defend->needs->care chain. The Fighter has Bread x2 (enough to eat) and Medicine x1 (enough to heal). `no_recovery_combat_profile()` prevents wound auto-recovery, ensuring the heal branch is reachable.
10. No mismatches found.

## Architecture Check

1. A single-agent scenario with no threat is the cleanest way to test domain crossing in isolation. Multi-agent scenarios (like S16S09GOLVAL-003) test divergence; this test specifically proves the *absence* of combat lock-in when no combat affordance exists.
2. No backwards-compatibility shims.

## Verification Layers

1. Defend commits after ~5 ticks -> action trace (`ActionTraceKind::Committed` for "defend")
2. No `ReduceDanger` goal selected post-defend -> decision trace (inspect `Planning` outcomes, verify goal kind is not `ReduceDanger`)
3. First post-defend action is eat (hunger dominates) -> authoritative world state (hunger decreases before wound_load decreases) + decision trace (first `Planning` outcome shows `ConsumeCommodity`)
4. Second post-defend action addresses wound care -> authoritative world state (wound_load eventually decreases) + decision trace (later `Planning` outcome shows `TreatWounds`)
5. Agent transitions through at least two domain families -> action trace (committed action names include at least one needs-family action and one care-family action)
6. Deterministic replay -> world hash + event log hash

## What to Change

### 1. Add `golden_combat_to_noncombat_domain_crossing` test to `golden_combat.rs`

Setup:
- `GoldenHarness::new(Seed([52; 32]))`
- Single agent (Fighter) at VillageSquare:
  - `no_recovery_combat_profile()` base with `defend_stance_ticks: nz(5)`.
  - `UtilityProfile { hunger_weight: pm(700), pain_weight: pm(500), .. }`.
  - High hunger pm(700). Pre-seeded wound (severity ~300 via `stable_wound_list`).
  - Give Bread x2, Medicine x1.
- NO hostile agents.
- Seed Fighter into active defend action (duration 5) with `CombatStance::Defending`.
- Seed local beliefs.
- Enable decision tracing and action tracing.

Observation loop (up to 60 ticks):
- Track defend commit tick, post-defend goal sequence, hunger deltas, wound_load deltas.

Assertions:
1. Defend commits after ~5 ticks.
2. After defend commit, Fighter does NOT select `ReduceDanger`.
3. Fighter's first post-defend action addresses hunger (hunger decreases before wound_load decreases).
4. Fighter subsequently addresses wound care (wound_load decreases).
5. Fighter transitions through at least two domain families (needs + care).
6. Deterministic replay.

### 2. Add `golden_combat_to_noncombat_domain_crossing_replays_deterministically` companion

Standard two-run hash comparison.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add two tests)

## Out of Scope

- Any engine/production code changes
- Modifying existing tests
- Changes to the golden harness module
- Testing with living threats (that's S16S09GOLVAL-002)
- Testing multi-agent divergence (that's S16S09GOLVAL-003)
- Asserting specific tick numbers for defend commit or subsequent actions

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_combat_to_noncombat_domain_crossing` — new test passes
2. `cargo test -p worldwake-ai golden_combat_to_noncombat_domain_crossing_replays_deterministically` — replay passes
3. `cargo test -p worldwake-ai` — full suite, no regressions

### Invariants

1. Append-only event log is never mutated
2. Conservation invariants hold
3. Determinism: identical seed produces identical hashes
4. No `ReduceDanger` candidate is generated when no hostile agent is alive and co-located
5. The ranking pipeline selects goals purely by pressure magnitude — no domain-specific priority overrides

## Test Plan

### New/Modified Tests

1. `golden_combat_to_noncombat_domain_crossing` in `crates/worldwake-ai/tests/golden_combat.rs` — proves Principle 1 (maximal emergence) and Principle 24 (system interaction through state): defend->needs->care chain crosses three domains without orchestration
2. `golden_combat_to_noncombat_domain_crossing_replays_deterministically` in `crates/worldwake-ai/tests/golden_combat.rs` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai golden_combat_to_noncombat_domain_crossing`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
