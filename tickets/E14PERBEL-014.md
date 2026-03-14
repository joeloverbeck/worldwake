# E14PERBEL-014: Extract Shared Corpse Belief Evidence Rules From Candidate Generation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` corpse belief-evidence helper extraction, candidate-generation cleanup, and focused regression coverage
**Deps**: `archive/tickets/E14PERBEL-012.md`, `archive/tickets/E14PERBEL-010.md`, `archive/tickets/E14PERBEL-009.md`, `specs/E14-perception-beliefs.md`, `specs/S03-planner-target-identity-and-affordance-binding.md`

## Problem

`E14PERBEL-012` fixed a real architecture seam by making corpse-backed `AcquireCommodity` evidence accept believed aggregate corpse commodity quantities, not just known direct corpse lots.

That fix is correct, but the longer-term architecture is still only partially hardened:

- corpse knowledge rules still live as local helper logic inside `crates/worldwake-ai/src/candidate_generation.rs`
- `corpse_has_known_loot()` and `corpse_contains_commodity()` remain separate call sites over the same conceptual rule family
- future corpse-related AI paths can still drift if they re-encode “what does the agent know about this corpse?” slightly differently

This is not an immediate bug, but it is a real extensibility seam. If corpse evidence remains encoded as ad hoc candidate-generation helpers, every later corpse-aware goal or ranking path must remember the same belief sufficiency rules manually.

## Assumption Reassessment (2026-03-14)

1. Current production code is already correct for the two active corpse commodity paths in `crates/worldwake-ai/src/candidate_generation.rs`:
   - `corpse_has_known_loot()` now recognizes believed corpse commodity quantities
   - `corpse_contains_commodity()` now recognizes both direct corpse lots and believed aggregate corpse quantity for the queried commodity
2. That correctness currently depends on local helper discipline inside one file, not on a named shared abstraction that expresses the corpse belief-evidence contract directly.
3. No current spec requires a dedicated corpse belief helper module, but `specs/E14-perception-beliefs.md` does require belief-only planning and stable world/belief separation. A clearer shared read-model helper supports that contract better than leaving the rule as duplicated local plumbing.
4. `archive/tickets/E14PERBEL-010.md` and `archive/tickets/E14PERBEL-012.md` show the same family of issue appearing twice: first corpse loot, then corpse-backed acquisition. That recurrence is evidence that drift risk is real, not hypothetical.
5. No active ticket in `tickets/` currently owns this narrower cleanup. `E14PERBEL-013` is about de-omniscient golden harness setup, which is adjacent but distinct.

## Architecture Check

1. Extracting a dedicated corpse belief-evidence helper is cleaner than leaving the rule embedded in candidate-generation plumbing because it gives the codebase one named place that defines subjective corpse inventory sufficiency.
2. This is more robust than the current architecture for future work. New corpse-related goals or ranking heuristics can call the shared helper instead of recreating possession-vs-belief fallbacks ad hoc.
3. The extraction should stay in the AI read/model layer. Do not move corpse evidence logic into planner search, action binding, or authoritative world queries.
4. No backwards-compatibility aliasing is allowed. The old local helper arrangement should be replaced, not preserved beside a new abstraction.
5. Keep scope narrow. This ticket is about shared corpse evidence rules, not a broad split of `candidate_generation.rs` or a general-purpose “entity knowledge” framework.

## What to Change

### 1. Introduce a named shared corpse belief-evidence helper

Create a small AI-side helper surface that answers corpse inventory evidence questions directly, for example:

- whether a corpse has any known loot under subjective beliefs
- whether a corpse has a known quantity of a specific commodity under subjective beliefs

The exact file and function names may vary, but the abstraction should make the belief contract explicit rather than burying it inside candidate-generation internals.

### 2. Rewire candidate generation to use the shared helper exclusively

Update corpse-aware candidate generation paths to depend only on the shared helper.

At minimum, that includes:

- loot-goal emission
- corpse-backed acquisition-path evidence

After this change, `candidate_generation.rs` should no longer define multiple corpse-knowledge rules independently.

### 3. Keep the helper conservative and belief-local

The shared helper must:

- read only through belief-facing AI interfaces already allowed by E14
- accept known direct corpse possession structure when available
- accept believed aggregate corpse commodity quantities when direct structure is absent
- avoid fabricating exact lot identity when only aggregate belief exists

Do not add any omniscient fallback, compatibility alias, or hidden authoritative query.

### 4. Add regression coverage at the helper boundary

Add tests that make the shared corpse evidence contract explicit, rather than proving it only indirectly through one goal kind.

The test surface may remain in `candidate_generation.rs` if extraction stays private to the module, or move with the helper if a separate module is introduced.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify if a new internal module is introduced)
- `crates/worldwake-ai/src/<corpse helper module>` (new, if extraction warrants a separate file)
- `tickets/E14PERBEL-014.md` (new)

## Out of Scope

- Further behavior changes to corpse candidate semantics beyond the `E14PERBEL-012` rule
- Planner target binding or exact lot identity work (`specs/S03-planner-target-identity-and-affordance-binding.md`)
- Perception acquisition changes (`E14PERBEL-011`)
- Golden harness cleanup (`E14PERBEL-013`)
- General decomposition of all candidate-generation helper logic

## Acceptance Criteria

### Tests That Must Pass

1. Corpse loot and corpse-backed `AcquireCommodity` paths both use the same named corpse evidence helper or helper family.
2. A regression proves believed corpse aggregate commodity quantities remain sufficient for corpse-backed `AcquireCommodity`.
3. A regression proves corpse loot still works from believed corpse aggregate inventory.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Corpse inventory sufficiency under subjective beliefs is defined in one AI-side abstraction, not re-encoded independently across corpse-aware paths.
2. The helper remains belief-local and deterministic.
3. Aggregate believed commodity state may justify candidate evidence, but exact lot identity is not fabricated.
4. No backwards-compatibility alias or omniscient fallback is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` or extracted helper module tests — assert corpse commodity sufficiency from direct corpse lots and from believed aggregate corpse quantity.
   Rationale: locks the helper contract directly instead of relying only on downstream goal emission.
2. `crates/worldwake-ai/src/candidate_generation.rs` — retain or update the existing loot-goal regression using believed corpse inventory.
   Rationale: proves the extraction does not regress the already-fixed loot path.
3. `crates/worldwake-ai/src/candidate_generation.rs` — retain or update the existing corpse-backed `AcquireCommodity` regression from believed corpse inventory.
   Rationale: proves the extraction preserves the newly-fixed acquisition path.

### Commands

1. `cargo test -p worldwake-ai corpse`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
