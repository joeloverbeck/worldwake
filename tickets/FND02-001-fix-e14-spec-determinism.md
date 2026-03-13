# FND02-001: Fix E14 Spec — Determinism Safety & Section H

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — spec-only change
**Deps**: Phase 2 complete, FND-01 complete

## Problem

`specs/E14-perception-beliefs.md` violates project-wide determinism invariants and CLAUDE.md Spec Drafting Rules:

1. **Line 35**: `known_facts: HashMap<FactId, PerceivedFact>` — `HashMap` has non-deterministic iteration order. All authoritative state must use `BTreeMap`/`BTreeSet`.
2. **Line 41**: `confidence: f32` — floats are forbidden in authoritative state. Must use `Permille`.
3. **Missing FND-01 Section H analysis** (information-path, positive-feedback, dampeners, stored vs derived) — required for all specs per CLAUDE.md.
4. **Missing LoyalTo resolution requirement** — `LoyalTo.strength: Permille` must be replaced with concrete relational state derived from shared experiences, obligations, and betrayals.
5. **Missing OmniscientBeliefView full replacement mandate** — E14 must fully replace `OmniscientBeliefView`, not wrap it.

## Assumption Reassessment (2026-03-13)

1. E14 spec exists at `specs/E14-perception-beliefs.md` — confirmed.
2. HashMap on line 35, f32 on line 41 — confirmed by direct reading.
3. No Section H analysis present — confirmed.
4. Section B deferred requirements (lines 82-91) reference information propagation and belief traceability — these should be cross-referenced but not duplicated.
5. No mismatch — ticket scope is accurate.

## Architecture Check

1. Spec-only change — no code impact. Ensures E14 implementation will be determinism-safe from the start, avoiding costly rework.
2. No backwards-compatibility shims — the spec is being corrected before any implementation begins.

## What to Change

### 1. Replace HashMap with BTreeMap

In the Memory component specification (around line 35), replace:
- `known_facts: HashMap<FactId, PerceivedFact>` with `known_facts: BTreeMap<FactId, PerceivedFact>`
- Apply the same replacement to any other `HashMap` occurrences in authoritative state definitions throughout the spec.

### 2. Replace f32 with Permille

In the `PerceivedFact` struct specification (around line 41), replace:
- `confidence: f32` with `confidence: Permille`
- Document the scale: `Permille(1000)` = direct observation, degrading for indirect sources (e.g., `Permille(700)` for second-hand report, `Permille(400)` for rumor).

### 3. Add FND-01 Section H analysis

Add a new section to the spec containing:
- **Information-path analysis**: How information reaches agents (event -> witness -> belief store, event -> rumor -> belief store with per-hop delay).
- **Positive-feedback analysis**: Amplifying loops in perception/belief system (e.g., more beliefs -> more actions -> more events -> more beliefs).
- **Concrete dampeners**: Physical mechanisms limiting each loop (perception radius, memory capacity, attention/processing time per tick).
- **Stored state vs. derived read-model list**: Authoritative (belief stores, witness records) vs. derived (belief queries, staleness checks, confidence lookups).

### 4. Add LoyalTo resolution requirement

Add a subsection requiring E14 to replace `LoyalTo.strength: Permille` with concrete relational state. Loyalty becomes a derived read-model computed from:
- Shared experiences (witnessed cooperation, life-saving events)
- Fulfilled obligations
- Betrayals
- Time spent together

### 5. Add OmniscientBeliefView full replacement mandate

Add explicit requirement: after E14 implementation, no code path may use `OmniscientBeliefView`. It must be fully replaced, not wrapped.

## Files to Touch

- `specs/E14-perception-beliefs.md` (modify)

## Out of Scope

- Do NOT implement E14 — this ticket only fixes the spec document.
- Do NOT modify any Rust code.
- Do NOT change other specs (S01-S06, E15-E22).
- Do NOT restructure the spec's overall organization — only add/fix the specific items listed.

## Acceptance Criteria

### Tests That Must Pass

1. Manual spec review: E14 spec contains zero `HashMap` in authoritative state definitions.
2. Manual spec review: E14 spec contains zero `f32` in authoritative state definitions.
3. Manual spec review: E14 spec includes complete FND-01 Section H with all four sub-analyses.
4. Manual spec review: E14 spec documents LoyalTo resolution through concrete relational state.
5. Manual spec review: E14 spec explicitly requires full OmniscientBeliefView removal.

### Invariants

1. Spec must remain internally consistent — no contradictions between Section H analysis and existing spec sections.
2. All cross-references to other specs (E15, FND-01, etc.) must be accurate.
3. No `f32`, `f64`, `HashMap`, or `HashSet` anywhere in authoritative state definitions.

## Test Plan

### New/Modified Tests

1. No code tests — spec-only change.

### Commands

1. `grep -i "hashmap\|hashset\|f32\|f64" specs/E14-perception-beliefs.md` — verify no forbidden types remain in authoritative state definitions.
