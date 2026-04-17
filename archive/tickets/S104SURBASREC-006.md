# S104SURBASREC-006: Rebuild golden test coverage (Layers 1-3)

**Status**: 🚫 NOT IMPLEMENTED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None
**Deps**: archive/tickets/S104SURBASREC-005.md

## Problem

After Layer 0 proves survival works, the project needs to rebuild the cross-system golden test coverage that was removed in S104SURBASREC-001. The old tests were grounded in sterile setups that assumed survival magically worked. The rebuilt tests must start from survival-capable agents and layer system-specific complexity incrementally. This ticket covers Layers 1 (single-system addition), 2 (cross-system interaction), and 3 (soak and determinism).

## Assumption Reassessment (2026-04-15)

1. Layer 0 (`golden_survival_baseline.rs`) landed in `archive/tickets/S104SURBASREC-005.md`, providing proven survival-capable agent configurations loaded from the authored `survival-baseline.ron` scenario.
2. The systems to rebuild coverage for include: trade, combat, social, offices, patrol, care, production, exploration (beyond survival), pursuit, supply chain, and multi-system emergent scenarios.
3. Golden harness infrastructure remains available: `golden_harness/mod.rs`, `golden_harness/soak_world.rs`, `golden_harness/timeline.rs`.
4. This ticket is intentionally large because the rebuild is incremental and each layer depends on the prior. It may be split further during implementation if the scope proves too broad for a single review cycle.
5. Factual follow-up from archived `S104SURBASREC-001`: new `crates/worldwake-ai/tests/golden_*.rs` files should be authored as standalone Rust integration tests with their own local `mod golden_harness;` declarations, not by editing a shared module-entrypoint file.

## Architecture Check

1. Layered rebuild ensures each layer validates survival preservation before adding complexity. This is the key architectural insight from S104 — system-specific tests must prove their system doesn't break survival, not assume survival magically works.
2. No backwards-compatibility shims. All tests are new, following invariant-first conventions established by Layer 0.

## Verification Layers

1. Layer 1: survival + single system → each new test proves survival invariants hold when one non-survival system is added
2. Layer 2: survival + cross-system → multi-system interaction chains work without degrading survival
3. Layer 3: soak and determinism → long-run stability and hash-based determinism with regenerated hashes
4. Test infrastructure only — no runtime changes.

## What to Change

### 1. Layer 1: Single-System Addition Tests

For each major non-survival system, create tests that:
- Start from survival-baseline agent configurations
- Add that system's profiles to one or more agents
- Run 1440 ticks
- Assert: Layer 0 survival invariants still hold (no deaths, needs managed)
- Assert: system-specific invariants (e.g., "merchant completes at least one trade AND stays alive")

Systems to cover (one test file or section per system):
- Trade (MerchandiseProfile)
- Combat (CombatProfile)
- Social (TellProfile — now safe after S104SURBASREC-003)
- Offices/Patrol (PatrolProfile, JusticeDispositionProfile)
- Care/Medical
- Production (beyond survival recipes)

### 2. Layer 2: Cross-System Golden Tests

Rebuilt versions of the most valuable removed multi-system tests, now grounded in survival-capable agents:
- Tests begin from survival-proven configurations
- Add cross-system interactions one at a time
- Each test proves its specific cross-system chain without assuming survival works
- Focus on the highest-value emergent scenarios from the removed `golden_emergent.rs` and `golden_integration.rs`

### 3. Layer 3: Soak and Determinism Rebuild

- New soak test using survival baseline world, longer duration (e.g., 5000+ ticks)
- Determinism test with freshly generated StateHash values — these hashes are regenerated from the new behavioral baseline, not carried over from the old tests
- Long-scenario tests with survival-capable agents

### 4. Author each new test file as a standalone integration test

Each new `golden_survival_*.rs` file should include its own local `mod golden_harness;` declaration and imports consistent with the surviving golden test files.

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_layer1.rs` (new — or multiple files per system)
- `crates/worldwake-ai/tests/golden_survival_cross_system.rs` (new)
- `crates/worldwake-ai/tests/golden_survival_soak.rs` (new)
- `crates/worldwake-ai/tests/golden_survival_determinism.rs` (new)

## Out of Scope

- Modifying Layer 0 tests (S104SURBASREC-005)
- Modifying the survival baseline scenario
- Modifying production code or engine behavior
- Rebuilding every removed test 1:1 — prioritize highest-value emergent scenarios
- Performance optimization of golden tests

## Acceptance Criteria

### Tests That Must Pass

1. All Layer 1 tests: survival invariants hold with each single-system addition
2. All Layer 2 tests: cross-system interactions work with survival-capable agents
3. Layer 3 soak test: extended run without crashes, panic, or need saturation
4. Layer 3 determinism test: identical hashes across runs with same seed
5. Existing suite: `cargo test -p worldwake-ai` — all tests pass
6. Full workspace: `cargo test --workspace`

### Invariants

1. Every new test verifies Layer 0 survival invariants as a baseline before asserting system-specific behavior
2. StateHash assertions in Layer 3 use freshly generated hashes, not carried-over values from removed tests
3. No test assumes survival works without proving it — all tests start from survival-proven configurations

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_layer1.rs` — single-system addition tests proving survival + each system coexists
2. `crates/worldwake-ai/tests/golden_survival_cross_system.rs` — cross-system interaction golden tests grounded in survival
3. `crates/worldwake-ai/tests/golden_survival_soak.rs` — long-run soak test with survival baseline world
4. `crates/worldwake-ai/tests/golden_survival_determinism.rs` — determinism verification with regenerated hashes

### Commands

1. `cargo test -p worldwake-ai -- golden_survival_layer1` — Layer 1 tests
2. `cargo test -p worldwake-ai -- golden_survival_cross_system` — Layer 2 tests
3. `cargo test -p worldwake-ai -- golden_survival_soak` — Layer 3 soak
4. `cargo test -p worldwake-ai -- golden_survival_determinism` — Layer 3 determinism
5. `cargo test -p worldwake-ai` — full AI crate suite
6. `cargo clippy --workspace --all-targets -- -D warnings` — clean

## Outcome

Archived as NOT IMPLEMENTED on 2026-04-17 per explicit user direction.

- No Layer 1, Layer 2, or Layer 3 rebuild work was implemented from this ticket.
- The survival-baseline recovery effort closed with the landed triage, profile-gating cleanup, authored baseline scenario, planner cleanup, and Layer 0 golden proof from archived tickets `S104SURBASREC-001` through `S104SURBASREC-005` and `S104SURBASREC-007`.
- The broader golden rebuild described here was intentionally not pursued further.
