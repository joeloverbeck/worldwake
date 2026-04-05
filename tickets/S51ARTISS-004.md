# S51ARTISS-004: Scenario tuning and golden closeout for artifact issuance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario profile tuning and golden closeout
**Deps**: S51ARTISS-005

## Problem

After the S51 core, planner, candidate-generation, and ranking-activation slices land, the stack still lacks the end-to-end closeout surface: the active CLI evaluation scenario has no intentional non-zero posting weights to exercise the new behavior, and no golden proves autonomous social-artifact issuance from belief to committed artifact creation.

## Assumption Reassessment (2026-04-05)

1. `scenarios/cli-evaluation.ron` is the active showcase scenario and already carries explicit `utility_profile` payloads for multiple agents, so it is the correct place to add non-zero posting weights once the behavior is live.
2. Existing social-artifact lifecycle goldens already live in `crates/worldwake-ai/tests/golden_integration.rs`: bounty posting/claim chains (Scenarios 105 and 108) and `ThreatWarning` notice posting with downstream route effects (Scenario 107). This is the strongest existing golden owner for general artifact-issuance closeout.
3. `crates/worldwake-ai/tests/golden_offices.rs` owns office-vacancy notice uptake into political action (Scenario 109), but that suite is narrower and office-specific. The general S51 posting closeout should reuse `golden_integration.rs` instead of fragmenting ownership.
4. Archived `S51ARTISS-003` made lawful posting candidates emit, but it intentionally left posting ranking at zero motive. Correction applied: this closeout ticket now depends on `S51ARTISS-005`, which owns the ranking/selection activation needed before showcase or golden proof can honestly claim autonomous posting behavior is live.
5. `S51ARTISS-001` already landed bounded `format_goal_kind()` support while sweeping exhaustive downstream handling for the new goal variants. Correction applied: CLI first-render ownership is no longer part of this ticket.

## Architecture Check

1. Reusing `golden_integration.rs` keeps issuance proof on the canonical social-artifact lifecycle suite instead of creating a duplicate posting-only golden file.
2. Scenario tuning belongs with the end-to-end closeout once posting behavior is actually live; landing non-zero showcase weights before ranking activation exists would add noise without proving anything.
3. `format_goal_kind()` already acknowledges the new goal variants, so this ticket can stay focused on making posting behavior intentionally visible in the showcase scenario and proving it end to end.
4. No backward-compatibility shims.

## Verification Layers

1. CLI evaluation scenario intentionally configures at least one autonomous posting agent -> authoritative RON world-init surface
2. Autonomous posting goal generation occurs from belief-driven motivation -> decision trace in golden
3. `post_bounty` / `post_notice` commits and creates the expected social artifact -> action trace + authoritative world state in golden
4. Generated golden inventory/docs remain aligned after new scenario metadata -> `python3 scripts/golden_inventory.py --write --check-docs`

## What to Change

### 1. Tune the active showcase scenario

In `scenarios/cli-evaluation.ron`, give at least one agent non-zero `bounty_posting_weight` and/or `notice_posting_weight` once the S51 stack is live.

### 2. Add end-to-end issuance goldens

In `crates/worldwake-ai/tests/golden_integration.rs`, add the strongest honest closeout scenarios for autonomous artifact issuance on the existing social-artifact suite. Reuse the current artifact helpers/harnesses rather than creating a new posting-specific golden file.

### 3. Refresh generated golden docs

Run `python3 scripts/golden_inventory.py --write --check-docs` after landing any new `// Scenario` blocks.

## Files to Touch

- `scenarios/cli-evaluation.ron` (modify)
- `crates/worldwake-ai/tests/golden_integration.rs` (modify)
- `docs/generated/golden-coverage-matrix.md` (modify)
- `docs/generated/golden-e2e-inventory.md` (modify)
- `docs/generated/golden-scenario-map.md` (modify)

## Out of Scope

- Core `GoalKind` / `UtilityProfile` shape changes — ticket 001
- CLI display acknowledgement for `PostBounty` / `PostNotice` — ticket 001
- Planner ops and dispatch wiring — ticket 002
- Candidate emission — archived `S51ARTISS-003`
- Ranking activation for posting goals — `S51ARTISS-005`
- Artifact revocation, reposting, or maintenance flows

## Acceptance Criteria

### Tests That Must Pass

1. At least one golden proves autonomous posting from belief-driven motivation through committed artifact creation
2. Generated golden docs refresh cleanly
3. Existing suite: `cargo test --workspace`

### Invariants

1. Golden proof reuses the canonical existing social-artifact suite rather than a duplicate posting-only file
2. Showcase scenario weights remain explicit and intentional rather than relying on hidden defaults
3. Posting proof stays belief-driven and artifact creation remains authoritative world state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs` — autonomous artifact-issuance golden scenario(s)
2. `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-map.md` — generated refresh after scenario metadata changes

### Commands

1. `cargo test -p worldwake-ai --test golden_integration`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
