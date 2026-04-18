# SCEROAD-002: Author `docs/scenario-roadmap.md`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None — documentation only.
**Deps**: SCEROAD-001 (Status Summary table and Section 5 retrospective cross-reference the generated companion at `docs/generated/scenario-coverage.md`; Section 3 feature catalog must mirror the binary's `FEATURES` list).

## Problem

After ~15 gameplay features landed with unit-scoped goldens, a 1440-tick observer run revealed the AI architecture could not even sustain basic needs when all features were active (design doc `docs/plans/2026-04-19-scenario-roadmap-doc-design.md`, §Brainstorm Context). The project's philosophy has shifted: goldens are backed by `scenarios/*.ron` observer runs, survival is now a coexistence invariant for every future scenario, and feature stacking happens one-at-a-time in architectural-risk order. No canonical document yet captures that priority order, the entry-contract template, the landed retrospective, the maintenance workflow, or the formal detection rule. Without it, authoring a new scenario is ad-hoc and the survival-always invariant can drift.

## Assumption Reassessment (2026-04-19)

1. All five scenario files live at `scenarios/survival-baseline.ron`, `scenarios/survival-scattered.ron`, `scenarios/survival-contested.ron`, `scenarios/drive-escalation-wash-priority.ron`, `scenarios/cli-evaluation.ron`. Backing goldens confirmed present: `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_scattered.rs`, `golden_survival_contested.rs`, `golden_drive_escalation_wash_priority.rs`. `cli-evaluation.ron` has no `survival_health_contract` and no golden, intentionally — this resolves the spec's two "Open items for authoring time" (trace landed scenarios to backing goldens, confirm `survival-baseline.ron` has a backing observer-level golden).
2. Design doc §3 feature catalog lists ~33 features, each mapped to profile structs confirmed present in `crates/worldwake-core/src/` (see SCEROAD-001 Assumption Reassessment #1). Section 7 gating rules reference fields confirmed present in `UtilityProfile`, `TellProfile`, `MetabolismProfile`, `DriveEscalationProfile`, `PerceptionProfile`, `CommunicationProfile`, and `PlaceVisibilityProfile`.
3. This is a documentation-only ticket. Shared abstraction boundary under audit: the Section 3 feature catalog, which must stay in lockstep with SCEROAD-001's `FEATURES` table. There is no runtime symbol surface to validate beyond reference accuracy.

## Architecture Check

1. **Single editorial source of truth.** `docs/scenario-roadmap.md` is hand-authored intent; `docs/generated/scenario-coverage.md` (from SCEROAD-001) is empirical evidence; CI (SCEROAD-003) enforces lockstep. No third authority. If the doc claims a feature is Landed in a scenario but the generated companion marks it Absent/PresentInactive, one of the two is wrong and CI surfaces the mismatch.
2. **Uniform template for planned and landed entries.** The contract template (design doc §2) is used for both Priority Roadmap (#4–#16) and Landed Scenarios retrospective — one shape, mechanical closure check.
3. **Cumulative "Deliberately inactive" list.** Each entry inherits the prior landed entry's inactive list and flips items explicitly. Reading entry N tells you exactly what is still off, making the feature-stacking rule auditable.
4. **No backwards-compat shim.** First canonical version of this document; no prior roadmap to deprecate.

## Verification Layers

1. Reference accuracy → grep during authoring confirms every path and symbol cited exists; Section 5 golden/scenario pairs are listed above under Assumption Reassessment #1.
2. Section 3 ↔ `FEATURES` lockstep → cross-check during authoring against the `FEATURES` const authored in SCEROAD-001; CI's `--check` (SCEROAD-003) indirectly enforces this via the generated companion.
3. Template compliance → each authored entry must include every field from design doc §2 (Status, Source scenario, Backing goldens, Depends on, Architectural risk rationale, Activation checklist, Must-exercise behaviors, Must-prove invariants, Deliberately inactive, Done-when).
4. Single-layer ticket (documentation): no action-trace or event-log-delta layer applies.

## What to Change

### 1. Create `docs/scenario-roadmap.md` with 7 sections per design doc §1–7

**Section 1 — Preamble / Philosophy.** Scenarios back goldens. Survival-always invariant. Architectural-risk ordering. Feature-stacking rule. One-feature-at-a-time cadence. Cite FOUNDATIONS principles relevant to the philosophy (FND-14 belief-only planning, FND-26 system decoupling, FND-1 maximal emergence through local causality).

**Section 2 — Entry Contract Template.** Reproduce the template verbatim from design doc §2 so authors copy-paste it when adding entries. Include the two design notes: cumulative "Deliberately inactive" and behavioral-not-structural "Must-exercise".

**Section 3 — Gameplay Feature Catalog.** Reproduce the table from design doc §3 (~33 rows). Columns: Feature | Activation signal | Backing systems. This table IS the doc's side of the lockstep with SCEROAD-001's `FEATURES` constant — every row must correspond 1:1 in name and gating fields. Each row links to the relevant spec (if any), the module source file, and — once a scenario lands — the roadmap entry that first activates it.

**Section 4 — Priority Roadmap.** State the architectural-risk ordering criterion verbatim from design doc §4. Provide the cohort table (#1–#16) from design doc §4; mark #1–#3 as Landed with pointers to Section 5 retrospective entries; author full contract-template entries for #4–#16 at the Planned status level (Source scenario `—`, Backing goldens `—`, status `Planned`) so a future author can pick one up directly.

**Section 5 — Landed Scenarios.** One retrospective entry per landed `.ron`, using the Section 2 template. Map scenarios to goldens exactly as validated in Assumption Reassessment #1:
- Entry #1: `survival-baseline.ron` ↔ `golden_survival_baseline.rs`
- Entry #2: `survival-scattered.ron` ↔ `golden_survival_scattered.rs`
- Entry #3: `survival-contested.ron` + `drive-escalation-wash-priority.ron` ↔ `golden_survival_contested.rs` + `golden_drive_escalation_wash_priority.rs`

Use the seed/agent/place counts and `max_authored_critical_run_ticks`/`critical_run_limits` values from design doc §5, but re-verify each value against the actual RON during authoring — do not copy numbers blindly.

Include the **Non-golden-backed scenarios** subsection for `cli-evaluation.ron` with the three points from design doc §5: purpose is CLI-command coverage, not a survival-health-contract scenario, not backed by a golden, and do NOT interpret CLI-evaluation's broad profile coverage as "feature X is proved" — feature proofs require a survival-health-contract scenario with a backing golden.

**Section 6 — Maintenance Workflow.** Reproduce the three procedures from design doc §6: "Adding a new roadmap entry", "Authoring a scenario for a planned entry", "Handling schema drift", "Closing out an entry". Reference `cargo run -p worldwake-cli --bin scenario-coverage -- --write` for regeneration and `--check` for CI integration.

**Section 7 — Detection Rule Appendix.** Reproduce the formal rule and the gating-fields-per-profile table from design doc §7. Include the world-feature gates subsection. This appendix must match SCEROAD-001's detection logic field-for-field — when that logic changes, this appendix changes in the same PR.

### 2. Add the "Status Summary" table (design doc §1 item 3)

Short derived table: feature → first scenario that landed it (or "Planned — see row N"). Derive from the generated companion at `docs/generated/scenario-coverage.md` (from SCEROAD-001) — do not hand-compile from RON.

### 3. Cross-references

At the top of the doc, link to `docs/generated/scenario-coverage.md`, `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, and `docs/profiles/all-profiles.md`.

## Files to Touch

- `docs/scenario-roadmap.md` (new)

## Out of Scope

- Writing new `.ron` scenarios for planned entries #4–#16 (each has its own future ticket at authoring time).
- Authoring new goldens.
- Changes to SCEROAD-001's binary or detection logic.
- Authoring a new survival-baseline golden (already exists: `golden_survival_baseline.rs`).
- Any modification to `scenarios/*.ron`.
- Changes to `docs/generated/scenario-coverage.md` — that file is owned by SCEROAD-001 + the generator.

## Acceptance Criteria

### Tests That Must Pass

1. Documentation-only ticket: content review + cross-ref checks. No automated tests added.
2. Manual cross-check: every Landed entry in Section 5 references a concrete `scenarios/*.ron` and a concrete `crates/worldwake-ai/tests/golden_*.rs`, and those files exist.
3. Manual cross-check: Section 3 Feature Catalog row count matches SCEROAD-001's `FEATURES` entry count; feature names match exactly.
4. Manual cross-check: Section 7 gating-fields table matches SCEROAD-001's gating logic field-for-field.
5. Existing suite: `./scripts/verify.sh` remains green (no code changes, but confirms the doc-only change didn't break lints that reach into `docs/`).

### Invariants

1. Every Landed entry in Section 5 has a concrete Source scenario path and a concrete Backing goldens path.
2. Section 3 catalog and SCEROAD-001's `FEATURES` list share exactly the same set of feature names (single editorial/evidence pair).
3. Section 7 appendix and SCEROAD-001's detection logic agree on every gating field.
4. "Deliberately inactive" list in each entry is cumulative from the prior entry plus anything this entry still zeros out.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo run -p worldwake-cli --bin scenario-coverage -- --check` — confirms the generated companion agrees with committed state; indirect cross-check on Section 3 mirror.
2. `rg -n '^###? ' docs/scenario-roadmap.md` — visual inspection of section structure vs design doc §1–7.
3. `./scripts/verify.sh` — full workspace verification.
