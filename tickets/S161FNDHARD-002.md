# S161FNDHARD-002: Downstream-doc anchoring of new principles

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `archive/tickets/S161FNDHARD-001.md` (the constitutional anchors must exist before downstream docs reference them)

## Problem

S161FNDHARD-001 adds FND-14B, strengthens FND-12 and FND-31, and inserts the FND-20
HTN guard into `docs/FOUNDATIONS.md`. The downstream docs that *cite* FOUNDATIONS as
authority must be updated to reference and apply the new principles (S161
Deliverable 6):

- `docs/planner-contracts.md` §2 states the source-scoped belief-view rule but cites
  only FND-14A — it needs an explicit FND-14B reference now that the constitutional
  anchor exists.
- `docs/spec-drafting-rules.md` has no causal-equivalence-contract checklist and no
  systemic-validation checklist — both are genuinely absent (verified) and required
  by the revised FND-12/FND-31.
- `docs/golden-e2e-testing.md` already carries the causal-reason doctrine but needs
  terminology aligned with the revised FND-31 plus an explicit "illegal planner-input
  absence" proof pattern.

## Assumption Reassessment (2026-05-21)

1. `docs/planner-contracts.md` cites FND-14A twice and zero times FND-14B (grep
   confirmed this session). Its "Planner-visible fields are source-scoped" rule lives
   at §2 (lines ~98–132) and already enumerates the S158 accessors and source
   classes — this ticket adds the FND-14B anchor, not new rule content.
2. `docs/spec-drafting-rules.md` (read this session) currently contains: the 5
   Section H analyses (lines 1–11), the Belief-View Accessor Source-Class Rule
   (13–37), the HTN Method Drafting Checklist (39–69), and the Agent Profile Scenario
   Contract (71–99). Grep confirms zero matches for "causal-equivalence" and
   "systemic-validation" — the two new checklist items are genuinely net-new.
3. `docs/golden-e2e-testing.md` already states (verified) that a scenario golden is
   valid only when it proves the authored causal reason, and names the 1440-tick
   failure taxonomy. This ticket aligns its terminology with the revised FND-31 and
   adds the illegal-planner-input-absence proof pattern, citing the existing
   `belief_wall_trap` negative-candidate assertions as the exemplar.
4. Shared boundary under audit: these three docs are the downstream consumers of the
   FOUNDATIONS principles changed in S161FNDHARD-001. This ticket must land after
   001 so the FND-14B / revised-FND-12 / revised-FND-31 references resolve to real
   constitutional sections.
5. Mismatch + correction: none. All three target docs exist and the claimed gaps
   (missing FND-14B ref; missing two checklists) are confirmed present-as-described.

## Architecture Check

1. Keeps the constitution as the single source of truth: downstream docs *reference*
   FOUNDATIONS principles rather than restating doctrine, so the FND-14B anchor in
   `planner-contracts.md` and the FND-12/FND-31 citations in `spec-drafting-rules.md`
   point upward instead of duplicating the rule text.
2. No backward-compatibility shims: the new spec-drafting checklist items are
   additive sections; the planner-contracts edit adds a citation to existing rule
   text without forking it.

## Verification Layers

1. FND-14B referenced in `planner-contracts.md` §2 -> grep for "FND-14B" returns ≥1.
2. Causal-equivalence and systemic-validation checklist items present in
   `spec-drafting-rules.md` -> grep for the two new section headings returns matches.
3. Single-layer documentation ticket: no runtime/trace surface is touched. The
   "illegal planner-input absence" proof pattern added to `golden-e2e-testing.md`
   names an *existing* test pattern (the `belief_wall_trap` negative assertions); no
   new test is authored here.

## What to Change

### 1. planner-contracts.md — anchor FND-14B (Deliverable 6a)

In §2 ("Planner-visible fields are source-scoped"), add an explicit FND-14B
reference stating that the source-class rule is the application of FND-14B to
belief-view accessors. Do not rewrite the existing rule body.

### 2. spec-drafting-rules.md — two new checklist items (Deliverable 6b)

Add a **causal-equivalence contract** checklist item (cite revised FND-12; require
the five named contract elements: explicit referent, preserved causal variables,
admitted error bounds, materialization/decompression boundary, comparison
tests/audits) for specs introducing offscreen sim, boundary compression, sleeping
entities, region summaries, population approximations, prehistory, or new
cache/save-load surfaces. Add a **systemic-validation** checklist item (cite revised
FND-31; require declaring negative illegal-path cases and naming which feature-scoped
systemic checks apply) for cross-system features.

### 3. golden-e2e-testing.md — terminology + illegal-input-absence pattern (Deliverable 6c)

Align terminology with the revised FND-31 and add an explicit "illegal planner-input
absence" proof pattern, citing the existing `belief_wall_trap` negative-candidate
assertions (`assert_no_steal_candidate_from_generation` /
`assert_no_steal_candidate_in_decision_trace`) as the exemplar.

## Files to Touch

- `docs/planner-contracts.md` (modify)
- `docs/spec-drafting-rules.md` (modify)
- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- All `docs/FOUNDATIONS.md` edits — those are S161FNDHARD-001.
- `docs/scenario-roadmap.md` rows for scenarios I–L — S161 Deliverable 7 defers
  these to when the backing goldens are scheduled.
- Creating `docs/causal-equivalence-contracts.md` — deferred (S161 Deliverable 7);
  this ticket only adds the *checklist item* that will require such a contract.
- Any code or test change. Documentation only.

## Acceptance Criteria

### Tests That Must Pass

1. `docs/planner-contracts.md` references "FND-14B" at least once within §2.
2. `docs/spec-drafting-rules.md` contains a causal-equivalence-contract checklist
   item naming the five required contract elements.
3. `docs/spec-drafting-rules.md` contains a systemic-validation checklist item
   requiring negative illegal-path cases.
4. `docs/golden-e2e-testing.md` contains an "illegal planner-input absence" proof
   pattern citing the `belief_wall_trap` negative assertions.
5. Existing suite unaffected: `cargo test --workspace` (sanity — no compiled surface
   changed).

### Invariants

1. Downstream docs reference FOUNDATIONS principles by number rather than restating
   their full doctrine (no double-truth between constitution and downstream docs).
2. No new authoritative state, component, action, or test is introduced.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `grep -c "FND-14B" docs/planner-contracts.md` (expect ≥1)
2. `grep -in "causal-equivalence\|systemic-validation\|illegal planner-input" docs/spec-drafting-rules.md docs/golden-e2e-testing.md`
3. `cargo test --workspace` (sanity — no compiled surface changed)
