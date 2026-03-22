# S19INSRECCON-004: Scenario 34 — Knowledge Asymmetry Race — Informed Agent Outpaces Consulting Agent

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: S19INSRECCON-001 (harness helpers); S19INSRECCON-002 (establishes ConsultRecord golden pattern)

## Problem

No golden test proves that institutional knowledge creates real competitive advantage through consultation duration cost. Existing Scenario 28 races two agents traveling from different locations — the asymmetry is geographic distance. Scenario 34 races two co-located agents where the asymmetry is knowledge state: one agent already knows the vacancy (Certain belief, skips ConsultRecord), while the other must consult the record first (Unknown belief, ConsultRecord takes ticks). The informed agent acts immediately and wins the office because the consultation delay lawfully exceeds the office's succession window.

This proves Principle 14 (Unknown vs Certain creates real behavioral divergence), Principle 20 (knowledge diversity → different competitive outcomes), and Principle 8 (consultation has real duration that costs competitive time).

## Assumption Reassessment (2026-03-22)

1. Unit test `search_political_goal_skips_consult_record_when_vacancy_belief_is_already_certain` at `search.rs:5448` confirms Agent A (Certain) will plan DeclareSupport directly. Unit test at `search.rs:5330` confirms Agent B (Unknown) will plan ConsultRecord→DeclareSupport. The golden test validates that this planning divergence produces a competitive outcome divergence end-to-end.
2. Live consultation duration comes from `consultation_ticks * consultation_speed_factor / 1000`, floored by integer division and clamped to at least 1 tick. With the harness default `consultation_ticks: 4` and `consultation_speed_factor: pm(500)`, ConsultRecord takes 2 ticks, not 8. That is not enough by itself to make the informed agent win before support succession resolves.
3. The `GoalKind` under test is `ClaimOffice` for both agents. Agent A's plan: `DeclareSupport` (1 tick). Agent B's plan: `ConsultRecord` → `DeclareSupport`. To preserve the intended invariant under the live engine, the scenario must explicitly raise the consulted record's `consultation_ticks` so B cannot finish consultation before the 5-tick support succession window closes.
4. This is a golden E2E ticket. Full action registries required (provided by `GoldenHarness`).
5. Ordering: the divergence depends on **consultation duration relative to the succession timer** (Agent B spends ticks consulting while Agent A acts immediately), not on rank-weight asymmetry. Both agents have identical `enterprise_weight`, so the relevant planning divergence is knowledge state plus the legally configured record duration.
8. Closure boundary: Agent A's `declare_support` commits (action trace) while Agent B is still executing `consult_record`. Succession resolves in A's favor (authoritative relation: `world.office_holder(office) == Some(agent_a)`). AI-layer: A's plan lacks ConsultRecord; B's plan includes ConsultRecord. Authoritative-layer: `office_holder()` returns A.
10. Isolation: both agents sated, no competing needs. Both at VillageSquare, no travel needed. Both have identical enterprise_weight. The decisive asymmetry is institutional knowledge state plus the authoritative consultation duration configured on the record.

## Architecture Check

1. Follows the established multi-agent golden pattern from Scenario 12 (competing claims with supporter). Two agents with different knowledge states instead of different loyalty/social weights.
2. The clean architecture is to make the decisive delay explicit in authoritative record state by raising `RecordData.consultation_ticks` for this scenario. That is better than pretending the existing `pm(500)` setting is slower or relying on nonexistent support-duration accumulation.
3. No backward-compatibility shims introduced.

## Verification Layers

1. Agent A's `declare_support` commits before Agent B finishes `consult_record` → action trace ordering
2. Agent A is office holder, not Agent B → authoritative world state (`world.office_holder(office) == Some(agent_a)`)
3. Agent A's plan has no ConsultRecord step; Agent B's plan includes ConsultRecord → decision trace
4. Deterministic → replay companion
4. The competitive outcome (who holds office) is asserted at the authoritative layer. The action trace ordering (A acts before B finishes) is supporting evidence for the causal explanation, not the primary contract. The primary contract is: "informed agent wins office."

## What to Change

### 1. Add `build_knowledge_asymmetry_race_scenario()` in `golden_offices.rs`

Setup function creating:
- Two sated agents at `VILLAGE_SQUARE`, both with `enterprise_weighted_utility(pm(800))` and perception profiles.
- Vacant office at `VILLAGE_SQUARE` via `seed_office()`.
- Vacancy entry in the OfficeRegister via `seed_office_vacancy_entry()`.
- Override the consulted OfficeRegister's `consultation_ticks` to a value that exceeds the `succession_period_ticks: 5` window under `consultation_speed_factor: pm(500)`. A concrete live-safe choice is `consultation_ticks: 12`, which yields a 6-tick consult.
- Agent A ("Informed"): entity beliefs about office AND `seed_office_holder_belief(agent_a, office, None, ...)` — Certain(None) institutional belief. Planner skips ConsultRecord.
- Agent B ("Uninformed"): entity beliefs about office and record, but **no** `seed_office_holder_belief()` — Unknown institutional belief. Planner inserts ConsultRecord.

### 2. Add `run_knowledge_asymmetry_race()` function

Runs 30 ticks. Asserts:
1. Action trace: Agent A's `declare_support` commits before Agent B finishes `consult_record`.
2. Authoritative state: Agent A is office holder (not B).
3. Decision trace: A's plan lacks ConsultRecord; B's plan includes ConsultRecord.
4. Returns `(StateHash, StateHash)` for replay.

### 3. Add primary test `golden_knowledge_asymmetry_race_informed_wins_office`

### 4. Add replay companion `golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- No engine code changes
- No changes to `golden_harness/mod.rs` (handled by S19INSRECCON-001)
- No changes to existing golden scenarios
- No remote record scenarios (handled by S19INSRECCON-003)
- No documentation updates (that's S19INSRECCON-005)
- Not testing what happens to Agent B after losing (StartFailed, replan, etc.) — that's a separate concern

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office` — new primary test
2. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically` — new replay test
3. `cargo test -p worldwake-ai` — full AI crate suite (no regressions)
4. `cargo test --workspace` — workspace suite
5. `cargo clippy --workspace --all-targets -- -D warnings` — lint

### Invariants

1. Agent A must not consult any record — it already knows the vacancy (Certain belief)
2. Agent B must consult the record before declaring support — it starts with Unknown belief
3. Agent A wins the office — the informed agent's time advantage from skipping consultation must be decisive under the scenario's explicit record-duration setup
4. Both agents have identical enterprise_weight, location, and office access. The decisive asymmetry is knowledge state plus the authoritative consultation duration configured on the record
5. Determinism: two runs with same seed produce identical state hashes
6. All existing golden tests continue to pass unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office` — proves knowledge asymmetry + explicit record consultation duration → competitive outcome
2. `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically` — deterministic replay

### Commands

1. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office` — targeted
2. `cargo test -p worldwake-ai` — AI crate
3. `cargo test --workspace` — full workspace
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint
