# S11WOULIFAUD-005: Golden verification and archival closeout

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `archive/tickets/completed/S11WOULIFAUD-003-deprivation-wound-worsening-instead-of-duplication.md`, `archive/tickets/completed/S11WOULIFAUD-004.md`, `archive/specs/S11-wound-lifecycle-audit.md`

## Problem

The active ticket was stale. It assumed S11WOULIFAUD-003 and S11WOULIFAUD-004 were still pending and that this ticket needed to recapture golden hashes after those changes landed. The codebase already contains the delivered wound-list lookup API, deprivation wound worsening, wound hardening coverage, and recovery-aware ranking changes. The remaining work is to verify the shipped behavior still passes the relevant suites and to close out the S11 ticket/spec trail accurately.

## Assumption Reassessment (2026-03-21)

1. The production changes this ticket references are already present:
   - [`crates/worldwake-core/src/wounds.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/wounds.rs) defines `WoundList::find_deprivation_wound()` and `find_deprivation_wound_mut()`.
   - [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) uses `worsen_or_create_deprivation_wound()`.
   - [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) already carries `has_clotted_wounds` in `RankingContext` and promotes recovery-relevant `High` needs to `Critical`.
2. The dependency naming in the active ticket was incomplete. The prerequisite tickets already exist only as archived completed work under [`archive/tickets/completed/S11WOULIFAUD-003-deprivation-wound-worsening-instead-of-duplication.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S11WOULIFAUD-003-deprivation-wound-worsening-instead-of-duplication.md) and [`archive/tickets/completed/S11WOULIFAUD-004.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S11WOULIFAUD-004.md).
3. Golden coverage is broader than the original ticket claimed. `cargo test -p worldwake-ai -- --list` confirms golden suites live across `golden_ai_decisions`, `golden_care`, `golden_combat`, `golden_determinism`, `golden_emergent`, `golden_offices`, `golden_production`, `golden_social`, `golden_supply_chain`, and `golden_trade`, not only in a generic `crates/worldwake-ai/tests/*.rs` bucket.
4. This is not an AI-regression implementation ticket anymore. The relevant verification boundary is existing golden/E2E plus workspace-wide regression coverage. No candidate-generation, ranking, plan-search, or authoritative mutation code remained to change after reassessment.
5. No ordering contract is being introduced or changed here. This closeout only verifies existing deterministic replay and canonical-state behavior.
6. No heuristic/filter removal is involved.
7. No stale-request, contested-affordance, or control-runtime behavior is involved.
8. The spec still legitimately called for golden verification, but the expected mismatch never materialized in the current repo state: the relevant goldens and workspace suites already pass without any hash or assertion recapture.
9. Corrected scope: this ticket should be treated as verification and archival closeout only. No code or test edits are required unless the verification commands reveal an actual failing golden.
10. Mismatch corrected: the original ticket's `PENDING` status and "hash recapture" scope no longer matched the repository. The ticket is complete once the verification commands pass and the archival trail is updated.

## Architecture Check

1. The current shipped architecture is better than the stale ticket implied. Deprivation harm is modeled as one persistent wound per deprivation kind, worsened in place from the authoritative needs write path, which is cleaner and more extensible than duplicate-wound accumulation or read-time deduplication.
2. The recovery-aware priority boost is also the right architectural shape: [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) derives the boost from concrete wound state in the belief view, keeping AI aligned with authoritative recovery gates without adding compatibility aliases, new goal kinds, or profile knobs.
3. Because the robust architecture is already in place, forcing extra hash-recapture churn would be worse than the current state. The clean result here is to record that no further golden expectation changes were necessary.
4. No backwards-compatibility shims or alias paths were introduced.

## Verification Layers

1. S11 deprivation-wound identity/worsening remains shipped in authoritative state -> existing focused `needs.rs` tests plus workspace regression coverage
2. S11 recovery-aware self-care promotion remains shipped in AI ranking -> existing focused `ranking.rs` tests plus `cargo test -p worldwake-ai`
3. Golden deterministic behavior after the delivered S11 changes -> `cargo test -p worldwake-ai`
4. Cross-crate regression safety for the delivered S11 work -> `cargo test --workspace`
5. Lint/static hygiene for the shipped state -> `cargo clippy --workspace`
6. Single-layer closeout ticket: no new action-trace or decision-trace assertions are added because this ticket only verifies and archives already-delivered behavior

## What to Change

### 1. Reassess the ticket against current code and tests

Confirm whether S11WOULIFAUD-003 and S11WOULIFAUD-004 are actually pending or already implemented. Update the ticket content first if that assumption is stale.

### 2. Run the current verification boundary

Run the existing AI golden suite and full workspace verification:

```bash
cargo test -p worldwake-ai
cargo test --workspace
cargo clippy --workspace
```

### 3. Close out the S11 documentation trail

If verification passes without any golden drift requiring recapture, mark this ticket complete, archive it, archive the completed S11 spec, and remove S11 from the active implementation-order inventory.

## Files to Touch

- `tickets/S11WOULIFAUD-005.md` (modify, then archive)
- `specs/S11-wound-lifecycle-audit.md` (modify, then archive)
- `specs/IMPLEMENTATION-ORDER.md` (modify)

## Out of Scope

- Re-implementing deprivation wound worsening in `needs.rs`
- Re-implementing recovery-aware ranking in `ranking.rs`
- Changing wound pruning/progression in `combat.rs`
- Inventing new golden scenarios or changing scenario setup
- Editing any golden expectations without an actual failing verification command
- Fixing unrelated workspace failures

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

### Invariants

1. This closeout ticket does not introduce new production behavior
2. The S11 archival trail accurately reflects the already-shipped architecture
3. If a golden recapture is not needed, the ticket must say so explicitly rather than pretending test files changed

## Test Plan

### New/Modified Tests

1. None — this closeout ticket required no new or modified tests because the shipped S11 behavior already passed the current golden and workspace verification boundary unchanged.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - corrected the stale ticket assumptions to match the current repository state
  - verified that the S11 wound lifecycle work was already implemented in core, systems, and AI
  - ran the relevant verification boundary without needing any hash recapture or test expectation edits
  - archived the S11 ticket/spec trail and updated the active implementation order
- Deviations from original plan:
  - no golden assertions or hashes needed recapture because `cargo test -p worldwake-ai` already passed unchanged
  - no code changes were necessary in this ticket; the architecture had already been improved by the earlier S11 tickets
- Verification results:
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace` ✅
