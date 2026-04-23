# SCEROAD-002: Author `docs/scenario-roadmap.md`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None — documentation only.
**Deps**: SCEROAD-001 (Status Summary table and Section 5 retrospective cross-reference the generated companion at `docs/generated/scenario-coverage.md`; Section 3 feature catalog must mirror the binary's `FEATURES` list).

## Problem

After ~15 gameplay features landed with unit-scoped goldens, a 1440-tick observer run revealed the AI architecture could not even sustain basic needs when all features were active (design doc `archive/brainstorming/2026-04-19-scenario-roadmap-doc-design.md`, §Brainstorm Context). The project's philosophy has shifted: goldens are backed by `scenarios/*.ron` observer runs, survival is now a coexistence invariant for every future scenario, and feature stacking happens one-at-a-time in architectural-risk order. No canonical document yet captures that priority order, the entry-contract template, the landed retrospective, the maintenance workflow, or the formal detection rule. Without it, authoring a new scenario is ad-hoc and the survival-always invariant can drift.
Recent S122 follow-through also exposed a second gap: a survival golden can pass or fail for reasons the scenario contract never spelled out, because the scenario materials distinguish structural activation from behavior but do not yet define what evidence makes a scenario golden valid for its intended causal branch. This roadmap doc needs to close that gap, or "Landed" will overclaim what the scenario actually proved.
Live reassessment exposed a third gap in the drafted roadmap boundary: `drive-escalation-wash-priority.ron` is backed by a dedicated golden, but it is not a survival-health-contract scenario and does not currently activate `Drive escalation` in the generated companion because it relies on the universal default `DriveEscalationProfile` rather than an authored non-default override. The roadmap must record that auxiliary-proof state explicitly instead of treating the contested + drive-escalation pair as a single landed scenario-coverage cohort.

## Assumption Reassessment (2026-04-19)

1. All five scenario files live at `scenarios/survival-baseline.ron`, `scenarios/survival-scattered.ron`, `scenarios/survival-contested.ron`, `scenarios/drive-escalation-wash-priority.ron`, and `scenarios/cli-evaluation.ron`. Backing goldens confirmed present for the three survival-contract scenarios plus the auxiliary drive-escalation scenario: `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_scattered.rs`, `golden_survival_contested.rs`, and `golden_drive_escalation_wash_priority.rs`. `cli-evaluation.ron` has no `survival_health_contract` and no golden, intentionally. `drive-escalation-wash-priority.ron` also has no `survival_health_contract`; it is golden-backed auxiliary proof for S116, not a landed survival-roadmap entry.
2. Design doc §3 feature catalog lists ~33 features, each mapped to profile structs confirmed present in `crates/worldwake-core/src/` (see SCEROAD-001 Assumption Reassessment #1). Section 7 gating rules reference fields confirmed present in `UtilityProfile`, `TellProfile`, `MetabolismProfile`, `DriveEscalationProfile`, `PerceptionProfile`, `CommunicationProfile`, and `PlaceVisibilityProfile`.
3. Live SCEROAD-001 output now also surfaces two authored-field warnings in `docs/generated/scenario-coverage.md`: `cli-evaluation.ron` contains `intention_disposition` and `last_seen_memory`, but neither is mapped to a current feature row. This ticket must treat that warning state truthfully: either classify those fields as intentionally outside the gameplay-feature catalog or extend the editorial contract to explain how warning-only authored fields are handled. The roadmap must not imply the generator is wrong simply because those warnings exist.
4. Live SCEROAD-001 output currently reports `Drive escalation` as absent in every scenario. That matches the generator's active rule: a scenario must author a non-default `drive_escalation_profile` to count the feature as active. The roadmap must therefore distinguish structural-landed scenario coverage from auxiliary golden evidence that exercises universal default behavior.
5. This is a documentation-only ticket. Shared abstraction boundary under audit: the gameplay feature catalog and detection appendix, which must stay in lockstep with SCEROAD-001's `FEATURES` table and gating logic. There is no runtime symbol surface to validate beyond reference accuracy.

## Architecture Check

1. **Single editorial source of truth.** `docs/scenario-roadmap.md` is hand-authored intent; `docs/generated/scenario-coverage.md` (from SCEROAD-001) is empirical evidence; CI (SCEROAD-003) enforces lockstep. No third authority. If the doc claims a feature is Landed in a scenario but the generated companion marks it Absent/PresentInactive, one of the two is wrong and CI surfaces the mismatch.
2. **Uniform template for planned and landed entries.** The contract template (design doc §2) is used for both Priority Roadmap (#4–#16) and Landed Scenarios retrospective — one shape, mechanical closure check.
3. **Cumulative "Deliberately inactive" list.** Each entry inherits the prior landed entry's inactive list and flips items explicitly. Reading entry N tells you exactly what is still off, making the feature-stacking rule auditable.
4. **Validity is stronger than activation.** The roadmap entry must distinguish three separate claims: structural activation (from SCEROAD-001), must-exercise behavior, and causal proof in the backing golden. A scenario cannot be marked Landed on outcome-only survival or broad profile coverage alone.
5. **Auxiliary proof is not a landed coverage row.** Golden-backed scenarios without a survival-health contract or without authored feature activation under the SCEROAD-001 rule must be called out separately, not folded into the landed feature summary.
6. **No backwards-compat shim.** First canonical version of this document; no prior roadmap to deprecate.

## Verification Layers

1. Reference accuracy → grep during authoring confirms every path and symbol cited exists; Section 5 golden/scenario pairs are listed above under Assumption Reassessment #1.
2. Section 3 ↔ `FEATURES` lockstep → cross-check during authoring against the `FEATURES` const authored in SCEROAD-001; CI's `--check` (SCEROAD-003) indirectly enforces this via the generated companion.
3. Template compliance → each authored entry must include every field from design doc §2 (Status, Source scenario, Backing goldens, Depends on, Architectural risk rationale, Activation checklist, Must-exercise behaviors, Must-prove invariants, Deliberately inactive, Done-when).
4. Scenario-validity compliance → each landed or planned entry must state the intended invariant, the proof surface, and the accepted/excluded rival lawful branches for the backing golden.
5. Landed-vs-auxiliary compliance → the doc must not report `Drive escalation` as landed through scenario coverage while SCEROAD-001 still reports it absent; instead it must explain the separate auxiliary proof state for `drive-escalation-wash-priority.ron`.
6. Single-layer ticket (documentation): no action-trace or event-log-delta layer applies.

## What to Change

### 1. Create `docs/scenario-roadmap.md` with 7 sections per design doc §1–7

**Section 1 — Preamble / Philosophy.** Scenarios back goldens. Survival-always invariant. Architectural-risk ordering. Feature-stacking rule. One-feature-at-a-time cadence. Cite FOUNDATIONS principles relevant to the philosophy (FND-14 belief-only planning, FND-12 performance may compress computation but not causality, FND-1 maximal emergence through local causality).

**Section 2 — Gameplay Feature Catalog.** Reproduce the feature table from the live `FEATURES` list in SCEROAD-001. This is the doc side of the lockstep with `docs/generated/scenario-coverage.md`; feature names must match exactly. Each row links to the relevant source module(s), the activation signal, and the current roadmap status.

**Section 3 — Status Summary.** Short derived table: feature → first survival-roadmap scenario that landed it, or `Planned`, or explicit auxiliary/CLI-only status where that is the truthful current state. Derive the statuses from the generated companion instead of hand-transcribing from the RON files.

**Section 4 — Priority Roadmap.** State the architectural-risk ordering criterion from design doc §4. Include the contract template from design doc §2 as a subsection, then list the planned scenario entries in order. Because live reassessment disproved the draft assumption that drive escalation is already landed through scenario coverage, the roadmap may insert a planned survival-contract drive-escalation entry ahead of the later social/economic cohorts if needed to keep the ordering truthful.

**Section 5 — Landed Scenarios.** One retrospective entry per landed `.ron`, using the Section 2 template. Map scenarios to goldens exactly as validated in Assumption Reassessment #1:
- Entry #1: `survival-baseline.ron` ↔ `golden_survival_baseline.rs`
- Entry #2: `survival-scattered.ron` ↔ `golden_survival_scattered.rs`
- Entry #3: `survival-contested.ron` ↔ `golden_survival_contested.rs`

Use the seed/agent/place counts and `max_authored_critical_run_ticks`/`critical_run_limits` values from design doc §5, but re-verify each value against the actual RON during authoring — do not copy numbers blindly.

Include the **Auxiliary and non-roadmap scenarios** subsection for:
- `drive-escalation-wash-priority.ron`: golden-backed auxiliary proof for S116, but not a survival-health-contract scenario and not currently counted as `Drive escalation` active by SCEROAD-001.
- `cli-evaluation.ron`: purpose is CLI-command coverage, not a survival-health-contract scenario, not backed by a golden, and must not be interpreted as "feature X is proved".

For the landed survival entries, add one short "Why this golden is valid" paragraph that names the feature-specific invariant beyond survival and the proof surface the golden uses for that invariant.

**Section 6 — Maintenance Workflow.** Reproduce the four procedures from design doc §6: "Adding a new roadmap entry", "Authoring a scenario for a planned entry", "Handling schema drift", and "Closing out an entry". Reference `cargo run -p worldwake-cli --bin scenario-coverage -- --write` for regeneration and `--check` for CI integration.
Make the live warning path explicit: when the generated companion reports authored fields that are not mapped to a gameplay feature row, the roadmap doc must either classify them as intentionally non-feature/editorial exclusions or point to the follow-up that will add them, rather than treating those warnings as unexplained noise.

**Section 7 — Detection Rule Appendix.** Reproduce the formal rule and the gating-fields-per-profile table from design doc §7. Include the world-feature gates subsection. This appendix must match SCEROAD-001's detection logic field-for-field — when that logic changes, this appendix changes in the same PR.

### 2. Cross-references

At the top of the doc, link to `docs/generated/scenario-coverage.md`, `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, and `docs/profiles/all-profiles.md`.

## Files to Touch

- `docs/scenario-roadmap.md` (new)
- `archive/tickets/SCEROAD-002.md` (reassessment + closeout truthing)

## Out of Scope

- Writing new `.ron` scenarios for planned entries #4–#16 (each has its own future ticket at authoring time).
- Authoring new goldens.
- Changes to SCEROAD-001's binary or detection logic.
- Authoring a new survival-baseline golden (already exists: `golden_survival_baseline.rs`).
- Any modification to `scenarios/*.ron`.
- Changes to `docs/generated/scenario-coverage.md` — that file is owned by SCEROAD-001 + the generator.
- Changing runtime test code; this ticket defines the documentation contract those future goldens must satisfy.

## Acceptance Criteria

### Tests That Must Pass

1. Documentation-only ticket: content review + cross-ref checks. No automated tests added.
2. Manual cross-check: every Landed entry in Section 5 references a concrete `scenarios/*.ron` and a concrete `crates/worldwake-ai/tests/golden_*.rs`, and those files exist.
3. Manual cross-check: Section 3 Feature Catalog row count matches SCEROAD-001's `FEATURES` entry count; feature names match exactly.
4. Manual cross-check: Section 7 gating-fields table matches SCEROAD-001's gating logic field-for-field.
5. Manual cross-check: every planned and landed scenario entry names a feature-specific invariant beyond "agents survive", identifies the proof surface, and states the accepted or excluded rival lawful branches.
6. Manual cross-check: any warning rows currently emitted by `docs/generated/scenario-coverage.md` are accounted for explicitly in the roadmap doc as either intentional exclusions from the gameplay-feature catalog or concrete follow-up work.
7. Manual cross-check: `drive-escalation-wash-priority.ron` is described truthfully as auxiliary golden evidence rather than as a landed feature-coverage row while SCEROAD-001 still reports `Drive escalation` absent.
8. Existing suite: `./scripts/verify.sh` remains green (no code changes, but confirms the doc-only change didn't break lints that reach into `docs/`).

### Invariants

1. Every Landed entry in Section 5 has a concrete Source scenario path and a concrete Backing goldens path.
2. Section 3 catalog and SCEROAD-001's `FEATURES` list share exactly the same set of feature names (single editorial/evidence pair).
3. Section 7 appendix and SCEROAD-001's detection logic agree on every gating field.
4. "Deliberately inactive" list in each entry is cumulative from the prior entry plus anything this entry still zeros out.
5. No entry treats profile activation or outcome-only survival as sufficient proof that the feature is validly covered.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo run -p worldwake-cli --bin scenario-coverage -- --check` — confirms the generated companion agrees with committed state; indirect cross-check on Section 3 mirror.
2. `rg -n '^###? ' docs/scenario-roadmap.md` — visual inspection of section structure vs design doc §1–7.
3. `./scripts/verify.sh` — full workspace verification.

## Outcome

Completed on 2026-04-22.

- Added `docs/scenario-roadmap.md` as the canonical editorial companion to `docs/generated/scenario-coverage.md`.
- Rebased the roadmap structure to the truthful live boundary: three landed survival-contract scenarios, an explicit auxiliary subsection for `drive-escalation-wash-priority.ron`, and an explicit CLI-only subsection for `cli-evaluation.ron`.
- Recorded the live SCEROAD-001 warning state for `intention_disposition` and `last_seen_memory` as intentional unresolved catalog gaps rather than silently treating the generator as wrong.
- Inserted a planned survival-contract `drive escalation` roadmap row because the live generator still reports `Drive escalation` absent in every scenario.

## Deviations

- The April 19 design/ticket draft treated `survival-contested.ron` plus `drive-escalation-wash-priority.ron` as one landed cohort. Live reassessment disproved that: `drive-escalation-wash-priority.ron` has no `survival_health_contract` and does not structurally activate `Drive escalation` under the current generator rule.
- The final roadmap therefore uses the design doc's 7-section outline, but moves the entry contract template under the Priority Roadmap section and adds the landed-vs-auxiliary distinction the draft was missing.

## Verification Result

- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`
- Passed `rg -n '^## |^### ' docs/scenario-roadmap.md`
- Passed `./scripts/verify.sh`
