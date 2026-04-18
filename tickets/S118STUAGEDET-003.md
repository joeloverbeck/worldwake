# S118STUAGEDET-003: Simplify stuck-agent detector caveat in scenario-analysis skill

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — skill-documentation-only edit.
**Deps**: archive/tickets/S118STUAGEDET-001.md, specs/S118-stuck-agent-detector-active-frame-exclusion.md

## Problem

The `scenario-analysis` skill's Smell-3 guidance at `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md:21` currently carries an expanded "Detector caveat" paragraph added by the 2026-04-17 skill-audit. It warns that the mechanical stuck-agent detector's "behavior is not 100% reliable for composite maintenance trips" and prescribes a multi-sentence manual verification procedure (cross-reference Section 7 decision timeline, cross-reference Section 4 action lifecycle pairs, note that anomalies may fire on windows containing active multi-tick work). Once S118STUAGEDET-001 lands, the described imprecision no longer exists — the detector correctly treats open-frame ticks as non-idle — and the expanded caveat becomes stale defensive guidance that wastes analyst time and mis-teaches future readers about the detector's actual contract. The doc must be simplified to match reality.

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Current state confirmed on 2026-04-18: the paragraph lives at `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md:21`, not in `SKILL.md`. A `grep -rn "100% reliable"` under `.claude/skills/scenario-analysis/` returns exactly one hit, at that line. The surrounding file (`layer-1-behavioral-smells.md`) is the canonical analyst-facing reference for the ten behavioral-smell categories; line 16-19 lists the three Stuck-Agent sub-patterns (Explainable idle / Pathological / Post-death) and line 21 is the "Detector caveat" paragraph targeted by this ticket.
2. Spec reference: `specs/S118-stuck-agent-detector-active-frame-exclusion.md` D5 specifies the simplified replacement text verbatim.
3. Shared boundary under audit: none — doc-only edit. The change is coherent only after S118STUAGEDET-001 ships the detector fix; until then the simplified text would be factually false. Hence the `Deps` on 001.

## Architecture Check

1. **Doc truthfulness**: The expanded caveat was a defensive workaround for the detector's imprecision. Once the detector is precise, keeping the workaround text is cargo-culted guidance that teaches readers a wrong mental model of the current system. Simplifying the text is the architecturally correct response (FND-28: no dead paths or fossilized logic — this extends to documentation that describes a no-longer-current failure mode).
2. **No new paths or shims**: the simplified text does not introduce new verification procedures or cross-references; it restores the pre-audit form of the paragraph.

## Verification Layers

1. Single-layer ticket: documentation edit only. Verification is behavioral — that the claim "multi-tick actions are not counted as idle" holds under the current detector — and is proved by S118STUAGEDET-001's `stuck_detector_excludes_wash_travel_cycle` test, not by any assertion in this ticket.

## What to Change

### 1. Replace the "Detector caveat" paragraph

At `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md:21`, replace the current paragraph:

> **Detector caveat**: The mechanical detector counts consecutive ticks with no action *started or in-progress*. Multi-tick actions like sleep usually occupy the agent and aren't counted as idle, but travel+multi-tick-action sequences (travel→wash→travel, travel→harvest→travel) can still register as stuck windows — behavior is not 100% reliable for composite maintenance trips. Before classifying a flagged window as a false positive, verify against Section 7 decision timeline and Section 4 ActionStarted/ActionCommitted pairs inside the window: continuous active frames or action-lifecycle pairs covering the window confirm a false positive; otherwise investigate. Section 2's "max consecutive idle ticks" may therefore exceed the detector threshold without triggering an anomaly, and anomalies may fire on windows containing active multi-tick work.

with the simplified form specified by the spec:

> **Detector caveat**: the mechanical stuck-agent detector counts consecutive ticks with no action *started or in-progress*. Multi-tick actions like sleep, wash, and travel legs occupy the agent and are not counted as idle. Therefore "max consecutive idle ticks" in Section 2 may exceed the detector's threshold without triggering an anomaly.

The removed clauses — "behavior is not 100% reliable for composite maintenance trips", the multi-sentence verification procedure, and the "anomalies may fire on windows containing active multi-tick work" sentence — are all made obsolete by the S118STUAGEDET-001 runtime fix.

### 2. No other files touched

`.claude/skills/scenario-analysis/SKILL.md` does not contain this paragraph (confirmed by `grep -rn "100% reliable"` returning only the one hit in `references/layer-1-behavioral-smells.md`). No cascade edit is required.

## Files to Touch

- `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` (modify — line 21 paragraph)

## Out of Scope

- Any edits to `.claude/skills/scenario-analysis/SKILL.md` itself (confirmed irrelevant by grep; any future relevance would be a separate ticket).
- Other Smell-3 guidance at lines 16-19 (the three sub-patterns) — they remain accurate and are not affected by this ticket.
- Caveats for other mechanical smells (ActionLoop, MaintenanceStarvation, etc.) — outside spec scope.
- The runtime detector fix and its regression/guardrail tests — owned by S118STUAGEDET-001 and S118STUAGEDET-002.

## Acceptance Criteria

### Tests That Must Pass

1. No code tests — doc-only ticket. Behavioral verification is inherited from S118STUAGEDET-001's `stuck_detector_excludes_wash_travel_cycle`, which proves the simplified text's claim ("multi-tick actions ... are not counted as idle") holds for composite trips.
2. Existing suite: `cargo test -p worldwake-cli --test golden_observer_anomalies stuck_detector_excludes_wash_travel_cycle` — re-run to confirm the detector fix is still in place before simplifying the doc.

### Invariants

1. `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` Smell-3 guidance accurately reflects the current detector contract. A `grep -n "100% reliable" .claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` returns zero matches after this ticket.
2. The simplified text remains loaded by the `scenario-analysis` skill the next time an analyst runs `/scenario-analysis` — the file is referenced from the skill's layered guidance model, so no frontmatter or registration change is needed.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies stuck_detector_excludes_wash_travel_cycle` — confirms the detector fix from S118STUAGEDET-001 is present before editing the doc.
2. `grep -n "100% reliable" .claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` — must return zero matches after the edit. (Verification via direct grep; narrower than `scripts/verify.sh` because the ticket changes no compiled or runtime code.)
