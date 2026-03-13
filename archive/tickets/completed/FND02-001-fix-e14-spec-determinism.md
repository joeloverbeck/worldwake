# FND02-001: Fix E14 Spec — Determinism Safety & Section H

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — spec-only change
**Deps**: Phase 2 complete, FND-01 complete

## Problem

`specs/E14-perception-beliefs.md` violates project-wide determinism invariants and CLAUDE.md Spec Drafting Rules:

1. **Line 35**: `known_facts: HashMap<FactId, PerceivedFact>` — `HashMap` has non-deterministic iteration order. All authoritative state must use `BTreeMap`/`BTreeSet`.
2. **Line 41**: `confidence: f32` — floats are forbidden in authoritative state. Must use `Permille`.
3. **Missing FND-01 Section H analysis** (information-path, positive-feedback, dampeners, stored vs derived) — required for all specs per CLAUDE.md.
4. **Missing belief-to-social boundary requirement** — E14 currently does not define how witnessed cooperation, obligations, betrayals, and co-presence become belief-side evidence that later social systems can use. Without that boundary, E16 risks rebuilding loyalty on another abstract scalar.
5. **Missing OmniscientBeliefView full replacement mandate** — E14 must fully replace `OmniscientBeliefView`, not wrap it.

## Assumption Reassessment (2026-03-13)

1. E14 spec exists at `specs/E14-perception-beliefs.md` — confirmed.
2. HashMap on line 35, f32 on line 41 — confirmed by direct reading.
3. No Section H analysis present — confirmed.
4. `OmniscientBeliefView` is still live across `worldwake-sim`, `worldwake-ai`, `worldwake-systems`, and CLI/integration tests — confirmed by code search. The replacement mandate is correct, but this ticket remains spec-only.
5. `LoyalTo` is already an authoritative relation in core state as `RelationValue::LoyalTo { subject, target, strength: Permille }` and in `RelationTables::{loyal_to, loyalty_from}` — confirmed. That means the original ticket was assigning a social-state redesign to the wrong epic.
6. E16, not E14, is where loyalty is currently specified as a gameplay concept (`LoyalTo` with strength/support behavior). E14 should define the evidence pipeline and anti-omniscience boundary that loyalty will consume later; it should not own the full relation-model redesign by itself.
7. E15 still describes confidence with float-style language (`1.0`, `< 1.0`, decimal ranges). Leaving that untouched would make the Phase 3 information specs internally inconsistent once E14 moves to `Permille`.
8. Ticket scope was too narrow in one place and misplaced in another. It needs immediate downstream/spec-order wording cleanup, but not core Rust relation changes.

## Architecture Check

1. `HashMap` -> `BTreeMap`, `f32` -> `Permille`, and Section H are unequivocal improvements. They make the belief spec deterministic, auditable, and aligned with the project foundations before implementation begins.
2. Requiring full `OmniscientBeliefView` replacement is also the right architectural direction. The current stand-in leaks authoritative truth too widely to survive Phase 3.
3. The original proposal to make E14 directly replace `LoyalTo.strength` was not well-scoped. Loyalty is a social/institutional model concern owned by E16 and core relation state, not a perception primitive. The cleaner architecture is:
   - E14 defines how social evidence enters beliefs and forbids new belief APIs from depending on scalar loyalty truth.
   - E16 owns the concrete loyalty/support model redesign using that evidence.
4. No backwards-compatibility shims — the specs should describe the intended end-state and callers/specs should be updated to match.

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

### 4. Replace the misplaced loyalty requirement with an E14/E16 boundary requirement

Add a subsection to E14 specifying the belief-side inputs that later social systems must consume:
- witnessed cooperation / conflict
- fulfilled or broken obligations
- public records and reports
- co-presence / shared travel history where relevant

State explicitly that:
- E14 must not introduce new omniscient or scalar-loyalty shortcuts in belief APIs.
- The concrete replacement of `LoyalTo.strength` belongs to the social/institutional work, with E16 as the owning spec.
- E14 should reference that downstream requirement so the two specs stay aligned.

### 5. Add OmniscientBeliefView full replacement mandate

Add explicit requirement: after E14 implementation, no code path may use `OmniscientBeliefView`. It must be fully replaced, not wrapped.

### 6. Keep downstream spec wording consistent

Update immediate downstream planning docs that would otherwise contradict the E14 fixes:
- `specs/FND-02-foundations-alignment-phase2.md`
- `specs/IMPLEMENTATION-ORDER.md`
- `specs/E15-rumor-witness-discovery.md` for `Permille`-based confidence wording

## Files to Touch

- `specs/E14-perception-beliefs.md` (modify)
- `specs/E15-rumor-witness-discovery.md` (modify — confidence terminology only)
- `specs/FND-02-foundations-alignment-phase2.md` (modify — correct FND02-001 wording)
- `specs/IMPLEMENTATION-ORDER.md` (modify — correct E14/FND02-001 wording)

## Out of Scope

- Do NOT implement E14 — this ticket only fixes planning/spec documents.
- Do NOT modify any Rust code.
- Do NOT redesign the authoritative loyalty relation in code.
- Do NOT broaden into E16 behavior design beyond the minimum cross-spec boundary wording needed to avoid contradiction.
- Do NOT restructure the specs' overall organization — only add/fix the specific items listed.

## Acceptance Criteria

### Tests That Must Pass

1. Manual spec review: E14 spec contains zero `HashMap` in authoritative state definitions.
2. Manual spec review: E14 spec contains zero `f32` in authoritative state definitions.
3. Manual spec review: E14 spec includes complete FND-01 Section H with all four sub-analyses.
4. Manual spec review: E14 spec defines the belief-side evidence boundary for later loyalty/social modeling and does not assign the full loyalty relation redesign to E14 alone.
5. Manual spec review: E14 spec explicitly requires full OmniscientBeliefView removal.
6. Manual spec review: FND-02, Implementation Order, and E15 wording are consistent with the E14 changes.

### Invariants

1. Spec must remain internally consistent — no contradictions between Section H analysis and existing spec sections.
2. All cross-references to other specs (E15, E16, FND-01, etc.) must be accurate.
3. No `f32`, `f64`, `HashMap`, or `HashSet` anywhere in authoritative state definitions.

## Test Plan

### New/Modified Tests

1. No code tests — spec-only change.

### Commands

1. `grep -i "hashmap\|hashset\|f32\|f64" specs/E14-perception-beliefs.md` — verify no forbidden types remain in authoritative state definitions.

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Corrected this ticket's assumptions and scope before implementation.
  - Updated `specs/E14-perception-beliefs.md` to use `BTreeMap` and `Permille`, added FND-01 Section H, added an explicit social-evidence boundary, and made full `OmniscientBeliefView` replacement an acceptance requirement.
  - Updated `specs/E15-rumor-witness-discovery.md` so confidence terminology matches the `Permille`-based E14 model and fixed the stale S01 dependency link.
  - Updated `specs/FND-02-foundations-alignment-phase2.md` and `specs/IMPLEMENTATION-ORDER.md` so they no longer assign the full loyalty-model redesign to E14 alone.
- Deviations from original plan:
  - Did not specify that E14 itself replaces `LoyalTo.strength` in code or spec ownership. The codebase and Phase 3 spec structure show that loyalty redesign belongs to the later social/institutional work, with E14 providing the belief-side evidence pipeline.
  - Expanded scope slightly to include immediate downstream planning-doc consistency (`E15`, `FND-02`, and `IMPLEMENTATION-ORDER`) because leaving those unchanged would preserve contradictions.
  - No Rust code changes were made.
- Verification results:
  - `cargo test -p worldwake-ai candidate_generation -- --nocapture` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
