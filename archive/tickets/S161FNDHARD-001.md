# S161FNDHARD-001: FOUNDATIONS.md constitutional amendments

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `specs/S161-foundations-constitutional-hardening.md`

## Problem

`docs/FOUNDATIONS.md` is underspecified at four pressure points the near-term AI
architecture work is actively touching:

- **FND-12** says approximation must "remain equivalent to the explicit model" but
  never defines what *equivalent* requires (no named referent, preserved variables,
  materialization boundary, or audit).
- The **belief-backed planner-input rule** is implied by FND-14/14A but is not
  constitutional, even though S158 already shipped its concrete form (the
  source-class rule) into `docs/planner-contracts.md` and
  `docs/spec-drafting-rules.md`. Those downstream docs cite FOUNDATIONS as authority
  for a principle FOUNDATIONS does not name.
- **FND-20** forbids scripts in spirit but does not explicitly bind HTN method
  selection/decomposition/rejection/fallback to the belief-backed source rule.
- **FND-31** is weaker than the *active* golden-testing doctrine
  (`docs/golden-e2e-testing.md`, `docs/scenario-roadmap.md`) it should match.

This ticket applies the five constitutional edits (S161 Deliverables 1–5) to
`docs/FOUNDATIONS.md`: strengthen FND-12, insert FND-14B, insert the FND-20 HTN
anti-script guard, replace FND-31, and add canonical scenarios I–L.

## Assumption Reassessment (2026-05-21)

1. Anchor texts verified exact against the live file (reassessed this session):
   FND-12 body begins "Optimization is allowed. Causal cheating is not." at
   `docs/FOUNDATIONS.md:123` and runs through the **Test** line at L131 (including
   the "The rule is simple: …" sentence at L129, which the replacement drops);
   FND-14A's **Test** paragraph ends at L166 (FND-15 begins L168 — the FND-14B
   insertion point); the FND-20 anchor sentence "A method-required goal needs an
   explicit schema contract and tests proving that fallback would be semantically
   invalid." is the final clause of L228; FND-31 header is L397 and its body begins
   "Interesting-looking output is not evidence that the model is right." at L399;
   Scenario H header is L523 and `## VII. Final Rule of Thumb` is L540 (the I–L
   insertion lands between them).
2. Spec source: `specs/S161-foundations-constitutional-hardening.md` Deliverables
   1–5 carry the verbatim replacement/insertion text. The report's two misquotes
   (a non-verbatim FND-20 anchor, and "## VII. Rule of Thumb" missing "Final") were
   corrected in the spec and re-verified here.
3. Shared boundary under audit: `docs/FOUNDATIONS.md` is the constitutional source
   of truth cited by `docs/planner-contracts.md`, `docs/spec-drafting-rules.md`,
   `docs/golden-e2e-testing.md`, and generated coverage. This ticket changes only
   the constitution; the downstream citations are anchored in S161FNDHARD-002.
4. Intra-edit coupling: Deliverable 3's inserted FND-20 text references "Principle
   14B", created by Deliverable 2. Both land in this ticket, so the cross-reference
   is never dangling. Apply the FND-14B insertion before (or with) the FND-20 edit.
5. Mismatch + correction: none. Zero reference drift found during reassessment;
   all anchors resolve exactly.

## Architecture Check

1. Documentation-only edit to the constitution; it strengthens existing principles
   rather than adding mechanism. The FND-14B addition anchors a rule the codebase
   *already enforces* (S158 belief-view gating + source-class docs), so it
   introduces no behavioral change and no new authoritative state — it makes the
   existing contract regression-proof for future planner surfaces.
2. No backward-compatibility shims: the FND-12 and FND-31 bodies are *replaced*,
   not aliased; the stale "The rule is simple…" sentence is removed rather than
   left beside the new text (FND-28 in spirit, applied to doctrine).

## Verification Layers

1. Constitutional-anchor existence (FND-14B, revised FND-12/FND-31, FND-20 guard,
   scenarios I–L present and well-formed) -> grep of `docs/FOUNDATIONS.md` for the
   new headings and key phrases.
2. No stale residue (the dropped "The rule is simple…" sentence and the old FND-31
   body removed) -> grep returns zero matches for the removed phrases.
3. Single-layer documentation ticket: no decision/action/event-log surfaces are
   touched, so no runtime-trace layer mapping applies. The FND-14B **Test** is
   *already* satisfied at runtime by the existing S158 belief-view goldens
   (`remote_listed_sale_lot_does_not_read_live_sale_listing`, the
   `belief_wall_trap` negative-candidate assertions); this ticket does not modify or
   add those tests — they are the standing proof surface, named in S161 Deliverable 7.

## What to Change

### 1. Replace the FND-12 body (Deliverable 1)

Replace the entire body of `### 12. Performance May Compress Computation, Never
Causality` — everything from L123 "Optimization is allowed. Causal cheating is
not." through the **Test** line at L131, including the L129 "The rule is simple…"
sentence — with the causal-equivalence-contract text in S161 Deliverable 1.

### 2. Insert FND-14B (Deliverable 2)

Insert the `### 14B. Planner-Visible Inputs Must Be Belief-Backed or Lawful Boundary
Artifacts` section (S161 Deliverable 2) immediately after the FND-14A **Test**
paragraph (after L166, before FND-15 at L168). The one-line non-constitutional
implementation note (S158 already enforces the in-scope accessors) may accompany
the commit message or sit outside the principle text — do not embed it in the
numbered principle.

### 3. Insert the FND-20 HTN anti-script guard (Deliverable 3)

Insert the "HTN methods are not scripts. …" paragraph (S161 Deliverable 3)
immediately after the L228 sentence "A method-required goal needs an explicit
schema contract and tests proving that fallback would be semantically invalid."
This references Principle 14B (added in change 2 above) — apply 2 first.

### 4. Replace the FND-31 body (Deliverable 4)

Replace the entire body of `### 31. Validation and Falsification Are First-Class`
(from L399 "Interesting-looking output is not evidence that the model is right."
through the **Test** line) with the systemic-validation text in S161 Deliverable 4.

### 5. Add canonical scenarios I–L (Deliverable 5)

Insert scenarios I, J, K, and L (S161 Deliverable 5) after Scenario H (L523) and
before `## VII. Final Rule of Thumb` (L540).

## Files to Touch

- `docs/FOUNDATIONS.md` (modify)

## Out of Scope

- Downstream-doc anchoring (`planner-contracts.md`, `spec-drafting-rules.md`,
  `golden-e2e-testing.md`) — that is S161FNDHARD-002, which depends on this ticket.
- All S161 Deliverable 7 deferred items: `docs/causal-equivalence-contracts.md`,
  scenario K/L goldens, `scenario-roadmap.md` rows for I–L, and the scenario-J /
  remote-seller-HTN-rejection goldens. Do NOT create these here.
- Any code, test, component, or behavioral change. This is a constitution edit only.

## Acceptance Criteria

### Tests That Must Pass

1. `docs/FOUNDATIONS.md` contains the new heading `### 14B. Planner-Visible Inputs
   Must Be Belief-Backed or Lawful Boundary Artifacts` exactly once.
2. `docs/FOUNDATIONS.md` contains the four scenario headings `### I.`, `### J.`,
   `### K.`, `### L.` between Scenario H and `## VII. Final Rule of Thumb`.
3. The stale FND-12 sentence "The rule is simple: performance may change how the
   machine computes a result, never what the world means." returns zero matches.
4. The FND-20 guard sentence "HTN methods are not scripts." is present immediately
   after the method-required-goal anchor sentence.
5. Existing suite unaffected: `cargo test --workspace` (no code changed; this is a
   sanity check that the doc edit did not touch anything compiled).

### Invariants

1. No principle is renumbered; FND-1…FND-31 numbering and the A–H scenario labels
   are unchanged (only 14B is inserted and I–L are appended).
2. No new authoritative state, component, action, system, or feedback loop is
   introduced — the edit is constitutional text only.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `grep -n "### 14B\.\|### I\.\|### J\.\|### K\.\|### L\.\|HTN methods are not scripts" docs/FOUNDATIONS.md`
2. `grep -c "The rule is simple: performance may change" docs/FOUNDATIONS.md` (expect 0)
3. `cargo test --workspace` (sanity — no compiled surface changed)

## Outcome

Completed: 2026-05-21

Implemented the S161 constitutional amendments in `docs/FOUNDATIONS.md`:

- Replaced FND-12 with the causal-equivalence-contract wording.
- Inserted FND-14B immediately after FND-14A.
- Added the FND-20 HTN anti-script guard immediately after the method-required-goal sentence.
- Replaced FND-31 with the systemic-validation doctrine.
- Added canonical scenarios I-L between Scenario H and `## VII. Final Rule of Thumb`.

Deviations from original plan: none. No code, tests, components, actions, systems,
or generated docs changed.

Verification:

- `grep -n "### 14B\\.\\|### I\\.\\|### J\\.\\|### K\\.\\|### L\\.\\|HTN methods are not scripts" docs/FOUNDATIONS.md` found the new FND-14B heading, HTN guard, and scenarios I-L.
- `grep -c "The rule is simple: performance may change" docs/FOUNDATIONS.md` returned `0`.
- `grep -c "Interesting-looking output is not evidence that the model is right" docs/FOUNDATIONS.md` returned `0`.
- `cargo test --workspace` passed.
