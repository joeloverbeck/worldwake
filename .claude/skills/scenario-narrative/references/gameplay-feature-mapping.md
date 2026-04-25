# Gameplay Feature Mapping

This reference defines how to identify which gameplay-feature rows from `docs/scenario-roadmap.md` Section 2 fired during a scenario run, what authored substrate enabled each, and what concrete outcomes to extract for the report's Section B.

The roadmap's Feature Catalog is the source of truth for *what* counts as a gameplay feature. This reference is the source of truth for *how* the dump signals each feature's activation in a single run.

## Mapping Procedure

For each feature row in the roadmap catalog:

1. Check the **agent-side substrate**: do any agents in this scenario have the activation profile/field at non-default values? (Read from the pre-flight scan.)
2. Check the **world-side substrate**: does the world state include the required facility / resource / authored field?
3. Check the **dump for committed activity**: is there a committed action, archive event, or world-state mutation in Sections 2, 4, 6, or 7 that proves the feature actually fired this run?

A feature is **exercised** in a run only when all three are present. A feature with substrate but no committed activity is **authored but inactive** — list it under that subsection at the bottom of report Section B.

## Per-Feature Mapping Table

| Feature row (catalog name) | Activation signal | Dump anchor for "exercised" | Numeric values worth quoting |
|---|---|---|---|
| Basic needs (Eat / Drink / Sleep / Relieve / Wash) | `needs` + non-zero per-need utility weight + `metabolism_profile` + `drive_thresholds` + recipe path | Section 2 action counts (`eat`, `drink`, `sleep`, `relieve_*`, `wash`); Section 7 goal selections | Per-need weights, per-need critical thresholds, per-need committed action count, per-need max value reached, per-need ticks above critical |
| Travel physiology | Any non-zero travel multiplier or wilderness relief dirtiness penalty in `metabolism_profile` | Section 2 location history with multiple places visited; per-need deltas correlated with travel ticks | Travel multipliers per need, wilderness relief dirtiness penalty, total travel ticks per agent |
| Drive escalation | Authored `drive_escalation_profile` non-default | Section 4/7 `escalation_*` events; sustained ticks above critical | Escalation onset tick, escalation end tick, the need that escalated, the value at escalation start |
| Need-driven exploration | `exploration_profile` active | Section 2 location history showing reach beyond starting place; Section 5 newly-discovered places | First-visit tick of each non-starting place, exploration parameters from the profile |
| Activation-decay perception | `perception_profile` active | Section 2 perception activity counts; Section 5 belief acquisition events | Perception fidelity, decay parameters, count of perception events per agent |
| Place concealment | Any place `visibility_profile.base_concealment > 0` | Section 5 belief deltas where co-located observers do not acquire normally-visible facts | Per-place base concealment values, witnessed-vs-co-located divergence count |
| Tell / peer info transfer | Active `tell_profile` + active `communication_profile` | Section 7 `tell` action commits; Section 5 hearsay belief acquisition | Tell candidate caps, conversation memory capacity, accepted/rejected tell counts |
| Ask-about-person | `social_weight > 0` + `communication_profile` + `epistemic_disposition` | Section 7 `ask_about_person` commits; Section 5 last-seen memory updates | Social weight, last-seen memory delta count, count of ask-target hits |
| Consult-record | `social_weight > 0` + `perception_profile` + record-bearing world state | Section 7 `consult_record` commits; Section 5 institutional belief deltas | Records consulted count, institutional memory capacity |
| Obligation satiation | `obligation_satiation_profile` present | Section 7 obligation-action commits; satiation-threshold crossing in event log | Satiation threshold, obligation actions committed, post-satiation behavior shift |
| Diversification / curiosity | `diversification_profile` present | Section 5 `SourceReliability` deltas; Section 7 novel-source selection over familiar | Diversification weights, failed-attempt counters, novel-vs-familiar selection counts |
| Experience preferences | `preference_profile` present | Section 7 preference-discounted candidates; Section 5 experience deltas | Preference parameters, discount magnitudes |
| Production (facility-backed craft) | Authored recipe set with at least one non-harvest production recipe | Section 7 `ProduceCommodity` selection; Section 4 `craft:*` commits; Section 6 produced output | Recipe identity, input/output commodities, count of successful crafts |
| Merchant selling | `merchandise_profile` present | Section 7 `stage_stock_for_sale` commits; Section 6 listed lots at market | Stock staged, sale price, market location |
| Trade negotiation | `trade_disposition` present | Section 4 `trade` action; Section 6 ownership transfers | Trade tolerance parameters, count of trades, transferred quantities |
| Commodity valuation | `commodity_valuation` present | Section 7 trade goal selection from valuation; price decisions | Valuation entries, comparison outcomes |
| Substitute preferences | `substitute_preferences` present | Section 7 substitute-branch selection (e.g., `AcquireCommodity(SelfConsume)` choosing Apple when Bread absent) | Ordered substitute list, substitute branches taken |
| Item decay | `commodity_decay` authored on the scenario | Section 4 `ItemDecay` archive events | Decay rate per commodity, decay events count |
| Disposal | `disposal_profile` present | Section 7 `FreeCarryCapacity` selection; Section 4 `drop_item` commits | Disposal threshold, dropped items count |
| Facility-queue contention | `contention_disposition` + facility `contention_policy` | Section 7 `queue_for_facility_use` commits; grant-promotion events | Queue patience values, grants emitted, harvest commits via grant path |
| Offices / succession / force-claim | Office entities + force-claim world state | Section 7 `ClaimOffice` selection; `press_force_claim` commit; Section 4 office-installation events | Force-control delay, claim outcomes |
| Bounty posting | Non-zero `bounty_posting_weight` + `artifact_posting_profile` | Section 7 `PostBounty` selection; Section 4 bounty artifact materialization; treasury encumbrance | Posting weight, treasury reward amount |
| Notice posting | Non-zero `notice_posting_weight` + `artifact_posting_profile` | Section 7 `PostNotice` selection; Section 4 notice artifact materialization | Posting weight, notice category, content |
| Theft | `theft_disposition` present | Section 7 `StealItem` selection; Section 4 `steal` commit | Theft thresholds, target item, witness suppression outcome |
| Justice / accusation | `justice_disposition` present | Section 7 `accuse` selection; Section 4 accusation in crime register | Accusation parameters, recorded verdict |
| Violation investigation | `violation_disposition` present | Section 7 `investigate_*` commits; Section 5 `SuspectedTheft` social observation | Investigation thresholds, suspicion outcomes |
| Patrol | `patrol_profile` + `patrol_route` | Section 4 `patrol` commits at authored waypoints | Route waypoints visited, patrol parameters |
| Pursuit | `pursuit_profile` present | Section 7 in-range remote `EngageHostile` candidate; `Travel -> Attack` plan; terminal `attack` commit | Pursuit thresholds, route cost, attack outcome |
| Combat | `combat_profile` present | Section 4 `attack` commits; downstream `DeadAt` events | Combat parameters, attacks per actor, lethality outcomes |
| Escort | `care_weight > 0` | Section 7 `EscortToSafety` selection; Section 4 `escort_to_safety` commits with co-located handoff | Care weight, ward identity, handoff destination |
| Bandit camps | Authored `bandit_camps` world state | Section 4 camp-membership events; `empty_since_tick`; `BanditCamp` component clearance | Faction grace period, members lost, camp clearance tick |
| Report / witness | `perception_profile` + `tell_profile` + `communication_profile` | Section 7 `report_*` commits; Section 5 testimony acceptance | Acceptance thresholds, count of accepted reports |
| Search | `violation_disposition` + `epistemic_disposition` | Section 7 `search_*` commits; expectation resolution events | Search outcomes, found-status writes |
| Stock / transport | `merchandise_profile` + stock-supporting world state | Section 4 stock movement / transport events | Transported quantities, stock cycles |

## Reading the Catalog Live

The hand-authored catalog above mirrors `docs/scenario-roadmap.md` Section 2. If a row exists in the live roadmap that is missing from this table, fall back to the roadmap's prose description; do not invent activation rules. If the roadmap evolves (new feature row added or activation gate changed), update this reference in the same change.

## How to Quote Numeric Values in the Report

For each exercised feature, the Section B paragraph should answer:

- **What is the mechanic** (one to three sentences in plain English).
- **What authored substrate enabled it in this run** — name the agent profiles and the world fields, with their concrete numeric values from the pre-flight scan.
- **What occurred during the run** — committed action counts (Section 2), archive events (Section 4), final-state outcomes (Section 6), and tick references for landmark events. Anchor at least one specific tick per feature wherever possible.

Avoid bare action counts without context. "`drink` committed 9 times" is weak; "Agent C committed `drink` 9 times across the run, half of them at Spring Basin and half at Stone Well, peaking around tick 600 when the wells' regeneration cadence (8 and 10 ticks respectively) was outpaced by the four converging agents" is the target shape.

## Authored-but-Inactive Subsection

A feature row whose substrate is present but whose dump anchor never fires belongs in the Section B "Authored but inactive" subsection. State why it stayed inactive when the cause is legible from the dump (e.g., "no hostile target was ever in the patrol's line of sight, so pursuit never triggered"). When the cause is not legible, list the row without speculation.
