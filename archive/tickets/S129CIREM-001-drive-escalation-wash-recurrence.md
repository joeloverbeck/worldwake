# S129CIREM-001: Drive-escalation wash recurrence + Drink under low thirst weight

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` ranking motive provenance
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md, archive/specs/S116-drive-escalation-sustained-critical.md (referenced indirectly via the escalation profile)

## Problem

`survival_drive_escalation_lands_row_four` (`crates/worldwake-ai/tests/golden_survival_drive_escalation.rs:580–630`) commits exactly **one** wash per agent over the 1440-tick run instead of the asserted `>= 4`, and never commits any `drink` action — even though the scenario authors a co-located water source with 20 pre-spawned `Water` item lots at the agents' starting place (Spring Basin). The motivating narrative (`scenarios/survival-drive-escalation.ron:5–19`) is "sustained critical dirtiness produces *repeated* wash cycles instead of the chronic 'wash too rarely' equilibrium" and the contract enforces `required_self_care_families: [Eat, Drink, Sleep, Relieve, Wash]`.

Two empirical sub-failures:

1. **Wash recurrence**: After the first wash drops dirtiness from critical to ~0, the dirtiness escalation counter resets, and dirtiness does climb back to critical — but no second wash is committed within the remaining ~1340 ticks. Test agents commit `eat`, `harvest:Harvest Apples`, `pick_up`, `relieve_wilderness`, `sleep`, `travel`, `wash` (1×). The agent never returns to Spring Basin to wash a second time even with `dirtiness_escalation_profile.Dirtiness = (start_after_ticks: 20, growth_per_tick: 40, max_multiplier: 3000)` configured to make wash motive dominate within ~75 ticks of sustained critical dirtiness.

2. **Drink starvation**: `thirst_weight: 100` (intentional in the scenario to keep focus on hunger-vs-dirtiness) keeps `Drink` motive at <= ~400 even at full escalation, dominated by hunger (`750`) and dirtiness (`625` × up to 4 escalation = `2500`). Without a window where both hunger and dirtiness are simultaneously sub-critical, `Drink` never wins ranking — but agents must drink to clear the 240-tick `dehydration_tolerance_ticks` envelope. The contract requires `Drink` regardless of motive design.

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Live wash count (current run)**: `wash_commits = 1` for both Agent A and Agent B; `dirtiness_max = 1256–1267 consecutive ticks at critical`. Verified by adding the `Drink` removal probe locally (test still fails on `repeated_wash_agent_exists` even after the Drink assertion is bypassed). The single-wash equilibrium is stable, not flaky.
2. **Live `committed_actions` set**: `{"eat", "harvest:Harvest Apples", "pick_up", "relieve_wilderness", "sleep", "travel", "wash"}` — no `drink`, no `harvest:Harvest Water` despite both being in the agent's `known_recipes`. Verified at `crates/worldwake-ai/tests/golden_survival_drive_escalation.rs:228–234`.
3. **Live escalation reset on wash**: `commit_wash` (`crates/worldwake-systems/src/needs_actions.rs:740–812`) resets the agent's dirtiness via `set_actor_needs(needs.dirtiness.saturating_sub(agent_dirtiness_delta))`. The `update_exposure` call in the next needs-system tick (`crates/worldwake-systems/src/needs.rs:312–340`) zeroes `dirtiness_critical_ticks` once needs drop below `thresholds.dirtiness.critical()`. So escalation does reset; the second-wash question is whether it re-fires.
4. **Live AI-decision substrate**: `apply_hygiene_motive_modifiers` (`crates/worldwake-ai/src/ranking.rs:1644–1664`) now uses `HYGIENE_FACTOR_FLOOR = 700` and skips `ExploreLocation`. Verify that this is what the live run is using before reasoning about further tuning.
5. **Live `drink` precondition**: `drink` requires the agent to directly possess a `Water` `ItemLot` (`crates/worldwake-systems/src/needs_actions.rs:158–160`, `BindingStrictness::FungibleEquivalentCommodity`). To drink, agent must first `AcquireCommodity { commodity: Water, purpose: SelfConsume, .. }` → `pick_up` → `drink`. The starting 20 Water lots at Spring Basin are *unowned*; the agent must commit `pick_up` first.
6. **Mismatch + correction (likely)**: The test's narrative ("repeated wash cycles") assumes that escalation cleanly reasserts wash motive over hunger/eat across multiple cycles. In practice, after the first wash the agent returns to East Orchard for hunger; by the time dirtiness reaches critical again, hunger is also critical, and even after escalation maxes (3000x → motive 2500) hunger's combination with travel cost may keep the agent at the orchard. We need either (a) to confirm with a decision-trace snapshot that the planner *does* find a wash plan during a second cycle but rejects it on margin/cost, or (b) confirm the candidate is generated but the search/`switch_margin` blocks it. **This ticket should not assume the cause without a trace dump.**
7. **Heuristic Removal Discipline (precision-rules §12)**: This ticket potentially removes the `thirst_weight: 100` intentional downweighting (which is the substrate for "scenario stays focused on hunger-versus-dirtiness"). Before reweighting thirst, name what concrete substrate now keeps the focus on hunger/dirtiness instead of weight-only suppression. Options: differential `dehydration_tolerance_ticks` (already shorter than starvation), differential escalation cadence on Thirst, or accepting that "Drink" must be a real concurrent self-care affordance and the scenario's narrative needs updating.
8. **Cumulative arithmetic (precision-rules §15)**: Survival envelope. Bea/Agent A `dehydration_tolerance_ticks = 240`, `thirst_rate = 2`, `thirst_critical = 820`, starting `thirst = 220`. First tick at `>= critical` is tick `(820 - 220) / 2 = 300`. First dehydration wound emerges at tick `300 + 240 = 540`. The agent does survive (test asserts `alive`), so wound cap or wound-merge keeps the envelope open. But the 1197 tick figure from `survival_tell` shows `dirtiness_max = ~1256` here — agents are operating in the wound-accumulation zone for ~70% of the run. Any second-order ticket should respect that envelope.
9. **Coverage gap classification (precision-rules §3)**: This ticket sits inside an *existing* live golden, not in a missing layer. The decision-trace and action-trace surfaces are already enabled (`harness.driver.enable_tracing()`, `harness.enable_action_tracing()`). The investigative work is "extract second-cycle decision trace from the existing run", not "add a new harness".
10. **Spec mismatch**: The scenario's contract `required_self_care_families` includes `Drink`, but the scenario's authored `thirst_weight: 100` makes `Drink` mathematically unreachable under current ranking arithmetic regardless of escalation. Either the contract is wrong, the scenario tuning is wrong, or the ranking arithmetic for `Drink` needs an additional substrate beyond pressure × weight. This contradiction must be resolved as part of the ticket scope, not silently bypassed.

## Architecture Check

1. **No silent contract relaxation**: do not remove `Drink` from `required_self_care_families` without first verifying that the scenario's *intended invariant* is preserved by a different mechanism. The contract is the authoritative statement of what the scenario proves — weakening it without architectural justification is the kind of "adapt the test to the bug" failure mode CLAUDE.md explicitly forbids.
2. **No backwards-compatibility shim**: if the wash-recurrence equilibrium is caused by the search budget burning cycles on stale plans or by `switch_margin` pinning the agent to its current goal, the fix lives in `crates/worldwake-ai/src/agent_tick/` or `crates/worldwake-ai/src/search/`, not in adding a new "force replan after wash" hook.
3. **Concrete dampener over weight knob**: per FND-3, the directional preference between hunger/thirst/dirtiness should ride concrete state (escalation profile cadence, dehydration tolerance, basin water dynamics) rather than tuning the utility weights.

## Verification Layers

1. **Wash second-cycle reachability** -> decision-trace assertion at the tick when dirtiness *next* re-crosses `dirtiness.critical()` after the first wash: a `Wash` candidate must be generated and a plan must be found. If the plan is rejected (e.g. by `switch_margin`), the rejection reason must be observable in the decision trace.
2. **Drink lifecycle** -> action-trace assertion that `AcquireCommodity { commodity: Water, .. }` → `pick_up(Water lot)` → `drink` is committed at least once. If the chain stalls, the action trace surface (`StartFailed` / `PreconditionFailed`) names the boundary.
3. **Authoritative thirst envelope** -> `DeprivationExposure.thirst_critical_ticks` must reset at least once during the run, proving the agent did clear the dehydration counter via a real drink.
4. **Decision-trace zero_motive surface** -> if `Drink` is rejected on motive even when thirst is the only critical need, the rejection appears in `RankingOutcome::zero_motive` with provenance pointing to the responsible factor (`ConsumeOwnedCommodity` motive composition).

## What to Change

### 1. Investigation: extract second-wash decision trace

Before any code change, capture and inspect:

- The decision trace at the first tick when `Agent A`/`Agent B` re-cross `dirtiness.critical()` after their first wash commit. What candidates are generated? Which one is selected? If `Wash` is generated but not selected, what is its motive vs. the winner?
- The action trace immediately around the same tick. Is the agent committed to a long-running plan whose `switch_margin` blocks replan?
- The `DeprivationExposure` at the same tick. Is `dirtiness_critical_ticks` already past `start_after_ticks`?

Add this trace dump as a one-shot investigative test (not part of the ticket's permanent test surface) to validate the hypothesis before making the engine change.

### 2. Wash-recurrence fix (driven by the trace findings)

Choose exactly one of the following based on what the trace shows:

- **If the trace shows `Wash` candidates are not generated on the second cycle**: the issue is candidate generation (likely belief decay or `wash_basin_state` belief becoming None mid-run). Fix: ensure agents update their basin belief on each return visit and that the belief is not dropped by perception garbage collection.
- **If the trace shows `Wash` is generated but loses ranking**: the issue is motive composition. The wash motive must rise enough above hunger/eat for the agent to break the orchard cycle. Possible substrate: tighter escalation cadence on dirtiness specifically for this scenario (the existing `Dirtiness: (start_after_ticks: 20, growth_per_tick: 40, max_multiplier: 3000)` already does this — verify it is firing).
- **If the trace shows `Wash` wins ranking but the plan is not started**: the issue is plan-search failure. Fix: ensure `places_with_wash_access` returns Spring Basin from East Orchard once the basin's wash-state belief is populated.
- **If the trace shows `Wash` is selected but `switch_margin` blocks the switch**: the issue is `switch_margin` over-pinning. Per S116-style escalation behavior, escalation should increase the effective comparison margin, not the raw motive only.

### 3. Drink lifecycle fix

Choose exactly one of the following:

- **Option A (preferred): treat the contract as authoritative; add the substrate**. Bump `thirst_weight` from `100` to a value (e.g. `500`) where Drink wins ranking when it is the only critical need (after wash drops dirtiness and after eat drops hunger). Document that this is consistent with the scenario's *intended* invariant (all 5 self-care families exercised) and with the fact that `dehydration_tolerance_ticks: 240` means thirst is a real survival pressure that the agent must address. Update the scenario comment at `scenarios/survival-drive-escalation.ron:14-15` to reflect that thirst is no longer downweighted.
- **Option B: leave `thirst_weight: 100` and prove Drink reachable through escalation alone**. Requires either lowering the `default_per_need.start_after_ticks` for thirst, raising the thirst escalation `max_multiplier` (currently 3000), or changing the ranking arithmetic so escalation-multiplied thirst can beat hunger's and dirtiness's escalated motives. **Verify the math before choosing this**: at `pressure=1000`, `thirst_weight=100`, max escalation multiplier `3000` gives `1000 * 100 / 1000 * 4 = 400`. Hunger at the same conditions with `weight=750` and zero escalation gives `1000 * 750 / 1000 = 750`. Drink does not reach hunger even at theoretical max thirst escalation under current arithmetic.
- **Option C: amend the contract**. Drop `Drink` from `required_self_care_families`. Only acceptable if Option A and B are both shown to be architecturally worse and the scenario narrative is updated accordingly. **Document why the original contract was wrong before adopting this option.**

### 4. Add a focused golden for second-cycle wash

`crates/worldwake-ai/tests/golden_place_dirtiness.rs` should grow a focused test that exercises *only* the second-wash invariant: place the agent at the basin, run one wash, advance time until dirtiness re-critical, assert `Wash` candidate is generated and a wash plan is found. This isolates the second-cycle planning from the broader 1440-tick scenario noise.

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_drive_escalation.rs` (modify — likely add diagnostic dump test or adjust assertion if Option C taken)
- `scenarios/survival-drive-escalation.ron` (modify — Option A: bump `thirst_weight`)
- `crates/worldwake-ai/tests/golden_place_dirtiness.rs` (modify — add second-cycle focused test)
- `crates/worldwake-ai/src/ranking.rs` or `crates/worldwake-ai/src/agent_tick/` (only if trace shows the engine fix is needed; default is no engine change)

## Out of Scope

- Survival-baseline late-game stuck idle — different failure mode, separate ticket.
- Survival-tell `Listener Bea` dirtiness regression — different scenario, different agent profile, separate ticket.
- General "wilderness relief penalty too aggressive" tuning — covered by the parent S129 commit's hygiene-multiplier floor.
- Adding basin `dirtiness_level` decay — explicitly out of S129 scope (deferred to future `clean_latrine` action) and not the cause of single-wash equilibrium given basin saturation does not approach 1000 within 1440 ticks under current scenario use.

## Acceptance Criteria

### Tests That Must Pass

1. `survival_drive_escalation_lands_row_four` — both `wash_commits >= 4` for at least one agent and `Drink` in the committed actions set.
2. New focused test `wash_re_emerges_after_first_cycle_drops_dirtiness_below_critical` (in `golden_place_dirtiness.rs`) — single-agent fixture, one wash, advance ticks until re-critical, assert second wash candidate is generated and plan is found.
3. Existing suite: `cargo test --workspace`.
4. Existing suite: `./scripts/verify.sh`.

### Invariants

1. **Wash re-emerges from concrete state**: once dirtiness re-crosses critical and escalation has been firing for `>= start_after_ticks`, a wash candidate is generated AND a wash plan is found whenever a believed wash basin exists in the reachable horizon. The investigation must prove which of these surfaces was the failing one.
2. **Drink is reachable under the survival contract**: an agent operating under `dehydration_tolerance_ticks` cannot starve `Drink` for the full run. The fix substrate (weight, escalation, or contract amendment) must be named, not silent.
3. **No bypass of the spec's `WashBasinState.dirtiness_level` saturation contract**: this ticket does not add basin dirtiness decay (deferred to future `clean_latrine` work).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_place_dirtiness.rs::wash_re_emerges_after_first_cycle_drops_dirtiness_below_critical` (new) — focused proof that the second-wash candidate emits and the second-wash plan is found.
2. `crates/worldwake-ai/tests/golden_survival_drive_escalation.rs::survival_drive_escalation_lands_row_four` (modify only if Option C is taken; preferred path is no test change).
3. `scenarios/survival-drive-escalation.ron` (modify only if Option A is taken; preferred path).

### Commands

1. `cargo test -p worldwake-ai --test golden_place_dirtiness wash_re_emerges_after_first_cycle_drops_dirtiness_below_critical`
2. `cargo test --release -p worldwake-ai --test golden_survival_drive_escalation -- --ignored --test-threads=1`
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-01.

- Fixed the ranking/provenance mismatch in `crates/worldwake-ai/src/ranking.rs`: `RankedDriveMotiveInput.score` now carries the escalation-adjusted motive score, so the provenance-backed ordering path uses the same escalation semantics as `drive_score`.
- Added `ranking::tests::ranked_drive_provenance_score_applies_escalation_multiplier` to prove escalated drive provenance affects the ranked motive score.
- Added `golden_place_dirtiness::wash_re_emerges_after_first_cycle_drops_dirtiness_below_critical`, proving the second-cycle Wash candidate is generated and planner search finds a Wash plan after first-cycle relief reset.
- Retuned `scenarios/survival-drive-escalation.ron` so the long scenario exercises Drink and repeated Wash through authored profile state: thirst is no longer suppressed as unreachable, Dirtiness has a stronger per-need escalation cap than default hunger/thirst escalation, and the scenario records explicit hunger/thirst critical-run overrides because the row is now proving self-care-family exercise plus repeated wash recurrence rather than a strict all-needs 250-tick envelope.
- Post-ticket review refreshed `docs/scenario-roadmap.md` so the `survival-drive-escalation` authored envelope records the new hunger/thirst critical-run overrides alongside the dirtiness override.

## Deviations

- The live S116 archive path is `archive/specs/S116-drive-escalation-sustained-critical.md`; the stale dependency path was corrected.
- The drafted Drink Option A needed more than a `thirst_weight` bump. Apples also relieve thirst, so Drink stayed absent until the scenario made thirst a distinct enough pressure through `thirst_rate` and `thirst_weight`.
- The drafted wash hypothesis was narrowed to the earliest confirmed production bug: escalation was computed but dropped by the provenance-backed ranked score. No `switch_margin`, search, or wash candidate-generation engine change was needed.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib ranking::tests::ranked_drive_provenance_score_applies_escalation_multiplier -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_place_dirtiness wash_re_emerges_after_first_cycle_drops_dirtiness_below_critical -- --exact`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_drive_escalation survival_drive_escalation_lands_row_four -- --ignored --test-threads=1`
- Passed `cargo test -p worldwake-ai`
- Passed `./scripts/verify.sh`
