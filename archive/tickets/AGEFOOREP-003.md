# AGEFOOREP-003: Theft survival has a founded food path under spoilage

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: No production engine changes — authored scenario and golden contract truthing
**Deps**: Follows from `archive/specs/S178-perishable-food-spoilage.md` and `archive/tickets/AGEFOOREP-001.md`.

## Problem

`scenarios/survival-theft.ron` proves a concealed displayed-lot theft branch: staged visible stock -> theft -> self-consume, with physical aftermath and later suspicion relay. The scenario intentionally gives the thief no coin, no harvestable food source, and no remote food fallback. With S178 spoilage enabled, this finite displayed stock branch exceeds the authored hunger critical-run envelope.

Before this ticket, the `commodity_perish_profile: {}` opt-out in `scenarios/survival-theft.ron` was temporary scenario containment, not completion. The follow-up needed either to define a founded theft-survival path under spoilage or revise the scenario's survival contract honestly.

## Assumption Reassessment (2026-06-02)

1. The motivating golden is `scenarios::survival_theft::survival_theft_proves_concealed_staged_lot_branch` in `crates/worldwake-ai/tests/scenarios/survival_theft.rs`.
2. The scenario comment explicitly excludes harvestable and remote fallback food for the thief, so direct harvest replenishment from AGEFOOREP-001 cannot make this scenario survive.
3. The live `GoalKind` surfaces under audit are `StealItem { target_item }`, `ConsumeOwnedCommodity`, and any lawful follow-on food acquisition goal that preserves the theft branch.
4. The shared abstraction boundary is ownership/custody/access around displayed sale lots plus thief belief of visible food, theft aftermath, and owner expectation/suspicion.
5. FOUNDATIONS alignment: the fix must preserve FND-4 explicit transfer/source-sink accounting, FND-10 theft aftermath, FND-14B belief-backed planner targets, FND-17 expectation-based missing-stock discovery, FND-20 reusable agent reasoning, and FND-24 ownership/custody/access separation.
6. Reassessment exposed this as a separate bug from AGEFOOREP-001: anonymous harvest-source workstation tagging fixes direct harvest execution, while this ticket owns the intentionally non-harvest theft survival contract.
7. Live proof without the opt-out showed the theft branch still runs deterministically, but a 60-unit apple lot leaves the thief above critical hunger for 408 consecutive ticks against the authored 220-tick limit.
8. The founded contract that landed here is a larger concrete local displayed apple lot: the merchant starts with 120 apples, has enough authored carry capacity to hold the starting stock, and the display container can stage that full lot. No harvestable food source, remote fallback, coin, hidden durable food, or production AI shortcut was added.
9. The golden harness now asserts `ScenarioDef.commodity_perish_profile == None` and that the spawned world has an Apple perish profile, proving the scenario uses default S178 spoilage rather than an empty-map opt-out.
10. `cargo test -p worldwake-ai` failed in seven library tests on both this worktree and a clean `HEAD` (`6d627d68`) baseline. That pre-existing broad-gate blocker was split to `archive/tickets/AILIBBASE-001.md`.

## Architecture Check

The theft scenario should not be made green by silently adding durable food, hidden fallback supply, or scenario-specific planner rails. A lawful solution must either let the thief repeatedly acquire visible food through ordinary theft/planning as stock remains available, create a concrete local replenishment/supply process that keeps the theft branch truthful, or narrow the golden's survival contract if finite displayed stock is the intended authored limit.

## Verified Layers

1. Thief sees and targets only locally visible or belief-backed displayed food -> decision trace for `StealItem`.
2. Theft transfers concrete item lots and leaves aftermath -> action trace plus authoritative ownership/custody/location state.
3. Owner suspicion still comes from expectation mismatch and evidence, not omniscience -> event/social observation trace.
4. Survival envelope passes with spoilage enabled -> affected golden and deterministic replay.
5. Apple spoilage opt-out is absent -> golden harness assertion on loaded `ScenarioDef` plus spawned world perish profile.

## Landed Changes

### 1. Theft Food Path Reassessment

Confirmed the live failure was finite stock under S178 spoilage, not missing theft candidate generation, ranking, or search. The original 60-unit lot exceeded the authored hunger critical-run envelope under spoilage.

### 2. Founded Contract Update

Removed the scenario `commodity_perish_profile: {}` opt-out. Increased Merchant Sera's concrete apple stock to 120 and carry capacity to 400 so the larger lot is lawful, while keeping the market display capacity at 120 so the whole visible lot can be staged and stolen. Updated the golden expected stolen quantity and added assertions that default Apple spoilage is active.

## Landed Files

- `scenarios/survival-theft.ron` — removed the spoilage opt-out; raised Merchant Sera's carry capacity and concrete Apple lot quantity to keep the theft-survival path founded under default spoilage.
- `crates/worldwake-ai/tests/scenarios/survival_theft.rs` — updated expected stolen Apple quantity, asserted that the scenario uses the default perishable-food profile with Apple spoilage enabled, and refreshed generator-facing scenario metadata.
- `docs/generated/golden-scenario-details/survival-theft.md`, `docs/generated/golden-scenario-index.md` — regenerated after the source metadata update so the published golden detail records the full spoilage-enabled contract.
- `archive/tickets/AILIBBASE-001.md` — follow-up for the pre-existing `cargo test -p worldwake-ai` library failures discovered during broad verification.

## Out of Scope

- Merchant substitute-market restock/restage; completed by `archive/tickets/AGEFOOREP-002.md`.
- Direct anonymous harvest-source workstation tagging; owned by `archive/tickets/AGEFOOREP-001.md`.

## Acceptance Result

### Tests

1. Focused golden coverage proves the selected theft-survival contract at the strongest relevant layer for this scenario.
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_theft::` passed.
3. `cargo test -p worldwake-ai` was run and failed on seven pre-existing clean-baseline library failures, then tracked by `archive/tickets/AILIBBASE-001.md`.

### Invariants

1. The thief still cannot plan from non-local, non-believed food or ownership facts.
2. The stolen and consumed food remains one explicit displayed Apple lot with transfer/source-sink history and theft aftermath.
3. The scenario no longer hides spoilage through a scenario-level empty perish-profile map.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_theft.rs` — existing golden now asserts the scenario uses default spoilage and still proves the concealed staged-lot theft branch.

### Executed Commands

1. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_theft::`
2. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-06-02.

The `survival-theft` scenario now runs under the default S178 perishable-food profile instead of opting out with `commodity_perish_profile: {}`. The founded food path is still the original local branch: Merchant Sera stages a visible owned Apple lot, Thief Rana selects and commits `StealItem`, the lot transfers into the thief's possession, the thief eats afterward, and the theft aftermath/suspicion relay remains intact. The concrete authored supply was raised from 60 to 120 apples, with matching lawful carry/display capacity, so the 1440-tick survival envelope passes under spoilage without adding harvest, coin, remote fallback, or scenario-specific AI rails.

Post-ticket review found the generated survival-theft detail page was publishing truncated first-line `Setup`, `Proves`, and `Cross-system chain` metadata from wrapped source comments. That blocker was resolved by rewriting the source metadata as complete generator-facing first-line fields and rerunning the golden inventory generator.

Outcome amended: 2026-06-02. The package-level AI blocker split out during this ticket was restored and archived as `archive/tickets/AILIBBASE-001.md`.

## Deviations

- No production AI files changed. Reassessment showed candidate generation, ranking, and search already support the theft branch; the missing slice was the authored scenario contract under S178 spoilage.
- At completion time, `cargo test -p worldwake-ai` was blocked by pre-existing clean-baseline library failures. Follow-up `archive/tickets/AILIBBASE-001.md` later restored that broad package gate.

## Verification Result

- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_theft::`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Waived `cargo test -p worldwake-ai` as completion proof for AGEFOOREP-003 because the same seven library failures reproduced on clean `HEAD` (`6d627d68`) without AGEFOOREP-003 edits; later restored by `archive/tickets/AILIBBASE-001.md`.
