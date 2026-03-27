# S32CRIMEMEGOLSUI-004: Close Out S32 Golden Coverage Docs, Inventory, and Archival Trail

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — closeout/docs/spec archival only
**Deps**: `archive/specs/S32-crime-emergence-golden-suites.md`, shipped S32 golden tests in `crates/worldwake-ai/tests/golden_emergent.rs`

## Problem

S32's implementation has already landed in live test source, but the closeout trail is incomplete and partially stale. The human-maintained dashboard still lists S32 in the pending backlog, generated golden artifacts on disk lag behind the current source inventory, and `specs/IMPLEMENTATION-ORDER.md` does not yet record S32 as completed/archived work. This ticket corrects the documentation/spec trail to match the shipped architecture and verification state.

## Assumption Reassessment (2026-03-27)

1. The ticket's original premise that Scenarios 41-43 still needed implementation is false. Live source already contains:
   - `golden_witness_deterrence_suppresses_theft_candidate`
   - `golden_exile_punishment_when_fine_is_not_locally_collectible`
   - `golden_dual_discovery_converges_without_double_accusation`
   - plus all three deterministic replay companions
   in `crates/worldwake-ai/tests/golden_emergent.rs`.
2. The live `GoalKind` surfaces under audit are already the shipped ones, not planned placeholders: `StealItem`, `Accuse`, `PunishAccused`, `InvestigateViolation`, and `ShareBelief`. The ticket scope is therefore closeout/verification, not code implementation.
3. `docs/golden-e2e-coverage.md` still lists S32 in `Pending Backlog Summary` and `Recommended Implementation Order`, which is now stale relative to the shipped tests.
4. `scripts/golden_inventory.py` does exist and remains the canonical generator for:
   - `docs/generated/golden-e2e-inventory.md`
   - `docs/generated/golden-scenario-map.md`
   - `docs/generated/golden-coverage-matrix.md`
   The generator validates current source successfully with `python3 scripts/golden_inventory.py --check-docs`, but the checked-in generated files lag behind current source and need regeneration with `--write`.
5. The canonical inventory rule in `tickets/README.md` means this ticket should not manually "move S32 to an implemented section" if that duplicates generated facts. The durable dashboard change is to remove S32 from the pending backlog and add a short implemented/removed-backlog rationale, while leaving detailed scenario counts to generated artifacts.
6. This is now a single-boundary closeout ticket. The shared boundary under audit is the golden coverage documentation contract: source-declared scenario metadata in `golden_emergent.rs` -> generated docs via `scripts/golden_inventory.py` -> human-maintained backlog/roadmap docs.
7. The spec's `GOLDE2E-020` deliverable is stale in one important way: it says "Add S32 scenarios to pending backlog" as part of the work. That was appropriate pre-implementation, but the correct end-state after shipped tests is removal from the pending backlog, regeneration of artifacts, and archival of the completed spec/ticket.
8. No adjacent production-code contradiction was exposed during reassessment. The architecture benefit has already been realized in the shipped tests: the scenarios prove crime/justice behavior through existing state-mediated planner and record surfaces rather than through any new compatibility layer or alias path. This ticket should preserve that clean architecture by only updating the closeout trail.

## Architecture Check

1. The clean architecture here is "source annotations are truth, generated docs are mechanical views, the human dashboard is interpretive backlog only." Updating docs to reflect that is better than hand-maintaining duplicate scenario inventories in multiple prose files.
2. No backwards-compatibility aliasing or shims are introduced. Stale backlog/spec text should be corrected or archived, not preserved beside the shipped path.

## Verification Layers

1. S32 scenarios exist in live golden source and compiled inventory -> `cargo test -p worldwake-ai -- --list` plus `python3 scripts/golden_inventory.py --check-docs`
2. S32 crime-emergence behavior still passes at the golden E2E layer -> `cargo test -p worldwake-ai golden_ -- --nocapture`
3. Generated docs match live source-declared scenario metadata -> `python3 scripts/golden_inventory.py --write --check-docs`
4. Roadmap/dashboard closeout reflects shipped state -> manual inspection of `docs/golden-e2e-coverage.md` and `specs/IMPLEMENTATION-ORDER.md`
5. Single-layer ticket beyond closeout docs/specs; no additional action-trace or authoritative-state mapping is required because no production behavior changes are being made here.

## What to Change

### 1. Correct the human-maintained golden coverage dashboard

- Remove S32 from the pending backlog summary and recommended implementation order in `docs/golden-e2e-coverage.md`
- Add a concise removed-backlog/completed entry that points at the shipped Scenario 41-43 tests instead of leaving S32 as planned work

### 2. Regenerate golden artifacts from live source

Run:

```bash
python3 scripts/golden_inventory.py --write --check-docs
```

This must refresh the checked-in generated files so they include the S32 tests/scenarios and current aggregate counts.

### 3. Close out roadmap/spec state

- Update `specs/IMPLEMENTATION-ORDER.md` to record S32 completion in Phase 3 golden coverage closeout
- Mark `archive/specs/S32-crime-emergence-golden-suites.md` completed and keep it archived under `archive/specs/`
- After the ticket itself is completed, archive it under `archive/tickets/`

## Files to Touch

- `tickets/S32CRIMEMEGOLSUI-004.md` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/generated/golden-scenario-map.md` (regenerate)
- `docs/generated/golden-e2e-inventory.md` (regenerate)
- `docs/generated/golden-coverage-matrix.md` (regenerate)
- `archive/specs/S32-crime-emergence-golden-suites.md` (modified, then archived)
- `specs/IMPLEMENTATION-ORDER.md` (modify)

## Out of Scope

- Any new Rust production-code changes
- Any new S32 golden scenarios beyond the shipped Scenario 41-43 suites
- Reworking the generator architecture; the existing source-annotation -> generated-doc pipeline is already the right long-term model

## Acceptance Criteria

### Tests That Must Pass

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test -p worldwake-ai golden_ -- --nocapture`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `docs/golden-e2e-coverage.md` no longer presents S32 as pending work.
2. Generated golden docs on disk include Scenario 41, Scenario 42, and Scenario 43 from live source annotations.
3. `specs/IMPLEMENTATION-ORDER.md` records S32 as completed closeout work rather than omitting it.
4. S32 ticket and spec are archived with an `Outcome` section that reflects what was actually shipped versus originally planned.

## Test Plan

### New/Modified Tests

1. None in this ticket. Existing shipped S32 tests are the verification surface, and the generated-doc refresh must reflect them accurately.

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test -p worldwake-ai golden_ -- --nocapture`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-03-27
- **What actually changed**: Corrected the ticket to the live shipped state, removed S32 from the pending golden backlog in `docs/golden-e2e-coverage.md`, regenerated the checked-in golden inventory/scenario-map/coverage-matrix docs from source annotations, updated `specs/IMPLEMENTATION-ORDER.md` to record S32 completion, and prepared S32 ticket/spec archival.
- **Deviation from original plan**: The original ticket assumed Scenarios 41-43 still needed implementation and described this as a simple documentation-only follow-up after future test work. Reassessment showed the S32 tests were already implemented and passing in `golden_emergent.rs`, so the real task was closeout and archival rather than new implementation.
- **Verification results**:
  - `python3 scripts/golden_inventory.py --write --check-docs` ✅
  - `cargo test -p worldwake-ai golden_ -- --nocapture` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
