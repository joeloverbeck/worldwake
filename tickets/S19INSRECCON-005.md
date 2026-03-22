# S19INSRECCON-005: Update golden-e2e-coverage.md and golden-e2e-scenarios.md with S19 scenarios

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation only
**Deps**: S19INSRECCON-002, S19INSRECCON-003, S19INSRECCON-004 (all three scenario tests must be passing)

## Problem

After implementing Scenarios 32–34 in S19INSRECCON-002..004, the golden E2E documentation must be updated to reflect the new coverage. Two docs need updates:
1. `docs/golden-e2e-coverage.md` — coverage matrix showing which systems/principles each scenario exercises.
2. `docs/golden-e2e-scenarios.md` — detailed descriptions of each scenario's setup, emergent behavior, and assertion surface.

Without these updates, future contributors will not know that ConsultRecord has E2E coverage, and may duplicate effort or miss the coverage gap being closed.

## Assumption Reassessment (2026-03-22)

1. `docs/golden-e2e-coverage.md` exists and tracks existing scenarios in a matrix format. Confirmed present on disk.
2. `docs/golden-e2e-scenarios.md` exists and contains detailed scenario descriptions. Confirmed present on disk.
3. No code changes — documentation only. Existing tests are unaffected.
11. No mismatches expected — this ticket is purely additive documentation.

## Architecture Check

1. Documentation follows existing patterns in both files. No structural changes needed.
2. No backward-compatibility concerns.

## Verification Layers

5. Single-layer ticket (documentation). Verification is visual/manual inspection plus the golden inventory script.

## What to Change

### 1. Update `docs/golden-e2e-coverage.md`

Add Scenarios 32, 33, 34 to the coverage matrix. New cross-system chains to document:

- **Scenario 32**: ConsultRecord prerequisite → institutional belief acquisition → DeclareSupport → succession installation
  - Systems: AI planning (ConsultRecord prerequisite), ConsultRecord handler, Political actions, Succession
  - Principles: P12 (world state ≠ belief), P13 (knowledge via carrier), P16 (records are world state), P21 (institutions as offices + records)

- **Scenario 33**: Remote record → travel to record → consultation → travel to office → political action
  - Systems: Travel (multi-hop), ConsultRecord handler, Political actions, Succession, AI planning (4-step plan)
  - Principles: P7 (locality), P8 (duration/cost), P1 (maximal emergence)

- **Scenario 34**: Knowledge asymmetry → consultation duration cost → competitive political outcome
  - Systems: AI planning (ConsultRecord vs direct), ConsultRecord handler, Political actions, Succession, multi-agent
  - Principles: P14 (Unknown vs Certain divergence), P20 (knowledge diversity), P8 (consultation cost), P1 (emergent outcome)

### 2. Update `docs/golden-e2e-scenarios.md`

Add detailed scenario descriptions for Scenarios 32–34. Each description should include:
- Scenario number and title
- Setup summary (agents, offices, records, beliefs, topology)
- Emergent behavior proven
- Assertion surface
- Distinctiveness from related scenarios

### 3. Run golden inventory script

Run `python3 scripts/golden_inventory.py --write --check-docs` to verify the inventory reflects the new tests.

## Files to Touch

- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- No code changes of any kind
- No changes to `golden_offices.rs` or `golden_harness/mod.rs`
- No changes to engine crates
- No changes to specs

## Acceptance Criteria

### Tests That Must Pass

1. `python3 scripts/golden_inventory.py --write --check-docs` — inventory script passes with updated docs
2. `cargo test -p worldwake-ai` — full AI crate suite (smoke check — no code changes, but verify nothing broke)

### Invariants

1. All existing scenario descriptions in both docs remain unchanged
2. New scenario entries follow the established format and numbering (Scenario 32, 33, 34)
3. Cross-system chain descriptions accurately reflect the actual test implementations from S19INSRECCON-002..004
4. No code files are modified

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs` — golden inventory refresh
2. `cargo test -p worldwake-ai` — smoke check
