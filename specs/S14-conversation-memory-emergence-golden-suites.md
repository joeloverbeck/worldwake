**Status**: PENDING

# S14: Conversation Memory Emergence Golden E2E Suites

## Summary

E15c landed the core conversation-memory architecture and added four E15c-focused golden tests in `golden_social.rs`: unchanged-repeat suppression, lawful re-tell after belief change, lawful re-tell after conversation-memory expiry, and decision-trace visibility for social re-enablement.

Those tests close the direct social regression risk, but they do not yet prove two architecturally important E15c contracts end to end:

1. removing the old same-place shortcut actually matters in a live multi-system chain, and
2. listener-aware resend filtering happens before truncation in the live AI/action path and not only in focused unit tests.

This spec adds two cross-system golden suites that exercise those missing contracts through ordinary downstream behavior rather than social-only assertions. Both suites are designed to prove that E15c is not just an anti-spam patch. It is part of the causal substrate for lawful information transfer, belief-driven planning, and downstream office behavior.

## Phase

Phase 3: Information & Politics (post-E15c, post-E16d)

## Crates

`worldwake-ai`
- add cross-system golden suites

`docs`
- update golden E2E coverage docs after the suites land

## Dependencies

- E14 (belief-only planning, locality, belief retention)
- E15b (social AI goals, `ShareBelief`)
- E15c (conversation memory, resend suppression, trace diagnostics)
- E16d (office-claim planning and locality goldens)

## Why This Exists

Current golden coverage proves E15c's local social mechanics, but it still leaves two meaningful end-to-end gaps:

1. **No golden currently proves removal of the old same-place suppression heuristic.**
   The old shortcut suppressed Tell when the speaker believed the subject was already at the current place. Existing autonomous Tell goldens use remote subjects, so they would still pass under the pre-E15c heuristic.

2. **No golden currently proves listener-aware pre-truncation in a live downstream chain.**
   Focused tests cover resend filtering before truncation in candidate generation and tell affordance enumeration, but no golden proves that an already-told recent subject does not crowd out an older untold subject in the real AI loop with a meaningful downstream consequence.

The gap is not that E15c lacks any goldens. The gap is that the remaining unproven behaviors are exactly the ones that distinguish the E15c architecture from a narrower social anti-spam patch.

## Foundational Alignment

This spec exists to strengthen proof of the following principles in [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md):

- Principle 1: maximal emergence through local causality
- Principle 7: locality of motion, interaction, and communication
- Principle 12: world state is not belief state
- Principle 13: knowledge is acquired locally and travels physically
- Principle 16: memory, evidence, and records are world state
- Principle 18: resource-bounded practical reasoning over scripts
- Principle 19: intentions are revisable commitments
- Principle 20: agent diversity through concrete variation
- Principle 24: systems interact through state, not through each other
- Principle 27: debuggability is a product feature

## Design Goals

1. Prove that same-place subject location is not a lawful proxy for listener knowledge.
2. Prove that E15c's listener-aware resend filtering affects real downstream planning outcomes.
3. Keep the suites maximally emergent: social state should influence politics through belief transfer, not through injected office facts or test-only shortcuts.
4. Use the strongest assertion surfaces per `docs/golden-e2e-testing.md`: decision traces for candidate absence/presence, action traces for lifecycle ordering, and authoritative state for durable office outcomes.
5. Keep setup lawful and explicit. If a branch must be isolated, remove competing lawful affordances through concrete world state such as eligibility or utility profiles, not by bypassing systems.

## Non-Goals

1. Re-testing the four E15c golden behaviors already covered in `golden_social.rs`
2. Adding new social system mechanics or new conversation-memory rules
3. Promoting every focused E15c unit invariant into a golden scenario
4. Adding test-only shortcuts that bypass Tell, belief transfer, or office planning

## Scenario Gaps To Close

### Scenario 24: Same-Place Office Fact Still Requires Tell

**File**: `crates/worldwake-ai/tests/golden_emergent.rs` or `crates/worldwake-ai/tests/golden_social.rs`
**Systems exercised**: Social (`tell`), belief acquisition, AI political candidate generation, political action execution (`declare_support`), succession
**Principles proven**: 1, 7, 12, 13, 24

**Intent**:
Prove that co-location with the office subject does not make the listener "already know" it, and therefore does not suppress a lawful Tell.

**Setup**:
- Vacant support-law office at `VillageSquare`
- Speaker and listener co-located at `VillageSquare`
- Speaker has direct belief about the local office and high `social_weight`
- Listener has `observation_fidelity = pm(0)` or equivalent concrete blindness for this scenario, so same-place presence does not seed the office belief passively
- Listener is politically ambitious and eligible to hold the office
- Speaker is prevented from becoming the claimant through concrete state:
  either `enterprise_weight = pm(0)` plus stronger listener motive, or an explicit eligibility rule that only the listener satisfies

**Emergent behavior proven**:
- Before Tell, the listener generates no `ClaimOffice` candidate despite sharing the office's place
- Speaker still generates and commits Tell about the same-place office subject
- After Tell, the listener gains a reported office belief and ordinary `ClaimOffice` behavior appears
- Listener declares support and is installed as office holder

**Why this is distinct**:
- Existing social-political goldens use remote office facts
- Existing autonomous Tell goldens use remote non-office subjects
- This scenario is the direct end-to-end regression proof for the E15c removal of the old same-place suppression shortcut

**Assertion surface**:
1. Decision traces: listener has no `ClaimOffice` before Tell and does after Tell
2. Action traces: `tell` commits before any `declare_support` commit by the listener
3. Authoritative state: listener becomes office holder
4. Negative contract: co-location alone never creates the political candidate before Tell
5. Determinism: replay companion

**Scenario-isolation choice**:
- The speaker must not win the office through an unrelated branch; use concrete eligibility or motive differences instead of test-only suppression

---

### Scenario 25: Already-Told Recent Subject Does Not Crowd Out Untold Older Office Fact

**File**: `crates/worldwake-ai/tests/golden_emergent.rs` or `crates/worldwake-ai/tests/golden_social.rs`
**Systems exercised**: Social (`tell`), conversation-memory resend suppression, listener-aware pre-truncation, AI political candidate generation, travel if remote office is used, political action execution (`declare_support`), succession
**Principles proven**: 1, 7, 13, 18, 19, 24, 27

**Intent**:
Prove the E15c ordering invariant that resend filtering happens before candidate truncation in the live AI/action path, not just in focused tests.

**Setup**:
- Speaker has `TellProfile { max_tell_candidates: 1, .. }`
- Speaker knows the listener and two relayable subjects:
  - subject A: more recent, already told current belief to this listener
  - subject B: older, not yet told to this listener
- Subject B should be an office fact with a real downstream payoff:
  the listener can claim the office only after hearing B
- Keep subject A lawful and shareable so the scenario exercises true ordering, not simple invalidation
- Speaker remains unable or unmotivated to claim subject B's office directly

**Emergent behavior proven**:
- After subject A has been told once, the speaker does not stall forever on the already-told recent subject
- Listener-aware resend filtering removes A before truncation
- The speaker next tells subject B through the ordinary Tell path
- The listener gains the previously missing office belief and enters the ordinary office-claim chain
- The downstream political outcome occurs only because B was not crowded out by stale duplicate resend state

**Why this is distinct**:
- Focused tests already prove pre-truncation filtering on the helper surfaces
- This suite proves the same invariant at the real AI/action level with a meaningful downstream causal consequence

**Assertion surface**:
1. Action traces: no second committed Tell for subject A before the first committed Tell for subject B
2. Decision traces: speaker omits A as `SpeakerHasAlreadyToldCurrentBelief` and later generates or ranks the Tell goal for B
3. Authoritative state: listener becomes office holder for B's office, or another equally concrete downstream office effect occurs
4. Optional travel ordering if B is remote: Tell for B commits before listener travel or `declare_support`
5. Determinism: replay companion

**Scenario-isolation choice**:
- Do not make subject A invalid or unreachable; the point is specifically that lawful stale duplicate content must not crowd out lawful untold content

## Preferred Placement

Prefer `golden_emergent.rs` if the downstream office chain is central to the scenario. Use `golden_social.rs` only if the scenario remains primarily social and the political consequence is secondary. The current gap is architectural emergence, so `golden_emergent.rs` is the default recommendation.

## Component Registration

No production component or record changes are allowed in this spec.

Test-only harness additions are permitted if they are generic helpers for:
- office seeding
- belief seeding
- decision-trace inspection
- action-trace inspection

## SystemFn Integration

No production SystemFn changes are expected.

The implementation should use the existing live stack:
- candidate generation
- ranking
- planner search
- Tell action execution
- belief mutation
- political planning
- succession resolution

If implementation pressure suggests changing live system behavior to make these tests pass, stop and reassess. The purpose of this spec is coverage for already-intended architecture, not a test-driven behavior rewrite.

## Cross-System Interactions (Principle 24)

The required state-mediated chain is:

1. speaker holds local belief plus conversation memory state
2. social candidate generation reads only the speaker's beliefs and remembered tells
3. Tell commits and mutates listener belief state
4. political candidate generation reacts to the listener's new belief state
5. travel and political actions execute through ordinary affordances
6. succession resolves through authoritative office state

No test should inject the listener's office belief after tick start once the scenario begins.

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path |
|-------------|--------|------|
| local office fact in Scenario 24 | direct observation by speaker | `known_entities` on speaker -> Tell -> listener `known_entities` -> `ClaimOffice` |
| already-told vs untold subject distinction in Scenario 25 | `told_beliefs` on speaker | conversation memory -> resend suppression -> post-filter truncation -> Tell selection |
| political action availability | listener belief state | decision traces -> planner -> `declare_support` |

### H.2 Positive-Feedback Analysis

**Loop 1: repeated social chatter**
- E15c already dampens this via conversation memory
- Scenario 25 specifically verifies that the dampener does not also starve lawful untold subjects

**Loop 2: Tell -> office claim -> more local social contact**
- Real but bounded by action duration, succession delay, co-location, and ordinary planning competition

### H.3 Concrete Dampeners

- Tell duration and occupancy
- co-location requirement for Tell
- conversation-memory retention and capacity
- `max_tell_candidates`
- office succession delay
- office eligibility
- travel time if remote office is used in Scenario 25
- ordinary motive/ranking competition

### H.4 Stored State vs Derived

**Stored**
- `AgentBeliefStore.known_entities`
- `AgentBeliefStore.told_beliefs`
- office state and succession relations
- action traces and decision traces for inspection

**Derived**
- social omission status such as `SpeakerHasAlreadyToldCurrentBelief`
- political candidate appearance after Tell
- final office-holder outcome as consequence of prior state changes

## Acceptance Criteria

1. We have a golden that fails under the pre-E15c same-place suppression heuristic and passes under the intended E15c model.
2. We have a golden that fails if listener-aware resend filtering happens after truncation and passes when filtering happens before truncation.
3. Both suites prove downstream office behavior through ordinary belief transfer and planning, not via manual mid-scenario belief injection.
4. Both suites use decision traces for negative candidate claims and action traces for ordering claims.
5. Both suites have deterministic replay companions unless implementation reveals a documented reason not to add one.
6. `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` are updated to reflect both the already-landed E15c tests and the new cross-system suites.

## Tickets

### S14-001: Same-Place Office Fact Still Requires Tell

**Deliverable**:
- `golden_same_place_office_fact_still_requires_tell`
- deterministic replay companion

**Verification**:
1. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`

---

### S14-002: Already-Told Recent Subject Does Not Crowd Out Untold Older Office Fact

**Deliverable**:
- `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`
- deterministic replay companion

**Verification**:
1. `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`

---

### S14-003: Golden E2E Docs Catch-Up

**Deliverable**:
- update `docs/golden-e2e-coverage.md`
- update `docs/golden-e2e-scenarios.md`

This doc work must explicitly include the already-landed E15c social tests, which are currently under-reported in the golden docs.

## Critical Files

| File | Role |
|------|------|
| `specs/S14-conversation-memory-emergence-golden-suites.md` | this spec |
| `crates/worldwake-ai/tests/golden_emergent.rs` | preferred home for the new suites |
| `crates/worldwake-ai/tests/golden_harness/mod.rs` | helper additions if needed |
| `docs/golden-e2e-coverage.md` | coverage matrix update |
| `docs/golden-e2e-scenarios.md` | scenario catalog update |

## Verification

1. targeted test names
2. owning golden test binary
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Implementation Order

S14-001 -> S14-002 -> S14-003
