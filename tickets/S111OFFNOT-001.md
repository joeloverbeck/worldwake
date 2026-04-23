# S111OFFNOT-001: Author a truthful notice-posting proof seam for `survival-offices`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario-authorable threat-warning or office-vacancy posting substrate
**Deps**: `scenarios/survival-offices.ron`, `crates/worldwake-ai/tests/golden_survival_offices.rs`, row 11 in `docs/scenario-roadmap.md`

## Problem

`survival-offices` now truthfully proves the office / succession / force-claim half of roadmap row 11 under a 1440-tick survival contract, but the row still cannot land because the posting half lacks a roadmap-owned, scenario-authored proof seam. The current autonomous notice path is not the office-vacancy auxiliary path; it is threat-warning posting from live danger/threat memory. The current scenario schema cannot author the initial threat-memory substrate needed to prove that branch cleanly inside `survival-offices` without borrowing a different row's causal setup or relying on test-only belief seeding.

## Assumption Reassessment (2026-04-23)

1. `docs/scenario-roadmap.md` row 11 currently owns `Offices / succession / force-claim + notice posting`, and `survival-offices` is now the roadmap-named scenario/golden owner for that row's in-progress work.
2. `crates/worldwake-ai/tests/golden_survival_offices.rs` proves the live office seam today: `ClaimOffice` selection, committed `press_force_claim`, force control, delayed holder installation, and the authored survival-health contract.
3. `crates/worldwake-ai/src/candidate_generation.rs::emit_notice_posting_candidates` only emits autonomous `GoalKind::PostNotice` for `NoticeTopic::ThreatWarning`; it does not autonomously emit office-vacancy notices.
4. `crates/worldwake-ai/tests/golden_offices.rs::golden_vacancy_notice_unlocks_political_action_without_record_consult` proves office-vacancy notice uptake only through an externally requested `post_notice` payload override, not through autonomous roadmap-scenario AI behavior.
5. `crates/worldwake-ai/src/route_threat.rs::strongest_threat_warning_place` derives notice-posting pressure from remembered combat activity, wounds, social conflict observations, or existing threat-warning notices. Current scenario authoring cannot directly seed those remembered threat surfaces in `ScenarioDef`.
6. `scenarios/cli-evaluation.ron` plus archived `S51ARTISS-004` already prove general posting outside the roadmap row, but that evidence is not a truthful substitute for a survival-roadmap landing because it lacks the row-owned 1440-tick survival contract and authored office coexistence branch.
7. Shared abstraction boundary under audit: the scenario authoring surface for threat-memory / artifact substrate versus the autonomous AI notice-posting candidate-generation contract.
8. The motivating invariant is not merely "a notice exists." The row needs a survival-roadmap proof that notice posting lawfully competes with self-care inside the same authored scenario that owns the office branch.
9. Adjacent contradiction classification: this is a real missing authoring/proof substrate for row 11, not a flaky golden or a problem with the already-landed office-force-claim seam.

## Architecture Check

1. The clean fix is to add or expose a truthful scenario-authored substrate for autonomous notice posting, or to move autonomous office-vacancy notice generation into the live AI contract if that is the intended design. Both approaches preserve causality and keep roadmap proof authored rather than test-injected.
2. Reusing test-only belief seeding to fake the posting half inside `golden_survival_offices.rs` would violate the roadmap contract by proving a branch the scenario itself does not author.

## Verification Layers

1. Scenario can author the required notice-posting substrate -> scenario parser/spawn plus `scenario_coverage`
2. Autonomous notice candidate generation occurs from that authored substrate -> decision trace
3. Posting action commits through the ordinary runtime -> action trace
4. Notice artifact exists and remains attributable in world state -> authoritative world state
5. Survival pressure still competes with posting rather than being bypassed -> survival-health assertions plus action mix

## What to Change

### 1. Add a truthful scenario-authorable posting trigger

Extend the scenario surface so row-11 scenarios can author the specific threat-memory or artifact substrate that `emit_notice_posting_candidates` already consumes, or deliberately broaden the live AI contract if autonomous office-vacancy notice posting is the intended mechanic.

### 2. Land the blocked half in `survival-offices`

Update `scenarios/survival-offices.ron` and `crates/worldwake-ai/tests/golden_survival_offices.rs` so the roadmap-owned scenario proves notice posting at the earliest honest causal surface without weakening the office branch or the survival-health contract.

### 3. Refresh roadmap truth

Once the posting seam is truly proven, update `docs/scenario-roadmap.md`, `docs/generated/scenario-coverage.md`, and the generated golden docs so row 11 can move from `In Progress` to `Landed`.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify, if new authored substrate is required)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify, if new authored substrate is required)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify only if the live AI contract is intentionally broadened)
- `scenarios/survival-offices.ron` (modify)
- `crates/worldwake-ai/tests/golden_survival_offices.rs` (modify)
- `docs/scenario-roadmap.md` (modify)

## Out of Scope

- Reopening the landed office-force-claim survival seam already proved by `golden_survival_offices.rs`
- Broad combat, justice, or theft implementation
- Replacing the roadmap-owned survival proof with auxiliary or non-survival posting evidence

## Acceptance Criteria

### Tests That Must Pass

1. `crates/worldwake-ai/tests/golden_survival_offices.rs` proves autonomous notice posting plus the existing force-law office branch under the scenario-authored survival contract
2. `cargo run -p worldwake-cli --bin scenario-coverage -- --write`
3. `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. The posting proof is scenario-authored, not test-injected
2. The office-force-claim branch remains truthful and survives the same 1440-tick scenario
3. Row 11 is not marked `Landed` until both halves are proven by the roadmap-owned scenario/golden

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_offices.rs` — prove the posting half at the earliest honest causal surface while preserving the existing office branch
2. `None — if the change is scenario-authoring only and existing focused runtime coverage already proves the lower-layer candidate-generation surface, cite that lower-layer coverage in the ticket update.`

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_offices survival_offices_proves_force_law_uptake -- --ignored --exact`
2. `cargo test -p worldwake-ai --test golden_survival_offices -- --ignored --test-threads=1`
3. `cargo run -p worldwake-cli --bin scenario-coverage -- --write`
4. `python3 scripts/golden_inventory.py --write --check-docs`
