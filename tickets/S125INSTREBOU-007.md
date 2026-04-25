# S125INSTREBOU-007: survival-justice institutional bounty golden + roadmap

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario authoring change + new golden test
**Deps**: [S125INSTREBOU-001](../archive/tickets/S125INSTREBOU-001.md), [S125INSTREBOU-002](../archive/tickets/S125INSTREBOU-002.md), [S125INSTREBOU-003](../archive/tickets/S125INSTREBOU-003.md), [S125INSTREBOU-005](../archive/tickets/S125INSTREBOU-005.md), [S125INSTREBOU-004](../archive/tickets/S125INSTREBOU-004.md), S125INSTREBOU-006, [S125INSTREBOU-008](../archive/tickets/S125INSTREBOU-008.md)

## Problem

S125 Deliverable D8 requires `survival-justice` to retain its three existing branches (accusation substrate, fine punishment, search-and-report) and add a new bounty extension proving institutional bounty posting under survival. S125 Evidence #6 records the constraint that motivated this whole spec: prior attempts to add office-owned coin to `survival-justice` perturbed theft-scene perception and broke the investigation. The treasury container surface from ticket 002 is designed to scope funds out of place-floor perception; this ticket lands the scenario change and the new golden, then closes S125 by regenerating golden docs and updating `IMPLEMENTATION-ORDER.md` (Deliverable D9) after the authorization-policy remainder landed in S125INSTREBOU-008.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-ai/tests/golden_survival_justice.rs:535/594/640` — three existing tests confirmed (`survival_justice_proves_accusation_substrate`, `survival_justice_proves_fine_punishment_for_same_theft_case`, `survival_justice_proves_search_and_report_found`). `scenarios/survival-justice.ron` authors a Market Warden office at Market Square but no funds. The total existing scenario+test surface is ~1162 lines; the new test slots into the existing file convention.
2. S125 Acceptance Criterion 6 names the new test `survival_justice_proves_institutional_bounty_posted`. S125 §6 (Golden Landing) lists the proof points: `PostBounty` ranks/selects after the local crime case exists, `post_bounty` commits with `RewardSource::InstitutionalTreasury { treasury_entity: <Market Warden> }`, `RewardEncumbrance` materializes on the Market Warden office, the survival-health contract still passes for the owning agent.
3. Shared abstraction boundary: `golden_survival_justice.rs` test fixtures + `survival-justice.ron` scenario authoring + the `golden-survival` ignored lane (per `docs/golden-e2e-testing.md`).
4. Live `GoalKind` under test: `PostBounty` (existing). Operator/affordance surface: no change. The scenario depends on the new `TreasuryDef` from ticket 002 and the candidate-emitter wiring from ticket 006.
5. Scenario isolation: existing branches require theft-scene perception to remain unperturbed. Treasury container at the Market Warden's seat (Market Square) must scope coin lots inside the container so place-floor perception is unchanged. Intended branch under test: institutional bounty posting via the existing accusation/fine substrate. Lawful competing branch intentionally excluded: a personal-funds bounty fallback (S125 explicitly forbids using `PersonalFunds` to satisfy the institutional row proof).
6. Adjacent contradictions: archived ticket 002 landed focused spawn coverage that keeps treasury lots inside the treasury container and off the place floor. This ticket still owns the ignored survival-justice regression proof; if running the existing three goldens with the authored treasury reveals that container-internal lots still perturb theft-scene perception, file a new ticket dependency for a perception-scoping fix and apply the 1-3-1 rule rather than silently working around it. Do not relax the existing goldens to accommodate a perception leak.
7. Cumulative arithmetic: the authored treasury quantity must be sufficient to fund a bounty whose terms match the candidate emitter's defaults (`bounty_posting_weight` → `BountyTerms.reward_quantity`). Verify the live arithmetic during implementation by reading `post_bounty_motive` and the candidate emitter; do not trust the spec's narrative numbers.

## Architecture Check

1. The new golden depends only on the lawful chain — accusation → fine → bounty posting → encumbrance → bounty artifact. No scripted shortcuts (FND-1 maximal emergence). This is the canonical regression scenario A from FOUNDATIONS.md ("Beast Starvation → Caravan Attack → Report → Bounty …") in its institutional-bounty-funding form.
2. No backward compatibility: extends existing scenario authoring + adds a new test; no aliasing.

## Verification Layers

1. Bounty artifact materializes → action trace `commit_post_bounty` event + authoritative world-state assertion that the SocialArtifact exists with the expected terms.
2. Encumbrance recorded → event-log delta + authoritative world-state assertion of the `RewardEncumbrance` component on the Market Warden office.
3. Reward source identity → assertion that `BountyTerms.reward_source == RewardSource::InstitutionalTreasury { treasury_entity: <Market Warden office> }`.
4. Existing three goldens continue to pass → ensures theft-scene perception is unperturbed by the treasury authoring (regression guard for ticket 002's container scoping).
5. Survival-health contract for owning agent → existing assertion mechanism in `golden_survival_justice.rs` reused.

## What to Change

### 1. Extend `survival-justice.ron`

Add a `treasury` field to the Market Warden office authored at `scenarios/survival-justice.ron`:
- `commodity: Coin` (or whatever the live coin enum variant is — confirm against `CommodityKind`).
- `quantity`: sufficient to fund a bounty under the candidate emitter's defaults; pin exact value during implementation by reading `post_bounty_motive` (`ranking.rs:1380`) and any reward-quantity defaults.

### 2. New golden test

Add `survival_justice_proves_institutional_bounty_posted` in `crates/worldwake-ai/tests/golden_survival_justice.rs` under the `golden-survival` ignored lane. Assertions:
- All existing branches' state preconditions hold (accusation, fine).
- Eventually a bounty `SocialArtifact` materializes whose `BountyTerms.reward_source == RewardSource::InstitutionalTreasury { treasury_entity: <Market Warden office> }`.
- A `RewardEncumbrance` exists on the Market Warden office matching the bounty's `commodity` and `quantity`.
- The owning agent's survival-health contract still passes.

### 3. Regenerate golden docs

Run `python3 scripts/golden_inventory.py --write --check-docs` to update `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and the relevant file under `docs/generated/golden-scenario-details/`.

### 4. Roadmap update

Update `specs/IMPLEMENTATION-ORDER.md` S125 entry to mark `✅ COMPLETED — archived at archive/specs/S125-institutional-treasuries-and-bounty-funding.md` (per `docs/archival-workflow.md`) once tickets 001-008 land. Archive the spec from `specs/` to `archive/specs/`.

## Files to Touch

- `scenarios/survival-justice.ron` (modify)
- `crates/worldwake-ai/tests/golden_survival_justice.rs` (modify — new test added)
- `docs/generated/golden-e2e-inventory.md` (regenerate)
- `docs/generated/golden-scenario-index.md` (regenerate)
- `docs/generated/golden-scenario-details/` (regenerate — file under this directory corresponds to survival-justice)
- `specs/IMPLEMENTATION-ORDER.md` (modify — mark S125 ✅ COMPLETED on landing)
- `archive/specs/S125-institutional-treasuries-and-bounty-funding.md` (move from `specs/` per `docs/archival-workflow.md`)

## Out of Scope

- Stale-balance memory for non-co-located holders — S125 OQ3, deferred.
- Faction treasuries — S125 Non-Goal.
- Modifying the three existing goldens — they must continue to pass without modification.
- Engine-level perception-scoping fixes if ticket 002's container scoping turns out to be incomplete — file as a new ticket dependency per Assumption #6.

## Acceptance Criteria

### Tests That Must Pass

1. New: `survival_justice_proves_institutional_bounty_posted`.
2. Existing must continue to pass without modification: `survival_justice_proves_accusation_substrate` (line 535), `survival_justice_proves_fine_punishment_for_same_theft_case` (line 594), `survival_justice_proves_search_and_report_found` (line 640).
3. Existing suite under the `golden-survival` lane: `cargo test -p worldwake-ai golden_survival_justice -- --ignored` (or the equivalent live invocation per `docs/golden-e2e-testing.md`).

### Invariants

1. The three existing survival-justice goldens continue to pass without modification — proves the treasury authoring did not perturb theft-scene perception.
2. The bounty's `reward_source` matches `InstitutionalTreasury { treasury_entity: <Market Warden office> }`.
3. A `RewardEncumbrance` is observable on the Market Warden office in the asserted final state.
4. After regeneration, `docs/generated/golden-*` reflects the new test name.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_justice.rs` — `survival_justice_proves_institutional_bounty_posted` (new).

### Commands

1. `cargo test -p worldwake-ai golden_survival_justice -- --ignored` (targeted ignored-lane run for survival-justice goldens).
2. `python3 scripts/golden_inventory.py --write --check-docs` (regenerate golden docs).
3. `scripts/verify.sh`
