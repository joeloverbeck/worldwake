# S117CONMAIOBS-008: Scenario-analysis skill documentation — graduate proposed smells to shipped

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None
**Deps**: `archive/tickets/S117CONMAIOBS-007.md`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

The `/scenario-analysis` skill's reference files describe the observer's detection surface for analysts. After S117 lands, four detectors that the skill previously enumerated as "proposed smells 11/12/13" (plus a fourth for `ACUTE_NEED_SPIKE`) ship as mechanical detectors — the skill's reference text is now stale. Leaving the stale text in place produces two hazards: (a) analysts following Layer 3 keep proposing smells that are already shipped; (b) the "Known Pathology Signatures" list in Layer 1 omits the new mechanical signatures, so analysts miss them when reading dumps. This ticket updates the three skill reference files to reflect the shipped behavior.

## Assumption Reassessment (2026-04-18)

1. The skill's entry point `.claude/skills/scenario-analysis/SKILL.md` does NOT contain the subsections to update — it is a thin entry point that loads references. Confirmed during S117 reassessment. The content lives in three reference files.
2. `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` contains "Known Pathology Signatures" at line 41 — this is where mechanical-detector signatures are listed.
3. `.claude/skills/scenario-analysis/references/layer-3-meta-analysis.md` contains "Step 6.4: Proposed New Smell Categories" at line 79 — this is where the graduation note lands.
4. `.claude/skills/scenario-analysis/references/report-templates.md` contains a "Proposed New Smell Categories" section at line 121 — documentation template; may benefit from a tightening edit but is not strictly required.
5. Shared abstraction boundary under audit: the /scenario-analysis skill's documented detector-surface surface area — purely documentation, no code consumer.
6. This is a documentation-only ticket; `Engine Changes: None` is honest. No simulation code, no observer code, no tests are modified.

## Architecture Check

1. Documentation and code stay in sync: the mechanical detectors in `bin/observer.rs` are the authority; the skill's reference files describe that authority for analyst consumption. This ticket preserves that relationship.
2. No backward-compatibility shim — the skill references are overwritten to match the new shipped behavior. Prior archived versions of the skill (in git history) remain accurate for their epoch.

## Verification Layers

1. Reference files reflect S117's shipped detectors → human review of the edited files plus a grep confirming the new detector labels (`GEOGRAPHIC_CONVERGENCE`, `MAINTENANCE_STARVATION`, `RECIPE_MONOCULTURE`, `ACUTE_NEED_SPIKE`) appear in Known Pathology Signatures and that the "Proposed New Smell Categories" template notes smells 11/12/13 as graduated.
2. Documentation-only ticket; no test, no action-trace, no event-log proof surface applies.

## What to Change

### 1. Update `layer-1-behavioral-smells.md`

In the "Known Pathology Signatures" section (around line 41), add new entries for the four shipped mechanical detectors:

- **Geographic Convergence** — 2+ agents anchored on a single place for ≥60% of a 200-tick window. Mechanical label: `GEOGRAPHIC_CONVERGENCE`. Investigate: is this a legitimate trade-hub pattern, or is agent rotation broken?
- **Maintenance Starvation** — per-agent per-need accumulation outpaces relief over a 200-tick window with mean above medium threshold. Mechanical label: `MAINTENANCE_STARVATION`. Investigate: where is the relief facility, and is travel-distance cost blocking cadence?
- **Recipe Monoculture** — ≥95% of an agent's need-category actions concentrate on one recipe despite ≥2 known and belief-reachable alternatives. Mechanical label: `RECIPE_MONOCULTURE`. Investigate: is the unused recipe's facility genuinely unreachable, or is it a ranking bug?
- **Acute Need Spike** — need stays at or above the agent's critical threshold for 30–99 consecutive ticks (sub-threshold of SUSTAINED_CRITICAL_NEED). Mechanical label: `ACUTE_NEED_SPIKE`. Investigate: is the agent's tolerance margin narrowing?

Match the existing signature-entry format in that file (each entry pattern).

### 2. Update `layer-3-meta-analysis.md`

In "Step 6.4: Proposed New Smell Categories" (around line 79), add a leading note before the proposal template:

> **Graduated smells (shipped as of S117)**: Smells 11 (Geographic Convergence), 12 (Maintenance Starvation), and 13 (Recipe Monoculture) are now mechanical detectors in the observer — do not propose them again. Sub-threshold acute spikes (the fourth S117 detector, `ACUTE_NEED_SPIKE`) is also shipped. If a future report surfaces a detection gap beyond the existing mechanical suite, propose it as a new spec or a successor to S117, not as a re-proposal of the already-shipped smells.

### 3. Review `report-templates.md` (optional tightening)

In the "Proposed New Smell Categories" section (around line 121), review the template phrasing. If the template implies smells 11/12/13 are still proposals, tighten it to point readers at `layer-3-meta-analysis.md` for the graduation note. Otherwise leave untouched.

### 4. No spec/ticket commit at the end

Per skill convention — leave changes for user review.

## Files to Touch

- `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` (modify)
- `.claude/skills/scenario-analysis/references/layer-3-meta-analysis.md` (modify)
- `.claude/skills/scenario-analysis/references/report-templates.md` (modify — optional)

## Out of Scope

- Any change to `.claude/skills/scenario-analysis/SKILL.md` (thin entry point; no subsection-level content).
- Any change to Layer 2 (needs diagnostics) references — unaffected by S117.
- Adding new layer templates or restructuring the skill's three-layer architecture.
- Observer code, tests, or simulation state.

## Acceptance Criteria

### Tests That Must Pass

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Invariants

1. The four new detector labels (`GEOGRAPHIC_CONVERGENCE`, `MAINTENANCE_STARVATION`, `RECIPE_MONOCULTURE`, `ACUTE_NEED_SPIKE`) appear in `layer-1-behavioral-smells.md` after this ticket lands.
2. `layer-3-meta-analysis.md` contains an explicit "graduated" note so future Layer 3 output does not re-propose shipped smells.
3. `.claude/skills/scenario-analysis/SKILL.md` is NOT modified — the thin entry point stays thin.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `grep -c "GEOGRAPHIC_CONVERGENCE\|MAINTENANCE_STARVATION\|RECIPE_MONOCULTURE\|ACUTE_NEED_SPIKE" .claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` — expect `>= 4` (one match per label minimum).
2. `grep -n "Graduated smells\|shipped as of S117" .claude/skills/scenario-analysis/references/layer-3-meta-analysis.md` — expect one match.
3. `diff <(git show HEAD:.claude/skills/scenario-analysis/SKILL.md) .claude/skills/scenario-analysis/SKILL.md` — expect empty (no changes to the entry point).
