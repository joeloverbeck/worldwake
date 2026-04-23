# Scenario Roadmap

Cross-reference:
- [Generated scenario coverage](generated/scenario-coverage.md)
- [Foundational principles](FOUNDATIONS.md)
- [Golden E2E testing conventions](golden-e2e-testing.md)
- [Agent profile reference](profiles/all-profiles.md)

This document is the hand-authored editorial companion to [generated scenario coverage](generated/scenario-coverage.md). The generated file answers structural activation from live `scenarios/*.ron`; this roadmap answers ordering, scenario validity, and what counts as a truthful landing.

## 1. Preamble / Philosophy

Scenario-backed goldens are now the canonical proof shape for feature integration. A feature is not "landed" in this roadmap because a unit golden exists, because a scenario survives for 1440 ticks, or because a profile field is structurally present. It lands when all three of these are true:

1. The scenario structurally activates the feature under the live detection rule.
2. The backing golden proves the authored behavior actually occurs.
3. The golden proves the scenario passed for the authored causal reason, not merely through a rival lawful branch.

The roadmap therefore follows four standing rules:

- Survival is a coexistence invariant. Future feature scenarios keep the proven survival loop alive while introducing one new architectural stressor at a time.
- Priority order is driven by architectural risk, not by player-facing importance. The next scenario should be the one most likely to destabilize belief-only planning, local information flow, or cross-system composition.
- Structural activation and behavioral proof are different layers. `scenario-coverage` proves the first; the scenario golden proves the second.
- Auxiliary scenarios stay auxiliary until they meet the same landing contract. A golden-backed scenario without a survival-health contract or without authored feature activation can provide useful evidence, but it does not become a landed roadmap row by implication.

This split is required by Worldwake's foundations: belief-only planning and local causality must stay explicit ([FND-14](FOUNDATIONS.md#14-world-state-is-not-belief-state), [FND-7](FOUNDATIONS.md#7-locality-of-motion-interaction-and-communication)), and performance or test convenience cannot blur the actual causal contract ([FND-12](FOUNDATIONS.md#12-performance-may-compress-computation-never-causality)).

## 2. Gameplay Feature Catalog

This catalog mirrors the live `FEATURES` table in [`scenario_coverage.rs`](../crates/worldwake-cli/src/bin/scenario_coverage.rs). Feature names here must stay byte-for-byte aligned with the generated companion.

| Feature | Activation signal | Backing systems / sources | Current roadmap status |
|---|---|---|---|
| Basic needs (Eat) | `needs` + non-zero hunger utility + `metabolism_profile` + `drive_thresholds` + food recipe path | [`needs.rs`](../crates/worldwake-systems/src/needs.rs), [`needs_actions.rs`](../crates/worldwake-systems/src/needs_actions.rs) | Landed in [§5.1](#51-landed-1-survival-baseline) |
| Basic needs (Drink) | `needs` + non-zero thirst utility + `metabolism_profile` + `drive_thresholds` + water recipe path | [`needs.rs`](../crates/worldwake-systems/src/needs.rs), [`needs_actions.rs`](../crates/worldwake-systems/src/needs_actions.rs) | Landed in [§5.1](#51-landed-1-survival-baseline) |
| Basic needs (Sleep) | `needs` + non-zero fatigue utility + `metabolism_profile` + `drive_thresholds` | [`needs.rs`](../crates/worldwake-systems/src/needs.rs), [`needs_actions.rs`](../crates/worldwake-systems/src/needs_actions.rs) | Landed in [§5.1](#51-landed-1-survival-baseline) |
| Basic needs (Relieve) | `needs` + non-zero bladder utility + `metabolism_profile` + `drive_thresholds` | [`needs.rs`](../crates/worldwake-systems/src/needs.rs), [`needs_actions.rs`](../crates/worldwake-systems/src/needs_actions.rs) | Landed in [§5.1](#51-landed-1-survival-baseline) |
| Basic needs (Wash) | `needs` + non-zero dirtiness utility + `metabolism_profile` + `drive_thresholds` + wash-capable water path | [`needs.rs`](../crates/worldwake-systems/src/needs.rs), [`needs_actions.rs`](../crates/worldwake-systems/src/needs_actions.rs) | Landed in [§5.1](#51-landed-1-survival-baseline) |
| Travel physiology | Any non-zero travel multiplier or wilderness relief dirtiness penalty in `metabolism_profile` | [`needs.rs`](../crates/worldwake-systems/src/needs.rs), [`travel_actions.rs`](../crates/worldwake-systems/src/travel_actions.rs) | Landed in [§5.2](#52-landed-2-survival-scattered) |
| Drive escalation | Authored `drive_escalation_profile` present and non-default | [`drive_escalation_profile.rs`](../crates/worldwake-core/src/drive_escalation_profile.rs), [`needs.rs`](../crates/worldwake-systems/src/needs.rs) | Landed in [§5.4](#54-landed-4-survival-drive-escalation) |
| Need-driven exploration | `exploration_profile` active | [`exploration.rs`](../crates/worldwake-core/src/exploration.rs), AI planner + perception | Landed in [§5.1](#51-landed-1-survival-baseline) |
| Activation-decay perception | `perception_profile` active | [`perception.rs`](../crates/worldwake-systems/src/perception.rs) | Landed in [§5.1](#51-landed-1-survival-baseline) |
| Place concealment | Any place `visibility_profile.base_concealment > 0` | [`observation_context.rs`](../crates/worldwake-core/src/observation_context.rs), [`perception.rs`](../crates/worldwake-systems/src/perception.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Tell / peer info transfer | Active `tell_profile` plus active `communication_profile` | [`tell_actions.rs`](../crates/worldwake-systems/src/tell_actions.rs), [`communication.rs`](../crates/worldwake-core/src/communication.rs) | Landed in [§5.5](#55-landed-5-survival-tell) |
| Ask-about-person | Non-zero `social_weight` plus `communication_profile` and `epistemic_disposition` | [`ask_about_person_actions.rs`](../crates/worldwake-systems/src/ask_about_person_actions.rs), epistemic surfaces | Landed in [§5.6](#56-landed-6-survival-ask-consult) |
| Consult-record | Non-zero `social_weight` plus `perception_profile` and record-bearing world state | [`consult_record_actions.rs`](../crates/worldwake-systems/src/consult_record_actions.rs) | Landed in [§5.6](#56-landed-6-survival-ask-consult) |
| Obligation satiation | `obligation_satiation_profile` present | [`obligation.rs`](../crates/worldwake-core/src/obligation.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Diversification / curiosity | `diversification_profile` present | [`diversification.rs`](../crates/worldwake-core/src/diversification.rs) | Landed in [§4.3](#43-landed-row-7) |
| Experience preferences | `preference_profile` present | [`experience.rs`](../crates/worldwake-core/src/experience.rs), [`experience_recording.rs`](../crates/worldwake-systems/src/experience_recording.rs) | Landed in [§4.3](#43-landed-row-7) |
| Production (facility-backed craft) | Authored recipe set with at least one non-harvest production recipe | [`production_actions.rs`](../crates/worldwake-systems/src/production_actions.rs) | Landed in [§5.8](#58-landed-8-survival-production) |
| Merchant selling | `merchandise_profile` present | [`trade_actions.rs`](../crates/worldwake-systems/src/trade_actions.rs) | Landed in [§5.9](#59-landed-9-survival-trade) |
| Trade negotiation | `trade_disposition` present | [`trade_actions.rs`](../crates/worldwake-systems/src/trade_actions.rs) | Landed in [§5.9](#59-landed-9-survival-trade) |
| Commodity valuation | `commodity_valuation` present | [`trade_actions.rs`](../crates/worldwake-systems/src/trade_actions.rs) | Landed in [§5.9](#59-landed-9-survival-trade) |
| Substitute preferences | `substitute_preferences` present | [`trade.rs`](../crates/worldwake-core/src/trade.rs) | Landed in [§5.9](#59-landed-9-survival-trade) |
| Item decay | `commodity_decay` authored on the scenario | [`item_decay.rs`](../crates/worldwake-systems/src/item_decay.rs) | Landed in [§5.10](#510-landed-10-survival-items-decay) |
| Disposal | `disposal_profile` present | [`disposal.rs`](../crates/worldwake-core/src/disposal.rs) | Landed in [§5.10](#510-landed-10-survival-items-decay) |
| Facility-queue contention | `contention_disposition` present | [`facility_queue_actions.rs`](../crates/worldwake-systems/src/facility_queue_actions.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Offices / succession / force-claim | Office entities plus force-claim world state | [`office_actions.rs`](../crates/worldwake-systems/src/office_actions.rs), [`offices.rs`](../crates/worldwake-core/src/offices.rs) | In progress in [§4.7](#47-in-progress-row-11) |
| Bounty posting | Non-zero `bounty_posting_weight` plus `artifact_posting_profile` | [`artifact_actions.rs`](../crates/worldwake-systems/src/artifact_actions.rs), [`social_artifact.rs`](../crates/worldwake-core/src/social_artifact.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Notice posting | Non-zero `notice_posting_weight` plus `artifact_posting_profile` | [`artifact_actions.rs`](../crates/worldwake-systems/src/artifact_actions.rs), [`social_artifact.rs`](../crates/worldwake-core/src/social_artifact.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Theft | `theft_disposition` present | [`theft.rs`](../crates/worldwake-ai/src/theft.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Justice / accusation | `justice_disposition` present | [`justice_actions.rs`](../crates/worldwake-systems/src/justice_actions.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Violation investigation | `violation_disposition` present | [`investigate_actions.rs`](../crates/worldwake-systems/src/investigate_actions.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Patrol | `patrol_profile` plus `patrol_route` | [`patrol_actions.rs`](../crates/worldwake-systems/src/patrol_actions.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Pursuit | `pursuit_profile` present | [`pursuit.rs`](../crates/worldwake-ai/src/pursuit.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Combat | `combat_profile` present | [`combat.rs`](../crates/worldwake-systems/src/combat.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Escort | Non-zero `care_weight` | [`escort_actions.rs`](../crates/worldwake-systems/src/escort_actions.rs) | Planned; structural coverage only in `cli-evaluation.ron` |
| Bandit camps | Authored bandit-agent + camp world state | bandit systems and camp actions | Planned |
| Report / witness | Active `perception_profile` + active `tell_profile` + active `communication_profile` | report/tell pipeline | Planned; current survival scenarios keep this gated inactive |
| Search | `violation_disposition` plus `epistemic_disposition` | search and investigation actions | Planned; structural coverage only in `cli-evaluation.ron` |
| Stock / transport | `merchandise_profile` plus stock-supporting world state | stock and transport actions | Landed in [§5.9](#59-landed-9-survival-trade) |

Coverage warnings from the generated companion are currently truthful and intentional for this roadmap:

- `intention_disposition` is an authored scenario field but not yet classified as its own gameplay feature row.
- `last_seen_memory` is an authored scenario field but not yet classified as its own gameplay feature row.

Those warnings should remain visible until the project either promotes them into the gameplay-feature catalog or decides they are permanently editorial/supporting fields rather than roadmap features.

## 3. Status Summary

This table is derived from the live generated companion and then narrowed by the roadmap's stronger validity rule.

| Feature status | Features |
|---|---|
| Landed in `survival-baseline.ron` | Basic needs (Eat/Drink/Sleep/Relieve/Wash), Need-driven exploration, Activation-decay perception |
| Landed in `survival-scattered.ron` | Travel physiology |
| Landed in `survival-contested.ron` | No new structural feature rows; the landing is a stronger survival-under-contention proof for the already-landed baseline + travel stack |
| Landed in `survival-drive-escalation.ron` | Drive escalation |
| Landed in `survival-tell.ron` | Tell / peer info transfer |
| Landed in `survival-ask-consult.ron` | Ask-about-person, Consult-record |
| Landed in `survival-preferences.ron` | Diversification / curiosity, Experience preferences |
| Landed in `survival-production.ron` | Production (facility-backed craft) |
| Landed in `survival-trade.ron` | Merchant selling, Trade negotiation, Commodity valuation, Substitute preferences, Stock / transport |
| Landed in `survival-items-decay.ron` | Item decay, Disposal |
| In progress in `survival-offices.ron` | Offices / succession / force-claim |
| Structural coverage only, not yet landed | Place concealment, Obligation satiation, Facility-queue contention, Bounty posting, Notice posting, Theft, Justice / accusation, Violation investigation, Patrol, Pursuit, Combat, Escort, Search |
| Authored but still gated inactive | Report / witness |
| Planned with no current scenario activation | Bandit camps |

The key constraint is that structural activation alone is not a feature landing. `cli-evaluation.ron`, `survival-tell.ron`, and `survival-ask-consult.ron` can expose future substrate without automatically promoting every structurally active row to `Landed`.

## 4. Priority Roadmap

Ordering criterion:

> Architectural risk is the likelihood that adding one more active feature to the proven survival loop will destabilize belief-only planning, local information flow, or state-mediated system composition. Highest priority goes to features that can steal planner attention from self-care, mutate or rely on belief transport, or introduce multi-actor coordination pressure.

### 4.1 Entry Contract Template

Every planned or landed roadmap entry uses this shape:

```markdown
### N. <Feature Name>

**Status**: Planned | Drafting | In Progress | Landed
**Source scenario**: `scenarios/<name>.ron` (or `--` until authored)
**Backing goldens**: `crates/worldwake-ai/tests/golden_<name>.rs` (or `--`)
**Depends on**: prior roadmap rows and any landed specs the row relies on

**Architectural risk rationale**
1-3 sentences on why this feature is risky to add to the survival loop.

**Activation checklist**
- Always-required survival baseline substrate
- Newly activated feature substrate
- Survival-health contract

**Must-exercise behaviors**
- Concrete behaviors the scenario must actually produce

**Must-prove invariants**
- Survival-health contract first
- Feature-specific proof, with named proof surface
- Accepted and excluded rival lawful branches

**Deliberately inactive**
- Cumulative list inherited from the previous landed row

**Done-when**
- Scenario exists
- Golden exists and passes
- Generated companion matches
- Roadmap row can be marked `Landed`
```

Use this template for both planned entries and retrospective landed entries. A row is not complete until the golden proves the authored branch rather than only a broad end-state.

### 4.2 Ordered Roadmap

| # | Scenario focus | New feature scope | Status | Why it sits here |
|---|---|---|---|---|
| 1 | `survival-baseline` | Survival substrate, exploration, activation-decay perception | Landed | Baseline self-care loop |
| 2 | `survival-scattered` | Travel physiology | Landed | First spatial/adversarial stress without new social systems |
| 3 | `survival-contested` | Survival under contention and route invalidation | Landed | Multi-agent pressure on the already-landed stack |
| 4 | `survival-drive-escalation` | Authored drive-escalation coverage inside a survival-health-contract scenario | Landed | Converts the old auxiliary wash-priority proof into a real survival-contract landing before later social features depend on it |
| 5 | `survival-tell` | Tell / peer info transfer | Landed | First belief-mutation feature under survival pressure |
| 6 | `survival-ask-consult` | Ask-about-person + consult-record | Landed | Explicit epistemic actions competing with self-care |
| 7 | `survival-preferences` | Experience preferences + diversification / curiosity | Landed | Proves proactive diversification under survival and a durable familiar-source failure memory that later discounts the stale orchard while selecting the discovered novel grove |
| 8 | `survival-production` | Production (facility-backed craft) | Landed | First survival row where food depends on a workstation-backed craft branch rather than direct harvest |
| 9 | `survival-trade` | Merchant selling, trade negotiation, commodity valuation, substitute preferences, stock / transport | Landed | Multi-agent coordination and ownership-sensitive planning through substitute-backed local trade |
| 10 | `survival-items-decay` | Item decay + disposal | Landed | Ongoing world maintenance pressure added to the landed survival-trade stack |
| 11 | `survival-offices` | Offices / succession / force-claim + notice posting | In Progress | Institution-level goals and artifacts competing with needs |
| 12 | `survival-theft` | Theft + place concealment | Planned | Antagonistic local behavior and hidden-state pressure |
| 13 | `survival-justice` | Justice / accusation + violation investigation + report / witness + search | Planned | Witness chains and evidence-driven social reaction |
| 14 | `survival-patrol` | Patrol + pursuit | Planned | Scheduled duties and interrupt-driven remote pursuit |
| 15 | `survival-combat` | Combat + bandit camps | Planned | Highest direct survival risk and adversarial planning pressure |
| 16 | `survival-escort` | Escort/care | Planned | Coordinated travel after the rest of the hostile world is live |
| 17 | `final-integration` | Full coexistence stack | Planned | Last because it only makes sense once every prior row has an honest scenario contract |

### 4.3 Landed Row 7

### 7. `survival-preferences`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-preferences.ron`](../scenarios/survival-preferences.ron)  
**Backing goldens**: [`golden_survival_preferences.rs`](../crates/worldwake-ai/tests/golden_survival_preferences.rs)  
**Depends on**: landed rows 1-6

`survival-preferences.ron` keeps the survival loop alive while `Scout Ilen` proactively discovers `Novel Grove`, later successfully recovers apples there, and also carries durable failure memory for the stale familiar orchard. The roadmap row now lands both halves of row 7 inside a real 1440-tick survival scenario: diversification / curiosity and experience preferences.

The golden proves the full causal chain at the live seam that matters: proactive exploration reaches `Novel Grove`; the familiar orchard persists a concrete `SourceReliability.failed_attempts` aftermath after the stale-source contradiction is encountered; and a later apple-acquisition planning pass still sees the familiar orchard as a discounted candidate while selecting the novel grove for the causal reason surfaced in the decision trace.

### 4.4 Landed Row 8

### 8. `survival-production`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-production.ron`](../scenarios/survival-production.ron)  
**Backing goldens**: [`golden_survival_production.rs`](../crates/worldwake-ai/tests/golden_survival_production.rs)  
**Depends on**: landed row 1

`survival-production.ron` keeps the baseline self-care loop alive while removing every direct-food fallback and making the baker's only authored food path a mill-backed `Bake Bread` craft branch. That lands row 8 at the truthful live seam: non-harvest production that depends on a workstation and real stocked inputs, not merely another harvest source.

The golden proves the branch at the earliest honest surfaces that matter: a planning trace selects `ProduceCommodity`; the action trace commits `craft:Bake Bread`; authoritative world state materializes Bread; and a later planning pass selects Bread consumption only after that production output exists. The same run also proves the authored survival-health contract for the full 1440 ticks.

### 4.5 Landed Row 9

### 9. `survival-trade`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-trade.ron`](../scenarios/survival-trade.ron)  
**Backing goldens**: [`golden_survival_trade.rs`](../crates/worldwake-ai/tests/golden_survival_trade.rs)  
**Depends on**: landed rows 1-8

`survival-trade.ron` now owns a truthful roadmap landing for the trade row. It keeps a full 1440-tick survival-health contract alive while authoring a merchant facility, staged apple stock, buyer purchasing power, `trade_disposition`, `commodity_valuation`, and `substitute_preferences` in one live scenario instead of relying on the old auxiliary merchant-only goldens.

The golden now proves the full live seam this row needs: `Merchant Sera` commits `stage_stock_for_sale`, a listed apple lot appears at the market, `Buyer Nila` reaches a local `AcquireCommodity(SelfConsume)` substitute branch with an explicit `trade` payload still bound to that apple lot, authoritative apple and coin transfer occur at the trade seam, and both agents satisfy the authored survival contract.

This is now a truthful substitute-isolation scenario rather than merely a bread-market progress row. The authored market exposes only apples for sale, the buyer's food substitutes are ordered `[Apple, Grain]`, and the buyer cannot short-circuit the row through a local listed bread purchase. Focused AI proof from the earlier trade tickets still owns the lower candidate-generation, ranking, and goal-model seams; the roadmap-owned golden now closes the remaining scenario/golden layer by proving that the landed row really is a substitute-driven trade branch for the authored reason.

**Deliberately inactive**
- Place concealment
- Obligation satiation, item decay, disposal, facility contention, offices, theft, justice, patrol, pursuit, combat, bandit camps, escort, and search remain unlanded
- Report / witness remains gated inactive in the authored scenario

### 4.6 Landed Row 10

### 10. `survival-items-decay`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-items-decay.ron`](../scenarios/survival-items-decay.ron)  
**Backing goldens**: [`golden_survival_items_decay.rs`](../crates/worldwake-ai/tests/golden_survival_items_decay.rs)  
**Depends on**: landed rows 1-9

`survival-items-decay.ron` keeps the landed survival-trade substrate alive while adding explicit maintenance pressure through authored `commodity_decay` and a local `disposal_profile`. `Merchant Sera` and `Buyer Nila` keep the substitute-trade branch active at `Market Square`, while `Caretaker Oren` starts above his disposal threshold with one carried Waste lot that must be dropped and later decay away.

The golden proves the row at the earliest honest surfaces that matter: `Caretaker Oren` reaches a real `FreeCarryCapacity` selection, commits `drop_item`, the same tracked Waste lot later receives an `ItemDecay` archive event, and the trade seam still remains live because `Buyer Nila` completes a real apple trade and later commits `eat`. The full survival-health contract is tracked on the merchant and caretaker who own the row's ongoing survival-maintenance pressure, while the buyer remains a supporting causal actor for the substitute-trade witness path.

### 4.7 In Progress Row 11

### 11. `survival-offices`

**Status**: In Progress  
**Source scenario**: [`scenarios/survival-offices.ron`](../scenarios/survival-offices.ron)  
**Backing goldens**: [`golden_survival_offices.rs`](../crates/worldwake-ai/tests/golden_survival_offices.rs)  
**Depends on**: landed rows 1-10

`survival-offices.ron` now owns the truthful row-11 office seam: a 1440-tick survival scenario where `Claimant Rhea` force-claims `Marsh Warden` while still satisfying the authored self-care envelope. This closes the office / succession / force-claim half of the row at the live boundary the runtime actually exposes today: `ClaimOffice` selection under survival pressure, committed `press_force_claim`, authoritative force control, and delayed holder installation.

The row is not landed yet because the posting half still lacks a roadmap-owned survival proof. The live AI currently emits autonomous notice posting only from threat-warning memory, not from office-vacancy uptake, and the current scenario authoring surface cannot yet express the required initial threat-memory substrate cleanly inside `survival-offices`. That blocker is owned by [`tickets/S111OFFNOT-001.md`](../tickets/S111OFFNOT-001.md).

**Scenario-owned progress**
- Offices / succession / force-claim

**Blocked**
- Notice posting: roadmap-owned survival proof still missing; see [`tickets/S111OFFNOT-001.md`](../tickets/S111OFFNOT-001.md)

#### 4.8 Remaining planned rows

Rows 12-17 follow the ordering table above. Each future author should expand the chosen row into the full entry template, inheriting the cumulative deliberately inactive list from the prior landed row and making the validity contract explicit before the scenario is authored.

## 5. Landed Scenarios

### 5.1 Landed #1: `survival-baseline`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-baseline.ron`](../scenarios/survival-baseline.ron)  
**Backing goldens**: [`golden_survival_baseline.rs`](../crates/worldwake-ai/tests/golden_survival_baseline.rs)

**Authored envelope**
- Seed: `104004`
- Agents: `3`
- Places: `4`
- Survival health contract: `max_authored_critical_run_ticks = 100`, `max_idle_window_ticks_with_elevated_need = 20`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`

**Landed feature rows**
- Basic needs (Eat/Drink/Sleep/Relieve/Wash)
- Need-driven exploration
- Activation-decay perception

**Why this golden is valid**

The golden does more than check survival. It proves that the authored survival substrate leads Agent B to reach `Fertile Fields`, perceive the orchard food source there, and do so through the intended exploration/perception chain rather than by a scripted shortcut. The proof surfaces are authoritative place state, belief contents, action traces, and survival-contract assertions in [`golden_survival_baseline.rs`](../crates/worldwake-ai/tests/golden_survival_baseline.rs).

**Deliberately inactive**
- Travel physiology
- Drive escalation as an authored feature row
- All social, trade, justice, office, patrol, pursuit, theft, combat, artifact, and concealment features

### 5.2 Landed #2: `survival-scattered`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-scattered.ron`](../scenarios/survival-scattered.ron)  
**Backing goldens**: [`golden_survival_scattered.rs`](../crates/worldwake-ai/tests/golden_survival_scattered.rs)

**Authored envelope**
- Seed: `205005`
- Agents: `3`
- Places: `6`
- Survival health contract: `max_authored_critical_run_ticks = 550`, `max_idle_window_ticks_with_elevated_need = 50`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`

**New landed feature row**
- Travel physiology

**Why this golden is valid**

The golden proves that the isolated agent starting at `Ravine Shelter` reaches a food-producing location under real travel costs and survival pressure. It also asserts that non-Wash survival planning does not fall into planner budget exhaustion while this longer-distance loop is live. That makes the landing about survivable spatial adversity, not merely "agents stayed alive".

**Deliberately inactive**
- Drive escalation as an authored feature row
- All social, trade, justice, office, patrol, pursuit, theft, combat, artifact, and concealment features

### 5.3 Landed #3: `survival-contested`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-contested.ron`](../scenarios/survival-contested.ron)  
**Backing goldens**: [`golden_survival_contested.rs`](../crates/worldwake-ai/tests/golden_survival_contested.rs)

**Authored envelope**
- Seed: `306006`
- Agents: `4`
- Places: `8`
- Survival health contract: `max_authored_critical_run_ticks = 300`, `max_idle_window_ticks_with_elevated_need = 40`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, with `critical_run_limits.dirtiness = 1300`

**Roadmap significance**

`survival-contested` does not land a new structural feature row in the generated companion. Its value is that it hardens the already-landed baseline + travel stack under multi-agent rivalry, route invalidation, and two competing water sources.

**Why this golden is valid**

The golden proves a specific contention-era causal branch: both north-side and south-side agents reach food-producing places, and committed `drink` actions occur at both `Stone Well` and `Spring Basin`. That distinguishes the intended belief-invalidation-and-replanning story from a weaker pass where every agent survives by camping one source.

**Deliberately inactive**
- Drive escalation as an authored feature row
- All social, trade, justice, office, patrol, pursuit, theft, combat, artifact, and concealment features

### 5.4 Landed #4: `survival-drive-escalation`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-drive-escalation.ron`](../scenarios/survival-drive-escalation.ron)  
**Backing goldens**: [`golden_survival_drive_escalation.rs`](../crates/worldwake-ai/tests/golden_survival_drive_escalation.rs)

**Authored envelope**
- Seed: `116006`
- Agents: `2`
- Places: `3`
- Survival health contract: `max_authored_critical_run_ticks = 250`, `max_idle_window_ticks_with_elevated_need = 60`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`, with `critical_run_limits.dirtiness = 1300`

**New landed feature row**
- Drive escalation

**Why this golden is valid**

The golden now proves the drive-escalation branch inside a real 1440-tick survival scenario rather than an auxiliary harness-only setup. It asserts that both agents survive, satisfy the authored self-care contract, repeatedly commit `wash`, and still hit `relieve_wilderness` under sustained dirtiness pressure. Companion tests retain the narrower architectural boundaries that matter for this row: escalation does not invent remote wash knowledge, and the authoritative `escalation_end:Dirtiness:*` event follows wash relief immediately.

**Deliberately inactive**
- All social, trade, justice, office, patrol, pursuit, theft, combat, artifact, and concealment features

### 5.5 Landed #5: `survival-tell`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-tell.ron`](../scenarios/survival-tell.ron)  
**Backing goldens**: [`golden_survival_tell.rs`](../crates/worldwake-ai/tests/golden_survival_tell.rs)

**Authored envelope**
- Seed: `417005`
- Agents: `2`
- Places: `2`
- Survival health contract: `max_authored_critical_run_ticks = 220`, `max_idle_window_ticks_with_elevated_need = 40`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`, with `critical_run_limits.dirtiness = 700`

**New landed feature row**
- Tell / peer info transfer

**Why this golden is valid**

The golden proves the tell row at the earliest honest causal surface that still matters to survival behavior. `Scout Una` begins with same-place orchard knowledge, returns to `Rill Camp` for the shared water-and-wash loop, commits an accepted `tell`, and only then does `Listener Bea` acquire the orchard food belief, reach `North Orchard`, and secure orchard food. The scenario remains a real 1440-tick survival contract rather than a social harness-only vignette, and the per-need dirtiness override stays authored in the scenario instead of being hidden in the test.

`survival-tell.ron` also structurally activates `Ask-about-person` and `Consult-record` because the tell row needs live social weight and communication substrate. Those rows remain planned here because this landing does not author the record world state or prove either action family behaviorally.

**Deliberately inactive**
- Place concealment
- Obligation satiation, diversification / curiosity, experience preferences, trade, decay, disposal, facility contention, offices, theft, justice, patrol, pursuit, combat, bandit, escort, search, and stock / transport rows remain unlanded
- Report / witness remains gated inactive in the authored scenario

### 5.6 Landed #6: `survival-ask-consult`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-ask-consult.ron`](../scenarios/survival-ask-consult.ron)  
**Backing goldens**: [`golden_survival_ask_consult.rs`](../crates/worldwake-ai/tests/golden_survival_ask_consult.rs)

**Authored envelope**
- Seed: `518006`
- Agents: `4`
- Places: `2`
- Survival health contract: `max_authored_critical_run_ticks = 260`, `max_idle_window_ticks_with_elevated_need = 45`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`, with `critical_run_limits.hunger = 450` and `critical_run_limits.dirtiness = 760`

**New landed feature rows**
- Ask-about-person
- Consult-record

**Why this golden is valid**

The golden proves two explicit epistemic actions inside a live 1440-tick survival scenario instead of a social harness-only vignette. `Searcher Rowan` begins with an overdue expectation but no hearsay last-seen record for `Forager Nia`; `Witness Mira` starts at the orchard beside the subject and only returns to `Commons Hall` after real self-care pressure. The proof then asserts that `ask_about_person` commits before the searcher gains the orchard lead in `LastSeenMemory`, so the ask row lands on the causal belief-transfer seam rather than on a weaker downstream outcome.

The same scenario authors a vacant support-law office with a local `OfficeRegister`, then proves the consult branch in order: `Claimant Ivo` starts with unknown office-holder belief, `consult_record` must commit before `declare_support`, and office installation must follow. That is enough to land `Consult-record` while staying honest about row scope: the office claimant is a supporting causal actor for the consult branch, but the full survival-health contract remains tracked only on the searcher and witness who carry the row's survival pressure.

`survival-ask-consult.ron` also structurally activates office succession state, violation investigation, search, and escort substrate. Those rows remain planned because this landing proves the ask/consult seams specifically, not the full broader feature families.

**Deliberately inactive**
- Place concealment
- Obligation satiation, diversification / curiosity, experience preferences, trade, decay, disposal, facility contention, theft, justice, patrol, pursuit, combat, bandit, and stock / transport rows remain unlanded
- Report / witness remains gated inactive in the authored scenario

### 5.7 Landed #7: `survival-preferences`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-preferences.ron`](../scenarios/survival-preferences.ron)  
**Backing goldens**: [`golden_survival_preferences.rs`](../crates/worldwake-ai/tests/golden_survival_preferences.rs)

**Authored envelope**
- Seed: `619007`
- Agents: `1`
- Places: `3`
- Survival health contract: `max_authored_critical_run_ticks = 240`, `max_idle_window_ticks_with_elevated_need = 45`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`, with `critical_run_limits.hunger = 320` and `critical_run_limits.dirtiness = 760`

**New landed feature rows**
- Diversification / curiosity
- Experience preferences

**Why this golden is valid**

The golden proves the proactive-preference row at the actual branch that matters: `Scout Ilen` reaches `Novel Grove` through a proactive exploration choice, later succeeds there as a real apple source, and carries durable failure memory for the stale familiar orchard that discounts the later retry. The row therefore lands on a concrete planning-and-memory seam inside a real 1440-tick survival contract, not on a decorative visit or a test-only preference probe.

**Deliberately inactive**
- Place concealment
- Obligation satiation, production, trade, decay, disposal, facility contention, offices, theft, justice, patrol, pursuit, combat, bandit, escort, search, and stock / transport rows remain unlanded
- Report / witness remains gated inactive in the authored scenario

### 5.8 Landed #8: `survival-production`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-production.ron`](../scenarios/survival-production.ron)  
**Backing goldens**: [`golden_survival_production.rs`](../crates/worldwake-ai/tests/golden_survival_production.rs)

**Authored envelope**
- Seed: `608008`
- Agents: `1`
- Places: `1`
- Survival health contract: `max_authored_critical_run_ticks = 180`, `max_idle_window_ticks_with_elevated_need = 24`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`

**New landed feature row**
- Production (facility-backed craft)

**Why this golden is valid**

The golden proves a real non-harvest production branch inside the survival loop rather than just checking that the baker survived. `Baker Nila` starts with a mill, a well, and stocked Firewood but no direct food source; the proof then shows a selected `ProduceCommodity` plan, a committed `craft:Bake Bread`, authoritative Bread materialization, and a later Bread-consumption planning pass that only becomes possible after the production output exists. That is enough to land the row at the truthful live seam without pretending the current runtime ships a broader multi-input contract than it actually does.

**Deliberately inactive**
- Place concealment
- Obligation satiation, trade, decay, disposal, facility contention, offices, theft, justice, patrol, pursuit, combat, bandit, escort, search, and stock / transport rows remain unlanded
- Report / witness remains gated inactive in the authored scenario

### 5.9 Landed #9: `survival-trade`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-trade.ron`](../scenarios/survival-trade.ron)  
**Backing goldens**: [`golden_survival_trade.rs`](../crates/worldwake-ai/tests/golden_survival_trade.rs)

**Authored envelope**
- Seed: `609009`
- Agents: `2`
- Places: `2`
- Survival health contract: `max_authored_critical_run_ticks = 220`, `max_idle_window_ticks_with_elevated_need = 28`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`

**Scenario-owned progress**
- Merchant selling
- Trade negotiation
- Commodity valuation
- Stock / transport

**Why this scenario is landed**

The scenario now proves a real substitute-driven survival trade branch instead of a narrow auxiliary trade harness. `Merchant Sera` stages apples into a listed sale lot at `Market Square`; `Buyer Nila` starts hungry with coin but no direct bread-market branch; and the golden proves the concrete causal chain that now exists in the live runtime: listing becomes visible, planning selects the substitute `AcquireCommodity(Apple, SelfConsume)` branch, the current runtime plan keeps an explicit `trade` payload against the apple lot, the committed trade transfers Apple and Coin authoritatively, `eat` follows the first successful purchase, and both agents survive the full 1440-tick run.

That proof now closes the row honestly. The focused AI tests still own the lower substitute candidate-generation, ranking, and goal-model surfaces, while this roadmap scenario/golden pair owns the authored market branch and full survival-contract integration seam. Together they make row 9 a truthful landing instead of merely partial progress.

**Deliberately inactive**
- Place concealment
- Obligation satiation, item decay, disposal, facility contention, offices, theft, justice, patrol, pursuit, combat, bandit camps, escort, and search remain unlanded
- Report / witness remains gated inactive in the authored scenario

### 5.10 Landed #10: `survival-items-decay`

**Status**: Landed  
**Source scenario**: [`scenarios/survival-items-decay.ron`](../scenarios/survival-items-decay.ron)  
**Backing goldens**: [`golden_survival_items_decay.rs`](../crates/worldwake-ai/tests/golden_survival_items_decay.rs)

**Authored envelope**
- Seed: `610010`
- Agents: `3`
- Places: `2`
- Survival health contract: `max_authored_critical_run_ticks = 220`, `max_idle_window_ticks_with_elevated_need = 28`, required self-care families `Eat`, `Drink`, `Sleep`, `Relieve`, `Wash`

**New landed feature rows**
- Item decay
- Disposal

**Why this golden is valid**

The golden lands the maintenance row without pretending the runtime already has a broader sanitation economy than it does. `Caretaker Oren` starts with one carried Waste lot above his authored disposal threshold, the proof shows a real `FreeCarryCapacity` selection followed by a committed `drop_item`, and the same tracked lot is later archived by `ItemDecay`. That closes both halves of the row at the live seam the architecture actually exposes today: disposal as carried-waste shedding, and item decay as authoritative archive of loose ground lots.

The same scenario still keeps the earlier trade substrate alive instead of becoming a disposal-only vignette. `Merchant Sera` and `Buyer Nila` reuse the landed substitute-trade market loop from row 9, and the golden proves that the buyer still completes a real apple trade followed by `eat` during the same run. The merchant and caretaker own the scenario's tracked survival-health envelope, while the buyer remains a supporting causal actor for the trade witness path rather than a third survival-tracked principal.

**Deliberately inactive**
- Place concealment
- Obligation satiation, facility contention, offices, theft, justice, patrol, pursuit, combat, bandit camps, escort, and search remain unlanded
- Report / witness remains gated inactive in the authored scenario

### 5.11 Auxiliary and Non-Roadmap Scenarios

#### `cli-evaluation.ron`

- Source scenario: [`scenarios/cli-evaluation.ron`](../scenarios/cli-evaluation.ron)
- Backing golden: none
- Status in this roadmap: CLI/schema coverage only

Why it is not a roadmap landing:

- It has no `survival_health_contract`.
- It is not backed by a scenario golden.
- Its broad profile coverage exists to keep CLI commands and scenario schema evolution honest, not to prove that those features coexist safely with the survival loop.

## 6. Maintenance Workflow

### 6.1 Adding a New Roadmap Entry

1. Pick the next row from the priority table.
2. Copy the entry contract template into the roadmap and fill the planned-state fields.
3. Inherit the prior landed row's deliberately inactive list, then flip only the feature(s) this new row is meant to activate.
4. Update the Status Summary only when the new row is actually landed, not when it is merely drafted.

### 6.2 Authoring a Scenario for a Planned Entry

1. Copy the closest landed scenario and move the row from `Planned` to `Drafting`.
2. Apply the activation checklist through authored scenario state, not through test-only helpers.
3. Run the observer locally until the survival-health contract and must-exercise behaviors both hold.
4. Write the backing golden so it proves the authored branch at the earliest honest causal surface.
5. Regenerate or verify the generated companion with:

```bash
cargo run -p worldwake-cli --bin scenario-coverage -- --write
```

or, for CI/read-only verification:

```bash
cargo run -p worldwake-cli --bin scenario-coverage -- --check
```

6. Mark the row `Landed` only after the scenario, golden, and generated companion all agree.

### 6.3 Handling Schema Drift

- `scenario-coverage` is the structural truth source. When profile or scenario schema changes, regenerate the companion and compare the resulting activation story to the roadmap.
- If the generator emits warning rows such as unmapped authored fields, do not silently delete or hide them. Either:
  - classify the field as intentionally outside the gameplay-feature catalog, or
  - create follow-up work to promote it into the catalog.
- `cli-evaluation.ron` remains the broad schema/CLI fixture and should continue to absorb structural drift even when it does not count as a landed roadmap proof.

### 6.4 Closing Out an Entry

An entry closes only when all of the following are true:

- the scenario file exists and is committed
- the golden exists and proves both survival and the feature-specific causal branch
- the generated companion marks the feature active under the live rule
- the roadmap's landed row, status summary, and auxiliary caveats are all updated in the same change

If a scenario turns out to prove only a narrower or different branch than originally planned, rewrite the roadmap row immediately instead of leaving the older claim in place.

## 7. Detection Rule Appendix

The live generated companion implements this rule in [`scenario_coverage.rs`](../crates/worldwake-cli/src/bin/scenario_coverage.rs):

> A gameplay feature is active in a scenario only when the required authored substrate exists and any gating field for that feature is non-default in the way the generator checks it.

This appendix describes structural activation only. It does not prove that a golden exercised the feature for the intended reason.

### 7.1 Profile and Field Gates

| Surface | Active rule |
|---|---|
| `UtilityProfile`-driven features | Feature-specific weight must be greater than zero |
| `TellProfile` | `max_tell_candidates > 0` and `conversation_memory_capacity > 0` |
| `CommunicationProfile` | `testimony_acceptance > 0` and `gossip_acceptance > 0` when paired with tell/report features |
| `MetabolismProfile` travel physiology | Any travel multiplier or wilderness relief dirtiness penalty greater than zero |
| `DriveEscalationProfile` | Authored profile present and not equal to `DriveEscalationProfile::default()` |
| `PerceptionProfile` | Presence is sufficient for activation-decay perception; other features can depend on it secondarily |
| Optional profiles (`CombatProfile`, `MerchandiseProfile`, `TradeDispositionProfile`, `PatrolProfile`, `PursuitProfile`, `JusticeDispositionProfile`, `TheftDispositionProfile`, `ViolationDispositionProfile`, `ContentionDispositionProfile`, `CommodityValuationProfile`, `DisposalProfile`, `DiversificationProfile`, `PreferenceProfile`, `ObligationSatiationProfile`, `ArtifactPostingProfile`, `SubstitutePreferences`) | Presence is sufficient unless the specific feature also requires an additional utility or world-state gate |

### 7.2 World-State Gates

| Feature family | World-state requirement |
|---|---|
| Item decay | `commodity_decay` authored on the scenario |
| Place concealment | At least one place with `visibility_profile.base_concealment > 0` |
| Production / stock / transport | World must expose facilities or resource sources in addition to the agent-side authored substrate |
| Patrol | Requires both `patrol_profile` and an authored `patrol_route` |
| Offices / succession / force-claim | Requires authored office world state, not only agent profiles |
| Bandit camps | Requires authored bandit/camp world state, not only ordinary combat agents |

### 7.3 Important Consequences

- `cli-evaluation.ron` can be structurally rich without proving feature landings.
- `survival-drive-escalation.ron` now counts as active `Drive escalation` because it authors a non-default `drive_escalation_profile` inside a survival-contract scenario.
- Any change to the generator's feature list or gates must update this appendix and the catalog in the same PR.
