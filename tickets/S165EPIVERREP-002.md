# S165EPIVERREP-002: Wire InsertVerification arm to candidate repair

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` plan repair (`plan_repair.rs`)
**Deps**: specs/S165-epistemic-verification-repair.md (D4)

## Problem

`RepairKind::InsertVerification` is wired into the live repair attempt order but its
handler unconditionally returns `Err(RepairFailure::NoEpistemicSubstrate)`
(`crates/worldwake-ai/src/plan_repair.rs:131`), consulting none of the
`PlanRepairContext`. The other candidate-bearing kinds (`RebindTarget`,
`ReplaceProvider`) already route through `attempt_candidate_repair`. This ticket makes
`InsertVerification` select a supplied verification candidate the same way, so that once
the seam (ticket 003) supplies one, the arm consumes it.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The current arm is `RepairKind::InsertVerification => Err(RepairFailure::NoEpistemicSubstrate)`
   at `crates/worldwake-ai/src/plan_repair.rs:131`. The sibling arms call
   `attempt_candidate_repair(context, kind).ok_or(<failure>)`
   (`plan_repair.rs:125-130`). `attempt_candidate_repair` (`plan_repair.rs:145`) looks up
   `context.replacement_candidates` for a candidate whose `kind` matches and composes the
   plan via `plan_from_parts`; it returns `None` when no matching candidate exists.
2. Spec deliverable D4 (`specs/S165-epistemic-verification-repair.md`).
3. Existing tests exercising this arm:
   `insert_verification_returns_no_epistemic_substrate_without_s139`
   (`crates/worldwake-ai/src/plan_repair.rs:546`) and
   `stale_belief_breach_attempts_insert_verification_without_s139`
   (`crates/worldwake-ai/tests/scenarios/plan_repair.rs:300`). After this ticket alone, no
   `InsertVerification` candidate is supplied (ticket 003 adds construction), so
   `attempt_candidate_repair` returns `None` → `ok_or(NoEpistemicSubstrate)` → the **same**
   result; both tests still pass. They are migrated in ticket 006 once construction lands.

## Architecture Check

1. The arm becomes structurally identical to its candidate-bearing siblings — a uniform
   "select the supplied candidate or fall through" contract — rather than a special-cased
   placeholder. This is cleaner than a bespoke synthesis path inside `plan_repair`, which
   cannot search and must not (it is a pure composition engine).
2. No shim: the placeholder `Err(...)` is replaced, not wrapped.

## Verification Layers

1. With no `InsertVerification` candidate supplied, the arm returns
   `RepairFailure::NoEpistemicSubstrate` (behavior-preserving) → focused unit test.
2. With an `InsertVerification` candidate supplied, the arm returns a `Repaired` plan
   containing the candidate's step → focused unit test (constructs a synthetic candidate).
3. Single-layer (plan repair) ticket — authoritative mutation is not reached here; the
   end-to-end event-log proof lives in ticket 006.

## What to Change

### 1. Replace the placeholder arm

Change `RepairKind::InsertVerification => Err(RepairFailure::NoEpistemicSubstrate)` to
`RepairKind::InsertVerification => attempt_candidate_repair(context, kind).ok_or(RepairFailure::NoEpistemicSubstrate)`,
matching the `RebindTarget`/`ReplaceProvider` pattern. This is behavior-preserving until
ticket 003 supplies a candidate; the construction site is named in this ticket's Out of
Scope and in ticket 003's What to Change (placeholder-replace pattern, replacement
direction).

## Files to Touch

- `crates/worldwake-ai/src/plan_repair.rs` (modify)

## Out of Scope

- Construction of the `InsertVerification` `RepairPlanCandidate` at the revalidation seam
  — owned by ticket 003 (which replaces the "no candidate supplied" state this ticket
  leaves in place).
- Authoritative anchor recording (ticket 003, D5) and trace fields (ticket 005, D7).

## Acceptance Criteria

### Tests That Must Pass

1. New: arm returns `NoEpistemicSubstrate` when `replacement_candidates` has no
   `InsertVerification` entry.
2. New: arm returns `Repaired` with the candidate's step when an `InsertVerification`
   `RepairPlanCandidate` is supplied.
3. Existing suite: `cargo test -p worldwake-ai plan_repair` (the two `without_s139` tests
   still pass).

### Invariants

1. `plan_repair` performs no search and constructs no goal — it only selects/arranges
   pre-built steps (FND-20/FND-26 layer boundary preserved).
2. The fall-through order (`InsertVerification` → `DowngradeToTypedBarrier` → `Abandon`)
   is unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_repair.rs` (inline `#[cfg(test)]`) — supplied-candidate
   and no-candidate arm behavior.

### Commands

1. `cargo test -p worldwake-ai plan_repair`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `./scripts/verify.sh`
