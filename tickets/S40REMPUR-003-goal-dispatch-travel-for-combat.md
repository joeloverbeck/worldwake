# S40REMPUR-003: Add Travel to combat goal relevant ops

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Deps**: None (independent structural change)

## Problem

`ENGAGE_HOSTILE_OPS` and `RAID_TARGET_OPS` in `goal_dispatch_decl.rs:69-70` currently contain only `[PlannerOpKind::Attack]`. Without `PlannerOpKind::Travel` in the relevant-ops list, the planner search cannot produce `Travel + Attack` plans for remote targets. Every other goal that requires reaching a remote place (e.g., `LOOT_OPS`, `SLEEP_OPS`, `SELL_OPS`) already includes `PlannerOpKind::Travel`.

## Assumption Reassessment (2026-03-30)

1. Current declarations at `goal_dispatch_decl.rs:69-70`:
   ```rust
   const ENGAGE_HOSTILE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Attack];
   const RAID_TARGET_OPS: &[PlannerOpKind] = &[PlannerOpKind::Attack];
   ```
2. These are referenced at `goal_dispatch_decl.rs:181` (EngageHostile) and `goal_dispatch_decl.rs:188` (RaidTarget).
3. Adding `Travel` follows the exact pattern used by `LOOT_OPS` (`goal_dispatch_decl.rs:107`): `&[PlannerOpKind::Travel, PlannerOpKind::Loot]`.
4. `goal_relevant_places()` for `EngageHostile` and `RaidTarget` already returns `state.effective_place(*target)` (`goal_model.rs:1005-1008`). Post-E14, when the target is believed remote, this returns the believed place. The heuristic already guides search toward that place — no change needed there.
5. `synthesized_root_candidate_targets()` for `Attack` returns `NoSynthesisPath` for combat goals (`goal_model.rs:~1733`). This must NOT change — attack targets come from candidate generation, not synthesis. The spec explicitly requires this.
6. Existing tests that check plan shapes for co-located combat must still pass. Adding `Travel` to the op list does not break co-located plans; search simply finds a zero-hop solution.
7. No adjacent contradictions exposed.

## Architecture Check

1. This is the minimal structural change needed: one line per constant. The planner's prerequisite-aware search already handles `Travel + terminal` sequences. No new search logic is required.
2. No backwards-compatibility shims. The old `Attack`-only list was a constraint that prevented remote plans; relaxing it is the correct fix.

## Verification Layers

1. Op list correctness → compile-time verification (constant declaration) + existing search tests
2. Co-located combat still works → existing golden/focused tests (no regression)
3. Remote combat search produces `Travel + Attack` → new focused search test (S40REMPUR-004 adds the candidate generation; this ticket enables the search path)
4. `NoSynthesisPath` preserved → existing test: `synthesized_root_candidate_targets` for Attack returns `NoSynthesisPath`
5. Single-layer ticket (constant change); cross-system impact is limited to search expansion.

## What to Change

### 1. Update op lists in `goal_dispatch_decl.rs`

```rust
const ENGAGE_HOSTILE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Attack];
const RAID_TARGET_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Attack];
```

That's the entire code change.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify) — two constant declarations

## Out of Scope

- Candidate generation changes (S40REMPUR-004)
- `PursuitProfile` or belief helper (S40REMPUR-001, S40REMPUR-002)
- `goal_relevant_places()` changes (none needed per spec — already returns believed place)
- `synthesized_root_candidate_targets()` changes (none needed — must remain `NoSynthesisPath`)
- Any new `PlannerOpKind` variants
- Invalidation logic (S40REMPUR-005)
- Decision trace changes (S40REMPUR-006)

## Acceptance Criteria

### Tests That Must Pass

1. `ENGAGE_HOSTILE_OPS` contains both `Travel` and `Attack`.
2. `RAID_TARGET_OPS` contains both `Travel` and `Attack`.
3. `synthesized_root_candidate_targets()` for `Attack` on combat goals still returns `NoSynthesisPath`.
4. All existing AI golden and focused tests pass: `cargo test -p worldwake-ai`

### Invariants

1. `Attack` remains a lawful same-place terminal affordance — no synthesized combat alias path.
2. Co-located combat plans are unaffected (search finds zero-hop solution).
3. Goal-relevant-places logic unchanged.

## Test Plan

### New/Modified Tests

1. None — this is a two-line constant change. Verification is through existing test suite plus the focused search tests added in S40REMPUR-004.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai && cargo test --workspace`
