# S174SHESLESUR-012: Repair S174 golden scenario identifier and regenerate golden docs

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — source-golden metadata and generated documentation only
**Deps**: `archive/tickets/S174SHESLESUR-007.md`, `archive/tickets/S174SHESLESUR-009.md`

## Outcome

Repaired S174's source-golden metadata and regenerated the golden documentation. `survival_safe_rest.rs` now uses non-colliding `Scenario 483`, and `survival_rest_interrupted_by_danger.rs` now has a source-golden metadata block as `Scenario 484`.

The generator now succeeds and the generated docs include S174 Scenario A, Scenario B, and Scenario C rest coverage. The executable golden behavior remained unchanged.

## Problem Resolved

The post-ticket review for `archive/tickets/S174SHESLESUR-009.md` attempted to validate generated golden documentation, but `python3 scripts/golden_inventory.py --check-docs` aborted before it could inspect the new Scenario C surface. The generator reported a duplicate source scenario identifier:

```text
ERROR: duplicate scenario identifier '481': survival_safe_rest.rs:379 and survival_self_care_interruption.rs:840
```

`crates/worldwake-ai/tests/scenarios/survival_safe_rest.rs` was introduced by S174 Scenario A and currently labels `scenario_a_rest_site_contention` as `Scenario 481`, which collides with the older S173 self-care golden. The Scenario C test added by `archive/tickets/S174SHESLESUR-009.md` also needs a source-golden metadata block so the generated docs can include its hostile-proximity proof. Until these metadata gaps are repaired, S174's generated golden scenario index/details cannot be regenerated to include the latest rest scenarios.

## Assumption Reassessment (2026-05-26)

1. Verified current failure: `python3 scripts/golden_inventory.py --check-docs` exits with the duplicate `Scenario 481` error before doc comparison.
2. Verified the duplicate source metadata: `crates/worldwake-ai/tests/scenarios/survival_safe_rest.rs:379` and `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs:840` both declare `// Scenario 481: ...`.
3. Shared abstraction boundary under audit: source-golden metadata comments are the canonical input to `scripts/golden_inventory.py`; generated docs under `docs/generated/` must be derived from that source metadata, not hand-edited.
4. This is not an engine behavior bug. The executable Scenario A and Scenario C assertions already pass; the blocked surface is golden inventory traceability.
5. The S174-owned source metadata should receive a non-colliding scenario identifier. Do not renumber the older S173 scenario unless the generator's existing numbering policy proves that is the intended owner.
6. Scenario isolation: this ticket only repairs the duplicate identifier and regenerates generated docs. It must not change scenario behavior, assertions, or engine code.
7. Mismatch + correction: `crates/worldwake-ai/tests/scenarios/survival_rest_interrupted_by_danger.rs` currently has module-level prose but no `// Scenario <id>:` metadata block for `scripts/golden_inventory.py`. Assign it the next non-colliding identifier after the repaired Scenario A id.

## Architecture Result

1. Repairing the source-golden comment preserves the generator-as-authority pattern for `docs/generated/*`.
2. No compatibility alias or duplicate documentation path is introduced; generated docs are refreshed from the single source metadata surface.

## Layer Proofs

1. Duplicate identifier is gone -> `python3 scripts/golden_inventory.py --check-docs` no longer aborts on source metadata parsing
2. Generated docs include the S174 safe-rest and hostile-proximity scenarios -> regenerated `docs/generated/golden-scenario-index.md` and relevant detail pages
3. Scenario behavior unchanged -> focused golden tests for `survival_safe_rest` and `survival_rest_interrupted_by_danger`
4. Markdown/source hygiene -> `git diff --check`

## Landed Changes

1. Changed `// Scenario 481: S174 Safe Rest Contention` to `// Scenario 483: S174 Safe Rest Contention`.
2. Added `// Scenario 484: S174 Hostile-Proximity Rest Interruption` metadata with setup/proves/chain prose to the Scenario C golden test.
3. Regenerated `docs/generated/*` golden inventory outputs with `python3 scripts/golden_inventory.py --write --check-docs`.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/survival_safe_rest.rs`
- `crates/worldwake-ai/tests/scenarios/survival_rest_interrupted_by_danger.rs`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-scenario-details/cognitive-archetypes.md`
- `docs/generated/golden-scenario-details/place-dirtiness.md`
- `docs/generated/golden-scenario-details/sleep-episode.md`
- `docs/generated/golden-scenario-details/survival-rest-interrupted-by-danger.md`
- `docs/generated/golden-scenario-details/survival-safe-rest.md`
- `docs/generated/golden-scenario-details/survival-self-care-interruption.md`
- `docs/generated/golden-scenario-details/survival-sleep-contention.md`

## Out of Scope

- No engine behavior changes
- No assertion rewrites in Scenario A or Scenario C
- No unrelated golden metadata cleanup beyond what is required to make the generator pass

## Acceptance Criteria

### Passed Checks

1. `python3 scripts/golden_inventory.py --write --check-docs` passed.
2. `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_safe_rest scenarios::survival_rest_interrupted_by_danger` passed.
3. `git diff --check` passed.

### Invariants

1. Every `Scenario <id>:` source-golden identifier is unique.
2. Generated golden docs are derived from source metadata by the generator.
3. Scenario A and Scenario C executable behavior remains unchanged.

## Tests

1. `None — metadata/docs repair only; existing golden tests prove behavior remains unchanged.`

## Verification Result

1. Passed `python3 scripts/golden_inventory.py --write --check-docs`
2. Passed `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_safe_rest scenarios::survival_rest_interrupted_by_danger`
3. Passed `git diff --check`
