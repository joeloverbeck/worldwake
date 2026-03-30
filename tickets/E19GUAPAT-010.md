# E19GUAPAT-010: Choose one canonical settlement-safety substrate for guards and thieves

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — patrol/thief ranking substrate, belief transport path, specs, and mixed-layer tests
**Deps**: [archive/specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/archive/specs/E19-guard-patrol.md), [archive/tickets/guard-patrol/E19GUAPAT-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-007.md), [archive/specs/E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/archive/specs/E17-crime-theft-justice.md), [specs/E22-integration-soak-tests.md](/home/joeloverbeck/projects/worldwake/specs/E22-integration-soak-tests.md)

## Problem

The live code still has two lawful but separate “settlement safety” substrates:

1. guard patrol urgency uses local crime memory plus office/control beliefs,
2. thief deterrence uses local witness counting.

That split is why the original E19 spec overclaimed a public-order feedback loop that the live architecture does not actually implement. It also leaves the simulation with two partially overlapping causal stories for the same domain question: “how dangerous is it to commit crime here?”

This is architecturally fragile. It weakens explainability, invites future alias paths, and makes it too easy for specs, tests, and implementation to drift apart again.

## Assumption Reassessment (2026-03-30)

1. The current guard-side patrol motive surface is in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) and depends on patrol profile weight plus local violation memory and believed office/control instability.
2. The current thief deterrence surface is in [`crates/worldwake-ai/src/theft.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/theft.rs) and is based on locally observed witness count, not on `public_order()`.
3. `public_order()` in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) is a derived view that already includes guard-presence bonuses. It is not currently the canonical criminal-deterrence input to thief decisions.
4. The same fact family, “this place is actively guarded / risky for theft,” therefore has multiple partially overlapping lawful transport paths today:
   - direct witness-presence observation for thieves,
   - violation/institutional memory for guards,
   - derived `public_order()` for designer/CLI visibility.
5. The exact shared abstraction boundary under audit is the safety/deterrence contract between patrol behavior, guard beliefs, thief beliefs, and any settlement-level derived view.
6. The motivating invariant is not “make thieves read a score.” The invariant is “one concrete causal substrate should explain both why guards intensify patrol and why thieves avoid or tolerate a place.” Any retained secondary path must be explicitly justified.
7. This ticket must not violate Principle 14 or Principle 15 in [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md). Agents cannot query omniscient settlement truth. Any canonical substrate must travel through lawful local evidence, beliefs, or explicit artifacts.
8. This is not a stale-request or start-failure ticket. The first failure boundary is architectural/spec coherence: the current code permits two different answers to “why was this place considered dangerous?” depending on which role is being debugged.
9. Reassessment already showed that E19’s full “crime rises -> patrol rises -> public order rises -> crime falls” loop is not live. This adjacent contradiction is not a documentation typo; it is a real architecture follow-up and should be tracked explicitly rather than silently deferred.
10. The clean end-state must name a canonical path and either remove the duplicate path in-scope or explicitly defer the removal to a follow-up ticket. Leaving both active without a named contract would violate the ticket authoring contract and Principle 3.
11. This aligns with `docs/FOUNDATIONS.md` Principle 3, Principle 7, Principle 11, Principle 15, and Principle 18: one concrete causal substrate, lawful information travel, explicit dampeners, provenance-bearing knowledge, and world-state evidence rather than implicit dual logic.

## Architecture Check

1. A single canonical settlement-safety substrate is cleaner than the current split because it gives one explainable answer to both guard escalation and thief deterrence.
2. The clean architectural bar is not “pipe `public_order()` straight into thieves.” That would risk violating locality and belief-only planning. The chosen substrate must be belief-legal and causally local.
3. Viable architectural directions that should be compared during implementation:
   - elevate local observed guard presence / recent patrol evidence into the canonical substrate and derive both thief deterrence and patrol reinforcement from it,
   - create an explicit, transmissible safety artifact or belief carrier that lawful agents can perceive, tell, or consult,
   - or deliberately keep thief deterrence purely local and reduce E19/public-order claims so only one domain uses settlement-level safety at all.
4. The ticket should choose one path and remove or demote the competing explanation. No backwards-compatibility aliasing, no “both are valid depending on subsystem.”

## Verification Layers

1. Canonical deterrence substrate is explicit and single-sourced -> focused unit/runtime coverage for the chosen ranking/input symbols
2. Guard patrol escalation and thief deterrence consume the same lawful substrate -> focused cross-role tests at ranking/candidate-generation layer
3. Information travel remains lawful -> golden scenario(s) proving local knowledge, delayed propagation, and no omniscient leak
4. Settlement-level feedback claims, if retained, are proven through mixed-layer golden coverage rather than spec narrative alone
5. If trace surfaces remain too weak to explain the new canonical path, open or depend on a traceability follow-up rather than hiding the gap in scenario-only assertions

## What to Change

### 1. Choose and document the canonical settlement-safety substrate

Decide which concrete, lawful carrier explains both patrol escalation and criminal deterrence. Update the relevant spec/doc language to reflect that canonical path and remove the over-broad E19 feedback-loop wording if it no longer matches the chosen design.

### 2. Remove or demote the competing substrate

Whichever non-canonical path remains today should either be removed in-scope or explicitly downgraded to a derived/helper role that no longer independently drives role behavior.

### 3. Add focused cross-role tests

Add tests proving guards and thieves respond to the same canonical substrate under lawful local information conditions, including stale/missing information boundaries.

### 4. Add mixed-layer proof for the resulting feedback contract

If the final architecture really supports a negative patrol/crime feedback loop, add a golden proving that lawful loop. If it does not, update the spec language and do not add a misleading golden.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/theft.rs` (modify)
- `crates/worldwake-systems/src/offices.rs` or another chosen canonical-carrier surface (modify, if needed)
- `crates/worldwake-ai/tests/golden_patrol.rs` and/or other mixed-layer golden files (modify, if needed)
- `archive/specs/E19-guard-patrol.md` or successor active spec material if the documented contract changes (modify)
- `specs/E22-integration-soak-tests.md` (modify, if the integration contract changes)

## Out of Scope

- Captain-issued patrol assignment systems
- General social-artifact unification beyond what is required to pick one canonical safety substrate
- Cosmetic CLI changes that do not clarify the causal contract
- Adding a hidden abstract “danger score” with no concrete carrier path

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove one canonical deterrence substrate now drives both relevant role behaviors
2. No test or spec still claims a public-order feedback loop unless the live code actually proves it
3. Existing suite: `cargo test -p worldwake-ai golden_patrol -- --nocapture`
4. Existing suite: `cargo test -p worldwake-ai golden_emergent -- --nocapture`
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo test --workspace`
7. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Settlement safety / theft deterrence has one canonical causal substrate, not two independent behavioral drivers
2. The chosen substrate remains belief-local and traceable through lawful information carriers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` and/or `crates/worldwake-ai/src/theft.rs` focused tests — prove guards and thieves consume the same canonical substrate under controlled local belief conditions
2. `crates/worldwake-ai/tests/golden_patrol.rs` or another mixed-layer golden — prove the resulting cross-role behavior without omniscient leakage
3. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.` should not be used here because the point is architectural behavior, not just wording

### Commands

1. `cargo test -p worldwake-ai golden_patrol -- --nocapture`
2. `cargo test -p worldwake-ai golden_emergent -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
