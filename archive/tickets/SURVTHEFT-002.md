# SURVTHEFT-002: Author a truthful `survival-theft` scenario and golden substrate

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - authored scenario substrate, scenario-backed golden proof, and any required world-model support exposed by reassessment
**Deps**: docs/scenario-roadmap.md row 12 (`survival-theft`), archive/tickets/SURVTHEFT-001.md

## Problem

Roadmap row 12 still lacks a truthful causal world where theft emerges as the surviving local branch and leaves durable aftermath. The AI-side blocker is fixed, but the row still has no stable authored seam for `stage_stock_for_sale -> theft -> post-theft eat -> concealment-limited witness aftermath`, so marking `survival-theft` landed would overstate the current world model.

## Assumption Reassessment (2026-04-24)

1. [SURVTHEFT-001](/home/joeloverbeck/projects/worldwake/archive/tickets/SURVTHEFT-001.md) repaired the AI-side theft contract in [goal_dispatch_decl.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs), [goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs), and [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). That ticket does not prove roadmap row 12.
2. [docs/scenario-roadmap.md](/home/joeloverbeck/projects/worldwake/docs/scenario-roadmap.md) still keeps row 12 at `Drafting`, which is truthful because no authored `scenarios/survival-theft.ron` or `golden_survival_theft.rs` currently lands the row.
3. Shared boundary under audit: the authored world substrate required for a stable theft branch across merchandise staging, ownership/visibility, local lawful-food exclusion, post-theft self-consumption, and witness/evidence aftermath under concealment.
4. Intended invariant: when a merchant stages visible owned food locally, a hungry thief with no lawful local food branch can theft-plan, commit a real `steal`, later consume the stolen food for the authored causal reason, and leave only the concealment-limited witness/evidence chain that the world state lawfully supports.
5. The live goal family under the row remains `GoalKind::StealItem` plus the downstream `Eat` / self-consume seam. The row also depends on `stage_stock_for_sale`, displayed-lot ownership visibility, perception/locality, and aftermath carriers that later justice rows can lawfully use.
6. This is a roadmap-owned mixed-layer ticket. Focused AI proof is not sufficient because the missing contract is the authored causal world itself, not only candidate emission or ranking.
7. The competing branches are not symmetric. If trade, harvest, forage, or another lawful self-care branch remains available, theft is no longer the truthful owner of the row. The scenario must isolate those branches by concrete world state, not by invisible script logic.
8. The hidden heuristic to avoid is "force theft somehow." This ticket must add or author the missing substrate that makes theft locally rational without introducing scene-only exception logic or weakening existing lawful branches globally.
9. The first failed boundary in the abandoned implementation attempt was scenario truth, not action validity: the world could not stably sustain the full staged-lot theft -> eat -> aftermath seam without rival branches or missing witness/evidence support.
10. Under [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), the row must satisfy at least FND-1, FND-4, FND-7, FND-8, FND-15, FND-17, FND-18, FND-20, and FND-21: local causality, explicit transfer, locality of information, explicit action costs/contention, provenance-bearing knowledge, expectation-based absence, world-state evidence, bounded practical reasoning, and revisable intentions.
11. The scenario cannot rely on omniscient theft detection. Witnesses or later investigators must learn through direct perception, expected-stock mismatch, physical aftermath, or transmitted evidence, and only after those carriers lawfully exist.
12. Adjacent contradictions exposed here should be classified carefully: merchant staging motivation, thief lawful-food isolation, and concealment witness/evidence support are in scope when they are required to make row 12 truthful; broader justice/patrol follow-on behavior stays out of scope.
13. Mismatch + correction: the original [SURVTHEFT-001](/home/joeloverbeck/projects/worldwake/archive/tickets/SURVTHEFT-001.md) draft treated scenario/golden landing as a tail task after the AI fix. Reassessment proved that this remainder is a distinct authored-substrate ticket and may require additional production code before the row can land.

## Architecture Check

1. Splitting the AI repair from the authored-substrate work keeps each proof seam honest. This ticket can now focus on building the minimal truthful world contract for theft emergence instead of forcing a brittle scenario to masquerade as a planner fix.
2. No backward-compatibility shims should be introduced. If the row needs new substrate, it should be added as the canonical world-model path that future justice/patrol rows also rely on.

## Verification Layers

1. Merchant stages vulnerable visible stock with explicit ownership and accessibility -> authored scenario state + action trace / authoritative world state
2. No rival lawful food branch owns the row -> decision trace plus authored scenario audit of excluded branches
3. The thief selects and commits a real local theft branch -> decision trace + action trace
4. Post-theft self-consumption happens from the stolen goods for the authored causal reason -> authoritative world state + later decision/action trace
5. Concealment limits immediate witness knowledge while preserving lawful aftermath carriers -> perception trace + social/evidence world state
6. Roadmap row 12 is landed only when the scenario-backed golden proves the full seam above and the generated/docs inventory stays synchronized

## What to Change

### 1. Author the missing theft substrate

Reassess and implement the minimal concrete world conditions needed for row 12: why the merchant stages stealable food, why the thief lacks lawful competing food acquisition, and what aftermath carriers remain after the theft under concealment.

### 2. Land the roadmap-owned proof surface

Once the world contract is truthful, add `scenarios/survival-theft.ron`, `crates/worldwake-ai/tests/golden_survival_theft.rs`, and any required workflow/generated-doc wiring so row 12 can move from `Drafting` to `Landed`.

## Files to Touch

- `scenarios/survival-theft.ron` (new)
- `crates/worldwake-ai/tests/golden_survival_theft.rs` (new)
- `.github/workflows/golden-survival.yml` (modify, if the scenario lands)
- `docs/scenario-roadmap.md` (modify)
- `docs/generated/*` or generator inputs touched by the new golden inventory/docs sync (modify, if required)
- Production code under the exact world-model boundary exposed by reassessment (modify only if the authored seam cannot be made truthful with existing substrate)

## Out of Scope

- Row 13 `survival-justice` or row 14 `survival-patrol` full landings
- Broad theft-economy redesign unrelated to the concrete row-12 seam
- Omniscient theft detection or hidden scripted triggers that bypass local evidence / witness acquisition

## Acceptance Criteria

### Tests That Must Pass

1. A scenario-backed golden proves `stage_stock_for_sale -> theft -> post-theft eat -> concealment-limited witness aftermath` for the authored causal reason.
2. Any new lower-layer tests required by reassessment prove newly added substrate or repaired world-model support.
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The row may be marked `Landed` only when theft is the truthful surviving local branch in the authored scenario, not when some rival lawful branch can explain the outcome.
2. Knowledge of the theft and its aftermath must travel through local perception, expected-stock mismatch, evidence, or explicit transmission carriers, never through global truth shortcuts.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_theft.rs` - prove the roadmap-owned staged-lot theft, post-theft self-consumption, and concealment-limited aftermath seam.
2. Focused lower-layer tests at the exact substrate changed during reassessment - prove any new ownership, perception, aftermath, or staging support required to make the scenario truthful.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `python3 scripts/golden_inventory.py --write --check-docs` once the golden lands and docs/generated files need sync

## Outcome

- 2026-04-24
- Added the authored [survival-theft scenario](/home/joeloverbeck/projects/worldwake/scenarios/survival-theft.ron) and new roadmap golden [golden_survival_theft.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_theft.rs) so row 12 now lands as a real `stage_stock_for_sale -> StealItem -> steal -> eat` survival branch under authored concealment.
- Reassessment exposed two production gaps that had to be fixed for the scenario seam to be truthful: planner synthetic steal transitions were incorrectly rejecting contained displayed lots in [planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), and scenario-spawned items authored on agents needed lawful ownership as well as possession in [scenario/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/mod.rs). Focused regression coverage was added in [search/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs) and the existing CLI spawn test was extended at the same seam.
- The roadmap/workflow handoff was completed by updating [golden-survival.yml](/home/joeloverbeck/projects/worldwake/.github/workflows/golden-survival.yml), [docs/scenario-roadmap.md](/home/joeloverbeck/projects/worldwake/docs/scenario-roadmap.md), and regenerated generated companions under [docs/generated](/home/joeloverbeck/projects/worldwake/docs/generated).
- Deviation from the draft: the landed scenario needed substantially larger staged apple stock plus higher thief thirst pressure so the 1440-tick survival contract remained truthful while still forcing the theft-owned local food branch; no justice or patrol follow-on behavior was pulled into this ticket.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_survival_theft survival_theft_proves_concealed_staged_lot_branch -- --ignored --exact --test-threads=1`
  - `cargo test -p worldwake-ai --lib search::tests::steal_goal_plans_for_contained_displayed_sale_lot_without_owner_belief -- --exact`
  - `cargo test -p worldwake-ai --lib search::tests::steal_goal_surfaces_search_candidates_after_action_lands -- --exact`
  - `cargo test -p worldwake-cli scenario::tests::test_spawn_items_on_agent -- --exact`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo run -p worldwake-cli --bin scenario-coverage -- --write`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
