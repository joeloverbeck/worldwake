# S16S09GOLVAL-002: Golden — Defend Re-Evaluation Under Changed Conditions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S16S09GOLVAL-001 (shared helpers in harness)

## Problem

The only existing defend golden (`golden_defend_replans_after_finite_stance_expires`) proves the mechanical lifecycle: defend commits, agent replans, agent does *something*. It does NOT prove that re-evaluation leads to a **different goal** when the world changed during the stance. Without this coverage, a regression could silently break Principle 19 (intentions are revisable commitments) — the agent might blindly re-enter combat even when the threat is dead.

## Assumption Reassessment (2026-03-20)

1. `golden_defend_replans_after_finite_stance_expires` exists at `crates/worldwake-ai/tests/golden_combat.rs:973`. It asserts: (a) seeded defend commits, (b) defender re-enters decision pipeline, (c) defender starts/commits another action. It does NOT assert the *kind* of post-defend goal. Verified by reading lines 973-1082.
2. No existing golden test covers "threat dies during defend stance → agent switches to non-combat goal." Checked via `grep -r "changed_conditions\|threat.*dies\|doomed.*attacker" crates/worldwake-ai/tests/` — no matches.
3. This is a golden E2E test. The verification layer is: decision trace for goal selection + action trace for lifecycle + authoritative world state for durable outcomes.
4. Ordering contract: The defend commit must precede the attacker's death observation (action lifecycle ordering). The post-defend goal selection must follow both events (decision trace ordering). The compared branches are asymmetric: `ReduceDanger` would require a living threat (pressure-derived), while `ConsumeCommodity`/`TreatWounds` are needs-derived. The divergence is driven by pressure absence (no threat) collapsing `ReduceDanger` priority to zero.
5. Not removing/weakening any heuristic.
6. Not a stale-request ticket.
7. Not a political ticket.
8. No ControlSource manipulation.
9. **Scenario isolation**: The Doomed Attacker is designed to die from deprivation within ~5 ticks, removing the combat affordance entirely. This intentionally eliminates the `ReduceDanger` branch post-death. The Defender has both Bread (for eat) and Medicine (for heal) available, so the post-defend branch is not artificially constrained — the agent genuinely chooses based on pressure ranking. The `no_recovery_combat_profile()` pattern (natural_recovery_rate=0) ensures wounds persist until healed, preventing a race where natural recovery removes the heal branch.
10. No mismatches found.

## Architecture Check

1. This test follows the established pattern in `golden_defend_replans_after_finite_stance_expires` (same defend-seeding technique, same trace queries) but extends it with behavioral assertions. The approach is cleaner than adding behavioral assertions to the existing test because the existing test has a different seed and setup (living attacker with coins), and modifying it would change its purpose.
2. No backwards-compatibility shims.

## Verification Layers

1. Defend action commits within ~5 ticks -> action trace (`ActionTraceKind::Committed` for "defend")
2. Doomed Attacker dies from deprivation -> authoritative world state (`agent_is_dead(attacker)`)
3. Post-defend goal is NOT `ReduceDanger` -> decision trace (`DecisionOutcome::Planning`, inspect `selected_plan` goal kind)
4. Defender eventually takes non-combat action (eat or heal) -> authoritative world state (hunger decreases or wound_load decreases)
5. Deterministic replay -> world hash + event log hash equality across two runs

## What to Change

### 1. Add `golden_defend_changed_conditions` test to `golden_combat.rs`

Setup:
- `GoldenHarness::new(Seed([50; 32]))`
- Defender at VillageSquare: `no_recovery_combat_profile()` with `defend_stance_ticks` overridden to `nz(3)`. Pre-seeded clotted wound (severity ~120 via `stable_wound_list`). Hunger at ~pm(300). Give Bread x1, Medicine x1.
- Doomed Attacker at VillageSquare: near-lethal deprivation (hunger pm(950)), `DeprivationExposure` with `hunger_critical_ticks: 2`, high hunger tick rate (pm(50)), low `wound_capacity` (pm(200)), pre-seeded starvation wound (severity ~150).
- Add hostility: Attacker -> Defender.
- Seed Defender into active defend action (duration 3) with `CombatStance::Defending`.
- Seed local beliefs for both agents.
- Enable decision tracing and action tracing.

Observation loop (up to 60 ticks):
- Track attacker death, defend commit, post-defend goal kind, state deltas.

Assertions:
1. Seeded defend commits within first ~5 ticks.
2. Doomed Attacker dies.
3. After defend commit + attacker death, Defender's next selected goal is NOT `ReduceDanger`.
4. Defender eventually takes a non-combat action (hunger or wound_load decreases).

### 2. Add `golden_defend_changed_conditions_replays_deterministically` companion

Standard two-run hash comparison using extracted scenario builder function.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add two tests)

## Out of Scope

- Any engine/production code changes
- Modifying `golden_defend_replans_after_finite_stance_expires` or any other existing test
- Changes to the golden harness module (helpers come from S16S09GOLVAL-001)
- Asserting the *specific* post-defend goal (eat vs heal) — only that it's NOT `ReduceDanger`
- Testing multi-agent divergence (that's S16S09GOLVAL-003)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_defend_changed_conditions` — new test passes
2. `cargo test -p worldwake-ai golden_defend_changed_conditions_replays_deterministically` — replay passes
3. `cargo test -p worldwake-ai` — full suite, no regressions

### Invariants

1. Append-only event log is never mutated
2. Conservation invariants hold (no items created/destroyed outside explicit actions)
3. Determinism: identical seed produces identical world hash and event log hash
4. The existing `golden_defend_replans_after_finite_stance_expires` test still passes unchanged

## Test Plan

### New/Modified Tests

1. `golden_defend_changed_conditions` in `crates/worldwake-ai/tests/golden_combat.rs` — proves Principle 19: agent revises intention when threat dies during defend stance
2. `golden_defend_changed_conditions_replays_deterministically` in `crates/worldwake-ai/tests/golden_combat.rs` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai golden_defend_changed_conditions`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
