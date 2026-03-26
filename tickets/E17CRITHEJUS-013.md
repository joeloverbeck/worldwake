# E17CRITHEJUS-013: Golden test — witnessed theft enables accusation chain

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None
**Deps**: E17CRITHEJUS-008 (accuse action), E17CRITHEJUS-009 (fine/exile actions), E17CRITHEJUS-011 (justice candidates), E17CRITHEJUS-012 (theft golden as validation baseline), E17CRITHEJUS-015 (typed social-evidence detail), E17CRITHEJUS-016 (relayable social evidence)

## Problem

No end-to-end proof exists that the full crime-to-punishment chain works: steal (witnessed) -> witness Tell -> authority accuses -> authority punishes. This golden test proves P1 (emergent justice chain), P7 (witness co-location), P16 (accusation as institutional record), and P22 (jurisdiction requirement).

## Assumption Reassessment (2026-03-25)

1. This is the most complex E17 golden test, requiring 4+ agents and multiple system interactions (theft, perception, social Tell, travel, accusation, punishment). It validates the full P1 emergent chain.
2. The golden harness supports multi-agent Tell propagation for entity beliefs, but witness-driven crime testimony additionally requires `E17CRITHEJUS-016` to relay typed social evidence through Tell.
3. Accusation requires the accuser to be at a `CrimeRegister` location. The test setup must create a `CrimeRegister` entity at the authority's jurisdiction.
4. Punishment (Fine or Exile) requires institutional authority + co-location with accused. This may require multiple travel legs or a constrained place graph.
5. AI-layer ticket: full action registries required. Decision traces essential for debugging the multi-agent sequence.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. Isolation: Agent A (thief with `TheftDispositionProfile`), Agent C (witness with `PerceptionProfile`), Agent D (authority — office holder with `JusticeDispositionProfile`). CrimeRegister at authority's seat. Minimize survival-pressure profiles to keep the scenario focused on crime/justice.
11. Mismatch: the original ticket assumed the witnessed theft fact could already travel through Tell. The live Tell path is entity-belief-only. Correct scope is to run this golden after `E17CRITHEJUS-015` and `E17CRITHEJUS-016` land so witness testimony is concrete typed social evidence, not an implied side effect of entity gossip.
12. Timing: A steals (multi-tick). C witnesses (same tick via perception). C travels to authority. C Tells authority. Authority travels to CrimeRegister. Authority accuses. Authority travels to accused. Authority punishes. Total tick budget needs to be generous (40-80 ticks depending on travel distances).
13. Follow-up architectural note: if the scenario needs political or record knowledge beyond the crime testimony itself, it should use existing direct consultation/setup surfaces and must not introduce new office-or-record entity Tell shims. First-class institutional Tell topics are tracked separately in `E17CRITHEJUS-017`.
14. Follow-up architectural note: profile-driven crime/justice ranking is tracked separately in `E17CRITHEJUS-021`. This golden should isolate away unrelated low-priority competitors rather than assuming today's placeholder constant-motive ordering is the intended architecture.

## Architecture Check

1. Golden test proves a multi-system emergent chain that is the canonical E17 acceptance scenario. If any link breaks (perception of Hidden events, Tell propagation, accusation filing, punishment execution), this test fails.
2. No backwards-compatibility aliasing.

## Verification Layers

1. A steals at P -> action trace confirms steal commit
2. C (co-located) witnesses Hidden theft event -> C's belief store contains crime observation
3. C's ShareBelief candidate fires for crime observation -> decision trace
4. C Tells authority D -> D's belief store gains crime evidence
5. D generates Accuse candidate -> decision trace
6. D travels to CrimeRegister and files accusation -> CrimeRegister has Accusation entry
7. D generates PunishAccused candidate -> decision trace
8. D punishes A (Fine or Exile) -> Verdict supersedes Accusation in CrimeRegister
9. If Fine: commodity transferred, conservation maintained
10. If Exile: faction membership removed, hostile_to added
11. Deterministic replay companion

## What to Change

### 1. New golden test scenario

Create a scenario in `golden_emergent.rs` or `golden_crime.rs`:

**Setup**:
- Place P (theft location), Place Q (authority seat with CrimeRegister), Place R or P/Q collapsed for simplicity
- Travel edges between places
- Agent A (thief): at P, `TheftDispositionProfile`
- Agent C (witness): at P, `PerceptionProfile`, social profiles for Tell
- Agent D (authority): office holder at Q, `JusticeDispositionProfile`, `PerceptionProfile`
- Item lot owned by some victim (could be a 4th agent or a faction) at P
- CrimeRegister entity at Q
- Office entity at Q with D as holder

**Execution**:
- Step ticks: A steals. C witnesses. C shares via Tell with D (may require C to travel to Q, or D to be reachable). D accuses at CrimeRegister. D punishes A.

**Assertions per stage**: See Verification Layers above.

### 2. Deterministic replay companion

Standard replay test.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` or `crates/worldwake-ai/tests/golden_crime.rs` (new scenario)

## Out of Scope

- Multiple concurrent crimes
- Wrong accusations (P14 edge case — could be a follow-up golden)
- Theft at empty location (no witness — covered by E17CRITHEJUS-012)
- Guard patrol response (E19)
- Exile cascade behavior (exiled agent stealing again)
- Appeal or reversal of verdict
- Refactoring institutional Tell topics; handled by `E17CRITHEJUS-017`

## Acceptance Criteria

### Tests That Must Pass

1. `golden_witnessed_theft_accusation_chain` (or similar): full pipeline from witnessed steal -> Tell -> accuse -> punish
2. `golden_witnessed_theft_accusation_chain_replay`: deterministic replay companion
3. CrimeRegister contains both Accusation and superseding Verdict entries
4. If Fine: conservation check passes throughout
5. If Exile: faction membership removed and hostile_to relation exists
6. Existing suite: `cargo test -p worldwake-ai --test golden_*`

### Invariants

1. P1: Entire chain emerges from interacting systems without authored quest logic
2. P7: Witness must be co-located with thief; authority must travel to CrimeRegister; punisher must be co-located with accused
3. P13: Crime information travels physically (witness -> Tell -> authority)
4. P16: Accusation is an institutional record (InstitutionalClaim in CrimeRegister)
5. P21: Punishment requires institutional authority (office holder)
6. P22: Jurisdiction verified — only office holders at the right place can punish
7. Conservation maintained for all transfers (steal, fine)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` (or `golden_crime.rs`) — golden scenario + replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent` (or `golden_crime`)
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
