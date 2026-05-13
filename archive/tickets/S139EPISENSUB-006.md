# S139EPISENSUB-006: Golden coverage for AskWitness and observer trace audit

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — adds golden coverage and fixes AskWitness cold-start emission plus feasibility integration
**Deps**: archive/tickets/S139EPISENSUB-001.md, archive/tickets/S139EPISENSUB-002.md, archive/tickets/S139EPISENSUB-003.md, archive/tickets/S139EPISENSUB-004.md, archive/tickets/S139EPISENSUB-005.md, archive/tickets/S139EPISENSUB-007.md

## Problem

Before this ticket, the full S139 AskWitness pipeline was not exercised end-to-end. The missing proof covered candidate emission, ranking/selection, plan search, action execution, belief import, satisfaction, cooldown, stress suppression, relocation revalidation, generated golden documentation, and the observer rendering path.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `docs/golden-e2e-testing.md` is the canonical golden-guide reference for the landed test metadata and generated docs.
2. `archive/specs/S139-epistemic-sensing-subgoals.md` already described cold-start AskWitness as in-scope, and archived `S139EPISENSUB-004` explicitly deferred cold-start local-witness emission to this ticket if the golden needed it.
3. The live shared boundary is the AskWitness goal layer from `emit_ask_witness_candidates` through `GoalKind::AskWitness` selection, `PlannerOpKind::AskWitness`, existing `ask_witness` action execution, `AgentBeliefStore` report-provenance writes, cooldown memory, stress suppression, and action revalidation.
4. Reassessment found two production gaps rather than a test-only ticket: cold-start local witnesses were not emitted unless the seeker already had report provenance from that exact witness, and `FeasibilityStrategy::ColocationOrDead` had no `GoalKind::AskWitness` arm.
5. The draft remote-travel relocation scenario was not the live S139 boundary. S139 emits co-located witness inquiries only; the landed relocation golden proves revalidation when a co-located witness moves after plan selection and before ask commit.
6. Observer audit required no code change: `observer.rs` summarizes decision payloads through debug rendering, and `display.rs` already has a `GoalKind::AskWitness` display arm.

## Outcome

Completed on 2026-05-13.

- Added `crates/worldwake-ai/tests/golden_epistemic_sensing.rs` with six focused tests: stale report refresh, deterministic replay, cold-start local witness import, critical-survival suppression, cooldown expiry/resume, and witness relocation revalidation.
- Extended `emit_ask_witness_candidates` so low-confidence non-report topics can emit AskWitness candidates for co-located local witnesses while preserving the existing report-from-witness branch, cooldown gate, threshold gate, per-topic cap, and testimony provenance trace.
- Added the missing AskWitness feasibility arm for `FeasibilityStrategy::ColocationOrDead`, closing the ranking/search integration panic discovered by the new golden.
- Regenerated golden inventory artifacts, including `docs/generated/golden-scenario-details/epistemic-sensing.md`.
- Confirmed the observer surface required no edit because existing payload/debug rendering and `GoalKind::AskWitness` display coverage already render the variant.

## Deviations

- The draft asked for six documented scenarios including a disputed Scenario G chain. The landed file has five documented scenario blocks plus one replay determinism test. Scenario G contradiction/disputed-envelope behavior is not forced into this ticket because the current S139 owned seam is AskWitness sensing, not downstream lie/dispute adjudication.
- The draft plan-failure sketch described traveling to a witness's last-known place. The live emitter is intentionally co-location-only, so the landed golden proves relocation after selected AskWitness plan and before action commit.
- The ticket became a production-fix ticket because the new golden exposed cold-start emission and feasibility gaps.
- Existing broad survival-golden regression was not run separately in this session. The focused S139 golden plus inventory were run; broader workspace proof remains for pre-PR verification.

## Touched Files

- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/feasibility.rs`
- `crates/worldwake-ai/tests/golden_epistemic_sensing.rs`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-scenario-details/epistemic-sensing.md`
- `archive/specs/S139-epistemic-sensing-subgoals.md`

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_epistemic_sensing`
- Passed `cargo test -p worldwake-ai --lib ask_witness_emitter_emits_cold_start_for_low_confidence_topic_and_local_witness`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo fmt --all`
- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py archive/tickets/S139EPISENSUB-006.md`
- Passed `git diff --check`
